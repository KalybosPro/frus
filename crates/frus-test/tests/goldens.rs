//! Tests du harnais lui-même : snapshot de scène, rendu de widgets, goldens.
//!
//! Sans adaptateur GPU les tests s'ignorent (le harnais renvoie `None`).

use frus_core::{Color, Point, Rect, Scene, TextStyle};
use frus_test::{render_scene, render_widget};
use frus_widgets::{
    Autocomplete, Avatar, Button, Chip, Container, DateTimePicker, Dropdown, Flex, IconName,
    LineChart, Menu, RangeSlider, Table, Text, TextInput, Theme, TimePicker, Variant,
};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Une scène mixte (rect arrondi + texte décoré) reproduit son golden à
/// l'identique — le pipeline entier est déterministe dans cet environnement.
#[test]
fn scene_matches_golden() {
    let mut scene = Scene::new();
    scene.draw_rect(
        Rect::new(8.0, 8.0, 104.0, 48.0),
        Color::rgb8(46, 160, 96),
        10.0,
        0.0,
        Color::TRANSPARENT,
    );
    scene.text_styled(
        Point::new(16.0, 20.0),
        "Golden",
        &TextStyle::new(20.0).underline(),
        Color::WHITE,
    );
    let Some(snapshot) = render_scene(&scene, 120, 64, Color::BLACK) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    // Sanité avant golden : le fond est noir, le rect est bien dessiné.
    assert_eq!(snapshot.pixel(2, 2), [0, 0, 0, 255], "coin = clear");
    assert!(snapshot.lit_pixels(16) > 100, "rect + texte dessinés");
    snapshot.assert_golden(golden("scene_rect_text"));
}

/// Un arbre de widgets rend comme le ferait le shell (layout + thème), et
/// reproduit son golden.
#[test]
fn widget_tree_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(8.0)
            .child(Text::styled("Title", theme.text.title_medium))
            .child(Text::new("done item").strikethrough().size(14.0)),
    );
    let Some(snapshot) = render_widget(&root, 160, 80, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 50, "du texte est dessiné");
    snapshot.assert_golden(golden("widget_column_text"));
}

/// Un **formulaire décoré** (jalon 132) : un champ en erreur (label + bordure +
/// message rouges) au-dessus d'un champ au repos (indice + texte d'aide discrets).
/// Reproduit son golden — les deux états de la décoration sont figés.
#[test]
fn decorated_form_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(24.0).child(
        Flex::column()
            .gap(16.0)
            .child(
                TextInput::<()>::new("ada@")
                    .width(280.0)
                    .label("Email")
                    .placeholder("you@example.com")
                    .error("Enter a valid email address"),
            )
            .child(
                TextInput::<()>::new("")
                    .width(280.0)
                    .label("Password")
                    .placeholder("At least 8 characters")
                    .helper("Use letters, numbers and symbols"),
            ),
    );
    let Some(snapshot) = render_widget(&root, 360, 260, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "labels, champs et textes dessinés");
    snapshot.assert_golden(golden("decorated_form"));
}

/// **Champ à contour (jalon 144)** : style `outlined`, le label flottant se pose sur la
/// bordure du haut, ouverte d'une **encoche** derrière lui. Le premier champ est rempli
/// (label monté, encoche ouverte) ; le second est vide (label au repos, pas d'encoche).
#[test]
fn outlined_field_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(24.0).child(
        Flex::column()
            .gap(20.0)
            .child(
                TextInput::<()>::new("Ada Lovelace")
                    .width(280.0)
                    .outlined()
                    .label("Full name"),
            )
            .child(
                TextInput::<()>::new("")
                    .width(280.0)
                    .outlined()
                    .label("Email")
                    .placeholder("you@example.com"),
            ),
    );
    let Some(snapshot) = render_widget(&root, 360, 200, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "bordures, labels et texte dessinés");
    snapshot.assert_golden(golden("outlined_field"));
}

/// **Tableau de données (jalon 145)** : en-tête triable (indicateur ▲ sur la colonne
/// triée) et ligne sélectionnée surlignée. Reproduit son golden.
#[test]
fn data_table_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Table::<()>::new(3)
            .width(300.0)
            .header(&["Name", "Role", "Score"])
            .sorted(0, true)
            .selected(&[1])
            .row(&["Ada", "Engineer", "5"])
            .row(&["Bob", "Designer", "3"])
            .row(&["Cara", "Manager", "4"]),
    );
    let Some(snapshot) = render_widget(&root, 340, 200, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête, lignes et textes dessinés");
    snapshot.assert_golden(golden("data_table"));
}

/// **Tableau à sélection multiple (jalon 148)** : colonne de cases à cocher (avec « tout
/// cocher » en en-tête), première colonne à largeur fixe. Deux lignes cochées.
#[test]
fn data_table_multiselect_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Table::<()>::new(3)
            .width(320.0)
            .column_widths(&[90.0])
            .header(&["Name", "Role", "Score"])
            .checkboxes(|_| (), ())
            .selected(&[0, 2])
            .row(&["Ada", "Engineer", "5"])
            .row(&["Bob", "Designer", "3"])
            .row(&["Cara", "Manager", "4"]),
    );
    let Some(snapshot) = render_widget(&root, 360, 200, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "cases, en-tête et lignes dessinés");
    snapshot.assert_golden(golden("data_table_multiselect"));
}

