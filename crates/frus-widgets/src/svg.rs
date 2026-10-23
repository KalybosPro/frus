//! [`SvgPicture`]: an SVG picture, drawn as vector paths at whatever size it is shown —
//! sharp on every screen and at every zoom, where a bitmap would blur.
//!
//! Sized like [`Image`](crate::Image): whatever you say, and failing that the picture's own
//! `width` and `height`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Color, Rect, Scene};
use frus_layout::{Dimension, Style};
use frus_svg::{Svg, SvgHandle};

use crate::image::State;
use crate::interaction::Status;
use crate::remote::{Remote, Store};
use crate::theme::Theme;
use crate::widget::Widget;

/// Parsed pictures, keyed by where their bytes live (`'static` bytes, as
/// [`frus_core::cached`] keys images) or by their text.
static BY_ADDRESS: Mutex<Option<HashMap<usize, Result<SvgHandle, String>>>> = Mutex::new(None);
static BY_TEXT: Mutex<Option<HashMap<u64, Result<SvgHandle, String>>>> = Mutex::new(None);

/// Pictures fetched from an address.
static FETCHED: Store<SvgHandle> = Store::new();

/// Fetches of pictures still on their way.
pub(crate) fn in_flight() -> usize {
    FETCHED.in_flight()
}

/// The most pictures parsed from text that are remembered. Text built at run time could
/// otherwise grow the store without end; past this, it starts again.
const TEXT_ENTRIES: usize = 256;

/// Parses once, and says in the console what went wrong or what will not be drawn.
fn parse(bytes: &[u8]) -> Result<SvgHandle, String> {
    match Svg::parse(bytes) {
        Ok(svg) => {
            if !svg.unsupported().is_empty() {
                log::warn!(
                    "frus: an SVG uses {} — not drawn yet; the rest of the picture is",
                    svg.unsupported().join(", ")
                );
            }
            Ok(Arc::new(svg))
        }
        Err(why) => {
            log::warn!("frus: {why}");
            Err(why.message().to_string())
        }
    }
}

/// **An SVG picture.**
///
/// ```ignore
/// SvgPicture::memory(include_bytes!("../assets/empty-inbox.svg")).width(160.0)
/// SvgPicture::network("https://example.com/logo.svg").height(32.0)
/// ```
///
/// Parsed once per process for a given source, whatever the number of frames that show
/// it. Paths, fills, strokes (caps, joins, dashes), gradients, group opacity and clip
/// paths are drawn; masks, filters, patterns, embedded bitmaps and text are not yet, and
/// the console says so when a picture uses them.
///
/// Needs the `svg` feature.
pub struct SvgPicture {
    source: Remote<SvgHandle>,
    width: Option<f32>,
    height: Option<f32>,
    fit: BoxFit,
    alignment: AlignmentGeometry,
    color: Option<Color>,
    opacity: f32,
    semantic_label: Option<String>,
    exclude_from_semantics: bool,
}

impl SvgPicture {
    /// A picture already parsed.
    pub fn new(svg: SvgHandle) -> Self {
        Self::from_remote(Remote::Ready(svg))
    }

    /// A picture from embedded bytes — what `include_bytes!` gives. Parsed once per
    /// process, keyed by the bytes' address as [`frus_core::cached`] keys images.
    pub fn memory(bytes: &'static [u8]) -> Self {
        let key = bytes.as_ptr() as usize;
        let mut guard = BY_ADDRESS.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard.get_or_insert_with(HashMap::new);
        let source = store.entry(key).or_insert_with(|| parse(bytes)).clone();
        Self::from_source(source)
    }

    /// A picture from SVG source text. Parsed once for the same text.
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

    /// **A picture fetched from `url`**, through the fetcher images use
    /// ([`frus_core::set_image_fetcher`], which the shell installs with the `net` feature).
    ///
    /// Fetched and parsed once per process for an address, whatever the number of frames
    /// that ask; a failure is remembered too. While it is on its way the widget takes the
    /// room given it and draws nothing, and [`state`](SvgPicture::state) says so — an
    /// application that wants a placeholder puts one there itself:
    ///
    /// ```ignore
    /// let logo = SvgPicture::network(url).height(32.0);
    /// match logo.state() {
    ///     ImageState::Ready => logo.boxed(),
    ///     _ => Skeleton::new().width(32.0).height(32.0).boxed(),
    /// }
    /// ```
    pub fn network(url: impl AsRef<str>) -> Self {
        Self::from_remote(FETCHED.get(url.as_ref(), parse))
    }

    fn from_source(source: Result<SvgHandle, String>) -> Self {
        Self::from_remote(match source {
            Ok(svg) => Remote::Ready(svg),
            Err(why) => Remote::Failed(why),
        })
    }

    fn from_remote(source: Remote<SvgHandle>) -> Self {
        Self {
            source,
            width: None,
            height: None,
            fit: BoxFit::Contain,
            alignment: AlignmentGeometry::Physical(Alignment::CENTER),
            color: None,
            opacity: 1.0,
            semantic_label: None,
            exclude_from_semantics: false,
        }
    }

    /// The width, in logical pixels; with no height, the height follows the picture's
    /// own ratio.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The height, in logical pixels; with no width, the width follows the picture's own
    /// ratio.
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

