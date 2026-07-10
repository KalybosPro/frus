//! Application exemple : une **liste de tâches** écrite avec frus, en tant que
//! **consommateur externe** du framework (implémente [`frus_shell::Application`]).
//!
//! Lancer avec : `cargo run -p frus-demo` (ajouter `RUST_LOG=info` pour les logs).

use frus_shell::{run, Application};
use frus_widgets::{
    spring_step, Align, Button, Card, Checkbox, Container, Dropdown, Flex, Justify, NavBar,
    Navigator, Placement, Portal, RadioGroup, Scroll, Slider, Switch, Text, TextInput, Theme,
    Variant, Widget,
};

fn main() -> anyhow::Result<()> {
    run(TodoApp::default())
}

// --- Constantes de mouvement (partagées geste ↔ navigation) ---

/// Horizon de projection de la vélocité (s) pour décider retour / annulation.
const BACK_PROJECT: f32 = 0.12;
/// Position projetée (fraction) au-delà de laquelle on valide le retour.
const BACK_COMMIT_POS: f32 = 0.5;
/// Raideur du ressort de transition (fraction·s⁻²).
const NAV_SPRING_K: f32 = 220.0;
/// Amortissement (~critique) → arrivée douce sans dépassement.
const NAV_SPRING_C: f32 = 30.0;

/// Libellés du menu déroulant (écran Réglages).
const MENU: [&str; 3] = ["Option A", "Option B", "Option C"];

// --- Modèle ---

/// Une tâche de la liste.
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

/// Filtre d'affichage de la liste des tâches.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Active,
    Done,
}

/// Les écrans de l'application.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Route {
    Home,
    Settings,
}

/// Geste retour : progression suivie, puis détente à ressort (validation/annulation).
struct BackGesture {
    progress: f32,
    velocity: f32,
    /// `Some(cible)` une fois relâché : `1.0` valide, `0.0` annule.
    settling: Option<f32>,
}

/// Messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    DraftChanged(String),
    AddTodo,
    ToggleTodo(u64),
    DeleteTodo(u64),
    SetFilter(Filter),
    AskClearDone,
    ConfirmClearDone,
    CancelClear,
    ToggleTheme,
    SetNotifs(bool),
    SetVolume(f32),
    SetRadio(usize),
    ToggleMenu,
    SetMenu(usize),
    Push(Route),
    Pop,
}

/// L'application todo : état + logique. Consommateur du framework `frus-shell`.
#[derive(Default)]
struct TodoApp {
    /// Les tâches, dans l'ordre d'ajout.
    todos: Vec<Todo>,
    /// Texte en cours de saisie.
    draft: String,
    /// Filtre courant.
    filter: Filter,
    /// Prochain identifiant de tâche.
    next_id: u64,
    /// Modale de confirmation d'effacement des terminées ouverte ?
    confirm_clear: bool,
    /// Thème clair (sinon sombre).
    light: bool,
    /// Thème sortant pendant un fondu de bascule (`None` = pas de transition).
    theme_from: Option<Theme>,
    /// Avancement du fondu de thème (`0 → 1`).
    theme_progress: f32,
    /// Pile d'écrans (vide = accueil).
    routes: Vec<Route>,
    /// Écran sortant pendant une transition.
    nav_from: Option<Route>,
    nav_progress: f32,
    nav_velocity: f32,
    nav_forward: bool,
    /// Geste retour en cours.
    back: Option<BackGesture>,
    // --- Contrôles de l'écran Réglages ---
    notifs: bool,
    volume: f32,
    radio: usize,
    menu_open: bool,
    menu_choice: usize,
}

fn current_route(app: &TodoApp) -> Route {
    app.routes.last().copied().unwrap_or(Route::Home)
}

/// Nombre de tâches non terminées.
fn active_count(app: &TodoApp) -> usize {
    app.todos.iter().filter(|t| !t.done).count()
}

/// Nombre de tâches terminées.
fn done_count(app: &TodoApp) -> usize {
    app.todos.iter().filter(|t| t.done).count()
}

/// Thème « cible » selon l'état (avant fondu).
fn theme_of(app: &TodoApp) -> Theme {
    if app.light {
        Theme::light()
    } else {
        Theme::dark()
    }
}