/// **Aperçu de réordonnancement (jalons 155/158/159)** : en glissant l'en-tête « Role »
/// vers la droite (sur « Score »), la colonne source est **retirée**, « Score » **coulisse**
/// pour combler le trou (place de dépôt ouverte à droite), et une **carte fidèle**
/// (fond + texte « Role », soulevée) suit le curseur. Reconstruit la superposition du
/// shell (`reflow_reorder_columns` + carte fantôme). Reproduit son golden.
#[test]
fn table_reorder_preview_matches_golden() {
    use frus_widgets::{build_ui, reflow_reorder_columns, Primitive, Runtime, Size};

    let theme = Theme::dark();
    let table = Table::<()>::new(3)
        .column_widths(&[110.0, 110.0, 90.0])
        .header(&["Name", "Role", "Score"])
        .on_sort(|_| ())
        .on_reorder(|_, _| ())
        .row(&["Ada", "Engineer", "5"])
        .row(&["Bob", "Designer", "3"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let (w, h) = (420u32, 150u32);
    let ui = build_ui(&root, Size::new(w as f32, h as f32), &Runtime::default(), &theme);

    // En-tête « Role » (colonne 1) glissé vers la droite, curseur au-delà de « Score »
    // (qui a donc pleinement coulissé pour combler la place de « Role »).
    let role = Point::new(16.0 + 110.0 + 2.0 + 55.0, 16.0 + 17.0);
    let id = ui.hit(role).expect("en-tête Role cliquable");
    let src = ui.widget_rect(id).expect("bornes de l'en-tête Role");
    let dx = 150.0;

    // Coulissement des colonnes voisines suivant le curseur (source retirée, « Score »
    // comblé vers la gauche à mesure que le curseur le dépasse).
    let mut scene = ui.scene().clone();
    let reflowed = reflow_reorder_columns(scene.primitives(), src, role.x + dx, id.as_u64());
    scene.clear();
    for primitive in reflowed {
        scene.push_primitive(primitive);
    }
    // Carte soulevée : ombre + face fidèle (primitives de l'en-tête translatées et
    // dé-découpées) + bord accentué.
    scene.set_clip(Rect::UNBOUNDED);
    let card = src.translate(dx, -2.0);
    scene.shadow(card.translate(0.0, 4.0), Color::BLACK.fade(0.28), theme.radius, 12.0);
    scene.draw_rect(card, theme.surface, theme.radius, 0.0, Color::TRANSPARENT);
    let ghost: Vec<Primitive> = ui
        .scene()
        .primitives()
        .iter()
        .filter(|p| p.owner() == id.as_u64())
        .map(|p| p.translated(dx, -2.0).with_clip(Rect::UNBOUNDED))
        .collect();
    for primitive in &ghost {
        scene.push_primitive(primitive.clone());
    }
    scene.draw_rect(card, Color::TRANSPARENT, theme.radius, 1.5, theme.primary.fade(0.9));

    let Some(snapshot) = render_scene(&scene, w, h, theme.background) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(!ghost.is_empty(), "la face fidèle capture les primitives de l'en-tête");
    assert!(snapshot.lit_pixels(40) > 100, "tableau réagencé + carte fantôme dessinés");
    snapshot.assert_golden(golden("table_reorder_preview"));
}

/// **Tableau à cellules-widgets (jalon 164)** : une colonne d'**avatars** et une colonne
/// de **puces** (`Chip`), au-delà du texte. Reproduit son golden.
#[test]
fn table_widget_cells_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .column_widths(&[70.0])
        .header(&["User", "Role"])
        .widget_row(vec![
            Box::new(|| Box::new(Avatar::new("Ada").size(26.0))),
            Box::new(|| Box::new(Chip::<()>::new("admin"))),
        ])
        .widget_row(vec![
            Box::new(|| Box::new(Avatar::new("Bo").size(26.0))),
            Box::new(|| Box::new(Chip::<()>::new("editor"))),
        ]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 180, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête, avatars et puces dessinés");
    snapshot.assert_golden(golden("table_widget_cells"));
}

/// **Tableau à hauteur de rangée adaptative (jalon 166)** : une rangée dont la cellule
/// contient un grand avatar (48 px) grandit au-delà de la hauteur nominale, tandis
/// qu'une rangée texte garde sa hauteur de confort — aucun rognage. Reproduit son golden.
#[test]
fn table_adaptive_rows_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .column_widths(&[70.0])
        .header(&["User", "Role"])
        .widget_row(vec![
            Box::new(|| Box::new(Avatar::new("Ada").size(48.0))),
            Box::new(|| Box::new(Chip::<()>::new("admin"))),
        ])
        .row(&["Bo", "editor"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 180, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "grande rangée et rangée texte dessinées");
    snapshot.assert_golden(golden("table_adaptive_rows"));
}

/// **En-têtes avec icône (jalon 168)** : une icône de tête précède le libellé de la
/// colonne (icône + texte), l'en-tête restant triable. Reproduit son golden.
#[test]
fn table_header_icons_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .header(&["Name", "Rating"])
        .header_icons(&[Some(IconName::Menu), Some(IconName::Star)])
        .sorted(1, false)
        .row(&["Ada", "5"])
        .row(&["Bob", "3"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 160, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-têtes à icônes et données dessinés");
    snapshot.assert_golden(golden("table_header_icons"));
}

/// **En-tête à widget d'action (jalon 170)** : un bouton (« Filter ») posé à droite d'un
/// en-tête, cliquable indépendamment — le reste de l'en-tête triant toujours (indicateur ▲
/// sur « Name »). Reproduit son golden.
#[test]
fn table_header_action_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(300.0)
        .header(&["Name", "Status"])
        .on_sort(|_| ())
        .sorted(0, true)
        .header_action(1, || {
            Box::new(Button::new("Filter").size(12.0).variant(Variant::Secondary).on_press(()))
        })
        .row(&["Ada", "Active"])
        .row(&["Bob", "Away"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 160, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête, bouton d'action et données dessinés");
    snapshot.assert_golden(golden("table_header_action"));
}

/// **En-tête entièrement widget (jalon 171)** : la ligne d'en-tête est faite de widgets
/// arbitraires — ici une puce « User » et un bouton de tri maison « Sort » — au lieu de
/// libellés texte. Le comportement (tri) est câblé par l'application. Reproduit son golden.
#[test]
fn table_widget_header_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(300.0)
        .column_widths(&[110.0])
        .widget_header(vec![
            Box::new(|| Box::new(Chip::<()>::new("User"))),
            Box::new(|| Box::new(Button::new("Sort").size(12.0).variant(Variant::Secondary).on_press(()))),
        ])
        .row(&["Ada", "Active"])
        .row(&["Bob", "Away"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 160, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-têtes widget et données dessinés");
    snapshot.assert_golden(golden("table_widget_header"));
}

/// **Menu de colonne (jalon 172)** : un `Menu` déposé en widget d'action d'en-tête ouvre un
/// menu **flottant** d'actions de colonne, rendu par-dessus la grille même imbriqué dans
/// l'en-tête — sans code spécifique côté tableau. Reproduit son golden.
#[test]
fn table_column_menu_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(300.0)
        .column_widths(&[150.0])
        .header(&["Name", "Score"])
        .on_sort(|_| ())
        .header_action(0, || {
            Box::new(
                Menu::new(Button::new("...").size(12.0).variant(Variant::Secondary).on_press(()), true, ())
                    .item("Sort ascending", ())
                    .item("Sort descending", ())
                    .item("Hide column", ()),
            )
        })
        .row(&["Ada", "5"])
        .row(&["Bob", "3"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 230, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête et menu de colonne flottant dessinés");
    snapshot.assert_golden(golden("table_column_menu"));
}

/// **Tableau virtualisé (jalon 173)** : 1000 lignes, dont seules les visibles sont
/// construites ; l'en-tête reste épinglé au-dessus d'un viewport défilant. Reproduit son
/// golden (la fenêtre visible du haut).
#[test]
fn table_virtualized_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .column_widths(&[60.0])
        .header(&["#", "Item"])
        .virtual_rows(1000, 120.0, |i| vec![format!("{}", i + 1), format!("Item {}", i + 1)]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 190, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête épinglé et lignes visibles dessinés");
    snapshot.assert_golden(golden("table_virtualized"));
}

/// **Tableau virtualisé à cellules-widgets (jalon 176)** : 500 lignes d'avatars + puces,
/// dont seules les visibles sont construites, sous un en-tête épinglé. Reproduit son golden.
#[test]
fn table_virtual_widgets_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .column_widths(&[70.0])
        .header(&["User", "Tag"])
        .virtual_widget_rows(500, 130.0, |i| {
            vec![
                Box::new(Avatar::new(format!("U{i}")).size(26.0)) as Box<dyn frus_widgets::Widget<()>>,
                Box::new(Chip::<()>::new(format!("tag {}", i + 1))),
            ]
        });
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 200, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête, avatars et puces virtualisés dessinés");
    snapshot.assert_golden(golden("table_virtual_widgets"));
}

/// **Tableau virtualisé à sélection multiple (jalon 177)** : colonne de cases à cocher (avec
/// « tout cocher » épinglé) sur des lignes virtualisées, deux lignes cochées. Reproduit son golden.
#[test]
fn table_virtual_checkboxes_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(280.0)
        .column_widths(&[120.0])
        .header(&["Name", "Role"])
        .checkboxes(|_| (), ())
        .selected(&[1, 2])
        .virtual_rows(1000, 130.0, |i| vec![format!("User {}", i + 1), "member".to_string()]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 320, 200, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "cases, en-tête et lignes virtualisées dessinés");
    snapshot.assert_golden(golden("table_virtual_checkboxes"));
}

/// **Tableau à colonnes gelées (jalon 178)** : la première colonne (« Name ») reste figée à
/// gauche tandis que les colonnes de trimestres défilent horizontalement (Q3 hors cadre).
/// Reproduit son golden (position initiale, offset horizontal nul).
#[test]
fn table_frozen_columns_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(4)
        .width(280.0)
        .column_widths(&[90.0, 90.0, 90.0, 90.0])
        .header(&["Name", "Q1", "Q2", "Q3"])
        .on_sort(|_| ())
        .sorted(0, true)
        .frozen_columns(1)
        .row(&["Ada", "10", "20", "30"])
        .row(&["Bob", "12", "18", "24"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 320, 150, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "colonne gelée et colonnes défilantes dessinées");
    snapshot.assert_golden(golden("table_frozen_columns"));
}

/// **Colonnes gelées aux deux bords (jalon 179)** : « Name » figée à gauche et « Act » figée
/// à droite, les colonnes du milieu défilant entre les deux, avec une ombre de séparation à
/// chaque bord de gel. Reproduit son golden.
#[test]
fn table_frozen_both_edges_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(4)
        .width(300.0)
        .column_widths(&[80.0, 110.0, 110.0, 70.0])
        .header(&["Name", "Q1", "Q2", "Act"])
        .frozen_columns(1)
        .frozen_columns_right(1)
        .row(&["Ada", "10", "20", "..."])
        .row(&["Bob", "12", "18", "..."]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 150, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "colonnes gelées des deux bords dessinées");
    snapshot.assert_golden(golden("table_frozen_both_edges"));
}

/// **Récapitulatif d'erreurs de formulaire (jalon 180)** : après une soumission invalide, une
/// carte teintée « erreur » liste tous les messages (`Form::errors` → `ErrorSummary`), au-dessus
/// du champ fautif. Reproduit son golden.
#[test]
fn form_error_summary_matches_golden() {
    use frus_widgets::form::{Form, Rule};
    let theme = Theme::dark();
    let report = Form::new()
        .field("email", "nope", Rule::email("Enter a valid email address"))
        .field("password", "x", Rule::min_len(8, "At least 8 characters"));
    let summary =
        frus_widgets::ErrorSummary::<()>::new(report.errors().into_iter().map(|(_, m)| m.to_string()));
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::column().gap(16.0).child(summary).child(
            TextInput::<()>::new("nope")
                .width(300.0)
                .label("Email")
                .error("Enter a valid email address"),
        ),
    );
    let Some(snapshot) = render_widget(&root, 360, 230, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "récapitulatif d'erreurs et champ dessinés");
    snapshot.assert_golden(golden("form_error_summary"));
}

/// **Formulaire multi-étapes (jalon 182)** : l'indicateur `Steps` (étape 2/3 en cours, la
/// première terminée avec une coche) coiffe le contenu de l'étape et une barre Précédent/Suivant.
/// Reproduit son golden.
#[test]
fn form_wizard_matches_golden() {
    use frus_widgets::Steps;
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::column()
            .gap(18.0)
            .child(Steps::new(["Account", "Profile", "Review"]).current(1))
            .child(Text::styled("Profile", theme.text.title_medium))
            .child(TextInput::<()>::new("Ada Lovelace").width(340.0).label("Full name"))
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Secondary))
                    .child(Button::new("Next")),
            ),
    );
    let Some(snapshot) = render_widget(&root, 420, 280, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "indicateur d'étapes, champ et boutons dessinés");
    snapshot.assert_golden(golden("form_wizard"));
}

