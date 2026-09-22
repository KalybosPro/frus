//! The main screen: the task list itself, its rows, and what they open.
//!
//! It is a `StatefulWidget`. What it keeps is what only it cares about — the filter, which
//! overlay is open, which section is on show — and the tasks it draws come from the shared
//! [`Demo`], handed in as its configuration.

use crate::prelude::*;
use frus_widgets::{column, row, Semantics};

/// The main screen: its configuration is the shared state it draws from.
pub(crate) struct HomePage {
    pub(crate) demo: Rc<Demo>,
}

/// What the main screen keeps between rebuilds.
#[derive(Default)]
pub(crate) struct HomeState {
    /// The current filter.
    pub(crate) filter: Filter,
    /// Is the "clear completed" confirmation modal open?
    pub(crate) confirm_clear: bool,
    /// Is the side navigation drawer open?
    pub(crate) drawer_open: bool,
    /// Is the quick-actions modal sheet open?
    pub(crate) sheet_open: bool,
    /// Is the (header) actions menu open?
    pub(crate) actions_open: bool,
    /// The active section (0 = Tasks, 1 = Stats, 2 = About).
    pub(crate) section: usize,
    /// The metric selected in the Stats section (a master-detail TwoPane).
    pub(crate) stat_sel: usize,
    /// In single-pane (narrow) mode, is the Stats detail open?
    pub(crate) stat_detail_open: bool,
}

impl StatefulWidget for HomePage {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState::default()
    }
}

impl HomeState {
    /// Asks for a screen to be pushed: what was open is shut first — half of the app bar's
    /// actions navigate, and a menu left open while its screen is left comes back the next
    /// time that screen is shown.
    fn go(&self, cx: &StateContext<Self>, location: &'static str) -> Callback {
        let router = cx.router();
        let handle = cx.handle();
        on(move || {
            handle.set_state(|s| {
                s.drawer_open = false;
                s.actions_open = false;
            });
            router.push(location);
        })
    }
}

impl State for HomeState {
    type Widget = HomePage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let demo = cx.widget().demo.clone();
        let theme = cx.theme().clone();
        let theme = &theme;
        // A device set to a right-to-left language mirrors the layout before anything is
        // picked: the host is told when that changes.
        demo.sync_direction();

        // Responsiveness: a narrow window closes the Stats detail as it becomes narrow.
        let surface = MediaQuery::of();
        let class = surface.size_class();
        {
            let handle = cx.handle();
            cx.use_effect(class, move || {
                if class == SizeClass::Compact {
                    handle.set_state(|s| s.stat_detail_open = false);
                }
            });
        }

        let todos = demo.todos();
        let active = active_count(&todos);
        let done = done_count(&todos);
        let prefs = demo.prefs();
        let lang = demo.lang();

        // How wide the card is allowed to get — **a ceiling, not a width**. On a phone there
        // is none: the card fills what it is given, and everything inside it stretches to the
        // card. Wider windows cap it, because a line of prose across a desktop is unreadable.
        //
        // This used to be an arithmetic — the window minus the body's padding (24 × 2) minus
        // the card's own (20 × 2) — and it was wrong by eight pixels, because a card carries a
        // margin nobody had counted. It drew past its own card on every phone, and until
        // milestone 392 nothing said so: each parent quietly grew to match.
        let measure = match class {
            SizeClass::Compact => None,
            SizeClass::Medium => Some(560.0),
            SizeClass::Expanded => Some(680.0),
        };

