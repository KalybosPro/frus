//! **Documents fetched from an address and parsed** — SVG pictures and Lottie animations
//! (milestone 693) — through the fetcher images use ([`frus_core::image_fetcher`]).
//!
//! Like a fetched image, the first ask starts the fetch and every later one is a lookup, so
//! asking from a view on every frame makes one request. A failure is remembered as well:
//! a link that is broken now will be broken on the next frame. The bytes are parsed where
//! the fetcher hands them over, off the interface's thread on every platform that has
//! threads.

use std::collections::HashMap;
use std::sync::Mutex;

/// Where a document is.
#[derive(Clone, Debug)]
pub(crate) enum Remote<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// The most documents a store keeps; past this, the ones not in flight are forgotten.
const ENTRIES: usize = 256;

/// One kind of document, by address.
pub(crate) struct Store<T> {
    entries: Mutex<Option<HashMap<String, Remote<T>>>>,
}

impl<T: Clone + Send + 'static> Store<T> {
    pub(crate) const fn new() -> Self {
        Self {
            entries: Mutex::new(None),
        }
    }

    /// The document at `url`, starting its fetch on the first ask.
    pub(crate) fn get(
        &'static self,
        url: &str,
        parse: fn(&[u8]) -> Result<T, String>,
    ) -> Remote<T> {
        let mut guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard.get_or_insert_with(HashMap::new);
        if let Some(known) = store.get(url) {
            return known.clone();
        }
        if store.len() >= ENTRIES {
            store.retain(|_, entry| matches!(entry, Remote::Loading));
        }
        let Some(fetcher) = frus_core::image_fetcher() else {
            let failed = Remote::Failed("no fetcher registered".to_string());
            store.insert(url.to_string(), failed.clone());
            return failed;
        };
        store.insert(url.to_string(), Remote::Loading);
        // The lock goes before the call: a fetcher may answer on this thread, and its
        // callback takes the same lock.
        drop(guard);
        let key = url.to_string();
        fetcher(
            url,
            Box::new(move |result| {
                let outcome = match result {
                    Ok(bytes) => {
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            parse(&bytes)
                        })) {
                            Ok(Ok(document)) => Remote::Ready(document),
                            Ok(Err(why)) => Remote::Failed(why),
                            Err(_) => Remote::Failed("the parser panicked".to_string()),
                        }
                    }
                    Err(why) => {
                        log::warn!("frus: {key} could not be fetched: {why}");
                        Remote::Failed(why)
                    }
                };
                let mut guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
                guard.get_or_insert_with(HashMap::new).insert(key, outcome);
            }),
        );
        // It may have answered already.
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|store| store.get(url).cloned())
            .unwrap_or(Remote::Loading)
    }

    /// How many fetches are still on their way.
    pub(crate) fn in_flight(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map_or(0, |store| {
                store
                    .values()
                    .filter(|entry| matches!(entry, Remote::Loading))
                    .count()
            })
    }
}

/// Fetched images, pictures and animations still on their way. The shell keeps frames
/// coming while there are any, so that what arrives is shown.
pub fn images_in_flight() -> usize {
    #[allow(unused_mut)]
    let mut count = frus_core::images_in_flight();
    #[cfg(feature = "svg")]
    {
        count += crate::svg::in_flight();
    }
    #[cfg(feature = "lottie")]
    {
        count += crate::lottie::in_flight();
    }
    count
}

/// Held by every test that registers a fetcher: there is one for the process, so tests
/// running side by side would replace each other's.
#[cfg(test)]
pub(crate) fn fetcher_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(bytes: &[u8]) -> Result<String, String> {
        std::str::from_utf8(bytes)
            .map(str::to_string)
            .map_err(|e| e.to_string())
    }

    fn explode(_: &[u8]) -> Result<String, String> {
        panic!("a parser bug")
    }

    /// Answers at once, with the address as the document; `bad` addresses fail.
    fn instant(url: &str, deliver: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>) {
        if url.contains("bad") {
            deliver(Err("404".to_string()));
        } else {
            deliver(Ok(url.as_bytes().to_vec()));
        }
    }

    #[test]
    fn a_document_is_fetched_once_and_failures_are_kept() {
        let _guard = fetcher_lock();
        static STORE: Store<String> = Store::new();
        frus_core::set_image_fetcher(instant);
        let Remote::Ready(text) = STORE.get("remote://a", parse) else {
            panic!("answered on the spot");
        };
        assert_eq!(text, "remote://a");
        assert!(matches!(STORE.get("remote://bad", parse), Remote::Failed(why) if why == "404"));
        assert_eq!(STORE.in_flight(), 0);

        static PANICKY: Store<String> = Store::new();
        assert!(
            matches!(PANICKY.get("remote://p", explode), Remote::Failed(why) if why.contains("panicked"))
        );

        // A fetcher that never answers: in flight, counted, and asked only once.
        static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        fn never(_: &str, _: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>) {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        static SLOW: Store<String> = Store::new();
        frus_core::set_image_fetcher(never);
        assert!(matches!(SLOW.get("remote://slow", parse), Remote::Loading));
        assert!(matches!(SLOW.get("remote://slow", parse), Remote::Loading));
        assert_eq!(CALLS.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(SLOW.in_flight(), 1);
        // The one already parsed is still there, fetcher or not.
        assert!(matches!(STORE.get("remote://a", parse), Remote::Ready(_)));
    }
}
