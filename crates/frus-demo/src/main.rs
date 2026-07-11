//! Application exemple : une **liste de tâches** écrite avec frus, en tant que
//! **consommateur externe** du framework (implémente [`frus_shell::Application`]).
//!
//! Lancer avec : `cargo run -p frus-demo` (ajouter `RUST_LOG=info` pour les logs).

use std::path::{Path, PathBuf};
use std::time::Duration;

use frus_shell::{run, Application, Command, Subscription};
use frus_widgets::{
    button, column, keyed, row, spacer, spring_step, text, Alert, Align, Autocomplete, Avatar, Badge,
    Breadcrumb, Card, Carousel, Checkbox, Chip, Collapsible, Color, ColorPicker, Container,
    DatePicker, Divider, Dropdown, Flex, Grid, Justify, Kbd, LayoutBuilder, List, Menu, NavBar,
    NavScaffold, Navigator, Pagination, Placement, Popover, Portal, ProgressBar, RadioGroup, Rating,
    Scroll, SegmentedControl, Size, SizeClass, Skeleton, Slider, Spinner, Stack, Stepper, Switch,
    Table, Tabs, TextInput, Theme, Timeline, Toast, Tree, TwoPane, Variant, Widget, Wrap,
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
    Journal,
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
    /// Tick du chrono (souscription timer).
    Tick,
    /// Démarre/arrête le chrono.
    ToggleTimer,
    /// Change l'onglet actif des Réglages.
    SetSettingsTab(usize),
    /// Ouvre/ferme le menu d'actions.
    ToggleActions,
    /// Déplie/replie « Options avancées ».
    ToggleAdvanced,
    /// Note en étoiles choisie.
    SetRating(u32),
    /// Nouvelle valeur du sélecteur numérique.
    SetCount(i32),
    /// Ferme la notification transitoire.
    DismissToast,
    /// Change la page (démo pagination).
    SetPage(usize),
    /// Déplie/replie un nœud d'arbre.
    ToggleNode(u64),
    /// Choisit une couleur.
    PickColor(Color),
    /// Sélectionne un jour dans le calendrier.
    PickDay(u32),
    /// Change de mois (±1).
    NavMonth(i32),
    /// Change de slide du carrousel.
    SetSlide(usize),
    /// Ouvre/ferme le popover d'info.
    ToggleInfo,
    /// Saisie de l'autocomplétion.
    TagInput(String),
    /// Choix d'une suggestion.
    TagPick(String),
    /// Sauvegarde les tâches sur disque (effet).
    Save,
    /// Demande le chargement des tâches (effet).
    Load,
    /// Tâches chargées depuis le disque (résultat d'un effet).
    Loaded(Vec<(bool, String)>),
    /// Change la section active de l'accueil (navigation adaptative).
    SetSection(usize),
    /// Sélectionne une métrique dans la section Stats (ouvre le détail en étroit).
    SelectStat(usize),
    /// Ferme le détail Stats (retour à la liste en panneau unique).
    CloseDetail,
}

/// Chemin du fichier de persistance des tâches.
fn todos_path() -> PathBuf {
    std::env::temp_dir().join("frus-todos.txt")
}

/// Sérialise les tâches en lignes `done<TAB>texte`.
fn save_todos(path: &Path, todos: &[(bool, String)]) -> std::io::Result<()> {
    let mut out = String::new();
    for (done, text) in todos {
        out.push(if *done { '1' } else { '0' });
        out.push('\t');
        // Neutralise les séparateurs dans le texte.
        out.push_str(&text.replace(['\t', '\n'], " "));
        out.push('\n');
    }
    std::fs::write(path, out)
}

