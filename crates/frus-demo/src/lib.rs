//! Application exemple : une **liste de tâches** écrite avec frus, en tant que
//! **consommateur externe** du framework (implémente [`frus_shell::Application`]).
//!
//! Deux points d'entrée pour le même code :
//! - bureau : `cargo run -p frus-demo` → binaire `src/bin/frus-demo.rs` → [`run_desktop`] ;
//! - Android : la bibliothèque `cdylib` expose `android_main`, appelé par
//!   l'activité native.

use std::path::{Path, PathBuf};
use std::time::Duration;

use frus_shell::{Application, Command, Lifecycle, Subscription};
use frus_l10n::{args, Localizer};
use std::sync::OnceLock;

/// Le localiseur de la démo : anglais + français, chargés une seule fois depuis
/// des ressources Fluent embarquées (`i18n/*.ftl`).
fn l10n() -> &'static Localizer {
    static L10N: OnceLock<Localizer> = OnceLock::new();
    L10N.get_or_init(|| {
        let mut l = Localizer::new("en");
        l.add("en", include_str!("../i18n/en.ftl"));
        l.add("fr", include_str!("../i18n/fr.ftl"));
        l.add("ar", include_str!("../i18n/ar.ftl"));
        l
    })
}

/// Les langues proposées par la démo (étiquette du menu, code de locale). La
/// dernière, l'arabe, est **droite-à-gauche** : la sélectionner retourne aussi
/// la mise en page (bidi + miroir).
const LANGS: [(&str, &str); 3] = [("English", "en"), ("Français", "fr"), ("العربية", "ar")];

/// La langue d'index `lang` s'écrit-elle de droite à gauche ?
fn lang_is_rtl(lang: usize) -> bool {
    LANGS[lang].1 == "ar"
}

/// Traduit une clé sans argument dans la langue d'index `lang`.
fn tr(lang: usize, key: &str) -> String {
    let loc = l10n();
    loc.format_for(&loc.langid(LANGS[lang].1), key, args![])
}

/// Traduit une clé avec un argument numérique `n` (pluriels CLDR).
fn tr_n(lang: usize, key: &str, n: usize) -> String {
    let loc = l10n();
    loc.format_for(&loc.langid(LANGS[lang].1), key, args![n: n])
}
use frus_widgets::{
    button, column, keyed, row, spacer, text, AnimationController, Alert, Align, AppBar, BarChart, BoxFit,
    Axis, FontWeight, SpringDescription, Autocomplete, Avatar, Breadcrumb, Card, Carousel, Checkbox, Chip, Collapsible, Color, ColorPicker,
    Container, CustomPaint, DataTable, DatePicker, Divider, Dropdown, Flex, Grid, Icon, IconName, Image, ImageData, ImageHandle, Insets, Justify, Kbd, LayoutBuilder, LineChart, List,
    Kanban, NavBar, Navigator, Orientation, Pagination, Placement, Popover, Portal, ProgressBar,
    RadioGroup, Rating, Rect, RichText, Scaffold, Scroll, SegmentedControl, Size, SizeClass, Skeleton,
    Slider, Stack,
    TextSpan, WindowInsets,
    Stepper, Steps, Switch, Table, Tabs, TextInput, Theme, Timeline, Toast, ToastHost,
    ToastPosition, Tree, TwoPane, Variant, Widget, ErrorSummary, SnackbarQueue,
};
use frus_widgets::form::{Form, Rule};

/// Logo de démo **décodé** depuis un PNG embarqué (jalon 91), partagé pour tout
/// le process via `OnceLock` — décodé une fois, mis en cache par identité côté
/// renderer. Repli sur un dégradé généré si le décodage échoue (robustesse).
fn demo_image() -> ImageHandle {
    use std::sync::OnceLock;
    static IMG: OnceLock<ImageHandle> = OnceLock::new();
    IMG.get_or_init(|| {
        frus_image::decode(include_bytes!("../assets/logo.png"))
            .map(ImageData::into_handle)
            .unwrap_or_else(|_| fallback_gradient())
    })
    .clone()
}

/// Dégradé 64×64 généré — repli si le décodage du PNG échoue.
fn fallback_gradient() -> ImageHandle {
    const W: u32 = 64;
    const H: u32 = 64;
    let mut rgba = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        for x in 0..W {
            rgba.push((x * 255 / (W - 1)) as u8);
            rgba.push((y * 255 / (H - 1)) as u8);
            rgba.push(160u8);
            rgba.push(255u8);
        }
    }
    ImageData::from_rgba(W, H, rgba).into_handle()
}

/// Point d'entrée bureau : ouvre la fenêtre et lance la boucle winit.
#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> anyhow::Result<()> {
    frus_shell::run(TodoApp::default())
}

/// Point d'entrée Android : appelé par l'activité native, reçoit l'`AndroidApp`
/// et démarre la même application.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: frus_shell::AndroidApp) {
    if let Err(err) = frus_shell::run_android(TodoApp::default(), android_app) {
        log::error!("frus-demo (android) s'est arrêté : {err:#}");
    }
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

/// Le ressort partagé par la navigation et le geste retour, exprimé dans la
/// couche d'animation de `frus-core` (`trait Simulation`).
fn nav_spring() -> SpringDescription {
    SpringDescription::new(1.0, NAV_SPRING_K, NAV_SPRING_C)
}

/// Démarre une transition d'écran : le ressort pousse la progression `0 → 1`.
fn start_nav(app: &mut TodoApp, forward: bool) {
    app.nav_forward = forward;
    app.nav.set_value(0.0);
    app.nav.spring_to(1.0, nav_spring(), 0.0);
}

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
    /// Assistant d'inscription multi-étapes (démo d'intégration : Steps + Form + Snackbar).
    Wizard,
    /// Grille de données **éditable en ligne** (démo d'intégration : Table + TextInput par cellule).
    Grid,
    /// Tableau de bord **graphique** (démo d'intégration : LineChart + légende cliquable, jalon 218).
    Charts,
    /// Tableau de données **en lecture seule** (démo d'intégration : DataTable auto-triant + paginé,
    /// jalon 237).
    Data,
    /// Tableau **Kanban** : colonnes de cartes, glisser-déposer inter-colonnes (jalon 247).
    Board,
}

/// Geste retour : progression suivie au doigt, puis détente à ressort
/// (validation/annulation) pilotée par un [`AnimationController`].
struct BackGesture {
    progress: f32,
    velocity: f32,
    /// `Some` une fois relâché : la détente à ressort en cours (`None` = suivi du
    /// doigt).
    settle: Option<AnimationController>,
    /// La détente, à son terme, valide-t-elle le retour (dépilement) ?
    commit: bool,
}

/// Messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    DraftChanged(String),
    /// Efface le champ de saisie (icône suffixe « ✕ »).
    ClearDraft,
    AddTodo,
    ToggleTodo(u64),
    DeleteTodo(u64),
    SetFilter(Filter),
    AskClearDone,
    ConfirmClearDone,
    CancelClear,
    ToggleTheme,
    /// Passe à la graine de thème suivante (default → Blue → Purple → Orange).
    CycleSeed,
    /// Bascule la direction de mise en page (LTR ↔ RTL).
    ToggleRtl,
    /// Passe à la langue suivante (English ↔ Français).
    CycleLang,
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
    /// Déclenche la **sortie** (fondu) de la notification en tête, avant son retrait.
    ToastExpire,
    /// Retire la notification courante (fin de sortie / clic) et enchaîne sur la suivante.
    DismissToast,
    /// Change la page (démo pagination).
    SetPage(usize),
    /// Déplie/replie un nœud d'arbre.
    ToggleNode(u64),
    /// Sélectionne un nœud de l'arbre de fichiers (jalon 246).
    SelectNode(u64),
    /// Déplace une carte du Kanban : `(from_col, from_pos, to_col, to_pos)` (jalon 247).
    KanbanMove(usize, usize, usize, usize),
    /// Ajoute une carte au bas d'une colonne du Kanban (jalon 249).
    KanbanAdd(usize),
    /// Supprime la carte `(col, pos)` du Kanban (jalon 249).
    KanbanDelete(usize, usize),
    /// Choisit une couleur.
    PickColor(Color),
    /// Sélectionne un jour dans le calendrier.
    PickDay(u32),
    /// Change de mois (±1).
    NavMonth(i32),
    /// Change de slide du carrousel.
    SetSlide(usize),
    /// Règle la densité (zoom applicatif).
    SetDensity(f32),
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
    /// Ouvre/ferme le tiroir de navigation latéral.
    ToggleDrawer,
    /// Ouvre/ferme la feuille modale d'actions rapides.
    ToggleSheet,
    // --- Assistant d'inscription (démo d'intégration) ---
    /// Saute à l'étape `i` de l'assistant (marqueur Steps cliqué).
    WizardStep(usize),
    /// Saute à l'étape `usize` **et focalise** le champ `u8` (puce du récapitulatif cliquée).
    WizardFocus(usize, u8),
    /// Saisie d'un champ de l'assistant : `(0=nom, 1=email, 2=mot de passe, 3=confirmation)`.
    WizardInput(u8, String),
    /// Étape précédente / suivante de l'assistant.
    WizardBack,
    WizardNext,
    /// Révèle / masque les mots de passe de l'assistant.
    WizardToggleReveal,
    // --- Grille éditable (démo d'intégration) ---
    /// Nouvelle valeur de la cellule `(ligne, colonne)`.
    GridInput(usize, usize, String),
    /// Entrée pressée dans la cellule `(ligne, colonne)` : saute à la ligne suivante, même colonne.
    GridEnter(usize, usize),
    /// Ajoute une ligne vide en fin de grille et y place le curseur.
    GridAddRow,
    /// Supprime la ligne `ligne`.
    GridDeleteRow(usize),
    /// Trie la grille par la colonne `colonne` (bascule croissant / décroissant).
    GridSort(usize),
    /// Soumet la grille : n'aboutit que si toutes les cellules sont valides.
    GridSave,
    /// Place le focus sur la première cellule invalide de la grille (le cas échéant).
    GridFocusError,
    /// Bascule la visibilité de la série `index` du graphique (clic de légende, jalon 218).
    ChartToggleSeries(usize),
    /// Change le type de graphique affiché (sélecteur, jalon 219).
    SetChartKind(usize),
    /// Épingle le détail d'un point cliqué `(catégorie, série)` (jalon 221).
    ChartPoint(usize, usize),
    /// Active/désactive l'empilage **100 %** (proportions) du graphique (jalon 224).
    SetChartNormalized(bool),
    /// Clic sur un en-tête du tableau de données : (dé)trie la colonne (jalon 237).
    DataSort(usize),
    /// Change la page du tableau de données (jalon 237).
    DataPage(usize),
    /// Change la taille de page du tableau de données (jalon 237).
    DataPageSize(usize),
    /// Clic sur une ligne du tableau de données : (dé)sélectionne la ligne **source** (jalon 239).
    DataSelectRow(usize),
    /// Coche/décoche une ligne **source** du tableau de données (sélection multiple, jalon 241).
    DataCheck(usize),
    /// Case « tout cocher » du tableau de données : coche tout, ou vide si déjà tout coché (jalon 241).
    DataCheckAll,
    /// Frappe dans le champ de recherche du tableau de données : met à jour le filtre (jalon 242).
    DataSearch(String),
    /// Action groupée : désélectionne toutes les lignes cochées du tableau de données (jalon 243).
    DataClearChecked,
    /// Action groupée : ouvre la confirmation de suppression des lignes cochées (jalon 245).
    DataAskDelete,
    /// Ferme la confirmation de suppression sans rien supprimer (jalon 245).
    DataCancelDelete,
    /// Action groupée : supprime les lignes cochées du tableau de données (jalon 243).
    DataDeleteChecked,
    /// Bascule « jours ouvrés seulement » du calendrier de la vitrine (jalon 238).
    SetWeekdaysOnly(bool),
    /// Soumet l'assistant : valide le formulaire, notifie ou affiche les erreurs.
    WizardSubmit,
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
    /// Progression de la transition d'écran (`0 → 1`), pilotée par ressort.
    nav: AnimationController,
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
    /// L'app est-elle en **arrière-plan** ? (cycle de vie, jalon 259) — mis à `true` sur
    /// `Paused`/`Detached` via [`Application::on_lifecycle`] ; le minuteur se suspend alors. Défaut
    /// `false` (premier plan) — compatible `#[derive(Default)]`.
    background: bool,
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
    /// File de notifications (Snackbar) : une à la fois, avec sortie animée (jalon 193).
    snackbars: SnackbarQueue<String>,
    /// Page courante du sélecteur de pagination (démo).
    page: usize,
    /// Nœuds d'arbre dépliés (démo Tree).
    expanded: std::collections::HashSet<u64>,
    /// Nœud d'arbre sélectionné (démo Tree, jalon 246) ; `None` = aucun.
    tree_selected: Option<u64>,
    /// Couleur choisie (démo ColorPicker).
    picked: Option<Color>,
    /// Calendrier : année / mois (1..12) / jour sélectionné.
    year: i32,
    month: u32,
    selected_day: Option<u32>,
    /// Calendrier de la vitrine : désactiver les week-ends (`DatePicker::filtered`) ? — jalon 238.
    weekdays_only: bool,
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
    /// Densité (zoom applicatif de toute l'UI) — `1.0` par défaut.
    density: f32,
    /// Palier de taille courant (mis à jour par `on_resize`).
    size_class: Option<SizeClass>,
    /// Orientation courante (mise à jour par `on_resize`).
    orientation: Option<Orientation>,
    /// Tiroir de navigation latéral ouvert ?
    drawer_open: bool,
    /// Feuille modale d'actions rapides ouverte ?
    sheet_open: bool,
    /// Insets système (zone de sécurité) : barres d'état/navigation, encoches.
    insets: Insets,
    /// Graine du thème : `0` = schéma écrit main, sinon `from_seed` (HCT).
    seed_index: usize,
    /// Mise en page droite-à-gauche (arabe/hébreu) ?
    rtl: bool,
    /// Langue courante (index dans `LANGS` : 0 = English, 1 = Français).
    lang: usize,
    /// L'état vient d'un instantané live-reload : `init` ne recharge pas les
    /// tâches depuis le disque (l'instantané fait foi).
    restored: bool,
    // --- Assistant d'inscription (démo d'intégration) ---
    /// Étape courante de l'assistant (0 = Account, 1 = Security, 2 = Review).
    wizard_step: usize,
    wizard_name: String,
    wizard_email: String,
    wizard_pass: String,
    wizard_confirm: String,
    /// L'assistant a-t-il été soumis au moins une fois ? (n'affiche les erreurs qu'après.)
    wizard_submitted: bool,
    /// Les mots de passe de l'assistant sont-ils **révélés** (démasqués) ?
    wizard_reveal: bool,
    /// Données de la grille éditable (lignes × colonnes de texte).
    grid: Vec<Vec<String>>,
    /// Tri courant de la grille : `(colonne, croissant)` ; `None` = ordre de saisie.
    grid_sort: Option<(usize, bool)>,
    /// Dernière cellule fautive visée par « Next error » (jalon 214) — pour cycler à la suivante.
    grid_error_cursor: Option<(usize, usize)>,
    /// Index des séries **masquées** du graphique (basculées via la légende, jalon 218).
    chart_hidden: Vec<usize>,
    /// Type de graphique affiché : 0 lignes, 1 aires empilées, 2 barres groupées, 3 barres empilées (jalon 219).
    chart_kind: usize,
    /// Détail **épinglé** d'un point cliqué (`série · catégorie = valeur`), le cas échéant (jalon 221).
    chart_pin: Option<String>,
    /// Point/barre **sélectionné** `(catégorie, série)`, mis en évidence dans le graphique (jalon 223).
    chart_sel: Option<(usize, usize)>,
    /// Empilage **100 %** (proportions) activé pour les graphiques empilés (jalon 224).
    chart_normalized: bool,
    /// Tri du tableau de données `(colonne, croissant)` ; `None` = ordre source (jalon 237). Le tri
    /// d'affichage est fait par le `DataTable`, **pas** recopié ici.
    data_sort: Option<(usize, bool)>,
    /// Page courante (1-indexée) du tableau de données ; `0` = page 1 (jalon 237).
    data_page: usize,
    /// Taille de page du tableau de données ; `0` = défaut (jalon 237).
    data_page_size: usize,
    /// Ligne **source** sélectionnée du tableau de données ; `None` = aucune (jalon 239). Le
    /// `DataTable` traduit cet index d'origine en position surlignée à travers tri/pagination.
    data_selected: Option<usize>,
    /// Lignes **source** cochées (sélection multiple) du tableau de données (jalon 241). Pilote les
    /// cases + le surlignage ; l'app décide ce que « tout cocher » recouvre (ici les 12 lignes).
    data_checked: Vec<usize>,
    /// Requête de recherche du tableau de données (jalon 242) ; le `DataTable` filtre l'affichage.
    data_query: String,
    /// Lignes du tableau de données (jalon 243) : `None` = jeu de départ `DATA_PEOPLE` ; `Some` dès
    /// qu'une action groupée (Delete) les modifie. Voir [`TodoApp::data_rows`].
    data_rows: Option<Vec<(String, String, u32, String)>>,
    /// Modale de confirmation de suppression groupée du tableau de données ouverte ? (jalon 245)
    data_confirm_delete: bool,
    /// Cartes du Kanban par colonne (jalon 247) ; `None` = jeu de départ `KANBAN_SEED`. `Some` dès
    /// qu'une carte est déplacée. Voir [`TodoApp::kanban_cols`].
    kanban: Option<Vec<Vec<String>>>,
}

