//! [`Subscription`] : les **sources continues** de messages déclarées par
//! l'application — le pendant « flux » de [`crate::Command`] (ponctuel).
//!
//! L'app déclare ses souscriptions **en fonction de son état** ; le framework les
//! **diffe** à chaque cycle (démarre les nouvelles, arrête celles retirées),
//! grâce à un **id** stable par souscription (hash de sa « recette »).

use std::time::Duration;

use web_time::Instant;

/// Fabrique de message d'un timer (appelée à chaque intervalle).
type TimerFn<Msg> = Box<dyn Fn(Instant) -> Msg + Send>;

/// La nature d'une souscription.
pub(crate) enum Kind<Msg> {
    /// Émet un message à intervalle régulier.
    Every {
        interval: Duration,
        make: TimerFn<Msg>,
    },
}

/// Une souscription unitaire : son id (pour le diff) et sa nature.
pub(crate) struct Entry<Msg> {
    pub(crate) id: u64,
    pub(crate) kind: Kind<Msg>,
}

/// Un ensemble de sources continues de messages (éventuellement vide).
pub struct Subscription<Msg> {
    entries: Vec<Entry<Msg>>,
}

/// Id stable d'un `every` : hash de sa recette (type + durée en ms).
fn every_id(interval: Duration) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    "every".hash(&mut hasher);
    interval.as_millis().hash(&mut hasher);
    hasher.finish()
}

impl<Msg: Send + 'static> Subscription<Msg> {
    /// Aucune souscription.
    pub fn none() -> Self {
        Self { entries: Vec::new() }
    }

    /// Regroupe plusieurs souscriptions.
    pub fn batch(subscriptions: impl IntoIterator<Item = Subscription<Msg>>) -> Self {
        let mut entries = Vec::new();
        for subscription in subscriptions {
            entries.extend(subscription.entries);
        }
        Self { entries }
    }

    /// Émet un message toutes les `interval` (le `Instant` du tick est fourni).
    pub fn every(interval: Duration, make: impl Fn(Instant) -> Msg + Send + 'static) -> Self {
        Self {
            entries: vec![Entry {
                id: every_id(interval),
                kind: Kind::Every {
                    interval,
                    make: Box::new(make),
                },
            }],
        }
    }

    /// `true` si aucune souscription.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Les ids des souscriptions (pour inspection / test).
    pub fn ids(&self) -> Vec<u64> {
        self.entries.iter().map(|entry| entry.id).collect()
    }

    /// Extrait les entrées (pour le diff par le framework).
    pub(crate) fn into_entries(self) -> Vec<Entry<Msg>> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_empty() {
        assert!(Subscription::<u32>::none().is_empty());
    }

    #[test]
    fn every_id_is_stable_per_duration() {
        let a = Subscription::every(Duration::from_secs(1), |_| 0u32);
        let b = Subscription::every(Duration::from_secs(1), |_| 0u32);
        let c = Subscription::every(Duration::from_secs(2), |_| 0u32);
        assert_eq!(a.ids(), b.ids(), "même durée → même id");
        assert_ne!(a.ids(), c.ids(), "durée différente → id différent");
    }

    #[test]
    fn batch_combines_entries() {
        let combined = Subscription::batch([
            Subscription::every(Duration::from_secs(1), |_| 0u32),
            Subscription::none(),
            Subscription::every(Duration::from_secs(2), |_| 0u32),
        ]);
        assert_eq!(combined.ids().len(), 2);
    }
}
