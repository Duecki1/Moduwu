//! Short looping animations, such as bundled help clips, decoded from GIF or
//! WebP bytes. Every frame is decoded and uploaded once, so the format suits
//! short clips rather than long video.
//!
//! An animation plays as soon as it is shown and loops. Clicking it, or
//! pressing Space or Enter while it has focus, pauses and resumes it, so
//! moving content can always be stopped.

use std::fmt;
use std::io::Cursor;
use std::time::Duration;

use egui::{Color32, Rect, Response, Sense, Ui};
use image::codecs::gif::GifDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, DynamicImage, ImageFormat};

/// Frames shorter than this play for [`SHORT_FRAME_DURATION`], as browsers
/// do: encoders write 0 or 10 ms delays meaning "as fast as allowed".
const MIN_FRAME_DURATION: Duration = Duration::from_millis(20);
const SHORT_FRAME_DURATION: Duration = Duration::from_millis(100);

/// A decoded animation whose frames are uploaded as egui textures.
pub struct AnimatedImage {
    frames: Vec<Frame>,
    /// One loop, the sum of the frame durations.
    period: Duration,
    /// Pixel size of every frame.
    size: [usize; 2],
}

struct Frame {
    texture: egui::TextureHandle,
    duration: Duration,
}

/// Why bytes could not be shown as an [`AnimatedImage`].
#[derive(Debug)]
pub enum AnimatedImageError {
    /// The bytes are not a GIF or WebP image.
    UnsupportedFormat,
    /// The image is malformed, or has no frames.
    Decode(String),
}

impl fmt::Display for AnimatedImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat => formatter.write_str("not a GIF or WebP image"),
            Self::Decode(message) => write!(formatter, "could not decode the animation: {message}"),
        }
    }
}

impl std::error::Error for AnimatedImageError {}

/// Where an animation is in its loop, kept in egui memory per widget.
#[derive(Clone, Copy, Debug, Default)]
struct Playback {
    /// The time, in egui's input clock, at which the loop position was zero.
    origin: f64,
    /// The loop position while paused.
    paused_at: Option<f64>,
}

impl AnimatedImage {
    /// Decodes a GIF or WebP image, animated or still, and uploads its frames
    /// as textures named after `name`.
    pub fn from_bytes(
        ctx: &egui::Context,
        name: &str,
        bytes: &[u8],
    ) -> Result<Self, AnimatedImageError> {
        let decoded = decode_frames(bytes)?;
        let size = decoded
            .first()
            .map(|(image, _)| [image.width() as usize, image.height() as usize])
            .ok_or_else(|| AnimatedImageError::Decode("the image has no frames".to_owned()))?;
        let frames: Vec<Frame> = decoded
            .into_iter()
            .enumerate()
            .map(|(index, (image, duration))| Frame {
                texture: ctx.load_texture(
                    format!("{name}#{index}"),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                ),
                duration,
            })
            .collect();
        Ok(Self {
            period: frames.iter().map(|frame| frame.duration).sum(),
            frames,
            size,
        })
    }

    /// Pixel size of the frames.
    pub fn size(&self) -> egui::Vec2 {
        egui::vec2(self.size[0] as f32, self.size[1] as f32)
    }

    /// Height divided by width, for fitting the animation.
    pub fn aspect(&self) -> f32 {
        self.size[1] as f32 / self.size[0].max(1) as f32
    }