        // The header: an adaptive AppBar. A title and some actions are declared; it decides on
        // its own how many fit on the line and folds the rest into a "⋯" overflow menu,
        // according to the width — without ever branching on mobile/desktop.
        let theme_label = if prefs.light { "Dark" } else { "Light" };
        // The title follows the active section (as a real app would) — the Tasks section is
        // localized (Fluent) for the i18n demo.
        let section_title = match self.section {
            1 => "Stats".to_string(),
            2 => "About".to_string(),
            _ => tr(lang, "app-title"),
        };
        let act = |change: fn(&Demo)| {
            let demo = demo.clone();
            on(move || change(&demo))
        };
        let header = AppBar::new()
            .title(frus_widgets::Text::new(section_title))
            .leading(
                // **The mark crosses as the drawer does.** Three bars while it is shut, a
                // cross while it is open, and the way between driven by the same `0 ↔ 1` the
                // drawer's own slide is driven by — the button is not running an animation of
                // its own, it is reading the same flag through the same machinery.
                // It was a text character (`☰`) until milestone 474.
                IconButton::animated(AnimatedIcons::MENU_CLOSE, self.drawer_open)
                    .label("Menu")
                    .icon_size(20.0)
                    .on_press(cx.callback(|s| s.drawer_open = !s.drawer_open)),
            )
            .overflow(
                self.actions_open,
                cx.callback(|s| s.actions_open = !s.actions_open),
            )
            .action(theme_label, act(Demo::toggle_theme))
            .action(seed_label(prefs.seed_index), act(Demo::cycle_seed))
            .action(if prefs.rtl { "LTR" } else { "RTL" }, act(Demo::toggle_rtl))
            // The language toggle: the label shows the language being switched TO.
            .action(lang_label(prefs.lang), act(Demo::cycle_lang))
            .action("A+", {
                let demo = demo.clone();
                on(move || demo.set_density(demo.prefs().density + 0.1))
            })
            .action("A−", {
                let demo = demo.clone();
                on(move || demo.set_density(demo.prefs().density - 0.1))
            })
            .action("Log →", self.go(cx, "/journal"))
            .action("Settings →", self.go(cx, "/settings"))
            .action(
                "Quick actions",
                cx.callback(|s| s.sheet_open = !s.sheet_open),
            )
            .action("Save", {
                let demo = demo.clone();
                on(move || demo.save())
            })
            .action(
                "Clear completed",
                cx.callback(|s| {
                    s.sheet_open = false;
                    s.confirm_clear = true;
                }),
            )
            .build();

        // Input: a field (Enter submits) + an add button. A non-empty field carries a
        // **clickable** "✕" suffix icon that clears it (milestone 198: a positional click on
        // the suffix). The text is held by a controller, which is what the field is driven by
        // and what the buttons read.
        let draft = cx.use_text_controller("");
        let add = {
            let (demo, draft) = (demo.clone(), draft.clone());
            on(move || {
                if demo.add(&draft.text()) {
                    draft.clear();
                }
            })
        };
        let mut draft_input = TextField::new(draft.text())
            .size(18.0)
            .controller(&draft)
            .on_submit(add.clone());
        if !draft.is_empty() {
            // Blank, `demo.add` refuses and clears nothing — so this is only reachable
            // when a submit really does add a task, which is exactly when the screen
            // reader should hear it. Found missing testing #18 in a real browser: a
            // field's Enter goes through neither of the two paths that already read a
            // widget's `announce()` (a click, or Enter/Space on a *clickable*), so
            // nothing here had ever been able to speak at all.
            draft_input = draft_input.announce(format!("{} added", draft.text()));
            let clear = draft.clone();
            draft_input = draft_input
                .suffix_icon(Icons::CLOSE)
                .on_suffix(on(move || clear.clear()));
        }
        // The field takes the room the button leaves — no subtraction, and it stays right
        // whatever the button's label ends up measuring.
        let input_row = row![Expanded::new(draft_input), button("Add", add.clone())]
            .align(Align::Center)
            .gap(10.0);

        // The filters: a segmented control (single selection).
        let segmented = SegmentedButton::new(
            self.filter.index(),
            cx.handler(|s, i: usize| s.filter = Filter::from_index(i)),
        )
        .segment(tr(lang, "filter-all"))
        .segment(tr(lang, "filter-active"))
        .segment(tr(lang, "filter-done"));
        let mut filters = row![segmented].align(Align::Center).gap(8.0);
        // The active filter (other than "All") is shown as a removable chip.
        if self.filter != Filter::All {
            let name = if self.filter == Filter::Active {
                "Active"
            } else {
                "Done"
            };
            filters = filters
                .child(spacer())
                .child(Chip::new(name).on_remove(cx.callback(|s| s.filter = Filter::All)));
        }