    /// How the picture fits its box; [`BoxFit::Contain`] by default.
    #[must_use]
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where the picture sits in a box it does not fill; centred by default.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// **Draws every shape in one colour**, keeping each shape's transparency: a
    /// single-colour icon recoloured to the theme. Without it the picture keeps its own
    /// colours.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Draws the picture at a fraction of its opacity.
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// What a screen reader says instead of the picture.
    #[must_use]
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }

    /// Leaves the picture out of what a screen reader walks: for decoration.
    #[must_use]
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    /// Why there is no picture, when the source could not be fetched or parsed.
    pub fn error(&self) -> Option<&str> {
        match &self.source {
            Remote::Failed(why) => Some(why),
            _ => None,
        }
    }

    /// The parsed picture, when there is one.
    pub fn svg(&self) -> Option<&SvgHandle> {
        match &self.source {
            Remote::Ready(svg) => Some(svg),
            _ => None,
        }
    }

    /// Whether the picture is here, on its way ([`SvgPicture::network`]), or will not come.
    pub fn state(&self) -> State<'_> {
        match &self.source {
            Remote::Ready(_) => State::Ready,
            Remote::Loading => State::Loading,
            Remote::Failed(why) => State::Failed(why),
        }
    }
}

impl<Msg> Widget<Msg> for SvgPicture {
    fn style(&self) -> Style {
        let length = |v: Option<f32>| v.map_or(Dimension::Auto, Dimension::Length);
        let Some(svg) = self.svg() else {
            // Nothing parsed (yet): no natural size, and only what the caller said.
            return Style {
                width: Dimension::Length(self.width.unwrap_or(0.0)),
                height: Dimension::Length(self.height.unwrap_or(0.0)),
                ..Default::default()
            };
        };
        let natural = svg.size();
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

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        if let Some(svg) = self.svg() {
            svg.paint(
                scene,
                bounds,
                self.fit,
                self.alignment,
                self.color,
                status.opacity * self.opacity,
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    const ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12"
        viewBox="0 0 24 12"><rect width="24" height="12" fill="#e53935"/></svg>"##;

    #[test]
    fn a_picture_takes_its_own_size_or_the_one_given() {
        let own = Widget::<()>::style(&SvgPicture::memory(ICON));
        assert_eq!(own.width, Dimension::Length(24.0));
        assert_eq!(own.height, Dimension::Length(12.0));
        let wide = Widget::<()>::style(&SvgPicture::memory(ICON).width(96.0));
        assert_eq!(wide.width, Dimension::Length(96.0));
        assert_eq!(wide.aspect_ratio, Some(2.0), "the height from the ratio");
    }

    #[test]
    fn the_same_bytes_are_parsed_once() {
        let a = SvgPicture::memory(ICON).svg().cloned().unwrap();
        let b = SvgPicture::memory(ICON).svg().cloned().unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let c = SvgPicture::string(std::str::from_utf8(ICON).unwrap())
            .svg()
            .cloned()
            .unwrap();
        let d = SvgPicture::string(std::str::from_utf8(ICON).unwrap())
            .svg()
            .cloned()
            .unwrap();
        assert!(Arc::ptr_eq(&c, &d));
    }

    #[test]
    fn it_paints_paths_in_its_box_and_recolours_on_request() {
        let mut scene = Scene::new();
        scene.set_clip(Rect::new(0.0, 0.0, 200.0, 200.0));
        let picture = SvgPicture::memory(ICON).color(Color::rgb(0.0, 0.0, 1.0));
        Widget::<()>::paint(
            &picture,
            Rect::new(10.0, 10.0, 48.0, 24.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let Primitive::Path {
            fill: Some(fill), ..
        } = &scene.primitives()[0]
        else {
            panic!("a path: {:?}", scene.primitives())
        };
        assert_eq!((fill.r, fill.g, fill.b), (0.0, 0.0, 1.0));
    }

    #[test]
    fn a_fetched_picture_takes_the_room_given_until_it_lands() {
        use std::sync::Mutex as StdMutex;
        type Deliver = Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>;
        static PENDING: StdMutex<Vec<Deliver>> = StdMutex::new(Vec::new());
        fn held(_: &str, deliver: Deliver) {
            PENDING.lock().unwrap().push(deliver);
        }
        let _guard = crate::remote::fetcher_lock();
        frus_core::set_image_fetcher(held);
        let url = "https://example.com/flag.svg";

        let waiting = SvgPicture::network(url).height(30.0);
        assert_eq!(waiting.state(), State::Loading);
        assert!(crate::images_in_flight() > 0, "the frames keep coming");
        let _ = SvgPicture::network(url);
        assert_eq!(PENDING.lock().unwrap().len(), 1, "asked once");
        let style = Widget::<()>::style(&waiting);
        assert_eq!(style.height, Dimension::Length(30.0));

        let deliver = PENDING.lock().unwrap().pop().unwrap();
        deliver(Ok(ICON.to_vec()));
        let arrived = SvgPicture::network(url).height(30.0);
        assert_eq!(arrived.state(), State::Ready);
        let style = Widget::<()>::style(&arrived);
        assert_eq!(
            style.aspect_ratio,
            Some(2.0),
            "now with the picture's ratio"
        );

        let missing = "https://example.com/missing.svg";
        let _ = SvgPicture::network(missing);
        let deliver = PENDING.lock().unwrap().pop().unwrap();
        deliver(Err("404".to_string()));
        assert_eq!(SvgPicture::network(missing).error(), Some("404"));
        let broken = "https://example.com/broken.svg";
        let _ = SvgPicture::network(broken);
        let deliver = PENDING.lock().unwrap().pop().unwrap();
        deliver(Ok(b"<svg".to_vec()));
        assert!(SvgPicture::network(broken).error().is_some(), "not an SVG");
    }

    #[test]
    fn a_broken_source_says_why_and_takes_only_the_room_given() {
        let broken = SvgPicture::string("<svg");
        assert!(broken.error().is_some());
        let style = Widget::<()>::style(&broken.size(10.0, 20.0));
        assert_eq!(style.width, Dimension::Length(10.0));
        assert_eq!(style.height, Dimension::Length(20.0));
    }
}
