//! A pixel test for the widgets that had none.
//!
//! The golden suite grew where the bugs were — tables, charts, forms, pickers — and by
//! milestone 295 it was 77 images deep there and empty everywhere else: 58 of the 86
//! widget modules had no pixel test at all, `Card`, `Checkbox`, `Switch`, `Icon` and
//! `Divider` among them. That is why a rendering defect had to be found on a phone
//! twice in five milestones — milestone 291's notched bar, and milestone 294's text
//! through every overlay, neither of which the suite could have caught, because
//! nothing in it drew the widgets involved.
//!
//! Widgets are grouped by what they are rather than given one image each: a golden
//! holding the three toggles in every state is a better test than three holding one,
//! and there is less to read when one of them moves.
//!
//! With no GPU adapter the tests skip themselves, the harness returning `None`.

use frus_core::{
    Alignment, BoxFit, Color, ImageData, ImageHandle, Insets, Path, Point, SizeClass, TextSpan,
};
use frus_test::render_widget;
use frus_widgets::{
    text, Alert, Align, AppBar, AspectRatio, Badge, BottomAppBar, BottomBar, BottomSheet,
    Breadcrumb, Card, CarouselView, Checkbox, CheckboxListTile, CircleAvatar,
    CircularProgressIndicator, ClipOval, ClipPath, ClipRRect, ColorPicker, ConstrainedBox,
    Container, ControlAffinity, CustomPaint, Divider, Expanded, ExpansionTile, FittedBox, Flex,
    FloatingActionButton, FontWeight, FractionallySizedBox, GridView, Icon, Icons, Image,
    Intrinsic, Kbd, LinearProgressIndicator, ListTile, ListView, MenuAnchor, NavigationBar,
    NavigationDrawer, NavigationRail, Offstage, Opacity, OverflowBox, OverlayPortal, Placement,
    RadioGroup, RadioListTile, RichText, RotatedBox, SafeArea, SearchAnchor, SearchBar,
    SegmentedButton, SingleChildScrollView, SizedBox, Skeleton, Spacer, Stack, Stepper, Switch,
    SwitchListTile, TabBar, Theme, Timeline, Transform, TwoPane, UserAccountsDrawerHeader,
    VerticalDivider, Visibility, Widget,
};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Renders `root` in a `width`×`height` dark-theme window, the way the shell would,
/// and compares it against `tests/goldens/<name>.png`.
fn check(name: &str, width: u32, height: u32, root: &dyn Widget<()>) {
    let theme = Theme::dark();
    let Some(snapshot) = render_widget(root, width, height, &theme) else {
        eprintln!("no GPU adapter available: {name} skipped");
        return;
    };
    // A frame that came out empty is a broken test, not a passing one: the golden
    // would be blessed blank and never say anything again.
    assert!(snapshot.lit_pixels(48) > 40, "{name}: the frame is empty");
    snapshot.assert_golden(golden(name));
}

/// A coloured box that fills whatever room it is given — what the sizing widgets
/// need, since the whole point of them is the box they hand their child.
fn fill_of(color: Color) -> Flex<()> {
    Flex::row()
        .flex(1.0)
        .child(Container::new().flex(1.0).color(color).radius(4.0))
}

/// A plain coloured box, so the layout widgets have something visible to place.
fn box_of(color: Color, width: f32, height: f32) -> Container<()> {
    Container::new()
        .width(width)
        .height(height)
        .color(color)
        .radius(4.0)
}

// `Color::rgb8` is not a const fn, so these are the same three colours in the
// floating-point constructor that is.
const TEAL: Color = Color::rgb(0.149, 0.651, 0.604);
const AMBER: Color = Color::rgb(1.0, 0.702, 0.0);
const INDIGO: Color = Color::rgb(0.361, 0.420, 0.753);

// ---------------------------------------------------------------------------
// The controls
// ---------------------------------------------------------------------------

/// Every toggle in both states, in one frame: the tick, the knob and the dot are
/// three different paths over three different backgrounds, and nothing pinned any of
/// them down before this.
#[test]
fn toggles_in_both_states() {
    let root: Card<()> = Card::new().padding(16.0).child(
        Flex::column()
            .gap(12.0)
            .child(Checkbox::new(true).label("Checked"))
            .child(Checkbox::new(false).label("Unchecked"))
            .child(
                Flex::row()
                    .gap(16.0)
                    .align(Align::Center)
                    .child(Switch::new(true))
                    .child(Switch::new(false)),
            )
            .child(
                RadioGroup::new(1, |_: usize| ())
                    .option("First")
                    .option("Second")
                    .option("Third"),
            ),
    );
    check("controls_toggles", 260, 240, &root);
}

