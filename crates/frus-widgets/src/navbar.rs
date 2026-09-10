//! [`NavigationBar`]: a persistent navigation bar — a centred title, an optional back
//! button on the left. Placed at the head of a screen, it **slides and fades
//! with it** during [`crate::Navigator`] transitions.

#[cfg(test)]
use frus_core::FontWeight;
use frus_core::{Color, Insets, Point, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Bar height, in logical pixels.
const HEIGHT: f32 = 56.0;
/// Left margin: beyond the back-gesture zone, so the button stays clickable
/// without triggering the swipe.
const PAD_LEFT: f32 = 28.0;
/// The bar's own padding: nothing above or below (see [`NavigationBar::style`]), 16 at
/// the trailing edge, and the gesture-clearing inset at the leading one.
const PADDING: Insets = Insets {
    top: 0.0,
    right: 16.0,
    bottom: 0.0,
    left: PAD_LEFT,
};
/// The hairline along the bottom edge.
const DIVIDER_THICKNESS: f32 = 1.0;
/// The title's type: what the caller said, else the step the reference gives an app bar's
/// title — `titleLarge`. It used to be a private `20.0` at a medium weight, which had **both
/// halves wrong**: the reference's is 22 and regular.
fn title_style_of(over: Option<TextStyle>, theme: &Theme) -> TextStyle {
    over.or(theme.widgets.nav_bar.title_style)
        .unwrap_or(theme.text.title_large)
}

/// A navigation bar: a title + an optional back button.
pub struct NavigationBar<Msg> {
    title: String,
    /// The caller's title style, if one was named. Unset, the theme's `titleLarge`.
    title_style: Option<TextStyle>,
    /// The caller's height, if one was named. Unset, the theme's, then [`HEIGHT`].
    height: Option<f32>,
    /// `[]` (root) or `[back button]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavigationBar<Msg> {
    /// Creates a root bar: the title alone, with no back button.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            title_style: None,
            height: None,
            children: Vec::new(),
        }
    }

    /// Overrides the title style (size/weight/italic/color).
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// Overrides the bar height (the theme's, else 56 px).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Adds a back button that emits `message`.
    ///
    /// It used to build `IconButton::glyph("←")` — a **character**, at the mercy of
    /// whatever font is loaded, with no control over its weight and none over its size
    /// beside the glyphs around it, and named by hand here rather than by the thing that
    /// knows what it is. [`BackButton`](crate::BackButton) is that thing.
    pub fn on_back(mut self, message: Msg) -> Self {
        self.children = vec![Box::new(
            crate::BackButton::new().icon_size(20.0).on_press(message),
        )];
        self
    }
}

/// The resolvers, in an impl of their own: the builders above ask for `Msg: Clone +
/// 'static` because they hold a message, and the `Widget` impl does not — a resolver
/// declared beside the builders could not be called from the paint.
impl<Msg> NavigationBar<Msg> {
    /// `what the caller said ?? what the theme says ?? what the framework ships` — the
    /// order every property here resolves in, and the one an added theme must not
    /// disturb: a bar told to be 72 tall is 72 tall in an application whose theme says
    /// 64.
    fn bar_height(&self, theme: Option<&Theme>) -> f32 {
        self.height
            .or_else(|| theme.and_then(|t| t.widgets.nav_bar.height))
            .unwrap_or(HEIGHT)
    }

    /// The bar's padding. There is no per-call override for it, so this is the theme's
    /// answer or the framework's.
    fn padding_of(theme: Option<&Theme>) -> Insets {
        theme
            .and_then(|t| t.widgets.nav_bar.padding)
            .unwrap_or(PADDING)
    }

    /// What the system has taken at the top of the screen and beside it, which a bar at
    /// the head of a screen is under — read **when it is asked for**, so a
    /// [`Scaffold`](crate::Scaffold) that has already decided about the status bar, or a
    /// [`SafeArea`](crate::SafeArea) that has consumed it, is believed.
    ///
    /// The bottom is not the bar's: something is under it by definition.
    fn clearance() -> Insets {
        let taken = crate::MediaQuery::of().padding;
        Insets::new(taken.top, taken.right, 0.0, taken.left)
    }

