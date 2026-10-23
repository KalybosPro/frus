//! [`Lottie`]: a Lottie animation, drawn as vector paths at whatever size it is shown, played
//! when it appears.
//!
//! Sized like [`Image`](crate::Image): whatever you say, and failing that the animation's own
//! `w` and `h`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Curve, Rect, Scene};
use frus_layout::{Dimension, Style};
use frus_lottie::{Animation, AnimationHandle};

use crate::image::State;
use crate::interaction::Status;
use crate::remote::{Remote, Store};
use crate::theme::Theme;
use crate::widget::Widget;

/// Parsed animations, keyed by where their bytes live (`'static` bytes) or by their text.
static BY_ADDRESS: Mutex<Option<HashMap<usize, Result<AnimationHandle, String>>>> =
    Mutex::new(None);
static BY_TEXT: Mutex<Option<HashMap<u64, Result<AnimationHandle, String>>>> = Mutex::new(None);

/// Animations fetched from an address.
static FETCHED: Store<AnimationHandle> = Store::new();

/// Fetches of animations still on their way.
pub(crate) fn in_flight() -> usize {
    FETCHED.in_flight()
}

/// The most animations parsed from text that are remembered; past this, it starts again.
const TEXT_ENTRIES: usize = 64;

/// How many plays a repeating animation is given: its value runs to this, read modulo one.
/// Ten thousand plays of a two-second loader is more than five hours, and the value is still
/// precise to a thousandth of a play.
const LOOPS: f32 = 10_000.0;

fn parse(bytes: &[u8]) -> Result<AnimationHandle, String> {
    match Animation::parse(bytes) {
        Ok(animation) => Ok(Arc::new(animation)),
        Err(why) => {
            log::warn!("frus: {why}");
            Err(why.message().to_string())
        }
    }
}

/// **A Lottie animation**, played when it appears.
///
/// ```ignore
/// Lottie::memory(include_bytes!("../assets/success.json")).size(120.0, 120.0)
/// Lottie::memory(LOADER).repeat(true)
/// Lottie::network("https://example.com/confetti.json")
/// ```
///
/// Parsed once per process for a given source. Shapes, fills, strokes, gradients, layer
/// opacity, masks and mattes are drawn; image assets and sweep gradients are not yet, and the
/// console says so when an animation uses them.
///
/// It plays once from its first frame when it appears, then rests on its last: [`repeat`]
/// loops it, [`reverse`] plays it backwards (back and forth when repeating), [`speed`] scales
/// its rate, and [`animate`]`(false)` holds it still. An application that drives it itself —
/// from a gesture, a scroll, its own animation — gives the moment with [`progress`].
///
/// Needs the `lottie` feature.
///
/// [`repeat`]: Lottie::repeat
/// [`reverse`]: Lottie::reverse
/// [`speed`]: Lottie::speed
/// [`animate`]: Lottie::animate
/// [`progress`]: Lottie::progress
pub struct Lottie {
    source: Remote<AnimationHandle>,
    width: Option<f32>,
    height: Option<f32>,
    fit: BoxFit,
    alignment: AlignmentGeometry,
    animate: bool,
    repeat: bool,
    reverse: bool,
    speed: f32,
    progress: Option<f32>,
    opacity: f32,
    semantic_label: Option<String>,
    exclude_from_semantics: bool,
}

impl Lottie {
    /// An animation already parsed.
    pub fn new(animation: AnimationHandle) -> Self {
        Self::from_remote(Remote::Ready(animation))
    }

    /// An animation from embedded bytes — what `include_bytes!` gives a `.json` Lottie file.
    /// Parsed once per process, keyed by the bytes' address.
    pub fn memory(bytes: &'static [u8]) -> Self {
        let key = bytes.as_ptr() as usize;
        let mut guard = BY_ADDRESS.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard.get_or_insert_with(HashMap::new);
        let source = store.entry(key).or_insert_with(|| parse(bytes)).clone();
        Self::from_source(source)
    }

    /// An animation from its JSON text. Parsed once for the same text.
    pub fn string(source: impl AsRef<str>) -> Self {
        use std::hash::{Hash, Hasher};
        let text = source.as_ref();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        let key = hasher.finish();
        let mut guard = BY_TEXT.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard.get_or_insert_with(HashMap::new);
        if store.len() >= TEXT_ENTRIES && !store.contains_key(&key) {
            store.clear();
        }
        let source = store
            .entry(key)
            .or_insert_with(|| parse(text.as_bytes()))
            .clone();
        Self::from_source(source)
    }

