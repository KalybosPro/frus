//! [`AppBar`]: an **adaptive application bar**, Material style.
//!
//! The developer declares **one** title, an optional `leading` and a list of
//! **actions** — never saying "this is for mobile / desktop". The AppBar decides on
//! its own, from the **available width**, how many actions fit inline and **folds
//! the rest into a `⋯` overflow menu**. A wide screen → everything inline; a narrow
//! phone → overflow. One piece of code, adapting automatically.
//!
//! **Everything is customisable** (themed defaults, never imposed): the title can
//! be an arbitrary widget (`title_widget`) or styled text (`title_style`), an
//! action can be an arbitrary widget (`action_widget`, always inline), and the
//! spacing, the action size, the background and the height can all be overridden.
//!
//! ```ignore
//! AppBar::new("My Tasks")
//!     .width(available_width)                 // a size, not a platform
//!     .title_style(TextStyle::new(22.0))      // or .title_widget(logo_row)
//!     .leading(button("☰", Msg::ToggleMenu))
//!     .overflow(app.menu_open, Msg::ToggleMenu)
//!     .action("Pause", Msg::ToggleTimer)
//!     .action_widget(Badge::new("3"))         // a free widget, never folded
//!     .action("Settings →", Msg::OpenSettings)
//!     .build()
//! ```

use frus_core::{Color, FontWeight, TextStyle};
use frus_layout::{Align, Dimension};

use crate::button::Variant;
use crate::container::Container;
use crate::dsl::button;
use crate::flex::Flex;
use crate::media::MediaQuery;
use crate::menu::PopupMenuButton;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

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
/// The title's font size — the reference's `title_large` (the default, overridden by
/// [`AppBar::title_style`]).
const TITLE_SIZE: f32 = 22.0;
/// The actions' font size — the reference's `label_large`, which is what its app bar's
/// actions are: text buttons (the default, overridden by [`AppBar::action_size`]).
const ACTION_SIZE: f32 = 14.0;
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
enum Title<Msg> {
    Text(String),
    Widget(Box<dyn Widget<Msg>>),
}

/// An action: labelled (foldable into the overflow) or a free widget (always
/// inline — an arbitrary widget cannot become a text menu row).
enum Action<Msg> {
    Labeled { label: String, message: Msg },
    Custom(Box<dyn Widget<Msg>>),
}

/// An adaptive application bar. A fluent builder finished by [`AppBar::build`].
pub struct AppBar<Msg> {
    title: Title<Msg>,
    title_style: TextStyle,
    /// Was the title's style left at the framework's default? Only then may the theme
    /// have its say — a caller who set one outranks it.
    title_style_default: bool,
    width: f32,
    leading: Option<Box<dyn Widget<Msg>>>,
    overflow: Option<(bool, Msg)>,
    actions: Vec<Action<Msg>>,
    action_size: f32,
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
            title: Title::Text(title.into()),
            title_style: TextStyle::new(TITLE_SIZE).weight(FontWeight::Medium),
            title_style_default: true,
            width: if surface.is_described() {
                surface.size.width
            } else {
                f32::MAX
            },
            leading: None,
            overflow: None,
            actions: Vec::new(),
            action_size: ACTION_SIZE,
            gap: GAP,
            background: None,
            height: None,
            center_title: None,
            bottom: None,
            leading_width: None,
            title_spacing: GAP * 2.0,
            foreground: None,
            elevation: 0.0,
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
        self.title_style = style;
        self.title_style_default = false;
        self
    }

    /// Replaces the title with an **arbitrary widget** — a logo, a composed row, and
    /// so on.
    pub fn title_widget(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.title = Title::Widget(Box::new(widget));
        self
    }

    /// The leading element (a menu or back button…), optional.
    pub fn leading(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.leading = Some(Box::new(widget));
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
        self.action_size = size;
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

    /// The width an action button would take for this label.
    fn action_width(label: &str, size: f32) -> f32 {
        frus_text::measure(label, size).width + BTN_PAD_X * 2.0
    }

    /// A widget's declared width (0 if it depends on layout).
    fn widget_width(widget: &dyn Widget<Msg>) -> f32 {
        match widget.style().width {
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
            mut title_style,
            title_style_default,
            width,
            leading,
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
        } = self;

        // `caller ?? theme ?? platform`. The platform is the last word rather than the
        // framework's own taste, because where a title sits is a system convention before
        // it is a design one — see `platform_centers_title`.
        let center_title = center_title
            .or(t.center_title)
            .unwrap_or_else(|| platform_centers_title(actions.len()));

        // The title's type, likewise: the caller's style, else the theme's, else the
        // reference's `title_large`.
        if title_style_default {
            if let Some(style) = t.title_style {
                title_style = style;
            }
        }
        let foreground = foreground
            .or(t.foreground)
            .unwrap_or(theme.scheme.on_surface);
        title_style.color = Some(foreground);
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
            (Some(widget), None) => Self::widget_width(widget.as_ref()).max(LEADING_SLOT),
        };
        let natural_title = match &title {
            Title::Text(content) => {
                frus_text::measure_styled(
                    content,
                    title_style.size,
                    title_style.weight,
                    title_style.italic,
                )
                .width
            }
            Title::Widget(widget) => Self::widget_width(widget.as_ref()),
        };

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
                Action::Custom(widget) => Self::widget_width(widget.as_ref()) + gap,
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

        let mut row = Flex::row().align(Align::Center).gap(gap);
        if let Some(leading) = leading {
            row = row.child(leading);
            if title_spacing > gap {
                row = row.child(Container::new().width(title_spacing - gap));
            }
        }
        // Centred: a spring on either side of the title. Otherwise one spring after it,
        // which is what pushes the actions to the right.
        if center_title {
            row = row.child(Container::new().flex(1.0));
        }
        match title {
            Title::Text(content) => {
                let content = crate::text::truncate(&content, &title_style, title_room);
                row = row.child(Text::styled(content, title_style));
            }
            Title::Widget(widget) => row = row.child(widget),
        }
        row = row.child(Container::new().flex(1.0));

        let mut labeled_seen = 0;
        let mut folded: Vec<(String, Msg)> = Vec::new();
        for action in actions {
            match action {
                Action::Custom(widget) => row = row.child(widget),
                Action::Labeled { label, message } => {
                    if labeled_seen < kept_labeled {
                        row = row.child(
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
                    row = row.child(menu);
                }
                // No overflow configured: show everything inline (it may overflow).
                None => {
                    for (label, message) in folded {
                        row = row.child(
                            button(label, message)
                                .variant(Variant::Outlined)
                                .size(action_size),
                        );
                    }
                }
            }
        }

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

        let content: Box<dyn Widget<Msg>> = match bottom {
            Some(bottom) => Box::new(Flex::column().child(toolbar).child(bottom)),
            None => Box::new(toolbar),
        };

        // The bar **occupies** the width it was told about rather than hugging its
        // content. Hugging made two things quietly wrong: a background painted only
        // behind the text instead of across the bar, and a centred title with no free
        // space to be centred in.
        let mut chrome = Container::new();
        if width.is_finite() {
            chrome = chrome.width(width);
        }
        if let Some(color) = background {
            chrome = chrome.color(color);
        }
        if elevation > 0.0 {
            chrome = chrome.shadow(0.0, elevation * 0.25, elevation, SHADOW);
        }
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
            .title_widget(Text::new("Logo").size(18.0))
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