/// The small indicators — a bar part-way and full, a spinner, a badge, two keycaps
/// and two skeleton lines.
#[test]
fn the_small_indicators() {
    let root: Card<()> = Card::new().padding(16.0).child(
        Flex::column()
            .gap(12.0)
            .child(LinearProgressIndicator::new(0.35).width(200.0))
            .child(LinearProgressIndicator::new(1.0).width(200.0))
            .child(
                Flex::row()
                    .gap(12.0)
                    .align(Align::Center)
                    .child(CircularProgressIndicator::new().size(24.0))
                    .child(Badge::new("3"))
                    .child(Kbd::new("Ctrl"))
                    .child(Kbd::new("K")),
            )
            .child(Skeleton::new().width(180.0).height(12.0).radius(6.0))
            .child(Skeleton::new().width(120.0).height(12.0).radius(6.0)),
    );
    check("small_indicators", 260, 220, &root);
}

/// **The four floating action buttons**, which had no widget at all until milestone 464
/// — only a helper returning a filled `Button`, two colour roles off the reference's.
/// Small, regular, large and extended, so the three numbers each size carries (its box,
/// its corner, its glyph) can be seen to be three and not one.
#[test]
fn the_floating_action_buttons() {
    let root: Container<()> = Container::new().padding(16.0).child(
        Flex::column()
            .gap(12.0)
            .align(Align::Center)
            .child(
                Flex::row()
                    .gap(12.0)
                    .align(Align::Center)
                    .child(FloatingActionButton::new(Icons::Add).small().on_press(()))
                    .child(FloatingActionButton::new(Icons::Add).on_press(()))
                    .child(FloatingActionButton::new(Icons::Add).large().on_press(())),
            )
            .child(
                FloatingActionButton::extended("New list")
                    .icon(Icons::Add)
                    .on_press(()),
            ),
    );
    check("floating_action_buttons", 260, 220, &root);
}

/// **The three control tiles**, none of which existed until milestone 465: a row whose
/// whole width works one control. A settings screen is a column of these, and the point
/// of the picture is that the label and the control read as one thing.
#[test]
fn the_control_list_tiles() {
    let root: Container<()> = Container::new().padding(8.0).child(
        Flex::column()
            .width(280.0)
            .child(
                SwitchListTile::new(true, ())
                    .title("Notifications")
                    .subtitle("Replies and mentions"),
            )
            .child(CheckboxListTile::new(true, ()).title("Sounds"))
            .child(
                CheckboxListTile::maybe(None, ())
                    .title("Select all")
                    .control_affinity(ControlAffinity::Leading),
            )
            .child(
                RadioListTile::new(true, ())
                    .title("Every day")
                    .control_affinity(ControlAffinity::Leading),
            )
            .child(
                RadioListTile::new(false, ())
                    .title("Once a week")
                    .control_affinity(ControlAffinity::Leading),
            ),
    );
    check("control_list_tiles", 300, 340, &root);
}

/// The four alert kinds together, which is the only way to see that they are four
/// different colours and not three and a duplicate.
#[test]
fn the_four_alert_kinds() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(8.0)
            .child(Alert::new("Saved as a draft.").title("Note"))
            .child(
                Alert::new("Everything went through.")
                    .title("Done")
                    .success(),
            )
            .child(
                Alert::new("Two rows were skipped.")
                    .title("Careful")
                    .warning(),
            )
            .child(
                Alert::new("The server refused the change.")
                    .title("Failed")
                    .error(),
            ),
    );
    check("alert_kinds", 320, 340, &root);
}

/// Every icon the framework ships, at two sizes and two colours, with a divider
/// between the rows. Icons are paths, and this is the frame that says whether a
/// change to the path pipeline moved any of them.
#[test]
fn every_icon_and_a_divider() {
    const NAMES: [Icons; 14] = [
        Icons::Check,
        Icons::Close,
        Icons::Add,
        Icons::Menu,
        Icons::Star,
        Icons::Heart,
        Icons::Circle,
        Icons::Square,
        Icons::Play,
        Icons::ArrowLeft,
        Icons::ChevronLeft,
        Icons::ChevronRight,
        Icons::Eye,
        Icons::EyeOff,
    ];
    let mut small = Flex::row().gap(8.0).align(Align::Center);
    let mut large = Flex::row().gap(8.0).align(Align::Center);
    for name in NAMES {
        small = small.child(Icon::new(name).size(14.0));
        large = large.child(Icon::new(name).size(22.0).color(AMBER));
    }
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(10.0)
            .child(small)
            .child(Divider::new())
            .child(large),
    );
    check("icon_set", 430, 110, &root);
}

