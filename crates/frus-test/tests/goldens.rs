//! Tests of the harness itself: scene snapshots, widget rendering, goldens.
//!
//! With no GPU adapter the tests skip themselves, the harness returning `None`.

use frus_core::{Color, Point, Rect, Scene, TextStyle};
use frus_test::{render_scene, render_widget};
use frus_widgets::{
    Autocomplete, Avatar, BarChart, Button, Chip, Container, DateTimePicker, Dropdown, Flex,
    IconName, LineChart, Menu, RangeSlider, Table, Text, TextInput, Theme, TimePicker, Variant,
};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// A mixed scene — a rounded rect plus decorated text — reproduces its golden
/// exactly: the whole pipeline is deterministic in this environment.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    // A sanity check before the golden: the background is black, the rect is drawn.
    assert_eq!(
        snapshot.pixel(2, 2),
        [0, 0, 0, 255],
        "the corner is the clear colour"
    );
    assert!(snapshot.lit_pixels(16) > 100, "rect and text are drawn");
    snapshot.assert_golden(golden("scene_rect_text"));
}

/// A widget tree renders the way the shell would, layout and theme included, and
/// reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 50, "some text is drawn");
    snapshot.assert_golden(golden("widget_column_text"));
}

/// A **decorated form** (milestone 132): a field in error, with a red label, border
/// and message, above a field at rest, with a discreet placeholder and helper text.
/// Reproduces its golden — both decoration states are pinned down.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "labels, fields and text are drawn"
    );
    snapshot.assert_golden(golden("decorated_form"));
}

/// **An outlined field (milestone 144)**: the `outlined` style, where the floating
/// label sits on the top border, which opens a **notch** behind it. The first field is
/// filled — label raised, notch open — and the second empty: label at rest, no notch.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "borders, labels and text are drawn"
    );
    snapshot.assert_golden(golden("outlined_field"));
}

/// **A data table (milestone 145)**: a sortable header, with a ▲ indicator on the
/// sorted column, and a highlighted selected row. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "header, rows and text are drawn"
    );
    snapshot.assert_golden(golden("data_table"));
}

/// **A multi-select table (milestone 148)**: a checkbox column, with a "check all" in
/// the header, and a fixed-width first column. Two rows checked.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "checkboxes, header and rows are drawn"
    );
    snapshot.assert_golden(golden("data_table_multiselect"));
}

/// **A reorder preview (milestones 155/158/159)**: dragging the "Role" header right,
/// onto "Score", **removes** the source column, **slides** "Score" over to close the
/// gap — opening the drop slot on the right — and a **faithful card**, its background
/// and "Role" text lifted, follows the pointer. This rebuilds the shell's overlay
/// (`reflow_reorder_columns` plus the ghost card). Reproduces its golden.
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
    let ui = build_ui(
        &root,
        Size::new(w as f32, h as f32),
        &Runtime::default(),
        &theme,
    );

    // The "Role" header, column 1, dragged right, the pointer past "Score", which has
    // therefore slid all the way over to fill "Role"'s slot.
    let role = Point::new(16.0 + 110.0 + 2.0 + 55.0, 16.0 + 17.0);
    let id = ui.hit(role).expect("the Role header is clickable");
    let src = ui.widget_rect(id).expect("the Role header's bounds");
    let dx = 150.0;

    // The neighbouring columns slide with the pointer: the source is removed and
    // "Score" fills leftwards as the pointer moves past it.
    let mut scene = ui.scene().clone();
    let reflowed = reflow_reorder_columns(scene.primitives(), src, role.x + dx, id.as_u64());
    scene.clear();
    for primitive in reflowed {
        scene.push_primitive(primitive);
    }
    // The lifted card: a shadow, a faithful face from the header's translated and
    // un-clipped primitives, and an accent border.
    scene.set_clip(Rect::UNBOUNDED);
    let card = src.translate(dx, -2.0);
    scene.shadow(
        card.translate(0.0, 4.0),
        Color::BLACK.fade(0.28),
        theme.radius,
        12.0,
    );
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
    scene.draw_rect(
        card,
        Color::TRANSPARENT,
        theme.radius,
        1.5,
        theme.primary.fade(0.9),
    );

    let Some(snapshot) = render_scene(&scene, w, h, theme.background) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        !ghost.is_empty(),
        "the faithful face captures the header's primitives"
    );
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the reflowed table and the ghost card are drawn"
    );
    snapshot.assert_golden(golden("table_reorder_preview"));
}

/// **A table with widget cells (milestone 164)**: a column of **avatars** and one of
/// **chips** (`Chip`), beyond mere text. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "header, avatars and chips are drawn"
    );
    snapshot.assert_golden(golden("table_widget_cells"));
}

/// **A table with adaptive row heights (milestone 166)**: a row whose cell holds a
/// large 48 px avatar grows past the nominal height, while a text row keeps its
/// comfortable height — nothing is cropped. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the tall row and the text row are drawn"
    );
    snapshot.assert_golden(golden("table_adaptive_rows"));
}

/// **Headers with icons (milestone 168)**: a leading icon precedes the column's
/// label, icon then text, and the header stays sortable. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the icon headers and the data are drawn"
    );
    snapshot.assert_golden(golden("table_header_icons"));
}

/// **A header with an action widget (milestone 170)**: a "Filter" button set at the
/// right of a header, clickable on its own, while the rest of the header still sorts —
/// note the ▲ indicator on "Name". Reproduces its golden.
#[test]
fn table_header_action_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(300.0)
        .header(&["Name", "Status"])
        .on_sort(|_| ())
        .sorted(0, true)
        .header_action(1, || {
            Box::new(
                Button::new("Filter")
                    .size(12.0)
                    .variant(Variant::Outlined)
                    .on_press(()),
            )
        })
        .row(&["Ada", "Active"])
        .row(&["Bob", "Away"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 160, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "header, action button and data are drawn"
    );
    snapshot.assert_golden(golden("table_header_action"));
}

/// **A fully widget header (milestone 171)**: the header row is made of arbitrary
/// widgets — here a "User" chip and a hand-rolled "Sort" button — instead of text
/// labels. The behaviour, sorting, is wired by the application. Reproduces its golden.
#[test]
fn table_widget_header_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(300.0)
        .column_widths(&[110.0])
        .widget_header(vec![
            Box::new(|| Box::new(Chip::<()>::new("User"))),
            Box::new(|| {
                Box::new(
                    Button::new("Sort")
                        .size(12.0)
                        .variant(Variant::Outlined)
                        .on_press(()),
                )
            }),
        ])
        .row(&["Ada", "Active"])
        .row(&["Bob", "Away"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 160, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the widget headers and the data are drawn"
    );
    snapshot.assert_golden(golden("table_widget_header"));
}