fn current_route(app: &TodoApp) -> Route {
    app.routes.last().copied().unwrap_or(Route::Home)
}

impl TodoApp {
    /// Lignes courantes du tableau de données : celles de l'état si elles ont été modifiées
    /// (Delete groupé), sinon le jeu de départ `DATA_PEOPLE` (jalon 243).
    fn data_rows(&self) -> Vec<(String, String, u32, String)> {
        match &self.data_rows {
            Some(rows) => rows.clone(),
            None => {
                DATA_PEOPLE.iter().map(|(n, r, s, l)| (n.to_string(), r.to_string(), *s, l.to_string())).collect()
            }
        }
    }

    /// Cartes du Kanban par colonne : celles de l'état si déplacées, sinon le jeu de départ
    /// `KANBAN_SEED` (jalon 247).
    fn kanban_cols(&self) -> Vec<Vec<String>> {
        match &self.kanban {
            Some(cols) => cols.clone(),
            None => KANBAN_SEED.iter().map(|col| col.iter().map(|s| s.to_string()).collect()).collect(),
        }
    }
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

/// Graines de démonstration du thème dynamique (`from_seed`, HCT).
const THEME_SEEDS: [(&str, Color); 3] = [
    ("Blue", Color { r: 0x42 as f32 / 255.0, g: 0x85 as f32 / 255.0, b: 0xF4 as f32 / 255.0, a: 1.0 }),
    ("Purple", Color { r: 0x9C as f32 / 255.0, g: 0x27 as f32 / 255.0, b: 0xB0 as f32 / 255.0, a: 1.0 }),
    ("Orange", Color { r: 0xE8 as f32 / 255.0, g: 0x71 as f32 / 255.0, b: 0x0A as f32 / 255.0, a: 1.0 }),
];

/// Libellé de l'action « graine » du menu (la **prochaine** graine du cycle).
fn seed_label(app: &TodoApp) -> String {
    match THEME_SEEDS.get(app.seed_index) {
        Some((name, _)) => format!("Seed: {name}"),
        None => "Seed: default".to_string(),
    }
}

/// Thème « cible » selon l'état (avant fondu) : schéma écrit main par défaut,
/// ou généré depuis une graine (Material 3 `from_seed`, HCT).
fn theme_of(app: &TodoApp) -> Theme {
    let theme = match app.seed_index.checked_sub(1).and_then(|i| THEME_SEEDS.get(i)) {
        Some((_, seed)) => Theme::from_seed(*seed, !app.light),
        None => {
            if app.light {
                Theme::light()
            } else {
                Theme::dark()
            }
        }
    };
    // Direction ambiante : RTL si l'utilisateur l'a demandé OU si la langue
    // courante s'écrit de droite à gauche (arabe). Tout le layout se retourne.
    if app.rtl || lang_is_rtl(app.lang) {
        theme.rtl()
    } else {
        theme
    }
}

/// Applique un message à l'état et renvoie l'effet éventuel à exécuter.
/// Effet minuté : déclenche la **sortie** de la notification en tête après ~2 s de présence.
fn toast_expire_after() -> Command<Msg> {
    Command::perform(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        Msg::ToastExpire
    })
}

/// Empile une notification (Snackbar) ; si elle devient la **tête** de file, programme sa sortie.
fn show_toast(app: &mut TodoApp, text: &str) -> Command<Msg> {
    let was_empty = app.snackbars.is_empty();
    app.snackbars.push(text.to_string(), 0.0);
    if was_empty {
        toast_expire_after()
    } else {
        Command::none()
    }
}