    /// The bar's height and padding with the clearance added. The intrusion is **added to
    /// the bar, not taken out of it** — the background runs up behind the status bar while
    /// the title and the button keep their full height underneath, which is how the
    /// reference's persistent navigation bar is sized (its height plus the top padding).
    fn sizing(&self, theme: Option<&Theme>) -> (f32, Insets) {
        let clear = Self::clearance();
        let pad = Self::padding_of(theme);
        (
            self.bar_height(theme) + clear.top,
            Insets::new(
                pad.top + clear.top,
                pad.right + clear.right,
                pad.bottom,
                pad.left + clear.left,
            ),
        )
    }
}

impl<Msg: Clone> Widget<Msg> for NavigationBar<Msg> {
    fn style(&self) -> Style {
        Style {
            // **The bar fills the width it is offered.** It used to say `Auto`, which in a
            // row means *hug your children*: given no width by its parent it came out the
            // size of the back button, and `paint` — which centres the title in the box it
            // is given, the only thing it can do — put the title underneath the button.
            //
            // Every screen in the demo happens to hand it a width, so this never showed in
            // the application; it showed the first time the widget was rendered on its own
            // (milestone 296). A chrome that spans the head of a screen has no other
            // sensible answer to "how wide would you like to be", and an app bar gives the
            // same one.
            width: Dimension::Percent(1.0),
            height: Dimension::Length(self.sizing(None).0),
            flex_direction: FlexDirection::Row,
            justify: Justify::Start,
            align: Align::Center,
            // **No vertical padding**, and that is the reference's arrangement rather than
            // a saving: a toolbar is 56 tall and holds a 48-pixel button centred in it
            // (`constants.dart:27`, `constants.dart:30`), with the four pixels either side
            // coming from the difference and not from a rule. Six pixels of padding left 44
            // for the button, which is under the target it now reserves (milestone 442) —
            // the bar squeezed the one control in it.
            //
            // The status bar, when the bar is under one, is added on top of that — see
            // [`NavigationBar::sizing`].
            padding: self.sizing(None).1,
            ..Default::default()
        }
    }