/// Lit les tâches depuis le fichier (vide si absent/illisible).
fn load_todos(path: &Path) -> Vec<(bool, String)> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| {
            let (flag, text) = line.split_once('\t')?;
            Some((flag == "1", text.to_string()))
        })
        .collect()
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
    // --- Chrono (souscription timer) ---
    /// Le chrono tourne-t-il ? (pilote la souscription `every`).
    running: bool,
    /// Secondes écoulées depuis le démarrage du chrono.
    elapsed: u32,
    /// Onglet actif de l'écran Réglages.
    settings_tab: usize,
    /// Menu d'actions (en-tête) ouvert ?
    actions_open: bool,
    /// Section « Options avancées » dépliée ?
    advanced_open: bool,
    /// Note en étoiles (Réglages).
    rating: u32,
    /// Compteur du sélecteur numérique (Réglages).
    count: i32,
    /// Notification transitoire courante (auto-fermée).
    toast: Option<String>,
    /// Page courante du sélecteur de pagination (démo).
    page: usize,
    /// Nœuds d'arbre dépliés (démo Tree).
    expanded: std::collections::HashSet<u64>,
    /// Couleur choisie (démo ColorPicker).
    picked: Option<Color>,
    /// Calendrier : année / mois (1..12) / jour sélectionné.
    year: i32,
    month: u32,
    selected_day: Option<u32>,
    /// Slide courant du carrousel (démo).
    slide: usize,
    /// Popover d'info ouvert ?
    info_open: bool,
    /// Saisie de l'autocomplétion (démo).
    tag_draft: String,
    /// Section active de l'accueil (0 = Tasks, 1 = Stats, 2 = About) — NavScaffold.
    section: usize,
    /// Métrique sélectionnée dans la section Stats (TwoPane maître-détail).
    stat_sel: usize,
    /// En panneau unique (étroit), le détail Stats est-il ouvert ?
    stat_detail_open: bool,
}

fn current_route(app: &TodoApp) -> Route {
    app.routes.last().copied().unwrap_or(Route::Home)
}

/// Index d'un filtre (pour le contrôle segmenté).
fn filter_index(filter: Filter) -> usize {
    match filter {
        Filter::All => 0,
        Filter::Active => 1,
        Filter::Done => 2,
    }
}