    /// Paints the frame due now, stretched to `rect`, and plays the
    /// animation. `label` names it for accessibility.
    pub fn show(&self, ui: &mut Ui, rect: Rect, label: &str) -> Response {
        let animated = self.frames.len() > 1 && !self.period.is_zero();
        let sense = if animated {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, ui.id().with(("animated-image", label)), sense);
        let now = ui.input(|input| input.time);
        let mut playback = ui
            .data(|data| data.get_temp::<Playback>(response.id))
            .unwrap_or(Playback {
                origin: now,
                paused_at: None,
            });
        let period = self.period.as_secs_f64();
        if animated && response.clicked() {
            playback = match playback.paused_at {
                Some(position) => Playback {
                    origin: now - position,
                    paused_at: None,
                },
                None => Playback {
                    paused_at: Some((now - playback.origin).rem_euclid(period)),
                    ..playback
                },
            };
        }
        ui.data_mut(|data| data.insert_temp(response.id, playback));

        let position = playback
            .paused_at
            .unwrap_or_else(|| (now - playback.origin).rem_euclid(period.max(f64::EPSILON)));
        let (index, remaining) = frame_at(
            self.frames.iter().map(|frame| frame.duration),
            Duration::from_secs_f64(position.max(0.0)),
        );
        if let Some(frame) = self.frames.get(index) {
            ui.painter_at(rect).image(
                frame.texture.id(),
                rect,
                Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        let paused = playback.paused_at.is_some();
        if animated {
            if !paused {
                ui.ctx().request_repaint_after(remaining);
            }
            if paused || response.hovered() || response.has_focus() {
                paint_playback_badge(ui, rect, paused);
            }
            let action = if paused { "Play" } else { "Pause" };
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("{action} {label}"),
                )
            });
        } else {
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, label));
        }
        response
    }
}

/// The index of the frame showing `position` into the loop, and how long it
/// stays. A position past the end shows the last frame.
fn frame_at(durations: impl Iterator<Item = Duration>, position: Duration) -> (usize, Duration) {
    let mut start = Duration::ZERO;
    let mut last = (0, Duration::ZERO);
    for (index, duration) in durations.enumerate() {
        let end = start + duration;
        if position < end {
            return (index, end - position);
        }
        last = (index, Duration::ZERO);
        start = end;
    }
    last
}

/// A play triangle while paused, pause bars while playing, centred in `rect`.
fn paint_playback_badge(ui: &Ui, rect: Rect, paused: bool) {
    let painter = ui.painter_at(rect);
    let radius = (rect.width().min(rect.height()) * 0.12).clamp(12.0, 28.0);
    let center = rect.center();
    painter.circle_filled(center, radius, Color32::from_black_alpha(150));
    let glyph = radius * 0.45;
    if paused {
        painter.add(egui::Shape::convex_polygon(
            vec![
                center + egui::vec2(-glyph * 0.6, -glyph),
                center + egui::vec2(glyph, 0.0),
                center + egui::vec2(-glyph * 0.6, glyph),
            ],
            Color32::WHITE,
            egui::Stroke::NONE,
        ));
    } else {
        for x in [-glyph * 0.45, glyph * 0.45] {
            painter.rect_filled(
                Rect::from_center_size(
                    center + egui::vec2(x, 0.0),
                    egui::vec2(glyph * 0.45, glyph * 1.8),
                ),
                1.0,
                Color32::WHITE,
            );
        }
    }
}