/// Applique un message à l'état.
fn reduce(app: &mut TodoApp, message: Msg) {
    match message {
        Msg::DraftChanged(text) => app.draft = text,
        Msg::AddTodo => {
            let text = app.draft.trim();
            if !text.is_empty() {
                app.todos.push(Todo {
                    id: app.next_id,
                    text: text.to_string(),
                    done: false,
                });
                app.next_id += 1;
                app.draft.clear();
            }
        }
        Msg::ToggleTodo(id) => {
            if let Some(todo) = app.todos.iter_mut().find(|t| t.id == id) {
                todo.done = !todo.done;
            }
        }
        Msg::DeleteTodo(id) => app.todos.retain(|t| t.id != id),
        Msg::SetFilter(filter) => app.filter = filter,
        Msg::AskClearDone => app.confirm_clear = true,
        Msg::ConfirmClearDone => {
            app.todos.retain(|t| !t.done);
            app.confirm_clear = false;
        }
        Msg::CancelClear => app.confirm_clear = false,
        Msg::ToggleTheme => {
            // Capture le thème courant (avant bascule) comme point de départ du fondu.
            app.theme_from = Some(theme_of(app));
            app.light = !app.light;
            app.theme_progress = 0.0;
        }
        Msg::SetNotifs(v) => app.notifs = v,
        Msg::SetVolume(v) => app.volume = v,
        Msg::SetRadio(i) => app.radio = i,
        Msg::ToggleMenu => app.menu_open = !app.menu_open,
        Msg::SetMenu(i) => {
            app.menu_choice = i;
            app.menu_open = false;
        }
        Msg::Push(route) => {
            app.nav_from = Some(current_route(app));
            app.routes.push(route);
            app.nav_progress = 0.0;
            app.nav_velocity = 0.0;
            app.nav_forward = true;
        }
        Msg::Pop => {
            if !app.routes.is_empty() {
                app.nav_from = Some(current_route(app));
                app.routes.pop();
                app.nav_progress = 0.0;
                app.nav_velocity = 0.0;
                app.nav_forward = false;
            }
        }
    }
}

impl Application for TodoApp {
    type Message = Msg;

    fn update(&mut self, message: Msg) {
        reduce(self, message);
    }

    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        Box::new(build_view(self, theme, width, height))
    }

    fn theme(&self) -> Theme {
        let target = theme_of(self);
        match self.theme_from {
            Some(from) => from.lerp(&target, self.theme_progress),
            None => target,
        }
    }

    fn tick(&mut self, dt: f32) -> bool {
        let mut animating = false;

        // Fondu de thème.
        if self.theme_from.is_some() {
            self.theme_progress += dt / 0.25;
            if self.theme_progress >= 1.0 {
                self.theme_progress = 1.0;
                self.theme_from = None;
            } else {
                animating = true;
            }
        }

        // Transition d'écran (ressort amorcé à vitesse nulle → ease-out).
        if self.nav_from.is_some() {
            let (p, v, done) = spring_step(
                self.nav_progress,
                self.nav_velocity,
                1.0,
                dt,
                NAV_SPRING_K,
                NAV_SPRING_C,
            );
            self.nav_progress = p;
            self.nav_velocity = v;
            if done {
                self.nav_progress = 1.0;
                self.nav_velocity = 0.0;
                self.nav_from = None;
            } else {
                animating = true;
            }
        }

        // Détente du geste retour (même ressort, amorcé par l'élan du doigt).
        let mut commit_back = false;
        if let Some(g) = self.back.as_mut() {
            if let Some(target) = g.settling {
                let (p, v, done) =
                    spring_step(g.progress, g.velocity, target, dt, NAV_SPRING_K, NAV_SPRING_C);
                g.progress = p;
                g.velocity = v;
                if done {
                    commit_back = target >= 1.0;
                    self.back = None;
                } else {
                    animating = true;
                }
            }
        }
        if commit_back {
            self.routes.pop();
        }

        animating
    }

    fn title(&self) -> String {
        "frus — Jalon 23 · Todo".to_string()
    }

    fn can_go_back(&self) -> bool {
        !self.routes.is_empty() && !self.confirm_clear && !self.menu_open
    }

    fn back_gesture(&mut self, progress: f32) {
        match self.back.as_mut() {
            Some(g) => g.progress = progress,
            None => {
                self.back = Some(BackGesture {
                    progress,
                    velocity: 0.0,
                    settling: None,
                })
            }
        }
    }

    fn back_gesture_end(&mut self, velocity: f32) {
        if let Some(g) = self.back.as_mut() {
            g.velocity = velocity;
            // Projection à la iOS : la position + l'élan décident.
            let projected = g.progress + velocity * BACK_PROJECT;
            let commit = projected > BACK_COMMIT_POS && !self.routes.is_empty();
            g.settling = Some(if commit { 1.0 } else { 0.0 });
        }
    }
}

// --- Vue ---