/// **A column menu (milestone 172)**: a `Menu` dropped in as a header action widget
/// opens a **floating** menu of column actions, rendered over the grid even though it
/// is nested in the header — with no table-specific code. Reproduces its golden.
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
                Menu::new(
                    Button::new("...")
                        .size(12.0)
                        .variant(Variant::Outlined)
                        .on_press(()),
                    true,
                    (),
                )
                .item("Sort ascending", ())
                .item("Sort descending", ())
                .item("Hide column", ()),
            )
        })
        .row(&["Ada", "5"])
        .row(&["Bob", "3"]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 340, 230, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the header and the floating column menu are drawn"
    );
    snapshot.assert_golden(golden("table_column_menu"));
}

/// **A virtualised table (milestone 173)**: 1000 rows, of which only the visible ones
/// are built, the header staying pinned above a scrolling viewport. Reproduces its
/// golden, which is the visible window at the top.
#[test]
fn table_virtualized_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .column_widths(&[60.0])
        .header(&["#", "Item"])
        .virtual_rows(1000, 120.0, |i| {
            vec![format!("{}", i + 1), format!("Item {}", i + 1)]
        });
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 190, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the pinned header and the visible rows are drawn"
    );
    snapshot.assert_golden(golden("table_virtualized"));
}

/// **A virtualised table with widget cells (milestone 176)**: 500 rows of avatars and
/// chips, of which only the visible ones are built, under a pinned header. Reproduces
/// its golden.
#[test]
fn table_virtual_widgets_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(260.0)
        .column_widths(&[70.0])
        .header(&["User", "Tag"])
        .virtual_widget_rows(500, 130.0, |i| {
            vec![
                Box::new(Avatar::new(format!("U{i}")).size(26.0))
                    as Box<dyn frus_widgets::Widget<()>>,
                Box::new(Chip::<()>::new(format!("tag {}", i + 1))),
            ]
        });
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 300, 200, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the header and the virtualised avatars and chips are drawn"
    );
    snapshot.assert_golden(golden("table_virtual_widgets"));
}

/// **A virtualised multi-select table (milestone 177)**: a checkbox column, with a
/// pinned "check all", over virtualised rows, two of them checked. Reproduces its
/// golden.
#[test]
fn table_virtual_checkboxes_matches_golden() {
    let theme = Theme::dark();
    let table = Table::<()>::new(2)
        .width(280.0)
        .column_widths(&[120.0])
        .header(&["Name", "Role"])
        .checkboxes(|_| (), ())
        .selected(&[1, 2])
        .virtual_rows(1000, 130.0, |i| {
            vec![format!("User {}", i + 1), "member".to_string()]
        });
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 320, 200, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "checkboxes, header and virtualised rows are drawn"
    );
    snapshot.assert_golden(golden("table_virtual_checkboxes"));
}

/// **A table with frozen columns (milestone 178)**: the first column, "Name", stays
/// pinned on the left while the quarter columns scroll horizontally, Q3 falling off
/// frame. Reproduces its golden at the initial position, horizontal offset zero.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the frozen column and the scrolling ones are drawn"
    );
    snapshot.assert_golden(golden("table_frozen_columns"));
}

/// **Columns frozen at both edges (milestone 179)**: "Name" pinned left and "Act"
/// pinned right, the middle columns scrolling between them, with a separating shadow at
/// each frozen edge. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the columns frozen at both edges are drawn"
    );
    snapshot.assert_golden(golden("table_frozen_both_edges"));
}

/// **A form error summary (milestone 180)**: after an invalid submission, an
/// error-tinted card lists every message (`Form::errors` → `ErrorSummary`), above the
/// offending field. Reproduces its golden.
#[test]
fn form_error_summary_matches_golden() {
    use frus_widgets::form::{Form, Rule};
    let theme = Theme::dark();
    let report = Form::new()
        .field("email", "nope", Rule::email("Enter a valid email address"))
        .field("password", "x", Rule::min_len(8, "At least 8 characters"));
    let summary = frus_widgets::ErrorSummary::<()>::new(
        report.errors().into_iter().map(|(_, m)| m.to_string()),
    );
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::column().gap(16.0).child(summary).child(
            TextInput::<()>::new("nope")
                .width(300.0)
                .label("Email")
                .error("Enter a valid email address"),
        ),
    );
    let Some(snapshot) = render_widget(&root, 360, 230, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the error summary and the field are drawn"
    );
    snapshot.assert_golden(golden("form_error_summary"));
}

/// **A multi-step form (milestone 182)**: the `Steps` indicator — step 2 of 3 under
/// way, the first complete with a tick — tops the step's content and a Back/Next bar.
/// Reproduces its golden.
#[test]
fn form_wizard_matches_golden() {
    use frus_widgets::Steps;
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::column()
            .gap(18.0)
            .child(Steps::new(["Account", "Profile", "Review"]).current(1))
            .child(Text::styled("Profile", theme.text.title_medium))
            .child(
                TextInput::<()>::new("Ada Lovelace")
                    .width(340.0)
                    .label("Full name"),
            )
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Outlined))
                    .child(Button::new("Next")),
            ),
    );
    let Some(snapshot) = render_widget(&root, 420, 280, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the step indicator, the field and the buttons are drawn"
    );
    snapshot.assert_golden(golden("form_wizard"));
}

/// **A bounded calendar (milestone 231)**: July 2026, with a selectable window of
/// `[10, 20]` — the days outside are disabled, dimmed and unclickable — and the 15th
/// selected. Reproduces its golden.
#[test]
fn date_bounded_matches_golden() {
    use frus_widgets::DatePicker;
    let theme = Theme::dark();
    let picker = DatePicker::bounded(
        2026,
        7,
        Some(15),
        Some((2026, 7, 10)),
        Some((2026, 7, 20)),
        |_| (),
        |_| (),
    );
    let root: Container<()> = Container::new().padding(16.0).child(picker);
    let Some(snapshot) = render_widget(&root, 300, 340, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the bounded calendar is drawn"
    );
    snapshot.assert_golden(golden("date_bounded"));
}

