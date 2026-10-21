//! Bitmap images: decoded, shared pixels, and how they fit inside a box.
//!
//! `frus-core` neither decodes (PNG/JPEG and friends) nor uploads to the GPU: it
//! holds only the **raw pixels** ([`ImageData`], RGBA sRGB) behind a shared handle
//! ([`ImageHandle`]), plus the **fitting** logic ([`BoxFit`]). Decoding lives in a
//! dedicated layer; uploading lives in `frus-gpu`, which caches the texture by
//! [`ImageData::id`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::{Alignment, Rect, Size};

/// Identity counter: every [`ImageData`] gets a unique, stable id, which is the
/// GPU-side cache key — it keeps the same pixels from being re-uploaded each frame.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// An image's **pixels**: 8-bit RGBA, sRGB, row by row from the top-left corner
/// (`width * height * 4` bytes). Immutable once built.
#[derive(Debug)]
pub struct ImageData {
    id: u64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl ImageData {
    /// Builds an image from raw RGBA pixels. Panics if the length is not
    /// `width * height * 4`.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(
            rgba.len(),
            (width as usize) * (height as usize) * 4,
            "RGBA pixels must be width*height*4 bytes"
        );
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rgba,
        }
    }

    /// Wraps the image in a shared handle (cheap to clone, stable for caching).
    pub fn into_handle(self) -> ImageHandle {
        Arc::new(self)
    }

    /// Unique, stable identity (the GPU cache key).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Size in pixels, as a [`Size`].
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// The RGBA bytes (sRGB), row by row.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Two images are "equal" when they share the same identity — the same resource —
/// without comparing pixels. That keeps cache lookups and scene equality cheap.
impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ImageData {}

/// A **shared** image handle: cheap to clone (reference counted), and stored as-is
/// inside a [`crate::Primitive::Image`].
pub type ImageHandle = Arc<ImageData>;

/// **The size an image is decoded at**, in physical pixels: the full size of the file, or
/// a smaller one that the place it is shown at needs.
///
/// A twelve-megapixel photograph shown as an 80×80 thumbnail does not need 48 MB of
/// pixels, and on a phone a gallery of them is what gets an application killed for
/// memory. Decoding at the size shown keeps the cost to what is on the screen.
///
/// - Neither side given: the file's own size ([`DecodeSize::FULL`]).
/// - One side: that side, and the other from the image's own ratio.
/// - Both: the smallest size, at the image's own ratio, that **covers** both. The ratio
///   is never changed, so a picture cropped to a square box keeps its shape, and one
///   letterboxed into it is not short of pixels either.
///
/// An image is **never decoded larger** than the file: asking for more gives the file's
/// own size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DecodeSize {
    /// The width wanted, in physical pixels.
    pub width: Option<u32>,
    /// The height wanted, in physical pixels.
    pub height: Option<u32>,
}

impl DecodeSize {
    /// The file's own size.
    pub const FULL: Self = Self {
        width: None,
        height: None,
    };

    /// A size with either side, both, or neither.
    pub fn new(width: Option<u32>, height: Option<u32>) -> Self {
        Self { width, height }
    }

    /// Whether this is the file's own size.
    pub fn is_full(self) -> bool {
        self.width.is_none() && self.height.is_none()
    }

    /// The size a `full_width` × `full_height` image is decoded at, by the rules above.
    ///
    /// ```
    /// use frus_core::DecodeSize;
    /// let photo = (4000, 3000);
    /// // A square thumbnail covered by the photo, at the photo's own ratio.
    /// assert_eq!(DecodeSize::new(Some(200), Some(200)).target(photo.0, photo.1), (267, 200));
    /// // One side: the other follows.
    /// assert_eq!(DecodeSize::new(Some(400), None).target(photo.0, photo.1), (400, 300));
    /// // Never larger than the file.
    /// assert_eq!(DecodeSize::new(Some(8000), None).target(photo.0, photo.1), photo);
    /// ```
    pub fn target(self, full_width: u32, full_height: u32) -> (u32, u32) {
        if full_width == 0 || full_height == 0 {
            return (full_width, full_height);
        }
        let along = |wanted: Option<u32>, full: u32| wanted.map(|w| w as f64 / full as f64);
        let scale = match (
            along(self.width, full_width),
            along(self.height, full_height),
        ) {
            (None, None) => return (full_width, full_height),
            (Some(s), None) | (None, Some(s)) => s,
            // Covering both is the larger of the two scales.
            (Some(a), Some(b)) => a.max(b),
        };
        if scale >= 1.0 {
            return (full_width, full_height);
        }
        let side = |full: u32| ((full as f64 * scale).round() as u32).clamp(1, full);
        (side(full_width), side(full_height))
    }
}

