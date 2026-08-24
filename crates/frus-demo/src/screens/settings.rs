//! The settings screen: the controls driving the theme, the language, and the
//! demo's own options.

use crate::prelude::*;
use frus_widgets::{column, row};

/// Labels of the dropdown menu (the Settings screen).
pub(crate) const MENU: [&str; 3] = ["Option A", "Option B", "Option C"];

/// The "Settings" screen: the card of controls (it demonstrates navigation + gesture + widgets).
pub(crate) fn settings_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Container<Msg> {
    let volume_pct = (app.volume * 100.0).round() as u32;
    let controls = Card::new().child(
        column![
            row![
                text("Notifications").size(18.0),
                spacer(),
                Switch::new(app.notifs).on_toggle(Msg::SetNotifs),
            ]
            .align(Align::Center)
            .gap(12.0),
            row![
                text(format!("Volume: {volume_pct}%")).size(18.0),
                // 220 is the width the slider **would like**. A loose flex child takes
                // that or the room left, whichever is smaller — so a phone, where the
                // label and the slider together want 340 in a card of 331, gets a
                // slightly shorter slider instead of nine pixels of overhang.
                Expanded::new(
                    Slider::new(app.volume)
                        .width(220.0)
                        .on_change(Msg::SetVolume),
                )
                .loose(),
            ]
            .align(Align::Center)
            .gap(12.0),
            RadioGroup::new(app.radio, Msg::SetRadio)
                .option("Small")
                .option("Medium")
                .option("Large"),
            DropdownButton::new(MENU[app.menu_choice], Msg::ToggleMenu).options(
                app.menu_open,
                &MENU,
                Msg::SetMenu,
            ),
            row![
                text("Your rating").size(18.0),
                spacer(),
                Rating::new(app.rating, 5, Msg::SetRating),
            ]
            .align(Align::Center)
            .gap(12.0),
            row![
                text("Quantity").size(18.0),
                spacer(),
                Stepper::new(app.count, Msg::SetCount).range(0, 20).step(1),
            ]
            .align(Align::Center)
            .gap(12.0),
            Divider::new(),
            // **A setting that depends on another** (milestone 322): scheduling which days
            // to be notified on means nothing while notifications are off, so the switch
            // goes unavailable rather than staying live and doing nothing visible. This is
            // what `enabled` is for, and the label has to follow it — a live label over a
            // dead control reads as a control that is merely quiet.
            row![
                text("Weekdays only").size(16.0).color(if app.notifs {
                    theme.on_surface
                } else {
                    disabled_content(theme)
                }),
                spacer(),
                Switch::new(app.weekdays_only)
                    .on_toggle(Msg::SetWeekdaysOnly)
                    .enabled(app.notifs),
            ]
            .align(Align::Center)
            .gap(12.0),
            demo_calendar(app),
        ]
        .gap(14.0),
    );
    let total = app.todos.len();
    let done = app.todos.iter().filter(|t| t.done).count();
    // How wide the showcases may get. A ceiling, not a measurement: they fill the tab and
    // stop at 480. Subtracting the paddings by hand — which is what stood here — missed
    // the card's own margin and came out eight pixels too wide.
    const SHOWCASE_MAX: f32 = 480.0;
    let stats = ConstrainedBox::new(
        GridView::new(3)
            .gap(10.0)
            .cell(stat_tile(theme, "Total", total))
            .cell(stat_tile(theme, "Active", total - done))
            .cell(stat_tile(theme, "Done", done)),
    )
    .max_width(SHOWCASE_MAX);
    let facts = ConstrainedBox::new(
        Table::new(2)
            .header(&["Metric", "Value"])
            .row(&["Widgets", "35"])
            .row(&["Milestones", "39"]),
    )
    .max_width(SHOWCASE_MAX);

    // The file tree (expanded according to the state).
    let open = |id: u64| app.expanded.contains(&id);
    // The chevron expands/collapses; the row's body selects the node (milestone 246).
    let mut tree = Tree::new(Msg::ToggleNode)
        .on_select(Msg::SelectNode)
        .selected(app.tree_selected)
        .node(1, 0, "src", true, open(1));
    if open(1) {
        tree = tree.node(2, 1, "widgets", true, open(2));
        if open(2) {
            tree = tree
                .node(3, 2, "button.rs", false, false)
                .node(4, 2, "grid.rs", false, false);
        }
        tree = tree.node(5, 1, "main.rs", false, false);
    }
    tree = tree.node(6, 0, "Cargo.toml", false, false);

    // The colour palette.
    let palette = [
        Color::rgb8(46, 160, 96),
        Color::rgb8(90, 158, 242),
        Color::rgb8(210, 96, 96),
        Color::rgb8(240, 180, 40),
        Color::rgb8(160, 110, 220),
        Color::rgb8(80, 200, 200),
    ];
    let mut picker = ColorPicker::new(app.picked, 6, Msg::PickColor);
    for color in palette {
        picker = picker.swatch(color);
    }

    // A timeline of the recent milestones.
    let timeline = Timeline::new()
        .event("Grid", "Milestone 35")
        .event("New widgets", "Milestones 36–37")
        .event("Hierarchy & color", "Milestone 38");

    // The carousel: the current slide is supplied by index.
    let slide = match app.slide {
        0 => text("Welcome to frus").size(16.0),
        1 => text("About 35 widgets").size(16.0),
        _ => text("Thanks for trying!").size(16.0),
    };
    let carousel = CarouselView::new(app.slide, 3, Msg::SetSlide, slide);

    // An info popover (arbitrary content, dismissed by an outside click).
    let info = MenuAnchor::new(
        button("Info", Msg::ToggleInfo)
            .variant(Variant::Outlined)
            .size(15.0),
        app.info_open,
        Msg::ToggleInfo,
    )
    .content(
        Card::new().padding(16.0).child(
            column![
                text("MenuAnchor").size(16.0),
                text("An arbitrary floating panel; closes on outside click.")
                    .size(14.0)
                    .color(theme.muted),
            ]
            .gap(6.0),
        ),
    );

    // Autocomplete: suggestions filtered by what is typed (controlled).
    const TAGS: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];
    let mut tags = Autocomplete::new(app.tag_draft.clone(), Msg::TagInput, Msg::TagPick);
    if !app.tag_draft.is_empty() {
        let q = app.tag_draft.to_lowercase();
        for tag in TAGS {
            if tag.starts_with(&q) {
                tags = tags.suggestion(tag);
            }
        }
    }

    // Keyboard shortcut hints.
    let shortcuts = row![
        text("Shortcuts:").size(14.0).color(theme.muted),
        Kbd::new("Enter"),
        text("add").size(14.0).color(theme.muted),
        Kbd::new("Tab"),
        text("navigate").size(14.0).color(theme.muted),
    ]
    .align(Align::Center)
    .gap(6.0);
    let about = column![
        text("frus — widget showcase").size(18.0),
        row![info, tags].align(Align::Start).gap(12.0),
        shortcuts,
        stats,
        facts,
        carousel,
        Pagination::new(app.page, 8, Msg::SetPage),
        column![
            ConstrainedBox::new(Skeleton::new()).max_width(SHOWCASE_MAX),
            ConstrainedBox::new(Skeleton::new().height(14.0)).max_width(SHOWCASE_MAX * 0.8),
        ]
        .gap(8.0),
        Divider::new(),
        ExpansionTile::new("Advanced options", app.advanced_open, Msg::ToggleAdvanced).content(
            column![
                text("Explorer, palette, timeline:")
                    .size(15.0)
                    .color(theme.muted),
                tree,
                picker,
                timeline,
                row![Chip::new("beta"), Chip::new("experimental")].gap(8.0),
            ]
            .gap(10.0)
        ),
    ]
    .gap(12.0);
    let tabs = TabBar::new(app.settings_tab, Msg::SetSettingsTab)
        .tab("Controls", controls)
        .tab("About", about);
    let content = column![
        Breadcrumb::new(|_| Msg::Pop)
            .crumb("Home")
            .crumb("Settings"),
        row![tabs].justify(Justify::Center),
    ]
    .padding(20.0)
    .gap(16.0);
    // The content (the calendar, the advanced options…) is taller than the screen: it scrolls
    // under the bar, which stays pinned.
    let body = SingleChildScrollView::new()
        .width(width)
        .flex(1.0)
        .child(content);
    let screen = column![NavigationBar::new("Settings").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}