        // Two zones a held task can be carried to. They are `DragTarget`s and nothing else:
        // the highlight while a task hovers one is the target's own, from `Status`.
        let zone = |label: &str, done: bool| {
            let demo = demo.clone();
            DragTarget::new(
                Container::new()
                    .flex(1.0)
                    .padding(12.0)
                    .radius(10.0)
                    .color(theme.surface)
                    .child(
                        row![text(label).size(14.0).color(theme.muted)].justify(Justify::Center),
                    ),
            )
            .on_drop(on_value(move |payload: u64| demo.set_done(payload, done)))
        };
        let zones = row![zone("↓ Mark active", false), zone("✓ Mark done", true)].gap(8.0);

        // The filtered list (or the empty state), whose rows can be **dragged into a new
        // order**.
        //
        // By the grip, not by a hold, and not because a phone would rather have a hold: this
        // row already answers to a hold, which lifts it towards the two state zones. Three
        // gestures on one row is what the grip is for — it is the one that takes nothing away
        // from the others, and the reference's two listeners exist for exactly this choice.
        let filter = self.filter;
        let mut list = ReorderableList::new({
            let demo = demo.clone();
            on_values(move |from: usize, to: usize| demo.move_todo(filter, from, to))
        })
        .grab(ReorderGrab::Handle)
        .gap(8.0);
        let shown = visible(&todos, self.filter);
        for todo in &shown {
            // A stable identity by `id`: the retained state (hover/animations) does not jump
            // when a task in the middle is deleted — or moved past its neighbour, which is the
            // same question asked twice as often.
            list = list.keyed_row(todo.id, todo_row_draggable(&demo, cx, todo, theme));
        }
        let list: Box<dyn Widget> = if shown.is_empty() {
            Box::new(column![text("Nothing to show for this filter.")
                .size(18.0)
                .italic()
                .color(theme.muted)])
        } else {
            Box::new(list)
        };
        // **Vertical** responsiveness: in a short window the hint is hidden to preserve the
        // usable height. The scrolling is handled by the Scaffold.
        let short = SizeClass::from_height(surface.size.height) == SizeClass::Compact;

        // The footer: the counters + clear completed (with a modal confirmation).
        let ask_clear = cx.callback(|s| {
            s.sheet_open = false;
            s.confirm_clear = true;
        });
        let clear_button = button("Clear completed", ask_clear)
            .variant(Variant::Danger)
            .size(15.0);
        let clear = if self.confirm_clear {
            OverlayPortal::new(clear_button)
                .overlay(confirm_content(&demo, cx, done), Placement::Center)
                .dismiss(cx.callback(|s| s.confirm_clear = false))
        } else {
            OverlayPortal::new(clear_button)
        };
        let total = todos.len().max(1);
        let pct = (done as f32 / total as f32 * 100.0).round() as u32;

        // A summary built from its ACTUAL box (LayoutBuilder). Long text (pluralised counters,
        // localized through Fluent) when there is room, short text when it is narrow — at a
        // fixed height.
        let muted = theme.muted;
        let total = active + done;
        let summary = LayoutBuilder::new(move |size: Size| {
            let label = if size.width >= 360.0 {
                format!(
                    "{} · {} · {pct}%",
                    tr_n(lang, "task-count", total),
                    tr_n(lang, "remaining", active)
                )
            } else {
                format!("{active}·{done}")
            };
            text(label).size(16.0).color(muted)
        })
        .flex(1.0)
        .height(20.0);
        let footer = row![
            summary,
            button("Load", {
                let demo = demo.clone();
                on(move || demo.load())
            })
            .variant(Variant::Outlined)
            .size(15.0),
            button("Save", {
                let demo = demo.clone();
                on(move || demo.save())
            })
            .variant(Variant::Outlined)
            .size(15.0),
            clear,
        ]
        .align(Align::Center)
        .gap(8.0)
        // Three buttons and a summary need about 365 px; a phone's card is 323. Nobody is
        // squeezed any more (milestone 349), so the row would run past the card and say so.
        // Wrapping is the answer the reference gives too: the line that does not fit becomes
        // two lines.
        .wrap();

        // The completion progress bar (done / total).
        let progress = LinearProgressIndicator::new(done as f32 / total as f32);

