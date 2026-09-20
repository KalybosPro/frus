//! The settings screen: the controls driving the demo's own options, and a gallery of the
//! widgets that hold a value.
//!
//! Every control's value is this screen's own — a `StatefulWidget` keeps it, and closes over
//! nothing but the state. Only the statistics read the shared task list.

use crate::prelude::*;
use frus_widgets::{column, row};
use std::collections::HashSet;

/// Labels of the dropdown menu (the Settings screen).
pub(crate) const MENU: [&str; 3] = ["Option A", "Option B", "Option C"];
/// A list long enough that filtering is the point of it rather than a decoration — and
/// with three names sharing a word that none of them starts with, so a substring rule and
/// a prefix rule visibly disagree.
pub(crate) const CITIES: [&str; 8] = [
    "Cape Town",
    "Kansas City",
    "Lomé",
    "Mexico City",
    "New York City",
    "Ouagadougou",
    "Porto-Novo",
    "Reykjavík",
];

/// The "Settings" screen: the card of controls (it demonstrates navigation + gesture +
/// widgets). Its configuration is the shared state the statistics read.
pub(crate) struct SettingsPage {
    pub(crate) demo: Rc<Demo>,
}

/// What the settings screen keeps: the value of every control on it.
pub(crate) struct SettingsState {
    pub(crate) notifs: bool,
    pub(crate) volume: f32,
    pub(crate) radio: usize,
    pub(crate) menu_open: bool,
    pub(crate) menu_choice: usize,
    /// The filtering dropdown's own open flag, query and choice. Three pieces of state
    /// rather than one, because the query is **only** what the field shows while the menu
    /// is open: shut, the widget derives the display from `city_choice`, so there is
    /// nothing here to reset when it closes.
    pub(crate) city_open: bool,
    pub(crate) city_query: String,
    pub(crate) city_choice: Option<usize>,
    /// The active tab.
    pub(crate) tab: usize,
    /// Is the about box open?
    pub(crate) about_open: bool,
    /// Is the "Advanced options" section expanded?
    pub(crate) advanced_open: bool,
    /// The star rating.
    pub(crate) rating: u32,
    /// The stepper's counter.
    pub(crate) count: i32,
    /// How many minutes the reminder wheel is resting on (milestone 496).
    pub(crate) reminder: usize,
    /// The pagination selector's current page (a demo).
    pub(crate) page: usize,
    /// The expanded tree nodes (the Tree demo).
    pub(crate) expanded: HashSet<u64>,
    /// The selected tree node (the Tree demo, milestone 246); `None` = none.
    pub(crate) tree_selected: Option<u64>,
    /// The colour picked (the ColorPicker demo).
    pub(crate) picked: Option<Color>,
    /// The calendar: year / month (1..12) / selected day.
    pub(crate) year: i32,
    pub(crate) month: u32,
    pub(crate) selected_day: Option<u32>,
    /// The showcase calendar: should weekends be disabled (`DatePicker::filtered`)? — milestone 238.
    pub(crate) weekdays_only: bool,
    /// The carousel's current slide (a demo).
    pub(crate) slide: usize,
    /// Is the info popover open?
    pub(crate) info_open: bool,
    /// What is typed in the autocomplete (a demo).
    pub(crate) tag_draft: String,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            notifs: false,
            volume: 0.0,
            radio: 0,
            menu_open: false,
            menu_choice: 0,
            city_open: false,
            city_query: String::new(),
            city_choice: None,
            tab: 0,
            about_open: false,
            advanced_open: false,
            rating: 0,
            count: 0,
            reminder: 0,
            page: 1,
            expanded: HashSet::new(),
            tree_selected: None,
            picked: None,
            year: 2026,
            month: 7,
            selected_day: None,
            weekdays_only: false,
            slide: 0,
            info_open: false,
            tag_draft: String::new(),
        }
    }
}

impl SettingsState {
    /// The city menu opened or shut. The query is cleared on the way **in**, not on the way
    /// out: a list that opens already filtered by what was typed last time is a list that
    /// looks broken.
    pub(crate) fn toggle_city(&mut self) {
        self.city_open = !self.city_open;
        self.city_query.clear();
    }

    /// Typing keeps the menu open. The field only shows the query while it is, so a query
    /// arriving at all means the reader is in the middle of choosing.
    pub(crate) fn type_city(&mut self, text: String) {
        self.city_query = text;
        self.city_open = true;
    }

    /// A city was chosen: the menu shuts.
    pub(crate) fn choose_city(&mut self, index: usize) {
        self.city_choice = Some(index);
        self.city_open = false;
    }