/// **A bounded range calendar (milestone 234)**: July 2026, the 10th to the 15th
/// selected within an allowed window of `[8, 20]` — the endpoints and the days between
/// stand out, and anything outside the window is disabled and dimmed. Reproduces its
/// golden.
#[test]
fn date_range_bounded_matches_golden() {
    use frus_widgets::DatePicker;
    let theme = Theme::dark();
    let picker = DatePicker::range_bounded(
        2026,
        7,
        Some((2026, 7, 10)),
        Some((2026, 7, 15)),
        Some((2026, 7, 8)),
        Some((2026, 7, 20)),
        |_| (),
        |_| (),
    );
    let root: Container<()> = Container::new().padding(16.0).child(picker);
    let Some(snapshot) = render_widget(&root, 300, 340, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the bounded range is drawn");
    snapshot.assert_golden(golden("date_range_bounded"));
}

/// **A filtered, or blacked-out, calendar (milestone 235)**: July 2026 with a few
/// scattered **unavailable** days, chosen by a selectable-day predicate — dimmed and
/// unclickable — and the 21st selected. Reproduces its golden.
#[test]
fn date_blackout_matches_golden() {
    use frus_widgets::DatePicker;
    let theme = Theme::dark();
    let blackout = [
        (2026, 7, 4),
        (2026, 7, 5),
        (2026, 7, 14),
        (2026, 7, 15),
        (2026, 7, 27),
    ];
    let picker = DatePicker::filtered(
        2026,
        7,
        Some(21),
        move |date| !blackout.contains(&date),
        |_| (),
        |_| (),
    );
    let root: Container<()> = Container::new().padding(16.0).child(picker);
    let Some(snapshot) = render_widget(&root, 300, 340, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the calendar with blacked-out days is drawn"
    );
    snapshot.assert_golden(golden("date_blackout"));
}

/// **A calendar in range mode (milestone 184)**: July 2026, the 10th to the 15th —
/// the endpoints as solid pills, the days between as a soft band. Reproduces its
/// golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the calendar and the range are drawn"
    );
    snapshot.assert_golden(golden("date_range"));
}

/// **A dual calendar (milestone 186)**: July and August 2026 side by side, the range
/// running from 28 July to 3 August — the range band carries on from one month to the
/// next. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 200,
        "both months and the range are drawn"
    );
    snapshot.assert_golden(golden("date_range_dual"));
}

/// **A time range (milestone 187)**: two time pickers labelled "Start" (09:00) and
/// "End" (17:30), with minutes in steps of 15. Reproduces its golden.
#[test]
fn time_range_matches_golden() {
    use frus_widgets::TimeRange;
    let theme = Theme::dark();
    let range = TimeRange::new((9, 0), (17, 30), |_, _, _| ()).minute_step(15);
    let root: Container<()> = Container::new().padding(16.0).child(range);
    let Some(snapshot) = render_widget(&root, 540, 420, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 200, "both time pickers are drawn");
    snapshot.assert_golden(golden("time_range"));
}

/// **A notification layer (milestone 188)**: two toasts stacked in the bottom-right
/// corner (`ToastHost`), the second carrying an "Undo" action. Reproduces its golden.
#[test]
fn toast_host_matches_golden() {
    use frus_widgets::{Toast, ToastHost, ToastPosition};
    let theme = Theme::dark();
    let host: ToastHost<()> = ToastHost::new(ToastPosition::BottomEnd)
        .toast(Toast::new("File uploaded").success())
        .toast(Toast::new("Message archived").action("Undo", ()));
    let Some(snapshot) = render_widget(&host, 420, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 60, "the stacked toasts are drawn");
    snapshot.assert_golden(golden("toast_host"));
}

/// **A date-and-time range (milestone 189)**: a dual calendar (28/07 → 03/08), a time
/// range (09:00 → 17:30) and a "start → end" summary. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 300,
        "the dual calendar, the times and the summary are drawn"
    );
    snapshot.assert_golden(golden("datetime_range"));
}

/// **The integrated wizard, Review step with errors (milestone 190)**: the `Steps`
/// indicator with Review current, a **clickable** error summary
/// (`ErrorSummary::links`), and the Back / Create bar — the demo's wizard screen as it
/// really assembles. Reproduces its golden.
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
            .child(
                Steps::new(["Account", "Security", "Review"])
                    .current(2)
                    .on_tap(|_| ()),
            )
            .child(summary)
            .child(Text::new("Creating account for Ada <not-an-email>").size(16.0))
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Outlined))
                    .child(Button::new("Create account")),
            ),
    );
    let Some(snapshot) = render_widget(&root, 480, 380, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the wizard, on the Review step with errors, is drawn"
    );
    snapshot.assert_golden(golden("wizard_review_errors"));
}

/// **A disabled button (milestone 191)**: an active "Next", accented and shadowed,
/// beside its disabled version, greyed and shadowless. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 40, "both buttons are drawn");
    snapshot.assert_golden(golden("button_disabled"));
}

/// **The wizard's Security step (milestone 192)**: `Steps` with Security current, two
/// **obscured** passwords (`TextInput::obscure`), and "Next" **disabled**, the
/// confirmation not matching. Reproduces its golden.
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
                    .child(
                        TextInput::<()>::new("secret12")
                            .width(340.0)
                            .label("Password")
                            .obscure(true),
                    )
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
                    .child(Button::new("Back").variant(Variant::Outlined))
                    .child(Button::new("Next").enabled(false)),
            ),
    );
    let Some(snapshot) = render_widget(&root, 440, 420, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the password step is drawn");
    snapshot.assert_golden(golden("wizard_password_step"));
}

/// **The wizard with passwords revealed, and steps marked by validity (milestones
/// 194–195)**: the "Hide password" toggle unmasks the fields (`obscure(false)`), and
/// the Account step is marked **complete**, with a tick, through `Steps::completed` —
/// not merely by position. Reproduces its golden.
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
                    .child(
                        TextInput::<()>::new("secret12")
                            .width(340.0)
                            .label("Password"),
                    )
                    .child(
                        TextInput::<()>::new("secret12")
                            .width(340.0)
                            .label("Confirm password"),
                    )
                    .child(Button::new("Hide password").variant(Variant::Outlined)),
            )
            .child(
                Flex::row()
                    .gap(12.0)
                    .child(Button::new("Back").variant(Variant::Outlined))
                    .child(Button::new("Next")),
            ),
    );
    let Some(snapshot) = render_widget(&root, 440, 460, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the revealed step is drawn");
    snapshot.assert_golden(golden("wizard_password_revealed"));
}