        // The app's card, of responsive width, centred at the top of the screen. The body is
        // built incrementally so the hint can be left out when the window is short.
        let mut card_body = Flex::column().gap(16.0);
        if !short {
            // A **static** banner: a repaint boundary (milestone 88). It is replayed from the
            // cache on frames of pure interaction (hover, focus, scrolling elsewhere) instead
            // of being repainted every frame.
            card_body = card_body.child(
                Container::new().repaint_boundary().child(
                    Alert::new("Press Enter to add a task; swipe from the left edge to go back.")
                        .title("Tip"),
                ),
            );
            // A row of vector icons (milestone 89) + a bitmap image (milestone 90): tessellated
            // paths coloured by the theme, and a GPU texture fitted with `Cover`. The widget
            // showcase (~360 px) is wider than the card on a phone, so it **scrolls
            // horizontally** (at a fixed height, the row's) rather than overflowing.
            let showcase = Flex::row()
                .gap(16.0)
                .align(Align::Center)
                .child(Icon::new(Icons::CHECK).color(theme.primary))
                .child(Icon::new(Icons::STAR))
                .child(Icon::new(Icons::FAVORITE))
                .child(Icon::new(Icons::MENU))
                .child(Icon::new(Icons::CHEVRON_RIGHT))
                // **The same logo, twice over** (milestone 481): once as a picture at the size
                // it was given, and once as an *icon* — where it answers the ambient icon size
                // and stands level with the five paths to its left without being told what
                // they are.
                .child(ImageIcon::new(demo_logo()))
                .child(demo_logo().size(72.0, 48.0).fit(BoxFit::Cover))
                // A group-opacity layer (milestone 92): two overlapping squares, composited as
                // one → the overlap does not darken (no double-blending of the alpha).
                .child(CustomPaint::new(72.0, 48.0, |scene, bounds, theme| {
                    scene.layer(0.55, |inner| {
                        let c = theme.primary;
                        inner.fill_rect(Rect::new(bounds.x + 6.0, bounds.y + 8.0, 32.0, 32.0), c);
                        inner.fill_rect(Rect::new(bounds.x + 30.0, bounds.y + 8.0, 32.0, 32.0), c);
                    });
                }));
            card_body = card_body.child(
                SingleChildScrollView::new()
                    .axis(Axis::Horizontal)
                    .height(52.0)
                    .child(showcase),
            );
        }
        // **Stable** identities (keys): the hint above is conditional — without keys, its
        // disappearance (an open keyboard → a short screen) shifts the siblings' positional ids
        // and the retained state (the field's focus!) jumps.
        card_body = card_body
            .child(keyed("draft-row", input_row))
            .child(keyed(
                "filters",
                SingleChildScrollView::new()
                    .child(filters)
                    .axis(Axis::Horizontal),
            ))
            .child(keyed("drop-zones", zones))
            .child(keyed("todo-list", list))
            .child(Divider::new())
            .child(progress)
            .child(footer);
        let card = Card::new().padding(20.0).child(card_body);
        // On a phone the card **is** the body's width; on a wide window it is capped and
        // centred. Either way the number below is a ceiling the design chose, never a
        // measurement of the screen.
        let placed: Box<dyn Widget> = match measure {
            Some(cap) => {
                Box::new(row![ConstrainedBox::new(card).max_width(cap)].justify(Justify::Center))
            }
            None => Box::new(card),
        };
        let tasks_body = column![placed].padding(24.0);

        // The body follows the active section (the adaptive navigation lives in the Scaffold).
        //
        // **Each section says whether it scrolls**, because each of the three answers
        // differently (milestone 321: the Scaffold no longer decides this for them). Tasks
        // grows with the list and About is a long read, so both go in a
        // `SingleChildScrollView`; Stats is a master-detail pane sized to the size class, and
        // wrapping it would give the screen a scrollable with nothing to scroll.
        let section: Box<dyn Widget> = match self.section {
            1 => Box::new(self.stats_section(cx, theme, class, &todos)),
            2 => Box::new(
                SingleChildScrollView::new()
                    .flex(1.0)
                    .child(about_section(theme)),
            ),
            _ => Box::new(SingleChildScrollView::new().flex(1.0).child(tasks_body)),
        };

