//! [`Subscription`]: the **continuous sources** of messages an application
//! declares — the streaming counterpart to [`crate::Command`], which is one-shot.
//!
//! An app declares its subscriptions **as a function of its state**, and the
//! framework **diffs** them every cycle, starting the new ones and stopping those
//! that were withdrawn. That works thanks to a stable **id** per subscription, the
//! hash of its recipe.

use std::time::Duration;

use web_time::Instant;

/// A timer's message factory, called at every interval.
type TimerFn<Msg> = Box<dyn Fn(Instant) -> Msg + Send>;

/// What a subscription is.
pub(crate) enum Kind<Msg> {
    /// Emits a message at a regular interval.
    Every {
        interval: Duration,
        make: TimerFn<Msg>,
    },
}

/// A single subscription: its id, used by the diff, and its nature.
pub(crate) struct Entry<Msg> {
    pub(crate) id: u64,
    pub(crate) kind: Kind<Msg>,
}

/// A set of continuous message sources, possibly empty.
pub struct Subscription<Msg> {
    entries: Vec<Entry<Msg>>,
}

/// An `every`'s stable id: the hash of its recipe, the kind plus the duration in ms.
fn every_id(interval: Duration) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    "every".hash(&mut hasher);
    interval.as_millis().hash(&mut hasher);
    hasher.finish()
}

impl<Msg: Send + 'static> Subscription<Msg> {
    /// No subscription at all.
    pub fn none() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Groups several subscriptions together.
    pub fn batch(subscriptions: impl IntoIterator<Item = Subscription<Msg>>) -> Self {
        let mut entries = Vec::new();
        for subscription in subscriptions {
            entries.extend(subscription.entries);
        }
        Self { entries }
    }

    /// Emits a message every `interval`; the tick's `Instant` is passed along.
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

    /// `true` when there is no subscription.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The subscriptions' ids, for inspection and testing.
    pub fn ids(&self) -> Vec<u64> {
        self.entries.iter().map(|entry| entry.id).collect()
    }

    /// Takes the entries out, for the framework to diff.
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
        assert_eq!(a.ids(), b.ids(), "same duration → same id");
        assert_ne!(a.ids(), c.ids(), "different duration → different id");
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
