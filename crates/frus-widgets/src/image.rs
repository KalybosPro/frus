//! [`Image`]: displays a bitmap image ([`frus_core::ImageHandle`]), fitted according to
//! a [`BoxFit`] — aspect preserved, letterboxed or cropped. The image is uploaded
//! **once** then cached by the renderer, keyed on the `ImageData`'s identity.
//!
//! **How big is it?** Whatever you say, and failing that whatever the bitmap is. The
//! reference's rule, and the only one that lets an image be shown by someone who does not
//! already know its pixel dimensions:
//!
//! | given | box |
//! |---|---|
//! | width and height | that box |
//! | one of the two | the other from the image's own ratio |
//! | neither | the image's own size |

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Color, Curve, ImageHandle, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::skeleton::Skeleton;
use crate::theme::Theme;
use crate::widget::Widget;

/// Turns file bytes into pixels, through whichever decoder this build carries.
#[cfg(feature = "images")]
fn decode(bytes: &[u8]) -> Result<frus_core::ImageData, String> {
    frus_image::decode(bytes).map_err(|e| e.message().to_string())
}

/// Without the `images` feature there is no decoder, and saying so is the whole job.
/// It must not panic: an application that dropped the feature deliberately, or a test
/// binary built with `--no-default-features`, is not a broken program.
#[cfg(not(feature = "images"))]
fn decode(_bytes: &[u8]) -> Result<frus_core::ImageData, String> {
    Err("no image decoder: this build dropped the `images` feature".to_string())
}

/// Where an [`Image`]'s pixels are: here, on their way, or never coming.
///
/// Three and not two, because *not here yet* and *will never be here* are different
/// answers and an interface shows them differently — one is a placeholder, the other is
/// a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State<'a> {
    /// The pixels are here.
    Ready,
    /// In flight; a later frame will have them. Only [`Image::network`] reports this.
    Loading,
    /// They will not arrive, and this is why.
    Failed(&'a str),
}

/// The same three, owning what they carry.
enum Source {
    Ready(ImageHandle),
    Loading,
    Failed(String),
}

/// How long an image takes to cross over its placeholder — the reference's own figure
/// for `FadeInImage` (`fade_in_image.dart:37`).
pub const IMAGE_FADE: f32 = 0.7;

/// **What stands in for an image while it is on its way.**
///
/// Not [`crate::Placeholder`], which is the design tool's crossed box: this one is the
/// slot [`Image::placeholder`] fills.
///
/// Three, because they are the three things anybody puts there: a flat colour, the
/// shimmering block a list of thumbnails wants, or a smaller picture — a blurred
/// thumbnail, a low-resolution copy, the one already in the cache.
///
/// It is **not a widget slot**, and that is deliberate: [`Image`] carries no message type
/// (`impl<Msg> Widget<Msg> for Image`), which is what lets one be built once and dropped
/// into any tree. Taking an arbitrary child would make it `Image<Msg>` and cost that
/// everywhere, to buy a fourth case nobody has asked for.
pub enum ImagePlaceholder {
    /// A flat colour.
    Color(Color),
    /// The shimmering block, at whatever shape it was built with.
    Skeleton(Skeleton),
    /// Another picture: a blur, a thumbnail, the cached copy.
    Image(Box<Image>),
}

impl From<Color> for ImagePlaceholder {
    fn from(color: Color) -> Self {
        ImagePlaceholder::Color(color)
    }
}

impl From<Skeleton> for ImagePlaceholder {
    fn from(skeleton: Skeleton) -> Self {
        ImagePlaceholder::Skeleton(skeleton)
    }
}

impl From<Image> for ImagePlaceholder {
    fn from(image: Image) -> Self {
        ImagePlaceholder::Image(Box::new(image))
    }
}

impl ImagePlaceholder {
    /// Draws the stand-in into `bounds` at `opacity`.
    fn paint(&self, bounds: Rect, status: &Status, theme: &Theme, scene: &mut Scene) {
        match self {
            ImagePlaceholder::Color(color) => scene.fill_rect(bounds, color.fade(status.opacity)),
            // Both of these are widgets already, and both are free of a message type, so
            // they are painted where they stand rather than mounted as children. A child
            // would have to be laid out, and the whole of what a placeholder wants is the
            // box the image was going to have.
            ImagePlaceholder::Skeleton(skeleton) => {
                Widget::<()>::paint(skeleton, bounds, *status, theme, scene)
            }
            ImagePlaceholder::Image(image) => {
                Widget::<()>::paint(image.as_ref(), bounds, *status, theme, scene)
            }
        }
    }
}

/// An image, fitted by `fit` into whatever box it ends up with.
pub struct Image {
    /// The pixels, why there are none, or that they are still on their way.
    source: Source,
    width: Option<f32>,
    height: Option<f32>,
    fit: BoxFit,
    tint: Option<Color>,
    opacity: f32,
    alignment: AlignmentGeometry,
    /// What a screen reader is told this picture is; see [`Image::semantic_label`].
    semantic_label: Option<String>,
    /// Whether to leave it out of the tree a screen reader walks.
    exclude_from_semantics: bool,
    /// Whether the picture is **mirrored** in a right-to-left reading direction.
    match_text_direction: bool,
    /// What stands in while the pixels are on their way; `None` leaves the box empty,
    /// which is what an image has always done.
    placeholder: Option<ImagePlaceholder>,
    /// How long the picture takes to cross over the placeholder.
    fade: f32,
    fade_curve: Curve,
}

impl Image {
    /// An image at **its own size**, fitted with [`BoxFit::Contain`] by default.
    pub fn new(image: ImageHandle) -> Self {
        Self::from_source(Source::Ready(image))
    }