/// **An inline-editable table (milestone 196)**: a grid in which every cell is a
/// widget (`Table::widget_row`) — static **clickable** cells, each a `Container` that
/// emits "edit this cell", and one cell **being edited**, rendered by a `TextInput`.
/// Proof that inline editing composes with no new mechanism. Reproduces its golden.
#[test]
fn table_editable_matches_golden() {
    use frus_widgets::Table;
    let theme = Theme::dark();
    // A factory for a static clickable cell; clicking it puts the cell into editing.
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
    // A factory for a cell **being edited**: an input field bound to the value.
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
        .widget_row(vec![
            cell("Ada Lovelace"),
            cell("Engineer"),
            cell("ada@example.com"),
        ])
        .widget_row(vec![
            cell("Alan Turing"),
            editing("Cryptographer"),
            cell("alan@example.com"),
        ])
        .widget_row(vec![
            cell("Grace Hopper"),
            cell("Admiral"),
            cell("grace@example.com"),
        ]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 560, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 150, "the editable grid is drawn");
    snapshot.assert_golden(golden("table_editable"));
}

/// **A self-sorting DataTable (milestone 232)**: a text table that **sorts its own
/// rows** from the `sorted(column, direction)` state — here by "Score",
/// **descending**, with a numeric-aware comparison and the direction indicator on the
/// header. Reproduces its golden.
#[test]
fn data_table_sorted_matches_golden() {
    use frus_widgets::DataTable;
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "9".to_string(), "London".to_string()],
        vec!["Bob".to_string(), "12".to_string(), "Paris".to_string()],
        vec!["Carol".to_string(), "2".to_string(), "Berlin".to_string()],
        vec!["Dan".to_string(), "10".to_string(), "Rome".to_string()],
    ];
    let table = DataTable::<()>::new(["Name", "Score", "City"], rows)
        .column_widths(&[150.0, 110.0, 150.0])
        .sorted(1, false)
        .on_sort(|_| ());
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 480, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the sorted DataTable is drawn"
    );
    snapshot.assert_golden(golden("data_table_sorted"));
}

/// **A paginated DataTable (milestones 233/236)**: seven rows sorted by "Score"
/// descending, in pages of **3** — page 1, three rows, under an "N–M of T" footer plus
/// [`Pagination`] and a page-size selector (3/5/10). Reproduces its golden.
#[test]
fn data_table_paginated_matches_golden() {
    use frus_widgets::DataTable;
    let theme = Theme::dark();
    let rows: Vec<Vec<String>> = [
        ("Ada", 9),
        ("Bob", 12),
        ("Carol", 2),
        ("Dan", 10),
        ("Eve", 6),
        ("Finn", 15),
        ("Gwen", 4),
    ]
    .iter()
    .map(|(n, s)| vec![n.to_string(), s.to_string()])
    .collect();
    let table = DataTable::<()>::new(["Name", "Score"], rows)
        .column_widths(&[160.0, 120.0])
        .sorted(1, false)
        .paginated(1, 3, |_| ())
        .page_sizes(&[3, 5, 10], |_| ());
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 560, 260, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the paginated DataTable is drawn"
    );
    snapshot.assert_golden(golden("data_table_paginated"));
}

/// **A DataTable with a selected row (milestone 239)**: the table sorted by "Score"
/// **descending**, with one **source row** marked `selected` — highlighted at its
/// **sorted position**, not at its original index. Proof of the source-index ↔
/// displayed-position translation. Reproduces its golden.
#[test]
fn data_table_selected_matches_golden() {
    use frus_widgets::DataTable;
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "9".to_string(), "London".to_string()],
        vec!["Bob".to_string(), "12".to_string(), "Paris".to_string()],
        vec!["Carol".to_string(), "2".to_string(), "Berlin".to_string()],
        vec!["Dan".to_string(), "10".to_string(), "Rome".to_string()],
    ];
    // Sorting by score descending gives [Bob 12, Dan 10, Ada 9, Carol 2]. **Source**
    // row 3 (Dan) must appear highlighted in the **second** displayed position.
    let table = DataTable::<()>::new(["Name", "Score", "City"], rows)
        .column_widths(&[150.0, 110.0, 150.0])
        .sorted(1, false)
        .on_sort(|_| ())
        .on_select_row(|_| ())
        .selected(&[3]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 480, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the DataTable with a selected row is drawn"
    );
    snapshot.assert_golden(golden("data_table_selected"));
}

/// **A DataTable with a custom sort (milestone 240)**: a "Priority" column sorted
/// **ascending** by a hand-written comparator (`Low < Medium < High`). The default text
/// sort would order them `High, Low, Medium`, alphabetically — here the displayed order
/// really is `Low, Medium, High`. Reproduces its golden.
#[test]
fn data_table_custom_sort_matches_golden() {
    use frus_widgets::DataTable;
    use std::cmp::Ordering;
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "High".to_string()],
        vec!["Bob".to_string(), "Low".to_string()],
        vec!["Carol".to_string(), "Medium".to_string()],
        vec!["Dan".to_string(), "High".to_string()],
    ];
    let rank = |s: &str| match s {
        "Low" => 0,
        "Medium" => 1,
        "High" => 2,
        _ => 3,
    };
    let table = DataTable::<()>::new(["Name", "Priority"], rows)
        .column_widths(&[170.0, 150.0])
        .sorted(1, true)
        .sort_with(1, move |a, b| rank(a).cmp(&rank(b)).then(Ordering::Equal))
        .on_sort(|_| ());
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 420, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the DataTable with a custom sort is drawn"
    );
    snapshot.assert_golden(golden("data_table_custom_sort"));
}

/// **A multi-select DataTable (milestone 241)**: a checkbox column topped by a "check
/// all". The table is sorted by "Score" descending, `[Bob, Dan, Ada, Carol]`, and
/// **source** rows 0 (Ada) and 3 (Dan) are checked → two boxes ticked, at their sorted
/// positions, and the header box **indeterminate**, 2 of 4. Reproduces its golden.
#[test]
fn data_table_checkboxes_matches_golden() {
    use frus_widgets::DataTable;
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "9".to_string(), "London".to_string()],
        vec!["Bob".to_string(), "12".to_string(), "Paris".to_string()],
        vec!["Carol".to_string(), "2".to_string(), "Berlin".to_string()],
        vec!["Dan".to_string(), "10".to_string(), "Rome".to_string()],
    ];
    let table = DataTable::<()>::new(["Name", "Score", "City"], rows)
        .column_widths(&[150.0, 110.0, 150.0])
        .sorted(1, false)
        .on_sort(|_| ())
        .checkboxes(|_| (), ())
        .selected(&[0, 3]);
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 500, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the DataTable with checkboxes is drawn"
    );
    snapshot.assert_golden(golden("data_table_checkboxes"));
}