/// Filtre depuis un index de segment.
fn filter_from_index(index: usize) -> Filter {
    match index {
        1 => Filter::Active,
        2 => Filter::Done,
        _ => Filter::All,
    }
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

/// Applique un message à l'état et renvoie l'effet éventuel à exécuter.
fn reduce(app: &mut TodoApp, message: Msg) -> Command<Msg> {
    match message {
        Msg::DraftChanged(text) => {
            app.draft = text;
            Command::none()
        }
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
            Command::none()
        }
        Msg::ToggleTodo(id) => {
            if let Some(todo) = app.todos.iter_mut().find(|t| t.id == id) {
                todo.done = !todo.done;
            }
            Command::none()
        }
        Msg::DeleteTodo(id) => {
            app.todos.retain(|t| t.id != id);
            Command::none()
        }
        Msg::SetFilter(filter) => {
            app.filter = filter;
            Command::none()
        }
        Msg::AskClearDone => {
            app.confirm_clear = true;
            Command::none()
        }
        Msg::ConfirmClearDone => {
            app.todos.retain(|t| !t.done);
            app.confirm_clear = false;
            Command::none()
        }
        Msg::CancelClear => {
            app.confirm_clear = false;
            Command::none()
        }
        Msg::ToggleTheme => {
            // Capture le thème courant (avant bascule) comme point de départ du fondu.
            app.theme_from = Some(theme_of(app));
            app.light = !app.light;
            app.theme_progress = 0.0;
            Command::none()
        }
        Msg::SetNotifs(v) => {
            app.notifs = v;
            Command::none()
        }
        Msg::SetVolume(v) => {
            app.volume = v;
            Command::none()
        }
        Msg::SetRadio(i) => {
            app.radio = i;
            Command::none()
        }
        Msg::ToggleMenu => {
            app.menu_open = !app.menu_open;
            Command::none()
        }
        Msg::SetMenu(i) => {
            app.menu_choice = i;
            app.menu_open = false;
            Command::none()
        }
        Msg::Push(route) => {
            app.nav_from = Some(current_route(app));
            app.routes.push(route);
            app.nav_progress = 0.0;
            app.nav_velocity = 0.0;
            app.nav_forward = true;
            Command::none()
        }
        Msg::Pop => {
            if !app.routes.is_empty() {
                app.nav_from = Some(current_route(app));
                app.routes.pop();
                app.nav_progress = 0.0;
                app.nav_velocity = 0.0;
                app.nav_forward = false;
            }
            Command::none()
        }
        Msg::Tick => {
            app.elapsed += 1;
            // Trace du tick : preuve que la souscription émet des messages.
            eprintln!("[demo] chrono : {}s", app.elapsed);
            Command::none()
        }
        Msg::ToggleTimer => {
            app.running = !app.running;
            Command::none()
        }
        Msg::SetSettingsTab(i) => {
            app.settings_tab = i;
            Command::none()
        }
        Msg::ToggleActions => {
            app.actions_open = !app.actions_open;
            Command::none()
        }
        Msg::ToggleAdvanced => {
            app.advanced_open = !app.advanced_open;
            Command::none()
        }
        Msg::SetRating(r) => {
            app.rating = r;
            Command::none()
        }
        Msg::SetCount(c) => {
            app.count = c;
            Command::none()
        }
        // --- Effets ---
        Msg::Save => {
            // Capture un instantané sérialisable ; l'écriture se fait hors update.
            let items: Vec<(bool, String)> =
                app.todos.iter().map(|t| (t.done, t.text.clone())).collect();
            // Affiche une notification, auto-fermée après 2 s (effet minuté).
            app.toast = Some("Saved".to_string());
            Command::batch([
                Command::run(move || {
                    let _ = save_todos(&todos_path(), &items);
                    None
                }),
                Command::perform(|| {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    Msg::DismissToast
                }),
            ])
        }
        Msg::DismissToast => {
            app.toast = None;
            Command::none()
        }
        Msg::SetPage(p) => {
            app.page = p;
            Command::none()
        }
        Msg::ToggleNode(id) => {
            if !app.expanded.remove(&id) {
                app.expanded.insert(id);
            }
            Command::none()
        }
        Msg::PickColor(c) => {
            app.picked = Some(c);
            Command::none()
        }
        Msg::PickDay(d) => {
            app.selected_day = Some(d);
            Command::none()
        }
        Msg::NavMonth(delta) => {
            let mut m = app.month as i32 + delta;
            while m < 1 {
                m += 12;
                app.year -= 1;
            }
            while m > 12 {
                m -= 12;
                app.year += 1;
            }
            app.month = m as u32;
            app.selected_day = None;
            Command::none()
        }
        Msg::SetSlide(i) => {
            app.slide = i;
            Command::none()
        }
        Msg::ToggleInfo => {
            app.info_open = !app.info_open;
            Command::none()
        }
        Msg::TagInput(s) => {
            app.tag_draft = s;
            Command::none()
        }
        Msg::TagPick(s) => {
            app.tag_draft = s;
            Command::none()
        }
        Msg::Load => Command::perform(|| Msg::Loaded(load_todos(&todos_path()))),
        Msg::Loaded(items) => {
            app.todos = items
                .into_iter()
                .map(|(done, text)| {
                    let id = app.next_id;
                    app.next_id += 1;
                    Todo { id, text, done }
                })
                .collect();
            Command::none()
        }
        Msg::SetSection(i) => {
            app.section = i;
            Command::none()
        }
        Msg::SelectStat(i) => {
            app.stat_sel = i;
            app.stat_detail_open = true;
            Command::none()
        }
        Msg::CloseDetail => {
            app.stat_detail_open = false;
            Command::none()
        }
    }
}

impl Application for TodoApp {
    type Message = Msg;

    fn update(&mut self, message: Msg) -> Command<Msg> {
        reduce(self, message)
    }

    fn init(&mut self) -> Command<Msg> {
        // Démarre le chrono et charge les tâches persistées au démarrage.
        self.running = true;
        self.page = 1;
        self.year = 2026;
        self.month = 7;
        Command::perform(|| Msg::Loaded(load_todos(&todos_path())))
    }

    fn subscription(&self) -> Subscription<Msg> {
        // Un tick par seconde tant que le chrono tourne (sinon rien).
        if self.running {
            Subscription::every(Duration::from_secs(1), |_| Msg::Tick)
        } else {
            Subscription::none()
        }
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
        "frus — Todo".to_string()
    }