    /// **An animation fetched from `url`**, through the fetcher images use
    /// ([`frus_core::set_image_fetcher`], which the shell installs with the `net` feature).
    ///
    /// Fetched and parsed once per process for an address; a failure is remembered too.
    /// While it is on its way the widget takes the room given it and draws nothing, and
    /// [`state`](Lottie::state) says so. It **plays from its first frame when it arrives**,
    /// not from when the widget appeared: a fetch that took longer than the animation does
    /// not skip it.
    pub fn network(url: impl AsRef<str>) -> Self {
        Self::from_remote(FETCHED.get(url.as_ref(), parse))
    }

    fn from_source(source: Result<AnimationHandle, String>) -> Self {
        Self::from_remote(match source {
            Ok(animation) => Remote::Ready(animation),
            Err(why) => Remote::Failed(why),
        })
    }

    fn from_remote(source: Remote<AnimationHandle>) -> Self {
        Self {
            source,
            width: None,
            height: None,
            fit: BoxFit::Contain,
            alignment: AlignmentGeometry::Physical(Alignment::CENTER),
            animate: true,
            repeat: false,
            reverse: false,
            speed: 1.0,
            progress: None,
            opacity: 1.0,
            semantic_label: None,
            exclude_from_semantics: false,
        }
    }

    /// The width, in logical pixels; with no height, the height follows the animation's ratio.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The height, in logical pixels; with no width, the width follows the animation's ratio.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Both at once.
    #[must_use]
    pub fn size(self, width: f32, height: f32) -> Self {
        self.width(width).height(height)
    }

    /// How the animation fits its box; [`BoxFit::Contain`] by default.
    #[must_use]
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where the animation sits in a box it does not fill; centred by default.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// Whether it plays by itself; `true` by default. Held still, it shows its first frame
    /// (its last, reversed) unless [`progress`](Lottie::progress) says another.
    #[must_use]
    pub fn animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }

    /// Whether it starts again when it ends; `false` by default: it rests on its last frame.
    #[must_use]
    pub fn repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    /// Plays it backwards — and, when it repeats, back and forth.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Its rate: `2.0` twice as fast, `0.5` half. `1.0` by default.
    #[must_use]
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = if speed.is_finite() && speed > 0.0 {
            speed
        } else {
            1.0
        };
        self
    }

    /// **Shows the moment `progress`** (`0` the first frame, `1` the last) instead of playing:
    /// for an application that drives the animation itself.
    #[must_use]
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Draws it at a fraction of its opacity.
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// What a screen reader says instead of the animation.
    #[must_use]
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }

    /// Leaves it out of what a screen reader walks: for decoration.
    #[must_use]
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    /// Why there is no animation, when the source could not be fetched or parsed.
    pub fn error(&self) -> Option<&str> {
        match &self.source {
            Remote::Failed(why) => Some(why),
            _ => None,
        }
    }

    /// The parsed animation, when there is one.
    pub fn animation(&self) -> Option<&AnimationHandle> {
        match &self.source {
            Remote::Ready(animation) => Some(animation),
            _ => None,
        }
    }

    /// Whether the animation is here, on its way ([`Lottie::network`]), or will not come.
    pub fn state(&self) -> State<'_> {
        match &self.source {
            Remote::Ready(_) => State::Ready,
            Remote::Loading => State::Loading,
            Remote::Failed(why) => State::Failed(why),
        }
    }

    /// Whether the widget plays by itself, with something to play.
    fn plays(&self) -> bool {
        self.progress.is_none()
            && self.animate
            && self
                .animation()
                .is_some_and(|animation| animation.duration() > 0.0)
    }

    /// The moment shown, from the played value `value` (plays from `0`, one per play).
    fn moment(&self, value: f32) -> f32 {
        if let Some(progress) = self.progress {
            return progress;
        }
        if !self.plays() {
            return if self.reverse { 1.0 } else { 0.0 };
        }
        let (play, within) = if self.repeat {
            (value.floor(), value.fract())
        } else {
            (0.0, value.clamp(0.0, 1.0))
        };
        match (self.reverse, self.repeat) {
            (false, _) => within,
            (true, false) => 1.0 - within,
            // Back and forth: every second play goes the other way.
            (true, true) => {
                if play % 2.0 == 0.0 {
                    within
                } else {
                    1.0 - within
                }
            }
        }
    }
}