    /// An image **decoded from embedded bytes** — what `include_bytes!` gives, and the
    /// answer to "how do I just show my logo".
    ///
    /// ```ignore
    /// Image::memory(include_bytes!("../assets/logo.png")).width(96.0)
    /// ```
    ///
    /// The bytes are decoded **once per process**, not once per frame. A view is rebuilt
    /// every frame and decoding a PNG is not free, so the result is kept in a shared
    /// store keyed by where the bytes live — see [`frus_core::cached`] for why an
    /// address is the right key and why the `'static` bound is what makes it sound.
    ///
    /// **Failure paints nothing.** A file that is not an image does not become one on the
    /// next frame, so the failure is remembered rather than retried, and the widget
    /// occupies whatever box it was given without drawing into it. That is the
    /// reference's behaviour for an image with no error widget supplied, and
    /// [`Image::error`] is how an application asks what went wrong.
    ///
    /// Needs the `images` feature, which is on by default. Without it this reports a
    /// missing decoder rather than panicking — the same rule the bundled fonts follow.
    pub fn memory(bytes: &'static [u8]) -> Self {
        Self::from_source(match frus_core::cached(bytes, decode) {
            Ok(handle) => Source::Ready(handle),
            Err(why) => Source::Failed(why),
        })
    }

    /// An image **fetched over the network**, decoded and shown when it arrives.
    ///
    /// ```ignore
    /// Image::network("https://example.com/ada.png").width(240.0)
    /// ```
    ///
    /// The **first** frame that asks starts the request and reports [`State::Loading`];
    /// every later frame is a lookup. That is what makes this safe to write in a view,
    /// which runs sixty times a second — a fetch per frame would be sixty requests for
    /// one picture. The interface keeps drawing while anything is in flight, so the
    /// frame that shows the picture does happen without the application arranging it.
    ///
    /// There is no `loading` or `error` **slot** here, and that is the framework's model
    /// rather than a gap. A view is a function of state, and this state is readable:
    /// [`Image::state`] answers, and the application writes the `match` it would have
    /// written anyway.
    ///
    /// ```ignore
    /// let photo = Image::network(&url);
    /// match photo.state() {
    ///     State::Loading => CircularProgressIndicator::new().boxed(),
    ///     State::Failed(why) => text(why).boxed(),
    ///     State::Ready => photo.width(240.0).boxed(),
    /// }
    /// ```
    ///
    /// A closure stored in the widget would say the same thing in a place the
    /// application cannot see it, and would need the widget to carry the message type
    /// for the sake of a branch the view can already take.
    ///
    /// Needs a registered fetcher — [`frus_core::set_image_fetcher`] — which the shell
    /// installs on the way up when built with the `net` feature. Without one this fails
    /// with a message saying so, rather than waiting for ever.
    pub fn network(url: impl AsRef<str>) -> Self {
        Self::from_source(match frus_core::fetched(url.as_ref(), decode) {
            frus_core::Fetched::Loading => Source::Loading,
            frus_core::Fetched::Ready(handle) => Source::Ready(handle),
            frus_core::Fetched::Failed(why) => Source::Failed(why),
        })
    }

    /// Where this image has got to: here, on its way, or never coming.
    ///
    /// An embedded image is only ever [`State::Ready`] or [`State::Failed`] — there is
    /// nothing to wait for. [`Image::network`] is the one that can be loading.
    pub fn state(&self) -> State<'_> {
        match &self.source {
            Source::Ready(_) => State::Ready,
            Source::Loading => State::Loading,
            Source::Failed(why) => State::Failed(why),
        }
    }

    /// Why there are no pixels, if there are none.
    ///
    /// An application that wants to put something else in the gap — a placeholder, a
    /// message, a retry — asks here and builds it itself. That is a `match` in the view
    /// rather than a closure the widget stores, which is the honest shape while the
    /// pixels are something the application already holds.
    ///
    /// An image still **loading** has no error: it has not failed, it has not arrived.
    /// [`Image::state`] is the one that tells the three apart.
    pub fn error(&self) -> Option<&str> {
        match &self.source {
            Source::Failed(why) => Some(why),
            _ => None,
        }
    }

    fn from_source(source: Source) -> Self {
        Self {
            source,
            width: None,
            height: None,
            fit: BoxFit::Contain,
            tint: None,
            opacity: 1.0,
            alignment: AlignmentGeometry::Physical(Alignment::CENTER),
            semantic_label: None,
            exclude_from_semantics: false,
            match_text_direction: false,
            placeholder: None,
            fade: IMAGE_FADE,
            fade_curve: Curve::ease_out(),
        }
    }

    /// Sets the width, in logical pixels. With no height, the height follows from the
    /// image's own ratio.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the height, in logical pixels. With no width, the width follows from the
    /// image's own ratio.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Both at once.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.width(width).height(height)
    }

    /// Changes the fit mode.
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where the image sits in a box it does not fill, and which part of it survives a
    /// box it overflows. Centre by default, as in the reference.
    ///
    /// It is the same anchor either way round: aligning to the top means the top of the
    /// box for a letterboxed image and the top of the *image* for a cropped one, which
    /// is the answer a photograph of a person wants in both cases.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// A multiplied tint, white leaving it unchanged — for bitmap icons, say.
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Draws the image at a fraction of its opacity, `1.0` being fully opaque.
    ///
    /// Unlike a group opacity this needs no layer: an image is one primitive, so the
    /// fade goes into the tint it is already multiplied by.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// What a screen reader says instead of the picture.
    ///
    /// A picture with no label is a picture nobody reading by ear can see, and until
    /// this existed **every** image in every application was one: the vocabulary had
    /// [`frus_core::Role::Image`] in it and nothing ever emitted it.
    ///
    /// Say what the picture *is*, not that it is a picture — a reader is already told
    /// the role. If it carries no meaning of its own, do not label it: reach for
    /// [`exclude_from_semantics`](Image::exclude_from_semantics) instead.
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }

    /// Leaves the image out of the tree a screen reader walks.
    ///
    /// The right answer for **decoration** — a divider, a texture, a shape behind a
    /// heading. Announcing those interrupts a reader with something that was never
    /// meant to be read, and an empty label would still announce the role. This is the
    /// reference's `excludeFromSemantics`, and like it, it wins over any label given.
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    /// Mirrors the image horizontally when the reading direction is right-to-left.
    ///
    /// **Off** by default, which is the reference's default and the right one: an image
    /// is a picture rather than a run of text, and a photograph of a person does not
    /// want to be flipped because the interface is in Arabic. It is a per-image
    /// decision, so it is a per-image switch.
    ///
    /// Turn it on for a picture that **points**: an arrow meaning *forward*, a
    /// speech bubble with a tail, a hand indicating the next step. Those follow the
    /// direction the reader's eye travels, and in RTL that is the other way round.
    pub fn match_text_direction(mut self, match_direction: bool) -> Self {
        self.match_text_direction = match_direction;
        self
    }

    /// **What stands in while the pixels are on their way**, and what the picture crosses
    /// over when they arrive.
    ///
    /// Takes a colour, a [`Skeleton`], or another [`Image`] — see [`ImagePlaceholder`]. Without
    /// one the box stays empty and the picture appears in a single frame, which is what an
    /// image has always done here and what a list of thumbnails should not do.
    ///
    /// ```ignore
    /// Image::network(url).width(120.0).placeholder(Skeleton::new().height(90.0))
    /// ```
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<ImagePlaceholder>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// How long the crossing takes, in seconds. [`IMAGE_FADE`] — the reference's 700 ms —
    /// unless a caller says otherwise. `0.0` is a hard cut with a placeholder before it.
    #[must_use]
    pub fn fade_duration(mut self, seconds: f32) -> Self {
        self.fade = seconds.max(0.0);
        self
    }

    /// The crossing's curve; [`Curve::ease_out`](Curve::ease_out) by
    /// default, so the picture arrives quickly and settles.
    #[must_use]
    pub fn fade_curve(mut self, curve: Curve) -> Self {
        self.fade_curve = curve;
        self
    }

    /// Whether the picture is here.
    fn ready(&self) -> bool {
        matches!(self.source, Source::Ready(_))
    }

    /// The box this asks for, given the bitmap's own size.
    fn box_style(&self) -> Style {
        // Nothing decoded: there is no natural size to fall back on, so an image given
        // no measurements takes no room at all. Anything the caller *did* say is still
        // honoured, which is what keeps a layout from jumping when one asset is broken.
        let Source::Ready(image) = &self.source else {
            return Style {
                width: self.width.map_or(Dimension::Length(0.0), Dimension::Length),
                height: self
                    .height
                    .map_or(Dimension::Length(0.0), Dimension::Length),
                ..Default::default()
            };
        };
        let natural = image.size();
        let ratio = if natural.height > 0.0 {
            natural.width / natural.height
        } else {
            1.0
        };
        match (self.width, self.height) {
            (Some(w), Some(h)) => Style {
                width: Dimension::Length(w),
                height: Dimension::Length(h),
                ..Default::default()
            },
            // One side and a ratio: the layout engine derives the other, which is the
            // reference's rule and the reason an image can be given a width alone.
            (Some(w), None) => Style {
                width: Dimension::Length(w),
                aspect_ratio: Some(ratio),
                ..Default::default()
            },
            (None, Some(h)) => Style {
                height: Dimension::Length(h),
                aspect_ratio: Some(ratio),
                ..Default::default()
            },
            (None, None) => Style {
                width: Dimension::Length(natural.width),
                height: Dimension::Length(natural.height),
                ..Default::default()
            },
        }
    }
}

