//! [`AppBar`]: an **adaptive application bar**, Material style.
//!
//! The developer declares **one** title, an optional `leading` and a list of
//! **actions** — never saying "this is for mobile / desktop". The AppBar decides on
//! its own, from the **available width**, how many actions fit inline and **folds
//! the rest into a `⋯` overflow menu**. A wide screen → everything inline; a narrow
//! phone → overflow. One piece of code, adapting automatically.
//!
//! **Everything is customisable** (themed defaults, never imposed): the title can
//! be an arbitrary widget (`title`) or styled text (`title_style`), an
//! action can be an arbitrary widget (`action_widget`, always inline), and the
//! spacing, the action size, the background and the height can all be overridden.
//!
//! ```ignore
//! AppBar::new("My Tasks")
//!     .width(available_width)                 // a size, not a platform
//!     .title_style(TextStyle::new(22.0))      // or .title(logo_row)
//!     .leading(button("☰", Msg::ToggleMenu))
//!     .overflow(app.menu_open, Msg::ToggleMenu)
//!     .action("Pause", Msg::ToggleTimer)
//!     .action_widget(Badge::new("3"))         // a free widget, never folded
//!     .action("Settings →", Msg::OpenSettings)
//!     .build()
//! ```

#[cfg(test)]
use frus_core::FontWeight;
use frus_core::{BorderRadius, Color, TextStyle};
use frus_layout::{Align, Dimension};

use crate::button::Variant;
use crate::constraints::ConstrainedBox;
use crate::container::Container;
use crate::dsl::button;
use crate::flex::Flex;
use crate::media::MediaQuery;
use crate::menu::PopupMenuButton;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgettheme::{DefaultTextStyle, IconTheme};

/// Does this platform centre an application bar's title by default?
///
/// Where the system centres its own — Apple's platforms — so does a bar that has not
/// been told otherwise, and **only while there is room to read it that way**: past one
/// action the title goes back to being flush after the leading, because a centred title
/// squeezed between a leading and three buttons is neither centred nor readable. Every
/// other platform starts the title after the leading and leaves it there.
///
/// Resolved at **compile time** from the target, like
/// [`ScrollPhysics::platform_default`](crate::ScrollPhysics::platform_default): a build
/// is for one platform. [`AppBar::center_title`] overrides it either way.
pub const fn platform_centers_title(actions: usize) -> bool {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        actions < 2
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        // Referenced so the parameter is not dead on the platforms that ignore it.
        let _ = actions;
        false
    }
}

/// The bar's height, the reference's Material 3 `toolbarHeight`.
///
/// A **fixed** height, not the content's: chrome that changes height with what happens
/// to be in it makes every screen a slightly different shape, and the page below it move
/// when an action appears. [`AppBar::height`] overrides it.
pub const APP_BAR_HEIGHT: f32 = 64.0;
/// The title's type: what the caller said, else what the theme's `app_bar` says, else the
/// step of the type scale the reference names — `titleLarge`.
///
/// It used to be a private `TextStyle::new(22.0).weight(Medium)`, which had the size right
/// and **the weight wrong**: `titleLarge` is regular. A number written beside the scale is a
/// number that can drift from it without anybody seeing, which is milestone 413's lesson and
/// this is the same constant one file over.
fn title_style_of(over: Option<TextStyle>, theme: &Theme) -> TextStyle {
    over.or(theme.widgets.app_bar.title_style)
        .unwrap_or(theme.text.title_large)
}
/// How far the reader's font setting may enlarge the **title**, and no further.
///
/// A bar is chrome: it keeps [`APP_BAR_HEIGHT`] whatever the reader asked for, because a
/// toolbar that grew with the type would push every screen down. So the reference caps the
/// title's scaler rather than the bar's height — the same 1.34 — "to keep the visual
/// hierarchy the same even with larger font sizes". A caller who wants the whole scale
/// gives the title its own [`AppBar::title_style`] and their own height.
pub const APP_BAR_MAX_TITLE_SCALE: f32 = 1.34;
/// The actions' size: what the caller said, else the step the reference gives them.
///
/// A bar's actions **are text buttons**, so they take `labelLarge` — the same step
/// [`crate::Button`] already reads — rather than the `bodyMedium` the reference gives the
/// toolbar's other text.
fn action_size_of(over: Option<f32>, theme: &Theme) -> f32 {
    over.or(theme.text.label_large.size)
        .unwrap_or(frus_core::DEFAULT_TEXT_SIZE)
}
/// Whether an untold bar sits at the top of the screen. The reference's default, and the
/// one that makes a bar used **outside** a shell behave: it consumes the status bar itself
/// rather than waiting for something to inset it.
const PRIMARY: bool = true;

/// A button's inner horizontal padding (must follow `button::PAD_X`).
const BTN_PAD_X: f32 = 20.0;
/// The space between the bar's elements (the default, overridden by [`AppBar::gap`]).
const GAP: f32 = 8.0;
/// The width reserved for the `leading` slot (a leading icon, Material style).
const LEADING_SLOT: f32 = 56.0;
/// The bar's horizontal margin: the content does not touch the edges (Material
/// style). Counted in the folding budget.
const H_PAD: f32 = 8.0;
/// The width the title is never squeezed below, in px. A title cut to nothing tells a
/// reader less than a title cut to two characters and an ellipsis.
const TITLE_MIN: f32 = 64.0;
/// The glyph on the overflow button.
const OVERFLOW_GLYPH: &str = "\u{22ef}";
/// The elevation shadow's colour. Only its alpha is local; the hue is the theme's
/// job, and a bar that needs another one sets its own background instead.
const SHADOW: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.22,
};

/// The title: styled text, or any widget at all.
/// An action: labelled (foldable into the overflow) or a free widget (always
/// inline — an arbitrary widget cannot become a text menu row).
enum Action<Msg> {
    Labeled { label: String, message: Msg },
    Custom(Box<dyn Widget<Msg>>),
}

/// An adaptive application bar. A fluent builder finished by [`AppBar::build`].
pub struct AppBar<Msg> {
    title: Box<dyn Widget<Msg>>,
    title_style: Option<TextStyle>,
    /// Was the title's style left at the framework's default? Only then may the theme
    /// have its say — a caller who set one outranks it.
    width: f32,
    leading: Option<Box<dyn Widget<Msg>>>,
    /// Whether a bar with no `leading` of its own may take one from the shell it stands in.
    automatically_imply_leading: bool,
    /// The same for the trailing end.
    automatically_imply_actions: bool,
    overflow: Option<(bool, Msg)>,
    actions: Vec<Action<Msg>>,
    action_size: Option<f32>,
    primary: bool,
    gap: f32,
    background: Option<Color>,
    height: Option<f32>,
    /// `None` = the platform's convention (see [`platform_centers_title`]).
    center_title: Option<bool>,
    bottom: Option<Box<dyn Widget<Msg>>>,
    leading_width: Option<f32>,
    title_spacing: f32,
    foreground: Option<Color>,
    elevation: f32,
    shape: Option<BorderRadius>,
    shadow_color: Option<Color>,
    surface_tint: Option<Color>,
    force_material_transparency: bool,
    toolbar_opacity: f32,
    bottom_opacity: f32,
    actions_padding: Option<f32>,
    flexible_space: Option<Box<dyn Widget<Msg>>>,
    icon_theme: Option<IconTheme>,
    actions_icon_theme: Option<IconTheme>,
    toolbar_text_style: Option<TextStyle>,
    exclude_header_semantics: bool,
}

