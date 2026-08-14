//! Pixel tests of the widgets whose picture is a **gesture in flight**.
//!
//! Milestone 296 gave 75 of the 86 widget modules a golden and left eleven, all for
//! the same reason: what they draw is not a function of their arguments. A swipe
//! half-done, a pull past the top edge, a glow where a list hit its end, a page
//! between two pages — none of that is in the widget. It is in the `Runtime`, and
//! the shell is what puts it there.
//!
//! [`Stage`] puts it there instead, through the same entry points the shell uses,
//! and steps the frame loop the same way. The five that need it are here; the six
//! that only ever needed the right arguments are here too, since they belong with
//! their neighbours.
//!
//! With no GPU adapter the tests skip themselves, the harness returning `None`.

use frus_core::{Color, Size, SizeClass};
use frus_test::{Snapshot, Stage};
use frus_widgets::{
    text, Align, Container, Dismissible, DragTarget, Draggable, Flex, GlowEdge, Hero, Keyed,
    LayoutBuilder, NavScaffold, Navigator, PageView, Refresh, Responsive, Scroll,
};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Compares a frame against `tests/goldens/<name>.png`, refusing an empty one.
fn accept(name: &str, snapshot: Option<Snapshot>) {
    let Some(snapshot) = snapshot else {
        eprintln!("no GPU adapter available: {name} skipped");
        return;
    };
    assert!(snapshot.lit_pixels(48) > 40, "{name}: the frame is empty");
    snapshot.assert_golden(golden(name));
}

/// A coloured box that fills its row, the filler these screens are made of.
fn band(color: Color, label: &str) -> Container<()> {
    Container::new()
        .height(44.0)
        .color(color)
        .radius(6.0)
        .padding(10.0)
        .child(text(label.to_string()).size(14.0))
}

const TEAL: Color = Color::rgb(0.149, 0.651, 0.604);
const AMBER: Color = Color::rgb(1.0, 0.702, 0.0);
const SLATE: Color = Color::rgb(0.157, 0.173, 0.216);

// ---------------------------------------------------------------------------
// The five that need the frame loop
// ---------------------------------------------------------------------------

/// A row swiped a little past its threshold: the item has moved, and the background
/// it was hiding is showing behind it. Neither half of that picture exists in a
/// settled frame, which is why `Dismissible` had no golden until now.
#[test]
fn a_row_swiped_half_way() {
    let row = || -> Dismissible<()> {
        Dismissible::new(band(SLATE, "Swipe me"))
            .background(
                Container::new()
                    .color(Color::rgb8(198, 40, 40))
                    .radius(6.0)
                    .padding(10.0)
                    .alignment(frus_core::Alignment::CENTER_LEFT)
                    .child(text("Delete").size(14.0)),
            )
            .height(44.0)
    };
    let root: Container<()> = Container::new().width(280.0).padding(12.0).child(
        Flex::column()
            .width(256.0)
            .gap(8.0)
            .child(band(SLATE, "Above"))
            .child(row())
            .child(band(SLATE, "Below")),
    );

    let mut stage = Stage::new(280, 160);
    stage.settle(&root);
    // The swipeable is the only one in the frame, so its identity comes straight
    // from the frame rather than from counting nodes.
    let item = stage.build(&root).dismissables().to_vec();
    let Some(item) = item.first().cloned() else {
        panic!("the frame declared no dismissable");
    };
    stage.runtime.dismiss_drag(
        item.id,
        item.extent() * 0.45,
        item.extent(),
        Default::default(),
    );
    stage.advance(&root, 1.0 / 60.0);
    accept("swiped_half_way", stage.render(&root));
}

