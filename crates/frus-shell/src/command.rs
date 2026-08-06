//! [`Command`] : les **effets** renvoyés par `Application::update`.
//!
//! Une commande décrit un travail à effectuer **hors** du cycle `update` — une
//! écriture fichier, un chargement, un `fetch` réseau, un calcul long… — dont le
//! résultat revient sous forme de **message** réinjecté dans `update`. Deux formes :
//!
//! - **synchrone** ([`Command::perform`] / [`Command::run`]) : une closure exécutée
//!   sur un thread de fond (natif) ou en microtâche (Web).
//! - **asynchrone** ([`Command::perform_async`] / [`Command::run_async`]) : une
//!   **future** qui peut `await`. Sur le **Web** (mono-thread), elle est pilotée par
//!   le navigateur (`spawn_local`) — un vrai `fetch` peut ainsi s'attendre. En
//!   **natif**, elle est menée à terme sur un thread (`block_on`).

use std::future::Future;
use std::pin::Pin;

/// Une tâche **synchrone** : un travail qui produit éventuellement un message.
type Task<Msg> = Box<dyn FnOnce() -> Option<Msg> + Send + 'static>;

/// Une tâche **asynchrone** : une future qui produit éventuellement un message.
///
/// En **natif**, elle traverse un thread (`block_on`) : bornée `Send`. Sur le **Web**
/// (mono-thread), les futures du navigateur (`JsFuture`/`fetch`) ne sont **pas** `Send`
/// et n'en ont pas besoin — d'où la borne relâchée.
#[cfg(not(web))]
type AsyncTask<Msg> = Pin<Box<dyn Future<Output = Option<Msg>> + Send + 'static>>;
#[cfg(web)]
type AsyncTask<Msg> = Pin<Box<dyn Future<Output = Option<Msg>> + 'static>>;

/// Un lot d'effets à exécuter (éventuellement vide) : des **tâches** de fond
/// (synchrones et/ou asynchrones) et/ou des demandes de **focus** (par clé de widget).
pub struct Command<Msg> {
    tasks: Vec<Task<Msg>>,
    async_tasks: Vec<AsyncTask<Msg>>,
    /// Clés de widgets à focaliser (hash de la clé, comme [`crate::Subscription`] et
    /// le `keyed(...)` des widgets). Le shell les résout après le prochain build.
    focus: Vec<u64>,
}