impl<Msg: Clone + 'static> AppBar<Msg> {
    /// Creates a bar with a text title, as wide as **the surface it is being built
    /// for** — which is what decides how many actions fit on the line and how many
    /// fold into the overflow menu.
    ///
    /// The width is read from [`MediaQuery::of`], so no caller passes it. Outside any
    /// surface description — a unit test that builds a bar on its own — there is no
    /// width to fold against and nothing folds, which is what this did before the
    /// surface was ambient. A bar that is **not** the full width of the screen (one
    /// beside a rail, say) still says so with [`AppBar::width`].
    pub fn new(title: impl Into<String>) -> Self {
        let surface = MediaQuery::of();
        Self {
            // **A widget, never a string.** The reference's `title` is a `Widget?` and
            // nothing else (`app_bar.dart:1067`), and a string constructor that took a
            // second path through the bar is what made a bar's accessibility depend on
            // which constructor the caller had reached for (milestone 397).
            //
            // No style on it. It takes the resolved title type from the
            // `DefaultTextStyle` the bar hands down, which is what the reference does at
            // `app_bar.dart:1084` — and it is why this could not be written before
            // milestone 400.
            title: Box::new(Text::new(title)),
            title_style: None,
            width: if surface.is_described() {
                surface.size.width
            } else {
                f32::MAX
            },
            leading: None,
            automatically_imply_leading: true,
            automatically_imply_actions: true,
            overflow: None,
            actions: Vec::new(),
            action_size: None,
            primary: PRIMARY,
            gap: GAP,
            background: None,
            height: None,
            center_title: None,
            bottom: None,
            leading_width: None,
            title_spacing: GAP * 2.0,
            foreground: None,
            elevation: 0.0,
            shape: None,
            shadow_color: None,
            surface_tint: None,
            force_material_transparency: false,
            toolbar_opacity: 1.0,
            bottom_opacity: 1.0,
            actions_padding: None,
            flexible_space: None,
            icon_theme: None,
            actions_icon_theme: None,
            toolbar_text_style: None,
            exclude_header_semantics: false,
        }
    }

    /// The **available width** for the bar, in logical pixels: what drives the
    /// folding. It is a *size*, not a platform indicator.
    ///
    /// An override. [`AppBar::new`] already takes the surface's width from
    /// [`MediaQuery::of`]; this is for a bar that does not get the whole of it.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The text title's style (size/weight/italic/color). Default: the reference's
    /// `title_large` at medium weight, in the theme's colour.
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// The title, as **any widget** — a logo, a composed row, a text of the caller's own.
    ///
    /// This is the title in the reference, where `title` is a `Widget?` and there is no
    /// string form at all. [`AppBar::new`] is a convenience that wraps a string in a plain
    /// text and hands it here; both end at the same place, wearing the same type and
    /// announced as the same landmark.
    ///
    /// It was called `title_widget` while the bar had two kinds of title. It has one.
    pub fn title(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.title = Box::new(widget);
        self
    }

    /// The leading element (a menu or back button…), optional.
    pub fn leading(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.leading = Some(Box::new(widget));
        self
    }

    /// Whether a bar with **no `leading` of its own** may take one from the shell it
    /// stands in. `true` unless said otherwise, as in the reference.
    ///
    /// A bar on a screen that has a drawer then grows the button that opens it, and a bar
    /// on a screen that has none stays as it was — the shell is what knows, and until
    /// milestone 422 it had no way to say so to a slot handed to it already built.
    ///
    /// The reference's other branch has no counterpart here. There a bar also implies a
    /// **back button** when the route it is in can be popped; frus's
    /// [`Navigator`](crate::Navigator) is controlled — the application holds the stack — so
    /// the framework does not know a depth to imply anything from. A screen that can be
    /// left says so with [`AppBar::leading`].
    ///
    /// `false` is for a bar that wants its leading slot empty on a screen that has a
    /// drawer: a logo where the button would be, and some other way in.
    pub fn automatically_imply_leading(mut self, imply: bool) -> Self {
        self.automatically_imply_leading = imply;
        self
    }

    /// Whether a bar with **no actions of its own** may take one from the shell it stands
    /// in. `true` unless said otherwise, as in the reference (`app_bar.dart:1113`).
    ///
    /// A bar with nothing at its trailing end, on a screen that has an end drawer, grows
    /// the button that opens it. One action of its own and it does not: the reference's
    /// test is the same — the implied button is what fills an **empty** end, never
    /// something added beside what the caller put there.
    pub fn automatically_imply_actions(mut self, imply: bool) -> Self {
        self.automatically_imply_actions = imply;
        self
    }

    /// Enables the overflow menu: its open state (controlled by the app) and the
    /// toggle message (emitted by the `⋯` button and on an outside click).
    pub fn overflow(mut self, open: bool, toggle: Msg) -> Self {
        self.overflow = Some((open, toggle));
        self
    }

    /// Adds a labelled action (a button). Shown inline if it fits, otherwise folded
    /// into the overflow menu.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.actions.push(Action::Labeled {
            label: label.into(),
            message,
        });
        self
    }

    /// Adds a **free widget** action (a badge, an avatar, a field…). Always inline —
    /// an arbitrary widget cannot fold into a menu row.
    pub fn action_widget(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.actions.push(Action::Custom(Box::new(widget)));
        self
    }

    /// The labelled actions' font size (16 px by default).
    pub fn action_size(mut self, size: f32) -> Self {
        self.action_size = Some(size);
        self
    }

    /// Whether this bar sits at the **top of the screen**, and so has to keep its content
    /// out of the status bar. `true` unless said otherwise, as in the reference.
    ///
    /// The bar pads **itself**: its surface still runs behind the status bar — that is what
    /// a Material bar looks like — and only its toolbar and its `bottom` are held clear.
    /// The two things this separates are the shell's job and the bar's: a
    /// [`Scaffold`](crate::Scaffold) makes the slot tall enough, and the bar decides
    /// whether to use the room. Until milestone 417 the shell insetted the bar from
    /// outside, which is one switch where the reference has two — and why a bar used
    /// *without* a shell drew under the status bar.
    ///
    /// Turn it off for a bar that is not at the top: one nested in a page, or a second bar
    /// under the first.
    #[must_use]
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    /// The space between the bar's elements (8 px by default).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// The bar's background color (transparent by default: the parent decides).
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// An imposed bar height (by default, the content's natural height).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Centres the title in the space left between the leading and the actions,
    /// rather than starting it flush after the leading.
    ///
    /// **Overrides the platform's convention**, which is what a bar that has not been
    /// told anything follows: centred where the system centres its own titles, flush
    /// after the leading everywhere else. See [`platform_centers_title`].
    ///
    /// Centred, the title still yields to the actions first: it is centred in what is
    /// left, not in the window.
    pub fn center_title(mut self, centered: bool) -> Self {
        self.center_title = Some(centered);
        self
    }

    /// A widget **under** the bar, spanning its width — a row of tabs, a search field,
    /// a progress bar. It belongs to the bar, so it sits inside the background.
    pub fn bottom(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.bottom = Some(Box::new(widget));
        self
    }

    /// The width reserved for the leading slot. Unset, it is the Material slot width;
    /// a leading wider than that would otherwise push the title without the folding
    /// budget knowing, and the actions would run off the edge.
    pub fn leading_width(mut self, width: f32) -> Self {
        self.leading_width = Some(width);
        self
    }

    /// The gap between the leading and the title.
    pub fn title_spacing(mut self, spacing: f32) -> Self {
        self.title_spacing = spacing;
        self
    }

    /// The colour of the bar's own text — the title, and the labelled actions. Unset,
    /// each follows the theme.
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// A shadow under the bar, in px of blur. `0` — the default — draws none, which is
    /// what a bar sitting on a surface of the same colour wants.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation.max(0.0);
        self
    }

    /// The bar's **shape**: how far its corners are rounded. Square by default, as the
    /// reference's is.
    ///
    /// The bar clips to it, so a coloured surface and anything drawn on it stop at the
    /// curve rather than squaring off the corner the shadow already rounded. The
    /// reference's `shape` is a whole `ShapeBorder`; this is the part of it a bar
    /// actually uses — a rounded rectangle, per corner if wanted.
    pub fn shape(mut self, shape: impl Into<BorderRadius>) -> Self {
        self.shape = Some(shape.into());
        self
    }

    /// The colour of the shadow the bar casts. Only visible with an
    /// [`elevation`](Self::elevation).
    ///
    /// The reference's `shadowColor`. Left unset it is the framework's own near-black,
    /// which is right on a light surface and too heavy on some dark ones.
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = Some(color);
        self
    }

    /// The colour laid over the bar's surface **in proportion to its elevation** — the
    /// reference's `surfaceTintColor`, and Material 3's way of showing height.
    ///
    /// A shadow says a thing is above the page; the tint says how far, and it is what
    /// still reads on a dark background where a shadow shows nothing. The strength comes
    /// from the specification's table (see
    /// [`surface_tint_opacity`](frus_core::surface_tint_opacity)), so an elevation of 3
    /// tints at 8% whoever asks.
    ///
    /// Nothing is tinted at elevation zero, which is the default: a flat bar is the
    /// surface it was given.
    pub fn surface_tint(mut self, color: Color) -> Self {
        self.surface_tint = Some(color);
        self
    }

    /// Draws the bar with **no surface at all**: no background, no tint, no shadow —
    /// only what it holds.
    ///
    /// The reference's `forceMaterialTransparency`, and it exists for the same case: a
    /// bar over an image or a video, where the chrome should be the controls and nothing
    /// else. It overrides the background, the tint and the elevation rather than
    /// arguing with them, because a caller asking for transparency has already decided.
    pub fn force_material_transparency(mut self, force: bool) -> Self {
        self.force_material_transparency = force;
        self
    }

    /// The opacity of the toolbar row — the leading, the title and the actions — without
    /// touching the bar's surface. `1.0` by default.
    ///
    /// The reference's `toolbarOpacity`. It is what fades a collapsing header's contents
    /// out while its background stays, and it is a **group** opacity: overlapping
    /// children do not darken where they overlap.
    pub fn toolbar_opacity(mut self, opacity: f32) -> Self {
        self.toolbar_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// The opacity of the [`bottom`](Self::bottom) slot, independently of the toolbar.
    /// `1.0` by default. The reference's `bottomOpacity`.
    pub fn bottom_opacity(mut self, opacity: f32) -> Self {
        self.bottom_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Padding around the **actions** as a group, on top of the gap between them.
    ///
    /// The reference's `actionsPadding`, added in its 3.27 line for the same reason: an
    /// icon button's own hit area already reaches the bar's edge, so a design that wants
    /// the *glyphs* inset has nowhere else to say so.
    pub fn actions_padding(mut self, padding: f32) -> Self {
        self.actions_padding = Some(padding);
        self
    }
    /// Stops the title being announced as a **heading**. `false` by default, as the
    /// reference's `excludeHeaderSemantics` is.
    ///
    /// A screen reader's user moves through a screen by its headings, and the bar's title
    /// is the one every screen has. Excluding it is for a bar whose title is decorative,
    /// or one of two bars on a page where only the outer one names it — announcing both
    /// as headings gives the user two landmarks where there is one screen.
    pub fn exclude_header_semantics(mut self, exclude: bool) -> Self {
        self.exclude_header_semantics = exclude;
        self
    }

    /// A widget stacked **behind** the toolbar and the [`bottom`](Self::bottom), filling
    /// the bar's whole box.
    ///
    /// The reference's `flexibleSpace`, and it is what makes a collapsing header possible
    /// at all: an image, a gradient, a photograph that the title sits on. The bar's height
    /// is unchanged — this is a layer, not a slot — so the space is exactly as tall as
    /// the bar however tall the widget would rather be.
    ///
    /// The bar's own background is drawn first and this over it, so a translucent surface
    /// still tints what is behind, and a caller who wants only the image says
    /// [`force_material_transparency`](Self::force_material_transparency).
    pub fn flexible_space(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.flexible_space = Some(Box::new(widget));
        self
    }

    /// The colour and the size of the **glyphs in the bar** — the reference's `iconTheme`.
    ///
    /// It reaches every icon in the leading slot and, unless
    /// [`actions_icon_theme`](Self::actions_icon_theme) says otherwise, in the actions
    /// too. It is delivered as a theme for that subtree rather than as an argument to
    /// each widget, so it reaches an icon nested inside a button the bar never sees —
    /// which is what the reference's inherited `IconTheme` does and why it is one.
    ///
    /// An icon that names its own colour still wins: `caller ?? this ?? the scheme`.
    pub fn icon_theme(mut self, icons: IconTheme) -> Self {
        self.icon_theme = Some(icons);
        self
    }

    /// The same, for the **actions** alone — the reference's `actionsIconTheme`.
    ///
    /// Unset, the actions follow [`icon_theme`](Self::icon_theme). Set, they part company
    /// with the leading slot, which is the case it exists for: a back arrow in the
    /// foreground colour beside actions in a muted one.
    pub fn actions_icon_theme(mut self, icons: IconTheme) -> Self {
        self.actions_icon_theme = Some(icons);
        self
    }

    /// The type worn by the **words in the bar that are not the title** — a label beside
    /// the back arrow, a "Save" in the actions, anything a caller handed over already
    /// assembled.
    ///
    /// Not the title: the title has [`title_style`](Self::title_style), and the reference
    /// keeps the same two apart for the same reason. A bar's title is one line the bar
    /// itself lays out and can measure; the rest is other people's widgets, and the only
    /// way to reach a `Text` nested three levels inside one of them is to hand the style
    /// *down* rather than pass it *in*.
    ///
    /// It is handed down **field by field**: setting a size leaves the colours alone, and
    /// a `Text` that chose its own size keeps it. See
    /// [`DefaultTextStyle`](crate::DefaultTextStyle) for the rule in full.
    pub fn toolbar_text_style(mut self, style: TextStyle) -> Self {
        self.toolbar_text_style = Some(style);
        self
    }

    /// The width an action button would take for this label.
    fn action_width(label: &str, size: f32) -> f32 {
        frus_text::measure(label, size).width + BTN_PAD_X * 2.0
    }

    /// A widget's declared width (0 if it depends on layout).
    fn widget_width(widget: &dyn Widget<Msg>, theme: &Theme) -> f32 {
        match widget.style_themed(theme).width {
            Dimension::Length(v) => v,
            _ => 0.0,
        }
    }

    /// Assembles the bar into a widget ready to display (the row `leading . title .
    /// spring . inline actions . overflow`, over an optional `bottom`).
    ///
    /// The order in which space is handed out is worth stating, because it is a
    /// decision and not an accident: **the actions are served first and the title
    /// yields.** A title cut short is still a title; an action pushed off the edge is
    /// gone. So the folding budget is computed against the title's *floor*, and
    /// whatever is left afterwards is what the title gets - with an ellipsis if it
    /// does not fit.
    /// Finishes the bar.
    ///
    /// The composition is **deferred until the theme is known** ([`ThemeBuilder`]):
    /// `center_title` decides which children exist and in what order, so it cannot be
    /// resolved by a hook that runs after the row has been assembled. Everything the
    /// theme has a say in — the title's centring, its type, the bar's colours — is
    /// resolved inside, `caller ?? theme ?? framework`.
    ///
    /// [`ThemeBuilder`]: crate::ThemeBuilder
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        Box::new(crate::themebuilder::ThemeBuilder::boxed(move |theme| {
            self.build_with(theme)
        }))
    }

    /// The composition proper, against a known theme.
    fn build_with(self, theme: &Theme) -> Box<dyn Widget<Msg>> {
        let t = theme.widgets.app_bar;
        let AppBar {
            title,
            title_style,
            width,
            leading,
            automatically_imply_leading,
            automatically_imply_actions,
            overflow,
            actions,
            action_size,
            gap,
            background,
            height,
            center_title,
            bottom,
            leading_width,
            title_spacing,
            foreground,
            elevation,
            shape,
            shadow_color,
            surface_tint,
            force_material_transparency,
            toolbar_opacity,
            bottom_opacity,
            actions_padding,
            primary,
            flexible_space,
            icon_theme,
            actions_icon_theme,
            toolbar_text_style,
            exclude_header_semantics,
        } = self;

        // **What the shell knows and the bar does not** (milestone 422). This bar was
        // handed to its `Scaffold` already built, so it cannot see the screen it stands on;
        // the shell tells it, through the ambient the walk installs on the way down, and
        // this composition runs under that ambient because it is deferred.
        //
        // A bar with no `leading` of its own, on a screen with a drawer, grows the button
        // that opens it — the reference's first branch at `app_bar.dart:1010`. The second,
        // a back button for a route that can be popped, has no counterpart: frus's
        // `Navigator` is controlled, so there is no stack depth for the framework to read.
        let shell = crate::ScaffoldInfo::of();
        let leading = leading.or_else(|| {
            if !automatically_imply_leading {
                return None;
            }
            let toggle = shell.drawer_toggle::<Msg>()?;
            Some(
                Box::new(crate::IconButton::new(crate::Icons::Menu).on_press(toggle))
                    as Box<dyn Widget<Msg>>,
            )
        });
        // And the trailing end, on the same terms: **an empty end**, never a button added
        // beside what the caller put there (`app_bar.dart:1113`). An overflow toggle counts
        // as something at that end, since it is what the bar puts there itself.
        let mut actions = actions;
        if actions.is_empty() && overflow.is_none() && automatically_imply_actions {
            if let Some(toggle) = shell.end_drawer_toggle::<Msg>() {
                actions.push(Action::Custom(Box::new(
                    crate::IconButton::new(crate::Icons::Menu).on_press(toggle),
                )));
            }
        }

        // `caller ?? theme ?? platform`. The platform is the last word rather than the
        // framework's own taste, because where a title sits is a system convention before
        // it is a design one — see `platform_centers_title`.
        let center_title = center_title
            .or(t.center_title)
            .unwrap_or_else(|| platform_centers_title(actions.len()));

        // The title's type, likewise: the caller's style, else the theme's, else the
        // reference's `titleLarge`.
        let mut title_style = title_style_of(title_style, theme);
        let action_size = action_size_of(action_size, theme);
        let foreground = foreground
            .or(t.foreground)
            .unwrap_or(theme.scheme.on_surface);
        title_style.color = Some(foreground);
        // The bar keeps its height, so the title is capped rather than let out. See
        // [`APP_BAR_MAX_TITLE_SCALE`]: this is the reference's answer for chrome, and the
        // opposite of the one its components give (there the height is a floor and the
        // content wins).
        let title_style = title_style.clamp_scale(APP_BAR_MAX_TITLE_SCALE);
        // The title's type as something a subtree can **hand down**, plus the two settings
        // the reference pairs with it: one line, cut with an ellipsis. A `Text` that never
        // chose a size, a wrap or an overflow takes all three; one that did keeps its own.
        let title_words = DefaultTextStyle {
            soft_wrap: Some(false),
            overflow: Some(frus_core::TextOverflow::Ellipsis),
            ..DefaultTextStyle::from_text_style(title_style)
        };
        // The same thing as a whole theme, for asking a title how wide it naturally is.
        let dressed_title_theme = {
            let mut dressed = theme.clone();
            dressed.widgets.text = dressed.widgets.text.merge(title_words);
            dressed
        };
        let background = background.or(t.background);
        let elevation = if elevation > 0.0 {
            elevation
        } else {
            t.elevation.unwrap_or(0.0)
        };
        let height = height.or(t.height);
        let leading_w = match (&leading, leading_width) {
            (None, _) => 0.0,
            (Some(_), Some(w)) => w,
            (Some(widget), None) => Self::widget_width(widget.as_ref(), theme).max(LEADING_SLOT),
        };
        // The title's natural width, asked of the widget **under the type it will
        // actually wear**. Asking it bare would measure a string title at the framework's
        // 16 px and reserve room for a bar that draws it at 22.
        let natural_title = Self::widget_width(title.as_ref(), &dressed_title_theme);

        // The room the actions may claim: everything except the margins, the leading,
        // the spacing, and what the title is **reserved**.
        //
        // The order matters and it is a decision. A bar that folds actions into an
        // overflow already has a way of making room, and that is the one to use first:
        // the title keeps its natural width — up to half the bar, so a long one cannot
        // starve the actions — and the actions fold to fit what is left. Truncating
        // the title is the *last* resort, for when even one action and the overflow
        // button will not fit beside it.
        let fixed = H_PAD * 2.0 + leading_w + title_spacing + gap;
        let room = (width - fixed).max(0.0);
        let title_reserve = natural_title
            .min(room * 0.5)
            .max(TITLE_MIN.min(natural_title));
        let budget = room - title_reserve;
        let overflow_btn_w = Self::action_width(OVERFLOW_GLYPH, action_size) + gap;

        // Each action's width; free widgets are **always** inline.
        let widths: Vec<f32> = actions
            .iter()
            .map(|action| match action {
                Action::Labeled { label, .. } => Self::action_width(label, action_size) + gap,
                Action::Custom(widget) => Self::widget_width(widget.as_ref(), theme) + gap,
            })
            .collect();
        let total: f32 = widths.iter().sum();
        let custom_total: f32 = actions
            .iter()
            .zip(&widths)
            .filter(|(action, _)| matches!(action, Action::Custom(_)))
            .map(|(_, w)| *w)
            .sum();

        // How many **labelled** actions fit inline? If everything fits, no overflow;
        // otherwise reserve the overflow button and the free widgets and keep as many
        // labelled ones as possible, in order - so it is the last actions that fold,
        // which is where a reader has learnt to go looking for them.
        let (kept_labeled, actions_w) = if total <= budget {
            (usize::MAX, total)
        } else {
            let mut used = overflow_btn_w + custom_total;
            let mut kept = 0;
            for (action, w) in actions.iter().zip(&widths) {
                if matches!(action, Action::Custom(_)) {
                    continue;
                }
                if used + w <= budget {
                    used += w;
                    kept += 1;
                } else {
                    break;
                }
            }
            (kept, used)
        };

        // What is left for the title, once the actions have taken theirs.
        let title_room = (width - fixed - actions_w).max(TITLE_MIN.min(natural_title));

        // An icon theme is delivered as a **theme for the subtree**, not as an argument
        // to each widget: that is the only way it reaches a glyph nested inside a button
        // the bar never sees. The reference's `IconTheme` is an inherited widget for the
        // same reason.
        //
        // A toolbar text style travels the same road, and for the same reason: the words
        // it is meant for are inside widgets the bar was handed, not ones it built.
        let dress = |widget: Box<dyn Widget<Msg>>, icons: Option<IconTheme>| {
            if icons.is_none() && toolbar_text_style.is_none() {
                return widget;
            }
            let words = toolbar_text_style.map(DefaultTextStyle::from_text_style);
            Box::new(crate::Themed::tweak(
                move |t| {
                    if let Some(icons) = icons {
                        if let Some(color) = icons.color {
                            t.widgets.icon.color = Some(color);
                            t.widgets.icon_button.icon_color = Some(color);
                        }
                        if let Some(size) = icons.size {
                            t.widgets.icon.size = Some(size);
                            t.widgets.icon_button.icon_size = Some(size);
                        }
                    }
                    // **Merged onto** whatever an enclosing subtree already handed down,
                    // rather than replacing it: two nested subtrees each setting one field
                    // must leave a text wearing both.
                    if let Some(words) = words {
                        t.widgets.text = t.widgets.text.merge(words);
                    }
                },
                widget,
            )) as Box<dyn Widget<Msg>>
        };

        let mut row = Flex::row().align(Align::Center).gap(gap);
        if let Some(leading) = leading {
            row = row.child_boxed(dress(leading, icon_theme));
            if title_spacing > gap {
                row = row.child(Container::new().width(title_spacing - gap));
            }
        }
        // Centred: a spring on either side of the title. Otherwise one spring after it,
        // which is what pushes the actions to the right.
        if center_title {
            row = row.child(Container::new().flex(1.0));
        }
        // The bar's title is the **heading** of the screen it names — the landmark a
        // screen reader's user jumps to — and now that is true of *every* title rather
        // than only of one written as a string. The reference wraps the same way
        // (`app_bar.dart:1071`), and milestone 401's `Semantics` wrapper is what lets a
        // container state a role for a child it was handed already assembled.
        let title: Box<dyn Widget<Msg>> = if exclude_header_semantics {
            title
        } else {
            Box::new(crate::Semantics::heading(title))
        };
        // **The type, handed down rather than applied.** The words are cut by the box they
        // are given instead of by arithmetic here: `soft_wrap: false` and an ellipsis, on a
        // `Text` that never chose either, exactly as the reference sets them
        // (`app_bar.dart:1084`). Cutting the string by hand needed a width computed before
        // the layout ran, which is the class of mistake milestone 392 was.
        // And a **ceiling**, the reference's `_AppBarTitleBox` (`app_bar.dart:1069`). The
        // ellipsis alone would very nearly do it — a text handed an overflow mode grants
        // the squeeze, so flexbox may shrink it — but "very nearly" is how an over-long
        // task name evicted its own delete button in milestone 333. What is left after the
        // actions have taken theirs is a number the bar knows; saying it outright is one
        // line, and it is the line that makes the guarantee testable.
        row = row.child(
            ConstrainedBox::new(crate::Themed::tweak(
                move |t| t.widgets.text = t.widgets.text.merge(title_words),
                title,
            ))
            .max_width(title_room),
        );
        row = row.child(Container::new().flex(1.0));

        let mut labeled_seen = 0;
        let mut folded: Vec<(String, Msg)> = Vec::new();
        // The actions as a **group**, so that `actions_padding` insets the glyphs rather
        // than each button's own hit area. Without a padding the group is the row itself
        // and nothing is nested that was not nested before.
        let mut group = Flex::row().align(Align::Center).gap(gap);
        for action in actions {
            match action {
                Action::Custom(widget) => group = group.child_boxed(widget),
                Action::Labeled { label, message } => {
                    if labeled_seen < kept_labeled {
                        group = group.child(
                            button(label, message)
                                .variant(Variant::Outlined)
                                .size(action_size),
                        );
                    } else {
                        folded.push((label, message));
                    }
                    labeled_seen += 1;
                }
            }
        }

        if !folded.is_empty() {
            match overflow {
                // A controlled overflow menu: the glyph opens it, the items emit the
                // actions.
                Some((open, toggle)) => {
                    let mut menu = PopupMenuButton::new(
                        button(OVERFLOW_GLYPH, toggle.clone())
                            .variant(Variant::Outlined)
                            .size(action_size),
                        open,
                        toggle,
                    );
                    for (label, message) in folded {
                        menu = menu.item(label, message);
                    }
                    group = group.child(menu);
                }
                // No overflow configured: show everything inline (it may overflow).
                None => {
                    for (label, message) in folded {
                        group = group.child(
                            button(label, message)
                                .variant(Variant::Outlined)
                                .size(action_size),
                        );
                    }
                }
            }
        }
        // Unset, the actions follow the bar's icon theme; set, they part company with
        // the leading slot — a back arrow in the foreground colour beside actions in a
        // muted one, which is the case the reference's `actionsIconTheme` exists for.
        let group = dress(Box::new(group), actions_icon_theme.or(icon_theme));
        row = match actions_padding {
            Some(pad) => row.child(Container::new().padding(pad).child(group)),
            None => row.child_boxed(group),
        };

        // The toolbar proper: the row, with its horizontal margin and any imposed
        // height. The `bottom` sits under it inside the same background - it belongs to
        // the bar, rather than being something the application places beneath one.
        // The row is given the bar's width less its margins, so that the springs
        // inside it have something to share: a row that hugged its content would leave
        // a centred title with no space to be centred in.
        let row = if width.is_finite() {
            row.width((width - H_PAD * 2.0).max(0.0))
        } else {
            row
        };
        // The bar is a **fixed** height, the caller's or the reference's. It used to hug
        // its content, which made the chrome a different shape on every screen and moved
        // the page below it the moment an action appeared or a title wrapped.
        let mut toolbar = Container::new().padding_each(0.0, H_PAD, 0.0, H_PAD);
        toolbar = toolbar.height(height.unwrap_or(APP_BAR_HEIGHT));
        let toolbar = toolbar.child(row);

        // The two opacities are **group** opacities and independent of one another: a
        // collapsing header fades its contents out while its surface stays, and the
        // `bottom` — a tab strip, usually — fades on its own schedule.
        let toolbar: Box<dyn Widget<Msg>> = if toolbar_opacity < 1.0 {
            Box::new(Container::new().opacity(toolbar_opacity).child(toolbar))
        } else {
            Box::new(toolbar)
        };
        let bottom = bottom.map(|widget| -> Box<dyn Widget<Msg>> {
            if bottom_opacity < 1.0 {
                Box::new(Container::new().opacity(bottom_opacity).child(widget))
            } else {
                widget
            }
        });
        let content: Box<dyn Widget<Msg>> = match bottom {
            Some(bottom) => Box::new(Flex::column().child_boxed(toolbar).child_boxed(bottom)),
            None => toolbar,
        };
        // **The padding applies to the toolbar and the bottom, not to the surface** — the
        // reference says so in as many words (`app_bar.dart:1189`), and it is what makes a
        // Material bar look the way it does: the colour runs behind the status bar while
        // the title sits below it. The bottom edge is left alone; a bar has something under
        // it by definition.
        let content: Box<dyn Widget<Msg>> = if primary {
            Box::new(crate::SafeArea::new(content).edges(crate::Edges::ALL.without_bottom()))
        } else {
            content
        };

        // The bar **occupies** the width it was told about rather than hugging its
        // content. Hugging made two things quietly wrong: a background painted only
        // behind the text instead of across the bar, and a centred title with no free
        // space to be centred in.
        let mut chrome = Container::new();
        if width.is_finite() {
            chrome = chrome.width(width);
        }
        // **Transparency wins outright.** A caller asking for a bar over an image has
        // already decided; arguing with the background and the elevation it inherited
        // from a theme would only make that decision conditional on the theme.
        if !force_material_transparency {
            // Material 3 shows height by **tinting** the surface, not only by casting a
            // shadow: at elevation 3 the tint is 8%, and it is what still reads on a dark
            // background where a shadow shows nothing at all.
            let surface = match (background, surface_tint) {
                (Some(base), Some(tint)) => Some(base.surface_tint(tint, elevation)),
                (base, _) => base,
            };
            if let Some(color) = surface {
                chrome = chrome.color(color);
            }
            if elevation > 0.0 {
                let shadow = shadow_color.or(t.shadow_color).unwrap_or(SHADOW);
                chrome = chrome.shadow(0.0, elevation * 0.25, elevation, shadow);
            }
        }
        // The shape clips as well as rounds: a surface that stopped short of its own
        // corner would square off the one the shadow had already curved.
        if let Some(shape) = shape.or(t.shape) {
            chrome = chrome.radius(shape).clip();
        }
        // **Behind the toolbar, filling the bar's box.** A layer, not a slot: the bar is
        // exactly as tall as it was, however tall the widget would rather be. The bar's
        // own surface is painted by `chrome` underneath, so a translucent flexible space
        // still tints what the surface put there.
        let content: Box<dyn Widget<Msg>> = match flexible_space {
            Some(space) => Box::new(crate::Stack::new().layer(space).layer(content)),
            None => content,
        };
        Box::new(chrome.child(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        PopupMenuButton,
        A,
        B,
        C,
    }

    /// **A bar folds against the surface it was built for, with no caller saying how
    /// wide that is.**
    ///
    /// Milestone 393. The width used to be an argument, and an application that got it
    /// wrong got a bar that folded too early or ran off the edge. `AppBar::new` reads
    /// [`MediaQuery::of`] instead: the same three actions fit on a wide surface and
    /// fold into the overflow menu on a narrow one, and neither build says a number.
    #[test]
    fn a_bar_folds_against_the_surface_it_was_built_for() {
        let built = |width: f32| {
            let size = Size::new(width, 80.0);
            let bar = MediaQuery::new(size).scope(|| {
                AppBar::new("Title")
                    .overflow(false, Msg::PopupMenuButton)
                    .action("Action One", Msg::A)
                    .action("Action Two", Msg::B)
                    .action("Action Three", Msg::C)
                    .build()
            });
            let ui = build_ui(bar.as_ref(), size, &Runtime::default(), &Theme::default());
            ui.semantics()
                .iter()
                .filter(|(_, _, s)| s.role == frus_core::Role::Button)
                .count()
        };
        assert!(
            built(1000.0) > built(360.0),
            "a narrow surface folds what a wide one shows"
        );
    }

    /// The bar's surface as it is actually painted: the first filled rectangle as wide
    /// as the bar.
    fn surface(bar: &dyn Widget<Msg>, width: f32) -> Option<(Color, f32)> {
        let ui = build_ui(
            bar,
            Size::new(width, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // **Into the layers, not only across the top.** A clipped subtree is drained
        // into a composited `Layer`, so a bar with a shape paints nothing at the scene's
        // top level at all — which is what the first version of this helper concluded.
        fn find(primitives: &[crate::Primitive], width: f32) -> Option<(Color, f32)> {
            for p in primitives {
                match p {
                    // `blur > 0` is a **shadow**: drawn as wide as the thing casting it,
                    // and otherwise mistaken for the surface. This helper was, and
                    // reported a black at 22% alpha.
                    crate::Primitive::Rect {
                        rect,
                        color,
                        radius,
                        blur,
                        ..
                    } if rect.width >= width - 0.5 && *blur == 0.0 => {
                        return Some((*color, radius.top_left))
                    }
                    crate::Primitive::Layer { primitives, .. } => {
                        if let Some(found) = find(primitives, width) {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        find(ui.scene().primitives(), width)
    }

    /// **Material 3 shows height by tinting, not only by shadowing.**
    ///
    /// A shadow says a thing is above the page; the tint says how far, and on a dark
    /// background a shadow says nothing at all. The strength is the specification's
    /// table — 8% at elevation 3 — so the check is that the painted surface is the
    /// background moved **exactly** that far towards the tint, not merely that it moved.
    #[test]
    fn an_elevated_bar_is_tinted_by_its_elevation() {
        const W: f32 = 400.0;
        let base = Color::rgb(0.10, 0.10, 0.12);
        let tint = Color::rgb(0.60, 0.40, 1.00);
        let flat = AppBar::<Msg>::new("Title")
            .width(W)
            .background(base)
            .build();
        let raised = AppBar::<Msg>::new("Title")
            .width(W)
            .background(base)
            .elevation(3.0)
            .surface_tint(tint)
            .build();
        let (flat_color, _) = surface(flat.as_ref(), W).expect("a flat surface");
        let (raised_color, _) = surface(raised.as_ref(), W).expect("a raised surface");
        assert_eq!(flat_color, base, "nothing is tinted without a tint");
        let expected = base.lerp(tint, 0.08);
        assert!(
            (raised_color.r - expected.r).abs() < 1e-4
                && (raised_color.g - expected.g).abs() < 1e-4
                && (raised_color.b - expected.b).abs() < 1e-4,
            "elevation 3 should tint at 8%: {raised_color:?} against {expected:?}"
        );
    }

    /// **A bar told to be transparent has no surface at all** — no background, no tint,
    /// no shadow — whatever a theme handed it.
    ///
    /// The reference's `forceMaterialTransparency`, for a bar over an image where the
    /// chrome should be the controls and nothing else. It overrides rather than argues,
    /// because a caller asking for transparency has already decided.
    #[test]
    fn a_transparent_bar_paints_no_surface() {
        const W: f32 = 400.0;
        let bar = AppBar::<Msg>::new("Title")
            .width(W)
            .background(Color::rgb(0.9, 0.1, 0.1))
            .elevation(6.0)
            .surface_tint(Color::rgb(0.0, 1.0, 0.0))
            .force_material_transparency(true)
            .build();
        assert!(
            surface(bar.as_ref(), W).is_none(),
            "the bar painted a surface it was told not to"
        );
    }

    /// **The shape rounds the bar's corners**, and the caller's outranks the theme's.
    #[test]
    fn the_bar_takes_the_shape_it_was_given() {
        const W: f32 = 400.0;
        let square = AppBar::<Msg>::new("Title")
            .width(W)
            .background(Color::rgb(0.2, 0.2, 0.2))
            .build();
        let rounded = AppBar::<Msg>::new("Title")
            .width(W)
            .background(Color::rgb(0.2, 0.2, 0.2))
            .shape(frus_core::BorderRadius::uniform(18.0))
            .build();
        assert_eq!(surface(square.as_ref(), W).expect("square").1, 0.0);
        assert_eq!(surface(rounded.as_ref(), W).expect("rounded").1, 18.0);
    }

    /// **The bar's title is the screen's heading, and it was announced as a label.**
    ///
    /// A screen reader's user moves through a screen by its headings. The one every
    /// screen has is the bar's title, and ours carried `Role::Label` — one more piece of
    /// text among the rest, with nothing to jump to. The roadmap had this marked *done*
    /// since before milestone 397 and nothing in the framework emitted `Role::Heading` at
    /// all.
    #[test]
    fn the_bars_title_is_the_screens_heading() {
        const W: f32 = 400.0;
        let role_of_title = |exclude: bool| {
            let bar = AppBar::<Msg>::new("Inbox")
                .width(W)
                .exclude_header_semantics(exclude)
                .build();
            let ui = build_ui(
                bar.as_ref(),
                Size::new(W, 200.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.semantics()
                .iter()
                .find(|(_, _, s)| s.label.as_deref() == Some("Inbox"))
                .map(|(_, _, s)| s.role)
                .expect("the title is described")
        };
        assert_eq!(role_of_title(false), frus_core::Role::Heading);
        // And a bar that says its title is not the screen's name gets a plain label back.
        assert_eq!(role_of_title(true), frus_core::Role::Label);
    }

    /// **A widget title is the screen's heading too**, which it was not until now.
    ///
    /// Milestone 397 could mark a *text* title as a landmark and not a widget one, because
    /// by then a widget title is a `Box<dyn Widget>` and the bar had no way in. So the
    /// accessibility of a bar depended on which constructor the caller had reached for —
    /// not a distinction anybody using assistive technology should be able to feel, and not
    /// one the reference has: its `title` is a `Widget?` and nothing else, wrapped once in
    /// `Semantics(header: true)` (`app_bar.dart:1071`).
    ///
    /// The two constructors are now the same path, and this asserts they answer the same.
    #[test]
    fn a_widget_title_is_a_heading_exactly_as_a_string_one_is() {
        const W: f32 = 400.0;
        let role_of = |bar: Box<dyn Widget<Msg>>| {
            let ui = build_ui(
                bar.as_ref(),
                Size::new(W, 200.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.semantics()
                .iter()
                .find(|(_, _, s)| s.label.as_deref() == Some("Inbox"))
                .map(|(_, _, s)| s.role)
                .expect("the title is described")
        };
        let from_string = role_of(AppBar::<Msg>::new("Inbox").width(W).build());
        let from_widget = role_of(
            // The string a `new` needs is thrown away by the `title` that follows it.
            AppBar::<Msg>::new("")
                .title(crate::Text::new("Inbox"))
                .width(W)
                .build(),
        );
        assert_eq!(from_string, frus_core::Role::Heading);
        assert_eq!(
            from_widget, from_string,
            "the same bar, described differently by which constructor was used"
        );
    }

    /// **A widget title wears the bar's type**, without the bar reaching into it.
    ///
    /// The reference hands the title style down rather than applying it
    /// (`app_bar.dart:1084`), which is the only thing that works when the title is a
    /// caller's widget with a `Text` somewhere inside it. A text that chose its own size
    /// keeps it — that is milestone 400's rule, and it is what makes the handover safe to
    /// apply to a widget the bar never looked into.
    #[test]
    fn the_bar_hands_its_type_to_a_title_it_did_not_build() {
        const W: f32 = 500.0;
        let size_of = |bar: Box<dyn Widget<Msg>>| {
            let ui = build_ui(
                bar.as_ref(),
                Size::new(W, 200.0),
                &Runtime::default(),
                &Theme::default(),
            );
            let mut found = None;
            fn walk(prims: &[crate::Primitive], out: &mut Option<f32>) {
                for p in prims {
                    match p {
                        crate::Primitive::Text { text, size, .. } if text == "Inbox" => {
                            *out = Some(*size)
                        }
                        crate::Primitive::Layer { primitives, .. } => walk(primitives, out),
                        _ => {}
                    }
                }
            }
            walk(ui.scene().primitives(), &mut found);
            found.expect("the title is drawn")
        };
        let inherited = size_of(
            // The string a `new` needs is thrown away by the `title` that follows it.
            AppBar::<Msg>::new("")
                .title(crate::Text::new("Inbox"))
                .width(W)
                .build(),
        );
        assert_eq!(
            inherited,
            Theme::default().text.title_large.size.unwrap(),
            "a title that chose nothing wears the bar's type"
        );
        // And one that chose keeps its own, which is what makes the handover safe.
        let chosen = size_of(
            AppBar::<Msg>::new("")
                .title(crate::Text::new("Inbox").size(11.0))
                .width(W)
                .build(),
        );
        assert_eq!(chosen, 11.0);
    }

    /// Every path the bar paints, layers included.
    fn glyph_colors(bar: &dyn Widget<Msg>, width: f32) -> Vec<Color> {
        fn walk(primitives: &[crate::Primitive], out: &mut Vec<Color>) {
            for p in primitives {
                match p {
                    crate::Primitive::Path {
                        fill: Some(color), ..
                    } => out.push(*color),
                    crate::Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let ui = build_ui(
            bar,
            Size::new(width, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let mut out = Vec::new();
        walk(ui.scene().primitives(), &mut out);
        out
    }

    /// **The bar's icon theme reaches a glyph it never sees.**
    ///
    /// The reference's `IconTheme` is an *inherited* widget, and that is the whole point:
    /// an app bar cannot restyle an icon nested inside a button it was handed. Delivering
    /// the colour as a theme for the subtree is what reaches it. The check is a leading
    /// slot holding a plain `Icon`, recoloured without the bar touching the widget.
    #[test]
    fn the_bars_icon_theme_reaches_a_glyph_it_never_sees() {
        const W: f32 = 400.0;
        let wanted = Color::rgb(0.9, 0.2, 0.4);
        let plain = AppBar::<Msg>::new("Title")
            .width(W)
            .leading(crate::Icon::new(crate::Icons::Star))
            .build();
        let themed = AppBar::<Msg>::new("Title")
            .width(W)
            .leading(crate::Icon::new(crate::Icons::Star))
            .icon_theme(crate::widgettheme::IconTheme {
                color: Some(wanted),
                size: None,
            })
            .build();
        assert!(
            !glyph_colors(plain.as_ref(), W).contains(&wanted),
            "the plain bar should not already be that colour"
        );
        assert!(
            glyph_colors(themed.as_ref(), W).contains(&wanted),
            "the icon theme never reached the glyph"
        );
    }

    /// Every text in the built bar, as `(the words, their size)`. Composited layers are
    /// walked into: a clipped subtree is drained into one, and a bar with a shape clips.
    fn toolbar_texts(bar: &dyn Widget<Msg>, width: f32) -> Vec<(String, f32)> {
        fn walk(prims: &[crate::Primitive], out: &mut Vec<(String, f32)>) {
            for p in prims {
                match p {
                    crate::Primitive::Text { text, size, .. } => out.push((text.clone(), *size)),
                    crate::Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let ui = build_ui(
            bar,
            Size::new(width, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let mut out = Vec::new();
        walk(ui.scene().primitives(), &mut out);
        out
    }

    /// **The bar's toolbar style reaches a text it never sees, and not its title.**
    ///
    /// Same reasoning as the icon theme, and the same delivery: an app bar cannot restyle
    /// a run of words nested inside a widget it was handed, so the style goes down as a
    /// theme for the subtree rather than in as an argument.
    ///
    /// The second half is the one worth a test of its own. The reference keeps
    /// `toolbarTextStyle` and `titleTextStyle` apart, and a bar that let the first reach
    /// the title would quietly resize it — the one line in the bar that already had an
    /// answer, and the one whose width decides how many actions still fit.
    #[test]
    fn the_toolbar_style_reaches_the_words_around_the_title() {
        const W: f32 = 600.0;
        let size_of = |texts: &[(String, f32)], words: &str| {
            texts
                .iter()
                .find(|(t, _)| t == words)
                .map(|(_, s)| *s)
                .unwrap_or_else(|| panic!("no {words:?} in {texts:?}"))
        };
        let bar = |style: Option<TextStyle>| {
            let mut bar = AppBar::<Msg>::new("Title")
                .width(W)
                .leading(crate::Text::new("Back"));
            if let Some(style) = style {
                bar = bar.toolbar_text_style(style);
            }
            toolbar_texts(bar.build().as_ref(), W)
        };
        let plain = bar(None);
        let dressed = bar(Some(TextStyle::new(11.0)));
        assert!(
            size_of(&plain, "Back") > 11.0,
            "the plain bar should not already be at 11 px"
        );
        assert_eq!(size_of(&dressed, "Back"), 11.0, "the style never arrived");
        assert_eq!(
            size_of(&dressed, "Title"),
            size_of(&plain, "Title"),
            "and it must not have touched the title"
        );
    }

    /// **The actions may part company with the leading slot** — the reference's
    /// `actionsIconTheme`, for a back arrow in the foreground colour beside actions in a
    /// muted one. Unset, they follow the bar's own.
    #[test]
    fn the_actions_may_wear_their_own_icon_theme() {
        const W: f32 = 600.0;
        let lead = Color::rgb(0.1, 0.8, 0.2);
        let acts = Color::rgb(0.2, 0.1, 0.9);
        let bar = AppBar::<Msg>::new("Title")
            .width(W)
            .leading(crate::Icon::new(crate::Icons::Star))
            .action_widget(crate::Icon::new(crate::Icons::Heart))
            .icon_theme(crate::widgettheme::IconTheme {
                color: Some(lead),
                size: None,
            })
            .actions_icon_theme(crate::widgettheme::IconTheme {
                color: Some(acts),
                size: None,
            })
            .build();
        let colors = glyph_colors(bar.as_ref(), W);
        assert!(colors.contains(&lead), "the leading glyph kept its colour");
        assert!(colors.contains(&acts), "the action glyph took its own");
    }

    /// **The flexible space is a layer, not a slot**: it fills the bar's box and leaves
    /// its height alone, however tall the widget would rather be.
    #[test]
    fn the_flexible_space_does_not_make_the_bar_taller() {
        const W: f32 = 400.0;
        let height = |with_space: bool| {
            let mut bar = AppBar::<Msg>::new("Title").width(W);
            if with_space {
                bar = bar.flexible_space(Container::new().height(400.0));
            }
            let built = bar.build();
            let ui = build_ui(
                built.as_ref(),
                Size::new(W, 800.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    crate::Primitive::Text { position, .. } => Some(position.y),
                    _ => None,
                })
                .fold(0.0_f32, f32::max)
        };
        // The title sits where it sat: a 400 px flexible space did not push it down.
        assert!(
            (height(true) - height(false)).abs() < 0.5,
            "the flexible space moved the title by {}",
            height(true) - height(false)
        );
    }

    /// Counts the buttons (rectangles with a shadow), excluding floating menu items.
    fn inline_buttons(width: f32, open: bool) -> usize {
        let bar = AppBar::new("Title")
            .width(width)
            .overflow(open, Msg::PopupMenuButton)
            .action("Action One", Msg::A)
            .action("Action Two", Msg::B)
            .action("Action Three", Msg::C)
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(width, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Count the buttons by what they *are* rather than by what they paint: they used
        // to be counted by their shadows, and milestone 313 took the shadow off every
        // button but the elevated one — a proxy that stopped standing for anything.
        ui.semantics()
            .iter()
            .filter(|(_, _, s)| s.role == frus_core::Role::Button)
            .count()
    }

    /// The middle term milestone 318 could not add. `center_title` decides which children
    /// exist and in what order, so it cannot be resolved by any hook that runs after the
    /// row has been assembled — the composition is deferred to the theme instead.
    #[test]
    fn center_title_resolves_caller_then_theme_then_platform() {
        // A centred title is a spring, the title, another spring; a flush one is the
        // title straight after the leading. Counting the springs reads the decision.
        fn springs(widget: &dyn Widget<Msg>, theme: &crate::theme::Theme) -> usize {
            widget.build_themed(theme);
            let mine = usize::from(matches!(
                widget.style().flex_grow,
                g if g > 0.0
            ));
            mine + widget
                .children()
                .iter()
                .map(|c| springs(c.as_ref(), theme))
                .sum::<usize>()
        }
        let bar = || AppBar::<Msg>::new("Title").width(400.0);
        let plain = crate::theme::Theme::default();
        let mut centred = crate::theme::Theme::default();
        centred.widgets.app_bar.center_title = Some(true);
        let mut flush = crate::theme::Theme::default();
        flush.widgets.app_bar.center_title = Some(false);

        let untold = springs(bar().build().as_ref(), &plain);
        // Guard the instrument before trusting it: if centring did not change the
        // composition, every assertion below would pass while proving nothing.
        assert_ne!(
            springs(bar().center_title(true).build().as_ref(), &plain),
            springs(bar().center_title(false).build().as_ref(), &plain),
            "centring changes the row, or this test cannot see anything"
        );
        assert_eq!(
            springs(bar().build().as_ref(), &centred),
            springs(bar().center_title(true).build().as_ref(), &plain),
            "the theme centres it, as the caller would have"
        );
        assert_eq!(
            springs(bar().build().as_ref(), &flush),
            springs(bar().center_title(false).build().as_ref(), &plain),
            "and un-centres it"
        );
        // The caller outranks the theme, whichever way the theme leant.
        assert_eq!(
            springs(bar().center_title(false).build().as_ref(), &centred),
            springs(bar().center_title(false).build().as_ref(), &plain)
        );
        // And with neither, the platform still has the last word.
        assert_eq!(
            untold,
            springs(
                bar()
                    .center_title(platform_centers_title(0))
                    .build()
                    .as_ref(),
                &plain
            )
        );
    }

    /// Chrome is a **fixed** height, not its content's. A bar that hugged what happened
    /// to be in it made every screen a slightly different shape, and moved the page below
    /// it the moment an action appeared or a title grew.
    #[test]
    fn the_bar_is_the_references_height() {
        let plain = AppBar::<Msg>::new("Title").width(400.0).build();
        let busy = AppBar::<Msg>::new("Title")
            .width(400.0)
            .leading(button("=", Msg::A))
            .action("One", Msg::A)
            .action("Two", Msg::B)
            .build();
        assert_eq!(bar_height(plain.as_ref()), Some(APP_BAR_HEIGHT));
        assert_eq!(
            bar_height(busy.as_ref()),
            Some(APP_BAR_HEIGHT),
            "content does not change the chrome's height"
        );
        let tall = AppBar::<Msg>::new("Title")
            .width(400.0)
            .height(96.0)
            .build();
        assert_eq!(
            bar_height(tall.as_ref()),
            Some(96.0),
            "and the caller still decides"
        );
    }

    /// The toolbar's imposed height, wherever it sits in the composition.
    ///
    /// The bar defers its composition to the theme, so the walk down has to build it —
    /// exactly as the layout pass does — before the children are there to look at.
    fn bar_height(widget: &dyn Widget<Msg>) -> Option<f32> {
        widget.build_themed(&crate::theme::Theme::default());
        if let frus_layout::Dimension::Length(h) = widget.style().height {
            if (h - APP_BAR_HEIGHT).abs() < 0.01 || h > APP_BAR_HEIGHT {
                return Some(h);
            }
        }
        widget
            .children()
            .iter()
            .find_map(|c| bar_height(c.as_ref()))
    }

    #[test]
    fn wide_bar_shows_all_actions_inline() {
        // Wide enough: the 3 actions fit, and there is no overflow button.
        assert_eq!(inline_buttons(1200.0, false), 3);
    }

    #[test]
    fn narrow_bar_collapses_into_overflow() {
        // Narrow: at most one or two inline actions + the `⋯` button.
        let n = inline_buttons(300.0, false);
        assert!(
            n < 3,
            "expected a fold into overflow, got {n} inline buttons"
        );
        assert!(n >= 1, "the overflow button must be present");
    }

    /// The bar keeps a horizontal margin: the content (the leading on the left, the
    /// last action on the right) does not touch the viewport's edges.
    #[test]
    fn content_keeps_a_horizontal_margin() {
        const W: f32 = 400.0;
        let bar = AppBar::new("Title")
            .width(W)
            .leading(button("M", Msg::PopupMenuButton).size(16.0))
            .overflow(false, Msg::PopupMenuButton)
            .action("One", Msg::A)
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(W, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The horizontal bounds of the **texts** (title + labels): having no shadow,
        // they reflect the content's real position (a shadow's blur, by contrast,
        // legitimately overflows).
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        for p in ui.scene().primitives() {
            if let frus_core::Primitive::Text {
                position,
                size,
                text,
                ..
            } = p
            {
                min_x = min_x.min(position.x);
                // An approximate text width (an upper bound is enough here).
                max_x = max_x.max(position.x + text.chars().count() as f32 * size * 0.7);
            }
        }
        assert!(
            min_x >= H_PAD - 0.5,
            "content flush against the left edge ({min_x})"
        );
        assert!(
            max_x <= W - H_PAD + 0.5,
            "content overflowing on the right ({max_x} > {})",
            W - H_PAD
        );
    }

    /// **A bar outside a shell keeps its own content out of the status bar.**
    ///
    /// It did not, and that was the recorded cost of the shell owning the switch: the
    /// scaffold insetted the bar from outside, so a bar used on its own had nothing to
    /// inset it and would not inset itself. Milestone 417 gives it the reference's
    /// arrangement — the shell says what there is to consume, the bar consumes it.
    ///
    /// The **surface** still runs behind the status bar; only the toolbar is held clear.
    /// That is what a Material bar looks like, and what `app_bar.dart:1189` says in words.
    #[test]
    fn a_bar_on_its_own_clears_the_status_bar() {
        const TOP: f32 = 40.0;
        let title_y = |primary: bool| {
            let size = frus_core::Size::new(400.0, 200.0);
            let surface = crate::MediaQuery::new(size).with_insets(frus_core::WindowInsets::bars(
                frus_core::Insets::new(TOP, 0.0, 0.0, 0.0),
            ));
            // Built **and laid out** under the surface, the way a frame does it: the
            // shell holds one description across the build, the layout and the paint
            // (milestone 408), and a bar reads it when the walk reaches it.
            surface.scope(|| {
                let bar = AppBar::<Msg>::new("Inbox").primary(primary).build();
                crate::build_ui(
                    bar.as_ref(),
                    size,
                    &crate::Runtime::default(),
                    &Theme::default(),
                )
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { position, .. } => Some(position.y),
                    _ => None,
                })
                .expect("the title is drawn")
            })
        };
        assert!(
            (title_y(true) - title_y(false) - TOP).abs() < 0.5,
            "a primary bar holds its title off by exactly the intrusion: {} vs {}",
            title_y(true),
            title_y(false)
        );
        assert!(
            title_y(true) >= TOP,
            "and it is below the status bar, not under it: {}",
            title_y(true)
        );
    }

    #[test]
    fn title_style_is_customizable() {
        // An overridden title style: bold 24, instead of the default medium 20.
        let bar = AppBar::<Msg>::new("Title")
            .title_style(TextStyle::new(24.0).weight(FontWeight::Bold))
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(800.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                frus_core::Primitive::Text { text, size, weight, .. }
                    if text == "Title" && *size == 24.0 && *weight == FontWeight::Bold
            )
        });
        assert!(styled, "the title must carry the overridden style");
    }

    #[test]
    fn title_can_be_an_arbitrary_widget() {
        let bar = AppBar::<Msg>::new("ignored")
            .title(Text::new("Logo").size(18.0))
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(800.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let texts: Vec<_> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.contains(&"Logo".to_string()),
            "the title widget is rendered"
        );
        assert!(
            !texts.contains(&"ignored".to_string()),
            "the text title is replaced"
        );
    }

    /// The texts a bar paints, in paint order.
    fn texts_of(bar: &dyn Widget<Msg>, width: f32) -> Vec<String> {
        let ui = build_ui(
            bar,
            Size::new(width, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_title_too_long_for_the_bar_is_cut_not_pushed_off() {
        const W: f32 = 320.0;
        let bar = AppBar::new("A title far too long to fit in a narrow application bar")
            .width(W)
            .overflow(false, Msg::PopupMenuButton)
            .action("One", Msg::A)
            .build();
        let texts = texts_of(bar.as_ref(), W);
        let title = texts.first().expect("a title");
        assert!(title.ends_with('\u{2026}'), "cut with an ellipsis: {title}");
        assert!(
            title.len() < "A title far too long to fit in a narrow application bar".len(),
            "actually shortened: {title}"
        );
        // And the action survived: it is the actions that are served first.
        assert!(texts.iter().any(|t| t == "One"), "{texts:?}");
    }

    #[test]
    fn a_title_that_fits_is_left_exactly_as_it_was() {
        let bar = AppBar::<Msg>::new("Short").width(900.0).build();
        assert_eq!(
            texts_of(bar.as_ref(), 900.0).first().map(String::as_str),
            Some("Short")
        );
    }

    #[test]
    fn the_title_follows_the_platform_until_it_is_told_otherwise() {
        // The reference centres a bar's title where the system does, and only while
        // there is at most one action; everywhere else it is flush after the leading.
        // Whichever this build is, the caller's word wins over it.
        let bare = platform_centers_title(0);
        let crowded = platform_centers_title(2);
        assert!(
            !crowded || bare,
            "a platform that centres a crowded bar must centre a bare one"
        );
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            assert!(bare, "Apple's platforms centre a bare title");
            assert!(!crowded, "and stop once the actions crowd it");
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            assert!(!bare && !crowded, "everywhere else the title stays flush");
        }
    }

    #[test]
    fn a_centred_title_sits_between_the_two_ends() {
        const W: f32 = 800.0;
        let centred = AppBar::new("Task")
            .width(W)
            .center_title(true)
            .leading(button("M", Msg::PopupMenuButton).size(16.0))
            .build();
        // Explicitly flush: the default now follows the platform, and this test is
        // about the two arrangements, not about which one this build prefers.
        let flush = AppBar::new("Task")
            .width(W)
            .center_title(false)
            .leading(button("M", Msg::PopupMenuButton).size(16.0))
            .build();
        let x_of = |bar: &dyn Widget<Msg>| {
            let ui = build_ui(
                bar,
                Size::new(W, 80.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, position, .. } if text == "Task" => {
                        Some(position.x)
                    }
                    _ => None,
                })
                .expect("the title")
        };
        let centred_x = x_of(centred.as_ref());
        let flush_x = x_of(flush.as_ref());
        assert!(
            centred_x > flush_x + 100.0,
            "centred ({centred_x}) should sit well right of flush ({flush_x})"
        );
        assert!(centred_x < W * 0.6, "and not off to the right: {centred_x}");
    }

    #[test]
    fn a_bottom_slot_is_part_of_the_bar() {
        let bar = AppBar::<Msg>::new("Title")
            .width(600.0)
            .bottom(Text::new("Tabs").size(14.0))
            .build();
        let texts = texts_of(bar.as_ref(), 600.0);
        assert!(texts.iter().any(|t| t == "Tabs"), "{texts:?}");
        // Under the title, not beside it.
        let ui = build_ui(
            bar.as_ref(),
            Size::new(600.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let y = |wanted: &str| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, position, .. } if text == wanted => {
                        Some(position.y)
                    }
                    _ => None,
                })
                .expect("the text")
        };
        assert!(
            y("Tabs") > y("Title"),
            "the bottom slot sits under the toolbar"
        );
    }

    #[test]
    fn a_wide_leading_is_counted_in_the_budget() {
        // A leading far wider than the Material slot: declared, it must eat into what
        // the actions may claim, so more of them fold.
        let narrow = AppBar::new("Title")
            .width(520.0)
            .overflow(false, Msg::PopupMenuButton)
            .action("Action One", Msg::A)
            .action("Action Two", Msg::B)
            .build();
        let wide = AppBar::new("Title")
            .width(520.0)
            .leading_width(300.0)
            .leading(button("M", Msg::PopupMenuButton).size(16.0))
            .overflow(false, Msg::PopupMenuButton)
            .action("Action One", Msg::A)
            .action("Action Two", Msg::B)
            .build();
        let inline = |bar: &dyn Widget<Msg>| {
            texts_of(bar, 520.0)
                .iter()
                .filter(|t| t.starts_with("Action"))
                .count()
        };
        assert!(
            inline(wide.as_ref()) < inline(narrow.as_ref()),
            "a wide leading must push actions into the overflow"
        );
    }

    #[test]
    fn custom_widget_action_never_folds() {
        // A very narrow bar: the labelled actions fold, but the free widget (which
        // cannot be represented as a menu row) stays inline.
        let bar = AppBar::new("Title")
            .width(260.0)
            .overflow(false, Msg::PopupMenuButton)
            .action("A long labelled action", Msg::A)
            .action_widget(Text::new("★badge★").size(14.0))
            .action("Another long action", Msg::B)
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(260.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has_badge = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "★badge★"));
        assert!(
            has_badge,
            "the widget action stays inline even when cramped"
        );
    }
}