impl<Msg> Widget<Msg> for Lottie {
    fn style(&self) -> Style {
        let length = |v: Option<f32>| v.map_or(Dimension::Auto, Dimension::Length);
        let Some(animation) = self.animation() else {
            return Style {
                width: Dimension::Length(self.width.unwrap_or(0.0)),
                height: Dimension::Length(self.height.unwrap_or(0.0)),
                ..Default::default()
            };
        };
        let natural = animation.size();
        let ratio = if natural.height > 0.0 {
            natural.width / natural.height
        } else {
            1.0
        };
        match (self.width, self.height) {
            (None, None) => Style {
                width: Dimension::Length(natural.width),
                height: Dimension::Length(natural.height),
                ..Default::default()
            },
            (w, h) => Style {
                width: length(w),
                height: length(h),
                aspect_ratio: (w.is_none() || h.is_none()).then_some(ratio),
                ..Default::default()
            },
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// Playing, the value runs from nought — where it starts when the widget appears
    /// ([`Widget::anim_initial`]) — to one play's end, or to ten thousand plays when repeating.
    fn anim_target(&self) -> Option<f32> {
        self.plays()
            .then_some(if self.repeat { LOOPS } else { 1.0 })
    }

    fn anim_initial(&self) -> Option<f32> {
        self.plays().then_some(0.0)
    }

    fn anim_duration(&self) -> f32 {
        let one = self
            .animation()
            .map_or(0.0, |animation| animation.duration())
            / self.speed;
        if self.repeat {
            one * LOOPS
        } else {
            one
        }
    }

    /// One play every so long, evenly: the animation's own easing is in its keyframes.
    fn anim_curve(&self) -> Curve {
        Curve::Linear
    }

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        if let Some(animation) = self.animation() {
            animation.paint(
                scene,
                bounds,
                self.fit,
                self.alignment,
                self.moment(status.value),
                status.opacity * self.opacity,
            );
            for what in animation.unsupported() {
                log_once(what);
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        if self.exclude_from_semantics {
            return None;
        }
        let mut semantics = frus_core::SemanticsProperties::new(frus_core::Role::Image);
        if let Some(label) = self.semantic_label.as_deref() {
            semantics = semantics.label(label);
        }
        Some(semantics)
    }
}

/// Says once per process that an animation uses something not drawn.
fn log_once(what: &'static str) {
    static SAID: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut said = SAID.lock().unwrap_or_else(|e| e.into_inner());
    if !said.contains(&what) {
        said.push(what);
        log::warn!("frus: a Lottie animation uses {what}, which is not drawn yet");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    /// A red square moving from x = 20 to x = 80 over 60 frames at 30 fps: two seconds.
    const MOVING: &str = r#"{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":100,"h":100,"nm":"t","ddd":0,
      "assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"s","sr":1,"ip":0,"op":60,"st":0,"bm":0,
      "ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},
            "p":{"a":1,"k":[{"t":0,"s":[20,50,0],"i":{"x":[1],"y":[1]},"o":{"x":[0],"y":[0]}},
                            {"t":60,"s":[80,50,0]}]},
            "a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},
      "shapes":[{"ty":"gr","nm":"g","it":[
        {"ty":"rc","nm":"r","d":1,"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
        {"ty":"fl","nm":"f","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1},
        {"ty":"tr","p":{"a":0,"k":[0,0]},"a":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},
         "r":{"a":0,"k":0},"o":{"a":0,"k":100},"sk":{"a":0,"k":0},"sa":{"a":0,"k":0}}]}]}]}"#;

    /// The left edge of the square drawn at `value`.
    fn left_at(lottie: &Lottie, value: f32) -> f32 {
        let mut scene = Scene::new();
        scene.set_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
        let status = Status {
            value,
            ..Status::default()
        };
        Widget::<()>::paint(
            lottie,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            status,
            &Theme::default(),
            &mut scene,
        );
        let Some(Primitive::Path { path, .. }) = scene.primitives().first() else {
            panic!("a path, not in a layer: {:?}", scene.primitives())
        };
        path.verbs()
            .iter()
            .filter_map(|v| match *v {
                frus_core::PathVerb::MoveTo(p) | frus_core::PathVerb::LineTo(p) => Some(p.x),
                _ => None,
            })
            .fold(f32::MAX, f32::min)
    }

    #[test]
    fn it_takes_the_animations_size_or_the_one_given() {
        let own = Widget::<()>::style(&Lottie::string(MOVING));
        assert_eq!(own.width, Dimension::Length(100.0));
        let wide = Widget::<()>::style(&Lottie::string(MOVING).width(50.0));
        assert_eq!(wide.aspect_ratio, Some(1.0));
    }

    #[test]
    fn it_plays_once_from_the_start_when_it_appears() {
        let once = Lottie::string(MOVING);
        assert_eq!(Widget::<()>::anim_target(&once), Some(1.0));
        assert_eq!(Widget::<()>::anim_initial(&once), Some(0.0));
        assert!((Widget::<()>::anim_duration(&once) - 2.0).abs() < 1e-6);
        assert!((left_at(&once, 0.0) - 10.0).abs() < 0.5);
        assert!(
            (left_at(&once, 0.5) - 39.5).abs() < 1.0,
            "{}",
            left_at(&once, 0.5)
        );
        // Past the end it rests on the last frame.
        assert!((left_at(&once, 7.0) - left_at(&once, 1.0)).abs() < 1e-3);
    }

    #[test]
    fn it_loops_and_goes_back_and_forth_when_asked() {
        let looping = Lottie::string(MOVING).repeat(true);
        assert_eq!(Widget::<()>::anim_target(&looping), Some(LOOPS));
        assert!((left_at(&looping, 3.25) - left_at(&looping, 0.25)).abs() < 1e-3);
        let swinging = Lottie::string(MOVING).repeat(true).reverse(true);
        // The second play runs backwards: a quarter in is three quarters of the way.
        assert!((left_at(&swinging, 1.25) - left_at(&swinging, 0.75)).abs() < 1e-3);
        let faster = Lottie::string(MOVING).speed(2.0);
        assert!((Widget::<()>::anim_duration(&faster) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_driven_animation_shows_its_moment_and_does_not_play() {
        let driven = Lottie::string(MOVING).progress(1.0);
        assert_eq!(Widget::<()>::anim_target(&driven), None);
        assert!((left_at(&driven, 0.0) - left_at(&Lottie::string(MOVING), 1.0)).abs() < 1e-3);
        let still = Lottie::string(MOVING).animate(false);
        assert_eq!(Widget::<()>::anim_target(&still), None);
        assert!((left_at(&still, 0.0) - 10.0).abs() < 0.5);
    }

    /// **A fetched animation waits, then plays from its start when it lands**: nothing
    /// plays while it is on its way, so a fetch longer than the animation does not skip it.
    #[test]
    fn a_fetched_animation_plays_from_its_start_when_it_arrives() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Mutex as StdMutex;
        type Deliver = Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>;
        static PENDING: StdMutex<Option<Deliver>> = StdMutex::new(None);
        static ASKED: AtomicBool = AtomicBool::new(false);
        fn held(_: &str, deliver: Deliver) {
            ASKED.store(true, Ordering::SeqCst);
            *PENDING.lock().unwrap() = Some(deliver);
        }
        let _guard = crate::remote::fetcher_lock();
        frus_core::set_image_fetcher(held);
        let url = "https://example.com/moving.json";

        let mut rt = crate::runtime::Runtime::default();
        let waiting = Lottie::network(url).size(100.0, 100.0);
        assert!(ASKED.load(Ordering::SeqCst));
        assert_eq!(waiting.state(), State::Loading);
        assert!(crate::images_in_flight() > 0, "the frames keep coming");
        // Three seconds on its way: longer than the animation's two.
        for _ in 0..6 {
            rt.advance_values::<()>(&waiting, 0.5);
        }
        let style = Widget::<()>::style(&waiting);
        assert_eq!(
            style.width,
            Dimension::Length(100.0),
            "the room it was given"
        );

        let deliver = PENDING.lock().unwrap().take().unwrap();
        deliver(Ok(MOVING.as_bytes().to_vec()));
        let arrived = Lottie::network(url).size(100.0, 100.0);
        assert_eq!(arrived.state(), State::Ready);
        assert!(rt.advance_values::<()>(&arrived, 0.5), "playing");
        let value = rt.value(crate::WidgetId::ROOT);
        assert!(
            (value - 0.25).abs() < 1e-6,
            "a quarter in, not at the end: {value}"
        );
        assert!((left_at(&arrived, value) - 24.75).abs() < 1.0);
    }

    #[test]
    fn a_broken_source_says_why() {
        let broken = Lottie::string("{");
        assert!(broken.error().is_some());
        assert_eq!(Widget::<()>::anim_target(&broken), None);
    }
}