/// **A searchable DataTable (milestone 242)**: a search field tops the table, whose
/// source rows are **filtered** — a case-insensitive substring across every column —
/// before sorting. The query "ar" keeps only `Bob (Paris)` and `Carol (Berlin)` out of
/// four. Reproduces its golden.
#[test]
fn data_table_search_matches_golden() {
    use frus_widgets::DataTable;
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "London".to_string()],
        vec!["Bob".to_string(), "Paris".to_string()],
        vec!["Carol".to_string(), "Berlin".to_string()],
        vec!["Dan".to_string(), "Rome".to_string()],
    ];
    let table = DataTable::<()>::new(["Name", "City"], rows)
        .column_widths(&[150.0, 150.0])
        .searchable("ar", |_| ())
        .sorted(0, true)
        .on_sort(|_| ());
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 400, 220, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the searchable DataTable is drawn"
    );
    snapshot.assert_golden(golden("data_table_search"));
}

/// **A DataTable with bulk actions (milestone 243)**: when rows are checked a bar tops
/// the table — "N selected" plus the action buttons the application supplies, here a
/// secondary `Clear` and a danger `Delete`. Two rows selected gives "2 selected".
/// Reproduces its golden.
#[test]
fn data_table_bulk_actions_matches_golden() {
    use frus_widgets::{Button, DataTable, Variant};
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "9".to_string()],
        vec!["Bob".to_string(), "12".to_string()],
        vec!["Carol".to_string(), "2".to_string()],
        vec!["Dan".to_string(), "10".to_string()],
    ];
    let table = DataTable::<()>::new(["Name", "Score"], rows)
        .column_widths(&[160.0, 120.0])
        .checkboxes(|_| (), ())
        .selected(&[0, 3])
        .bulk_actions(|| {
            vec![
                Box::new(Button::new("Clear").variant(Variant::Outlined).size(14.0)),
                Box::new(Button::new("Delete").variant(Variant::Danger).size(14.0)),
            ]
        });
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 460, 250, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the DataTable with an action bar is drawn"
    );
    snapshot.assert_golden(golden("data_table_bulk_actions"));
}

/// **A DataTable's empty state (milestone 244)**: a search field whose "zzz" query
/// matches no row → under the header, a centred **empty-state** message, overridden
/// here, replaces the body, **without** a pagination footer. Reproduces its golden.
#[test]
fn data_table_empty_matches_golden() {
    use frus_widgets::DataTable;
    let theme = Theme::dark();
    let rows = vec![
        vec!["Ada".to_string(), "London".to_string()],
        vec!["Bob".to_string(), "Paris".to_string()],
    ];
    let table = DataTable::<()>::new(["Name", "City"], rows)
        .column_widths(&[150.0, 150.0])
        .searchable("zzz", |_| ())
        .paginated(1, 5, |_| ())
        .empty_text("No people match your search");
    let root: Container<()> = Container::new().padding(16.0).child(table);
    let Some(snapshot) = render_widget(&root, 400, 200, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 80,
        "the DataTable's empty state is drawn"
    );
    snapshot.assert_golden(golden("data_table_empty"));
}

/// **A selectable Tree (milestone 246)**: an expanded file tree — chevrons (▾/▸),
/// indentation, vertical **guide lines** back to the ancestors, and one **selected**
/// node highlighted (`button.rs`). Reproduces its golden.
#[test]
fn tree_selected_matches_golden() {
    use frus_widgets::Tree;
    let theme = Theme::dark();
    let tree = Tree::<()>::new(|_| ())
        .on_select(|_| ())
        .selected(Some(3))
        .node(1, 0, "src", true, true)
        .node(2, 1, "widgets", true, true)
        .node(3, 2, "button.rs", false, false)
        .node(4, 2, "grid.rs", false, false)
        .node(5, 1, "main.rs", false, false)
        .node(6, 0, "Cargo.toml", false, false);
    let root: Container<()> = Container::new().padding(16.0).child(tree);
    let Some(snapshot) = render_widget(&root, 320, 240, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the tree is drawn");
    snapshot.assert_golden(golden("tree_selected"));
}

/// **A Kanban board (milestone 247)**: three titled columns of cards
/// (`To do`/`Doing`/`Done`), each on a themed panel, with a **drop zone** at the bottom
/// of every column. Cross-column drag and drop is wired through the reorder mechanism;
/// this golden pins down the **layout**.
#[test]
fn kanban_matches_golden() {
    use frus_widgets::Kanban;
    let theme = Theme::dark();
    let board = Kanban::<()>::new(|_, _, _, _| ())
        .column("To do", ["Design API", "Write spec"])
        .column("Doing", ["Build widget"])
        .column("Done", ["Kickoff", "Research"]);
    let root: Container<()> = Container::new().padding(16.0).child(board);
    let Some(snapshot) = render_widget(&root, 760, 280, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 150, "the Kanban board is drawn");
    snapshot.assert_golden(golden("kanban"));
}

/// **A Kanban board with rich cards (milestone 249)**: **widget** cards, each a label
/// plus a × delete button, and an **"+ Add card"** button at the bottom of every
/// column. Reproduces its golden.
#[test]
fn kanban_rich_matches_golden() {
    use frus_widgets::{Align, Button, Flex, Kanban, Text, Variant};
    let theme = Theme::dark();
    // A factory for a rich card: the label on the left, the × button on the right.
    fn rich(label: &'static str) -> Box<dyn Fn() -> Box<dyn frus_widgets::Widget<()>>> {
        Box::new(move || {
            Box::new(
                Flex::row()
                    .align(Align::Center)
                    .gap(8.0)
                    .child(Text::new(label).size(14.0))
                    .child(Flex::row().flex(1.0))
                    .child(Button::new("×").variant(Variant::Outlined).size(13.0)),
            ) as Box<dyn frus_widgets::Widget<()>>
        })
    }
    let board = Kanban::<()>::new(|_, _, _, _| ())
        .on_add(|_| ())
        .column_widgets("To do", [rich("Design API"), rich("Write spec")])
        .column_widgets("Doing", [rich("Build widget")]);
    let root: Container<()> = Container::new().padding(16.0).child(board);
    let Some(snapshot) = render_widget(&root, 540, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 150,
        "the Kanban board with rich cards is drawn"
    );
    snapshot.assert_golden(golden("kanban_rich"));
}

/// **A bar chart (milestone 199)**: a `(day, value)` series as bars scaled to the
/// maximum, values above, labels below, and a baseline. Reproduces its golden.
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
    // `BarChart` fills the width (Percent), so the parent must have a **definite** width.
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(240.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the bar chart is drawn");
    snapshot.assert_golden(golden("bar_chart"));
}

/// **A grouped multi-series BarChart (milestone 212)**: two named series, their bars
/// grouped side by side per category, sharing a scale and an axis (`grid(4)`), with a
/// legend. Reproduces its golden.
#[test]
fn bar_chart_grouped_matches_golden() {
    use frus_widgets::BarChart;
    let theme = Theme::dark();
    let chart = BarChart::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("This year")
    .series(
        "Last year",
        Color::rgb8(220, 120, 80),
        [2.0, 5.0, 6.0, 4.0, 3.0],
    )
    .legend(true);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the grouped bars are drawn");
    snapshot.assert_golden(golden("bar_chart_grouped"));
}

/// **A stacked multi-series BarChart (milestone 216)**: two series accumulated into a
/// single bar per category (`stacked(true)`), the scale set by the total, with an axis
/// (`grid(4)`) and a legend. Reproduces its golden.
#[test]
fn bar_chart_stacked_matches_golden() {
    use frus_widgets::BarChart;
    let theme = Theme::dark();
    let chart = BarChart::<()>::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("This year")
    .series(
        "Last year",
        Color::rgb8(220, 120, 80),
        [2.0, 5.0, 6.0, 4.0, 3.0],
    )
    .legend(true)
    .stacked(true);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the stacked bars are drawn");
    snapshot.assert_golden(golden("bar_chart_stacked"));
}

/// **A line chart (milestone 200)**: the same `(day, value)` series as the BarChart,
/// but drawn as a polyline — segments plus round markers — values above, labels below,
/// and a baseline. Reproduces its golden.
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
    // `LineChart` fills the width (Percent), so the parent must have a **definite** width.
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(240.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the line chart is drawn");
    snapshot.assert_golden(golden("line_chart"));
}

/// **A line chart with an axis (milestone 203)**: the same series as `line_chart`, but
/// with a y axis of 4 divisions (`grid(4)`) — horizontal grid lines and `0..max` ticks
/// in a left margin, shared with the BarChart. Reproduces its golden.
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
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(240.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the chart with an axis is drawn"
    );
    snapshot.assert_golden(golden("line_chart_axis"));
}