    fn window_size(&self) -> Option<(f32, f32)> {
        Some((900.0, 680.0))
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
        Route::Journal => journal_screen(theme, width, height),
    }
}

/// Écran « Journal » : une **liste virtualisée** de 5000 lignes.
fn journal_screen(theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let t = *theme; // Theme est Copy — capturé par la fabrique d'éléments.
    let list = List::new(5000, 44.0, move |i| {
        Container::<Msg>::new()
            .height(44.0)
            .radius(8.0)
            .color(if i % 2 == 0 { t.surface } else { t.background })
            .border(1.0, t.border)
            .padding_each(12.0, 14.0, 12.0, 14.0)
            .child(text(format!("Row {}", i + 1)).size(16.0))
    })
    .width((width - 48.0).max(200.0))
    .height((height - 104.0).max(160.0));
    let content = column![list].padding(24.0);
    let screen = column![NavBar::new("Log · 5000 rows").on_back(Msg::Pop), content]
        .width(width)
        .height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}

/// Une tuile de statistique (grand nombre + libellé) pour la grille.
fn stat_tile(theme: &Theme, label: &str, value: usize) -> Container<Msg> {
    Container::new()
        .height(64.0)
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(10.0, 12.0, 10.0, 12.0)
        .child(column![
            text(value.to_string()).size(24.0),
            text(label.to_string()).size(13.0).color(theme.muted),
        ])
}

/// Écran « Réglages » : la carte de contrôles (démontre nav + geste + widgets).
fn settings_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
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
                Slider::new(app.volume).width(220.0).on_change(Msg::SetVolume),
            ]
            .align(Align::Center)
            .gap(12.0),
            RadioGroup::new(app.radio, Msg::SetRadio)
                .option("Small")
                .option("Medium")
                .option("Large"),
            Dropdown::new(MENU[app.menu_choice], Msg::ToggleMenu).options(
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
            DatePicker::new(app.year, app.month, app.selected_day, Msg::PickDay, Msg::NavMonth),
        ]
        .gap(14.0),
    );
    let total = app.todos.len();
    let done = app.todos.iter().filter(|t| t.done).count();
    let stats = Grid::new(3)
        .gap(10.0)
        .width(360.0)
        .cell(stat_tile(theme, "Total", total))
        .cell(stat_tile(theme, "Active", total - done))
        .cell(stat_tile(theme, "Done", done));
    let facts = Table::new(2)
        .width(320.0)
        .header(&["Metric", "Value"])
        .row(&["Widgets", "35"])
        .row(&["Milestones", "39"]);

    // Arbre de fichiers (déplié selon l'état).
    let open = |id: u64| app.expanded.contains(&id);
    let mut tree = Tree::new(Msg::ToggleNode).node(1, 0, "src", true, open(1));
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

    // Palette de couleurs.
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

    // Chronologie des jalons récents.
    let timeline = Timeline::new()
        .event("Grid", "Milestone 35")
        .event("New widgets", "Milestones 36–37")
        .event("Hierarchy & color", "Milestone 38");

    // Carrousel : le slide courant est fourni selon l'index.
    let slide = match app.slide {
        0 => text("Welcome to frus").size(16.0),
        1 => text("About 35 widgets").size(16.0),
        _ => text("Thanks for trying!").size(16.0),
    };
    let carousel = Carousel::new(app.slide, 3, Msg::SetSlide, slide);

    // Popover d'info (contenu libre, fermeture au clic extérieur).
    let info = Popover::new(
        button("Info", Msg::ToggleInfo).variant(Variant::Secondary).size(15.0),
        app.info_open,
        Msg::ToggleInfo,
    )
    .content(Card::new().padding(16.0).child(
        column![
            text("Popover").size(16.0),
            text("An arbitrary floating panel; closes on outside click.")
                .size(14.0)
                .color(theme.muted),
        ]
        .gap(6.0),
    ));

    // Autocomplétion : suggestions filtrées par la saisie (contrôlé).
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

    // Indices de raccourcis clavier.
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
            Skeleton::new().width(320.0),
            Skeleton::new().width(260.0).height(14.0),
        ]
        .gap(8.0),
        Divider::new(),
        Collapsible::new("Advanced options", app.advanced_open, Msg::ToggleAdvanced).content(
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
    let tabs = Tabs::new(app.settings_tab, Msg::SetSettingsTab)
        .tab("Controls", controls)
        .tab("About", about);
    let content = column![
        Breadcrumb::new(|_| Msg::Pop).crumb("Home").crumb("Settings"),
        row![tabs].justify(Justify::Center),
    ]
    .padding(20.0)
    .gap(16.0);
    let screen = column![NavBar::new("Settings").on_back(Msg::Pop), content].width(width).height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}