/// Point d'entrée de la vue : un `Navigator` autour de l'écran courant.
fn build_view(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Navigator<Msg> {
    // Geste retour en cours : prévisualise le dépilement, piloté par le doigt.
    if let Some(gesture) = &app.back {
        let progress = gesture.progress;
        let top = screen(current_route(app), app, theme, width, height);
        let below_route = app
            .routes
            .split_last()
            .and_then(|(_, rest)| rest.last().copied())
            .unwrap_or(Route::Home);
        let below = screen(below_route, app, theme, width, height);
        return Navigator::new(below, width, height).from(top, progress, false);
    }

    let current = screen(current_route(app), app, theme, width, height);
    match app.nav_from {
        Some(from) => Navigator::new(current, width, height).from(
            screen(from, app, theme, width, height),
            app.nav_progress,
            app.nav_forward,
        ),
        None => Navigator::new(current, width, height),
    }
}

/// Construit l'écran correspondant à une route.
fn screen(route: Route, app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    match route {
        Route::Home => todo_screen(app, theme, width, height),
        Route::Settings => settings_screen(app, theme, width, height),
    }
}

/// Écran « Réglages » : la carte de contrôles (démontre nav + geste + widgets).
fn settings_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let volume_pct = (app.volume * 100.0).round() as u32;
    let controls = Card::new().child(
        Flex::column()
            .gap(14.0)
            .child(
                Flex::row()
                    .align(Align::Center)
                    .gap(12.0)
                    .child(Text::new("Notifications").size(18.0))
                    .child(Flex::row().flex(1.0))
                    .child(Switch::new(app.notifs).on_toggle(Msg::SetNotifs)),
            )
            .child(
                Flex::row()
                    .align(Align::Center)
                    .gap(12.0)
                    .child(Text::new(format!("Volume : {volume_pct}%")).size(18.0))
                    .child(Slider::new(app.volume).width(220.0).on_change(Msg::SetVolume)),
            )
            .child(
                RadioGroup::new(app.radio, Msg::SetRadio)
                    .option("Petit")
                    .option("Moyen")
                    .option("Grand"),
            )
            .child(Dropdown::new(MENU[app.menu_choice], Msg::ToggleMenu).options(
                app.menu_open,
                &MENU,
                Msg::SetMenu,
            )),
    );
    let content = Flex::column()
        .padding(20.0)
        .gap(16.0)
        .child(Flex::row().justify(Justify::Center).child(controls));
    let column = Flex::column()
        .width(width)
        .height(height)
        .child(NavBar::new("Réglages").on_back(Msg::Pop))
        .child(content);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(column)
}

/// Une ligne de tâche : case à cocher, libellé (grisé si terminée) et suppression.
fn todo_row(todo: &Todo, theme: &Theme) -> Container<Msg> {
    let id = todo.id;
    let label_color = if todo.done { theme.muted } else { theme.on_surface };
    let row = Flex::row()
        .align(Align::Center)
        .gap(12.0)
        .child(Checkbox::new(todo.done).on_toggle(move |_| Msg::ToggleTodo(id)))
        .child(Text::new(todo.text.clone()).size(18.0).color(label_color))
        .child(Flex::row().flex(1.0))
        .child(
            Button::new("×")
                .variant(Variant::Danger)
                .size(15.0)
                .on_press(Msg::DeleteTodo(id)),
        );
    Container::new()
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(row)
}

/// Contenu de la modale de confirmation d'effacement des terminées.
fn confirm_content(done: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        Flex::column()
            .gap(16.0)
            .child(Text::new("Effacer les tâches terminées ?").size(22.0))
            .child(Text::new(format!("{done} tâche(s) seront supprimées.")).size(16.0))
            .child(
                Flex::row()
                    .justify(Justify::Center)
                    .gap(12.0)
                    .child(
                        Button::new("Annuler")
                            .variant(Variant::Secondary)
                            .on_press(Msg::CancelClear),
                    )
                    .child(
                        Button::new("Supprimer")
                            .variant(Variant::Danger)
                            .on_press(Msg::ConfirmClearDone),
                    ),
            ),
    )
}