// ---------------------------------------------------------------------------
// Getting around
// ---------------------------------------------------------------------------

/// The three ways of choosing one of several things, stacked: a trail, a tab strip
/// and a segmented control.
#[test]
fn the_pickers_of_one_thing() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(14.0)
            .child(
                Breadcrumb::new(|_: usize| ())
                    .crumb("Home")
                    .crumb("Projects")
                    .crumb("frus"),
            )
            .child(Divider::new())
            .child(
                TabBar::new(1, |_: usize| ())
                    .tab("Overview", text("the first pane"))
                    .tab("Details", text("the second pane"))
                    .tab("History", text("the third pane")),
            )
            .child(
                SegmentedButton::new(0, |_: usize| ())
                    .segment("Day")
                    .segment("Week")
                    .segment("Month"),
            ),
    );
    check("navigation_pickers", 340, 240, &root);
}

/// A section open beside one shut — the chevron turns and the content appears, and
/// both halves are in one image.
#[test]
fn a_section_open_and_one_shut() {
    // Given a **width**: an expansion tile is a row, and a row whose trailing slot is
    // the chevron only reads as one when there is a far edge for the chevron to reach.
    let root: Container<()> = Container::new().width(300.0).padding(12.0).child(
        Flex::column()
            .flex(1.0)
            .gap(8.0)
            .child(
                ExpansionTile::new("What is open", true, ())
                    .content(text("The content of the open section.").size(13.0)),
            )
            .child(
                ExpansionTile::new("What is shut", false, ())
                    .content(text("Never drawn.").size(13.0)),
            ),
    );
    check("collapsible_pair", 300, 160, &root);
}

/// A numeric stepper and a timeline: one is two buttons round a value, the other a
/// rail with dots down it.
#[test]
fn a_stepper_and_a_timeline() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(14.0)
            .child(Stepper::new(3, |_: i32| ()).range(0, 10))
            .child(Divider::new())
            .child(
                Timeline::new()
                    .event("Opened", "the request came in")
                    .event("Reviewed", "two comments")
                    .event("Merged", "into master"),
            ),
    );
    check("stepper_and_timeline", 300, 270, &root);
}

/// The top bar with everything on it: a leading icon, a title, two actions.
#[test]
fn the_app_bar() {
    let bar: Box<dyn Widget<()>> = AppBar::new("Inbox")
        .width(360.0)
        .leading(Icon::new(Icons::Menu).size(20.0))
        .action("Save", ())
        .action("Edit", ())
        .build();
    check("app_bar", 360, 80, &*bar);
}

/// The bottom bar, uncut — the notch belongs to the scaffold, which is the only
/// party that knows where the button is.
#[test]
fn the_bottom_app_bar() {
    let root: Container<()> = Container::new().width(320.0).height(90.0).child(
        BottomAppBar::new().padding(8.0).child(
            Flex::row()
                .width(304.0)
                .gap(16.0)
                .align(Align::Center)
                .child(Icon::new(Icons::Menu).size(20.0))
                .child(Icon::new(Icons::Star).size(20.0))
                .child(Container::new().flex(1.0))
                .child(Icon::new(Icons::Close).size(20.0)),
        ),
    );
    check("bottom_app_bar", 320, 90, &root);
}

/// The navigation chrome: a rail down the side, a bar across the top, a bar across
/// the bottom, one of the rail's items carrying a count.
#[test]
fn the_navigation_chrome() {
    let root: Flex<()> = Flex::row()
        .child(
            NavigationRail::new(1, |_: usize| ())
                .item("★", "Home")
                .item("♥", "Saved")
                .badge(4)
                .item("■", "Files"),
        )
        .child(
            // The width is explicit: `NavigationBar` is `Auto`, so with nothing to fill it
            // shrinks around its back button and paints its centred title underneath
            // it. A screen always gives it one; the harness has to as well.
            Flex::column()
                .width(260.0)
                .flex(1.0)
                .child(NavigationBar::new("Saved").on_back(()))
                .child(Container::new().flex(1.0))
                .child(
                    BottomBar::new(0, |_: usize| ())
                        .item("★", "Home")
                        .item("♥", "Saved"),
                ),
        );
    check("navigation_chrome", 340, 220, &root);
}