/// The process-wide store of images already turned into pixels, keyed by where their
/// bytes live.
///
/// Decoding a PNG is not free and a view is rebuilt every frame, so an image resolved
/// in `view` must be resolved **once** rather than sixty times a second. That is what
/// this is for, and it is a process-wide store for the same reason the font registry is
/// one: the same asset shown on three screens is one image, not three.
///
/// A failure is cached too. A file that is not a PNG will not become one on the next
/// frame, and retrying the decode every frame would turn one broken asset into a
/// permanent cost.
///
/// An entry is [`Fetched::Loading`] while a decode started by [`decoded_at`] is running
/// on a decoding thread.
static CACHE: Mutex<Option<HashMap<(usize, DecodeSize), Fetched>>> = Mutex::new(None);

/// How many threads decode images in the background, at most.
///
/// Two, whatever the machine: a decode is a burst of memory — a PNG is decoded at the
/// file's own size before it is brought down — and a gallery asking for forty pictures at
/// once must not hold forty of those bursts at the same time. Two keep a phone's other
/// cores for the interface and still overlap one decode with the next.
#[cfg(not(target_family = "wasm"))]
const DECODING_THREADS: usize = 2;

/// Work for the decoding threads.
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Runs `job` on one of the image decoding threads, or right here where there are none.
///
/// The threads are started on the first job and live for the process; the queue is first
/// in, first out. The Web has no threads to start, so there the job runs in place, which
/// is what every decode did before there were threads.
fn in_background(job: Job) {
    #[cfg(target_family = "wasm")]
    job();
    #[cfg(not(target_family = "wasm"))]
    {
        static QUEUE: std::sync::OnceLock<Option<std::sync::mpsc::Sender<Job>>> =
            std::sync::OnceLock::new();
        let queue = QUEUE.get_or_init(|| {
            let (sender, receiver) = std::sync::mpsc::channel::<Job>();
            let receiver = Arc::new(Mutex::new(receiver));
            let workers = std::thread::available_parallelism()
                .map_or(1, |n| n.get().saturating_sub(1))
                .clamp(1, DECODING_THREADS);
            let started = (0..workers)
                .filter(|i| {
                    let receiver = Arc::clone(&receiver);
                    std::thread::Builder::new()
                        .name(format!("frus-image-{i}"))
                        .spawn(move || loop {
                            let next = receiver.lock().unwrap_or_else(|e| e.into_inner()).recv();
                            match next {
                                Ok(job) => job(),
                                Err(_) => return,
                            }
                        })
                        .is_ok()
                })
                .count();
            // A platform that refuses threads decodes in place rather than queueing work
            // nobody will ever pick up.
            (started > 0).then_some(sender)
        });
        match queue {
            Some(sender) => {
                if let Err(refused) = sender.send(job) {
                    (refused.0)();
                }
            }
            None => job(),
        }
    }
}

/// `decode`, with a panic in it turned into a failure: a decoder that panics on a
/// decoding thread would otherwise leave its picture loading for ever, and the interface
/// redrawing for ever with it.
fn guarded(decode: impl FnOnce() -> Result<ImageData, String>) -> Fetched {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(decode)) {
        Ok(Ok(data)) => Fetched::Ready(data.into_handle()),
        Ok(Err(why)) => Fetched::Failed(why),
        Err(_) => Fetched::Failed("the image decoder panicked".to_string()),
    }
}

/// Resolves `bytes` to pixels **once**, returning the same handle on every later call.
///
/// The key is the **address** of the slice rather than its contents. That is exact for
/// what this is for — `include_bytes!` gives a `&'static [u8]` whose address is fixed
/// for the life of the process, and two distinct assets are two distinct statics — and
/// it costs a pointer comparison, where hashing the bytes would cost re-reading the
/// whole file on every frame that shows it.
///
/// The `'static` bound is what makes that sound: bytes that can be freed could have
/// their address reused by something else, and the cache would hand back the wrong
/// picture. An application holding runtime bytes decodes them itself and keeps the
/// [`ImageHandle`].
///
/// `load` runs only on the first call for a given slice, and its failure is remembered.
pub fn cached(
    bytes: &'static [u8],
    load: impl FnOnce(&[u8]) -> Result<ImageData, String>,
) -> Result<ImageHandle, String> {
    cached_at(bytes, DecodeSize::FULL, load)
}