/// Every frame of a GIF or WebP image with how long it shows. A still image
/// is one frame.
fn decode_frames(bytes: &[u8]) -> Result<Vec<(image::RgbaImage, Duration)>, AnimatedImageError> {
    let decode_error = |error: image::ImageError| AnimatedImageError::Decode(error.to_string());
    let frames = match image::guess_format(bytes) {
        Ok(ImageFormat::Gif) => GifDecoder::new(Cursor::new(bytes))
            .map_err(decode_error)?
            .into_frames(),
        Ok(ImageFormat::WebP) => {
            let decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(decode_error)?;
            if !decoder.has_animation() {
                let image = DynamicImage::from_decoder(decoder).map_err(decode_error)?;
                return Ok(vec![(image.to_rgba8(), Duration::ZERO)]);
            }
            decoder.into_frames()
        }
        _ => return Err(AnimatedImageError::UnsupportedFormat),
    };
    frames
        .map(|frame| {
            let frame = frame.map_err(decode_error)?;
            let (numerator, denominator) = frame.delay().numer_denom_ms();
            let duration =
                Duration::from_secs_f64(f64::from(numerator) / f64::from(denominator.max(1)) / 1e3);
            let duration = if duration < MIN_FRAME_DURATION {
                SHORT_FRAME_DURATION
            } else {
                duration
            };
            Ok((frame.into_buffer(), duration))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Rgba, RgbaImage};

    /// A 4×2 GIF whose frames are red for 50 ms, then green with a 0 ms delay.
    fn two_frame_gif() -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = GifEncoder::new(&mut bytes);
        for (colour, delay) in [([255, 0, 0, 255], 50), ([0, 255, 0, 255], 0)] {
            encoder
                .encode_frame(image::Frame::from_parts(
                    RgbaImage::from_pixel(4, 2, Rgba(colour)),
                    0,
                    0,
                    Delay::from_numer_denom_ms(delay, 1),
                ))
                .unwrap();
        }
        drop(encoder);
        bytes
    }

    #[test]
    fn gif_frames_keep_their_timing_and_fast_frames_slow_down() {
        let frames = decode_frames(&two_frame_gif()).unwrap();
        let durations: Vec<_> = frames.iter().map(|(_, duration)| *duration).collect();
        assert_eq!(durations, [Duration::from_millis(50), SHORT_FRAME_DURATION]);
        assert_eq!(frames[1].0.get_pixel(0, 0), &Rgba([0, 255, 0, 255]));

        let ctx = egui::Context::default();
        let animation = AnimatedImage::from_bytes(&ctx, "clip", &two_frame_gif()).unwrap();
        assert_eq!(animation.size(), egui::vec2(4.0, 2.0));
        assert_eq!(animation.aspect(), 0.5);
    }

    #[test]
    fn a_still_webp_is_one_frame() {
        let mut bytes = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
            .encode(
                &[0, 0, 255, 255].repeat(6),
                3,
                2,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        let frames = decode_frames(&bytes).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0.dimensions(), (3, 2));
    }

    #[test]
    fn other_bytes_are_refused() {
        let ctx = egui::Context::default();
        assert!(matches!(
            AnimatedImage::from_bytes(&ctx, "clip", b"not an image"),
            Err(AnimatedImageError::UnsupportedFormat)
        ));
    }

    #[test]
    fn the_frame_follows_the_loop_position() {
        let durations = [50, 100].map(Duration::from_millis);
        let at = |ms| frame_at(durations.into_iter(), Duration::from_millis(ms));
        assert_eq!(at(0), (0, Duration::from_millis(50)));
        assert_eq!(at(49), (0, Duration::from_millis(1)));
        assert_eq!(at(50), (1, Duration::from_millis(100)));
        assert_eq!(at(400), (1, Duration::ZERO));
    }

    #[test]
    fn a_click_pauses_and_resumes_the_loop() {
        let ctx = egui::Context::default();
        let animation = AnimatedImage::from_bytes(&ctx, "clip", &two_frame_gif()).unwrap();
        let rect = Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(80.0, 40.0));
        let mut ui_id = None;
        let mut frame = |time: f64, events: Vec<egui::Event>| {
            let _ = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = animation.show(ui, rect, "Help clip");
                    ui_id = Some(response.id);
                },
            );
            ctx.data(|data| data.get_temp::<Playback>(ui_id.unwrap()))
                .unwrap()
        };
        let pointer = rect.center();
        let click = |pressed| egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };

        assert!(frame(0.0, vec![egui::Event::PointerMoved(pointer)])
            .paused_at
            .is_none());
        frame(0.02, vec![click(true)]);
        let paused = frame(0.03, vec![click(false)]);
        assert!(paused
            .paused_at
            .is_some_and(|position| (position - 0.03).abs() < 1e-9));
        // Paused, the position no longer moves with time.
        assert_eq!(frame(1.0, Vec::new()).paused_at, paused.paused_at);

        frame(1.02, vec![click(true)]);
        let resumed = frame(1.03, vec![click(false)]);
        assert!(resumed.paused_at.is_none());
        assert!((1.03 - resumed.origin - 0.03).abs() < 1e-9);
    }
}