/// **Calendrier en mode plage (jalon 184)** : juillet 2026, intervalle du 10 au 15 —
/// bornes en pastille pleine, jours intermédiaires en bande douce. Reproduit son golden.
#[test]
fn date_range_matches_golden() {
    use frus_widgets::DatePicker;
    let theme = Theme::dark();
    let picker = DatePicker::range(
        2026,
        7,
        Some((2026, 7, 10)),
        Some((2026, 7, 15)),
        |_| (),
        |_| (),
    );
    let root: Container<()> = Container::new().padding(16.0).child(picker);
    let Some(snapshot) = render_widget(&root, 300, 340, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "calendrier et plage dessinés");
    snapshot.assert_golden(golden("date_range"));
}

/// **Calendrier double (jalon 186)** : juillet + août 2026 côte à côte, plage du 28 juillet au
/// 3 août — la bande de plage se poursuit d'un mois à l'autre. Reproduit son golden.
#[test]
fn date_range_dual_matches_golden() {
    use frus_widgets::DatePicker;
    let theme = Theme::dark();
    let picker = DatePicker::range_dual(
        2026,
        7,
        Some((2026, 7, 28)),
        Some((2026, 8, 3)),
        |_| (),
        |_| (),
    );
    let root: Container<()> = Container::new().padding(16.0).child(picker);
    let Some(snapshot) = render_widget(&root, 580, 320, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 200, "deux mois et plage dessinés");
    snapshot.assert_golden(golden("date_range_dual"));
}