    /// A tree node opened or closed.
    pub(crate) fn toggle_node(&mut self, id: u64) {
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
    }

    /// Clicking the already-selected node again deselects it (a toggle).
    pub(crate) fn select_node(&mut self, id: u64) {
        self.tree_selected = if self.tree_selected == Some(id) {
            None
        } else {
            Some(id)
        };
    }

    /// Moves the calendar by `delta` months, carrying over the year, and forgets the day.
    pub(crate) fn nav_month(&mut self, delta: i32) {
        let mut m = self.month as i32 + delta;
        while m < 1 {
            m += 12;
            self.year -= 1;
        }
        while m > 12 {
            m -= 12;
            self.year += 1;
        }
        self.month = m as u32;
        self.selected_day = None;
    }
}

impl StatefulWidget for SettingsPage {
    type State = SettingsState;

    fn create_state(&self) -> SettingsState {
        SettingsState::default()
    }
}

impl State for SettingsState {
    type Widget = SettingsPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let theme = &theme;
        // An open menu takes the back gesture before the router does.
        cx.block_back(self.menu_open || self.city_open);
        // The window this screen fills, read from the surface description in force:
        // nothing hands it down any more.
        let Size { width, height } = surface();
        let router = cx.router();
        let volume_pct = (self.volume * 100.0).round() as u32;
        let controls = Card::new().child(
            column![
                row![
                    text("Notifications").size(18.0),
                    spacer(),
                    Switch::new(self.notifs).on_toggle(cx.handler(|s, on: bool| s.notifs = on)),
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
                        Slider::new(self.volume)
                            .width(220.0)
                            .on_change(cx.handler(|s, v: f32| s.volume = v)),
                    )
                    .loose(),
                ]
                .align(Align::Center)
                .gap(12.0),
                RadioGroup::new(self.radio, cx.handler(|s, i: usize| s.radio = i))
                    .option("Small")
                    .option("Medium")
                    .option("Large"),
                DropdownButton::new(
                    MENU[self.menu_choice],
                    cx.callback(|s| s.menu_open = !s.menu_open)
                )
                .options(
                    self.menu_open,
                    &MENU,
                    cx.handler(|s, i: usize| {
                        s.menu_choice = i;
                        s.menu_open = false;
                    }),
                ),
                // **The same closed set behind a field rather than a button** (milestone 479),
                // which is worth having beside the one above: it filters as it is typed into,
                // and it cannot be left showing something that is not one of the eight — shut,
                // the field is drawn from `city_choice` and the query is not consulted at all.
                DropdownMenu::new(
                    self.city_query.as_str(),
                    self.city_open,
                    cx.handler(|s, text: String| s.type_city(text)),
                    cx.callback(|s| s.toggle_city()),
                )
                .label("City")
                .placeholder("Start typing")
                .width(240.0)
                .selected(self.city_choice)
                .max_visible(5)
                .options(&CITIES, cx.handler(|s, i: usize| s.choose_city(i))),
                row![
                    text("Your rating").size(18.0),
                    spacer(),
                    Rating::new(self.rating, 5, cx.handler(|s, r: u32| s.rating = r)),
                ]
                .align(Align::Center)
                .gap(12.0),
                row![
                    text("Quantity").size(18.0),
                    spacer(),
                    Stepper::new(self.count, cx.handler(|s, c: i32| s.count = c))
                        .range(0, 20)
                        .step(1),
                ]
                .align(Align::Center)
                .gap(12.0),
                Divider::new(),
                self.reminder_wheel(cx, theme),
                Divider::new(),
                // **A setting that depends on another** (milestone 322): scheduling which days
                // to be notified on means nothing while notifications are off, so the switch
                // goes unavailable rather than staying live and doing nothing visible. This is
                // what `enabled` is for, and the label has to follow it — a live label over a
                // dead control reads as a control that is merely quiet.
                row![
                    text("Weekdays only").size(16.0).color(if self.notifs {
                        theme.on_surface
                    } else {
                        disabled_content(theme)
                    }),
                    spacer(),
                    Switch::new(self.weekdays_only)
                        .on_toggle(cx.handler(|s, on: bool| s.weekdays_only = on))
                        .enabled(self.notifs),
                ]
                .align(Align::Center)
                .gap(12.0),
                self.calendar(cx),
            ]
            .gap(14.0),
        );
        let todos = cx.widget().demo.todos();
        let total = todos.len();
        let done = done_count(&todos);
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
        let open = |id: u64| self.expanded.contains(&id);
        // The chevron expands/collapses; the row's body selects the node (milestone 246).
        let mut tree = Tree::new(cx.handler(|s, id: u64| s.toggle_node(id)))
            .on_select(cx.handler(|s, id: u64| s.select_node(id)))
            .selected(self.tree_selected)
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
        let mut picker =
            ColorPicker::new(self.picked, 6, cx.handler(|s, c: Color| s.picked = Some(c)));
        for color in palette {
            picker = picker.swatch(color);
        }

        // A timeline of the recent milestones.
        let timeline = Timeline::new()
            .event("Grid", "Milestone 35")
            .event("New widgets", "Milestones 36–37")
            .event("Hierarchy & color", "Milestone 38");

        // The carousel: the current slide is supplied by index.
        let slide = match self.slide {
            0 => text("Welcome to frus").size(16.0),
            1 => text("About 35 widgets").size(16.0),
            _ => text("Thanks for trying!").size(16.0),
        };
        let carousel =
            CarouselView::new(self.slide, 3, cx.handler(|s, i: usize| s.slide = i), slide);

        // An info popover (arbitrary content, dismissed by an outside click).
        let info = MenuAnchor::new(
            button("Info", cx.callback(|s| s.info_open = !s.info_open))
                .variant(Variant::Outlined)
                .size(15.0),
            self.info_open,
            cx.callback(|s| s.info_open = !s.info_open),
        )
        // The anchor floats its content on a menu panel of its own (milestone 532), so the
        // content brings no surface: a card here would be a surface on a surface.
        .content(
            Container::new().padding(16.0).child(
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
        let mut tags = Autocomplete::new(
            self.tag_draft.clone(),
            cx.handler(|s, text: String| s.tag_draft = text),
            cx.handler(|s, text: String| s.tag_draft = text),
        );
        if !self.tag_draft.is_empty() {
            let q = self.tag_draft.to_lowercase();
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
            Pagination::new(self.page, 8, cx.handler(|s, p: usize| s.page = p)),
            column![
                ConstrainedBox::new(Skeleton::new()).max_width(SHOWCASE_MAX),
                ConstrainedBox::new(Skeleton::new().height(14.0)).max_width(SHOWCASE_MAX * 0.8),
            ]
            .gap(8.0),
            Divider::new(),
            // The row every application writes and every application gets slightly
            // differently — and the licences behind it, which every application shipping
            // anywhere owes and this framework had nothing to show with until milestone 492.
            AboutListTile::new(cx.callback(|s| s.about_open = !s.about_open))
                .application("frus demo")
                .subtitle(format!("Version {}", env!("CARGO_PKG_VERSION")))
                .build(),
            ExpansionTile::new(
                "Advanced options",
                self.advanced_open,
                cx.callback(|s| s.advanced_open = !s.advanced_open)
            )
            .content(
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
        let tabs = TabBar::new(self.tab, cx.handler(|s, i: usize| s.tab = i))
            .tab("Controls", controls)
            .tab("About", about);
        let crumb = router.clone();
        let content = column![
            Breadcrumb::new(on_value(move |_: usize| {
                crumb.pop();
            }))
            .crumb("Home")
            .crumb("Settings"),
            row![tabs].justify(Justify::Center),
        ]
        .padding(20.0)
        .gap(16.0);
        // The content (the calendar, the advanced options…) is taller than the screen: it
        // scrolls under the bar, which stays pinned.
        let body = SingleChildScrollView::new()
            .width(width)
            .flex(1.0)
            .child(content);
        let back = router.clone();
        let screen = column![
            NavigationBar::new("Settings").on_back(on(move || {
                back.pop();
            })),
            body
        ]
        .flex(1.0);
        // The about box, over the screen. It is a dialog like any other: the screen is its
        // body, and it is shut by the same message that opened it — including a tap outside,
        // which is what `on_close` is given to.
        let screen = AboutDialog::new(self.about_open)
            .application("frus demo")
            .version(format!("Version {}", env!("CARGO_PKG_VERSION")))
            .legalese("A demonstration application, MIT OR Apache-2.0.")
            .icon(Icon::new(Icons::INFO))
            .on_licences(on(move || router.push("/licenses")))
            .on_close(cx.callback(|s| s.about_open = !s.about_open))
            .body(screen);
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                // The background runs **under** the bars; the content does not. `SafeArea`
                // reads the intrusions from the surface description, so a screen with no
                // `Scaffold` to do it for it still keeps clear of the notch.
                .child(SafeArea::new(screen)),
        )
    }
}

impl SettingsState {
    /// The showcase calendar: `DatePicker::filtered`, greying out **weekends** when
    /// `weekdays_only` is set (milestone 238), otherwise every day is clickable
    /// (`DatePicker::new`).
    fn calendar(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let pick = cx.handler(|s, d: u32| s.selected_day = Some(d));
        let nav = cx.handler(|s, delta: i32| s.nav_month(delta));
        if self.weekdays_only {
            Box::new(DatePicker::filtered(
                self.year,
                self.month,
                self.selected_day,
                |(y, m, d)| !is_weekend(y, m, d),
                pick,
                nav,
            ))
        } else {
            Box::new(DatePicker::new(
                self.year,
                self.month,
                self.selected_day,
                pick,
                nav,
            ))
        }
    }

    /// **A wheel to pick a number of minutes** (milestone 496), beside the stepper above so
    /// that the two can be read against each other: a stepper is for a number you nudge, a
    /// wheel is for one you spin past sixty of.
    ///
    /// The band across the middle is a **layer of a stack**, not something the wheel draws.
    /// A wheel is a cylinder of rows and nothing else; what marks the chosen one is a
    /// decision about this screen — a band here, two rules elsewhere, a tinted panel on a
    /// dark page — and a widget that drew it would be a widget every one of those had to
    /// argue with.
    fn reminder_wheel(&self, cx: &StateContext<Self>, theme: &Theme) -> Container {
        const ROW: f32 = 34.0;
        const HEIGHT: f32 = 150.0;
        let ink = theme.on_surface;
        let wheel = ListWheel::new(60, ROW, move |minutes| {
            Flex::column()
                .width(120.0)
                .height(ROW)
                .align(Align::Center)
                .justify(Justify::Center)
                .child(text(format!("{minutes}")).size(20.0).color(ink))
        })
        .width(120.0)
        .height(HEIGHT)
        .selected(self.reminder)
        .on_selected(cx.handler(|s, minutes: usize| s.reminder = minutes))
        // Words, not an index: a reader is owed "seven minutes", and only this screen knows
        // that is what row seven means.
        .label(|minutes| format!("{minutes} minutes"));
        Container::new().child(
            row![
                text("Remind me in").size(18.0),
                spacer(),
                Stack::new()
                    .width(120.0)
                    .height(HEIGHT)
                    .layer(
                        Positioned::new(Container::new().height(ROW).radius(8.0).color(Color {
                            a: 0.10,
                            ..theme.primary
                        }),)
                        .left(0.0)
                        .right(0.0)
                        .top((HEIGHT - ROW) / 2.0),
                    )
                    .layer(wheel),
                text(format!("{} min", self.reminder))
                    .size(14.0)
                    .color(theme.muted),
            ]
            .align(Align::Center)
            .gap(12.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_node_selection_toggles() {
        let mut s = SettingsState::default();
        s.select_node(3);
        assert_eq!(s.tree_selected, Some(3));
        s.select_node(4);
        assert_eq!(s.tree_selected, Some(4), "another node replaces it");
        s.select_node(4);
        assert_eq!(s.tree_selected, None, "the same node again deselects");
        s.toggle_node(1);
        assert!(s.expanded.contains(&1));
        s.toggle_node(1);
        assert!(s.expanded.is_empty());
    }

    #[test]
    fn the_calendar_carries_the_year_and_forgets_the_day() {
        let mut s = SettingsState {
            selected_day: Some(9),
            ..Default::default()
        };
        assert_eq!((s.year, s.month), (2026, 7));
        s.nav_month(-7);
        assert_eq!((s.year, s.month), (2025, 12));
        assert_eq!(s.selected_day, None);
        s.nav_month(2);
        assert_eq!((s.year, s.month), (2026, 2));
        s.nav_month(12);
        assert_eq!((s.year, s.month), (2027, 2));
    }

    #[test]
    fn the_city_query_is_cleared_on_the_way_in_and_typing_keeps_the_menu_open() {
        let mut s = SettingsState::default();
        s.type_city("por".into());
        assert!(s.city_open, "typing opens it");
        s.toggle_city();
        assert!(!s.city_open);
        assert!(s.city_query.is_empty(), "cleared when toggled");
        s.toggle_city();
        s.type_city("mex".into());
        s.choose_city(3);
        assert_eq!(s.city_choice, Some(3));
        assert!(!s.city_open, "choosing shuts it");
    }
}