        // The screen's skeleton: the Scaffold pins the top bar and the navigation, places the
        // body, and coordinates the drawer / sheet / FAB — a single entry point. It takes no
        // size and is told no insets: both come from the surface description the shell
        // installed, and the Scaffold keeps its own slots clear of the bars and the notch
        // (milestone 393).
        let toggle_drawer = cx.callback(|s| s.drawer_open = !s.drawer_open);
        let toggle_sheet = cx.callback(|s| s.sheet_open = !s.sheet_open);
        let scaffold = Scaffold::new()
            .background(theme.background)
            .app_bar(header)
            .body(section)
            // A bottom bar, at every width — the default, and left unsaid on purpose so
            // that this reads the way an application would write it. Before milestone 305
            // the scaffold measured its own width and moved the navigation to a side rail
            // past a threshold, which meant turning the phone to landscape relocated it.
            // `.nav_placement(NavPlacement::Rail)` pins a rail instead; navigation that
            // follows the size class is `NavScaffold`, which is a different widget.
            .nav(
                self.section,
                cx.handler(|s, i: usize| {
                    s.section = i;
                    // Choosing a section from the drawer closes it.
                    s.drawer_open = false;
                }),
            )
            // **One list.** The bar here, the drawer below and — in an application that used
            // `NavScaffold` — the rail at a wider size all read the same declaration, so
            // there is nowhere for the three to drift apart. Milestone 473.
            .destinations(sections(active))
            .end_drawer(
                self.drawer_menu(cx, theme, active),
                self.drawer_open,
                toggle_drawer,
            )
            // Floating, not docked (milestone 290). Docking was tried here first and the
            // device settled it: this bar carries three destinations, so a button astride
            // its top edge lands on one of them. Docking is for a bar cut with a notch to
            // receive it, which frus has not got yet.
            .fab_location(FabLocation::EndFloat)
            // **The button gets out of the way of the sheet**, rather than sitting on top of
            // it or blinking out. Two heights down is clear of the bar it floats over, and
            // the number is a multiple of the button's own box — so nobody here has to know
            // how big a floating action button is, which is the whole point of a slide being
            // stated as a fraction (milestone 488).
            .fab(AnimatedSlide::new(
                0.0,
                if self.sheet_open { 2.0 } else { 0.0 },
                0.22,
                Curve::ease_out(),
                fab_button("+", add),
            ))
            .bottom_sheet(
                quick_actions_sheet(&demo, cx, theme),
                self.sheet_open,
                toggle_sheet,
            )
            .build();

        // The notification at the head of the queue floats above everything, anchored
        // bottom-centre by the `ScaffoldMessenger` layer (milestone 188): it fades **in**,
        // then fades **out** when it moves into its exit before being removed (milestone 193).
        match demo.current_toast() {
            Some((message, leaving)) => {
                let host = ScaffoldMessenger::new(SnackBarPosition::BottomCenter)
                    .toast(SnackBar::new(message).success());
                let host = if leaving {
                    host.fade_out(0.3)
                } else {
                    host.fade_in(0.25)
                };
                Box::new(
                    Stack::new()
                        .width(surface.size.width)
                        .height(surface.size.height)
                        .layer(scaffold)
                        .layer(host),
                )
            }
            None => scaffold,
        }
    }
}