/// **Plage horaire (jalon 187)** : deux sélecteurs d'heure étiquetés « Start » (09:00) et
/// « End » (17:30), minutes par pas de 15. Reproduit son golden.
#[test]
fn time_range_matches_golden() {
    use frus_widgets::TimeRange;
    let theme = Theme::dark();
    let range = TimeRange::new((9, 0), (17, 30), |_, _, _| ()).minute_step(15);
    let root: Container<()> = Container::new().padding(16.0).child(range);
    let Some(snapshot) = render_widget(&root, 540, 420, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 200, "deux sélecteurs d'heure dessinés");
    snapshot.assert_golden(golden("time_range"));
}

/// **Couche de notifications (jalon 188)** : deux toasts empilés dans le coin bas-droit
/// (`ToastHost`), le second portant une action « UNDO ». Reproduit son golden.
#[test]
fn toast_host_matches_golden() {
    use frus_widgets::{Toast, ToastHost, ToastPosition};
    let theme = Theme::dark();
    let host: ToastHost<()> = ToastHost::new(ToastPosition::BottomEnd)
        .toast(Toast::new("File uploaded").success())
        .toast(Toast::new("Message archived").action("Undo", ()));
    let Some(snapshot) = render_widget(&host, 420, 220, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 60, "toasts empilés dessinés");
    snapshot.assert_golden(golden("toast_host"));
}

/// **Plage date + heure (jalon 189)** : calendrier double (28/07 → 03/08), plage horaire
/// (09:00 → 17:30) et récapitulatif « début → fin ». Reproduit son golden.
#[test]
fn datetime_range_matches_golden() {
    use frus_widgets::DateTimeRange;
    let theme = Theme::dark();
    let picker = DateTimeRange::new(
        2026,
        7,
        Some((2026, 7, 28)),
        Some((2026, 8, 3)),
        (9, 0),
        (17, 30),
        |_| (),
        |_| (),
        |_, _, _| (),
    );
    let root: Container<()> = Container::new().padding(16.0).child(picker);
    let Some(snapshot) = render_widget(&root, 580, 760, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 300, "calendrier double, heures et récap dessinés");
    snapshot.assert_golden(golden("datetime_range"));
}

/// **Assistant intégré, étape Review avec erreurs (jalon 190)** : l'indicateur `Steps`
/// (Review courant), un récapitulatif d'erreurs **cliquable** (`ErrorSummary::links`) et la
/// barre Back / Create — l'assemblage réel de l'écran assistant du démo. Reproduit son golden.
#[test]
fn wizard_review_errors_matches_golden() {
    use frus_widgets::{ErrorSummary, Steps};
    let theme = Theme::dark();
    let summary = ErrorSummary::links([
        ("Email is required".to_string(), ()),
        ("Passwords do not match".to_string(), ()),
    ]);
    let root: Container<()> = Container::new().padding(24.0).child(
        Flex::column()
            .gap(24.0)
            .child(Steps::new(["Account", "Security", "Review"]).current(2).on_tap(|_| ()))
            .child(summary)
            .child(Text::new("Creating account for Ada <not-an-email>").size(16.0))
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Secondary))
                    .child(Button::new("Create account")),
            ),
    );
    let Some(snapshot) = render_widget(&root, 480, 380, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "assistant (étape Review, erreurs) dessiné");
    snapshot.assert_golden(golden("wizard_review_errors"));
}

/// **Bouton désactivé (jalon 191)** : « Next » actif (accent, ombre) à côté de sa version
/// désactivée (grisée, sans ombre). Reproduit son golden.
#[test]
fn button_disabled_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::row()
            .gap(16.0)
            .child(Button::new("Next"))
            .child(Button::new("Next").enabled(false)),
    );
    let Some(snapshot) = render_widget(&root, 260, 90, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 40, "les deux boutons sont dessinés");
    snapshot.assert_golden(golden("button_disabled"));
}

