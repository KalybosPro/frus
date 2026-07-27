//! Aperçu jetable (jalon 139) : un champ multi-lignes **défilé** (scroll retenu
//! injecté dans le runtime), rendu hors écran pour vérifier à l'œil.

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

    // Identité du champ pour y poser un défilement retenu.
    let ids = collect_ids(&root);
    let mut runtime = Runtime::default();
    // Le champ est le 2e nœud (Container=racine, enfant=champ).
    let field_id = ids[1];
    runtime.scroll.insert(field_id, (0.0, 44.0)); // ~deux lignes plus bas

    let ui = build_ui(&root, size, &runtime, &theme);
    let Some(snap) = frus_test::render_scene(ui.scene(), 340, 160, theme.background) else {
        eprintln!("pas de GPU : aperçu ignoré");
        return;
    };
    // Défilé de ~2 lignes : « Line three/four/five » visibles, clippées à la boîte.
    snap.assert_golden(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/multiline_scrolled.png"));
}