/// **The third navigation form**, which the framework had no widget for until milestone
/// 467: a rail and a bar were the only two, and an application with eight destinations
/// had to choose between hiding six of them and inventing its own list.
///
/// What the picture is for: the indicator is the **whole row** here, not a pill around
/// the glyph, so the glyph and the label take the same colour when selected — the one
/// visible difference from a rail, and the one a set of assertions about colours cannot
/// show. The rule between the second group and the first is a child and not a
/// destination, so `Trash` answers with 2.
#[test]
fn the_navigation_drawer() {
    let root: NavigationDrawer<()> = NavigationDrawer::new(1, |_: usize| ())
        .width(300.0)
        .header(
            Container::new()
                .padding(16.0)
                .child(text("Mailbox").size(16.0)),
        )
        .item("\u{2709}", "Inbox")
        .badge(12)
        .item("\u{2605}", "Starred")
        .item("\u{2691}", "Drafts")
        .child(Divider::new())
        .item("\u{2717}", "Trash")
        .item("\u{2699}", "Settings")
        .disabled();
    check("navigation_drawer", 300, 340, &root);
}

/// **The block at the top of a side panel**, which milestone 467 left a slot for and
/// nothing to put in it. The account variant is the one worth a picture: the two lines
/// are not centred as a pair — the address sits at the middle of its 56-pixel row so that
/// it stays level with the control beside it, and the name goes above it.
#[test]
fn the_drawer_account_header() {
    let root: NavigationDrawer<()> = NavigationDrawer::new(0, |_: usize| ())
        .width(300.0)
        .header(
            UserAccountsDrawerHeader::new()
                .account_name("Ada Lovelace")
                .account_email("ada@example.com")
                .current_picture(
                    CircleAvatar::new("Ada Lovelace")
                        .size(72.0)
                        .color(Color::rgb8(24, 60, 40)),
                )
                .other_picture(
                    CircleAvatar::new("Charles Babbage")
                        .size(40.0)
                        .color(Color::rgb8(24, 60, 40)),
                )
                .other_picture(
                    CircleAvatar::new("Grace Hopper")
                        .size(40.0)
                        .color(Color::rgb8(24, 60, 40)),
                )
                .on_details_pressed(()),
        )
        .item("\u{2709}", "Inbox")
        .item("\u{2605}", "Starred");
    check("drawer_account_header", 300, 340, &root);
}

/// **A rule down a row and a gap that takes what is left.** Neither existed before
/// milestone 468: every separator in the framework ran across a column, and every gap was
/// a number somebody had to guess from the parent's width.
#[test]
fn a_rule_down_a_row_and_the_room_left_over() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::row()
            .width(256.0)
            .height(40.0)
            .align(Align::Center)
            .child(text("Drafts"))
            .child(VerticalDivider::new())
            .child(text("Sent"))
            .child(Spacer::new())
            .child(text("12")),
    );
    check("row_rules_and_space", 280, 64, &root);
}

/// **The search bar**, which is a field with no container of its own inside a container of
/// the framework's. The picture is for that: the pill's corners are the pill's, and there
/// is no second box sitting inside them. Three of them — carrying a query, empty with a
/// hint, and disabled, which fades rather than flattening.
#[test]
fn the_search_bars() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(14.0)
            .child(
                SearchBar::new("flight to lisbon")
                    .min_width(340.0)
                    .leading(Icon::new(Icons::Menu).size(20.0))
                    .trailing(Icon::new(Icons::Close).size(20.0)),
            )
            .child(
                SearchBar::new("")
                    .min_width(340.0)
                    .hint("Search mail")
                    .leading(Icon::new(Icons::Star).size(20.0)),
            )
            .child(
                SearchBar::new("")
                    .min_width(340.0)
                    .hint("Search is off")
                    .enabled(false),
            ),
    );
    check("search_bars", 380, 230, &root);
}