/// **A line chart with an area (milestone 206)**: the same series, with the area under
/// the curve filled (`area(true)`) and the y axis (`grid(4)`). Reproduces its golden.
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
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(240.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 260, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the chart with an area is drawn"
    );
    snapshot.assert_golden(golden("line_chart_area"));
}

/// **A multi-series chart with a legend (milestone 209)**: two named series sharing
/// the same categories and scale, drawn in their own colours, with an axis (`grid(4)`)
/// and a legend of swatch plus name. Reproduces its golden.
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
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the multi-series chart is drawn"
    );
    snapshot.assert_golden(golden("line_chart_multi"));
}

/// **Stacked areas (milestone 213)**: two accumulated series (`stacked(true)`) — each
/// band adds on top of the previous one and the scale holds the total — with an axis
/// (`grid(4)`) and a legend. Reproduces its golden.
#[test]
fn line_chart_stacked_matches_golden() {
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
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true)
    .stacked(true);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the stacked areas are drawn");
    snapshot.assert_golden(golden("line_chart_stacked"));
}

/// **A legend with a hidden series (milestone 215)**: two series, the second hidden
/// (`hidden([1])`) — not drawn, and dimmed in the legend, which the app can still make
/// clickable. Reproduces its golden.
#[test]
fn line_chart_hidden_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::<()>::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("Sales")
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true)
    .hidden([1]);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "a hidden series leaves one curve"
    );
    snapshot.assert_golden(golden("line_chart_hidden"));
}

/// **A pinned point (milestone 223)**: two line series, the `(Thu, Sales)` point
/// **selected** (`selected(Some((3, 0)))`) receiving a persistent halo and accent ring,
/// with no hover involved. Reproduces its golden.
#[test]
fn line_chart_selected_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::<()>::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("Sales")
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true)
    .selected(Some((3, 0)));
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the pinned point stands out");
    snapshot.assert_golden(golden("line_chart_selected"));
}

/// **A pinned bar (milestone 223)**: two grouped bar series, the `(Thu, Sales)` bar
/// **selected** (`selected(Some((3, 0)))`) receiving a persistent accent ring.
/// Reproduces its golden.
#[test]
fn bar_chart_selected_matches_golden() {
    let theme = Theme::dark();
    let chart = BarChart::<()>::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("Sales")
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true)
    .selected(Some((3, 0)));
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the pinned bar stands out");
    snapshot.assert_golden(golden("bar_chart_selected"));
}

/// **100% stacked bars (milestone 224)**: `stacked(true).normalized(true)` — every
/// column fills the full height, each layer taking its share, and the axis reads in
/// percentages. Reproduces its golden.
#[test]
fn bar_chart_normalized_matches_golden() {
    let theme = Theme::dark();
    let chart = BarChart::<()>::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("Sales")
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true)
    .stacked(true)
    .normalized(true);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the 100% columns are drawn");
    snapshot.assert_golden(golden("bar_chart_normalized"));
}

/// **100% stacked areas (milestone 224)**: `stacked(true).normalized(true)` — every
/// category fills the full height, each band taking its share, and the axis reads in
/// percentages. Reproduces its golden.
#[test]
fn line_chart_normalized_matches_golden() {
    let theme = Theme::dark();
    let chart = LineChart::<()>::new([
        ("Mon", 3.0),
        ("Tue", 7.0),
        ("Wed", 5.0),
        ("Thu", 8.0),
        ("Fri", 4.0),
    ])
    .height(200.0)
    .grid(4)
    .name("Sales")
    .series(
        "Costs",
        Color::rgb8(220, 120, 80),
        [2.0, 4.0, 3.0, 5.0, 2.0],
    )
    .legend(true)
    .stacked(true)
    .normalized(true);
    let root: Container<()> = Container::new()
        .width(360.0)
        .height(260.0)
        .padding(20.0)
        .child(chart);
    let Some(snapshot) = render_widget(&root, 400, 300, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 100, "the 100% areas are drawn");
    snapshot.assert_golden(golden("line_chart_normalized"));
}

/// **A password field with an eye (milestone 202)**: an **obscured** `TextInput`
/// carrying the suffix eye icon (`on_suffix`) that reveals the text. Reproduces its
/// golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 60,
        "the obscured field and the eye are drawn"
    );
    snapshot.assert_golden(golden("password_eye"));
}