/// [`cached`], at a [`DecodeSize`]: the same bytes decoded at two sizes are two entries,
/// and `load` is the one that decodes at `size`.
pub fn cached_at(
    bytes: &'static [u8],
    size: DecodeSize,
    load: impl FnOnce(&[u8]) -> Result<ImageData, String>,
) -> Result<ImageHandle, String> {
    let key = (bytes.as_ptr() as usize, size);
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(HashMap::new);
    match cache.get(&key) {
        Some(Fetched::Ready(handle)) => return Ok(handle.clone()),
        Some(Fetched::Failed(why)) => return Err(why.clone()),
        // Still decoding on a thread, for [`decoded_at`]: the caller of this one wants the
        // pixels now, so it decodes them itself, and the thread's copy replaces this one
        // when it lands.
        Some(Fetched::Loading) | None => {}
    }
    let resolved = load(bytes).map(ImageData::into_handle);
    let state = match &resolved {
        Ok(handle) => Fetched::Ready(handle.clone()),
        Err(why) => Fetched::Failed(why.clone()),
    };
    cache.insert(key, state);
    resolved
}

/// [`cached_at`], **decoding on a background thread** when `in_the_background` says so:
/// [`Fetched::Loading`] until the pixels are there, and the frames keep coming meanwhile
/// ([`images_in_flight`] counts it).
///
/// A large photograph takes tens of milliseconds to decode on a phone — several frames —
/// and an interface that decodes it in the frame that first shows it stops for that long.
/// A small picture decodes in less time than a thread takes to pick it up, and showing it
/// a frame late would only make it blink in, so the caller says which this is.
///
/// On the Web, which has no threads, the decode happens here either way.
pub fn decoded_at(
    bytes: &'static [u8],
    size: DecodeSize,
    decode: fn(&[u8], DecodeSize) -> Result<ImageData, String>,
    in_the_background: bool,
) -> Fetched {
    let key = (bytes.as_ptr() as usize, size);
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(found) = cache.get(&key) {
        return found.clone();
    }
    if !in_the_background {
        let state = guarded(|| decode(bytes, size));
        cache.insert(key, state.clone());
        return state;
    }
    cache.insert(key, Fetched::Loading);
    // The lock goes before the job is handed over: where there are no threads, the job
    // runs on this one and takes the same lock to put its result in.
    drop(guard);
    in_background(Box::new(move || {
        let state = guarded(|| decode(bytes, size));
        let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        guard.get_or_insert_with(HashMap::new).insert(key, state);
    }));
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|cache| cache.get(&key).cloned())
        .unwrap_or(Fetched::Loading)
}

/// Empties the store. For tests that want a decode to happen again; an application has
/// no reason to call it, since the entries are one per embedded asset and size, and
/// bounded by how many the binary carries. [`trim_image_caches`] is the one for memory.
pub fn forget_cached_images() {
    if let Ok(mut guard) = CACHE.lock() {
        *guard = None;
    }
}

/// **Gives back every decoded image nobody is showing**: what the system asks for when it
/// is short of memory.
///
/// Both stores are swept — embedded images and fetched ones — and only the pictures held
/// by nothing but the store go. One on screen stays, since dropping it would only make the
/// next frame decode it again; one in flight stays, since forgetting it does not cancel it.
/// What goes comes back, decoded again, the next time it is shown.
///
/// The shell calls this when the platform says memory is low (Android's low-memory
/// callback, iOS's memory warning). An application can call it too — on leaving a screen
/// full of photographs, say.
pub fn trim_image_caches() {
    if let Ok(mut guard) = CACHE.lock() {
        if let Some(cache) = guard.as_mut() {
            cache.retain(|_, entry| match entry {
                Fetched::Ready(handle) => Arc::strong_count(handle) > 1,
                // A failure costs a sentence, and remembering it is what keeps a broken
                // asset from being decoded again on every frame; one still decoding is on
                // its way to somebody who asked.
                Fetched::Failed(_) | Fetched::Loading => true,
            });
        }
    }
    let mut guard = FETCHED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(store) = guard.as_mut() {
        evict_over_budget(store, 0);
    }
}

/// What is known about an image that has to be **fetched** before it can be shown.
///
/// Three states rather than two, because "not here yet" and "will never be here" are
/// different answers and an interface shows them differently: one is a placeholder, the
/// other is a message.
#[derive(Clone, Debug)]
pub enum Fetched {
    /// In flight. Ask again on a later frame.
    Loading,
    /// Here.
    Ready(ImageHandle),
    /// It will not arrive, and this is why.
    Failed(String),
}