impl HomeState {
    /// The "Stats" section: a responsive master-detail layout (`TwoPane`). Side by side when
    /// large, a single pane when narrow (tapping a metric opens the detail).
    fn stats_section(
        &self,
        cx: &StateContext<Self>,
        theme: &Theme,
        class: SizeClass,
        todos: &[Todo],
    ) -> TwoPane {
        let metrics = [
            ("Total tasks", todos.len()),
            ("Active tasks", active_count(todos)),
            ("Completed", done_count(todos)),
        ];

        // The master pane: the list of metrics (a selection).
        let mut cats = Flex::column().gap(6.0);
        for (i, (label, _)) in metrics.iter().enumerate() {
            let variant = if self.stat_sel == i {
                Variant::Filled
            } else {
                Variant::Outlined
            };
            cats = cats.child(
                button(
                    *label,
                    cx.callback(move |s| {
                        // Which metric is shown, and — when narrow — the detail opens.
                        s.stat_sel = i;
                        s.stat_detail_open = true;
                    }),
                )
                .variant(variant)
                .size(15.0),
            );
        }
        let list = Card::new().padding(12.0).child(cats);

        // The detail pane: the selected metric.
        let (label, value) = metrics[self.stat_sel.min(metrics.len() - 1)];
        // The number **changes in place** rather than jumping — a task ticked, another metric
        // picked — and the old figure shrinks away as the new one grows in over it.
        let primary = theme.primary;
        let figure = AnimatedSwitcher::new(0.25, value, move |n: &usize| {
            text(n.to_string()).size(44.0).color(primary)
        })
        .switch_in_curve(Curve::ease_out())
        .transition(|child, t| ScaleTransition::new(0.6 + 0.4 * t, FadeTransition::new(t, child)));
        let mut detail_col = column![
            text(label).size(22.0),
            figure,
            text("Detail for the selected metric.")
                .size(14.0)
                .color(theme.muted),
        ]
        .gap(10.0);
        // In single-pane mode, a way back to the list.
        if class != SizeClass::Expanded {
            detail_col = detail_col.child(
                button("← Back", cx.callback(|s| s.stat_detail_open = false))
                    .variant(Variant::Outlined)
                    .size(15.0),
            );
        }
        let detail = Card::new().padding(20.0).child(detail_col);

        TwoPane::new(class)
            .ratio(0.36)
            .show_detail(self.stat_detail_open)
            .list(list)
            .detail(detail)
    }

    /// The navigation drawer's content: a header + the destinations + settings.
    ///
    /// Wrapped in a `SafeArea`: a drawer is an **overlay**, so it is placed against the window
    /// and not inside the padded box `view` builds the rest of the interface in — its title
    /// came out under the status bar (found on the device, 2026-08-16). The reference has the
    /// same shape of answer: its drawer runs the full height and the header adds the status
    /// bar's own height to its padding.
    fn drawer_menu(&self, cx: &StateContext<Self>, theme: &Theme, active: usize) -> SafeArea {
        let entry = |label: &str, index: usize| {
            let here = self.section == index;
            let variant = if here {
                Variant::Filled
            } else {
                Variant::Outlined
            };
            // **The section you are on makes room for itself.** An inset that jumps when the
            // selection moves reads as a relayout; one that slides reads as the selection
            // moving, which is what actually happened. `AnimatedPadding` is layout, not
            // paint, so the entries below really do move aside — milestone 477.
            AnimatedPadding::new(
                if here { 6.0 } else { 0.0 },
                0.18,
                Curve::ease_out(),
                button(
                    label.to_string(),
                    cx.callback(move |s| {
                        s.section = index;
                        s.drawer_open = false;
                    }),
                )
                .variant(variant)
                .size(16.0),
            )
        };
        let link = |label: &str, location: &'static str| {
            button(label.to_string(), self.go(cx, location))
                .variant(Variant::Outlined)
                .size(15.0)
        };
        // The same declaration the bottom bar reads, so the menu cannot name a section the
        // bar has not got, or call it something else.
        let sections = sections(active);
        SafeArea::new(
            Container::new().padding(16.0).child(
                column![
                    text("frus").size(22.0),
                    text("Navigation").size(13.0).color(theme.muted),
                    Divider::new(),
                    entry(sections[0].label(), 0),
                    entry(sections[1].label(), 1),
                    entry(sections[2].label(), 2),
                    Divider::new(),
                    text(format!("{active} task(s) pending"))
                        .size(14.0)
                        .color(theme.muted),
                    link("Settings →", "/settings"),
                    link("Sign-up wizard →", "/wizard"),
                    link("Editable grid →", "/grid"),
                    link("Charts →", "/charts"),
                    link("Data table →", "/data"),
                    link("Guided tour →", "/tour"),
                    link("Draggable sheet →", "/sheet"),
                    link("Kanban board →", "/board"),
                ]
                .gap(12.0),
            ),
        )
    }
}