fn reduce(app: &mut TodoApp, message: Msg) -> Command<Msg> {
    match message {
        Msg::ClearDraft => {
            app.draft.clear();
            Command::none()
        }
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
            app.sheet_open = false;
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
        Msg::CycleSeed => {
            // Même fondu que la bascule clair/sombre, vers le schéma généré.
            app.theme_from = Some(theme_of(app));
            app.seed_index = (app.seed_index + 1) % (THEME_SEEDS.len() + 1);
            app.theme_progress = 0.0;
            Command::none()
        }
        Msg::ToggleRtl => {
            // La direction est discrète (pas de fondu) : bascule immédiate.
            app.rtl = !app.rtl;
            Command::none()
        }
        Msg::CycleLang => {
            app.lang = (app.lang + 1) % LANGS.len();
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
            app.drawer_open = false;
            app.menu_open = false;
            app.nav_from = Some(current_route(app));
            app.routes.push(route);
            start_nav(app, true);
            Command::none()
        }
        Msg::Pop => {
            if !app.routes.is_empty() {
                app.nav_from = Some(current_route(app));
                app.routes.pop();
                start_nav(app, false);
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
            app.sheet_open = false;
            // Capture un instantané sérialisable ; l'écriture se fait hors update.
            let items: Vec<(bool, String)> =
                app.todos.iter().map(|t| (t.done, t.text.clone())).collect();
            // Affiche une notification (Snackbar) et écrit sur disque (effet).
            let show = show_toast(app, "Saved");
            Command::batch([
                Command::run(move || {
                    let _ = save_todos(&todos_path(), &items);
                    None
                }),
                show,
            ])
        }
        Msg::ToastExpire => {
            // Passe la notification en **sortie** (l'hôte joue le fondu), puis la retire.
            app.snackbars.start_leaving();
            Command::perform(|| {
                std::thread::sleep(std::time::Duration::from_millis(300));
                Msg::DismissToast
            })
        }
        Msg::DismissToast => {
            app.snackbars.dismiss();
            // Enchaîne sur la notification suivante en file, s'il y en a une.
            if app.snackbars.is_empty() {
                Command::none()
            } else {
                toast_expire_after()
            }
        }
        Msg::WizardStep(i) => {
            app.wizard_step = i.min(2);
            Command::none()
        }
        Msg::WizardFocus(step, field) => {
            // Saute à l'étape du champ fautif puis focalise ce champ (keyed + Command::focus).
            app.wizard_step = step.min(2);
            Command::focus(("wizard", field))
        }
        Msg::WizardInput(field, value) => {
            match field {
                0 => app.wizard_name = value,
                1 => app.wizard_email = value,
                2 => app.wizard_pass = value,
                _ => app.wizard_confirm = value,
            }
            Command::none()
        }
        Msg::WizardBack => {
            app.wizard_step = app.wizard_step.saturating_sub(1);
            Command::none()
        }
        Msg::WizardNext => {
            app.wizard_step = (app.wizard_step + 1).min(2);
            Command::none()
        }
        Msg::WizardToggleReveal => {
            app.wizard_reveal = !app.wizard_reveal;
            Command::none()
        }
        Msg::GridInput(r, c, value) => {
            if let Some(cell) = app.grid.get_mut(r).and_then(|row| row.get_mut(c)) {
                *cell = value;
            }
            Command::none()
        }
        Msg::GridEnter(r, c) => {
            // Entrée = descendre d'une ligne (même colonne) ; sur la dernière, on en crée une.
            if r + 1 < app.grid.len() {
                Command::focus(("grid", r + 1, c))
            } else {
                let cols = app.grid.first().map(|row| row.len()).unwrap_or(3);
                app.grid.push(vec![String::new(); cols]);
                Command::focus(("grid", r + 1, c))
            }
        }
        Msg::GridAddRow => {
            let cols = app.grid.first().map(|row| row.len()).unwrap_or(3);
            app.grid.push(vec![String::new(); cols]);
            // Focalise la première cellule de la nouvelle ligne.
            Command::focus(("grid", app.grid.len() - 1, 0))
        }
        Msg::GridDeleteRow(r) => {
            if r < app.grid.len() {
                app.grid.remove(r);
            }
            Command::none()
        }
        Msg::GridSave => {
            let errors = grid_error_count(&app.grid);
            if errors == 0 {
                return show_toast(app, "Grid saved");
            }
            let msg = if errors == 1 {
                "Fix 1 error before saving".to_string()
            } else {
                format!("Fix {errors} errors before saving")
            };
            return show_toast(app, &msg);
        }
        Msg::GridFocusError => {
            // Cycle vers la cellule fautive suivante (bouclage), et la focalise.
            match grid_next_error(&app.grid, app.grid_error_cursor) {
                Some(pos) => {
                    app.grid_error_cursor = Some(pos);
                    Command::focus(("grid", pos.0, pos.1))
                }
                None => {
                    app.grid_error_cursor = None;
                    Command::none()
                }
            }
        }
        Msg::ChartToggleSeries(i) => {
            // Clic de légende : masque la série si visible, la ré-affiche sinon.
            if let Some(pos) = app.chart_hidden.iter().position(|&h| h == i) {
                app.chart_hidden.remove(pos);
            } else {
                app.chart_hidden.push(i);
            }
            Command::none()
        }
        Msg::SetChartKind(k) => {
            app.chart_kind = k;
            Command::none()
        }
        Msg::SetChartNormalized(on) => {
            app.chart_normalized = on;
            Command::none()
        }
        Msg::DataSort(c) => {
            // Bascule croissant/décroissant (nouvelle colonne = croissant). Le `DataTable` trie
            // lui-même l'affichage — on ne recopie **pas** de `sort_by` ici (jalon 237).
            let asc = match app.data_sort {
                Some((col, asc)) if col == c => !asc,
                _ => true,
            };
            app.data_sort = Some((c, asc));
            app.data_page = 1; // le tri ramène à la première page
            Command::none()
        }
        Msg::DataPage(p) => {
            app.data_page = p;
            Command::none()
        }
        Msg::DataPageSize(s) => {
            app.data_page_size = s;
            app.data_page = 1; // changer la taille ramène à la première page
            Command::none()
        }
        Msg::DataSelectRow(i) => {
            // Re-clic sur la ligne déjà sélectionnée = désélection (bascule).
            app.data_selected = if app.data_selected == Some(i) { None } else { Some(i) };
            Command::none()
        }
        Msg::DataCheck(i) => {
            // Bascule l'appartenance de la ligne source à l'ensemble coché.
            match app.data_checked.iter().position(|&x| x == i) {
                Some(pos) => {
                    app.data_checked.remove(pos);
                }
                None => app.data_checked.push(i),
            }
            Command::none()
        }
        Msg::DataCheckAll => {
            // Tout coché → on vide ; sinon on coche toutes les lignes source courantes.
            let n = app.data_rows().len();
            app.data_checked = if app.data_checked.len() == n { Vec::new() } else { (0..n).collect() };
            Command::none()
        }
        Msg::DataSearch(q) => {
            app.data_query = q;
            app.data_page = 1; // un nouveau filtre ramène à la première page
            Command::none()
        }
        Msg::DataClearChecked => {
            app.data_checked.clear();
            Command::none()
        }
        Msg::DataAskDelete => {
            app.data_confirm_delete = true;
            Command::none()
        }
        Msg::DataCancelDelete => {
            app.data_confirm_delete = false;
            Command::none()
        }
        Msg::DataDeleteChecked => {
            // Supprime réellement les lignes cochées (index source, en ordre décroissant pour ne pas
            // décaler les suivants), puis remet à zéro sélection, focus et modale.
            app.data_confirm_delete = false;
            let mut rows = app.data_rows();
            let mut checked: Vec<usize> = app.data_checked.iter().copied().filter(|&i| i < rows.len()).collect();
            checked.sort_unstable();
            checked.dedup();
            for &i in checked.iter().rev() {
                rows.remove(i);
            }
            app.data_rows = Some(rows);
            app.data_checked.clear();
            app.data_selected = None;
            Command::none()
        }
        Msg::SetWeekdaysOnly(on) => {
            app.weekdays_only = on;
            Command::none()
        }
        Msg::ChartPoint(cat, s) => {
            // Re-clic sur l'élément déjà sélectionné : on **désépingle** (jalon 225). Sinon on épingle
            // « série · catégorie = valeur » et on retient la sélection `(cat, série)` (jalons 221/223).
            if app.chart_sel == Some((cat, s)) {
                app.chart_sel = None;
                app.chart_pin = None;
            } else if let (Some((name, vals)), Some(label)) = (CHART_SERIES.get(s), CHART_CATS.get(cat)) {
                if let Some(v) = vals.get(cat) {
                    app.chart_pin = Some(format!("{name} · {label} = {}", *v as i64));
                    app.chart_sel = Some((cat, s));
                }
            }
            Command::none()
        }
        Msg::GridSort(c) => {
            // Bascule croissant / décroissant sur la colonne cliquée, puis trie les lignes.
            let asc = match app.grid_sort {
                Some((col, asc)) if col == c => !asc,
                _ => true,
            };
            app.grid_sort = Some((c, asc));
            app.grid.sort_by(|a, b| {
                let (x, y) = (a.get(c).map(String::as_str).unwrap_or(""), b.get(c).map(String::as_str).unwrap_or(""));
                let ord = x.to_lowercase().cmp(&y.to_lowercase());
                if asc { ord } else { ord.reverse() }
            });
            Command::none()
        }
        Msg::WizardSubmit => {
            if wizard_form(app).is_valid() {
                // Succès : réinitialise l'assistant et notifie (Snackbar, sortie animée).
                app.wizard_step = 0;
                app.wizard_name.clear();
                app.wizard_email.clear();
                app.wizard_pass.clear();
                app.wizard_confirm.clear();
                app.wizard_submitted = false;
                show_toast(app, "Account created")
            } else {
                // Erreurs : les révèle et montre le récapitulatif sur l'étape Review.
                app.wizard_submitted = true;
                app.wizard_step = 2;
                Command::none()
            }
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
        Msg::SelectNode(id) => {
            // Re-clic sur le nœud déjà sélectionné = désélection (bascule).
            app.tree_selected = if app.tree_selected == Some(id) { None } else { Some(id) };
            Command::none()
        }
        Msg::KanbanMove(from_col, from_pos, to_col, to_pos) => {
            let mut cols = app.kanban_cols();
            if from_col < cols.len() && from_pos < cols[from_col].len() && to_col < cols.len() {
                let card = cols[from_col].remove(from_pos);
                // Après retrait, une cible plus loin dans la **même** colonne se décale d'un cran.
                let mut tp = to_pos;
                if from_col == to_col && from_pos < tp {
                    tp -= 1;
                }
                tp = tp.min(cols[to_col].len());
                cols[to_col].insert(tp, card);
                app.kanban = Some(cols);
            }
            Command::none()
        }
        Msg::KanbanAdd(col) => {
            let mut cols = app.kanban_cols();
            if col < cols.len() {
                cols[col].push("New card".to_string());
                app.kanban = Some(cols);
            }
            Command::none()
        }
        Msg::KanbanDelete(col, pos) => {
            let mut cols = app.kanban_cols();
            if col < cols.len() && pos < cols[col].len() {
                cols[col].remove(pos);
                app.kanban = Some(cols);
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
            // Choisir une section depuis le tiroir le referme.
            app.drawer_open = false;
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
        Msg::SetDensity(d) => {
            app.density = d.clamp(0.8, 1.4);
            Command::none()
        }
        Msg::ToggleDrawer => {
            app.drawer_open = !app.drawer_open;
            Command::none()
        }
        Msg::ToggleSheet => {
            app.sheet_open = !app.sheet_open;
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
        self.density = 1.0;
        // Données de démonstration de la grille éditable.
        self.grid = vec![
            vec!["Ada Lovelace".into(), "Engineer".into(), "ada@example.com".into()],
            vec!["Alan Turing".into(), "Cryptographer".into(), "alan@example.com".into()],
            vec!["Grace Hopper".into(), "Admiral".into(), "grace@example.com".into()],
        ];
        if self.restored {
            // Live-reload : l'instantané fait foi, ne pas l'écraser du disque.
            return Command::none();
        }
        Command::perform(|| Msg::Loaded(load_todos(&todos_path())))
    }

    /// Live-reload : l'essentiel de l'état survit à la recompilation — tâches,
    /// brouillon, filtre, thème (clair/sombre + graine), onglet et écran.
    fn save_state(&self) -> Option<Vec<u8>> {
        let mut out = String::from("frus-demo-state v1\n");
        out.push_str(&format!("light {}\n", self.light as u8));
        out.push_str(&format!("seed {}\n", self.seed_index));
        out.push_str(&format!("filter {}\n", filter_index(self.filter)));
        out.push_str(&format!("section {}\n", self.section));
        let route = match current_route(self) {
            Route::Home => 0,
            Route::Settings => 1,
            Route::Journal => 2,
            Route::Wizard => 3,
            Route::Grid => 4,
            Route::Charts => 5,
            Route::Data => 6,
            Route::Board => 7,
        };
        out.push_str(&format!("route {route}\n"));
        out.push_str(&format!("draft {}\n", self.draft));
        for todo in &self.todos {
            out.push_str(&format!("todo {}\t{}\n", todo.done as u8, todo.text));
        }
        Some(out.into_bytes())
    }

    /// Réhydrate un instantané [`Application::save_state`] — tolérant : toute
    /// ligne inconnue (autre version du code) est ignorée.
    fn restore_state(&mut self, bytes: &[u8]) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        let mut lines = text.lines();
        if lines.next() != Some("frus-demo-state v1") {
            return;
        }
        for line in lines {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "light" => self.light = value == "1",
                "seed" => self.seed_index = value.parse().unwrap_or(0),
                "filter" => self.filter = filter_from_index(value.parse().unwrap_or(0)),
                "section" => self.section = value.parse().unwrap_or(0),
                "route" => {
                    self.routes.clear();
                    match value {
                        "1" => self.routes.push(Route::Settings),
                        "2" => self.routes.push(Route::Journal),
                        "3" => self.routes.push(Route::Wizard),
                        "4" => self.routes.push(Route::Grid),
                        "5" => self.routes.push(Route::Charts),
                        "6" => self.routes.push(Route::Data),
                        "7" => self.routes.push(Route::Board),
                        _ => {}
                    }
                }
                "draft" => self.draft = value.to_string(),
                "todo" => {
                    if let Some((done, text)) = value.split_once('\t') {
                        let id = self.next_id;
                        self.next_id += 1;
                        self.todos.push(Todo {
                            id,
                            text: text.to_string(),
                            done: done == "1",
                        });
                    }
                }
                _ => {}
            }
        }
        self.restored = true;
    }

    fn subscription(&self) -> Subscription<Msg> {
        // Un tick par seconde tant que le chrono tourne **et** que l'app est au premier plan : en
        // arrière-plan (`on_lifecycle` → `foreground = false`) le minuteur se **suspend**, façon
        // Flutter (le framework arrête la souscription par diff, puis la relance au retour).
        if self.running && !self.background {
            Subscription::every(Duration::from_secs(1), |_| Msg::Tick)
        } else {
            Subscription::none()
        }
    }

    fn density(&self) -> f32 {
        if self.density > 0.0 {
            self.density
        } else {
            1.0
        }
    }

    fn on_resize(&mut self, width: f32, height: f32) {
        // Réagit au changement de palier : ferme le détail Stats en étroit.
        let class = SizeClass::from_width(width);
        if self.size_class != Some(class) {
            self.size_class = Some(class);
            if class == SizeClass::Compact {
                self.stat_detail_open = false;
            }
            eprintln!("[demo] palier : {class:?}");
        }
        // Axe supplémentaire de responsivité (Lot C) : orientation portrait/paysage.
        let orientation = Orientation::from_size(width, height);
        if self.orientation != Some(orientation) {
            self.orientation = Some(orientation);
            eprintln!("[demo] orientation : {orientation:?}");
        }
    }

    fn on_insets(&mut self, insets: WindowInsets) {
        // Zone sûre totale : barres système **et** clavier logiciel — le contenu
        // (champs de saisie compris) reste au-dessus du clavier.
        let safe = insets.safe();
        if self.insets != safe {
            self.insets = safe;
            eprintln!("[demo] insets : {safe:?}");
        }
    }

    fn on_lifecycle(&mut self, state: Lifecycle) {
        // Démonstration du contrat de cycle de vie (jalon 259) : on **suspend le chrono** en
        // arrière-plan et on le **reprend** au premier plan — comme une app Flutter couperait ses
        // minuteurs/capteurs. La trace part aussi dans logcat (vérification sur appareil).
        eprintln!("[demo] lifecycle : {state:?}");
        // Suspend en arrière-plan (Paused/Detached) ; garde le minuteur en `Inactive` (perte de
        // focus mais toujours visible) — comme Flutter suspend sur `paused`, pas sur `inactive`.
        self.background = matches!(state, Lifecycle::Paused | Lifecycle::Detached);
    }

    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        // Zone de sécurité : on construit l'interface aux dimensions **internes**
        // (fenêtre moins insets système), puis on l'enrobe d'un fond plein-fenêtre
        // écarté par `padding` — le fond s'étend sous les barres, le contenu non.
        let i = self.insets;
        let w = (width - i.left - i.right).max(0.0);
        let h = (height - i.top - i.bottom).max(0.0);
        let nav = build_view(self, theme, w, h);
        if i == Insets::ZERO {
            Box::new(nav)
        } else {
            Box::new(
                Container::new()
                    .width(width)
                    .height(height)
                    .color(theme.background)
                    .padding_each(i.top, i.right, i.bottom, i.left)
                    .child(nav),
            )
        }
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

        // Transition d'écran : le contrôleur échantillonne le ressort partagé.
        if self.nav_from.is_some() {
            if self.nav.tick(dt) {
                animating = true;
            } else {
                self.nav_from = None;
            }
        }

        // Détente du geste retour (même ressort, amorcé par l'élan du doigt).
        let mut commit_back = false;
        if let Some(g) = self.back.as_mut() {
            if let Some(settle) = g.settle.as_mut() {
                if settle.tick(dt) {
                    g.progress = settle.value();
                    animating = true;
                } else {
                    commit_back = g.commit;
                    self.back = None;
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
        !self.routes.is_empty()
            && !self.confirm_clear
            && !self.data_confirm_delete
            && !self.menu_open
            && !self.drawer_open
            && !self.sheet_open
    }

    fn back_gesture(&mut self, progress: f32) {
        match self.back.as_mut() {
            Some(g) => g.progress = progress,
            None => {
                self.back = Some(BackGesture {
                    progress,
                    velocity: 0.0,
                    settle: None,
                    commit: false,
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
            g.commit = commit;
            // Détente à ressort depuis la position courante, amorcée par l'élan
            // du doigt, vers la cible (validée `1` ou annulée `0`).
            let mut settle = AnimationController::unit();
            settle.set_value(g.progress);
            settle.spring_to(if commit { 1.0 } else { 0.0 }, nav_spring(), velocity);
            g.settle = Some(settle);
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
            app.nav.value(),
            app.nav_forward,
        ),
        None => Navigator::new(current, width, height),
    }
}

/// Construit l'écran correspondant à une route.
fn screen(route: Route, app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    match route {
        Route::Home => todo_screen(app, theme, width, height),
        Route::Settings => Box::new(settings_screen(app, theme, width, height)),
        Route::Journal => Box::new(journal_screen(theme, width, height)),
        Route::Wizard => wizard_screen(app, theme, width, height),
        Route::Grid => grid_screen(app, theme, width, height),
        Route::Charts => charts_screen(app, theme, width, height),
        Route::Data => data_screen(app, theme, width, height),
        Route::Board => board_screen(app, theme, width, height),
    }
}

/// Validation **pure** d'une cellule de la grille : `Name` (col 0) obligatoire, `Email` (col 2)
/// doit ressembler à une adresse. `None` = valide. Démontre `TextInput::error` par cellule.
fn grid_cell_error(col: usize, value: &str) -> Option<&'static str> {
    match col {
        0 if value.trim().is_empty() => Some("Required"),
        2 if !value.is_empty() && !(value.contains('@') && value.contains('.')) => {
            Some("Invalid email")
        }
        _ => None,
    }
}

/// Nombre total de cellules invalides dans la grille (jalon 204/207) — sert à jauger la soumission.
fn grid_error_count(grid: &[Vec<String>]) -> usize {
    grid.iter()
        .flat_map(|row| (0..3).filter(move |&c| grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or("")).is_some()))
        .count()
}

/// Toutes les cellules invalides `(ligne, colonne)`, en ordre ligne par ligne.
fn grid_faults(grid: &[Vec<String>]) -> Vec<(usize, usize)> {
    grid.iter()
        .enumerate()
        .flat_map(|(r, row)| {
            (0..3).filter_map(move |c| {
                grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or(""))
                    .is_some()
                    .then_some((r, c))
            })
        })
        .collect()
}

/// La **première** cellule invalide (jalon 210).
fn grid_first_error(grid: &[Vec<String>]) -> Option<(usize, usize)> {
    grid_faults(grid).first().copied()
}

/// La cellule fautive **suivant** `after` (ordre ligne par ligne, bouclage en fin) — pour cycler
/// entre toutes les fautes (jalon 214). `after = None` renvoie la première.
fn grid_next_error(grid: &[Vec<String>], after: Option<(usize, usize)>) -> Option<(usize, usize)> {
    let faults = grid_faults(grid);
    match after {
        None => faults.first().copied(),
        Some(cur) => faults.iter().copied().find(|&f| f > cur).or_else(|| faults.first().copied()),
    }
}

/// Écran **grille éditable** : une `Table` dont chaque cellule est un `TextInput` toujours éditable.
/// Tab / Maj+Tab navigue de cellule en cellule (focusables du shell), Entrée descend d'une ligne
/// (jalon 201). Les en-têtes trient (jalon 204, `on_sort`), les cellules invalides s'affichent en
/// erreur, et Entrée sur la dernière ligne en crée une.
fn grid_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    const COL_W: [f32; 3] = [190.0, 170.0, 240.0];
    let muted = theme.muted;
    let mut table = Table::new(4)
        .header(&["Name", "Role", "Email", ""])
        .column_widths(&[COL_W[0], COL_W[1], COL_W[2], 44.0])
        .on_sort(Msg::GridSort);
    // Indicateur de tri (flèche d'en-tête) sur la colonne triée.
    if let Some((col, asc)) = app.grid_sort {
        table = table.sorted(col, asc);
    }
    for (r, row) in app.grid.iter().enumerate() {
        let mut cells: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>> = (0..3)
            .map(|c| {
                let value = row[c].clone();
                let w = COL_W[c] - 14.0;
                let err = grid_cell_error(c, &value);
                let factory: Box<dyn Fn() -> Box<dyn Widget<Msg>>> = Box::new(move || {
                    let mut input = TextInput::new(value.clone())
                        .width(w)
                        .size(15.0)
                        .on_input(move |v| Msg::GridInput(r, c, v))
                        .on_submit(Msg::GridEnter(r, c));
                    if let Some(e) = err {
                        input = input.error(e);
                    }
                    Box::new(keyed(("grid", r, c), input)) as Box<dyn Widget<Msg>>
                });
                factory
            })
            .collect();
        // Bouton de suppression de ligne (Container non focusable : Tab le saute).
        cells.push(Box::new(move || {
            Box::new(
                Container::<Msg>::new()
                    .padding(6.0)
                    .child(Icon::new(IconName::Close).size(16.0).color(muted))
                    .on_click(Msg::GridDeleteRow(r)),
            ) as Box<dyn Widget<Msg>>
        }));
        table = table.widget_row(cells);
    }
    let hint = text("Click a header to sort, Tab between cells, Enter for the next row.")
        .size(13.0)
        .color(theme.muted);
    // Barre d'état de validation : verte si tout est valide, sinon le décompte d'erreurs.
    let errors = grid_error_count(&app.grid);
    let status = if errors == 0 {
        text("All cells valid").size(13.0).color(theme.primary)
    } else {
        let label = if errors == 1 { "1 error".to_string() } else { format!("{errors} errors") };
        text(label).size(13.0).color(theme.error)
    };
    let add = button("Add row", Msg::GridAddRow);
    // `Save` est désactivé (non cliquable) tant qu'une cellule est invalide (jalon 210).
    let save = button("Save", Msg::GridSave).enabled(errors == 0);
    let mut actions = row![add, save].gap(12.0).align(Align::Center);
    // Raccourci qui cycle entre les cellules fautives, seulement s'il y a des erreurs.
    if errors > 0 {
        actions = actions.child(button("Next error", Msg::GridFocusError));
    }
    actions = actions.child(status);
    let body = column![table, actions, hint].gap(16.0).padding(24.0);
    let screen = column![NavBar::new("Editable grid").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Box::new(Container::new().width(width).height(height).color(theme.background).child(screen))
}

/// Jeu de données statique (nom, rôle, score) du tableau de données — jalon 237.
const DATA_PEOPLE: [(&str, &str, u32, &str); 12] = [
    ("Ada Lovelace", "Engineer", 92, "High"),
    ("Alan Turing", "Cryptographer", 88, "Medium"),
    ("Grace Hopper", "Admiral", 95, "High"),
    ("Katherine Johnson", "Mathematician", 90, "Medium"),
    ("Edsger Dijkstra", "Researcher", 84, "Low"),
    ("Barbara Liskov", "Professor", 91, "High"),
    ("Donald Knuth", "Author", 87, "Low"),
    ("Margaret Hamilton", "Director", 93, "High"),
    ("Tim Berners-Lee", "Inventor", 89, "Medium"),
    ("Linus Torvalds", "Maintainer", 86, "Low"),
    ("Radia Perlman", "Engineer", 90, "Medium"),
    ("Vint Cerf", "Architect", 85, "Low"),
];

/// Rang sémantique d'un niveau de priorité (`Low < Medium < High`) — clé de tri **personnalisée**
/// de la colonne « Level » du tableau de données (le tri texte la classerait alphabétiquement).
fn level_rank(s: &str) -> u8 {
    match s {
        "Low" => 0,
        "Medium" => 1,
        "High" => 2,
        _ => 3,
    }
}

/// Écran **tableau de données** : un `DataTable` en lecture seule qui **trie ses propres lignes**
/// (jalon 232) et **pagine** avec sélecteur de taille (jalons 233/236). L'app ne garde que l'état
/// `(tri, page, taille)` — le tri d'affichage n'est **pas** recopié dans le reducer (contraste
/// volontaire avec la grille éditable voisine). — jalon 237.
fn data_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let people = app.data_rows();
    let rows: Vec<Vec<String>> =
        people.iter().map(|(n, r, s, l)| vec![n.clone(), r.clone(), s.to_string(), l.clone()]).collect();
    // `0` (défaut dérivé) = valeurs de départ raisonnables.
    let per = if app.data_page_size == 0 { 5 } else { app.data_page_size };
    let page = app.data_page.max(1);
    let mut table: DataTable<Msg> = DataTable::new(["Name", "Role", "Score", "Level"], rows)
        .column_widths(&[200.0, 170.0, 90.0, 110.0])
        // Colonne « Level » : clé de tri **personnalisée** (Low < Medium < High), sinon le tri
        // texte la classerait par ordre alphabétique (jalon 240).
        .sort_with(3, |a, b| level_rank(a).cmp(&level_rank(b)))
        // Recherche : filtre les lignes source (toutes colonnes) avant tri/pagination (jalon 242).
        .searchable(app.data_query.as_str(), Msg::DataSearch)
        // État vide surchargé (jalon 244) : message quand le filtre/les données ne montrent rien.
        .empty_text("No people match your search")
        .on_sort(Msg::DataSort)
        .on_select_row(Msg::DataSelectRow)
        // Sélection multiple (jalon 241) : cases à cocher pour une sélection groupée, en plus du
        // clic de ligne (focus). Les lignes cochées pilotent le surlignage via `selected`.
        .checkboxes(Msg::DataCheck, Msg::DataCheckAll)
        .selected(&app.data_checked)
        // Barre d'actions groupées (jalon 243) : visible quand des lignes sont cochées.
        .bulk_actions(|| {
            vec![
                Box::new(button("Clear", Msg::DataClearChecked).variant(Variant::Secondary).size(14.0))
                    as Box<dyn Widget<Msg>>,
                Box::new(button("Delete", Msg::DataAskDelete).variant(Variant::Danger).size(14.0)),
            ]
        })
        .paginated(page, per, Msg::DataPage)
        .page_sizes(&[5, 10], Msg::DataPageSize);
    if let Some((col, asc)) = app.data_sort {
        table = table.sorted(col, asc);
    }
    let hint = text("Check rows for bulk actions; click a row to focus it; click a header to sort.")
        .size(13.0)
        .color(theme.muted);
    // Détail de la ligne **focalisée** (clic sur le corps de la ligne) : lue dans les données courantes.
    let detail = match app.data_selected.and_then(|i| people.get(i)) {
        Some((n, r, s, l)) => {
            text(format!("Focused: {n} — {r} (score {s}, {l} priority)")).size(15.0).color(theme.on_surface)
        }
        None => text("No row focused.").size(15.0).color(theme.muted),
    };
    // Résumé de la sélection **groupée** (cases cochées).
    let summary = text(format!("{} checked", app.data_checked.len())).size(13.0).color(theme.muted);
    let body = column![table, detail, summary, hint].gap(16.0).padding(24.0);
    let screen = column![NavBar::new("Data table").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    let content = Container::new().width(width).height(height).color(theme.background).child(screen);
    // Confirmation avant suppression groupée (jalon 245) : une modale centrée, fermable au clic
    // extérieur/Échap (`dismiss`), superposée à l'écran.
    if app.data_confirm_delete {
        Box::new(
            Portal::new(content)
                .overlay(data_confirm_content(app.data_checked.len()), Placement::Center)
                .dismiss(Msg::DataCancelDelete),
        )
    } else {
        Box::new(content)
    }
}

/// Titres des colonnes du Kanban (démo, jalon 247).
const KANBAN_TITLES: [&str; 3] = ["To do", "Doing", "Done"];
/// Cartes de départ du Kanban, par colonne (démo, jalon 247).
const KANBAN_SEED: [&[&str]; 3] = [
    &["Design API", "Write spec", "Triage bugs"],
    &["Build widget"],
    &["Kickoff", "Research"],
];

/// Une **carte riche** du Kanban (jalon 249) : le libellé à gauche, un bouton **×** de suppression à
/// droite (`KanbanDelete(col, pos)`).
fn rich_card(label: &str, col: usize, pos: usize) -> Box<dyn Widget<Msg>> {
    Box::new(
        row![
            text(label).size(14.0),
            Flex::row().flex(1.0),
            button("×", Msg::KanbanDelete(col, pos)).variant(Variant::Danger).size(12.0),
        ]
        .align(Align::Center)
        .gap(8.0),
    )
}

/// Écran **Kanban** : des colonnes de **cartes riches** (libellé + × de suppression), avec ajout par
/// colonne (jalon 249) et glisser-déposer inter-colonnes (jalon 247). L'app tient les cartes ; le widget
/// émet `KanbanMove`/`KanbanAdd`/`KanbanDelete`, le reducer applique.
fn board_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let cols = app.kanban_cols();
    let mut board = Kanban::new(Msg::KanbanMove).on_add(Msg::KanbanAdd);
    for (c, title) in KANBAN_TITLES.iter().enumerate() {
        let cards = cols.get(c).cloned().unwrap_or_default();
        let factories: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>> = cards
            .iter()
            .enumerate()
            .map(|(pos, label)| {
                let label = label.clone();
                Box::new(move || rich_card(&label, c, pos)) as Box<dyn Fn() -> Box<dyn Widget<Msg>>>
            })
            .collect();
        board = board.column_widgets(*title, factories);
    }
    // Le hint **s'enroule** dans la largeur (sinon la ligne déborde à droite hors écran).
    let hint = text("Add cards with + Add card; remove with ×; drag a card to move it.")
        .size(13.0)
        .color(theme.muted)
        .wrap();
    // Le board (rangée de colonnes de largeur fixe) dépasse souvent la largeur — et sa hauteur peut
    // dépasser l'écran : on le rend **défilable en 2D** dans un viewport qui remplit l'espace sous la
    // barre (comme Flutter borne le contenu au viewport et le fait défiler). Le padding est **dans**
    // le contenu défilé pour garder une marge visuelle. Glisser une carte réordonne ; glisser une
    // zone vide fait défiler.
    let board_area = Scroll::new()
        .axis(Axis::Both)
        .width(width)
        .flex(1.0)
        .child(Container::new().padding(24.0).child(board));
    let hint_bar = Container::new().width(width).padding(24.0).child(hint);
    let screen = column![NavBar::new("Kanban board").on_back(Msg::Pop), board_area, hint_bar]
        .width(width)
        .height(height);
    Box::new(Container::new().width(width).height(height).color(theme.background).child(screen))
}

/// Catégories (axe des abscisses) du tableau de bord graphique.
const CHART_CATS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
/// Séries du tableau de bord : `(nom, valeurs)`. Série 0 = accent du thème ; 1.. = `CHART_COLORS`.
const CHART_SERIES: [(&str, [f32; 5]); 3] = [
    ("Sales", [3.0, 7.0, 5.0, 8.0, 4.0]),
    ("Costs", [2.0, 4.0, 3.0, 5.0, 2.0]),
    ("Profit", [1.0, 3.0, 2.0, 3.0, 2.0]),
];
/// Couleurs des séries **additionnelles** (1.. ; la série 0 prend l'accent du thème).
const CHART_COLORS: [Color; 2] = [
    Color { r: 220.0 / 255.0, g: 120.0 / 255.0, b: 80.0 / 255.0, a: 1.0 },
    Color { r: 90.0 / 255.0, g: 158.0 / 255.0, b: 242.0 / 255.0, a: 1.0 },
];

/// Construit le graphique du tableau de bord selon `app.chart_kind` (jalon 219) : lignes (0), aires
/// empilées (1), barres groupées (2), barres empilées (3). Toutes les variantes partagent les
/// mêmes données, l'axe, et l'état de visibilité `chart_hidden`. `legend` câble (ou non) la légende
/// cliquable — utile pour un graphique **compagnon** qui ne réaffiche pas sa propre légende.
fn dashboard_chart(app: &TodoApp, kind: usize, height: f32, legend: bool) -> Box<dyn Widget<Msg>> {
    let hidden = app.chart_hidden.clone();
    let cats = (0..5).map(|i| (CHART_CATS[i], CHART_SERIES[0].1[i]));
    if kind < 2 {
        let mut c = LineChart::new(cats)
            .height(height)
            .grid(4)
            .name(CHART_SERIES[0].0)
            .series(CHART_SERIES[1].0, CHART_COLORS[0], CHART_SERIES[1].1)
            .series(CHART_SERIES[2].0, CHART_COLORS[1], CHART_SERIES[2].1)
            .hidden(hidden)
            .animated(true);
        if kind == 1 {
            c = c.stacked(true).normalized(app.chart_normalized);
        }
        if legend {
            // Le graphique principal : légende cliquable + points cliquables (jalon 221) + point
            // sélectionné mis en évidence (jalon 223).
            c = c
                .legend(true)
                .on_legend(Msg::ChartToggleSeries)
                .on_point(Msg::ChartPoint)
                .selected(app.chart_sel);
        }
        Box::new(c)
    } else {
        let mut c = BarChart::new(cats)
            .height(height)
            .grid(4)
            .name(CHART_SERIES[0].0)
            .series(CHART_SERIES[1].0, CHART_COLORS[0], CHART_SERIES[1].1)
            .series(CHART_SERIES[2].0, CHART_COLORS[1], CHART_SERIES[2].1)
            .hidden(hidden);
        if kind == 3 {
            c = c.stacked(true).normalized(app.chart_normalized);
        }
        if legend {
            // Le graphique principal : légende cliquable + barres cliquables (jalon 222) + barre
            // sélectionnée mise en évidence (jalon 223).
            c = c
                .legend(true)
                .on_legend(Msg::ChartToggleSeries)
                .on_point(Msg::ChartPoint)
                .selected(app.chart_sel);
        }
        Box::new(c)
    }
}

/// Écran **tableau de bord graphique** : un `SegmentedControl` choisit le type (lignes / aires
/// empilées / barres groupées / barres empilées, jalon 219), et la légende **cliquable** masque ou
/// affiche une série (jalon 215/218). Démontre le routage de clics de sous-région vers l'état.
fn charts_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let selector = SegmentedControl::new(app.chart_kind, Msg::SetChartKind)
        .segment("Lines")
        .segment("Stacked area")
        .segment("Grouped bars")
        .segment("Stacked bars");
    // Bascule **100 %** (jalon 224) : visible seulement pour les types empilés (aires/barres empilées),
    // où la normalisation a un sens.
    let stacked_kind = app.chart_kind == 1 || app.chart_kind == 3;
    let normalize_row: Box<dyn Widget<Msg>> = if stacked_kind {
        Box::new(
            row![
                text("100% stacking").size(13.0).color(theme.muted),
                Switch::new(app.chart_normalized).on_toggle(Msg::SetChartNormalized)
            ]
            .gap(10.0)
            .align(Align::Center),
        )
    } else {
        Box::new(Container::new().width(0.0).height(0.0))
    };
    let chart = dashboard_chart(app, app.chart_kind, 240.0, true);
    // Graphique **compagnon** : la famille complémentaire (barres si le principal est en lignes, et
    // inversement), sans sa propre légende — il partage `chart_hidden`, donc masquer une série via la
    // légende du principal la masque **aussi** ici (jalon 220).
    let companion_kind = if app.chart_kind < 2 { 2 } else { 0 };
    let companion = dashboard_chart(app, companion_kind, 150.0, false);
    let hint = text("Click a legend entry to toggle a series; click a point to pin it, or again to unpin.")
        .size(13.0)
        .color(theme.muted);
    // Détail épinglé du dernier point cliqué (jalon 221).
    let pinned: Box<dyn Widget<Msg>> = match &app.chart_pin {
        Some(detail) => Box::new(Chip::new(detail.clone())),
        None => Box::new(text("No point selected").size(13.0).color(theme.muted)),
    };
    let body = column![
        row![selector].align(Align::Center),
        normalize_row,
        chart,
        row![pinned].align(Align::Center),
        text("Companion view").size(13.0).color(theme.muted),
        companion,
        hint
    ]
    .gap(16.0)
    .padding(24.0);
    let screen = column![NavBar::new("Charts").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Box::new(Container::new().width(width).height(height).color(theme.background).child(screen))
}

/// Le formulaire de l'assistant : validation **pure** de l'état courant (jalons 180–181).
/// L'ordre déclare `password` avant `confirm` (validation croisée `matches`).
fn wizard_form(app: &TodoApp) -> Form {
    Form::new()
        .field("name", app.wizard_name.as_str(), Rule::required("Name is required"))
        .field(
            "email",
            app.wizard_email.as_str(),
            Rule::all([
                Rule::required("Email is required"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field(
            "password",
            app.wizard_pass.as_str(),
            Rule::min_len(8, "Password must be at least 8 characters"),
        )
        .matches("confirm", app.wizard_confirm.as_str(), "password", "Passwords do not match")
}

/// À quelle étape (0 = Account, 1 = Security) vit le champ `key` — pour que cliquer une puce du
/// récapitulatif d'erreurs saute à la bonne étape (jalons 181 + 183).
fn wizard_step_of(key: &str) -> usize {
    match key {
        "name" | "email" => 0,
        _ => 1,
    }
}

/// Index de champ (clé de focus) d'un champ de l'assistant — pour `keyed`/`Command::focus`.
fn wizard_field_of(key: &str) -> u8 {
    match key {
        "name" => 0,
        "email" => 1,
        "password" => 2,
        _ => 3,
    }
}

/// L'étape `step` est-elle **valide** ? (pour n'autoriser « Next » qu'une fois l'étape remplie.)
fn wizard_step_valid(form: &Form, step: usize) -> bool {
    match step {
        0 => form.error("name").is_none() && form.error("email").is_none(),
        1 => form.error("password").is_none() && form.error("confirm").is_none(),
        _ => form.is_valid(),
    }
}

/// Un champ de l'assistant : erreur affichée **qu'après** soumission, valeur **masquée** pour un
/// mot de passe, et **clé de focus** (`keyed`) pour que le récapitulatif puisse y sauter.
fn wizard_input(
    form: &Form,
    submitted: bool,
    label: &str,
    value: &str,
    key: &str,
    field: u8,
    obscure: bool,
    eye: Option<bool>,
) -> impl Widget<Msg> + 'static {
    let mut input = TextInput::new(value)
        .width(360.0)
        .size(16.0)
        .label(label)
        .obscure(obscure)
        .on_input(move |s| Msg::WizardInput(field, s));
    // `eye = Some(revealed)` : une icône œil **dans le champ** bascule le masquage (jalon 198).
    if let Some(revealed) = eye {
        let icon = if revealed { IconName::EyeOff } else { IconName::Eye };
        input = input.suffix_icon(icon).on_suffix(Msg::WizardToggleReveal);
    }
    if submitted {
        if let Some(err) = form.error(key) {
            input = input.error(err);
        }
    }
    keyed(("wizard", field), input)
}

/// Écran **assistant d'inscription** : preuve d'intégration des briques récentes —
/// indicateur [`Steps`] cliquable (jalon 183), formulaire validé [`Form`] (180) avec
/// récapitulatif d'erreurs **cliquable** (181), et notification de succès (185/188).
fn wizard_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let form = wizard_form(app);
    let submitted = app.wizard_submitted;

    // Les étapes sont marquées « terminées » par **validité** (jalon 195), pas seulement par
    // position — cohérent avec « Next » gardé par la même validité.
    let steps = Steps::new(["Account", "Security", "Review"])
        .current(app.wizard_step)
        .completed([
            wizard_step_valid(&form, 0),
            wizard_step_valid(&form, 1),
            form.is_valid(),
        ])
        .on_tap(Msg::WizardStep);

    // Contenu de l'étape courante.
    let content: Box<dyn Widget<Msg>> = match app.wizard_step {
        0 => Box::new(
            Flex::column()
                .gap(14.0)
                .child(wizard_input(&form, submitted, "Full name", &app.wizard_name, "name", 0, false, None))
                .child(wizard_input(&form, submitted, "Email", &app.wizard_email, "email", 1, false, None)),
        ),
        1 => {
            // Mots de passe masqués sauf révélation : l'icône œil **dans le champ** bascule (198).
            let obscure = !app.wizard_reveal;
            let eye = Some(app.wizard_reveal);
            Box::new(
                Flex::column()
                    .gap(14.0)
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Password",
                        &app.wizard_pass,
                        "password",
                        2,
                        obscure,
                        eye,
                    ))
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Confirm password",
                        &app.wizard_confirm,
                        "confirm",
                        3,
                        obscure,
                        eye,
                    )),
            )
        }
        _ => {
            let mut review = Flex::column().gap(14.0);
            // Récapitulatif cliquable : chaque puce saute à l'étape du champ fautif **et**
            // le focalise (jalons 181 + 183 + focus programmatique).
            if submitted && !form.is_valid() {
                let links = form.errors().into_iter().map(|(key, message)| {
                    (message.to_string(), Msg::WizardFocus(wizard_step_of(key), wizard_field_of(key)))
                });
                review = review.child(ErrorSummary::links(links));
            }
            review = review.child(
                text(format!(
                    "Creating account for {} <{}>",
                    if app.wizard_name.is_empty() { "—" } else { app.wizard_name.as_str() },
                    if app.wizard_email.is_empty() { "—" } else { app.wizard_email.as_str() },
                ))
                .size(16.0),
            );
            Box::new(review)
        }
    };

    // Barre de navigation : Précédent / Suivant, ou Créer sur la dernière étape.
    let mut nav = Flex::row().gap(12.0);
    if app.wizard_step > 0 {
        nav = nav.child(button("Back", Msg::WizardBack).variant(Variant::Secondary).size(16.0));
    }
    if app.wizard_step < 2 {
        // « Next » n'est actif qu'une fois l'étape courante valide (jalon 191 : Button désactivé).
        nav = nav.child(
            button("Next", Msg::WizardNext)
                .variant(Variant::Primary)
                .size(16.0)
                .enabled(wizard_step_valid(&form, app.wizard_step)),
        );
    } else {
        nav = nav.child(
            button("Create account", Msg::WizardSubmit).variant(Variant::Primary).size(16.0),
        );
    }

    let body = column![steps, content, nav].gap(24.0).padding(24.0);
    let screen = column![NavBar::new("Sign-up wizard").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Box::new(Container::new().width(width).height(height).color(theme.background).child(screen))
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
            row![
                text("Weekdays only").size(16.0),
                spacer(),
                Switch::new(app.weekdays_only).on_toggle(Msg::SetWeekdaysOnly),
            ]
            .align(Align::Center)
            .gap(12.0),
            demo_calendar(app),
        ]
        .gap(14.0),
    );
    let total = app.todos.len();
    let done = app.todos.iter().filter(|t| t.done).count();
    // Largeur utile de l'onglet (viewport moins les paddings colonne/tab),
    // bornée : les vitrines s'adaptent au Compact au lieu de déborder.
    let inner_w = (width - 72.0).clamp(240.0, 480.0);
    let stats = Grid::new(3)
        .gap(10.0)
        .width(inner_w)
        .cell(stat_tile(theme, "Total", total))
        .cell(stat_tile(theme, "Active", total - done))
        .cell(stat_tile(theme, "Done", done));
    let facts = Table::new(2)
        .width(inner_w)
        .header(&["Metric", "Value"])
        .row(&["Widgets", "35"])
        .row(&["Milestones", "39"]);

    // Arbre de fichiers (déplié selon l'état).
    let open = |id: u64| app.expanded.contains(&id);
    // Chevron = (dé)plier ; corps de la ligne = sélectionner le nœud (jalon 246).
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
            Skeleton::new().width(inner_w),
            Skeleton::new().width(inner_w * 0.8).height(14.0),
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
    // Le contenu (calendrier, options avancées…) dépasse l'écran : il défile
    // sous la barre, qui reste épinglée.
    let body = Scroll::new().width(width).flex(1.0).child(content);
    let screen = column![NavBar::new("Settings").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}

/// Une ligne de tâche : case à cocher, libellé (grisé **et barré** si terminée)
/// et suppression.
fn todo_row(todo: &Todo, theme: &Theme) -> Container<Msg> {
    let id = todo.id;
    let label_color = if todo.done { theme.muted } else { theme.on_surface };
    let mut label = text(todo.text.clone()).size(18.0).color(label_color);
    if todo.done {
        label = label.strikethrough();
    }
    let line = row![
        Avatar::new(todo.text.clone()).size(30.0),
        Checkbox::new(todo.done).on_toggle(move |_| Msg::ToggleTodo(id)),
        label,
        spacer(),
        button("×", Msg::DeleteTodo(id)).variant(Variant::Danger).size(15.0),
    ]
    .align(Align::Center)
    .gap(12.0);
    Container::new()
        // Appui long sur la ligne = suppression (le motif mobile, en plus du ×).
        .on_long_press(Msg::DeleteTodo(id))
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(line)
}

/// Contenu de la modale de confirmation de suppression groupée du tableau de données (jalon 245).
fn data_confirm_content(count: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Delete selected rows?").size(22.0).weight(FontWeight::Medium),
            text(format!("{count} row(s) will be removed.")).size(16.0),
            row![
                button("Cancel", Msg::DataCancelDelete).variant(Variant::Secondary),
                button("Delete", Msg::DataDeleteChecked).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// Contenu de la modale de confirmation d'effacement des terminées.
fn confirm_content(done: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Clear completed tasks?").size(22.0).weight(FontWeight::Medium),
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
fn todo_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let active = active_count(app);
    let done = done_count(app);

    // Responsivité : la carte s'élargit avec la fenêtre par paliers (Lot A). En
    // Compact, elle suit la largeur disponible ; les champs internes s'y adaptent.
    let class = SizeClass::from_width(width);
    // Largeur **interne** de la carte : la fenêtre moins le padding du corps
    // (24 × 2) et celui de la carte (20 × 2) — sinon la carte déborde du
    // viewport Compact et tout l'écran défile horizontalement.
    let card_width = match class {
        SizeClass::Compact => (width - 88.0).max(240.0),
        SizeClass::Medium => 560.0,
        SizeClass::Expanded => 680.0,
    };

    // En-tête : une AppBar adaptative. On déclare un titre + des actions ; elle
    // décide seule combien tiennent en ligne et replie le reste dans un menu
    // overflow « ⋯ », selon la largeur — sans jamais brancher sur mobile/desktop.
    let theme_label = if app.light { "Dark" } else { "Light" };
    let timer_label = if app.running { "Pause" } else { "Resume" };
    // Le titre suit la section active (comme le ferait une vraie app) — la
    // section Tasks est localisée (Fluent) pour la démo i18n.
    let section_title = match app.section {
        1 => "Stats".to_string(),
        2 => "About".to_string(),
        _ => tr(app.lang, "app-title"),
    };
    let header = AppBar::new(section_title)
        .width(width)
        .leading(button("☰", Msg::ToggleDrawer).variant(Variant::Secondary).size(16.0))
        .overflow(app.actions_open, Msg::ToggleActions)
        .action(timer_label, Msg::ToggleTimer)
        .action(theme_label, Msg::ToggleTheme)
        .action(seed_label(app), Msg::CycleSeed)
        .action(if app.rtl { "LTR" } else { "RTL" }, Msg::ToggleRtl)
        // Bascule de langue : l'étiquette montre la LANGUE VERS LAQUELLE on va.
        .action(LANGS[(app.lang + 1) % LANGS.len()].0, Msg::CycleLang)
        .action("A+", Msg::SetDensity(app.density + 0.1))
        .action("A−", Msg::SetDensity(app.density - 0.1))
        .action("Log →", Msg::Push(Route::Journal))
        .action("Settings →", Msg::Push(Route::Settings))
        .action("Quick actions", Msg::ToggleSheet)
        .action("Save", Msg::Save)
        .action("Clear completed", Msg::AskClearDone)
        .build();

    // Saisie : champ (Entrée valide) + bouton d'ajout. Le champ non vide porte une icône
    // suffixe **cliquable** « ✕ » qui l'efface (jalon 198 : clic positionnel du suffixe).
    let mut draft_input = TextInput::new(app.draft.as_str())
        .width((card_width - 150.0).max(160.0))
        .size(18.0)
        .on_input(Msg::DraftChanged)
        .on_submit(Msg::AddTodo);
    if !app.draft.is_empty() {
        draft_input = draft_input.suffix_icon(IconName::Close).on_suffix(Msg::ClearDraft);
    }
    let input_row = row![draft_input, button("Add", Msg::AddTodo)].align(Align::Center).gap(10.0);

    // Filtres : un contrôle segmenté (sélection unique).
    let segmented = SegmentedControl::new(filter_index(app.filter), |i| {
        Msg::SetFilter(filter_from_index(i))
    })
    .segment(tr(app.lang, "filter-all"))
    .segment(tr(app.lang, "filter-active"))
    .segment(tr(app.lang, "filter-done"));
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
        list = column![text("Nothing to show for this filter.").size(18.0).italic().color(theme.muted)];
    }
    // Responsivité **verticale** (Lot C) : en fenêtre courte, l'astuce est masquée
    // pour préserver la hauteur utile. Le défilement est assuré par le Scaffold.
    let short = SizeClass::from_height(height) == SizeClass::Compact;

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
    // (compteurs pluralisés, localisés Fluent) quand il y a de la place, court
    // quand c'est étroit — hauteur fixe.
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
        button("Load", Msg::Load).variant(Variant::Secondary).size(15.0),
        button("Save", Msg::Save).variant(Variant::Secondary).size(15.0),
        clear,
    ]
    .align(Align::Center)
    .gap(8.0);

    // Barre de progression de complétion (terminées / total).
    let progress = ProgressBar::new(done as f32 / total as f32).width((card_width - 40.0).max(200.0));

    // Carte de l'app, largeur responsive (Lot A), centrée en haut de l'écran.
    // Le corps est bâti de façon incrémentale pour omettre l'astuce si court.
    let mut card_body = Flex::column().width(card_width).gap(16.0);
    if !short {
        // Bannière **statique** : frontière de repaint (jalon 88). Elle est
        // rejouée depuis le cache aux frames d'interaction pure (survol, focus,
        // défilement ailleurs) au lieu d'être repeinte à chaque frame.
        card_body = card_body.child(
            Container::new().repaint_boundary().child(
                Alert::new("Press Enter to add a task; swipe from the left edge to go back.")
                    .title("Tip"),
            ),
        );
        // Rangée d'icônes vectorielles (jalon 89) + image bitmap (jalon 90) :
        // chemins tessellisés colorés au thème, et une texture GPU ajustée `Cover`.
        card_body = card_body.child(
            Flex::row()
                .gap(16.0)
                .align(Align::Center)
                .child(Icon::new(IconName::Check).color(theme.primary))
                .child(Icon::new(IconName::Star))
                .child(Icon::new(IconName::Heart))
                .child(Icon::new(IconName::Menu))
                .child(Icon::new(IconName::ChevronRight))
                .child(Image::new(demo_image(), 72.0, 48.0).fit(BoxFit::Cover))
                // Calque à opacité de groupe (jalon 92) : deux carrés qui se
                // chevauchent, composités d'un bloc → le chevauchement ne fonce
                // pas (pas de double-superposition de l'alpha).
                .child(CustomPaint::new(72.0, 48.0, |scene, bounds, theme| {
                    scene.layer(0.55, |inner| {
                        let c = theme.primary;
                        inner.fill_rect(Rect::new(bounds.x + 6.0, bounds.y + 8.0, 32.0, 32.0), c);
                        inner.fill_rect(Rect::new(bounds.x + 30.0, bounds.y + 8.0, 32.0, 32.0), c);
                    });
                })),
        );
    }
    // Identités **stables** (clés) : l'astuce ci-dessus est conditionnelle —
    // sans clés, sa disparition (clavier ouvert → écran court) décale les ids
    // positionnels des frères, et l'état retenu (focus du champ !) saute.
    card_body = card_body
        .child(keyed("draft-row", input_row))
        .child(keyed("filters", filters))
        .child(keyed("todo-list", list))
        .child(Divider::new())
        .child(progress)
        .child(footer);
    let card = Card::new().padding(20.0).child(card_body);
    let tasks_body = column![row![card].justify(Justify::Center)].padding(24.0);

    // Corps selon la section active (la navigation adaptative est dans le Scaffold).
    let section: Box<dyn Widget<Msg>> = match app.section {
        1 => Box::new(stats_section(app, theme, class)),
        2 => Box::new(about_section(theme, width)),
        _ => Box::new(tasks_body),
    };

    // Ossature d'écran : le Scaffold épingle la barre haute et la navigation, fait
    // défiler le corps, et coordonne tiroir / feuille / FAB — un seul point d'entrée.
    // Les insets sont déjà gérés par `view` (qui passe des dimensions sûres) ; le
    // Scaffold s'épingle donc simplement dans ce viewport.
    let scaffold = Scaffold::new(width, height)
        .background(theme.background)
        .app_bar(header)
        .body(section)
        .nav(app.section, Msg::SetSection)
        .destination("✔", "Tasks")
        .badge(active as u32)
        .destination("▦", "Stats")
        .destination("★", "About")
        .end_drawer(drawer_menu(app, theme, active), app.drawer_open, Msg::ToggleDrawer)
        .bottom_sheet(quick_actions_sheet(theme), app.sheet_open, Msg::ToggleSheet)
        .build();

    // La notification en tête de file flotte au-dessus de tout, ancrée en bas-centre par la
    // couche `ToastHost` (jalon 188) : fondu d'**entrée**, puis fondu de **sortie** quand elle
    // passe « en sortie » avant son retrait (jalon 193).
    match app.snackbars.current() {
        Some(message) => {
            let host = ToastHost::new(ToastPosition::BottomCenter)
                .toast(Toast::new(message.clone()).success());
            let host = if app.snackbars.is_leaving() {
                host.fade_out(0.3)
            } else {
                host.fade_in(0.25)
            };
            Box::new(Stack::new().width(width).height(height).layer(scaffold).layer(host))
        }
        None => scaffold,
    }
}

/// Contenu de la feuille modale : quelques actions rapides.
fn quick_actions_sheet(theme: &Theme) -> Container<Msg> {
    Container::new().padding(20.0).child(
        Flex::column()
            .gap(12.0)
            .child(text("Quick actions").size(20.0).color(theme.on_surface))
            .child(button("💾  Save", Msg::Save).variant(Variant::Primary).size(16.0))
            .child(
                button("🗑  Clear completed", Msg::AskClearDone)
                    .variant(Variant::Secondary)
                    .size(16.0),
            )
            .child(button("Close", Msg::ToggleSheet).variant(Variant::Secondary).size(16.0)),
    )
}

/// Contenu du tiroir de navigation : en-tête + destinations + réglages.
fn drawer_menu(app: &TodoApp, theme: &Theme, active: usize) -> Container<Msg> {
    let entry = |icon: &str, label: &str, index: usize| {
        let variant = if app.section == index { Variant::Primary } else { Variant::Secondary };
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
            text(format!("{active} task(s) pending")).size(14.0).color(theme.muted),
            button("Settings →", Msg::Push(Route::Settings)).variant(Variant::Secondary).size(15.0),
            button("Sign-up wizard →", Msg::Push(Route::Wizard))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Editable grid →", Msg::Push(Route::Grid))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Charts →", Msg::Push(Route::Charts))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Data table →", Msg::Push(Route::Data))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Kanban board →", Msg::Push(Route::Board))
                .variant(Variant::Secondary)
                .size(15.0),
        ]
        .gap(12.0),
    )
}

