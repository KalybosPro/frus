//! Tests du harnais lui-même : snapshot de scène, rendu de widgets, goldens.
//!
//! Sans adaptateur GPU les tests s'ignorent (le harnais renvoie `None`).

use frus_core::{Color, Point, Rect, Scene, TextStyle};
use frus_test::{render_scene, render_widget};
use frus_widgets::{
    Autocomplete, Container, DateTimePicker, Dropdown, Flex, RangeSlider, Table, Text, TextInput,
    Theme, TimePicker,
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
