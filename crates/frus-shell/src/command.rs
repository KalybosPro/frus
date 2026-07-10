//! [`Command`] : les **effets** renvoyés par `Application::update`.
//!
//! Une commande décrit un travail à effectuer **hors** du cycle `update` — une
//! écriture fichier, un chargement, un calcul long… — dont le résultat revient
//! sous forme de **message** réinjecté dans `update`. Le framework exécute chaque
//! tâche sur un thread de fond et renvoie son message via la boucle d'événements.

/// Une tâche : un travail qui produit éventuellement un message en retour.
type Task<Msg> = Box<dyn FnOnce() -> Option<Msg> + Send + 'static>;

/// Un lot d'effets à exécuter (éventuellement vide).
pub struct Command<Msg> {
    tasks: Vec<Task<Msg>>,
}

impl<Msg: Send + 'static> Command<Msg> {
    /// Aucun effet.
    pub fn none() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Regroupe plusieurs commandes en une seule.
    pub fn batch(commands: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let mut tasks = Vec::new();
        for command in commands {
            tasks.extend(command.tasks);
        }
        Self { tasks }
    }

    /// Exécute une tâche en arrière-plan ; son résultat devient un message.
    pub fn perform(task: impl FnOnce() -> Msg + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(move || Some(task()))],
        }
    }

    /// Exécute un effet de bord ; il peut renvoyer un message (`None` = aucun).
    pub fn run(task: impl FnOnce() -> Option<Msg> + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(task)],
        }
    }

    /// `true` si la commande n'a aucun effet.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Extrait les tâches (pour exécution par le framework).
    pub(crate) fn into_tasks(self) -> Vec<Task<Msg>> {
        self.tasks
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
        let tasks = command.into_tasks();
        assert_eq!(tasks.len(), 1);
        let produced = (tasks.into_iter().next().unwrap())();
        assert_eq!(produced, Some(42));
    }

    #[test]
    fn run_may_produce_nothing() {
        let command = Command::run(|| -> Option<u32> { None });
        let tasks = command.into_tasks();
        assert_eq!((tasks.into_iter().next().unwrap())(), None);
    }

    #[test]
    fn batch_flattens_and_drops_empties() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::none(),
            Command::run(|| Some(2u32)),
        ]);
        assert_eq!(command.into_tasks().len(), 2);
    }
}
