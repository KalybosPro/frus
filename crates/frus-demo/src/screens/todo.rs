//! The main screen: the task list itself, its rows, and what they open.

use crate::prelude::*;
use frus_widgets::{column, row};

/// One task row, **swipeable**: dragging it sideways past 40 % of its width — or
/// flicking it — deletes it, the same thing the × and the long press already do. The
/// row's height is explicit because a `Dismissible` overlays its background under its
/// child, which makes it a layout leaf.
/// The row, made **liftable**: held down, it can be carried to one of the state zones
/// below the filters.
///
/// It lifts on a **hold**, not on the first movement, because the same finger on the
/// same row already means two other things — dragging sideways dismisses it, dragging
/// up and down scrolls the list. Three gestures on one row, told apart by what the
/// finger does rather than by what is on top.
pub(crate) fn todo_row_draggable(todo: &Todo, theme: &Theme) -> Draggable<Msg> {
    Draggable::new(todo_row_swipeable(todo, theme))
        .payload(todo.id)
        .long_press()
}

pub(crate) fn todo_row_swipeable(todo: &Todo, theme: &Theme) -> Dismissible<Msg> {
    Dismissible::new(todo_row(todo, theme))
        .height(TODO_ROW_HEIGHT)
        .on_dismiss(Msg::DeleteTodo(todo.id))
        .background(
            Container::new()
                .radius(10.0)
                .color(theme.error)
                .padding_each(0.0, 16.0, 0.0, 16.0)
                .child(row![text("Delete").size(16.0).color(theme.on_error)].align(Align::Center)),
        )
}

/// The height of a task row. Fixed, because a swipeable row is a layout leaf.
pub(crate) const TODO_ROW_HEIGHT: f32 = 62.0;

/// One task row: a checkbox, the label (dimmed **and struck through** when done) and a delete
/// button.
pub(crate) fn todo_row(todo: &Todo, theme: &Theme) -> Container<Msg> {
    let id = todo.id;
    let label_color = if todo.done {
        theme.muted
    } else {
        theme.on_surface
    };
    let mut label = text(todo.text.clone()).size(18.0).color(label_color);
    if todo.done {
        label = label.strikethrough();
    }
    let line = row![
        // The shared element: the same avatar, tagged by the task's id, appears bigger
        // on the task's own screen and flies between the two.
        Container::new()
            .on_click(Msg::OpenTask(id))
            .child(Hero::new(id, Avatar::new(todo.text.clone()).size(30.0))),
        Checkbox::new(todo.done).on_toggle(move |_| Msg::ToggleTodo(id)),
        label,
        spacer(),
        IconButton::new(IconName::Close)
            .label("Delete task")
            .icon_color(theme.error)
            .icon_size(18.0)
            .on_press(Msg::DeleteTodo(id)),
    ]
    .align(Align::Center)
    .gap(12.0);
    Container::new()
        // No long press here: the hold is what **lifts** the row for dragging
        // (`todo_row_draggable`), and one hold cannot mean two things. Deleting is the
        // ×, or a swipe.
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(line)
}