/// Hands the bytes at `url` to the callback, once, from wherever suits the platform.
///
/// A **function pointer** and not a closure: this is registered once for the process, and
/// a plain `fn` needs no allocation, no lifetime and no `Sync` wrapper to store.
pub type ImageFetcher = fn(&str, Box<dyn FnOnce(Result<Vec<u8>, String>) + Send + 'static>);

impl Fetched {
    /// The decoded pixels this state is holding on to, in bytes.
    ///
    /// `Loading` holds nothing yet and `Failed` holds a sentence; only `Ready` is
    /// heavy, and it is as heavy as the picture is big.
    fn bytes(&self) -> usize {
        match self {
            Fetched::Ready(handle) => handle.rgba.len(),
            _ => 0,
        }
    }
}

/// A store entry: the state, plus **when it was last asked for**.
struct Entry {
    state: Fetched,
    /// A tick from [`CLOCK`], not a wall time.
    used: u64,
}

/// The default ceiling on decoded network images held in memory: the reference's own
/// figure.
///
/// It is a **default**, not a rule — see [`set_image_cache_budget`]. A phone with a
/// modest heap wants less; a desktop tool showing one enormous photograph may want more
/// than one picture's worth.
pub const DEFAULT_IMAGE_CACHE_BYTES: usize = 100 * 1024 * 1024;

/// The ceiling on the total decoded size of `Ready` entries.
static BUDGET: AtomicUsize = AtomicUsize::new(DEFAULT_IMAGE_CACHE_BYTES);

/// A monotonic tick, bumped every time an image is asked for, and the store's whole
/// notion of *recently*.
///
/// A counter rather than a clock. It needs no platform time source — `frus-core`
/// compiles for the Web, where `Instant::now` is not a thing — it cannot go backwards
/// when a machine's clock is corrected, and *least recently used* only ever asks which
/// of two numbers is smaller. A wall time would answer the same question with more
/// machinery and one more way to be wrong.
static CLOCK: AtomicU64 = AtomicU64::new(0);

static FETCHER: Mutex<Option<ImageFetcher>> = Mutex::new(None);
static FETCHED: Mutex<Option<HashMap<String, Entry>>> = Mutex::new(None);

/// Sets the ceiling on decoded network images held in memory, in bytes.
///
/// `0` keeps nothing that nobody is holding: every image is dropped the moment it
/// leaves the screen and fetched again when it comes back. That is a legitimate answer
/// for a device with almost no memory, and a bad one for anything else.
pub fn set_image_cache_budget(bytes: usize) {
    BUDGET.store(bytes, Ordering::Relaxed);
    let mut guard = FETCHED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(store) = guard.as_mut() {
        evict_over_budget(store, bytes);
    }
}

/// The ceiling currently in force.
pub fn image_cache_budget() -> usize {
    BUDGET.load(Ordering::Relaxed)
}

/// The decoded bytes the store is currently holding.
pub fn image_cache_bytes() -> usize {
    FETCHED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map_or(0, |store| store.values().map(|e| e.state.bytes()).sum())
}

/// Drops least-recently-used images until the store is inside its budget.
///
/// **Two entries are never dropped**, and both exclusions are the difference between a
/// cache and a bug:
///
/// - a `Loading` one is *in flight*. Dropping it does not cancel the request — nothing
///   here can — it only forgets that one was made, so the next frame starts a second,
///   and [`images_in_flight`] falls to zero, and the redraw loop that was keeping the
///   frame alive until the picture arrived stops. The picture then lands in a store
///   nobody will look at again until something else forces a frame.
/// - one whose handle is held **elsewhere** is on screen, or in a scene about to be
///   drawn. Dropping that is not unsafe — the `Arc` keeps the pixels alive for whoever
///   holds them — but the next `view` finds nothing, asks again, and the image flickers
///   through a placeholder on its way back to exactly where it was. A store that evicts
///   what is visible is a store that fetches the same picture for ever.
///
/// `Arc::strong_count == 1` is how the second is asked: one reference, and it is this
/// store's own.
///
/// The budget is a **parameter** rather than a read of [`BUDGET`] inside. The rule is
/// worth testing on its own — which of two pictures goes first, and which two never do
/// — and a function that reached for a process-wide value would make those tests race
/// each other for it.
fn evict_over_budget(store: &mut HashMap<String, Entry>, budget: usize) {
    let mut held: usize = store.values().map(|e| e.state.bytes()).sum();
    while held > budget {
        let victim = store
            .iter()
            .filter(|(_, entry)| match &entry.state {
                Fetched::Ready(handle) => Arc::strong_count(handle) == 1,
                _ => false,
            })
            .min_by_key(|(_, entry)| entry.used)
            .map(|(url, _)| url.clone());
        // Nothing left that can go: every remaining picture is either in flight or on
        // screen. Over budget is the right answer here — the alternative is dropping
        // something that is being looked at.
        let Some(url) = victim else { return };
        if let Some(entry) = store.remove(&url) {
            held -= entry.state.bytes();
        }
    }
}

/// Names the thing that gets bytes over the network.
///
/// The widget layer cannot do this itself and should not: it has no runtime, no socket
/// and no dependency on the shell — the dependency runs the other way. So the shell,
/// which has all three, says how, and the widget layer only asks. It is the shape the
/// image **decoder** took a step earlier, for the same reason.
///
/// An application using the framework's own shell never calls this: the shell registers
/// its `fetch_bytes` on the way up. One embedding frus in a host that already has an HTTP
/// client can point this at that client instead.
pub fn set_image_fetcher(fetcher: ImageFetcher) {
    if let Ok(mut guard) = FETCHER.lock() {
        *guard = Some(fetcher);
    }
}

/// What is known about the image at `url`, starting the fetch if this is the first ask.
///
/// Called from a view, which runs every frame: the **first** call starts the work and
/// says `Loading`, and every later one is a lookup. That is what makes it safe to write
/// in a `view` at all — a fetch per frame would be sixty requests a second for one
/// picture.
///
/// A failure is remembered like any other, and for the same reason as milestone 372's:
/// a URL that 404s will 404 again, and retrying every frame turns one bad link into a
/// permanent load on somebody's server.
pub fn fetched(url: &str, decode: fn(&[u8]) -> Result<ImageData, String>) -> Fetched {
    fetch(url, url, Box::new(decode))
}

/// [`fetched`], at a [`DecodeSize`]: `decode` is handed the size along with the bytes.
///
/// The same address at two sizes is two entries, each fetched on its own: a thumbnail and
/// the full picture it opens are two downloads, though an HTTP cache usually answers the
/// second.
pub fn fetched_at(
    url: &str,
    size: DecodeSize,
    decode: fn(&[u8], DecodeSize) -> Result<ImageData, String>,
) -> Fetched {
    if size.is_full() {
        return fetch(url, url, Box::new(move |bytes| decode(bytes, size)));
    }
    let key = sized_key(url, size);
    fetch(&key, url, Box::new(move |bytes| decode(bytes, size)))
}

/// The store's key for `url` at a size other than the full one. The full size keeps the
/// address alone, so [`fetched`] and a full-size [`fetched_at`] share an entry; a NUL
/// cannot appear in a URL, so no address collides with a sized key.
fn sized_key(url: &str, size: DecodeSize) -> String {
    let side = |s: Option<u32>| s.map_or(String::new(), |s| s.to_string());
    format!("{url}\0{}x{}", side(size.width), side(size.height))
}

/// What turns the fetched bytes into pixels.
type Decoder = Box<dyn FnOnce(&[u8]) -> Result<ImageData, String> + Send + 'static>;

/// The store's lookup, and the start of the fetch when `key` is new.
fn fetch(key: &str, url: &str, decode: Decoder) -> Fetched {
    let mut guard = FETCHED.lock().unwrap_or_else(|e| e.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    let now = CLOCK.fetch_add(1, Ordering::Relaxed);
    if let Some(known) = store.get_mut(key) {
        // Asked for is used: a picture on a screen nobody has scrolled away from is
        // asked for on every frame, which is exactly what keeps it out of the way of
        // the eviction sweep.
        known.used = now;
        return known.state.clone();
    }
    let Some(fetcher) = *FETCHER.lock().unwrap_or_else(|e| e.into_inner()) else {
        // No fetcher: a build without the network, or a host that never named one.
        // Saying so is an answer; hanging on `Loading` for ever is not.
        let failed = Fetched::Failed("no image fetcher registered".to_string());
        store.insert(
            key.to_string(),
            Entry {
                state: failed.clone(),
                used: now,
            },
        );
        return failed;
    };
    store.insert(
        key.to_string(),
        Entry {
            state: Fetched::Loading,
            used: now,
        },
    );
    // The lock goes **before** the call. A fetcher is free to answer on this thread —
    // a cache hit in a host client, a test double — and its callback locks the same
    // store, which would be a deadlock against a guard still held here.
    drop(guard);

    let key = key.to_string();
    fetcher(
        url,
        Box::new(move |result| match result {
            // The bytes are decoded on the decoding threads, not on whichever thread the
            // fetcher answered on: that one is the network's, and a picture is a burst of
            // memory that has to be counted with the others.
            Ok(bytes) => in_background(Box::new(move || {
                let outcome = guarded(|| decode(&bytes));
                land(key, outcome);
            })),
            Err(why) => land(key, Fetched::Failed(why)),
        }),
    );
    Fetched::Loading
}

/// Puts what became of a fetch in the store.
fn land(key: String, outcome: Fetched) {
    if let Ok(mut guard) = FETCHED.lock() {
        let store = guard.get_or_insert_with(HashMap::new);
        store.insert(
            key,
            Entry {
                state: outcome,
                used: CLOCK.fetch_add(1, Ordering::Relaxed),
            },
        );
        // The one moment the store grows. Sweeping here rather than on every ask means
        // the cost is paid once per picture that arrives, not once per frame per picture
        // on screen.
        evict_over_budget(store, BUDGET.load(Ordering::Relaxed));
    }
}

/// How many images are in flight right now.
///
/// The interface has to keep drawing while any of them are, or the frame that would show
/// the picture never happens. A count rather than a flag on the widget, because the
/// application may well have taken the image **out** of the tree while it loads — that
/// is what showing a placeholder means — and the work is still going on.
///
/// Counted off the store rather than kept in a separate tally beside it. A tally is a
/// second copy of the same fact and the two can disagree: incremented here and
/// decremented in a callback, it survives a [`forget_fetched_images`] that empties the
/// store, and it can be decremented below zero by a callback that arrives after one.
/// Either way the interface is left redrawing for ever over work that finished. The
/// store already knows — the entries that say `Loading` **are** the ones in flight —
/// and there are only ever as many entries as the application has network images.
pub fn images_in_flight() -> usize {
    let fetched = FETCHED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map_or(0, |store| {
            store
                .values()
                .filter(|entry| matches!(entry.state, Fetched::Loading))
                .count()
        });
    // And the embedded pictures still decoding on a thread.
    let decoding = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map_or(0, |cache| {
            cache
                .values()
                .filter(|state| matches!(state, Fetched::Loading))
                .count()
        });
    fetched + decoding
}

/// Forgets every fetched image and every remembered failure. For tests; see
/// [`forget_cached_images`].
pub fn forget_fetched_images() {
    if let Ok(mut guard) = FETCHED.lock() {
        *guard = None;
    }
}

/// How an image is fitted into its destination box — the same set of modes as the
/// CSS `object-fit` property.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoxFit {
    /// Stretches to fill the box; the aspect ratio is **not** preserved.
    Fill,
    /// The largest scale that still **fits** inside the box (letterbox).
    #[default]
    Contain,
    /// The smallest scale that **covers** the box (cropping).
    Cover,
    /// Fits to the **width**; may overflow vertically.
    FitWidth,
    /// Fits to the **height**; may overflow horizontally.
    FitHeight,
    /// Natural size, centred — neither scaled up nor down.
    None,
    /// Like [`BoxFit::Contain`], but **never scaled up** — only down.
    ScaleDown,
}

impl BoxFit {
    /// Computes the **destination** rectangle (inside or around `dst`) and the **UV**
    /// rectangle (the sub-region of the texture, in `0..1`) for drawing an image of
    /// size `src` in this mode. `dst` is never cropped: letterboxing shrinks `dst`
    /// and keeps full UV; cropping keeps `dst` full and shrinks the UV.
    pub fn apply(self, src: Size, dst: Rect) -> (Rect, Rect) {
        self.apply_aligned(src, dst, Alignment::CENTER)
    }

    /// [`BoxFit::apply`] with a say in **where** the spare room goes: which part of the
    /// box a letterboxed image sits in, and which part of the image a cropping one keeps.
    ///
    /// Centre is the reference's default and what `apply` uses. It matters as soon as an
    /// image is not the shape of its box: a portrait cropped to a banner should usually
    /// keep its top, not its middle.
    pub fn apply_aligned(self, src: Size, dst: Rect, align: Alignment) -> (Rect, Rect) {
        let full_uv = Rect::new(0.0, 0.0, 1.0, 1.0);
        if src.width <= 0.0 || src.height <= 0.0 || dst.width <= 0.0 || dst.height <= 0.0 {
            return (dst, full_uv);
        }
        let sx = dst.width / src.width;
        let sy = dst.height / src.height;
        let (fx, fy) = (align.fraction_x(), align.fraction_y());

        // Letterbox: the image at `scale`, placed inside `dst` by the alignment, with
        // full UV.
        let letterbox = |scale: f32| {
            let w = src.width * scale;
            let h = src.height * scale;
            let x = dst.x + (dst.width - w) * fx;
            let y = dst.y + (dst.height - h) * fy;
            (Rect::new(x, y, w, h), full_uv)
        };

        match self {
            BoxFit::Fill => (dst, full_uv),
            BoxFit::Contain => letterbox(sx.min(sy)),
            BoxFit::FitWidth => letterbox(sx),
            BoxFit::FitHeight => letterbox(sy),
            BoxFit::None => letterbox(1.0),
            BoxFit::ScaleDown => letterbox(sx.min(sy).min(1.0)),
            BoxFit::Cover => {
                // The image covers `dst`; the crop happens in the UV, centred.
                let scale = sx.max(sy);
                let scaled_w = src.width * scale;
                let scaled_h = src.height * scale;
                let uv_w = (dst.width / scaled_w).min(1.0);
                let uv_h = (dst.height / scaled_h).min(1.0);
                // The crop travels the other way: aligning the image to the top means
                // keeping the **top** of it, which is the low end of the UV.
                let uv = Rect::new((1.0 - uv_w) * fx, (1.0 - uv_h) * fy, uv_w, uv_h);
                (dst, uv)
            }
        }
    }

    /// The `(sx, sy)` scale factors that fit content of size `src` into a box of
    /// size `dst` in this mode — the building block of `FittedBox`, where the
    /// content, unlike a sampled image, is **scaled** and then centred. `Fill`
    /// stretches per axis; every other mode is **uniform**, preserving the aspect
    /// ratio. A degenerate `src` yields `(1, 1)`.
    pub fn scale(self, src: Size, dst: Size) -> (f32, f32) {
        if src.width <= 0.0 || src.height <= 0.0 {
            return (1.0, 1.0);
        }
        let sx = dst.width / src.width;
        let sy = dst.height / src.height;
        match self {
            BoxFit::Fill => (sx, sy),
            BoxFit::Contain => {
                let s = sx.min(sy);
                (s, s)
            }
            BoxFit::Cover => {
                let s = sx.max(sy);
                (s, s)
            }
            BoxFit::FitWidth => (sx, sx),
            BoxFit::FitHeight => (sy, sy),
            BoxFit::None => (1.0, 1.0),
            BoxFit::ScaleDown => {
                let s = sx.min(sy).min(1.0);
                (s, s)
            }
        }
    }
}

/// The eviction sweep, driven directly rather than through a fetcher.
///
/// A network image needs a socket, a runtime and a shell; the rule being checked here
/// needs none of them, and a test that stood up all three to prove that the older of two
/// pictures goes first would be checking the shell.
#[cfg(test)]
mod cache_tests {
    use super::*;

    /// A picture of `mb` megabytes, ready and held only by the store.
    fn ready(mb: usize) -> Fetched {
        let side = 512u32;
        let bytes = (side * side * 4) as usize;
        let count = mb * 1024 * 1024 / bytes;
        let rgba = vec![0u8; bytes * count.max(1)];
        Fetched::Ready(
            ImageData::from_rgba(side, (side as usize * count.max(1)) as u32, rgba).into_handle(),
        )
    }

    fn store_of(entries: Vec<(&str, Fetched, u64)>) -> HashMap<String, Entry> {
        entries
            .into_iter()
            .map(|(url, state, used)| (url.to_string(), Entry { state, used }))
            .collect()
    }

    fn held(store: &HashMap<String, Entry>) -> usize {
        store.values().map(|e| e.state.bytes()).sum()
    }

    /// Only `Ready` weighs anything: a request in flight holds no pixels yet, and a
    /// failure holds a sentence.
    #[test]
    fn only_a_ready_image_weighs_anything() {
        assert_eq!(Fetched::Loading.bytes(), 0);
        assert_eq!(Fetched::Failed("gone".into()).bytes(), 0);
        assert!(ready(4).bytes() >= 4 * 1024 * 1024);
    }

    /// Over the budget, the **least recently asked for** goes first.
    #[test]
    fn the_oldest_unheld_picture_goes_first() {
        let mut store = store_of(vec![
            ("old", ready(4), 1),
            ("middling", ready(4), 5),
            ("fresh", ready(4), 9),
        ]);
        evict_over_budget(&mut store, 9 * 1024 * 1024);
        assert!(!store.contains_key("old"), "the oldest goes");
        assert!(store.contains_key("fresh"), "the freshest stays");
        assert!(held(&store) <= 9 * 1024 * 1024);
    }

    /// **A picture in flight is never dropped.** Dropping it cancels nothing — nothing
    /// here can — it only forgets the request was made, so the next frame starts a
    /// second one *and* `images_in_flight` falls to zero, stopping the redraw loop that
    /// was keeping the frame alive until the picture arrived.
    #[test]
    fn a_picture_in_flight_is_never_dropped() {
        let mut store = store_of(vec![
            ("arriving", Fetched::Loading, 0),
            ("big", ready(8), 1),
        ]);
        evict_over_budget(&mut store, 0);
        assert!(store.contains_key("arriving"), "still in flight");
        assert!(!store.contains_key("big"), "the pixels go");
    }

    /// **A picture somebody else is holding is never dropped.** It is on screen. Losing
    /// it is not unsafe — the `Arc` keeps the pixels alive for whoever holds them —
    /// but the next `view` would find nothing, ask again, and the image would flicker
    /// through a placeholder on its way back to exactly where it was.
    #[test]
    fn a_picture_on_screen_is_never_dropped() {
        let onscreen = ready(8);
        // What a scene holding the image looks like from here: a second reference.
        let Fetched::Ready(handle) = &onscreen else {
            panic!("ready")
        };
        let _in_a_scene = handle.clone();

        let mut store = store_of(vec![("visible", onscreen, 0), ("stale", ready(8), 1)]);
        evict_over_budget(&mut store, 0);
        assert!(store.contains_key("visible"), "held elsewhere: it stays");
        assert!(!store.contains_key("stale"), "held only here: it goes");
    }

    /// When **everything** left is in flight or on screen, the sweep stops rather than
    /// spinning. Over budget is the right answer there: the alternative is dropping
    /// something being looked at.
    #[test]
    fn a_store_that_cannot_shrink_stops_rather_than_spins() {
        let onscreen = ready(8);
        let Fetched::Ready(handle) = &onscreen else {
            panic!("ready")
        };
        let _in_a_scene = handle.clone();

        let mut store = store_of(vec![
            ("visible", onscreen, 0),
            ("arriving", Fetched::Loading, 1),
        ]);
        evict_over_budget(&mut store, 0);
        assert_eq!(store.len(), 2, "nothing could go, and nothing hung");
    }

    /// The default is the reference\'s figure, and it is a **default**: a caller can set
    /// its own, including nothing at all.
    #[test]
    fn the_budget_is_the_callers() {
        assert_eq!(DEFAULT_IMAGE_CACHE_BYTES, 100 * 1024 * 1024);
        let before = image_cache_budget();
        set_image_cache_budget(7);
        assert_eq!(image_cache_budget(), 7);
        set_image_cache_budget(before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32) -> ImageData {
        ImageData::from_rgba(w, h, vec![0u8; (w * h * 4) as usize])
    }

    #[test]
    fn ids_are_unique_and_stable() {
        let a = img(1, 1);
        let b = img(1, 1);
        assert_ne!(a.id(), b.id());
        assert_eq!(a.id(), a.id());
        assert_ne!(a, b); // equality is by identity, not by pixels
    }

    #[test]
    fn fill_uses_the_whole_box_and_full_uv() {
        let (dst, uv) = BoxFit::Fill.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(dst, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn contain_letterboxes_preserving_aspect() {
        // A square source in a wide box → full height, centred horizontally.
        let (dst, uv) =
            BoxFit::Contain.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(dst, Rect::new(30.0, 0.0, 40.0, 40.0));
        assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn scale_fits_content_per_mode() {
        let src = Size::new(40.0, 20.0); // wide
        let dst = Size::new(200.0, 200.0);
        // Fill: per axis.
        assert_eq!(BoxFit::Fill.scale(src, dst), (5.0, 10.0));
        // Contain: the smaller factor (it fits) → uniform.
        assert_eq!(BoxFit::Contain.scale(src, dst), (5.0, 5.0));
        // Cover: the larger factor (it covers) → uniform.
        assert_eq!(BoxFit::Cover.scale(src, dst), (10.0, 10.0));
        // FitWidth / FitHeight follow a single axis, uniformly.
        assert_eq!(BoxFit::FitWidth.scale(src, dst), (5.0, 5.0));
        assert_eq!(BoxFit::FitHeight.scale(src, dst), (10.0, 10.0));
        // None does not scale; ScaleDown only ever shrinks.
        assert_eq!(BoxFit::None.scale(src, dst), (1.0, 1.0));
        assert_eq!(BoxFit::ScaleDown.scale(src, dst), (1.0, 1.0));
        // ScaleDown does shrink when the box is smaller.
        assert_eq!(
            BoxFit::ScaleDown.scale(src, Size::new(20.0, 20.0)),
            (0.5, 0.5)
        );
        // Degenerate source → neutral.
        assert_eq!(BoxFit::Cover.scale(Size::new(0.0, 10.0), dst), (1.0, 1.0));
    }

    #[test]
    fn cover_fills_the_box_and_crops_uv() {
        // A square source in a wide box → covers everything, crops vertically (UV).
        let (dst, uv) =
            BoxFit::Cover.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(dst, Rect::new(0.0, 0.0, 100.0, 40.0));
        // scale = max(10, 4) = 10 → a 100×100 image; visible = 40/100 = 0.4 of its height.
        assert_eq!(uv.width, 1.0);
        assert!((uv.height - 0.4).abs() < 1e-6);
        assert!((uv.y - 0.3).abs() < 1e-6);
    }
}