/// A list pulled past its top edge, the indicator out and part-way round. The pull
/// is a number in the runtime; the widget draws whatever that number says.
#[test]
fn a_list_pulled_past_its_top() {
    let mut rows = Flex::column().gap(6.0);
    for i in 0..8 {
        rows = rows.child(band(SLATE, &format!("Row {i}")));
    }
    let root: Container<()> = Container::new()
        .width(260.0)
        .padding(10.0)
        .child(Refresh::new(Scroll::new().width(240.0).height(160.0).child(rows)).on_refresh(()));

    let mut stage = Stage::new(260, 190);
    stage.settle(&root);
    let area = stage.build(&root).refresh_areas().to_vec();
    let Some(area) = area.first().cloned() else {
        panic!("the frame declared no refreshable area");
    };
    // Pulled 70 px past the top of a 160 px viewport: beyond the threshold, so the
    // indicator is armed rather than merely peeking.
    stage
        .runtime
        .refresh_pull(area.id, 70.0, area.viewport.height);
    stage.advance(&root, 1.0 / 60.0);
    accept("pulled_past_the_top", stage.render(&root));
}

/// A list that has been pushed past its end: the glow at the edge it hit. It fades
/// on its own, so the frame is taken one step in, while it is still bright.
#[test]
fn a_list_glowing_at_its_edge() {
    let mut rows = Flex::column().gap(6.0);
    for i in 0..8 {
        rows = rows.child(band(SLATE, &format!("Row {i}")));
    }
    let root: Container<()> = Container::new()
        .width(260.0)
        .padding(10.0)
        .child(Scroll::new().width(240.0).height(150.0).child(rows));

    let mut stage = Stage::new(260, 170);
    stage.settle(&root);
    let region = stage.build(&root).scroll_regions().to_vec();
    let Some(region) = region.first().cloned() else {
        panic!("the frame declared no scrollable region");
    };
    stage
        .runtime
        .glow_pull(region.id, GlowEdge::Top, 90.0, 150.0, 0.0, 240.0);
    stage.advance(&root, 1.0 / 60.0);
    accept("overscroll_glow", stage.render(&root));
}

/// The same glow on a **full-width** viewport, at the proportions of a phone, pulled
/// hard. The narrow one above hides what this shows: the arc spans the whole width
/// and is a flat wash with a hard curved edge, which reads as the page being bent
/// rather than as a glow. Found on a device (2026-08-14).
#[test]
fn a_wide_list_glowing_at_its_top() {
    let mut rows = Flex::column().gap(8.0);
    for i in 0..14 {
        rows = rows.child(band(SLATE, &format!("Row {i}")));
    }
    let root: Container<()> = Container::new()
        .width(424.0)
        .child(Scroll::new().width(424.0).height(600.0).child(rows));

    let mut stage = Stage::new(424, 600);
    stage.settle(&root);
    let region = stage.build(&root).scroll_regions().to_vec();
    let Some(region) = region.first().cloned() else {
        panic!("the frame declared no scrollable region");
    };
    // 220 px past the top of a 600 px viewport, the finger near the middle: the pull
    // a thumb gives a list that is already at its top.
    stage
        .runtime
        .glow_pull(region.id, GlowEdge::Top, 220.0, 600.0, 212.0, 424.0);
    stage.advance(&root, 1.0 / 60.0);
    accept("overscroll_glow_wide", stage.render(&root));
}

/// A paged view caught **between** two pages: half of one and half of the next,
/// which is the state a snap animation passes through and the only one worth a
/// picture.
#[test]
fn a_page_view_between_pages() {
    const PAGE: f32 = 220.0;
    let colours = [TEAL, AMBER, Color::rgb(0.361, 0.420, 0.753)];
    let root: Container<()> = Container::new().padding(10.0).child(
        PageView::new(3, move |i| {
            Container::new()
                .flex(1.0)
                .color(colours[i])
                .radius(8.0)
                .padding(12.0)
                .child(text(format!("Page {i}")).size(16.0))
        })
        .width(PAGE)
        .height(120.0),
    );

    let mut stage = Stage::new(240, 140);
    stage.settle(&root);
    let region = stage.build(&root).scroll_regions().to_vec();
    let Some(region) = region.first().cloned() else {
        panic!("the frame declared no scrollable region");
    };
    // Half a page along: the seam between page 0 and page 1 sits mid-viewport.
    stage.runtime.scroll.insert(region.id, (PAGE * 0.5, 0.0));
    accept("page_view_mid_swipe", stage.render(&root));
}

