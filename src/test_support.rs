//! Headless input helpers shared by the widget tests.

use egui::{self, Pos2, Rect, Ui};

pub(crate) fn key_press(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

/// A touch at `pos`, with the pointer events egui's integrations send with
/// it.
pub(crate) fn touch(phase: egui::TouchPhase, pos: Pos2) -> Vec<egui::Event> {
    let touch = egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase,
        pos,
        force: None,
    };
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let pointer = match phase {
        egui::TouchPhase::Start => vec![egui::Event::PointerMoved(pos), button(true)],
        egui::TouchPhase::Move => vec![egui::Event::PointerMoved(pos)],
        egui::TouchPhase::End | egui::TouchPhase::Cancel => vec![button(false)],
    };
    std::iter::once(touch).chain(pointer).collect()
}

/// Runs one frame of `show` and returns its accessibility and click events.
pub(crate) fn run_frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    show: &mut impl FnMut(&mut Ui) -> Rect,
) -> (Rect, Vec<egui::output::OutputEvent>) {
    let mut rect = Rect::NOTHING;
    let output = ctx.run_ui(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ui| rect = show(ui),
    );
    (rect, output.platform_output.events)
}

/// Clicks the point `target` picks from the rect `show` returned on the
/// previous frame: one frame to lay out, then a press and a release.
pub(crate) fn click(
    ctx: &egui::Context,
    mut show: impl FnMut(&mut Ui) -> Rect,
    target: impl Fn(Rect) -> Pos2,
) -> Vec<egui::output::OutputEvent> {
    let (rect, _) = run_frame(ctx, Vec::new(), &mut show);
    let pos = target(rect);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let (_, mut events) = run_frame(
        ctx,
        vec![egui::Event::PointerMoved(pos), button(true)],
        &mut show,
    );
    let (_, released) = run_frame(ctx, vec![button(false)], &mut show);
    events.extend(released);
    events
}

/// The last widget information reported by a click or value change.
pub(crate) fn last_widget_info(events: &[egui::output::OutputEvent]) -> Option<egui::WidgetInfo> {
    events.iter().rev().find_map(|event| match event {
        egui::output::OutputEvent::ValueChanged(info)
        | egui::output::OutputEvent::Clicked(info) => Some(info.clone()),
        _ => None,
    })
}

/// Where `show` paints `text` with the pointer away, then resting on the
/// widget, in a themed context.
pub(crate) fn label_positions_idle_and_hovered(
    text: &str,
    mut show: impl FnMut(&mut Ui) -> Rect,
) -> (Pos2, Pos2) {
    let ctx = egui::Context::default();
    crate::Design::ObsidianBlue.theme().apply(&ctx);
    let mut paint = |events: Vec<egui::Event>| {
        let mut rect = Rect::NOTHING;
        let output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| rect = show(ui),
        );
        let pos = output
            .shapes
            .iter()
            .find_map(|clipped| painted_text(&clipped.shape, text))
            .expect("the label is painted")
            .pos;
        (rect, pos)
    };
    let (rect, _) = paint(vec![egui::Event::PointerMoved(egui::pos2(-50.0, -50.0))]);
    let (_, idle) = paint(Vec::new());
    paint(vec![egui::Event::PointerMoved(rect.center())]);
    let (_, hovered) = paint(Vec::new());
    (idle, hovered)
}

/// The layout rect of `text` among a frame's `shapes`.
pub(crate) fn painted_text_rect(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<Rect> {
    shapes
        .iter()
        .find_map(|clipped| painted_text(&clipped.shape, text))
        .map(|shape| shape.galley.rect.translate(shape.pos.to_vec2()))
}

fn painted_text<'a>(shape: &'a egui::Shape, text: &str) -> Option<&'a egui::epaint::TextShape> {
    match shape {
        egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape),
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| painted_text(shape, text)),
        _ => None,
    }
}