/// **A field with a clickable suffix (milestone 198)**: a filled `TextInput` carrying
/// a clickable "✕" suffix icon (`on_suffix`) that clears it. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 60,
        "the field and the suffix are drawn"
    );
    snapshot.assert_golden(golden("textinput_clear"));
}

/// **A snackbar with an action (milestone 185)**: a transient notification carrying an
/// "Undo" button on the right, in the Material manner. Reproduces its golden.
#[test]
fn snackbar_action_matches_golden() {
    use frus_widgets::Toast;
    let theme = Theme::dark();
    let toast = Toast::new("Message archived").action("Undo", ());
    let root: Container<()> = Container::new().padding(20.0).child(toast);
    let Some(snapshot) = render_widget(&root, 340, 90, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 60,
        "the card, the text and the action are drawn"
    );
    snapshot.assert_golden(golden("snackbar_action"));
}

/// **A resizable table (milestone 151)**: fixed-width columns with a thin vertical
/// handle at each column's right edge, the last one excepted. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "header, rows and handles are drawn"
    );
    snapshot.assert_golden(golden("data_table_resizable"));
}

/// **A time picker (milestone 146)**: an `HH:MM` preview, a grid of hours (0–23) and
/// one of minutes in steps of 5, the selected cell highlighted. Reproduces its
/// golden.
#[test]
fn time_picker_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> =
        Container::new()
            .padding(20.0)
            .child(TimePicker::<()>::new(9, 30, |_| (), |_| ()));
    let Some(snapshot) = render_widget(&root, 280, 400, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the preview, the grids and the cells are drawn"
    );
    snapshot.assert_golden(golden("time_picker"));
}

/// **A 12-hour time picker (milestone 147)**: an AM/PM toggle plus a 1–12 grid, with
/// a `3:05 PM` preview.
#[test]
fn time_picker_12h_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new()
        .padding(20.0)
        .child(TimePicker::<()>::new(15, 5, |_| (), |_| ()).hour12());
    let Some(snapshot) = render_widget(&root, 280, 420, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 100,
        "the preview, AM/PM and the grids are drawn"
    );
    snapshot.assert_golden(golden("time_picker_12h"));
}

/// **A date-and-time flow (milestone 147)**: a calendar plus a time picker, topped by
/// a summary of the selection. Reproduces its golden.
#[test]
fn date_time_picker_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new()
        .padding(20.0)
        .child(DateTimePicker::<()>::new(
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 200,
        "the summary, the calendar and the time are drawn"
    );
    snapshot.assert_golden(golden("date_time_picker"));
}

/// **An open dropdown (milestone 150)**: a header plus a floating menu, the selected
/// option highlighted and ticked. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 80,
        "the header, the options, the highlight and the tick are drawn"
    );
    snapshot.assert_golden(golden("dropdown_menu"));
}

/// **Autocomplete (milestone 152)**: an "ap" field and a floating list; the matching
/// portion, "ap", stands out in every suggestion and the second, the active one, is
/// highlighted. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 80,
        "the field, the suggestions, the highlight and the emphasis are drawn"
    );
    snapshot.assert_golden(golden("autocomplete"));
}

/// **A range slider (milestone 156)**: two handles bounding an interval, with the
/// active segment tinted `primary` between them. Reproduces its golden.
#[test]
fn range_slider_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new()
        .padding(24.0)
        .child(RangeSlider::<()>::new(0.3, 0.7).width(240.0));
    let Some(snapshot) = render_widget(&root, 300, 80, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 40,
        "the track, the active segment and both handles are drawn"
    );
    snapshot.assert_golden(golden("range_slider"));
}

/// **A labelled range slider (milestones 160/162)**: the value tooltip appears only on
/// **hover or focus** of a handle. Here the lower handle is focused, so its "30%"
/// bubble and its focus ring show. Reproduces its golden.
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
    // The lower handle: centre x = 24 + 0.3·240 = 96, within the lower track band.
    let probe = Point::new(96.0, 62.0);
    let base = build_ui(
        &root,
        Size::new(w as f32, h as f32),
        &Runtime::default(),
        &theme,
    );
    let id = base
        .draggable_at(probe)
        .map(|(id, _)| id)
        .expect("the lower handle is grabbable");
    // Rebuild with the lower handle **focused**, which reveals the bubble and the ring.
    let mut runtime = Runtime::default();
    runtime.input.focused = Some(id);
    let ui = build_ui(&root, Size::new(w as f32, h as f32), &runtime, &theme);

    let Some(snapshot) = render_scene(ui.scene(), w, h, theme.background) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 60,
        "the track, the handles and the focused tooltip are drawn"
    );
    snapshot.assert_golden(golden("range_slider_labels"));
}

/// **Scrolling autocomplete (milestone 154)**: a list longer than the threshold
/// (`max_visible(3)`) gives a viewport bounded to 3 rows over scrollable content, six
/// suggestions in all. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 80,
        "the field and the bounded list are drawn"
    );
    snapshot.assert_golden(golden("autocomplete_scroll"));
}

/// A **password field** (milestone 133): the value masked by dots, with a prefix icon
/// on the left and a suffix icon on the right. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 80,
        "the dots, the icons and the text are drawn"
    );
    snapshot.assert_golden(golden("password_field"));
}