    /// The theme has a say in the **height and the padding**, not only the colours: a bar
    /// that took its colours from the theme and its size from a constant would be themed
    /// in the half that is easy to see and unthemed in the half that decides where
    /// everything under it starts.
    fn style_themed(&self, theme: &Theme) -> Style {
        let (height, padding) = self.sizing(Some(theme));
        Style {
            height: Dimension::Length(height),
            padding,
            ..Widget::<Msg>::style(self)
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let t = &theme.widgets.nav_bar;
        // Background + a thin bottom separator.
        let background: Color = t.background.unwrap_or(theme.background);
        scene.fill_rect(bounds, background.fade(o));
        // The hairline never draws thicker than the bar it sits in, the way a divider's
        // never outgrows its own box.
        let thickness = t
            .divider_thickness
            .unwrap_or(DIVIDER_THICKNESS)
            .min(bounds.height);
        let rule: Color = t.divider_color.unwrap_or(theme.scheme.outline_variant);
        scene.fill_rect(
            Rect::new(
                bounds.x,
                bounds.y + bounds.height - thickness,
                bounds.width,
                thickness,
            ),
            rule.fade(o),
        );

        // The title is centred in the part of the bar **below the status bar and between
        // the cutouts** — the background above runs behind the system's bar, the title
        // does not — following `title_style` (the style's color is inherited from the
        // theme when absent).
        let clear = Self::clearance();
        let room = Rect::new(
            bounds.x + clear.left,
            bounds.y + clear.top,
            (bounds.width - clear.left - clear.right).max(0.0),
            (bounds.height - clear.top).max(0.0),
        );
        let style = title_style_of(self.title_style, theme);
        let measured = frus_text::measure_style(&self.title, style);
        let tx = room.x + (room.width - measured.width) * 0.5;
        let ty = room.y + (room.height - measured.height) * 0.5;
        scene.text(
            Point::new(tx, ty),
            self.title.clone(),
            &style.resolved(),
            style.color.unwrap_or(theme.on_surface).fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Color;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Back,
    }

    #[test]
    fn root_bar_has_no_back_button() {
        let bar: NavigationBar<Msg> = NavigationBar::new("Home");
        assert!(Widget::children(&bar).is_empty());
    }

    #[test]
    fn title_style_and_height_are_customizable() {
        // Overridden title style and height (defaults: medium 20, 56 px).
        let bar: NavigationBar<Msg> = NavigationBar::new("Title")
            .title_style(TextStyle::new(24.0).weight(FontWeight::Bold).italic())
            .height(72.0);
        match Widget::style(&bar).height {
            Dimension::Length(h) => assert_eq!(h, 72.0),
            _ => panic!("an imposed height was expected"),
        }
        let ui = build_ui(
            &bar,
            Size::new(400.0, 72.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Text { text, size, weight, italic, .. }
                    if text == "Title" && *size == 24.0 && *weight == FontWeight::Bold && *italic
            )
        });
        assert!(styled, "the title must carry the overridden style");
    }

    #[test]
    fn back_button_emits_message() {
        let bar: NavigationBar<Msg> = NavigationBar::new("Settings").on_back(Msg::Back);
        let ui = build_ui(
            &bar,
            Size::new(400.0, 56.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The back button is on the left; a click there returns the back message.
        let id = ui.hit(Point::new(40.0, 28.0)).expect("back button");
        assert_eq!(ui.msg_for(id), Some(Msg::Back));
    }

    /// **A bar given no width used to hug its back button**, and `paint` — which centres
    /// the title in the box it is handed, the only thing it can do — put the title
    /// underneath the button.
    ///
    /// A row is the arrangement that shows it: a row hands each child the width it asks
    /// for, and `Auto` asks for the width of its contents. Every screen in the demo
    /// happens to give the bar a width, which is why this survived 296 milestones without
    /// being seen and then showed the first time the widget was rendered on its own.
    #[test]
    fn a_bar_given_no_width_still_spans_what_it_is_offered() {
        const FRAME: f32 = 400.0;
        let bar: NavigationBar<Msg> = NavigationBar::new("Settings").on_back(Msg::Back);
        // A row of a known width whose child is asked for its own: the row spans the
        // frame, and a child saying `Auto` there is handed exactly what it hugs. The
        // width has to be definite for the bar's own answer to mean anything — a
        // percentage of an undecided width is undecided too.
        let root = crate::Flex::row().width(FRAME).child(bar);
        let theme = Theme::default();
        let ui = build_ui(&root, Size::new(FRAME, HEIGHT), &Runtime::default(), &theme);
        let at = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Text { text, position, .. } if text == "Settings" => Some(*position),
                _ => None,
            })
            .expect("the title is painted");
        let measured = frus_text::measure_style("Settings", title_style_of(None, &theme));
        let centred = (FRAME - measured.width) * 0.5;
        assert!(
            (at.x - centred).abs() < 1.0,
            "the title is centred in the frame: {} against {centred}",
            at.x
        );
        // And the thing the bug actually looked like: the title sat on the button.
        assert!(
            at.x > PAD_LEFT + crate::ICON_BUTTON_SIZE,
            "the title is clear of the back button: {} against {}",
            at.x,
            PAD_LEFT + crate::ICON_BUTTON_SIZE
        );
    }

    /// Every rectangle the bar paints, in order: the background first, the hairline on
    /// top of it.
    fn rects(bar: &NavigationBar<Msg>, theme: &Theme, frame: Size) -> Vec<(Rect, Color)> {
        let ui = build_ui(bar, frame, &Runtime::default(), theme);
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// **Every field of the theme reaches the painting** — and the two that are not
    /// colours are the ones worth naming, because a bar themed in its colours and sized
    /// from a constant is themed in the half that shows and unthemed in the half that
    /// decides where everything under it starts.
    ///
    /// The colours are asserted on the scene rather than on a rendered pixel: this widget
    /// hands an opaque colour straight to `fill_rect` and does no blending arithmetic of
    /// its own, and that a colour asked for is the colour painted is pinned once for the
    /// quad path in `frus-test`'s `painted_colours`, not once per widget.
    #[test]
    fn a_theme_reaches_every_property_the_bar_paints() {
        const BACKGROUND: Color = Color::rgb(0.10, 0.20, 0.30);
        const RULE: Color = Color::rgb(0.90, 0.10, 0.40);
        let mut theme = Theme::default();
        theme.widgets.nav_bar.height = Some(64.0);
        theme.widgets.nav_bar.padding = Insets::new(0.0, 4.0, 0.0, 40.0).into();
        theme.widgets.nav_bar.background = Some(BACKGROUND);
        theme.widgets.nav_bar.divider_color = Some(RULE);
        theme.widgets.nav_bar.divider_thickness = Some(3.0);
        theme.widgets.nav_bar.title_style = Some(TextStyle::new(18.0).weight(FontWeight::Bold));

        let bar: NavigationBar<Msg> = NavigationBar::new("Title").on_back(Msg::Back);
        // The height and the padding are layout, so they have to come back through the
        // themed style — the unthemed one is what a parent asks before a theme exists.
        match Widget::<Msg>::style_themed(&bar, &theme).height {
            Dimension::Length(h) => assert_eq!(h, 64.0, "the theme's height"),
            other => panic!("a definite height was expected, got {other:?}"),
        }
        assert_eq!(
            Widget::<Msg>::style_themed(&bar, &theme).padding,
            Insets::new(0.0, 4.0, 0.0, 40.0),
            "the theme's padding"
        );

        let painted = rects(&bar, &theme, Size::new(400.0, 64.0));
        let (bg_rect, bg) = painted[0];
        assert_eq!(bg, BACKGROUND, "the theme's background");
        assert_eq!(bg_rect.height, 64.0, "and it fills the themed height");
        let (rule_rect, rule) = painted[1];
        assert_eq!(rule, RULE, "the theme's hairline colour");
        assert_eq!(rule_rect.height, 3.0, "and its thickness");
        assert_eq!(
            rule_rect.y + rule_rect.height,
            bg_rect.y + bg_rect.height,
            "the hairline sits on the bottom edge whatever it weighs"
        );

        let ui = build_ui(&bar, Size::new(400.0, 64.0), &Runtime::default(), &theme);
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(p, Primitive::Text { text, size, weight, .. }
                if text == "Title" && *size == 18.0 && *weight == FontWeight::Bold)
        });
        assert!(styled, "the theme's title style");
    }

