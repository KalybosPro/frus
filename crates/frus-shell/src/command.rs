//! [`Command`] : les **effets** renvoyés par `Application::update`.
//!
//! Une commande décrit un travail à effectuer **hors** du cycle `update` — une
//! écriture fichier, un chargement, un calcul long… — dont le résultat revient
//! sous forme de **message** réinjecté dans `update`. Le framework exécute chaque
//! tâche sur un thread de fond et renvoie son message via la boucle d'événements.

/// Une tâche : un travail qui produit éventuellement un message en retour.
type Task<Msg> = Box<dyn FnOnce() -> Option<Msg> + Send + 'static>;

/// Un lot d'effets à exécuter (éventuellement vide) : des **tâches** de fond et/ou
/// des demandes de **focus** (par clé de widget).
pub struct Command<Msg> {
    tasks: Vec<Task<Msg>>,
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
        Self { tasks: Vec::new(), focus: Vec::new() }
    }

    /// Regroupe plusieurs commandes en une seule.
    pub fn batch(commands: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let mut tasks = Vec::new();
        let mut focus = Vec::new();
        for command in commands {
            tasks.extend(command.tasks);
            focus.extend(command.focus);
        }
        Self { tasks, focus }
    }

    /// Exécute une tâche en arrière-plan ; son résultat devient un message.
    pub fn perform(task: impl FnOnce() -> Msg + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(move || Some(task()))],
            focus: Vec::new(),
        }
    }

    /// Exécute un effet de bord ; il peut renvoyer un message (`None` = aucun).
    pub fn run(task: impl FnOnce() -> Option<Msg> + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(task)],
            focus: Vec::new(),
        }
    }

    /// Demande le **focus** du widget portant la clé `key` (le champ enveloppé par
    /// `keyed(key, …)`). Résolu par le shell après la prochaine construction de la
    /// vue — typiquement renvoyé quand une soumission de formulaire échoue, pour
    /// sauter au premier champ invalide (`Form::first_invalid`).
    pub fn focus(key: impl std::hash::Hash) -> Self {
        Self { tasks: Vec::new(), focus: vec![focus_key(key)] }
    }

    /// `true` si la commande n'a ni effet ni demande de focus.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty() && self.focus.is_empty()
    }

    /// Extrait tâches et demandes de focus (pour exécution par le framework).
    pub(crate) fn into_parts(self) -> (Vec<Task<Msg>>, Vec<u64>) {
        (self.tasks, self.focus)
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
        let (tasks, _) = command.into_parts();
        assert_eq!(tasks.len(), 1);
        let produced = (tasks.into_iter().next().unwrap())();
        assert_eq!(produced, Some(42));
    }

    #[test]
    fn run_may_produce_nothing() {
        let command = Command::run(|| -> Option<u32> { None });
        let (tasks, _) = command.into_parts();
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
        let (tasks, focus) = f.into_parts();
        assert!(tasks.is_empty());
        assert_eq!(focus, vec![focus_key("email")]);
    }
}