/// One task row, **swipeable**: dragging it sideways past 40 % of its width — or
/// flicking it — deletes it, the same thing the × and the long press already do. The
/// row's height is explicit because a `Dismissible` overlays its background under its
/// child, which makes it a layout leaf.
///
/// The row, made **liftable**: held down, it can be carried to one of the state zones
/// below the filters.
///
/// It lifts on a **hold**, not on the first movement, because the same finger on the
/// same row already means two other things — dragging sideways dismisses it, dragging
/// up and down scrolls the list. Three gestures on one row, told apart by what the
/// finger does rather than by what is on top.
pub(crate) fn todo_row_draggable(
    demo: &Rc<Demo>,
    cx: &BuildContext,
    todo: &Todo,
    theme: &Theme,
) -> Draggable {
    Draggable::new(todo_row_swipeable(demo, cx, todo, theme))
        .payload(todo.id)
        .long_press()
}

pub(crate) fn todo_row_swipeable(
    demo: &Rc<Demo>,
    cx: &BuildContext,
    todo: &Todo,
    theme: &Theme,
) -> Dismissible {
    let id = todo.id;
    let demo = demo.clone();
    Dismissible::new(todo_row(&demo, cx, todo, theme))
        .height(TODO_ROW_HEIGHT)
        .on_dismiss(on(move || demo.delete(id)))
        .background(
            Container::new()
                .radius(10.0)
                .color(theme.error)
                .padding_each(0.0, 16.0, 0.0, 16.0)
                .child(row![text("Delete").size(16.0).color(theme.on_error)].align(Align::Center)),
        )
}

/// The height of a task row. Fixed, because a swipeable row is a layout leaf.
///
/// Sixty-six, not sixty-two: the checkbox and the delete button each reserve a 48-pixel
/// tap target (milestone 442), and around them this row has 8 pixels of padding above and
/// below plus a one-pixel rule. It was pinned at 62 when those controls were 20 and 40,
/// and the framework's own overflow check said so on all nine screens at once the moment
/// they grew — which is the instrument working.
pub(crate) const TODO_ROW_HEIGHT: f32 = 66.0;

/// One task row: a checkbox, the label (dimmed **and struck through** when done) and a delete
/// button.
///
/// The label's **type is handed down rather than stated on it**, and it moves. Ticking a
/// task changes its colour and strikes it through, and until milestone 498 both happened
/// between one frame and the next — the row read as a different row rather than as the
/// same one, answered. The colour glides; the line, which cannot be half-drawn, appears at
/// the middle of the movement.
///
/// It has to be handed down, not set on the text: a field the caller states on a
/// particular run of words wins over what its subtree says, so a `.color()` here would
/// quietly ignore the animation. What the label still states for itself is what does not
/// change with the task's state.
pub(crate) fn todo_row(
    demo: &Rc<Demo>,
    cx: &BuildContext,
    todo: &Todo,
    theme: &Theme,
) -> Container {
    let id = todo.id;
    let router = cx.router();
    let done_style = frus_widgets::TextStyle::NONE
        .color(if todo.done {
            theme.muted
        } else {
            theme.on_surface
        })
        .decoration(if todo.done {
            frus_widgets::TextDecoration::STRIKETHROUGH
        } else {
            frus_widgets::TextDecoration::NONE
        });
    let label = AnimatedDefaultTextStyle::new(
        0.2,
        Curve::ease_out(),
        done_style,
        text(todo.text.clone()).size(18.0).ellipsis(),
    );
    let (toggle, delete) = (demo.clone(), demo.clone());
    // The checkbox has nothing beside it a screen reader can read — its caption is the
    // row's own text, drawn separately so it can animate independently (see `label`
    // above) — so alone it announced "checkbox, ticked" with no way to tell which task.
    // Found testing #18 in a real browser: `Semantics::merge` joins the two into one
    // control, the framework's own answer to exactly this shape, so the reader hears
    // both without the text being drawn twice.
    let checkbox_and_label = Expanded::new(Semantics::merge(
        row![
            Checkbox::new(todo.done).on_toggle(on_value(move |_: bool| toggle.toggle(id))),
            // Cut with an ellipsis at the width the row leaves — laid out by its own
            // content instead, a long task title pushed the delete button off the card
            // and out of the hit registry: the task could no longer be deleted
            // (milestones 333 and 334).
            Expanded::new(label),
        ]
        .align(Align::Center)
        .gap(10.0),
    ));
    let line = row![
        // The shared element: the same avatar, tagged by the task's id, appears bigger
        // on the task's own screen and flies between the two.
        Container::new()
            .on_click(on(move || router.push(format!("/task/{id}"))))
            .child(Hero::new(
                id,
                CircleAvatar::new(todo.text.clone()).size(30.0)
            )),
        // No `spacer()` is needed — the expanding pair is what pushes the button to the
        // right edge (milestone 334).
        checkbox_and_label,
        IconButton::new(Icons::CLOSE)
            .label("Delete task")
            .icon_color(theme.error)
            .icon_size(18.0)
            .on_press(on(move || delete.delete(id))),
    ]
    .align(Align::Center)
    .gap(12.0)
    // Fills the card. A row is sized by its content on its own main axis, so without
    // this it is only as wide as its children and the expanding label has nothing to
    // expand into — the delete button then sits against the label instead of the card's
    // right edge (milestone 334).
    .flex(1.0);
    Container::new()
        // No long press here: the hold is what **lifts** the row for dragging
        // (`todo_row_draggable`), and one hold cannot mean two things. Deleting is the
        // ×, or a swipe.
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.outline_variant)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(line)
}