impl<Msg> Widget<Msg> for Image {
    fn style(&self) -> Style {
        self.box_style()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        self.paint_tinted(bounds, status, theme, scene, None);
    }

    /// **Where the crossing is going**: all the way over once the pixels are here, and
    /// nowhere at all until then.
    ///
    /// Only claimed when there is a placeholder, so an image without one costs the runtime
    /// nothing and keeps the behaviour it has always had.
    fn anim_target(&self) -> Option<f32> {
        self.placeholder
            .as_ref()
            .map(|_| if self.ready() { 1.0 } else { 0.0 })
    }

    /// **The crossing has a duration in one direction only.**
    ///
    /// Coming in it takes its time: that is the whole point. Going back — a source
    /// replaced at the same place in the tree, so the widget is loading again — it is
    /// instant, because there is nothing left to cross *from*. The pixels of the picture
    /// that was there are already gone by the time this is asked; tweening down would fade
    /// the placeholder **in** over three quarters of a second while nothing else was
    /// drawn, which reads as a stall rather than as a change.
    ///
    /// This is also what answers *do not restart on a rebuild*. The value lives against
    /// the widget's place in the tree and survives every rebuild, so nothing restarts
    /// while the source stands still; and a source that changes passes through *not
    /// ready*, which resets it to nought in one frame and lets the next arrival cross
    /// properly. No record of which image this timeline is about is needed, because the
    /// journey through the middle is the record.
    fn anim_duration(&self) -> f32 {
        match self.ready() {
            true => self.fade,
            false => 0.0,
        }
    }