/// **The view a search bar opens**, which milestone 469 left the bar without. The picture
/// is of the floating form: a panel under the anchor, its header the same field again on
/// the panel's own surface — flat and transparent, so the panel casts one shadow and not
/// two — a rule, and the rows underneath.
#[test]
fn the_search_view() {
    let root: Container<()> = Container::new().padding(10.0).child(
        SearchAnchor::new(true, "lis")
            .full_screen(false)
            .hint("Search mail")
            .view_min_width(340.0)
            .view_min_height(215.0)
            .on_close(())
            .on_clear(())
            .suggestion(ListTile::new().dense().title("flight to lisbon").on_tap(()))
            .suggestion(
                ListTile::new()
                    .dense()
                    .title("flights from lisbon")
                    .on_tap(()),
            )
            .suggestion(ListTile::new().dense().title("lisbon weather").on_tap(())),
    );
    check("search_view", 380, 320, &root);
}

// ---------------------------------------------------------------------------
// What comes over the top
// ---------------------------------------------------------------------------

/// A drawer open over its body. Every overlay in this section is here for one
/// reason: milestone 295 found that the text beneath them read straight through, and
/// nothing in the suite drew one.
#[test]
fn a_drawer_open_over_its_body() {
    let root: Container<()> = Container::new().child(
        frus_widgets::Drawer::new(true)
            .panel(
                Container::new().padding(16.0).child(
                    Flex::column()
                        .gap(10.0)
                        .child(text("Panel").size(18.0).weight(FontWeight::Bold))
                        .child(text("Inbox").size(14.0))
                        .child(text("Archive").size(14.0)),
                ),
            )
            .body(
                Container::new().padding(16.0).child(
                    Flex::column()
                        .gap(8.0)
                        .child(text("Body behind the drawer").size(15.0))
                        .child(text("A second line, covered.").size(15.0)),
                ),
            ),
    );
    check("drawer_open", 360, 220, &root);
}

/// A sheet risen over its body.
#[test]
fn a_sheet_over_its_body() {
    let root: Container<()> = Container::new().child(
        BottomSheet::new(true)
            .sheet(
                Container::new().padding(16.0).child(
                    Flex::column()
                        .gap(8.0)
                        .child(text("Share this").size(16.0))
                        .child(text("Copy link").size(14.0)),
                ),
            )
            .body(
                Container::new().padding(16.0).child(
                    Flex::column()
                        .gap(8.0)
                        .child(text("Body under the sheet").size(15.0))
                        .child(text("Covered by it.").size(15.0)),
                ),
            ),
    );
    check("bottom_sheet_open", 300, 240, &root);
}

/// A popover open on its anchor, and a portal placed below one — the two ways a
/// widget puts something over the frame.
#[test]
fn a_popover_and_a_portal() {
    let popover: MenuAnchor<()> = MenuAnchor::new(text("Anchor").size(14.0), true, ()).content(
        Container::new()
            .padding(10.0)
            .color(Color::rgb8(46, 52, 66))
            .radius(6.0)
            .child(text("Over the top").size(13.0)),
    );
    let portal: OverlayPortal<()> = OverlayPortal::new(text("Second anchor").size(14.0))
        .overlay(
            Container::new()
                .padding(10.0)
                .color(Color::rgb8(46, 52, 66))
                .radius(6.0)
                .child(text("Placed below").size(13.0)),
            Placement::Below,
        )
        .dismiss(());
    let root: Container<()> = Container::new().padding(14.0).child(
        Flex::column()
            .gap(60.0)
            .child(popover)
            .child(text("A line the popover covers").size(14.0))
            .child(portal),
    );
    check("popover_and_portal", 300, 260, &root);
}

// ---------------------------------------------------------------------------
// Putting things in places
// ---------------------------------------------------------------------------

/// Layers over one another, cells in a grid, and rows built on demand.
#[test]
fn stacked_gridded_and_listed() {
    let stack: Stack<()> = Stack::new()
        .width(120.0)
        .height(90.0)
        .layer(box_of(INDIGO, 120.0, 90.0))
        .layer(
            Container::new()
                .padding(14.0)
                .child(box_of(TEAL, 60.0, 44.0)),
        )
        .layer(
            Container::new()
                .padding(26.0)
                .child(text("on top").size(12.0)),
        );

    let mut grid: GridView<()> = GridView::new(3).gap(6.0).width(120.0);
    for i in 0..6 {
        let shade = 60 + i as u8 * 24;
        grid = grid.cell(box_of(Color::rgb8(shade, 90, 140), 34.0, 26.0));
    }

    let list: ListView<()> = ListView::new(12, 24.0, |i| text(format!("Row {i}")).size(12.0))
        .width(110.0)
        .height(120.0);

    let root: Container<()> = Container::new()
        .padding(12.0)
        .child(Flex::row().gap(12.0).child(stack).child(grid).child(list));
    check("stack_grid_list", 420, 150, &root);
}