/// A drop zone lit up because something is being dragged over it, beside one that
/// is not. The highlight lives in the runtime's `drag_over`, so a settled frame
/// never shows it.
#[test]
fn a_drop_zone_under_a_drag() {
    // `DragTarget` washes its own box **before** its children paint, so a child with
    // an opaque background would hide the highlight entirely. Outlined, as a drop
    // target is drawn in practice.
    let zone = |label: &str| -> DragTarget<()> {
        DragTarget::new(
            Container::new()
                .width(90.0)
                .height(60.0)
                .border(1.0, SLATE)
                .radius(6.0)
                .padding(8.0)
                .child(text(label.to_string()).size(13.0)),
        )
        .highlight(TEAL)
    };
    let root: Container<()> = Container::new().width(300.0).padding(12.0).child(
        Flex::row()
            .gap(12.0)
            .align(Align::Center)
            .child(Draggable::new(band(AMBER, "Drag")).payload(1))
            .child(zone("Here"))
            .child(zone("Not here")),
    );

    let mut stage = Stage::new(300, 100);
    stage.settle(&root);
    let zones: Vec<_> = (0..300)
        .step_by(4)
        .filter_map(|x| {
            stage
                .build(&root)
                .drop_zone_at(frus_core::Point::new(x as f32, 50.0))
                .map(|z| z.id)
        })
        .collect();
    let first = *zones.first().expect("a drop zone in the frame");
    stage.runtime.drag_over = Some(first);
    accept("drop_zone_highlighted", stage.render(&root));
}

// ---------------------------------------------------------------------------
// The six that only ever needed the right arguments
// ---------------------------------------------------------------------------

/// A navigator with a transition in flight: the outgoing screen sliding off, the
/// incoming one sliding on, both in the frame at once.
#[test]
fn a_navigator_mid_push() {
    let screen = |title: &str, colour: Color| -> Container<()> {
        Container::new()
            .width(260.0)
            .height(140.0)
            .color(colour)
            .padding(16.0)
            .child(text(title.to_string()).size(18.0))
    };
    let root: Navigator<()> = Navigator::new(screen("Second", SLATE), 260.0, 140.0).from(
        screen("First", Color::rgb(0.101, 0.113, 0.145)),
        0.45,
        true,
    );

    let mut stage = Stage::new(260, 140);
    stage.settle(&root);
    accept("navigator_mid_push", stage.render(&root));
}

/// A shared element between the two screens of a flight. Statically this is the tag
/// carrying its child; the flight itself belongs to the navigator, which is the
/// milestone-286 story.
#[test]
fn a_hero_at_rest() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::row()
            .gap(12.0)
            .align(Align::Center)
            .child(Hero::new(
                "avatar",
                Container::new()
                    .width(56.0)
                    .height(56.0)
                    .color(TEAL)
                    .radius(28.0),
            ))
            .child(
                Flex::column()
                    .gap(4.0)
                    .child(text("Shared element").size(15.0))
                    .child(text("tagged \"avatar\"").size(12.0)),
            ),
    );

    let mut stage = Stage::new(260, 90);
    stage.settle(&root);
    accept("hero_at_rest", stage.render(&root));
}

/// Two subtrees under two keys: `Keyed` is what tells the runtime that the thing in
/// this slot has been replaced rather than merely changed, and it must forward the
/// structure of whatever it wraps.
#[test]
fn keyed_subtrees() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(8.0)
            .child(Keyed::new("first", band(TEAL, "Under key \"first\"")))
            .child(Keyed::new("second", band(AMBER, "Under key \"second\""))),
    );

    let mut stage = Stage::new(260, 130);
    stage.settle(&root);
    accept("keyed_subtrees", stage.render(&root));
}