    fn anim_curve(&self) -> Curve {
        self.fade_curve.clone()
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

impl Image {
    /// The paint, with a **tint the caller supplies** rather than the one on the widget —
    /// what [`ImageIcon`] needs, since it holds an `Image` by reference and cannot rebuild
    /// one to colour it.
    pub(crate) fn paint_tinted(
        &self,
        bounds: Rect,
        status: Status,
        theme: &Theme,
        scene: &mut Scene,
        over: Option<Color>,
    ) {
        // **How far across the crossing is**, and it is only asked for when there is
        // something to cross over from. Without a placeholder the picture is drawn at its
        // own opacity as it always was, so nothing already written changes.
        let across = match self.placeholder {
            Some(_) => status.value.clamp(0.0, 1.0),
            None => 1.0,
        };
        if let Some(placeholder) = &self.placeholder {
            if across < 1.0 {
                let under = Status {
                    opacity: status.opacity * (1.0 - across),
                    ..status
                };
                placeholder.paint(bounds, &under, theme, scene);
            }
        }
        // The **alignment** stays physical whatever the direction. An image is a
        // picture rather than a run of text: a portrait aligned to the top of its
        // crop wants the top of its crop in every language. Mirroring is the separate,
        // opt-in question below, which is how the reference splits it too.
        // Nothing decoded, nothing drawn: the box stays empty rather than showing a
        // stand-in nobody asked for. `Image::error` is how an application finds out.
        let Source::Ready(image) = &self.source else {
            return;
        };
        let align = self.alignment.resolve(frus_core::TextDirection::Ltr);
        let (dst, mut uv) = self.fit.apply_aligned(image.size(), bounds, align);
        // A mirror costs nothing but a sign. The shader reads
        // `uv.xy + unit_pos * uv.zw` with `unit_pos` running 0..1, so a **negative**
        // width walks the same span backwards — which is the reference's "scaling
        // factor of -1 in the horizontal direction", without a transform, a layer, or
        // a second copy of the pixels.
        if self.match_text_direction && theme.direction == frus_core::TextDirection::Rtl {
            uv = Rect::new(uv.x + uv.width, uv.y, -uv.width, uv.height);
        }
        let tint = over
            .or(self.tint)
            .unwrap_or(Color::WHITE)
            .fade(status.opacity * self.opacity * across);
        scene.draw_image(image, dst, uv, tint);
    }
}

/// **A picture used where an icon would go**: a brand mark, a flag, a custom glyph that is
/// artwork rather than a path.
///
/// It is an icon in every way that matters to the layout and the theme — the ambient size
/// from [`IconTheme`](crate::IconTheme), the same square box, the same place in a row of
/// them — and a picture in the one way that matters to the painter. So it sits in an
/// [`IconButton`](crate::IconButton), a list tile's leading slot or an app bar action
/// without any of them knowing.
///
/// **Untinted by default**, which is the one place it parts from [`crate::Icon`]. An icon
/// is a silhouette and takes the foreground colour; a picture usually has colours of its
/// own, and a brand mark flattened to `on_surface` the first time an application themes its
/// icons is a brand mark nobody recognises. [`ImageIcon::color`] is for the case that wants
/// it — a monochrome glyph shipped as a bitmap, which is why the reference's own honours
/// `IconTheme.color` at all.
///
/// ```ignore
/// ImageIcon::new(Image::memory(include_bytes!("../assets/mark.png")))
/// ```
pub struct ImageIcon {
    image: Image,
    size: Option<f32>,
    color: Option<Color>,
}

impl ImageIcon {
    /// A picture at the ambient icon size, in its own colours.
    pub fn new(image: Image) -> Self {
        Self {
            image,
            size: None,
            color: None,
        }
    }

    /// The square's side, in logical pixels. Outranks the theme.
    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// **Tints** the picture, the way an icon is coloured.
    ///
    /// Multiplied into the pixels, so it is only a colour for artwork that is white or
    /// grey to begin with — which is what a bitmap glyph is. A photograph tinted red comes
    /// out a red photograph.
    ///
    /// Say it with [`crate::Icon`]'s own default and the picture follows the icon theme:
    /// `image_icon.color(theme.widgets.icon.color.unwrap_or(theme.on_surface))`. It is not
    /// done for you, for the reason in the type's own documentation.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The side actually drawn: `caller ?? theme ?? the grid`. The same chain
    /// [`crate::Icon`] answers, so a picture and a path line up in one row.
    fn resolved_size(&self, theme: Option<&Theme>) -> f32 {
        self.size
            .or_else(|| theme.and_then(|t| t.widgets.icon.size))
            .unwrap_or(crate::icons::GRID)
    }

    fn sized(&self, side: f32) -> Style {
        Style {
            width: Dimension::Length(side),
            height: Dimension::Length(side),
            ..Default::default()
        }
    }

    /// Draws the picture into a square somebody else chose — what
    /// [`IconButton`](crate::IconButton) needs, since a button sizes its own mark and a
    /// picture must be the same size as the path beside it.
    pub(crate) fn paint_at(&self, square: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        self.image
            .paint_tinted(square, status, theme, scene, self.color);
    }
}

impl<Msg> Widget<Msg> for ImageIcon {
    fn style(&self) -> Style {
        self.sized(self.resolved_size(None))
    }

    /// The theme has a say in the **size**, not only the colour, exactly as it does for a
    /// path icon — so an app bar that makes its glyphs smaller makes this smaller too.
    fn style_themed(&self, theme: &Theme) -> Style {
        self.sized(self.resolved_size(Some(theme)))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // The square the icon would have had, centred in whatever box the layout gave —
        // the same two lines `Icon` uses, which is what keeps a picture and a path level
        // in one row.
        let side = self.resolved_size(Some(theme));
        let square = Rect::new(
            bounds.x + (bounds.width - side) * 0.5,
            bounds.y + (bounds.height - side) * 0.5,
            side,
            side,
        );
        self.paint_at(square, status, theme, scene);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// The crossing is the picture's, forwarded — a brand mark fetched over the network
    /// fades in behind an icon button exactly as it would anywhere else.
    fn anim_target(&self) -> Option<f32> {
        Widget::<()>::anim_target(&self.image)
    }

    fn anim_duration(&self) -> f32 {
        Widget::<()>::anim_duration(&self.image)
    }

    fn anim_curve(&self) -> Curve {
        Widget::<()>::anim_curve(&self.image)
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        Widget::<()>::semantics(&self.image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;
    use frus_core::{ImageData, Primitive};

    fn handle(w: u32, h: u32) -> ImageHandle {
        ImageData::from_rgba(w, h, vec![255u8; (w * h * 4) as usize]).into_handle()
    }

    /// Renders one image on its own and returns `(the image tints, the flat fills)`.
    fn painted(image: Image, value: f32) -> (Vec<Color>, Vec<Color>) {
        let mut runtime = Runtime::default();
        runtime.set_value(crate::interaction::WidgetId::ROOT, value);
        let ui: crate::Ui<()> = crate::ui::build_ui(
            &image,
            frus_core::Size::new(100.0, 100.0),
            &runtime,
            &Theme::default(),
        );
        let mut images = Vec::new();
        let mut fills = Vec::new();
        fn walk(primitives: &[Primitive], images: &mut Vec<Color>, fills: &mut Vec<Color>) {
            for p in primitives {
                match p {
                    Primitive::Image { tint, .. } => images.push(*tint),
                    Primitive::Rect { color, .. } => fills.push(*color),
                    Primitive::Layer { primitives, .. } => walk(primitives, images, fills),
                    _ => {}
                }
            }
        }
        walk(ui.scene().primitives(), &mut images, &mut fills);
        (images, fills)
    }

    /// **An image with no placeholder is drawn exactly as it always was.** The crossing is
    /// only asked for when there is something to cross over from, so nothing already
    /// written acquired an animation or a frame of transparency.
    #[test]
    fn an_image_with_no_placeholder_is_untouched() {
        let image = Image::new(handle(8, 8)).size(50.0, 50.0);
        assert_eq!(Widget::<()>::anim_target(&image), None);
        // Even asked at nought — which is what a runtime that had never seen it would
        // answer — the picture is opaque.
        let (images, _) = painted(image, 0.0);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].a, 1.0);
    }

    /// **Half way across, both are drawn and neither is whole.** That is the crossing: the
    /// placeholder is on its way out at the same rate the picture is on its way in, so the
    /// box is never empty and never shows two solid things at once.
    #[test]
    fn half_way_across_both_are_drawn() {
        let stand_in = Color::rgb(0.9, 0.1, 0.1);
        let image = Image::new(handle(8, 8))
            .size(50.0, 50.0)
            .placeholder(stand_in);
        let (images, fills) = painted(image, 0.5);
        assert_eq!(images.len(), 1, "the picture");
        assert!(
            (images[0].a - 0.5).abs() < 1e-3,
            "half faded in: {:?}",
            images[0].a
        );
        let under = fills
            .iter()
            .find(|c| (c.r, c.g, c.b) == (stand_in.r, stand_in.g, stand_in.b))
            .expect("the placeholder");
        assert!(
            (under.a - 0.5).abs() < 1e-3,
            "half faded out: {:?}",
            under.a
        );
    }

    /// **At rest the placeholder is not drawn at all** — not drawn at nought opacity,
    /// which would still be a primitive per image on every frame of a list of thumbnails.
    #[test]
    fn at_rest_only_the_picture_is_drawn() {
        let stand_in = Color::rgb(0.9, 0.1, 0.1);
        let image = Image::new(handle(8, 8))
            .size(50.0, 50.0)
            .placeholder(stand_in);
        let (images, fills) = painted(image, 1.0);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].a, 1.0);
        assert!(
            !fills
                .iter()
                .any(|c| (c.r, c.g, c.b) == (stand_in.r, stand_in.g, stand_in.b)),
            "no placeholder once it is over"
        );
    }

    /// **The crossing has a duration in one direction only.**
    ///
    /// Coming in it takes its time. Going back it is instant, because there is nothing
    /// left to cross *from*: the pixels of the picture that was there are gone by then, so
    /// a tween down would fade the placeholder **in** over three quarters of a second with
    /// nothing else on screen — a stall rather than a change.
    ///
    /// It is also what makes a source swapped at one place in the tree do the right thing
    /// without anyone recording which picture the timeline was about: the swap passes
    /// through *not ready*, which resets it in one frame, and the next arrival crosses
    /// properly.
    #[test]
    fn the_crossing_runs_one_way() {
        let ready = Image::new(handle(8, 8)).placeholder(Color::WHITE);
        assert_eq!(Widget::<()>::anim_target(&ready), Some(1.0));
        assert_eq!(Widget::<()>::anim_duration(&ready), IMAGE_FADE);

        let waiting = Image::network("https://example.invalid/x.png").placeholder(Color::WHITE);
        assert_eq!(Widget::<()>::anim_target(&waiting), Some(0.0));
        assert_eq!(
            Widget::<()>::anim_duration(&waiting),
            0.0,
            "back to the placeholder in one frame"
        );
    }

    /// **A rebuild does not restart it.** A view is rebuilt every frame here, so a
    /// crossing that began again each time would never finish — the picture would sit at
    /// its first step for ever.
    ///
    /// Nothing in the widget arranges that: the value lives against the widget's place in
    /// the tree and the target does not move while the source stands still, so the runtime
    /// carries it. Worth a test because it is the property the whole design turns on.
    #[test]
    fn a_rebuild_does_not_restart_the_crossing() {
        let image = || {
            Image::new(handle(8, 8))
                .size(50.0, 50.0)
                .placeholder(Color::WHITE)
        };
        let mut runtime = Runtime::default();
        // Mount: adopts, since there was no placeholder shown before the widget existed.
        runtime.advance_values::<()>(&image(), 0.0);
        let mounted = runtime.value(crate::interaction::WidgetId::ROOT);
        assert_eq!(mounted, 1.0, "an image already here does not fade in");

        // Ten rebuilds of the same view, a frame apart.
        for _ in 0..10 {
            runtime.advance_values::<()>(&image(), 1.0 / 60.0);
        }
        assert_eq!(
            runtime.value(crate::interaction::WidgetId::ROOT),
            1.0,
            "and it has not gone back to the beginning"
        );
    }

    /// **A picture takes the ambient icon size**, the same chain a path icon answers, so
    /// the two are interchangeable in a row of actions.
    #[test]
    fn an_image_icon_takes_the_ambient_icon_size() {
        let icon = ImageIcon::new(Image::new(handle(64, 64)));
        assert_eq!(
            Widget::<()>::style(&icon).width,
            frus_layout::Dimension::Length(crate::icons::GRID)
        );
        let mut theme = Theme::default();
        theme.widgets.icon.size = Some(18.0);
        assert_eq!(
            Widget::<()>::style_themed(&icon, &theme).width,
            frus_layout::Dimension::Length(18.0),
            "the theme has a say in the size, not only the colour"
        );
        assert_eq!(
            Widget::<()>::style_themed(&icon.size(30.0), &theme).width,
            frus_layout::Dimension::Length(30.0),
            "and the caller outranks it"
        );
    }

    /// **A picture is not tinted unless the caller asks.** That is the one place this
    /// parts from a path icon, and it is deliberate: an icon is a silhouette and takes the
    /// foreground colour, where a brand mark flattened to one grey the first time an
    /// application themes its icons is a brand mark nobody recognises.
    #[test]
    fn a_picture_is_not_tinted_unless_it_is_asked() {
        let mut theme = Theme::default();
        theme.widgets.icon.color = Some(Color::rgb(0.9, 0.1, 0.1));
        let paint = |icon: ImageIcon| {
            let mut scene = frus_core::Scene::new();
            let status = Status::default();
            Widget::<()>::paint(
                &icon,
                Rect::new(0.0, 0.0, 24.0, 24.0),
                status,
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Image { tint, .. } => Some(*tint),
                    _ => None,
                })
                .expect("a picture")
        };
        let plain = paint(ImageIcon::new(Image::new(handle(8, 8))));
        assert_eq!(
            (plain.r, plain.g, plain.b),
            (1.0, 1.0, 1.0),
            "its own colours, whatever the icon theme says"
        );
        let tinted =
            paint(ImageIcon::new(Image::new(handle(8, 8))).color(Color::rgb(0.0, 1.0, 0.0)));
        assert_eq!((tinted.r, tinted.g, tinted.b), (0.0, 1.0, 0.0));
    }