/// Content of the data table's bulk-delete confirmation modal (milestone 245).
pub(crate) fn data_confirm_content(count: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Delete selected rows?")
                .size(22.0)
                .weight(FontWeight::Medium),
            text(format!("{count} row(s) will be removed.")).size(16.0),
            row![
                button("Cancel", Msg::DataCancelDelete).variant(Variant::Outlined),
                button("Delete", Msg::DataDeleteChecked).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// Content of the "clear completed" confirmation modal.
pub(crate) fn confirm_content(done: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Clear completed tasks?")
                .size(22.0)
                .weight(FontWeight::Medium),
            text(format!("{done} task(s) will be removed.")).size(16.0),
            row![
                button("Cancel", Msg::CancelClear).variant(Variant::Outlined),
                button("Delete", Msg::ConfirmClearDone).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// The main screen: the task list (the sample app itself).
pub(crate) fn todo_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Box<dyn Widget<Msg>> {
    let active = active_count(app);
    let done = done_count(app);

    // Responsiveness: the card widens with the window in steps. In Compact it follows the
    // available width, and the fields inside adapt to it.
    let class = SizeClass::from_width(width);
    // The card's **inner** width: the window minus the body's padding (24 × 2) and the card's own
    // (20 × 2) — otherwise the card overflows the Compact viewport and the whole screen scrolls
    // horizontally.
    let card_width = match class {
        SizeClass::Compact => (width - 88.0).max(240.0),
        SizeClass::Medium => 560.0,
        SizeClass::Expanded => 680.0,
    };

    // The header: an adaptive AppBar. A title and some actions are declared; it decides on its
    // own how many fit on the line and folds the rest into a "⋯" overflow menu, according to the
    // width — without ever branching on mobile/desktop.
    let theme_label = if app.light { "Dark" } else { "Light" };
    let timer_label = if app.running { "Pause" } else { "Resume" };
    // The title follows the active section (as a real app would) — the Tasks section is
    // localized (Fluent) for the i18n demo.
    let section_title = match app.section {
        1 => "Stats".to_string(),
        2 => "About".to_string(),
        _ => tr(app.lang, "app-title"),
    };
    let header = AppBar::new(section_title)
        .width(width)
        .leading(
            IconButton::glyph("☰")
                .label("Menu")
                .icon_size(20.0)
                .on_press(Msg::ToggleDrawer),
        )
        .overflow(app.actions_open, Msg::ToggleActions)
        .action(timer_label, Msg::ToggleTimer)
        .action(theme_label, Msg::ToggleTheme)
        .action(seed_label(app), Msg::CycleSeed)
        .action(if app.rtl { "LTR" } else { "RTL" }, Msg::ToggleRtl)
        // The language toggle: the label shows the language being switched TO.
        .action(LANGS[(app.lang + 1) % LANGS.len()].0, Msg::CycleLang)
        .action("A+", Msg::SetDensity(app.density + 0.1))
        .action("A−", Msg::SetDensity(app.density - 0.1))
        .action("Log →", Msg::Push(Route::Journal))
        .action("Settings →", Msg::Push(Route::Settings))
        .action("Quick actions", Msg::ToggleSheet)
        .action("Save", Msg::Save)
        .action("Clear completed", Msg::AskClearDone)
        .build();

    // Input: a field (Enter submits) + an add button. A non-empty field carries a **clickable**
    // "✕" suffix icon that clears it (milestone 198: a positional click on the suffix).
    let mut draft_input = TextInput::new(app.draft.as_str())
        .width((card_width - 150.0).max(160.0))
        .size(18.0)
        .on_input(Msg::DraftChanged)
        .on_submit(Msg::AddTodo);
    if !app.draft.is_empty() {
        draft_input = draft_input
            .suffix_icon(IconName::Close)
            .on_suffix(Msg::ClearDraft);
    }
    let input_row = row![draft_input, button("Add", Msg::AddTodo)]
        .align(Align::Center)
        .gap(10.0);

    // The filters: a segmented control (single selection).
    let segmented = SegmentedControl::new(filter_index(app.filter), |i| {
        Msg::SetFilter(filter_from_index(i))
    })
    .segment(tr(app.lang, "filter-all"))
    .segment(tr(app.lang, "filter-active"))
    .segment(tr(app.lang, "filter-done"));
    let mut filters = row![segmented].align(Align::Center).gap(8.0);
    // The active filter (other than "All") is shown as a removable chip.
    if app.filter != Filter::All {
        let name = if app.filter == Filter::Active {
            "Active"
        } else {
            "Done"
        };
        filters = filters
            .child(spacer())
            .child(Chip::new(name).on_remove(Msg::SetFilter(Filter::All)));
    }

    // Two zones a held task can be carried to. They are `DragTarget`s and nothing else:
    // the highlight while a task hovers one is the target's own, from `Status`.
    let zone = |label: &str, done: bool, theme: &Theme| {
        DragTarget::new(
            Container::new()
                .flex(1.0)
                .padding(12.0)
                .radius(10.0)
                .color(theme.surface)
                .child(row![text(label).size(14.0).color(theme.muted)].justify(Justify::Center)),
        )
        .on_drop(move |payload| Msg::SetTodoDone(payload, done))
    };
    let zones = row![
        zone("↓ Mark active", false, theme),
        zone("✓ Mark done", true, theme)
    ]
    .gap(8.0);

    // The filtered list (or the empty state).
    let mut list = Flex::column().gap(8.0);
    let mut shown = 0;
    for todo in app.todos.iter().filter(|t| match app.filter {
        Filter::All => true,
        Filter::Active => !t.done,
        Filter::Done => t.done,
    }) {
        // A stable identity by `id`: the retained state (hover/animations) does not jump when a
        // task in the middle is deleted.
        list = list.child(keyed(todo.id, todo_row_draggable(todo, theme)));
        shown += 1;
    }
    if shown == 0 {
        list = column![text("Nothing to show for this filter.")
            .size(18.0)
            .italic()
            .color(theme.muted)];
    }
    // **Vertical** responsiveness: in a short window the hint is hidden to preserve the usable
    // height. The scrolling is handled by the Scaffold.
    let short = SizeClass::from_height(height) == SizeClass::Compact;

    // The footer: the counters + clear completed (with a modal confirmation).
    let clear_button = button("Clear completed", Msg::AskClearDone)
        .variant(Variant::Danger)
        .size(15.0);
    let clear = if app.confirm_clear {
        Portal::new(clear_button)
            .overlay(confirm_content(done), Placement::Center)
            .dismiss(Msg::CancelClear)
    } else {
        Portal::new(clear_button)
    };
    let total = app.todos.len().max(1);
    let pct = (done as f32 / total as f32 * 100.0).round() as u32;

    // A summary built from its ACTUAL box (LayoutBuilder). Long text (pluralised counters,
    // localized through Fluent) when there is room, short text when it is narrow — at a fixed
    // height.
    let muted = theme.muted;
    let lang = app.lang;
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
        button("Load", Msg::Load)
            .variant(Variant::Outlined)
            .size(15.0),
        button("Save", Msg::Save)
            .variant(Variant::Outlined)
            .size(15.0),
        clear,
    ]
    .align(Align::Center)
    .gap(8.0);

    // The completion progress bar (done / total).
    let progress =
        ProgressBar::new(done as f32 / total as f32).width((card_width - 40.0).max(200.0));

    // The app's card, of responsive width, centred at the top of the screen. The body is built
    // incrementally so the hint can be left out when the window is short.
    let mut card_body = Flex::column().width(card_width).gap(16.0);
    if !short {
        // A **static** banner: a repaint boundary (milestone 88). It is replayed from the cache
        // on frames of pure interaction (hover, focus, scrolling elsewhere) instead of being
        // repainted every frame.
        card_body = card_body.child(
            Container::new().repaint_boundary().child(
                Alert::new("Press Enter to add a task; swipe from the left edge to go back.")
                    .title("Tip"),
            ),
        );
        // A row of vector icons (milestone 89) + a bitmap image (milestone 90): tessellated paths
        // coloured by the theme, and a GPU texture fitted with `Cover`. The widget showcase
        // (~360 px) is wider than the card on a phone, so it **scrolls horizontally** (at a fixed
        // height, the row's) rather than overflowing.
        let showcase = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(Icon::new(IconName::Check).color(theme.primary))
            .child(Icon::new(IconName::Star))
            .child(Icon::new(IconName::Heart))
            .child(Icon::new(IconName::Menu))
            .child(Icon::new(IconName::ChevronRight))
            .child(Image::new(demo_image(), 72.0, 48.0).fit(BoxFit::Cover))
            // A group-opacity layer (milestone 92): two overlapping squares, composited as one →
            // the overlap does not darken (no double-blending of the alpha).
            .child(CustomPaint::new(72.0, 48.0, |scene, bounds, theme| {
                scene.layer(0.55, |inner| {
                    let c = theme.primary;
                    inner.fill_rect(Rect::new(bounds.x + 6.0, bounds.y + 8.0, 32.0, 32.0), c);
                    inner.fill_rect(Rect::new(bounds.x + 30.0, bounds.y + 8.0, 32.0, 32.0), c);
                });
            }));
        card_body = card_body.child(
            Scroll::new()
                .axis(Axis::Horizontal)
                .width(card_width)
                .height(52.0)
                .child(showcase),
        );
    }
    // **Stable** identities (keys): the hint above is conditional — without keys, its
    // disappearance (an open keyboard → a short screen) shifts the siblings' positional ids and
    // the retained state (the field's focus!) jumps.
    card_body = card_body
        .child(keyed("draft-row", input_row))
        .child(keyed("filters", filters))
        .child(keyed("drop-zones", zones))
        .child(keyed("todo-list", list))
        .child(Divider::new())
        .child(progress)
        .child(footer);
    let card = Card::new().padding(20.0).child(card_body);
    let tasks_body = column![row![card].justify(Justify::Center)].padding(24.0);

    // The body follows the active section (the adaptive navigation lives in the Scaffold).
    let section: Box<dyn Widget<Msg>> = match app.section {
        1 => Box::new(stats_section(app, theme, class)),
        2 => Box::new(about_section(theme, width)),
        _ => Box::new(tasks_body),
    };

    // The screen's skeleton: the Scaffold pins the top bar and the navigation, scrolls the body,
    // and coordinates the drawer / sheet / FAB — a single entry point. The insets are already
    // handled by `view` (which passes safe dimensions), so the Scaffold simply pins itself inside
    // that viewport.
    let scaffold = Scaffold::new(width, height)
        .background(theme.background)
        .app_bar(header)
        .body(section)
        // A bottom bar, at every width — the default, and left unsaid on purpose so
        // that this reads the way an application would write it. Before milestone 305
        // the scaffold measured its own width and moved the navigation to a side rail
        // past a threshold, which meant turning the phone to landscape relocated it.
        // `.nav_placement(NavPlacement::Rail)` pins a rail instead; navigation that
        // follows the size class is `NavScaffold`, which is a different widget.
        .nav(app.section, Msg::SetSection)
        .destination("✔", "Tasks")
        .badge(active as u32)
        .destination("▦", "Stats")
        .destination("★", "About")
        .end_drawer(
            drawer_menu(app, theme, active),
            app.drawer_open,
            Msg::ToggleDrawer,
        )
        // Floating, not docked (milestone 290). Docking was tried here first and the
        // device settled it: this bar carries three destinations, so a button astride
        // its top edge lands on one of them. Docking is for a bar cut with a notch to
        // receive it, which frus has not got yet.
        .fab_location(FabLocation::EndFloat)
        .fab(fab_button("+", Msg::AddTodo))
        .bottom_sheet(quick_actions_sheet(theme), app.sheet_open, Msg::ToggleSheet)
        .build();

    // The notification at the head of the queue floats above everything, anchored bottom-centre by
    // the `ToastHost` layer (milestone 188): it fades **in**, then fades **out** when it moves into
    // its exit before being removed (milestone 193).
    match app.snackbars.current() {
        Some(message) => {
            let host = ToastHost::new(ToastPosition::BottomCenter)
                .toast(Toast::new(message.clone()).success());
            let host = if app.snackbars.is_leaving() {
                host.fade_out(0.3)
            } else {
                host.fade_in(0.25)
            };
            Box::new(
                Stack::new()
                    .width(width)
                    .height(height)
                    .layer(scaffold)
                    .layer(host),
            )
        }
        None => scaffold,
    }
}

/// The modal sheet's content: a few quick actions.
pub(crate) fn quick_actions_sheet(theme: &Theme) -> Container<Msg> {
    Container::new().padding(20.0).child(
        Flex::column()
            .gap(12.0)
            .child(text("Quick actions").size(20.0).color(theme.on_surface))
            .child(
                button("💾  Save", Msg::Save)
                    .variant(Variant::Filled)
                    .size(16.0),
            )
            .child(
                button("🗑  Clear completed", Msg::AskClearDone)
                    .variant(Variant::Outlined)
                    .size(16.0),
            )
            .child(
                button("Close", Msg::ToggleSheet)
                    .variant(Variant::Outlined)
                    .size(16.0),
            ),
    )
}

/// The navigation drawer's content: a header + the destinations + settings.
pub(crate) fn drawer_menu(app: &TodoApp, theme: &Theme, active: usize) -> Container<Msg> {
    let entry = |icon: &str, label: &str, index: usize| {
        let variant = if app.section == index {
            Variant::Filled
        } else {
            Variant::Outlined
        };
        button(format!("{icon}  {label}"), Msg::SetSection(index))
            .variant(variant)
            .size(16.0)
    };
    Container::new().padding(16.0).child(
        column![
            text("frus").size(22.0),
            text("Navigation").size(13.0).color(theme.muted),
            Divider::new(),
            entry("✔", "Tasks", 0),
            entry("▦", "Stats", 1),
            entry("★", "About", 2),
            Divider::new(),
            text(format!("{active} task(s) pending"))
                .size(14.0)
                .color(theme.muted),
            button("Settings →", Msg::Push(Route::Settings))
                .variant(Variant::Outlined)
                .size(15.0),
            button("Sign-up wizard →", Msg::Push(Route::Wizard))
                .variant(Variant::Outlined)
                .size(15.0),
            button("Editable grid →", Msg::Push(Route::Grid))
                .variant(Variant::Outlined)
                .size(15.0),
            button("Charts →", Msg::Push(Route::Charts))
                .variant(Variant::Outlined)
                .size(15.0),
            button("Data table →", Msg::Push(Route::Data))
                .variant(Variant::Outlined)
                .size(15.0),
            button("Guided tour →", Msg::Push(Route::Tour))
                .variant(Variant::Outlined)
                .size(15.0),
            button("Kanban board →", Msg::Push(Route::Board))
                .variant(Variant::Outlined)
                .size(15.0),
        ]
        .gap(12.0),
    )
}