/// The same `Responsive` at the two size classes that matter, side by side: it
/// chooses one of three subtrees, and the choice is the whole widget.
#[test]
fn responsive_picks_by_class() {
    let arm = |label: &str, colour: Color| -> Container<()> {
        Container::new()
            .width(110.0)
            .height(50.0)
            .color(colour)
            .radius(6.0)
            .padding(10.0)
            .child(text(label.to_string()).size(13.0))
    };
    let tree = |class: SizeClass| -> Responsive<()> {
        Responsive::new(class)
            .compact(arm("compact", TEAL))
            .medium(arm("medium", AMBER))
            .expanded(arm("expanded", Color::rgb(0.361, 0.420, 0.753)))
    };
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(8.0)
            .child(tree(SizeClass::Compact))
            .child(tree(SizeClass::Expanded)),
    );

    let mut stage = Stage::new(160, 150);
    stage.settle(&root);
    accept("responsive_by_class", stage.render(&root));
}

/// A builder that reads the box it was given and draws it: the number in the frame
/// is the proof that it received the real one, not the parent's.
#[test]
fn a_layout_builder_reads_its_box() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column().gap(8.0).child(
            LayoutBuilder::new(|size: Size| {
                Container::new()
                    .flex(1.0)
                    .color(SLATE)
                    .radius(6.0)
                    .padding(10.0)
                    .child(text(format!("{:.0} × {:.0}", size.width, size.height)).size(15.0))
            })
            .width(180.0)
            .height(60.0),
        ),
    );

    let mut stage = Stage::new(220, 90);
    stage.settle(&root);
    accept("layout_builder_box", stage.render(&root));
}

/// The navigation scaffold at both presentations: a bar across the bottom when the
/// window is compact, a rail down the side when it is not.
#[test]
fn the_nav_scaffold_both_ways() {
    let scaffold = |class: SizeClass| -> NavScaffold<()> {
        NavScaffold::new(class, 1, |_: usize| ())
            .destination("★", "Home")
            .destination("♥", "Saved")
            .destination("■", "Files")
            .body(
                Container::new()
                    .flex(1.0)
                    .padding(12.0)
                    .child(text("Body").size(15.0)),
            )
    };
    let root: Flex<()> = Flex::row()
        .gap(10.0)
        .child(
            Container::new()
                .width(150.0)
                .height(190.0)
                .child(scaffold(SizeClass::Compact)),
        )
        .child(
            Container::new()
                .width(180.0)
                .height(190.0)
                .child(scaffold(SizeClass::Expanded)),
        );

    let mut stage = Stage::new(350, 200);
    stage.settle(&root);
    accept("nav_scaffold_both_ways", stage.render(&root));
}

/// The stage steps the frame loop rather than jumping to the end: a glow pulled and
/// then left alone fades, and the two frames differ. Without this the harness could
/// set state but never watch it settle, which is half of what it is for.
#[test]
fn the_stage_actually_advances_time() {
    let mut rows = Flex::column().gap(6.0);
    for i in 0..8 {
        rows = rows.child(band(SLATE, &format!("Row {i}")));
    }
    let root: Container<()> =
        Container::new().child(Scroll::new().width(200.0).height(120.0).child(rows));

    let mut stage = Stage::new(200, 120);
    stage.settle(&root);
    let region = stage.build(&root).scroll_regions()[0];
    stage
        .runtime
        .glow_pull(region.id, GlowEdge::Top, 90.0, 120.0, 0.0, 200.0);
    stage.advance(&root, 1.0 / 60.0);
    let Some(bright) = stage.render(&root) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };

    // Half a second of nothing happening: the glow has faded a long way.
    stage.advance_by(&root, 1.0 / 60.0, 30);
    let faded = stage
        .render(&root)
        .expect("the adapter was there a moment ago");

    assert!(
        bright.diff_count(&faded, 2) > 200,
        "the glow did not change over half a second: the loop is not running"
    );
}