/// Jour de la semaine (0 = dimanche … 6 = samedi) d'une date (Sakamoto) — jalon 238.
fn weekday(y: i32, m: u32, d: u32) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    (((yy + yy / 4 - yy / 100 + yy / 400 + T[(m - 1) as usize] + d as i32) % 7 + 7) % 7) as u32
}

/// Vrai si la date tombe un **week-end** (samedi ou dimanche).
fn is_weekend(y: i32, m: u32, d: u32) -> bool {
    matches!(weekday(y, m, d), 0 | 6)
}

/// Le calendrier de la vitrine : `DatePicker::filtered` grisant les **week-ends** si `weekdays_only`
/// (jalon 238), sinon tous les jours cliquables (`DatePicker::new`).
fn demo_calendar(app: &TodoApp) -> Box<dyn Widget<Msg>> {
    if app.weekdays_only {
        Box::new(DatePicker::filtered(
            app.year,
            app.month,
            app.selected_day,
            |(y, m, d)| !is_weekend(y, m, d),
            Msg::PickDay,
            Msg::NavMonth,
        ))
    } else {
        Box::new(DatePicker::new(app.year, app.month, app.selected_day, Msg::PickDay, Msg::NavMonth))
    }
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
fn about_section(theme: &Theme, width: f32) -> Container<Msg> {
    // Largeur du contenu = viewport moins les paddings (conteneur 24×2 +
    // carte 20×2), bornée à une lecture confortable — sinon débordement
    // horizontal en Compact.
    let content_width = (width - 88.0).max(240.0).min(560.0);
    Container::new().padding(24.0).child(
        Card::new().padding(20.0).child(
            column![
                text("About frus").size(24.0),
                // Texte riche : styles mêlés en une ligne, héritage en cascade.
                RichText::new(
                    TextSpan::new("A ")
                        .child(TextSpan::new("fast").bold())
                        .child(TextSpan::new(", "))
                        .child(TextSpan::new("portable").italic().underline())
                        .child(TextSpan::new(" Rust UI framework — "))
                        .child(TextSpan::new("no GC").bold().color(theme.primary))
                        .child(TextSpan::new(".")),
                )
                .base_style(theme.text.body_medium.color(theme.muted))
                .wrap(),
                Divider::new(),
                Timeline::new()
                    .event("Responsive primitives", "Milestone 42")
                    .event("Adaptive navigation", "Milestone 43"),
                // Paragraphe : revient à la ligne à la largeur de la carte.
                text(
                    "Layout, painting, typography and animation are engine-level \
                     foundations shared by every widget in this gallery.",
                )
                .size(13.0)
                .color(theme.muted)
                .wrap(),
            ]
            .gap(12.0)
            .width(content_width),
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
    fn density_is_clamped() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::SetDensity(5.0));
        assert_eq!(app.density, 1.4);
        reduce(&mut app, Msg::SetDensity(0.1));
        assert_eq!(app.density, 0.8);
        // density() protège contre un état non initialisé (0.0 → 1.0).
        app.density = 0.0;
        assert_eq!(Application::density(&app), 1.0);
    }

    #[test]
    fn on_resize_tracks_class_and_closes_detail_when_compact() {
        let mut app = TodoApp::default();
        app.stat_detail_open = true;
        // Large : palier Expanded, le détail reste ouvert.
        app.on_resize(1000.0, 700.0);
        assert_eq!(app.size_class, Some(SizeClass::Expanded));
        assert!(app.stat_detail_open);
        // Étroit : bascule Compact et ferme le détail.
        app.on_resize(500.0, 700.0);
        assert_eq!(app.size_class, Some(SizeClass::Compact));
        assert!(!app.stat_detail_open);
    }

    #[test]
    fn drawer_toggles_and_section_choice_closes_it() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::ToggleDrawer);
        assert!(app.drawer_open);
        // Choisir une section referme le tiroir.
        reduce(&mut app, Msg::SetSection(1));
        assert_eq!(app.section, 1);
        assert!(!app.drawer_open);
        // Naviguer (Push) referme aussi le tiroir.
        reduce(&mut app, Msg::ToggleDrawer);
        reduce(&mut app, Msg::Push(Route::Settings));
        assert!(!app.drawer_open);
    }

    #[test]
    fn on_insets_updates_safe_area() {
        let mut app = TodoApp::default();
        assert_eq!(app.insets, Insets::ZERO);
        // Barres système seules.
        app.on_insets(WindowInsets {
            padding: Insets::new(84.0, 0.0, 45.0, 0.0),
            view_insets: Insets::ZERO,
        });
        assert_eq!(app.insets, Insets::new(84.0, 0.0, 45.0, 0.0));
        // Clavier ouvert : la zone sûre du bas suit le clavier (évitement).
        app.on_insets(WindowInsets {
            padding: Insets::new(84.0, 0.0, 45.0, 0.0),
            view_insets: Insets::new(0.0, 0.0, 345.0, 0.0),
        });
        assert_eq!(app.insets, Insets::new(84.0, 0.0, 345.0, 0.0));
        // La vue se construit sans paniquer avec des insets non nuls (chemin d'enrobage).
        let theme = Theme::dark();
        let tree = Application::view(&app, &theme, 400.0, 800.0);
        let ui = build_ui(tree.as_ref(), Size::new(400.0, 800.0), &Runtime::default(), &theme);
        assert!(!ui.scene().primitives().is_empty());
    }

    #[test]
    fn sheet_toggles_and_action_closes_it() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::ToggleSheet);
        assert!(app.sheet_open);
        // Une action de la feuille (Save) la referme.
        reduce(&mut app, Msg::Save);
        assert!(!app.sheet_open);
        // Idem pour « Clear completed » (qui ouvre la confirmation).
        reduce(&mut app, Msg::ToggleSheet);
        reduce(&mut app, Msg::AskClearDone);
        assert!(!app.sheet_open);
    }

    #[test]
    fn on_resize_tracks_orientation() {
        let mut app = TodoApp::default();
        app.on_resize(400.0, 800.0);
        assert_eq!(app.orientation, Some(Orientation::Portrait));
        app.on_resize(900.0, 500.0);
        assert_eq!(app.orientation, Some(Orientation::Landscape));
    }

    #[test]
    fn clear_draft_empties_the_field() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::DraftChanged("half-typed".to_string()));
        assert_eq!(app.draft, "half-typed");
        // Le suffixe « ✕ » (clic positionnel) émet ClearDraft, qui vide le champ.
        reduce(&mut app, Msg::ClearDraft);
        assert!(app.draft.is_empty());
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
    fn wizard_flow_validates_navigates_and_notifies() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Wizard));
        assert_eq!(current_route(&app), Route::Wizard);
        assert!(primitive_count(&app) > 0, "l'écran assistant se rend");

        // Étape Account invalide au départ (empêche « Next », jalon 191/192).
        assert!(!wizard_step_valid(&wizard_form(&app), 0), "Account invalide au départ");

        // Soumission vide → erreurs révélées, saut à l'étape Review (récapitulatif).
        reduce(&mut app, Msg::WizardSubmit);
        assert!(app.wizard_submitted);
        assert_eq!(app.wizard_step, 2);
        assert!(app.snackbars.is_empty());
        assert!(primitive_count(&app) > 0, "le récapitulatif d'erreurs se rend");

        // Une puce du récapitulatif saute à l'étape du champ **et** demande son focus.
        let cmd = reduce(&mut app, Msg::WizardFocus(0, 1));
        assert_eq!(app.wizard_step, 0);
        assert!(!cmd.is_empty(), "WizardFocus emet une demande de focus");

        // Remplir Account → l'étape devient valide.
        reduce(&mut app, Msg::WizardInput(0, "Ada".to_string()));
        reduce(&mut app, Msg::WizardInput(1, "ada@example.com".to_string()));
        assert!(wizard_step_valid(&wizard_form(&app), 0), "Account valide une fois rempli");
        // Remplir Security (mots de passe concordants).
        reduce(&mut app, Msg::WizardInput(2, "secret12".to_string()));
        reduce(&mut app, Msg::WizardInput(3, "secret12".to_string()));
        assert!(wizard_step_valid(&wizard_form(&app), 1), "Security valide");
        reduce(&mut app, Msg::WizardSubmit);
        // Succès : notification + assistant réinitialisé.
        assert_eq!(app.snackbars.current().map(String::as_str), Some("Account created"));
        assert_eq!(app.wizard_step, 0);
        assert!(!app.wizard_submitted);
        assert!(app.wizard_name.is_empty() && app.wizard_email.is_empty());

        // Navigation par étapes (Suivant / saut direct / Précédent, bornés).
        reduce(&mut app, Msg::WizardNext);
        assert_eq!(app.wizard_step, 1);
        reduce(&mut app, Msg::WizardStep(2));
        assert_eq!(app.wizard_step, 2);
        reduce(&mut app, Msg::WizardNext);
        assert_eq!(app.wizard_step, 2, "borné à la dernière étape");
        reduce(&mut app, Msg::WizardBack);
        assert_eq!(app.wizard_step, 1);
    }

    #[test]
    fn grid_edit_navigate_and_resize() {
        let mut app = TodoApp::default();
        app.grid = vec![
            vec!["Ada".to_string(), "Engineer".to_string(), "a@x.com".to_string()],
            vec!["Alan".to_string(), "Crypto".to_string(), "b@x.com".to_string()],
        ];
        reduce(&mut app, Msg::Push(Route::Grid));
        assert_eq!(current_route(&app), Route::Grid);
        assert!(primitive_count(&app) > 0, "la grille se rend");
        // Saisir dans une cellule met à jour la bonne case (grille toujours éditable).
        reduce(&mut app, Msg::GridInput(0, 1, "Mathematician".to_string()));
        assert_eq!(app.grid[0][1], "Mathematician");
        assert_eq!(app.grid[0][0], "Ada", "les autres cellules sont intactes");
        // Entrée descend d'une ligne (même colonne) et demande le focus.
        let cmd = reduce(&mut app, Msg::GridEnter(0, 1));
        assert!(!cmd.is_empty(), "Entrée focalise la cellule du dessous");
        // Entrée sur la dernière ligne en crée une (jalon 204) et y saute.
        let last = app.grid.len() - 1;
        let before_enter = app.grid.len();
        let cmd = reduce(&mut app, Msg::GridEnter(last, 1));
        assert_eq!(app.grid.len(), before_enter + 1, "Entrée sur la dernière ligne en crée une");
        assert!(!cmd.is_empty(), "et y place le focus");
        // Ajouter une ligne : une ligne vide en fin, focus sur sa 1re cellule.
        let before = app.grid.len();
        let cmd = reduce(&mut app, Msg::GridAddRow);
        assert_eq!(app.grid.len(), before + 1);
        assert_eq!(app.grid[before], vec!["", "", ""], "nouvelle ligne vide (3 colonnes)");
        assert!(!cmd.is_empty(), "AddRow focalise la nouvelle ligne");
        // Supprimer une ligne.
        reduce(&mut app, Msg::GridDeleteRow(0));
        assert_eq!(app.grid.len(), before, "une ligne retirée");
    }

    #[test]
    fn grid_sort_toggles_and_validates() {
        let mut app = TodoApp::default();
        app.grid = vec![
            vec!["Charlie".to_string(), "QA".to_string(), "c@x.com".to_string()],
            vec!["Ada".to_string(), "Engineer".to_string(), "a@x.com".to_string()],
            vec!["Bob".to_string(), "PM".to_string(), "b@x.com".to_string()],
        ];
        // Tri colonne 0 croissant, puis bascule décroissant.
        reduce(&mut app, Msg::GridSort(0));
        assert_eq!(app.grid_sort, Some((0, true)));
        assert_eq!(app.grid[0][0], "Ada");
        assert_eq!(app.grid[2][0], "Charlie");
        reduce(&mut app, Msg::GridSort(0));
        assert_eq!(app.grid_sort, Some((0, false)));
        assert_eq!(app.grid[0][0], "Charlie");
        // Validation par cellule : Name vide et email malformé sont signalés.
        assert_eq!(grid_cell_error(0, ""), Some("Required"));
        assert!(grid_cell_error(0, "Ada").is_none());
        assert_eq!(grid_cell_error(2, "not-an-email"), Some("Invalid email"));
        assert!(grid_cell_error(2, "a@x.com").is_none());
        assert!(grid_cell_error(2, "").is_none(), "email vide toléré (pas encore saisi)");
    }

    #[test]
    fn grid_save_is_gated_on_cell_errors() {
        let mut app = TodoApp::default();
        // Une ligne valide, une avec un Name vide et un email malformé (2 erreurs).
        app.grid = vec![
            vec!["Ada".to_string(), "Engineer".to_string(), "a@x.com".to_string()],
            vec!["".to_string(), "PM".to_string(), "nope".to_string()],
        ];
        assert_eq!(grid_error_count(&app.grid), 2);
        reduce(&mut app, Msg::GridSave);
        assert_eq!(
            app.snackbars.current().map(String::as_str),
            Some("Fix 2 errors before saving"),
            "la soumission est bloquée et compte les erreurs"
        );
        // Corrige les deux cellules : la grille devient valide, la sauvegarde aboutit.
        app.grid[1][0] = "Bob".to_string();
        app.grid[1][2] = "b@x.com".to_string();
        assert_eq!(grid_error_count(&app.grid), 0);
        let mut app2 = TodoApp { grid: app.grid.clone(), ..TodoApp::default() };
        reduce(&mut app2, Msg::GridSave);
        assert_eq!(app2.snackbars.current().map(String::as_str), Some("Grid saved"));
    }

    #[test]
    fn grid_focus_error_targets_the_first_faulty_cell() {
        let mut app = TodoApp::default();
        // Ligne 0 valide, ligne 1 : Name vide (colonne 0) = première faute attendue.
        app.grid = vec![
            vec!["Ada".to_string(), "Engineer".to_string(), "a@x.com".to_string()],
            vec!["".to_string(), "PM".to_string(), "nope".to_string()],
        ];
        assert_eq!(grid_first_error(&app.grid), Some((1, 0)));
        assert!(!reduce(&mut app, Msg::GridFocusError).is_empty(), "focalise la cellule fautive");
        // Tout valide : plus de cible, aucune commande.
        app.grid[1][0] = "Bob".to_string();
        app.grid[1][2] = "b@x.com".to_string();
        assert_eq!(grid_first_error(&app.grid), None);
        assert!(reduce(&mut app, Msg::GridFocusError).is_empty(), "rien à focaliser");
    }

    #[test]
    fn chart_legend_toggle_hides_and_shows_series() {
        let mut app = TodoApp::default();
        // L'écran graphique se rend.
        reduce(&mut app, Msg::Push(Route::Charts));
        assert_eq!(current_route(&app), Route::Charts);
        assert!(primitive_count(&app) > 0, "le tableau de bord se rend");
        // Clic de légende : masque la série, re-clic la ré-affiche (bascule).
        assert!(app.chart_hidden.is_empty());
        reduce(&mut app, Msg::ChartToggleSeries(1));
        assert_eq!(app.chart_hidden, vec![1], "clic masque la série 1");
        reduce(&mut app, Msg::ChartToggleSeries(2));
        assert_eq!(app.chart_hidden, vec![1, 2]);
        reduce(&mut app, Msg::ChartToggleSeries(1));
        assert_eq!(app.chart_hidden, vec![2], "re-clic ré-affiche la série 1");
    }

    #[test]
    fn chart_kind_selector_switches_type_and_each_renders() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert_eq!(app.chart_kind, 0, "lignes par défaut");
        // Chaque type (aires empilées, barres groupées, barres empilées) se rend.
        for k in [1usize, 2, 3] {
            reduce(&mut app, Msg::SetChartKind(k));
            assert_eq!(app.chart_kind, k);
            assert!(primitive_count(&app) > 0, "le type {k} se rend");
        }
    }

    #[test]
    fn clicking_a_point_pins_its_detail() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert!(app.chart_pin.is_none(), "rien d'épinglé au départ");
        // Thu (index 3) de Sales (série 0) = 8.
        reduce(&mut app, Msg::ChartPoint(3, 0));
        assert_eq!(app.chart_pin.as_deref(), Some("Sales · Thu = 8"));
        // Tue (index 1) de Costs (série 1) = 4 : remplace l'épingle.
        reduce(&mut app, Msg::ChartPoint(1, 1));
        assert_eq!(app.chart_pin.as_deref(), Some("Costs · Tue = 4"));
        assert!(primitive_count(&app) > 0, "l'écran avec épingle se rend");
    }

    #[test]
    fn tree_node_selection_toggles() {
        // Arbre de fichiers (vitrine Settings) : sélectionner un nœud, puis re-cliquer désélectionne.
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Settings));
        assert_eq!(app.tree_selected, None, "aucun noeud selectionne au depart");
        reduce(&mut app, Msg::SelectNode(6));
        assert_eq!(app.tree_selected, Some(6), "clic = noeud selectionne");
        reduce(&mut app, Msg::SelectNode(1));
        assert_eq!(app.tree_selected, Some(1), "clic ailleurs = deplace la selection");
        reduce(&mut app, Msg::SelectNode(1));
        assert_eq!(app.tree_selected, None, "re-clic = deselection");
        assert!(primitive_count(&app) > 0, "la vitrine se rend avec selection");
    }

    #[test]
    fn kanban_move_relocates_a_card() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Board));
        assert_eq!(current_route(&app), Route::Board);
        let start = app.kanban_cols();
        // "Design API" est en (0,0). Le deplacer en tete de "Doing" (colonne 1) : il quitte la 0 et
        // apparait en tete de la 1.
        reduce(&mut app, Msg::KanbanMove(0, 0, 1, 0));
        let after = app.kanban_cols();
        assert_eq!(after[0].len(), start[0].len() - 1, "la carte quitte la colonne source");
        assert_eq!(after[1][0], "Design API", "la carte arrive en tete de la colonne cible");
        assert!(!after[0].contains(&"Design API".to_string()), "plus dans la source");
        // Deplacement dans la MEME colonne : (1,0) vers la fin — le decalage d'index est gere.
        let doing_len = after[1].len();
        reduce(&mut app, Msg::KanbanMove(1, 0, 1, doing_len));
        let end = app.kanban_cols();
        assert_eq!(end[1].len(), doing_len, "meme nombre de cartes (reordonne, pas duplique)");
        assert_eq!(end[1].last().unwrap(), "Design API", "carte deplacee en fin de colonne");
        assert!(primitive_count(&app) > 0, "le tableau se rend");
        // Ajout / suppression (jalon 249) : + Add card ajoute au bas ; × supprime la carte visee.
        let col0_len = app.kanban_cols()[0].len();
        reduce(&mut app, Msg::KanbanAdd(0));
        assert_eq!(app.kanban_cols()[0].len(), col0_len + 1, "Add ajoute une carte");
        assert_eq!(app.kanban_cols()[0].last().unwrap(), "New card", "ajoutee au bas de la colonne");
        reduce(&mut app, Msg::KanbanDelete(0, 0));
        assert_eq!(app.kanban_cols()[0].len(), col0_len, "Delete retire une carte");
        assert!(primitive_count(&app) > 0, "se rend apres ajout/suppression");
    }

    #[test]
    fn grouped_bars_are_clickable_in_dashboard() {
        // Le graphique principal en **barres groupées** (kind 2) câble `on_point` (jalon 222) : au
        // moins un point de la zone de tracé émet `ChartPoint`. Balayage (indépendant des
        // constantes de géométrie internes de frus-widgets).
        let app = TodoApp::default();
        let chart = dashboard_chart(&app, 2, 240.0, true);
        let (w, h) = (600.0, 240.0);
        let mut hit = false;
        let mut y = 30.0;
        while y < h - 22.0 && !hit {
            let mut x = 40.0;
            while x < w {
                if let Some(Msg::ChartPoint(_, _)) = chart.positional_click(x, y, w, h) {
                    hit = true;
                    break;
                }
                x += 4.0;
            }
            y += 4.0;
        }
        assert!(hit, "une barre du tableau de bord emet ChartPoint");
    }

    #[test]
    fn clicking_a_point_marks_it_selected() {
        // Le clic épingle non seulement le détail (jalon 221) mais aussi la sélection `(cat, série)`
        // mise en évidence dans le graphique (jalon 223).
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert_eq!(app.chart_sel, None, "rien de selectionne au depart");
        reduce(&mut app, Msg::ChartPoint(3, 0));
        assert_eq!(app.chart_sel, Some((3, 0)), "le point cliqué devient la selection");
        reduce(&mut app, Msg::ChartPoint(1, 1));
        assert_eq!(app.chart_sel, Some((1, 1)), "la selection suit le dernier clic");
        assert!(primitive_count(&app) > 0, "l'ecran avec point mis en evidence se rend");
    }

    #[test]
    fn normalized_toggle_applies_to_stacked_kinds() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert!(!app.chart_normalized, "absolu par defaut");
        reduce(&mut app, Msg::SetChartNormalized(true));
        assert!(app.chart_normalized, "bascule activee");
        // Les deux types empilés (aires empilées kind 1, barres empilées kind 3) se rendent en 100 %.
        for k in [1usize, 3] {
            reduce(&mut app, Msg::SetChartKind(k));
            assert!(primitive_count(&app) > 0, "le type empile {k} se rend en 100%");
        }
        reduce(&mut app, Msg::SetChartNormalized(false));
        assert!(!app.chart_normalized, "bascule desactivee");
    }

    #[test]
    fn re_clicking_a_selected_point_unpins_it() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        reduce(&mut app, Msg::ChartPoint(2, 1));
        assert_eq!(app.chart_sel, Some((2, 1)), "premier clic epingle");
        assert!(app.chart_pin.is_some(), "detail epingle");
        // Re-clic sur le même point : désépingle (sélection et détail effacés).
        reduce(&mut app, Msg::ChartPoint(2, 1));
        assert_eq!(app.chart_sel, None, "re-clic desepingle");
        assert!(app.chart_pin.is_none(), "detail efface");
        // Un autre point ré-épingle normalement.
        reduce(&mut app, Msg::ChartPoint(0, 0));
        assert_eq!(app.chart_sel, Some((0, 0)), "un autre point re-epingle");
    }

    #[test]
    fn data_table_screen_sorts_and_paginates_without_touching_data() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Data));
        assert_eq!(current_route(&app), Route::Data);
        assert!(primitive_count(&app) > 0, "l'ecran data se rend");
        // Tri : premier clic croissant, re-clic décroissant ; le tri ramène à la page 1.
        reduce(&mut app, Msg::DataPage(2));
        assert_eq!(app.data_page, 2);
        reduce(&mut app, Msg::DataSort(2));
        assert_eq!(app.data_sort, Some((2, true)), "premier clic = croissant");
        assert_eq!(app.data_page, 1, "le tri ramene a la page 1");
        reduce(&mut app, Msg::DataSort(2));
        assert_eq!(app.data_sort, Some((2, false)), "re-clic = decroissant");
        // Taille de page : change et ramène à la page 1.
        reduce(&mut app, Msg::DataPage(2));
        reduce(&mut app, Msg::DataPageSize(10));
        assert_eq!(app.data_page_size, 10);
        assert_eq!(app.data_page, 1, "changer la taille ramene a la page 1");
        assert!(primitive_count(&app) > 0, "se rend apres tri/pagination");
        // Sélection de ligne : un clic sélectionne la ligne **source**, re-clic désélectionne.
        assert_eq!(app.data_selected, None, "aucune ligne au depart");
        reduce(&mut app, Msg::DataSelectRow(3));
        assert_eq!(app.data_selected, Some(3), "clic = ligne source selectionnee");
        reduce(&mut app, Msg::DataSelectRow(7));
        assert_eq!(app.data_selected, Some(7), "clic ailleurs = deplace la selection");
        reduce(&mut app, Msg::DataSelectRow(7));
        assert_eq!(app.data_selected, None, "re-clic = deselection");
        assert!(primitive_count(&app) > 0, "se rend avec detail de ligne");
        // Tri personnalisé de la colonne Level (index 3) : ordre semantique, pas alphabetique.
        assert_eq!(level_rank("Low"), 0);
        assert!(level_rank("Low") < level_rank("Medium") && level_rank("Medium") < level_rank("High"));
        reduce(&mut app, Msg::DataSort(3));
        assert_eq!(app.data_sort, Some((3, true)), "tri de la colonne Level");
        assert!(primitive_count(&app) > 0, "se rend trie par priorite");
        // Sélection multiple (cases) : toggle d'une ligne, puis « tout cocher »/« tout décocher ».
        assert!(app.data_checked.is_empty(), "rien de coche au depart");
        reduce(&mut app, Msg::DataCheck(2));
        assert_eq!(app.data_checked, vec![2], "coche la ligne source 2");
        reduce(&mut app, Msg::DataCheck(2));
        assert!(app.data_checked.is_empty(), "re-check = decoche");
        reduce(&mut app, Msg::DataCheckAll);
        assert_eq!(app.data_checked.len(), DATA_PEOPLE.len(), "tout cocher = 12 lignes");
        reduce(&mut app, Msg::DataCheckAll);
        assert!(app.data_checked.is_empty(), "re-tout-cocher = tout decocher");
        assert!(primitive_count(&app) > 0, "se rend avec cases a cocher");
        // Recherche : la frappe met a jour le filtre et ramene a la page 1.
        reduce(&mut app, Msg::DataPage(2));
        reduce(&mut app, Msg::DataSearch("ada".to_string()));
        assert_eq!(app.data_query, "ada", "le filtre est mis a jour");
        assert_eq!(app.data_page, 1, "un nouveau filtre ramene a la page 1");
        assert!(primitive_count(&app) > 0, "se rend filtre");
        // Actions groupees (jalon 243) : Clear vide la selection, Delete supprime les lignes cochees.
        reduce(&mut app, Msg::DataCheck(0));
        reduce(&mut app, Msg::DataCheck(1));
        reduce(&mut app, Msg::DataSelectRow(1));
        assert_eq!(app.data_checked.len(), 2);
        reduce(&mut app, Msg::DataClearChecked);
        assert!(app.data_checked.is_empty(), "Clear vide la selection");
        assert_eq!(app.data_selected, Some(1), "Clear ne touche pas le focus");
        let before = app.data_rows().len();
        reduce(&mut app, Msg::DataCheck(0));
        // Confirmation (jalon 245) : Delete ouvre la modale ; Cancel ferme sans supprimer.
        reduce(&mut app, Msg::DataAskDelete);
        assert!(app.data_confirm_delete, "Delete ouvre la confirmation");
        assert_eq!(app.data_rows().len(), before, "rien de supprime tant qu'on n'a pas confirme");
        assert!(primitive_count(&app) > 0, "se rend avec la modale");
        reduce(&mut app, Msg::DataCancelDelete);
        assert!(!app.data_confirm_delete, "Cancel ferme la modale");
        assert_eq!(app.data_rows().len(), before, "Cancel ne supprime rien");
        // Confirmer supprime reellement et ferme la modale.
        reduce(&mut app, Msg::DataAskDelete);
        reduce(&mut app, Msg::DataDeleteChecked);
        assert!(!app.data_confirm_delete, "confirmer ferme la modale");
        assert_eq!(app.data_rows().len(), before - 1, "Delete confirme retire la ligne cochee");
        assert!(app.data_checked.is_empty(), "Delete vide la selection");
        assert_eq!(app.data_selected, None, "Delete remet le focus a zero");
        assert!(primitive_count(&app) > 0, "se rend apres suppression");
        // Etat vide (jalon 244) : un filtre sans resultat se rend quand meme (en-tete + message).
        reduce(&mut app, Msg::DataSearch("zzzzz".to_string()));
        assert!(primitive_count(&app) > 0, "se rend avec l'etat vide");
    }

    #[test]
    fn calendar_weekdays_only_filters_weekends() {
        // Juillet 2026 : 4-5 = samedi/dimanche ; 6 = lundi.
        assert!(is_weekend(2026, 7, 4) && is_weekend(2026, 7, 5), "4-5 juillet = week-end");
        assert!(!is_weekend(2026, 7, 6), "6 juillet = lundi (ouvre)");
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Settings));
        assert!(!app.weekdays_only, "tous les jours par defaut");
        assert!(primitive_count(&app) > 0, "vitrine se rend");
        // Bascule : le calendrier filtré se rend, puis revient.
        reduce(&mut app, Msg::SetWeekdaysOnly(true));
        assert!(app.weekdays_only);
        assert!(primitive_count(&app) > 0, "calendrier filtre se rend");
        reduce(&mut app, Msg::SetWeekdaysOnly(false));
        assert!(!app.weekdays_only);
    }

    #[test]
    fn companion_chart_renders_across_families_with_hidden() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        // Une série masquée est partagée par le principal et le compagnon (même `chart_hidden`).
        reduce(&mut app, Msg::ChartToggleSeries(1));
        assert_eq!(app.chart_hidden, vec![1]);
        // Lignes (compagnon = barres) puis barres (compagnon = lignes) : l'écran se rend des deux.
        reduce(&mut app, Msg::SetChartKind(0));
        assert!(primitive_count(&app) > 0, "principal lignes + compagnon barres");
        reduce(&mut app, Msg::SetChartKind(2));
        assert!(primitive_count(&app) > 0, "principal barres + compagnon lignes");
    }

    #[test]
    fn grid_next_error_cycles_through_all_faults() {
        let mut app = TodoApp::default();
        // Fautes attendues, en ordre : (0,0) Name vide, (0,2) email invalide, (1,2) email invalide.
        app.grid = vec![
            vec!["".to_string(), "PM".to_string(), "nope".to_string()],
            vec!["Ada".to_string(), "Engineer".to_string(), "bad".to_string()],
        ];
        assert_eq!(grid_faults(&app.grid), vec![(0, 0), (0, 2), (1, 2)]);
        // Chaque appel avance ; le dernier boucle sur la première.
        for expected in [(0, 0), (0, 2), (1, 2), (0, 0)] {
            reduce(&mut app, Msg::GridFocusError);
            assert_eq!(app.grid_error_cursor, Some(expected));
        }
    }

    #[test]
    fn snackbar_queue_orders_and_exits() {
        let mut app = TodoApp::default();
        // Deux notifications empilées : la 1re est visible, la 2e attend.
        show_toast(&mut app, "A");
        show_toast(&mut app, "B");
        assert_eq!(app.snackbars.current().map(String::as_str), Some("A"));
        assert!(!app.snackbars.is_leaving(), "affichée, pas encore en sortie");
        // Expiration → la tête passe **en sortie** (fondu) sans disparaître.
        reduce(&mut app, Msg::ToastExpire);
        assert!(app.snackbars.is_leaving());
        assert_eq!(app.snackbars.current().map(String::as_str), Some("A"));
        // Retrait → la suivante prend le relais (fondu d'entrée).
        reduce(&mut app, Msg::DismissToast);
        assert_eq!(app.snackbars.current().map(String::as_str), Some("B"));
        assert!(!app.snackbars.is_leaving());
        // Dernière : sortie puis retrait → file vide.
        reduce(&mut app, Msg::ToastExpire);
        reduce(&mut app, Msg::DismissToast);
        assert!(app.snackbars.is_empty());
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

    /// Live-reload : l'instantané `save_state` réhydrate un binaire neuf —
    /// tâches, brouillon, filtre, thème (mode + graine), écran empilé.
    #[test]
    fn live_reload_state_round_trips() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::DraftChanged("Buy milk".to_string()));
        reduce(&mut app, Msg::AddTodo);
        let id = app.todos[0].id;
        reduce(&mut app, Msg::ToggleTodo(id));
        reduce(&mut app, Msg::DraftChanged("half-typed".to_string()));
        reduce(&mut app, Msg::SetFilter(Filter::Done));
        reduce(&mut app, Msg::ToggleTheme);
        reduce(&mut app, Msg::CycleSeed);
        reduce(&mut app, Msg::Push(Route::Settings));

        let snapshot = Application::save_state(&app).expect("un instantané");

        let mut fresh = TodoApp::default();
        Application::restore_state(&mut fresh, &snapshot);
        assert_eq!(fresh.todos.len(), 1);
        assert_eq!(fresh.todos[0].text, "Buy milk");
        assert!(fresh.todos[0].done);
        assert_eq!(fresh.draft, "half-typed");
        assert!(fresh.filter == Filter::Done);
        assert_eq!(fresh.light, app.light);
        assert_eq!(fresh.seed_index, 1);
        assert_eq!(current_route(&fresh), Route::Settings);
        // `init` après réhydratation ne relance PAS le chargement disque
        // (l'instantané fait foi) : aucun effet émis.
        assert!(fresh.init().is_empty(), "pas de Loaded après réhydratation");
        // Un instantané corrompu / d'une autre version est ignoré sans paniquer.
        let mut other = TodoApp::default();
        Application::restore_state(&mut other, b"garbage \xFF");
        Application::restore_state(&mut other, b"frus-demo-state v999\nlight 1\n");
        assert!(other.todos.is_empty() && !other.restored);
    }
}