/// **Assistant, étape Security (jalon 192)** : `Steps` (Security courant), deux mots de passe
/// **masqués** (`TextInput::obscure`), et « Next » **désactivé** (confirmation non concordante).
/// Reproduit son golden.
#[test]
fn wizard_password_step_matches_golden() {
    use frus_widgets::Steps;
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(24.0).child(
        Flex::column()
            .gap(24.0)
            .child(Steps::new(["Account", "Security", "Review"]).current(1))
            .child(
                Flex::column()
                    .gap(14.0)
                    .child(TextInput::<()>::new("secret12").width(340.0).label("Password").obscure(true))
                    .child(
                        TextInput::<()>::new("secr")
                            .width(340.0)
                            .label("Confirm password")
                            .obscure(true),
                    ),
            )
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Secondary))
                    .child(Button::new("Next").enabled(false)),
            ),
    );
    let Some(snapshot) = render_widget(&root, 440, 420, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "étape mot de passe dessinée");
    snapshot.assert_golden(golden("wizard_password_step"));
}

/// **Assistant, mots de passe révélés + étapes par validité (jalons 194–195)** : la bascule
/// « Hide password » démasque les champs (`obscure(false)`), et l'étape Account est marquée
/// **terminée** (coche) via `Steps::completed` — pas seulement par position. Reproduit son golden.
#[test]
fn wizard_password_revealed_matches_golden() {
    use frus_widgets::Steps;
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(24.0).child(
        Flex::column()
            .gap(24.0)
            .child(
                Steps::new(["Account", "Security", "Review"])
                    .current(1)
                    .completed([true, false, false]),
            )
            .child(
                Flex::column()
                    .gap(14.0)
                    .child(TextInput::<()>::new("secret12").width(340.0).label("Password"))
                    .child(TextInput::<()>::new("secret12").width(340.0).label("Confirm password"))
                    .child(Button::new("Hide password").variant(Variant::Secondary)),
            )
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Secondary))
                    .child(Button::new("Next")),
            ),
    );
    let Some(snapshot) = render_widget(&root, 440, 460, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "étape révélée dessinée");
    snapshot.assert_golden(golden("wizard_password_revealed"));
}

/// **Table éditable en ligne (jalon 196)** : une grille où chaque cellule est un widget
/// (`Table::widget_row`) — cellules statiques **cliquables** (un `Container` qui émet
/// « éditer cette cellule ») et une cellule **en édition** rendue par un `TextInput`. Prouve
/// que l'édition en ligne se compose sans nouveau mécanisme. Reproduit son golden.
#[test]
fn table_editable_matches_golden() {
    use frus_widgets::Table;
    let theme = Theme::dark();
    // Fabrique d'une cellule statique cliquable (clic → passer cette cellule en édition).
    let cell = |value: &str| -> Box<dyn Fn() -> Box<dyn frus_widgets::Widget<()>>> {
        let value = value.to_string();
        Box::new(move || {
            Box::new(
                Container::<()>::new()
                    .padding_each(6.0, 10.0, 6.0, 10.0)
                    .child(Text::new(value.clone()).size(15.0))
                    .on_click(()),
            ) as Box<dyn frus_widgets::Widget<()>>
        })
    };
    // Fabrique d'une cellule **en édition** : un champ de saisie lié à la valeur.
    let editing = |value: &str| -> Box<dyn Fn() -> Box<dyn frus_widgets::Widget<()>>> {
        let value = value.to_string();
        Box::new(move || {
            Box::new(TextInput::<()>::new(value.clone()).width(180.0).size(15.0))
                as Box<dyn frus_widgets::Widget<()>>
        })
    };
    let table = Table::new(3)
        .header(&["Name", "Role", "Email"])
        .column_widths(&[150.0, 150.0, 200.0])
        .widget_row(vec![cell("Ada Lovelace"), cell("Engineer"), cell("ada@example.com")])
        .widget_row(vec![cell("Alan Turing"), editing("Cryptographer"), cell("alan@example.com")])
        .widget_row(vec![cell("Grace Hopper"), cell("Admiral"), cell("grace@example.com")]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 560, 220, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 150, "grille éditable dessinée");
    snapshot.assert_golden(golden("table_editable"));
}

/// **Graphique à barres (jalon 199)** : une série `(jour, valeur)` en barres mises à l'échelle
/// du maximum, valeurs au-dessus, libellés en dessous, ligne de base. Reproduit son golden.
#[test]
fn bar_chart_matches_golden() {
    use frus_widgets::BarChart;
    let theme = Theme::dark();
    let chart = BarChart::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0);
    // `BarChart` remplit la largeur (Percent) : le parent doit avoir une largeur **définie**.
    let root: Container<()> = Container::new().width(360.0).height(240.0).padding(20.0).child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "graphique à barres dessiné");
    snapshot.assert_golden(golden("bar_chart"));
}

/// **Graphique en lignes (jalon 200)** : la même série `(jour, valeur)` que la BarChart, mais
/// tracée en polyligne (segments + marqueurs ronds), valeurs au-dessus, libellés dessous, ligne
/// de base. Reproduit son golden.
#[test]
fn line_chart_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0);
    // `LineChart` remplit la largeur (Percent) : le parent doit avoir une largeur **définie**.
    let root: Container<()> = Container::new().width(360.0).height(240.0).padding(20.0).child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "graphique en lignes dessiné");
    snapshot.assert_golden(golden("line_chart"));
}

/// **Graphique en lignes avec axe (jalon 203)** : la même série que `line_chart`, mais avec un axe
/// des ordonnées à 4 divisions (`grid(4)`) — lignes de grille horizontales et graduations `0..max`
/// dans une marge à gauche, partagées avec la BarChart. Reproduit son golden.
#[test]
fn line_chart_axis_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4);
    let root: Container<()> = Container::new().width(360.0).height(240.0).padding(20.0).child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "graphique avec axe dessiné");
    snapshot.assert_golden(golden("line_chart_axis"));
}

/// **Graphique en lignes avec aire (jalon 206)** : la même série, avec l'aire sous la courbe
/// remplie (`area(true)`) et l'axe des ordonnées (`grid(4)`). Reproduit son golden.
#[test]
fn line_chart_area_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .area(true);
    let root: Container<()> = Container::new().width(360.0).height(240.0).padding(20.0).child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "graphique avec aire dessiné");
    snapshot.assert_golden(golden("line_chart_area"));
}