/// Une ligne de tâche : case à cocher, libellé (grisé si terminée) et suppression.
fn todo_row(todo: &Todo, theme: &Theme) -> Container<Msg> {
    let id = todo.id;
    let label_color = if todo.done { theme.muted } else { theme.on_surface };
    let line = row![
        Avatar::new(todo.text.clone()).size(30.0),
        Checkbox::new(todo.done).on_toggle(move |_| Msg::ToggleTodo(id)),
        text(todo.text.clone()).size(18.0).color(label_color),
        spacer(),
        button("×", Msg::DeleteTodo(id)).variant(Variant::Danger).size(15.0),
    ]
    .align(Align::Center)
    .gap(12.0);
    Container::new()
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(line)
}

/// Contenu de la modale de confirmation d'effacement des terminées.
fn confirm_content(done: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Clear completed tasks?").size(22.0),
            text(format!("{done} task(s) will be removed.")).size(16.0),
            row![
                button("Cancel", Msg::CancelClear).variant(Variant::Secondary),
                button("Delete", Msg::ConfirmClearDone).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// Écran principal : la liste de tâches (l'app exemple).
fn todo_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let active = active_count(app);
    let done = done_count(app);

    // Responsivité : la carte s'élargit avec la fenêtre par paliers (Lot A). En
    // Compact, elle suit la largeur disponible ; les champs internes s'y adaptent.
    let class = SizeClass::from_width(width);
    let card_width = match class {
        SizeClass::Compact => (width - 40.0).max(280.0),
        SizeClass::Medium => 560.0,
        SizeClass::Expanded => 680.0,
    };

    // En-tête : titre + chrono + bascule de thème + accès Réglages.
    let theme_label = if app.light { "Dark" } else { "Light" };
    let timer_label = if app.running { "Pause" } else { "Resume" };
    // Indicateur : un spinner (animation continue) avec une pastille du nombre
    // de tâches actives dans le coin (pile de couches).
    let indicator = Stack::new()
        .width(30.0)
        .height(30.0)
        .layer(Spinner::new().size(30.0))
        .layer(row![Badge::new(format!("{active}"))].justify(Justify::End).align(Align::Start));
    // Les actions passent à la ligne quand l'en-tête est trop étroit (Lot B :
    // flex-wrap). `flex(1.0)` borne leur largeur pour déclencher le reflow.
    let actions = Wrap::new::<Msg>()
        .flex(1.0)
        .gap(8.0)
        .justify(Justify::End)
        .align(Align::Center)
        .child(button(timer_label, Msg::ToggleTimer).variant(Variant::Secondary).size(15.0))
        .child(button(theme_label, Msg::ToggleTheme).variant(Variant::Secondary).size(15.0))
        .child(button("Log →", Msg::Push(Route::Journal)).variant(Variant::Secondary).size(15.0))
        .child(button("Settings →", Msg::Push(Route::Settings)).variant(Variant::Secondary).size(15.0))
        .child(
            Menu::new(
                button("⋯", Msg::ToggleActions).variant(Variant::Secondary).size(15.0),
                app.actions_open,
                Msg::ToggleActions,
            )
            .item("Save", Msg::Save)
            .item("Clear completed", Msg::AskClearDone),
        );
    let title_size = if class == SizeClass::Compact { 22.0 } else { 30.0 };
    let header = row![
        indicator,
        text("My Tasks").size(title_size),
        text(format!("· {}s", app.elapsed)).size(18.0).color(theme.muted),
        actions,
    ]
    .align(Align::Center)
    .gap(10.0);

    // Saisie : champ (Entrée valide) + bouton d'ajout.
    let input_row = row![
        TextInput::new(app.draft.as_str())
            .width((card_width - 150.0).max(160.0))
            .size(18.0)
            .on_input(Msg::DraftChanged)
            .on_submit(Msg::AddTodo),
        button("Add", Msg::AddTodo),
    ]
    .align(Align::Center)
    .gap(10.0);

    // Filtres : un contrôle segmenté (sélection unique).
    let segmented = SegmentedControl::new(filter_index(app.filter), |i| {
        Msg::SetFilter(filter_from_index(i))
    })
    .segment("All")
    .segment("Active")
    .segment("Done");
    let mut filters = row![segmented].align(Align::Center).gap(8.0);
    // Le filtre actif (hors « Toutes ») s'affiche en puce supprimable.
    if app.filter != Filter::All {
        let name = if app.filter == Filter::Active { "Active" } else { "Done" };
        filters = filters
            .child(spacer())
            .child(Chip::new(name).on_remove(Msg::SetFilter(Filter::All)));
    }

    // Liste filtrée (ou état vide).
    let mut list = Flex::column().gap(8.0);
    let mut shown = 0;
    for todo in app.todos.iter().filter(|t| match app.filter {
        Filter::All => true,
        Filter::Active => !t.done,
        Filter::Done => t.done,
    }) {
        // Identité stable par `id` : l'état retenu (survol/animations) ne saute
        // pas quand on supprime une tâche au milieu.
        list = list.child(keyed(todo.id, todo_row(todo, theme)));
        shown += 1;
    }
    if shown == 0 {
        list = column![text("Nothing to show for this filter.").size(18.0).color(theme.muted)];
    }
    let scroll = Scroll::new().flex(1.0).height(320.0).child(list);

    // Pied : compteurs + effacer les terminées (avec confirmation modale).
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

    // Résumé construit selon sa boîte RÉELLE (Lot C : LayoutBuilder). Texte long
    // quand il y a de la place, court quand c'est étroit — hauteur fixe.
    let muted = theme.muted;
    let summary = LayoutBuilder::new(move |size: Size| {
        let label = if size.width >= 360.0 {
            format!("{active} active · {done} done · {pct}% complete")
        } else {
            format!("{active}·{done}")
        };
        text(label).size(16.0).color(muted)
    })
    .flex(1.0)
    .height(20.0);
    let footer = row![
        summary,
        button("Load", Msg::Load).variant(Variant::Secondary).size(15.0),
        button("Save", Msg::Save).variant(Variant::Secondary).size(15.0),
        clear,
    ]
    .align(Align::Center)
    .gap(8.0);

    // Barre de progression de complétion (terminées / total).
    let progress = ProgressBar::new(done as f32 / total as f32).width((card_width - 40.0).max(200.0));

    // Carte de l'app, largeur responsive (Lot A), centrée en haut de l'écran.
    let card = Card::new().padding(20.0).child(
        column![
            Alert::new("Press Enter to add a task; swipe from the left edge to go back.")
                .title("Tip"),
            header,
            input_row,
            filters,
            scroll,
            Divider::new(),
            progress,
            footer
        ]
        .width(card_width)
        .gap(16.0),
    );
    let tasks_body = column![row![card].justify(Justify::Center)].padding(24.0);

    // Sections de l'accueil, choisies via la navigation adaptative (NavScaffold) :
    // rail vertical en grand, barre basse en étroit.
    let section: Box<dyn Widget<Msg>> = match app.section {
        1 => Box::new(stats_section(app, theme, class)),
        2 => Box::new(about_section(theme)),
        _ => Box::new(tasks_body),
    };
    let shell = NavScaffold::new(class, app.section, Msg::SetSection)
        .destination("✔", "Tasks")
        .destination("▦", "Stats")
        .destination("★", "About")
        .body(section);
    let screen = Container::new().width(width).height(height).child(shell);

    // La notification transitoire flotte en bas-centre, par-dessus l'écran.
    let mut layers = Stack::new().width(width).height(height).layer(screen);
    if let Some(message) = &app.toast {
        layers = layers.layer(
            column![Toast::new(message.clone()).success()]
                .justify(Justify::End)
                .align(Align::Center)
                .padding(20.0),
        );
    }

    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(layers)
}

/// Section « Stats » : un agencement maître-détail responsive (`TwoPane`). Côte à
/// côte en grand, panneau unique en étroit (taper une métrique ouvre le détail).
fn stats_section(app: &TodoApp, theme: &Theme, class: SizeClass) -> TwoPane<Msg> {
    let total = app.todos.len();
    let metrics = [
        ("Total tasks", total),
        ("Active tasks", active_count(app)),
        ("Completed", done_count(app)),
    ];

    // Panneau maître : la liste des métriques (sélection).
    let mut cats = Flex::column().gap(6.0);
    for (i, (label, _)) in metrics.iter().enumerate() {
        let variant = if app.stat_sel == i { Variant::Primary } else { Variant::Secondary };
        cats = cats.child(button(*label, Msg::SelectStat(i)).variant(variant).size(15.0));
    }
    let list = Card::new().padding(12.0).child(cats);

    // Panneau détail : la métrique sélectionnée.
    let (label, value) = metrics[app.stat_sel.min(metrics.len() - 1)];
    let mut detail_col = column![
        text(label).size(22.0),
        text(value.to_string()).size(44.0).color(theme.primary),
        text("Detail for the selected metric.").size(14.0).color(theme.muted),
    ]
    .gap(10.0);
    // En panneau unique, un retour vers la liste.
    if class != SizeClass::Expanded {
        detail_col = detail_col
            .child(button("← Back", Msg::CloseDetail).variant(Variant::Secondary).size(15.0));
    }
    let detail = Card::new().padding(20.0).child(detail_col);

    TwoPane::new(class)
        .ratio(0.36)
        .show_detail(app.stat_detail_open)
        .list(list)
        .detail(detail)
}

/// Section « About » : contenu statique de présentation.
fn about_section(theme: &Theme) -> Container<Msg> {
    Container::new().padding(24.0).child(
        Card::new().padding(20.0).child(
            column![
                text("About frus").size(24.0),
                text("A Rust cross-platform UI framework.").size(15.0).color(theme.muted),
                Divider::new(),
                Timeline::new()
                    .event("Responsive primitives", "Milestone 42")
                    .event("Adaptive navigation", "Milestone 43"),
            ]
            .gap(12.0)
            .width(360.0),
        ),
    )
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
    fn save_then_load_roundtrips() {
        let path = std::env::temp_dir().join("frus-todos-test-roundtrip.txt");
        let items = vec![
            (false, "acheter du pain".to_string()),
            (true, "ranger le bureau".to_string()),
        ];
        save_todos(&path, &items).unwrap();
        assert_eq!(load_todos(&path), items);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loaded_replaces_todos_with_unique_ids() {
        let mut app = TodoApp::default();
        add(&mut app, "ancienne");
        reduce(
            &mut app,
            Msg::Loaded(vec![(true, "a".to_string()), (false, "b".to_string())]),
        );
        assert_eq!(app.todos.len(), 2);
        assert_eq!(app.todos[0].text, "a");
        assert!(app.todos[0].done);
        assert!(!app.todos[1].done);
        assert_ne!(app.todos[0].id, app.todos[1].id);
    }

    #[test]
    fn timer_subscription_gated_by_running() {
        let mut app = TodoApp::default();
        // Par défaut le chrono ne tourne pas (init le démarre à l'exécution).
        assert!(app.subscription().is_empty());

        app.running = true;
        let subs = app.subscription();
        assert!(!subs.is_empty());
        // Deux évaluations donnent le même id (souscription stable).
        assert_eq!(subs.ids(), app.subscription().ids());
    }

    #[test]
    fn tick_increments_elapsed() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Tick);
        reduce(&mut app, Msg::Tick);
        assert_eq!(app.elapsed, 2);
    }

    #[test]
    fn save_produces_a_run_effect() {
        let mut app = TodoApp::default();
        add(&mut app, "x");
        // Save renvoie une commande non vide (l'écriture est un effet).
        assert!(!reduce(&mut app, Msg::Save).is_empty());
        // Une mutation simple n'a aucun effet.
        assert!(reduce(&mut app, Msg::DraftChanged("y".to_string())).is_empty());
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