/// A carousel on its second slide, and a two-pane at its expanded size class.
#[test]
fn a_carousel_and_two_panes() {
    let carousel: CarouselView<()> =
        CarouselView::new(1, 3, |_: usize| (), box_of(TEAL, 140.0, 70.0));
    let panes: TwoPane<()> = TwoPane::new(SizeClass::Expanded)
        .ratio(0.4)
        .show_detail(true)
        .list(
            Container::new()
                .padding(10.0)
                .color(Color::rgb8(38, 42, 54))
                .child(text("The list").size(13.0)),
        )
        .detail(
            Container::new()
                .padding(10.0)
                .child(text("The detail").size(13.0)),
        );
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(12.0)
            .child(carousel)
            .child(Divider::new())
            // Expanded, because a `TwoPane` asks for the **whole** of its parent's height
            // (a percentage) and here it is sharing that parent with two other things. It
            // used to be squeezed back into what was left; nothing is squeezed now
            // (milestone 349), so the pane has to be told to take the remainder instead.
            .child(Expanded::new(panes)),
    );
    check("carousel_and_two_pane", 320, 220, &root);
}

/// A column taller than its viewport, inside a scroll area: the point is the clip at
/// the bottom edge, which is what says the viewport was honoured.
#[test]
fn a_column_that_does_not_fit() {
    let mut column = Flex::column().gap(6.0);
    for i in 0..12 {
        column = column.child(
            Container::new()
                .padding(6.0)
                .color(Color::rgb8(40, 44, 56))
                .radius(4.0)
                .child(text(format!("Item {i}")).size(13.0)),
        );
    }
    let root: Container<()> = Container::new().padding(10.0).child(
        SingleChildScrollView::new()
            .width(180.0)
            .height(140.0)
            .child(column),
    );
    check("scroll_clips_its_column", 220, 170, &root);
}

/// The boxes that decide a size: a ratio, a fraction of the parent, a fixed square,
/// and a maximum.
#[test]
fn the_boxes_that_size() {
    let root: Container<()> = Container::new().width(340.0).padding(12.0).child(
        Flex::column()
            .gap(10.0)
            .child(
                SizedBox::new(AspectRatio::new(16.0 / 9.0).child(fill_of(TEAL)))
                    .width(120.0)
                    .height(68.0),
            )
            .child(
                SizedBox::new(
                    FractionallySizedBox::new()
                        .width_factor(0.5)
                        // `flex(1.0)`: the fractional box sizes *itself*, and the
                        // child has to fill it to show what size that was.
                        .child(
                            Container::new()
                                .flex(1.0)
                                .height(20.0)
                                .color(AMBER)
                                .radius(4.0),
                        ),
                )
                .width(200.0)
                .height(20.0),
            )
            .child(SizedBox::square(40.0, fill_of(INDIGO)))
            .child(ConstrainedBox::new(box_of(TEAL, 300.0, 18.0)).max_width(150.0)),
    );
    check("boxes_that_size", 340, 220, &root);
}

/// The boxes that fit something into a size: a scaled child, a child allowed past
/// its parent, and one measured at its natural width.
#[test]
fn the_boxes_that_fit() {
    let root: Container<()> = Container::new().width(300.0).padding(12.0).child(
        Flex::column()
            .gap(14.0)
            .child(
                SizedBox::new(
                    FittedBox::new(BoxFit::Contain)
                        .width(120.0)
                        .height(50.0)
                        .child(box_of(TEAL, 200.0, 60.0)),
                )
                .width(120.0)
                .height(50.0),
            )
            .child(
                Container::new()
                    .width(80.0)
                    .height(30.0)
                    .color(Color::rgb8(40, 44, 56))
                    .child(
                        OverflowBox::new(fill_of(AMBER))
                            .width(140.0)
                            .height(20.0)
                            .alignment(Alignment::CENTER),
                    ),
            )
            .child(Intrinsic::width(
                Container::new()
                    .color(INDIGO)
                    .padding(6.0)
                    .child(text("natural width").size(13.0)),
            )),
    );
    check("boxes_that_fit", 300, 200, &root);
}