/// **Graphique multi-séries avec légende (jalon 209)** : deux séries nommées partageant les mêmes
/// catégories et la même échelle, tracées dans leurs couleurs, avec axe (`grid(4)`) et légende
/// (pastille + nom). Reproduit son golden.
#[test]
fn line_chart_multi_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("Sales")
    .series("Costs", Color::rgb8(220, 120, 80), [2.0, 4.0, 3.0, 5.0, 2.0])
    .legend(true);
    let root: Container<()> = Container::new().width(360.0).height(260.0).padding(20.0).child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "graphique multi-séries dessiné");
    snapshot.assert_golden(golden("line_chart_multi"));
}

/// **Champ mot de passe avec œil (jalon 202)** : un `TextInput` **masqué** (`obscure`) portant
/// l'icône « œil » suffixe (`on_suffix`) qui révèle le texte. Reproduit son golden.
#[test]
fn password_eye_matches_golden() {
    let theme = Theme::dark();
    let field = TextInput::<()>::new("hunter2")
        .width(280.0)
        .label("Password")
        .obscure(true)
        .suffix_icon(IconName::Eye)
        .on_suffix(());
    let root: Container<()> = Container::new().padding(20.0).child(field);
    let Some(snapshot) = render_widget(&root, 340, 110, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 60, "champ masqué et œil dessinés");
    snapshot.assert_golden(golden("password_eye"));
}

/// **Champ avec suffixe cliquable (jalon 198)** : un `TextInput` rempli portant une icône
/// suffixe « ✕ » cliquable (`on_suffix`) qui l'efface. Reproduit son golden.
#[test]
fn textinput_clear_matches_golden() {
    let theme = Theme::dark();
    let field = TextInput::<()>::new("Buy milk")
        .width(280.0)
        .label("New task")
        .suffix_icon(IconName::Close)
        .on_suffix(());
    let root: Container<()> = Container::new().padding(20.0).child(field);
    let Some(snapshot) = render_widget(&root, 340, 110, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 60, "champ et suffixe dessinés");
    snapshot.assert_golden(golden("textinput_clear"));
}

/// **Snackbar avec action (jalon 185)** : une notification transitoire portant un bouton
/// « UNDO » à droite (façon Material). Reproduit son golden.
#[test]
fn snackbar_action_matches_golden() {
    use frus_widgets::Toast;
    let theme = Theme::dark();
    let toast = Toast::new("Message archived").action("Undo", ());
    let root: Container<()> = Container::new().padding(20.0).child(toast);
    let Some(snapshot) = render_widget(&root, 340, 90, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 60, "carte, texte et action dessinés");
    snapshot.assert_golden(golden("snackbar_action"));
}

/// **Tableau redimensionnable (jalon 151)** : colonnes à largeur fixe avec une fine
/// poignée verticale au bord droit de chaque colonne (sauf la dernière). Reproduit son
/// golden.
#[test]
fn data_table_resizable_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Table::<()>::new(3)
            .column_widths(&[110.0, 110.0, 70.0])
            .header(&["Name", "Role", "Score"])
            .sorted(0, true)
            .on_resize(|_, _| ())
            .row(&["Ada", "Engineer", "5"])
            .row(&["Bob", "Designer", "3"])
            .row(&["Cara", "Manager", "4"]),
    );
    let Some(snapshot) = render_widget(&root, 360, 200, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "en-tête, lignes et poignées dessinés");
    snapshot.assert_golden(golden("data_table_resizable"));
}

/// **Sélecteur d'heure (jalon 146)** : aperçu `HH:MM`, grille des heures (0–23) et des
/// minutes (pas de 5), case sélectionnée surlignée. Reproduit son golden.
#[test]
fn time_picker_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new()
        .padding(20.0)
        .child(TimePicker::<()>::new(9, 30, |_| (), |_| ()));
    let Some(snapshot) = render_widget(&root, 280, 400, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "aperçu, grilles et cases dessinés");
    snapshot.assert_golden(golden("time_picker"));
}

/// **Sélecteur d'heure 12 h (jalon 147)** : bascule AM/PM + grille 1–12, aperçu `3:05 PM`.
#[test]
fn time_picker_12h_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new()
        .padding(20.0)
        .child(TimePicker::<()>::new(15, 5, |_| (), |_| ()).hour12());
    let Some(snapshot) = render_widget(&root, 280, 420, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "aperçu, AM/PM et grilles dessinés");
    snapshot.assert_golden(golden("time_picker_12h"));
}

/// **Flux date + heure (jalon 147)** : calendrier + sélecteur d'heure, coiffés d'un
/// récapitulatif de la sélection. Reproduit son golden.
#[test]
fn date_time_picker_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(DateTimePicker::<()>::new(
        2026,
        7,
        Some(11),
        9,
        30,
        |_| (),
        |_| (),
        |_| (),
        |_| (),
    ));
    let Some(snapshot) = render_widget(&root, 320, 640, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 200, "récap, calendrier et heure dessinés");
    snapshot.assert_golden(golden("date_time_picker"));
}

/// **Liste déroulante ouverte (jalon 150)** : en-tête + menu flottant, l'option
/// sélectionnée surlignée et cochée. Reproduit son golden.
#[test]
fn dropdown_menu_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(16.0).child(
        Dropdown::<()>::new("Medium", ())
            .width(200.0)
            .selected(1)
            .options(true, &["Small", "Medium", "Large"], |_| ()),
    );
    let Some(snapshot) = render_widget(&root, 240, 260, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 80, "en-tête, options, surlignage et coche dessinés");
    snapshot.assert_golden(golden("dropdown_menu"));
}