/// Content of the "clear completed" confirmation modal.
fn confirm_content(demo: &Rc<Demo>, cx: &StateContext<HomeState>, done: usize) -> Card {
    let confirm = {
        let (demo, handle) = (demo.clone(), cx.handle());
        on(move || {
            demo.clear_done();
            handle.set_state(|s| s.confirm_clear = false);
        })
    };
    Card::new().padding(24.0).child(
        column![
            text("Clear completed tasks?")
                .size(22.0)
                .weight(FontWeight::Medium),
            text(format!("{done} task(s) will be removed.")).size(16.0),
            row![
                button("Cancel", cx.callback(|s| s.confirm_clear = false))
                    .variant(Variant::Outlined),
                button("Delete", confirm).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// The modal sheet's content: a few quick actions.
fn quick_actions_sheet(demo: &Rc<Demo>, cx: &StateContext<HomeState>, theme: &Theme) -> Container {
    let save = {
        let (demo, handle) = (demo.clone(), cx.handle());
        on(move || {
            handle.set_state(|s| s.sheet_open = false);
            demo.save();
        })
    };
    Container::new().padding(20.0).child(
        Flex::column()
            .gap(12.0)
            .child(text("Quick actions").size(20.0).color(theme.on_surface))
            .child(button("💾  Save", save).variant(Variant::Filled).size(16.0))
            .child(
                button(
                    "🗑  Clear completed",
                    cx.callback(|s| {
                        s.sheet_open = false;
                        s.confirm_clear = true;
                    }),
                )
                .variant(Variant::Outlined)
                .size(16.0),
            )
            .child(
                button("Close", cx.callback(|s| s.sheet_open = false))
                    .variant(Variant::Outlined)
                    .size(16.0),
            ),
    )
}

/// **The application's three sections, declared once.**
///
/// Each is a drawn icon at rest and its solid twin when selected — the convention the
/// icon set is drawn for, and the thing a colour alone cannot say. `active` is the number
/// of tasks still to do, which the first destination wears as a badge; `0` shows nothing,
/// so the count goes straight in with no `if` around it.
pub(crate) fn sections(active: usize) -> Vec<NavigationDestination> {
    vec![
        NavigationDestination::new(Icons::CHECK_CIRCLE_OUTLINE, "Tasks")
            .selected_icon(Icons::CHECK_CIRCLE)
            .badge(active as u32)
            .tooltip("What is still to do"),
        NavigationDestination::new(Icons::BAR_CHART, "Stats")
            .selected_icon(Icons::INSERT_CHART)
            .tooltip("How the week went"),
        NavigationDestination::new(Icons::STAR_BORDER, "About")
            .selected_icon(Icons::STAR)
            .tooltip("What this is"),
    ]
}