/// A quarter turn and a free rotation, side by side: one is a layout operation, the
/// other a paint transform, and they are easy to confuse until you see both.
#[test]
fn turned_and_rotated() {
    let root: Container<()> = Container::new().padding(24.0).child(
        Flex::row()
            .gap(44.0)
            .align(Align::Center)
            .child(
                SizedBox::new(RotatedBox::new(1).child(text("turned").size(14.0)))
                    .width(24.0)
                    .height(70.0),
            )
            .child(Transform::rotate(0.35).child(box_of(AMBER, 70.0, 40.0)))
            .child(Transform::scale(0.6).child(box_of(TEAL, 70.0, 40.0))),
    );
    check("turned_and_rotated", 360, 180, &root);
}

/// The three clips, each over the same box: rounded, oval, and an arbitrary path.
#[test]
fn the_three_clips() {
    let triangle = Path::new()
        .move_to(Point::new(40.0, 0.0))
        .line_to(Point::new(80.0, 60.0))
        .line_to(Point::new(0.0, 60.0))
        .close();

    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::row()
            .gap(14.0)
            .child(ClipRRect::new(16.0).child(box_of(TEAL, 80.0, 60.0).radius(0.0)))
            .child(ClipOval::new().child(box_of(AMBER, 80.0, 60.0).radius(0.0)))
            .child(ClipPath::new(triangle).child(box_of(INDIGO, 80.0, 60.0).radius(0.0))),
    );
    check("the_three_clips", 300, 90, &root);
}

// ---------------------------------------------------------------------------
// What draws itself
// ---------------------------------------------------------------------------

/// A canvas an application paints itself — a rectangle, a stroked path and a line of
/// text, which is also the smallest scene that puts all three kinds in one widget.
#[test]
fn a_canvas_painted_by_hand() {
    let root: Container<()> = Container::new().padding(10.0).child(CustomPaint::new(
        200.0,
        100.0,
        |scene: &mut frus_core::Scene, rect: frus_core::Rect, theme: &Theme| {
            scene.draw_rect(rect, Color::rgb8(28, 32, 42), 8.0, 0.0, Color::TRANSPARENT);
            let (left, top) = (rect.x, rect.y);
            let (right, bottom) = (rect.x + rect.width, rect.y + rect.height);
            let wave = Path::new()
                .move_to(Point::new(left + 12.0, bottom - 20.0))
                .line_to(Point::new(left + 60.0, top + 24.0))
                .line_to(Point::new(left + 110.0, bottom - 32.0))
                .line_to(Point::new(right - 12.0, top + 16.0));
            scene.stroke_path(&wave, TEAL, 3.0);
            scene.text(
                Point::new(left + 12.0, top + 8.0),
                "painted by hand",
                &frus_core::ResolvedTextStyle::exact(13.0),
                theme.on_surface,
            );
        },
    ));
    check("custom_paint", 220, 120, &root);
}

/// A bitmap under three fits, so that a change to the image pipeline says which of
/// them it moved.
#[test]
fn an_image_under_three_fits() {
    let root: Container<()> = Container::new().padding(10.0).child(
        Flex::row()
            .gap(10.0)
            .child(
                Image::new(swatch_image())
                    .size(70.0, 70.0)
                    .fit(BoxFit::Cover),
            )
            .child(
                Image::new(swatch_image())
                    .size(70.0, 70.0)
                    .fit(BoxFit::Contain),
            )
            .child(
                Image::new(swatch_image())
                    .size(70.0, 70.0)
                    .fit(BoxFit::Fill),
            )
            .child(Image::new(swatch_image()).size(70.0, 70.0).tint(AMBER)),
    );
    check("image_fits", 340, 90, &root);
}

/// A 8×4 gradient, deliberately the wrong shape for the boxes it is put in, so the
/// fits differ visibly.
fn swatch_image() -> ImageHandle {
    const W: u32 = 8;
    const H: u32 = 4;
    let mut rgba = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        for x in 0..W {
            rgba.extend_from_slice(&[
                (x * 255 / (W - 1)) as u8,
                (y * 255 / (H - 1)) as u8,
                160,
                255,
            ]);
        }
    }
    ImageData::from_rgba(W, H, rgba).into_handle()
}