/// **Autocomplétion (jalon 152)** : champ « ap » et liste flottante ; la portion
/// correspondante (« ap ») est mise en avant dans chaque suggestion et la 2ᵉ (active)
/// est surlignée. Reproduit son golden.
#[test]
fn autocomplete_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(16.0).child(
        Autocomplete::<()>::new("ap", |_| (), |_| ())
            .width(220.0)
            .active(1)
            .suggestion("apple")
            .suggestion("apricot")
            .suggestion("grape"),
    );
    let Some(snapshot) = render_widget(&root, 260, 240, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 80, "champ, suggestions, surlignage et mise en avant dessinés");
    snapshot.assert_golden(golden("autocomplete"));
}

/// **Curseur de plage (jalon 156)** : deux poignées délimitant un intervalle, segment
/// actif teinté `primary` entre elles. Reproduit son golden.
#[test]
fn range_slider_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new()
        .padding(24.0)
        .child(RangeSlider::<()>::new(0.3, 0.7).width(240.0));
    let Some(snapshot) = render_widget(&root, 300, 80, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 40, "piste, segment actif et deux poignées dessinés");
    snapshot.assert_golden(golden("range_slider"));
}

/// **Curseur de plage étiqueté (jalons 160/162)** : l'infobulle de valeur n'apparaît
/// qu'au **survol / focus** d'une poignée. Ici la poignée basse est focalisée : sa bulle
/// « 30% » et son anneau de focus s'affichent. Reproduit son golden.
#[test]
fn range_slider_labels_matches_golden() {
    use frus_widgets::{build_ui, Runtime, Size};

    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(24.0).child(
        RangeSlider::<()>::new(0.3, 0.7)
            .width(240.0)
            .value_label(|v| format!("{}%", (v * 100.0).round() as i32)),
    );
    let (w, h) = (300u32, 110u32);
    // Poignée basse : centre x = 24 + 0.3·240 = 96, dans la bande piste basse.
    let probe = Point::new(96.0, 62.0);
    let base = build_ui(&root, Size::new(w as f32, h as f32), &Runtime::default(), &theme);
    let id = base.draggable_at(probe).map(|(id, _)| id).expect("poignée basse saisissable");
    // Reconstruit avec la poignée basse **focalisée** (révèle la bulle + l'anneau).
    let mut runtime = Runtime::default();
    runtime.input.focused = Some(id);
    let ui = build_ui(&root, Size::new(w as f32, h as f32), &runtime, &theme);

    let Some(snapshot) = render_scene(ui.scene(), w, h, theme.background) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 60, "piste, poignées et infobulle focalisée dessinées");
    snapshot.assert_golden(golden("range_slider_labels"));
}

/// **Autocomplétion défilante (jalon 154)** : liste plus longue que le seuil
/// (`max_visible(3)`) → viewport borné à 3 lignes, contenu défilable (6 suggestions).
/// Reproduit son golden.
#[test]
fn autocomplete_scroll_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(16.0).child(
        Autocomplete::<()>::new("a", |_| (), |_| ())
            .width(220.0)
            .max_visible(3)
            .suggestion("Alabama")
            .suggestion("Alaska")
            .suggestion("Arizona")
            .suggestion("Arkansas")
            .suggestion("California")
            .suggestion("Colorado"),
    );
    let Some(snapshot) = render_widget(&root, 260, 220, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 80, "champ et liste bornée dessinés");
    snapshot.assert_golden(golden("autocomplete_scroll"));
}

/// Un **champ mot de passe** (jalon 133) : valeur masquée par des points, icône
/// de préfixe à gauche et de suffixe à droite. Reproduit son golden.
#[test]
fn password_field_matches_golden() {
    use frus_widgets::IconName;

    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        TextInput::<()>::new("hunter2")
            .width(280.0)
            .label("Password")
            .obscure(true)
            .prefix_icon(IconName::Circle)
            .suffix_icon(IconName::Check)
            .helper("Tap the eye to reveal"),
    );
    let Some(snapshot) = render_widget(&root, 340, 130, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 80, "points, icônes et textes dessinés");
    snapshot.assert_golden(golden("password_field"));
}

/// **Bout-en-bout (jalon 135)** : un `Form` valide des valeurs saisies et pilote
/// le `error(...)` de chaque champ — le rendu montre le formulaire d'inscription
/// *après une soumission invalide*. Reproduit son golden.
#[test]
fn validated_signup_form_matches_golden() {
    use frus_widgets::form::{Form, Rule};

    // Ce que l'utilisateur aurait saisi avant de soumettre.
    let (email, password) = ("ada", "short");
    let report = Form::new()
        .field(
            "email",
            email,
            Rule::all([Rule::required("Required"), Rule::email("Enter a valid email address")]),
        )
        .field("password", password, Rule::min_len(8, "At least 8 characters"));

    // Les erreurs du rapport alimentent directement les champs.
    let mut email_field = TextInput::<()>::new(email).width(280.0).label("Email");
    if let Some(e) = report.error("email") {
        email_field = email_field.error(e);
    }
    let mut password_field =
        TextInput::<()>::new(password).width(280.0).label("Password").obscure(true);
    if let Some(e) = report.error("password") {
        password_field = password_field.error(e);
    }

    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::column().gap(14.0).child(email_field).child(password_field),
    );
    let Some(snapshot) = render_widget(&root, 340, 210, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(!report.is_valid(), "les deux champs sont invalides");
    assert_eq!(report.first_invalid(), Some("email"), "le premier à focaliser");
    snapshot.assert_golden(golden("validated_signup_form"));
}

/// Un **champ multi-lignes** (jalon 137) : label flottant, plusieurs lignes de
/// contenu (retours explicites) dans une boîte de `rows` lignes. Reproduit son golden.
#[test]
fn multiline_field_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        TextInput::<()>::new(
            "Roses are red, violets are blue, and this long line wraps softly to the field width.",
        )
        .width(300.0)
        .label("Message")
        .rows(4),
    );
    let Some(snapshot) = render_widget(&root, 360, 170, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 120, "label et trois lignes de texte dessinés");
    snapshot.assert_golden(golden("multiline_field"));
}