    /// The box an image asks for, down a column 400 wide.
    ///
    /// A `Column` and not a `Flex`: the reference's column centres its children, where
    /// flexbox stretches them, and a **stretched** cross axis is a width the image was
    /// handed — which beats a ratio, here as there. That is the parent's say, not a
    /// property of the image.
    fn asked_box(image: Image) -> Rect {
        let root = crate::Column::<()>::new().child(image);
        let runtime = crate::Runtime::default();
        let theme = Theme::default();
        let mut layout = frus_layout::Layout::new();
        let node = crate::ui::build_layout(
            &root,
            crate::interaction::WidgetId::ROOT,
            &runtime,
            &theme,
            &mut layout,
        );
        layout.compute_filled(node, 400.0, 400.0);
        // The column is first; the image is its only child.
        layout.absolute_rects(node)[1].0
    }

    /// No size given: the image's own, which is the whole point — showing a bitmap
    /// should not require already knowing how many pixels across it is.
    #[test]
    fn an_image_with_no_size_is_its_own_size() {
        let r = asked_box(Image::new(handle(64, 32)));
        assert_eq!((r.width, r.height), (64.0, 32.0));
    }

    /// One side given: the other follows the image's ratio, as in the reference — as
    /// long as the parent is not handing that side a size of its own.
    #[test]
    fn one_side_derives_the_other_from_the_ratio() {
        let by_width = asked_box(Image::new(handle(64, 32)).width(128.0));
        assert_eq!((by_width.width, by_width.height), (128.0, 64.0));

        let by_height = asked_box(Image::new(handle(64, 32)).height(8.0));
        assert_eq!((by_height.width, by_height.height), (16.0, 8.0));
    }