/// One line, several styles, inheritance cascading through the tree.
#[test]
fn rich_text_in_one_line() {
    let theme = Theme::dark();
    // The width is explicit: a `Container` that is `Auto` shrinks to fit, so the run
    // measures unbounded and never wraps. The demo works around the same thing by
    // computing its content width by hand.
    let root: Container<()> = Container::new().width(280.0).padding(12.0).child(
        RichText::new(
            TextSpan::new("A ")
                .child(TextSpan::new("bold").bold())
                .child(TextSpan::new(", "))
                .child(TextSpan::new("italic underlined").italic().underline())
                .child(TextSpan::new(" and a "))
                .child(TextSpan::new("coloured").color(TEAL))
                .child(TextSpan::new(" run.")),
        )
        .base_style(theme.text.body_medium)
        .wrap(),
    );
    check("rich_text_runs", 280, 90, &root);
}

/// A palette with one swatch selected, which is the only state worth a picture: the
/// selection ring is drawn by the picker, not the swatch.
#[test]
fn a_colour_palette() {
    let swatches = [
        Color::rgb8(244, 67, 54),
        Color::rgb8(233, 30, 99),
        Color::rgb8(156, 39, 176),
        Color::rgb8(63, 81, 181),
        Color::rgb8(3, 169, 244),
        Color::rgb8(0, 150, 136),
        Color::rgb8(139, 195, 74),
        Color::rgb8(255, 152, 0),
    ];
    let mut picker: ColorPicker<()> = ColorPicker::new(Some(swatches[3]), 4, |_: Color| ());
    for colour in swatches {
        picker = picker.swatch(colour);
    }
    let root: Container<()> = Container::new().padding(12.0).child(picker);
    check("colour_palette", 240, 140, &root);
}

// ---------------------------------------------------------------------------
// What a widget withholds
// ---------------------------------------------------------------------------

/// A group faded as one — the overlap between the two boxes must not darken, which
/// is the whole difference between group opacity and per-widget alpha.
///
/// The offset is 10 px, not the 20 it was: at 20 the padded box needed 130x100 of a
/// 120x80 stack, and milestone 345 painted a striped band across the very overlap this
/// exists to look at. The fixture was overflowing all along and nothing said so.
#[test]
fn a_faded_group() {
    let group: Stack<()> = Stack::new()
        .width(120.0)
        .height(80.0)
        .layer(box_of(TEAL, 90.0, 60.0))
        .layer(
            Container::new()
                .padding(10.0)
                .child(box_of(AMBER, 90.0, 60.0)),
        );
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::row()
            .gap(16.0)
            .child(Opacity::new(1.0, group))
            .child(Opacity::new(
                0.4,
                Stack::new()
                    .width(120.0)
                    .height(80.0)
                    .layer(box_of(TEAL, 90.0, 60.0))
                    .layer(
                        Container::new()
                            .padding(10.0)
                            .child(box_of(AMBER, 90.0, 60.0)),
                    ),
            )),
    );
    check("group_opacity_pair", 320, 110, &root);
}

/// Withheld three ways: hidden but still taking its room, hidden and taking none,
/// and hidden with something put in its place.
#[test]
fn what_is_withheld() {
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(6.0)
            .child(text("above").size(13.0))
            .child(
                Visibility::new(box_of(TEAL, 120.0, 20.0))
                    .visible(false)
                    .maintain_size(),
            )
            .child(text("after a kept gap").size(13.0))
            .child(Offstage::new(box_of(AMBER, 120.0, 20.0)).offstage(true))
            .child(text("after no gap at all").size(13.0))
            .child(
                Visibility::new(box_of(INDIGO, 120.0, 20.0))
                    .visible(false)
                    .replacement(box_of(AMBER, 60.0, 20.0)),
            )
            .child(text("after a replacement").size(13.0)),
    );
    check("withheld_widgets", 240, 200, &root);
}

/// A safe area with a minimum inset on every edge: the box inside is pushed off all
/// four, which is the only thing to look at.
#[test]
fn a_safe_area_with_a_minimum() {
    let root: Container<()> = Container::new()
        .width(200.0)
        .height(120.0)
        .color(Color::rgb8(30, 34, 44))
        .child(
            // 200×120 less 16 on each edge is exactly 168×88: the box lands flush
            // against the safe area, so any error in the inset shifts or clips it.
            SafeArea::new(
                box_of(TEAL, 168.0, 88.0)
                    .padding(6.0)
                    .child(text("inside").size(13.0)),
            )
            .minimum(Insets::uniform(16.0)),
        );
    check("safe_area_minimum", 200, 120, &root);
}