/// Le calque **inspecteur** (contours + surlignage + fiche du widget désigné)
/// par-dessus un arbre rendu — reproduit son golden.
#[test]
fn inspector_overlay_matches_golden() {
    use frus_core::Size;
    use frus_widgets::{build_ui_inspected, paint_inspector_overlay, Runtime};

    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(10.0).child(
        Flex::column()
            .gap(6.0)
            .child(Text::new("Inspect me").size(14.0))
            .child(Text::new("plain").size(12.0)),
    );
    let runtime = Runtime::default();
    let size = Size::new(180.0, 120.0);
    let (ui, nodes) = build_ui_inspected(&root, size, &runtime, &theme);
    assert!(nodes.len() >= 4, "l'arbre entier est observé ({})", nodes.len());

    let mut scene = ui.scene().clone();
    // Le curseur désigne le premier texte : surlignage + fiche.
    paint_inspector_overlay(&nodes, Some(Point::new(20.0, 18.0)), size, &theme, &mut scene);
    let Some(snapshot) = frus_test::render_scene(&scene, 180, 120, theme.background) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    snapshot.assert_golden(golden("inspector_overlay"));
}

/// **RTL** : la même rangée [rouge][vert][bleu] se retourne horizontalement —
/// le rouge (1er enfant) passe à droite. Preuve indépendante de la police du
/// miroir de mise en page.
#[test]
fn rtl_mirrors_the_row() {
    let red = Color::rgb(0.9, 0.2, 0.2);
    let blue = Color::rgb(0.2, 0.4, 0.9);
    let make = || {
        Flex::<()>::row()
            .width(150.0)
            .height(40.0)
            .child(Container::new().width(50.0).height(40.0).color(red))
            .child(Container::new().width(50.0).height(40.0).color(Color::rgb(0.2, 0.8, 0.4)))
            .child(Container::new().width(50.0).height(40.0).color(blue))
    };
    // LTR : rouge à gauche, bleu à droite.
    let ltr_theme = Theme::dark();
    let rtl_theme = Theme::dark().rtl();
    let (Some(ltr), Some(rtl)) = (
        render_widget(&make(), 150, 40, &ltr_theme),
        render_widget(&make(), 150, 40, &rtl_theme),
    ) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    let is_red = |px: [u8; 4]| px[0] > 180 && px[1] < 120;
    let is_blue = |px: [u8; 4]| px[2] > 180 && px[0] < 120;
    // LTR : rouge au bord gauche, bleu au bord droit.
    assert!(is_red(ltr.pixel(10, 20)) && is_blue(ltr.pixel(140, 20)), "LTR normal");
    // RTL : miroir — rouge à droite, bleu à gauche.
    assert!(is_red(rtl.pixel(140, 20)) && is_blue(rtl.pixel(10, 20)), "RTL retourné");
    rtl.assert_golden(golden("rtl_row"));
}

/// **RTL** : un tiroir de bord (`end_drawer`, côté *end* = droite en LTR)
/// passe à **gauche** en RTL — le placement des overlays suit la direction.
#[test]
fn rtl_flips_the_drawer_side() {
    use frus_widgets::Scaffold;
    let drawer_color = Color::rgb(0.9, 0.3, 0.3);
    let make = || {
        Scaffold::<()>::new(200.0, 120.0)
            .body(Container::new().width(200.0).height(120.0).color(Color::rgb(0.1, 0.1, 0.12)))
            .end_drawer(
                Container::new().width(90.0).height(120.0).color(drawer_color),
                true,
                (),
            )
            .build()
    };
    let is_drawer = |px: [u8; 4]| px[0] > 180 && px[1] < 120 && px[2] < 120;
    let (Some(ltr), Some(rtl)) = (
        render_widget(make().as_ref(), 200, 120, &Theme::dark()),
        render_widget(make().as_ref(), 200, 120, &Theme::dark().rtl()),
    ) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    let edge = |s: &frus_test::Snapshot, x: u32| is_drawer(s.pixel(x, 60));
    // LTR : le tiroir est ancré au bord **gauche** (côté start).
    assert!(edge(&ltr, 2) && !edge(&ltr, 197), "LTR : tiroir à gauche");
    // RTL : miroir — le tiroir passe au bord **droit**.
    assert!(edge(&rtl, 197) && !edge(&rtl, 2), "RTL : tiroir à droite");
    rtl.assert_golden(golden("rtl_drawer"));
}

/// **Opacité de groupe** (widget → walk → calque → GPU) : un `Container` à
/// `opacity(0.5)` atténue son fond rouge par rapport au même à `opacity(1.0)`
/// (rendu opaque, sans calque). Preuve pixel de bout en bout du fondu de groupe.
#[test]
fn group_opacity_fades_the_box() {
    let make = |o: f32| {
        Container::<()>::new()
            .width(40.0)
            .height(40.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .opacity(o)
    };
    let (Some(opaque), Some(faded)) = (
        render_widget(&make(1.0), 40, 40, &Theme::dark()),
        render_widget(&make(0.5), 40, 40, &Theme::dark()),
    ) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    let r_opaque = opaque.pixel(20, 20)[0];
    let r_faded = faded.pixel(20, 20)[0];
    assert!(r_opaque > 230, "opaque → plein rouge : {r_opaque}");
    assert!(
        r_faded < r_opaque - 40,
        "opacité de groupe 0.5 atténue le rouge : {r_faded} vs {r_opaque}"
    );
}

/// Le comparateur : identique → 0 diff ; un pixel changé → 1 diff.
#[test]
fn diff_count_is_exact() {
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), Color::rgb(0.3, 0.5, 0.7));
    let Some(a) = render_scene(&scene, 64, 64, Color::BLACK) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    let mut b = render_scene(&scene, 64, 64, Color::BLACK).unwrap();
    assert_eq!(a.diff_count(&b, 0), 0, "deux rendus identiques");
    // Corrompt un pixel au-delà de la tolérance.
    b.rgba[0] = b.rgba[0].wrapping_add(64);
    assert_eq!(a.diff_count(&b, 2), 1);
    assert_eq!(a.diff_count(&b, 255), 0, "tolérance maximale absorbe tout");
}