/// Écran principal : la liste de tâches (l'app exemple).
fn todo_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let active = active_count(app);
    let done = done_count(app);

    // En-tête : titre + bascule de thème + accès Réglages.
    let theme_label = if app.light { "Sombre" } else { "Clair" };
    let header = Flex::row()
        .align(Align::Center)
        .gap(10.0)
        .child(Text::new("Mes tâches").size(30.0))
        .child(Flex::row().flex(1.0))
        .child(
            Button::new(theme_label)
                .variant(Variant::Secondary)
                .size(15.0)
                .on_press(Msg::ToggleTheme),
        )
        .child(
            Button::new("Réglages →")
                .variant(Variant::Secondary)
                .size(15.0)
                .on_press(Msg::Push(Route::Settings)),
        );

    // Saisie : champ (Entrée valide) + bouton d'ajout.
    let input_row = Flex::row()
        .align(Align::Center)
        .gap(10.0)
        .child(
            TextInput::new(app.draft.as_str())
                .width(400.0)
                .size(18.0)
                .on_input(Msg::DraftChanged)
                .on_submit(Msg::AddTodo),
        )
        .child(Button::new("Ajouter").on_press(Msg::AddTodo));

    // Filtres : le filtre actif est mis en avant.
    let filter_button = |label: &str, f: Filter| {
        let variant = if app.filter == f {
            Variant::Primary
        } else {
            Variant::Secondary
        };
        Button::new(label)
            .variant(variant)
            .size(15.0)
            .on_press(Msg::SetFilter(f))
    };
    let filters = Flex::row()
        .gap(8.0)
        .child(filter_button("Toutes", Filter::All))
        .child(filter_button("Actives", Filter::Active))
        .child(filter_button("Terminées", Filter::Done));

    // Liste filtrée (ou état vide).
    let mut list = Flex::column().gap(8.0);
    let mut shown = 0;
    for todo in app.todos.iter().filter(|t| match app.filter {
        Filter::All => true,
        Filter::Active => !t.done,
        Filter::Done => t.done,
    }) {
        list = list.child(todo_row(todo, theme));
        shown += 1;
    }
    if shown == 0 {
        list = Flex::column().child(
            Text::new("Rien à afficher pour ce filtre.")
                .size(18.0)
                .color(theme.muted),
        );
    }
    let scroll = Scroll::new().flex(1.0).height(320.0).child(list);

    // Pied : compteurs + effacer les terminées (avec confirmation modale).
    let clear_button = Button::new("Effacer les terminées")
        .variant(Variant::Danger)
        .size(15.0)
        .on_press(Msg::AskClearDone);
    let clear = if app.confirm_clear {
        Portal::new(clear_button)
            .overlay(confirm_content(done), Placement::Center)
            .dismiss(Msg::CancelClear)
    } else {
        Portal::new(clear_button)
    };
    let footer = Flex::row()
        .align(Align::Center)
        .gap(12.0)
        .child(
            Text::new(format!("{active} active(s) · {done} terminée(s)"))
                .size(16.0)
                .color(theme.muted),
        )
        .child(Flex::row().flex(1.0))
        .child(clear);

    // Carte de l'app, largeur fixe, centrée en haut de l'écran.
    let card = Card::new().padding(20.0).child(
        Flex::column()
            .width(560.0)
            .gap(16.0)
            .child(header)
            .child(input_row)
            .child(filters)
            .child(scroll)
            .child(footer),
    );
    let column = Flex::column()
        .width(width)
        .height(height)
        .padding(24.0)
        .child(Flex::row().justify(Justify::Center).child(card));

    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::{build_ui, Runtime, Size};

    /// Ajoute une tâche depuis un libellé.
    fn add(app: &mut TodoApp, text: &str) {
        reduce(app, Msg::DraftChanged(text.to_string()));
        reduce(app, Msg::AddTodo);
    }

    fn primitive_count(app: &TodoApp) -> usize {
        let theme = Theme::default();
        let tree = build_view(app, &theme, 800.0, 600.0);
        build_ui(&tree, Size::new(800.0, 600.0), &Runtime::default(), &theme)
            .scene()
            .primitives()
            .len()
    }

    #[test]
    fn add_todo_from_draft_and_trims_blanks() {
        let mut app = TodoApp::default();
        add(&mut app, "Acheter du pain");
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "Acheter du pain");
        assert!(app.draft.is_empty(), "le champ est vidé après l'ajout");

        add(&mut app, "   ");
        assert_eq!(app.todos.len(), 1);
    }

    #[test]
    fn toggle_delete_and_clear_done() {
        let mut app = TodoApp::default();
        for t in ["a", "b", "c"] {
            add(&mut app, t);
        }
        let id_b = app.todos[1].id;
        reduce(&mut app, Msg::ToggleTodo(id_b));
        assert!(app.todos[1].done);
        assert_eq!(done_count(&app), 1);
        assert_eq!(active_count(&app), 2);

        let id_a = app.todos[0].id;
        reduce(&mut app, Msg::DeleteTodo(id_a));
        assert_eq!(app.todos.len(), 2);

        reduce(&mut app, Msg::ConfirmClearDone);
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "c");
    }

    #[test]
    fn view_builds_a_non_empty_scene() {
        let mut app = TodoApp::default();
        add(&mut app, "tâche");
        assert!(primitive_count(&app) > 0);
    }

    #[test]
    fn back_gesture_flick_commits_pop() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Settings));
        while app.nav_from.is_some() {
            app.tick(0.05);
        }
        assert_eq!(app.routes.len(), 1);

        // Petit glissement mais flick rapide → doit valider le retour.
        app.back_gesture(0.2);
        app.back_gesture_end(5.0);
        for _ in 0..200 {
            if app.back.is_none() {
                break;
            }
            app.tick(0.05);
        }
        assert!(app.routes.is_empty(), "le flick a dépilé l'écran");
    }
}