    /// Both given: that box, ratio or no ratio.
    #[test]
    fn both_sides_given_win() {
        let r = asked_box(Image::new(handle(64, 32)).size(10.0, 90.0));
        assert_eq!((r.width, r.height), (10.0, 90.0));
    }

    /// The alignment decides where a letterboxed image sits, and which part of a
    /// cropped one survives. Both from one anchor, which is the reference's answer too.
    #[test]
    fn the_alignment_places_the_letterbox_and_moves_the_crop() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        // A square in a wide box, contained: 40×40, and the anchor says where across.
        let left = paint(
            Image::new(handle(10, 10))
                .size(100.0, 40.0)
                .alignment(Alignment::CENTER_LEFT),
            bounds,
        );
        let right = paint(
            Image::new(handle(10, 10))
                .size(100.0, 40.0)
                .alignment(Alignment::CENTER_RIGHT),
            bounds,
        );
        let x_of = |p: &Primitive| match p {
            Primitive::Image { rect, .. } => rect.x,
            other => panic!("an image, not {other:?}"),
        };
        assert_eq!(x_of(&left), 0.0);
        assert_eq!(x_of(&right), 60.0, "hard against the right edge");

        // A tall image cropped to a wide box keeps the part the anchor names, which
        // travels the other way: aligning to the top keeps the top of the image.
        let uv_y = |p: &Primitive| match p {
            Primitive::Image { uv, .. } => uv.y,
            other => panic!("an image, not {other:?}"),
        };
        let top = paint(
            Image::new(handle(10, 100))
                .size(100.0, 40.0)
                .fit(BoxFit::Cover)
                .alignment(Alignment::TOP_CENTER),
            bounds,
        );
        let middle = paint(
            Image::new(handle(10, 100))
                .size(100.0, 40.0)
                .fit(BoxFit::Cover),
            bounds,
        );
        assert_eq!(uv_y(&top), 0.0, "the top of the image is kept");
        assert!(uv_y(&middle) > 0.0, "centred, the top is cropped away");
    }

    /// Opacity goes into the tint rather than into a layer: an image is one primitive.
    #[test]
    fn opacity_fades_the_tint() {
        let faded = paint(
            Image::new(handle(4, 4)).size(20.0, 20.0).opacity(0.5),
            Rect::new(0.0, 0.0, 20.0, 20.0),
        );
        match faded {
            Primitive::Image { tint, .. } => assert_eq!(tint.a, 0.5),
            other => panic!("an image, not {other:?}"),
        }
    }

    fn paint(image: Image, bounds: Rect) -> Primitive {
        paint_in(image, bounds, &Theme::default())
    }

    /// The same, under a theme of the caller's choosing — which is how the reading
    /// direction reaches `paint`.
    fn paint_in(image: Image, bounds: Rect, theme: &Theme) -> Primitive {
        let mut scene = Scene::new();
        Widget::<()>::paint(&image, bounds, Status::default(), theme, &mut scene);
        scene.primitives()[0].clone()
    }

    #[test]
    fn contain_letterboxes_a_square_in_a_wide_box() {
        // A square 10×10 source in a 100×40 box gives 40×40, centred in x.
        let prim = paint(
            Image::new(handle(10, 10)).size(100.0, 40.0),
            Rect::new(0.0, 0.0, 100.0, 40.0),
        );
        match prim {
            Primitive::Image { rect, uv, .. } => {
                assert_eq!(rect, Rect::new(30.0, 0.0, 40.0, 40.0));
                assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
            }
            _ => panic!("expected an image"),
        }
    }

    #[test]
    fn tint_override_is_applied() {
        let prim = paint(
            Image::new(handle(4, 4))
                .size(20.0, 20.0)
                .tint(Color::rgb(1.0, 0.0, 0.0)),
            Rect::new(0.0, 0.0, 20.0, 20.0),
        );
        match prim {
            Primitive::Image { tint, .. } => {
                assert_eq!(tint.r, 1.0);
                assert_eq!(tint.g, 0.0);
            }
            _ => panic!("expected an image"),
        }
    }

    #[test]
    fn size_drives_the_layout_box() {
        let image = Image::new(handle(4, 4)).size(64.0, 48.0);
        let style = Widget::<()>::style(&image);
        assert_eq!(style.width, Dimension::Length(64.0));
        assert_eq!(style.height, Dimension::Length(48.0));
    }

    /// The hole this milestone came for. `Role::Image` was in the vocabulary and mapped
    /// to the platform's, and **nothing in the framework ever emitted it** — so every
    /// picture in every application was silent to a screen reader.
    #[test]
    fn a_labelled_image_is_announced() {
        let semantics = Widget::<()>::semantics(&Image::new(handle(4, 4)).semantic_label("Ada"))
            .expect("an image is announced");
        assert_eq!(semantics.role, frus_core::Role::Image);
        assert_eq!(semantics.label.as_deref(), Some("Ada"));
    }

    /// Unlabelled, it still says *there is a picture here*. A reader who meets it knows
    /// something is there and can move past it; leaving it out entirely would be the
    /// application's decision, not the widget's default.
    #[test]
    fn an_unlabelled_image_still_announces_the_role() {
        let semantics =
            Widget::<()>::semantics(&Image::new(handle(4, 4))).expect("still announced");
        assert_eq!(semantics.role, frus_core::Role::Image);
        assert_eq!(semantics.label, None);
    }

    /// Decoration is excluded outright, and the exclusion wins over a label — the
    /// reference's rule, and the only one that is not ambiguous when both are given.
    #[test]
    fn decoration_is_left_out_of_the_tree_a_reader_walks() {
        let decoration = Image::new(handle(4, 4))
            .semantic_label("a texture")
            .exclude_from_semantics(true);
        assert!(Widget::<()>::semantics(&decoration).is_none());
    }

    /// The sub-region an image samples, as painted.
    fn sampled(image: Image, theme: &Theme) -> Rect {
        match paint_in(image, Rect::new(0.0, 0.0, 40.0, 40.0), theme) {
            Primitive::Image { uv, .. } => uv,
            _ => panic!("expected an image"),
        }
    }

    /// A picture that **points** is mirrored in a right-to-left reading direction, which
    /// the reference describes as a scaling factor of -1 horizontally. Here it is a sign
    /// on the sampled width: the shader reads `uv.xy + unit_pos * uv.zw`, so a negative
    /// width walks the same span backwards — no transform, no layer, no second copy of
    /// the pixels.
    #[test]
    fn a_directional_image_is_mirrored_in_rtl() {
        let forward = sampled(Image::new(handle(8, 4)), &Theme::default());
        let mirrored = sampled(
            Image::new(handle(8, 4)).match_text_direction(true),
            &Theme::default().rtl(),
        );
        assert_eq!(
            mirrored.width, -forward.width,
            "the span is walked backwards"
        );
        assert_eq!(
            mirrored.x,
            forward.x + forward.width,
            "and it starts at the far edge"
        );
        // The vertical span is untouched: this is a mirror, not a rotation.
        assert_eq!((mirrored.y, mirrored.height), (forward.y, forward.height));
    }

    /// It is **opt-in**, and off it stays off in both directions. A photograph of a
    /// person does not want to be flipped because the interface is in Arabic, which is
    /// why the reference makes this a per-image switch rather than a global rule.
    #[test]
    fn an_ordinary_image_is_not_mirrored_by_the_reading_direction() {
        let plain = Image::new(handle(8, 4));
        assert_eq!(
            sampled(plain, &Theme::default().rtl()),
            sampled(Image::new(handle(8, 4)), &Theme::default()),
        );
    }

    /// And the switch alone does nothing: it is the direction that mirrors, not the flag.
    #[test]
    fn a_directional_image_is_left_alone_in_ltr() {
        assert_eq!(
            sampled(
                Image::new(handle(8, 4)).match_text_direction(true),
                &Theme::default()
            ),
            sampled(Image::new(handle(8, 4)), &Theme::default()),
        );
    }

    /// And it reaches the tree, not just the hook.
    ///
    /// A trait method nobody calls is the shape of bug this project has already been
    /// bitten by — a hook that answers correctly while the walk never asks it, green
    /// unit tests over a feature that does nothing. So this drives `build_ui` and reads
    /// what the walk actually collected.
    #[test]
    fn the_walk_collects_the_image_and_skips_the_decoration() {
        let tree = crate::Flex::<()>::column()
            .child(Image::new(handle(8, 8)).semantic_label("Ada Lovelace"))
            .child(Image::new(handle(8, 8)).exclude_from_semantics(true));
        let ui = crate::build_ui(
            &tree,
            crate::Size::new(200.0, 200.0),
            &crate::Runtime::default(),
            &Theme::default(),
        );
        let images: Vec<_> = ui
            .semantics()
            .iter()
            .filter(|(_, _, s)| s.role == frus_core::Role::Image)
            .map(|(_, _, s)| s.label.clone())
            .collect();
        assert_eq!(
            images,
            vec![Some("Ada Lovelace".to_string())],
            "the labelled one is announced and the decoration is not"
        );
    }

    /// A 2x2 PNG, encoded at run time so the test carries no binary fixture.
    ///
    /// Leaked deliberately: [`frus_core::cached`] keys on the address of `'static`
    /// bytes, and this is how a test gets some. One 90-odd byte leak per test process is
    /// the price of exercising the real path rather than a stand-in.
    #[cfg(feature = "images")]
    fn png() -> &'static [u8] {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&[255u8; 16]).expect("pixels");
        }
        Vec::leak(out)
    }

    /// The question an application asks first: how do I show my logo. One call, and the
    /// pixels come out the far side at the size the file says.
    #[cfg(feature = "images")]
    #[test]
    fn embedded_bytes_become_pixels() {
        let image = Image::memory(png());
        assert_eq!(image.error(), None, "a valid PNG decodes");
        let style = Widget::<()>::style(&image);
        assert_eq!(
            style.width,
            Dimension::Length(2.0),
            "at the file's own size"
        );
    }

    /// **Once** per process, not once per frame.
    ///
    /// This is the whole reason the store exists: a view is rebuilt sixty times a second
    /// and decoding a PNG is not free. The two handles being the same `Arc` is the proof
    /// — a second decode would produce a second `ImageData` with a different identity.
    #[cfg(feature = "images")]
    #[test]
    fn the_same_bytes_are_decoded_once() {
        let bytes = png();
        let first = Image::memory(bytes);
        let second = Image::memory(bytes);
        let handle = |image: &Image| match &image.source {
            Source::Ready(handle) => handle.clone(),
            _ => panic!("decoded"),
        };
        let (a, b) = (handle(&first), handle(&second));
        let (a, b) = (&a, &b);
        assert_eq!(a.id(), b.id(), "one decode, one image, shared");
        assert!(
            std::sync::Arc::ptr_eq(a, b),
            "and literally the same handle"
        );
    }

    /// Bytes that are not an image: the widget says so and paints nothing, rather than
    /// panicking or drawing a stand-in nobody asked for.
    #[test]
    fn bytes_that_are_not_an_image_report_it_and_draw_nothing() {
        static NOT_AN_IMAGE: &[u8] = b"this is not a PNG";
        let broken = Image::memory(NOT_AN_IMAGE);
        assert!(broken.error().is_some(), "it says what went wrong");
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &broken,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene.primitives().is_empty(), "and draws nothing at all");
    }

    /// A broken image takes **no room** unless it was given some. There is no natural
    /// size to fall back on, and anything the caller did say is still honoured — which
    /// is what keeps a page from jumping because one asset is bad.
    #[test]
    fn a_broken_image_takes_the_room_it_was_given_and_no_more() {
        static NOT_AN_IMAGE: &[u8] = b"nor is this";
        let bare = Widget::<()>::style(&Image::memory(NOT_AN_IMAGE));
        assert_eq!(bare.width, Dimension::Length(0.0));
        assert_eq!(bare.height, Dimension::Length(0.0));
        let sized = Widget::<()>::style(&Image::memory(NOT_AN_IMAGE).size(64.0, 48.0));
        assert_eq!(sized.width, Dimension::Length(64.0));
        assert_eq!(sized.height, Dimension::Length(48.0));
    }

    /// The failure is remembered too. A file that is not a PNG will not become one on
    /// the next frame, and retrying every frame would turn one broken asset into a
    /// permanent cost.
    #[test]
    fn a_failure_is_remembered_rather_than_retried() {
        static NOT_AN_IMAGE: &[u8] = b"still not a PNG";
        let first = Image::memory(NOT_AN_IMAGE).error().map(str::to_string);
        let second = Image::memory(NOT_AN_IMAGE).error().map(str::to_string);
        assert!(first.is_some());
        assert_eq!(first, second);
    }

    /// The network path, with the network standing in for itself.
    ///
    /// These share one process-wide store and one registered fetcher, so they run under
    /// a lock rather than in parallel: two of them racing would see each other's URLs
    /// and each other's fetcher. That is a property of the thing being tested — it is
    /// deliberately global — rather than a weakness of the tests.
    #[cfg(feature = "images")]
    mod network {
        use super::*;
        use std::sync::{Mutex, MutexGuard, OnceLock};

        fn serialised() -> MutexGuard<'static, ()> {
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            let lock = LOCK.get_or_init(|| Mutex::new(()));
            let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            frus_core::forget_fetched_images();
            guard
        }

        /// A fetcher that answers **on the spot**, with a PNG.
        ///
        /// Synchronous on purpose: it is the harder case for the store, since the
        /// callback runs while `fetched` is still inside the call that started it. If
        /// the lock were still held there, this would deadlock rather than fail.
        fn instant_png(_url: &str, deliver: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>) {
            deliver(Ok(png().to_vec()));
        }

        /// A fetcher that answers on the spot with a failure.
        fn instant_failure(_url: &str, deliver: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>) {
            deliver(Err("404".to_string()));
        }

        /// One that never answers at all — a request still in flight.
        fn never(_url: &str, _deliver: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>) {}

        #[test]
        fn a_fetched_image_arrives_and_is_shown() {
            let _guard = serialised();
            frus_core::set_image_fetcher(instant_png);
            // The first ask started the work; this fetcher had already finished by the
            // time it returned, so the second ask has the pixels.
            let _ = Image::network("https://example.com/a.png");
            let arrived = Image::network("https://example.com/a.png");
            assert_eq!(arrived.state(), State::Ready);
            assert_eq!(Widget::<()>::style(&arrived).width, Dimension::Length(2.0));
        }

        #[test]
        fn a_request_still_in_flight_reads_as_loading_and_keeps_the_frames_coming() {
            let _guard = serialised();
            frus_core::set_image_fetcher(never);
            let waiting = Image::network("https://example.com/slow.png");
            assert_eq!(waiting.state(), State::Loading);
            assert_eq!(waiting.error(), None, "loading has not failed");
            // The interface must keep drawing, or the frame that would show the picture
            // never happens.
            assert!(frus_core::images_in_flight() > 0);
        }

        #[test]
        fn a_request_that_fails_says_why_rather_than_waiting_for_ever() {
            let _guard = serialised();
            frus_core::set_image_fetcher(instant_failure);
            let _ = Image::network("https://example.com/gone.png");
            let dead = Image::network("https://example.com/gone.png");
            assert_eq!(dead.state(), State::Failed("404"));
            assert_eq!(dead.error(), Some("404"));
        }

        /// **Once**, however many frames ask. A view runs sixty times a second, and a
        /// fetch per frame would be sixty requests for one picture.
        #[test]
        fn a_view_asking_every_frame_fetches_once() {
            let _guard = serialised();
            static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            fn counted(_url: &str, deliver: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>) {
                CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                deliver(Ok(png().to_vec()));
            }
            CALLS.store(0, std::sync::atomic::Ordering::Relaxed);
            frus_core::set_image_fetcher(counted);
            for _ in 0..60 {
                let _ = Image::network("https://example.com/once.png");
            }
            assert_eq!(CALLS.load(std::sync::atomic::Ordering::Relaxed), 1);
        }

        /// And nothing is left in flight once it has landed, so the interface settles
        /// back to drawing only when something changes.
        #[test]
        fn the_count_falls_back_to_zero_when_the_work_is_done() {
            let _guard = serialised();
            frus_core::set_image_fetcher(instant_png);
            let _ = Image::network("https://example.com/settle.png");
            assert_eq!(frus_core::images_in_flight(), 0);
        }
    }
}
