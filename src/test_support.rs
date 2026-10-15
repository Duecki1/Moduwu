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
            .find_map(|clipped| text_pos(&clipped.shape, text))
            .expect("the label is painted");
        (rect, pos)
    };
    let (rect, _) = paint(vec![egui::Event::PointerMoved(egui::pos2(-50.0, -50.0))]);
    let (_, idle) = paint(Vec::new());
    paint(vec![egui::Event::PointerMoved(rect.center())]);
    let (_, hovered) = paint(Vec::new());
    (idle, hovered)
}

fn text_pos(shape: &egui::Shape, text: &str) -> Option<Pos2> {
    match shape {
        egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape.pos),
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| text_pos(shape, text)),
        _ => None,
    }
}