/// **End to end (milestone 135)**: a `Form` validates the entered values and drives
/// each field's `error(...)` — the rendering shows the sign-up form *after an invalid
/// submission*. Reproduces its golden.
#[test]
fn validated_signup_form_matches_golden() {
    use frus_widgets::form::{Form, Rule};

    // What the user would have typed before submitting.
    let (email, password) = ("ada", "short");
    let report = Form::new()
        .field(
            "email",
            email,
            Rule::all([
                Rule::required("Required"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field(
            "password",
            password,
            Rule::min_len(8, "At least 8 characters"),
        );

    // The report's errors feed the fields directly.
    let mut email_field = TextInput::<()>::new(email).width(280.0).label("Email");
    if let Some(e) = report.error("email") {
        email_field = email_field.error(e);
    }
    let mut password_field = TextInput::<()>::new(password)
        .width(280.0)
        .label("Password")
        .obscure(true);
    if let Some(e) = report.error("password") {
        password_field = password_field.error(e);
    }

    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(20.0).child(
        Flex::column()
            .gap(14.0)
            .child(email_field)
            .child(password_field),
    );
    let Some(snapshot) = render_widget(&root, 340, 210, &theme) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(!report.is_valid(), "both fields are invalid");
    assert_eq!(
        report.first_invalid(),
        Some("email"),
        "the first one to focus"
    );
    snapshot.assert_golden(golden("validated_signup_form"));
}

/// A **multi-line field** (milestone 137): a floating label and several lines of
/// content, with explicit breaks, in a box `rows` lines tall. Reproduces its golden.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(
        snapshot.lit_pixels(40) > 120,
        "the label and three lines of text are drawn"
    );
    snapshot.assert_golden(golden("multiline_field"));
}

/// The **inspector** overlay — outlines, highlight, and a card for the widget being
/// pointed at — over a rendered tree. Reproduces its golden.
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
    assert!(
        nodes.len() >= 4,
        "the whole tree is observed ({})",
        nodes.len()
    );

    let mut scene = ui.scene().clone();
    // The pointer designates the first text: highlight plus card.
    paint_inspector_overlay(
        &nodes,
        Some(Point::new(20.0, 18.0)),
        size,
        &theme,
        &mut scene,
    );
    let Some(snapshot) = frus_test::render_scene(&scene, 180, 120, theme.background) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    snapshot.assert_golden(golden("inspector_overlay"));
}

/// **RTL**: the same [red][green][blue] row flips horizontally — red, the first
/// child, moves to the right. Font-independent proof of the layout mirroring.
#[test]
fn rtl_mirrors_the_row() {
    let red = Color::rgb(0.9, 0.2, 0.2);
    let blue = Color::rgb(0.2, 0.4, 0.9);
    let make = || {
        Flex::<()>::row()
            .width(150.0)
            .height(40.0)
            .child(Container::new().width(50.0).height(40.0).color(red))
            .child(
                Container::new()
                    .width(50.0)
                    .height(40.0)
                    .color(Color::rgb(0.2, 0.8, 0.4)),
            )
            .child(Container::new().width(50.0).height(40.0).color(blue))
    };
    // LTR: red on the left, blue on the right.
    let ltr_theme = Theme::dark();
    let rtl_theme = Theme::dark().rtl();
    let (Some(ltr), Some(rtl)) = (
        render_widget(&make(), 150, 40, &ltr_theme),
        render_widget(&make(), 150, 40, &rtl_theme),
    ) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let is_red = |px: [u8; 4]| px[0] > 180 && px[1] < 120;
    let is_blue = |px: [u8; 4]| px[2] > 180 && px[0] < 120;
    // LTR: red at the left edge, blue at the right one.
    assert!(
        is_red(ltr.pixel(10, 20)) && is_blue(ltr.pixel(140, 20)),
        "LTR, unflipped"
    );
    // RTL mirrors it: red on the right, blue on the left.
    assert!(
        is_red(rtl.pixel(140, 20)) && is_blue(rtl.pixel(10, 20)),
        "RTL, flipped"
    );
    rtl.assert_golden(golden("rtl_row"));
}

/// **RTL**: an edge drawer (`end_drawer`, the *end* side being the right under LTR)
/// moves to the **left** under RTL — overlay placement follows the direction.
#[test]
fn rtl_flips_the_drawer_side() {
    use frus_widgets::Scaffold;
    let drawer_color = Color::rgb(0.9, 0.3, 0.3);
    // The window is **wider than a drawer panel**, deliberately. It used to be 200 px
    // against a panel of 280, and the assertions below used to read the other way round
    // — LTR on the left — and passed, because a panel anchored to the right edge of a
    // window narrower than itself starts at a negative x, and the strip of its content
    // that survived on screen was the *left* edge. The test was inverted and the
    // overflow was hiding it. Widening the window removes the artefact; the panel now
    // sits where it says it does (milestone 307).
    const W: u32 = 400;
    let make = || {
        Scaffold::<()>::new(W as f32, 120.0)
            .body(
                Container::new()
                    .width(W as f32)
                    .height(120.0)
                    .color(Color::rgb(0.1, 0.1, 0.12)),
            )
            .end_drawer(
                // As wide as the panel it fills: a narrower block would only say where
                // the panel's *leading* edge is, which is not what is being asked.
                Container::new()
                    .width(frus_widgets::DRAWER_WIDTH)
                    .height(120.0)
                    .color(drawer_color),
                true,
                (),
            )
            .build()
    };
    let is_drawer = |px: [u8; 4]| px[0] > 180 && px[1] < 120 && px[2] < 120;
    let (Some(ltr), Some(rtl)) = (
        render_widget(make().as_ref(), W, 120, &Theme::dark()),
        render_widget(make().as_ref(), W, 120, &Theme::dark().rtl()),
    ) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    // Where the drawer's own colour actually is, rather than two hopeful pixel probes:
    // the middle of the run of drawer-coloured pixels across the window's waist.
    let centre = |s: &frus_test::Snapshot| {
        let xs: Vec<u32> = (0..W).filter(|x| is_drawer(s.pixel(*x, 60))).collect();
        assert!(!xs.is_empty(), "the drawer is on screen at all");
        xs.iter().sum::<u32>() / xs.len() as u32
    };
    // The *end* side is the right one under LTR…
    assert!(
        centre(&ltr) > W / 2,
        "LTR: the end drawer is on the right, found at {}",
        centre(&ltr)
    );
    // …and RTL mirrors it to the left.
    assert!(
        centre(&rtl) < W / 2,
        "RTL: the end drawer is on the left, found at {}",
        centre(&rtl)
    );
    rtl.assert_golden(golden("rtl_drawer"));
}

/// **Group opacity** (widget → walk → layer → GPU): a `Container` at `opacity(0.5)`
/// dims its red background relative to the same one at `opacity(1.0)`, which renders
/// opaque with no layer. End-to-end pixel proof of the group fade.
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
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let r_opaque = opaque.pixel(20, 20)[0];
    let r_faded = faded.pixel(20, 20)[0];
    assert!(r_opaque > 230, "opaque → full red: {r_opaque}");
    assert!(
        r_faded < r_opaque - 40,
        "a group opacity of 0.5 dims the red: {r_faded} vs {r_opaque}"
    );
}

/// The comparator: identical gives 0 differences, one changed pixel gives 1.
#[test]
fn diff_count_is_exact() {
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), Color::rgb(0.3, 0.5, 0.7));
    let Some(a) = render_scene(&scene, 64, 64, Color::BLACK) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let mut b = render_scene(&scene, 64, 64, Color::BLACK).unwrap();
    assert_eq!(a.diff_count(&b, 0), 0, "two identical renderings");
    // Corrupt one pixel, beyond the tolerance.
    b.rgba[0] = b.rgba[0].wrapping_add(64);
    assert_eq!(a.diff_count(&b, 2), 1);
    assert_eq!(
        a.diff_count(&b, 255),
        0,
        "the maximum tolerance absorbs everything"
    );
}