/// Hash d'une clé de focus — **identique** au hachage du `keyed(key, …)` des widgets,
/// pour que `Command::focus(k)` cible le widget `keyed(k, …)`.
fn focus_key(key: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

impl<Msg: Send + 'static> Command<Msg> {
    /// Aucun effet.
    pub fn none() -> Self {
        Self { tasks: Vec::new(), async_tasks: Vec::new(), focus: Vec::new() }
    }

    /// Regroupe plusieurs commandes en une seule.
    pub fn batch(commands: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let mut tasks = Vec::new();
        let mut async_tasks = Vec::new();
        let mut focus = Vec::new();
        for command in commands {
            tasks.extend(command.tasks);
            async_tasks.extend(command.async_tasks);
            focus.extend(command.focus);
        }
        Self { tasks, async_tasks, focus }
    }

    /// Exécute une tâche **synchrone** en arrière-plan ; son résultat devient un message.
    pub fn perform(task: impl FnOnce() -> Msg + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(move || Some(task()))],
            async_tasks: Vec::new(),
            focus: Vec::new(),
        }
    }

    /// Exécute un effet de bord **synchrone** ; il peut renvoyer un message (`None` = aucun).
    pub fn run(task: impl FnOnce() -> Option<Msg> + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(task)],
            async_tasks: Vec::new(),
            focus: Vec::new(),
        }
    }

    /// Exécute une **future** asynchrone ; sa valeur devient un message.
    ///
    /// Sur le **Web** (mono-thread), la future est pilotée par le navigateur
    /// (`spawn_local`) — un vrai `fetch` peut ainsi `await` sans bloquer. En **natif**,
    /// elle est menée à terme sur un thread de fond (`block_on`) : idéale pour une future
    /// **autonome** (calcul, canal, minuterie pilotée) ; une E/S réseau réelle demande le
    /// runtime async de l'application (voir la note plateforme du module).
    #[cfg(not(web))]
    pub fn perform_async<F>(future: F) -> Self
    where
        F: Future<Output = Msg> + Send + 'static,
    {
        Self {
            tasks: Vec::new(),
            async_tasks: vec![Box::pin(async move { Some(future.await) })],
            focus: Vec::new(),
        }
    }

    /// Exécute une **future** asynchrone ; sa valeur devient un message. Voir la version
    /// native pour la sémantique complète (bornes `Send` relâchées sur le Web).
    #[cfg(web)]
    pub fn perform_async<F>(future: F) -> Self
    where
        F: Future<Output = Msg> + 'static,
    {
        Self {
            tasks: Vec::new(),
            async_tasks: vec![Box::pin(async move { Some(future.await) })],
            focus: Vec::new(),
        }
    }

    /// Exécute une **future** asynchrone à effet de bord ; elle peut renvoyer un message
    /// (`None` = aucun). Pendant asynchrone de [`Command::run`].
    #[cfg(not(web))]
    pub fn run_async<F>(future: F) -> Self
    where
        F: Future<Output = Option<Msg>> + Send + 'static,
    {
        Self { tasks: Vec::new(), async_tasks: vec![Box::pin(future)], focus: Vec::new() }
    }

    /// Exécute une **future** asynchrone à effet de bord (`None` = aucun message). Version
    /// Web (borne `Send` relâchée).
    #[cfg(web)]
    pub fn run_async<F>(future: F) -> Self
    where
        F: Future<Output = Option<Msg>> + 'static,
    {
        Self { tasks: Vec::new(), async_tasks: vec![Box::pin(future)], focus: Vec::new() }
    }

    /// Demande le **focus** du widget portant la clé `key` (le champ enveloppé par
    /// `keyed(key, …)`). Résolu par le shell après la prochaine construction de la
    /// vue — typiquement renvoyé quand une soumission de formulaire échoue, pour
    /// sauter au premier champ invalide (`Form::first_invalid`).
    pub fn focus(key: impl std::hash::Hash) -> Self {
        Self { tasks: Vec::new(), async_tasks: Vec::new(), focus: vec![focus_key(key)] }
    }

    /// `true` si la commande n'a ni effet ni demande de focus.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty() && self.async_tasks.is_empty() && self.focus.is_empty()
    }

    /// Extrait tâches synchrones, tâches asynchrones et demandes de focus (pour
    /// exécution par le framework).
    pub(crate) fn into_parts(self) -> (Vec<Task<Msg>>, Vec<AsyncTask<Msg>>, Vec<u64>) {
        (self.tasks, self.async_tasks, self.focus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_empty() {
        assert!(Command::<u32>::none().is_empty());
    }

    #[test]
    fn perform_yields_a_message() {
        let command = Command::perform(|| 42u32);
        let (tasks, _, _) = command.into_parts();
        assert_eq!(tasks.len(), 1);
        let produced = (tasks.into_iter().next().unwrap())();
        assert_eq!(produced, Some(42));
    }

    #[test]
    fn run_may_produce_nothing() {
        let command = Command::run(|| -> Option<u32> { None });
        let (tasks, _, _) = command.into_parts();
        assert_eq!((tasks.into_iter().next().unwrap())(), None);
    }

    #[test]
    fn batch_flattens_and_drops_empties() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::none(),
            Command::run(|| Some(2u32)),
        ]);
        assert_eq!(command.into_parts().0.len(), 2);
    }

    #[test]
    fn focus_carries_a_key_and_no_task() {
        // Une commande de focus n'a pas de tâche mais n'est pas « vide ».
        let f = Command::<u32>::focus("email");
        assert!(!f.is_empty());
        let (tasks, asyncs, focus) = f.into_parts();
        assert!(tasks.is_empty());
        assert!(asyncs.is_empty());
        assert_eq!(focus, vec![focus_key("email")]);
    }

    #[cfg(not(web))]
    #[test]
    fn perform_async_yields_a_message() {
        // La future est menée à terme (natif : `block_on`) et sa valeur devient un message.
        let command = Command::perform_async(async { 7u32 });
        let (tasks, mut asyncs, _) = command.into_parts();
        assert!(tasks.is_empty());
        assert_eq!(asyncs.len(), 1);
        let produced = pollster::block_on(asyncs.remove(0));
        assert_eq!(produced, Some(7));
    }

    #[cfg(not(web))]
    #[test]
    fn run_async_may_produce_nothing() {
        let command = Command::run_async(async { None::<u32> });
        let (_, mut asyncs, _) = command.into_parts();
        assert_eq!(pollster::block_on(asyncs.remove(0)), None);
    }

    #[cfg(not(web))]
    #[test]
    fn batch_combines_sync_and_async_tasks() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::perform_async(async { 2u32 }),
        ]);
        let (tasks, asyncs, _) = command.into_parts();
        assert_eq!(tasks.len(), 1);
        assert_eq!(asyncs.len(), 1);
    }
}