    /// **What the caller said outranks what the theme says.** The middle term of
    /// `caller ?? theme ?? framework` is the one an added theme can quietly promote, and
    /// a bar told to be 72 tall in an application whose theme says 64 is 72 tall.
    #[test]
    fn a_call_sites_own_values_beat_the_theme() {
        let mut theme = Theme::default();
        theme.widgets.nav_bar.height = Some(64.0);
        theme.widgets.nav_bar.title_style = Some(TextStyle::new(18.0));

        let bar: NavigationBar<Msg> = NavigationBar::new("Title")
            .height(72.0)
            .title_style(TextStyle::new(30.0));
        match Widget::<Msg>::style_themed(&bar, &theme).height {
            Dimension::Length(h) => assert_eq!(h, 72.0, "the caller's height, not the theme's"),
            other => panic!("a definite height was expected, got {other:?}"),
        }
        let ui = build_ui(&bar, Size::new(400.0, 72.0), &Runtime::default(), &theme);
        let sized = ui.scene().primitives().iter().any(
            |p| matches!(p, Primitive::Text { text, size, .. } if text == "Title" && *size == 30.0),
        );
        assert!(sized, "the caller's title style, not the theme's");

        // And with nothing said at the call site, the theme is what answers — otherwise
        // the test above would pass on a bar that ignores the theme entirely.
        let plain: NavigationBar<Msg> = NavigationBar::new("Title");
        match Widget::<Msg>::style_themed(&plain, &theme).height {
            Dimension::Length(h) => assert_eq!(h, 64.0, "the theme's height"),
            other => panic!("a definite height was expected, got {other:?}"),
        }
    }

    /// The bar's own background, told apart from anything a shell around it paints.
    const BAR_BG: Color = Color::rgb(0.12, 0.34, 0.56);
    const STATUS: f32 = 48.0;
    const SCREEN: Size = Size::new(400.0, 800.0);

    /// A phone: a status bar of [`STATUS`] at the top, nothing else taken.
    fn phone() -> crate::MediaQuery {
        crate::MediaQuery::new(SCREEN).with_insets(frus_core::WindowInsets::bars(Insets::new(
            STATUS, 0.0, 0.0, 0.0,
        )))
    }

