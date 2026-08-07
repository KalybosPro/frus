//! A throwaway preview (milestone 139): a **scrolled** multi-line field — the
//! retained scroll is injected into the runtime — rendered offscreen to be checked by
//! eye.

use frus_core::Size;
use frus_widgets::{build_ui, collect_ids, Container, Runtime, TextInput, Theme};

#[test]
fn scrolled_multiline_matches_golden() {
    let field = TextInput::<()>::new(
        "Line one\nLine two\nLine three\nLine four\nLine five\nLine six\nLine seven",
    )
    .width(280.0)
    .label("Notes")
    .rows(3);
    let root: Container<()> = Container::new().padding(20.0).child(field);

    let theme = Theme::dark();
    let size = Size::new(340.0, 160.0);

    // The field's identity, so a retained scroll can be placed on it.
    let ids = collect_ids(&root);
    let mut runtime = Runtime::default();
    // The field is the second node: Container is the root, its child is the field.
    let field_id = ids[1];
    runtime.scroll.insert(field_id, (0.0, 44.0)); // about two lines further down

    let ui = build_ui(&root, size, &runtime, &theme);
    let Some(snap) = frus_test::render_scene(ui.scene(), 340, 160, theme.background) else {
        eprintln!("no GPU: preview skipped");
        return;
    };
    // Scrolled by about 2 lines: "Line three/four/five" visible, clipped to the box.
    snap.assert_golden(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/goldens/multiline_scrolled.png"
    ));
}