    /// Where the title is painted and where the bar's background is, for `root` built
    /// and walked under `surface` — and the built frame, to click on.
    fn painted_under(
        surface: crate::MediaQuery,
        root: &dyn Widget<Msg>,
    ) -> (Point, Rect, crate::Ui<Msg>) {
        let mut theme = Theme::default();
        theme.widgets.nav_bar.background = Some(BAR_BG);
        let ui = surface.scope(|| build_ui(root, SCREEN, &Runtime::default(), &theme));
        let title = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Text { text, position, .. } if text == "Task" => Some(*position),
                _ => None,
            })
            .expect("the title is painted");
        let background = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == BAR_BG => Some(*rect),
                _ => None,
            })
            .expect("the bar's background is painted");
        (title, background, ui)
    }

    fn screen_with(bar: NavigationBar<Msg>) -> crate::Flex<Msg> {
        crate::Flex::column()
            .width(SCREEN.width)
            .height(SCREEN.height)
            .child(bar)
    }

    /// **A bar at the head of a screen drew under the status bar** (milestone 504), and
    /// always had: the title and the back button sat behind the clock. The scaffold told
    /// its app-bar slot what the status bar took, `AppBar` read it, and this bar never
    /// did. Found on a device while checking the colour of the system bars (#46).
    ///
    /// The intrusion is added to the bar, as a drawer header adds it: the background runs
    /// up behind the status bar and the content keeps its full height below.
    #[test]
    fn a_bar_at_the_head_of_a_screen_clears_the_status_bar() {
        let (flat, _, _) = painted_under(
            crate::MediaQuery::new(SCREEN),
            &screen_with(NavigationBar::new("Task")),
        );
        let (title, background, ui) = painted_under(
            phone(),
            &screen_with(NavigationBar::new("Task").on_back(Msg::Back)),
        );
        assert_eq!(
            background.y, 0.0,
            "the background runs behind the status bar"
        );
        assert_eq!(
            background.height,
            HEIGHT + STATUS,
            "and the bar grows by it rather than giving it up"
        );
        assert!(
            (title.y - (flat.y + STATUS)).abs() < 0.5,
            "the title is centred below the status bar: {} against {}",
            title.y,
            flat.y + STATUS
        );
        // The button moved with it: pressed below the status bar it answers, and behind
        // the clock there is nothing of it.
        let below = ui
            .hit(Point::new(40.0, STATUS + HEIGHT / 2.0))
            .expect("the back button, below the status bar");
        assert_eq!(ui.msg_for(below), Some(Msg::Back));
        assert!(
            ui.hit(Point::new(40.0, STATUS / 2.0))
                .and_then(|id| ui.msg_for(id))
                .is_none(),
            "the back button is not under the status bar"
        );
    }

    /// **A bar inside a `SafeArea` is not pushed down twice.** Most of the demo's screens
    /// are a column in a `SafeArea` with this bar at the head; the safe area has taken the
    /// status bar, so the bar must be told there is nothing left to clear.
    #[test]
    fn a_bar_in_a_safe_area_does_not_clear_the_status_bar_again() {
        let (flat, _, _) = painted_under(
            crate::MediaQuery::new(SCREEN),
            &screen_with(NavigationBar::new("Task")),
        );
        let root = crate::Flex::column()
            .width(SCREEN.width)
            .height(SCREEN.height)
            .child(crate::SafeArea::new(
                crate::Flex::column().child(NavigationBar::new("Task")),
            ));
        let (title, background, _) = painted_under(phone(), &root);
        assert_eq!(
            background.y, STATUS,
            "the safe area holds the bar below the notch"
        );
        assert_eq!(
            background.height, HEIGHT,
            "and the bar adds nothing of its own"
        );
        assert!(
            (title.y - (flat.y + STATUS)).abs() < 0.5,
            "the title is one status bar down, not two: {} against {}",
            title.y,
            flat.y + STATUS
        );
    }

    /// In a scaffold's app-bar slot — the demo's task screen — the shell tells the bar what
    /// the status bar takes, and the bar clears it as it does at the head of a column.
    #[test]
    fn a_bar_in_a_scaffolds_app_bar_slot_clears_the_status_bar() {
        let (flat, _, _) = painted_under(
            crate::MediaQuery::new(SCREEN),
            &screen_with(NavigationBar::new("Task")),
        );
        // Built under the surface, as an application's view is: the scaffold reads its
        // intrusions when it is constructed.
        let scaffold = phone().scope(|| {
            crate::Scaffold::new()
                .size(SCREEN.width, SCREEN.height)
                .app_bar(NavigationBar::new("Task"))
                .body(crate::Container::new().flex(1.0))
                .build()
        });
        let (title, background, _) = painted_under(phone(), &scaffold);
        assert_eq!(background.y, 0.0);
        assert_eq!(background.height, HEIGHT + STATUS);
        assert!((title.y - (flat.y + STATUS)).abs() < 0.5, "{}", title.y);
    }

    #[test]
    fn bar_paints_title_and_divider() {
        let bar: NavigationBar<Msg> = NavigationBar::new("Title");
        let ui = build_ui(
            &bar,
            Size::new(400.0, 56.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has_text = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Title"));
        assert!(has_text, "the title is painted");
    }
}
