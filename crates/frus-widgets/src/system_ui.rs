//! **The platform's own bars** — the status bar across the top of a phone, the navigation
//! bar along its bottom — and what colour they are and how their icons are drawn.
//!
//! They are not this framework's to paint: the platform draws them, beside or over the
//! window. What it has to be told is their colour and whether their icons are dark or
//! light, and the cost of never telling it is the bug #46 opened on — a light screen under
//! the platform's light icons, and the clock, the battery and the signal simply gone.
//!
//! ## Who decides
//!
//! Not the application, once, at start-up: **the part of the screen underneath**. A screen
//! with a photograph behind the status bar wants light icons, and the settings screen next
//! to it wants dark ones. So a subtree says what it wants with an [`AnnotatedRegion`], and
//! each frame asks which regions lie under each bar — the one drawn last first, **field by
//! field**, so a region that only states its icons still takes its colour from the one
//! beneath it. What no region states comes from the theme: bars in its background colour,
//! with icons that can be read on it.
//!
//! On a desktop and on the web there are no such bars, and nothing is sent anywhere.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::media::Brightness;
use crate::theme::Theme;
use crate::widget::Widget;

/// What a part of the screen asks of the system bars over it — the reference's
/// `SystemUiOverlayStyle`.
///
/// Every field is optional: a region states what it cares about, and the rest comes from
/// the region beneath it and finally from the theme ([`SystemUiOverlayStyle::resolve`]).
///
/// The icons are named by **their own** brightness: `Brightness::Dark` is dark icons, for
/// a light bar. That is the question a reader of the screen answers — can I read the clock
/// — and the platform's own name for it, a "light status bar", is the opposite word.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemUiOverlayStyle {
    /// The status bar's background.
    pub status_bar_color: Option<Color>,
    /// The status bar's icons: the clock, the battery, the signal.
    pub status_bar_icons: Option<Brightness>,
    /// The navigation bar's background.
    pub navigation_bar_color: Option<Color>,
    /// The navigation bar's icons, or its gesture handle.
    pub navigation_bar_icons: Option<Brightness>,
}

impl SystemUiOverlayStyle {
    /// States nothing: every field is left to what lies beneath.
    pub const NONE: Self = Self {
        status_bar_color: None,
        status_bar_icons: None,
        navigation_bar_color: None,
        navigation_bar_icons: None,
    };

    /// Both bars in `color`, with icons that can be read on it.
    pub fn for_background(color: Color) -> Self {
        let icons = icons_on(color);
        Self {
            status_bar_color: Some(color),
            status_bar_icons: Some(icons),
            navigation_bar_color: Some(color),
            navigation_bar_icons: Some(icons),
        }
    }

    /// The status bar's background. Its icons, unless stated too, are chosen to be read
    /// on it — not taken from the theme, which answers for a different colour.
    pub fn status_bar_color(mut self, color: Color) -> Self {
        self.status_bar_color = Some(color);
        self
    }

    /// The status bar's icons: `Brightness::Dark` for dark icons.
    pub fn status_bar_icons(mut self, icons: Brightness) -> Self {
        self.status_bar_icons = Some(icons);
        self
    }

    /// The navigation bar's background, with the same rule for its icons.
    pub fn navigation_bar_color(mut self, color: Color) -> Self {
        self.navigation_bar_color = Some(color);
        self
    }

    /// The navigation bar's icons: `Brightness::Dark` for dark icons.
    pub fn navigation_bar_icons(mut self, icons: Brightness) -> Self {
        self.navigation_bar_icons = Some(icons);
        self
    }

    /// This, with whatever it does not state filled in from `under`.
    pub fn over(self, under: Self) -> Self {
        Self {
            status_bar_color: self.status_bar_color.or(under.status_bar_color),
            status_bar_icons: self.status_bar_icons.or(under.status_bar_icons),
            navigation_bar_color: self.navigation_bar_color.or(under.navigation_bar_color),
            navigation_bar_icons: self.navigation_bar_icons.or(under.navigation_bar_icons),
        }
    }

    /// Every question answered: what is stated, and the theme for the rest.
    ///
    /// A bar whose colour is stated but not its icons gets icons **read on that colour**,
    /// so a region that only paints a bar black still gets a readable clock.
    pub fn resolve(self, theme: &Theme) -> SystemBars {
        let status_bar_color = self.status_bar_color.unwrap_or(theme.background);
        let navigation_bar_color = self.navigation_bar_color.unwrap_or(theme.background);
        SystemBars {
            status_bar_color,
            status_bar_icons: self
                .status_bar_icons
                .unwrap_or_else(|| icons_on(status_bar_color)),
            navigation_bar_color,
            navigation_bar_icons: self
                .navigation_bar_icons
                .unwrap_or_else(|| icons_on(navigation_bar_color)),
        }
    }
}

/// Both system bars with every question answered — what the shell hands the platform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemBars {
    /// The status bar's background.
    pub status_bar_color: Color,
    /// The status bar's icons: `Brightness::Dark` for dark icons.
    pub status_bar_icons: Brightness,
    /// The navigation bar's background.
    pub navigation_bar_color: Color,
    /// The navigation bar's icons: `Brightness::Dark` for dark icons.
    pub navigation_bar_icons: Brightness,
}

/// Icons that can be read on `color`: dark on a light colour, light on a dark one.
///
/// The threshold is the reference's estimate, a relative luminance of about a third rather
/// than a half, because it weighs the contrast against white and against black the way the
/// accessibility guidelines do: a mid grey reads better under dark icons.
pub(crate) fn icons_on(color: Color) -> Brightness {
    let lin = color.to_linear();
    let luminance = 0.2126 * lin.r + 0.7152 * lin.g + 0.0722 * lin.b;
    if (luminance + 0.05) * (luminance + 0.05) > 0.15 {
        Brightness::Dark
    } else {
        Brightness::Light
    }
}

/// Says what the system bars over this part of the screen should look like — the
/// reference's `AnnotatedRegion<SystemUiOverlayStyle>`.
///
/// ```
/// use frus_core::Color;
/// use frus_widgets::{AnnotatedRegion, Container, SystemUiOverlayStyle};
///
/// // A dark photograph under the status bar wants light icons, whatever the theme is.
/// let photo = Container::<()>::new().color(Color::rgb(0.08, 0.08, 0.1));
/// let _header = AnnotatedRegion::new(
///     SystemUiOverlayStyle::for_background(Color::rgb(0.08, 0.08, 0.1)),
///     photo,
/// );
/// ```
///
/// It takes effect where its box lies **against a bar**: a region covering the first row of
/// the content under the status bar answers for the status bar, and one covering the last
/// row above the navigation bar for the navigation bar. Nested regions answer field by
/// field, the inner one first.
pub struct AnnotatedRegion<Msg> {
    style: SystemUiOverlayStyle,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> AnnotatedRegion<Msg> {
    /// Asks for `style` over `child`.
    pub fn new(style: SystemUiOverlayStyle, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            style,
            children: vec![Box::new(child)],
        }
    }
}

impl<Msg: Clone> Widget<Msg> for AnnotatedRegion<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn system_ui_style(&self) -> Option<SystemUiOverlayStyle> {
        Some(self.style)
    }

    fn debug_name(&self) -> &'static str {
        "AnnotatedRegion"
    }
}

/// The style the regions under two points ask for, the region drawn last first and field
/// by field: `status_bar` answers for the status bar, `navigation_bar` for the other.
pub(crate) fn style_under(
    regions: &[(Rect, SystemUiOverlayStyle)],
    status_bar: Point,
    navigation_bar: Point,
) -> SystemUiOverlayStyle {
    let under = |point: Point| {
        regions
            .iter()
            .rev()
            .filter(|(rect, _)| rect.contains(point))
            .fold(SystemUiOverlayStyle::NONE, |above, (_, style)| {
                above.over(*style)
            })
    };
    let top = under(status_bar);
    let bottom = under(navigation_bar);
    SystemUiOverlayStyle {
        status_bar_color: top.status_bar_color,
        status_bar_icons: top.status_bar_icons,
        navigation_bar_color: bottom.navigation_bar_color,
        navigation_bar_icons: bottom.navigation_bar_icons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::build_ui;
    use crate::{Container, Flex, Keyed, Runtime};
    use frus_core::Size;

    const SCREEN: Size = Size::new(200.0, 400.0);

    /// The first row of the screen, and the last.
    fn top() -> Point {
        Point::new(100.0, 0.5)
    }
    fn bottom() -> Point {
        Point::new(100.0, 399.5)
    }

    /// A dark photograph.
    fn photo() -> Color {
        Color::rgb(0.05, 0.05, 0.08)
    }

    /// Both the screen's full width: a column here places its children at the start rather
    /// than stretching them, and a region with no width lies under nothing.
    fn header() -> Container<()> {
        Container::new().width(200.0).height(100.0).color(photo())
    }

    fn body() -> Container<()> {
        Container::new().width(200.0).height(300.0)
    }

    fn style_of(root: &dyn Widget<()>) -> SystemUiOverlayStyle {
        let ui = build_ui(root, SCREEN, &Runtime::default(), &Theme::light());
        ui.system_ui_style(top(), bottom())
    }

    /// **The region under the status bar answers for it, and for nothing else.** A dark
    /// header at the top of a light screen asks for light icons over it; the navigation bar,
    /// over the body, is left to the theme.
    #[test]
    fn a_region_under_the_status_bar_answers_for_it_and_no_further() {
        let screen = Flex::<()>::column()
            .child(AnnotatedRegion::new(
                SystemUiOverlayStyle::for_background(photo()),
                header(),
            ))
            .child(body());
        let style = style_of(&screen);
        assert_eq!(style.status_bar_color, Some(photo()));
        assert_eq!(style.status_bar_icons, Some(Brightness::Light));
        assert_eq!(style.navigation_bar_color, None, "the body said nothing");
        assert_eq!(style.navigation_bar_icons, None);
    }

    /// **What no region states comes from the theme**: its background, and icons that can
    /// be read on it — dark on the light theme, light on the dark one. This is the whole of
    /// #46 for an application that never names a region.
    #[test]
    fn the_theme_answers_what_no_region_states() {
        let light = SystemUiOverlayStyle::NONE.resolve(&Theme::light());
        assert_eq!(light.status_bar_color, Theme::light().background);
        assert_eq!(
            light.status_bar_icons,
            Brightness::Dark,
            "dark icons on a light bar"
        );
        assert_eq!(light.navigation_bar_icons, Brightness::Dark);

        let dark = SystemUiOverlayStyle::NONE.resolve(&Theme::dark());
        assert_eq!(dark.status_bar_color, Theme::dark().background);
        assert_eq!(dark.status_bar_icons, Brightness::Light);
        assert_eq!(dark.navigation_bar_icons, Brightness::Light);
    }

    /// **A colour stated alone gets icons read on that colour**, not the theme's: the theme
    /// answers for its own background, and a region that painted the bar black under a light
    /// theme would otherwise get dark icons on black.
    #[test]
    fn a_colour_stated_alone_gets_icons_read_on_it() {
        let bars = SystemUiOverlayStyle::NONE
            .status_bar_color(photo())
            .resolve(&Theme::light());
        assert_eq!(bars.status_bar_icons, Brightness::Light);
        assert_eq!(
            bars.navigation_bar_icons,
            Brightness::Dark,
            "the other bar is still the theme's"
        );
    }

    /// **Nested regions answer field by field, the inner one first.** An outer region
    /// paints both bars white; an inner one over the header only asks for light icons. The
    /// status bar gets the inner icons on the outer colour; the navigation bar, outside the
    /// inner region, gets the outer answer whole.
    #[test]
    fn nested_regions_answer_field_by_field_the_inner_first() {
        let screen = AnnotatedRegion::new(
            SystemUiOverlayStyle::for_background(Color::WHITE),
            Flex::<()>::column()
                .child(AnnotatedRegion::new(
                    SystemUiOverlayStyle::NONE.status_bar_icons(Brightness::Light),
                    header(),
                ))
                .child(body()),
        );
        let style = style_of(&screen);
        assert_eq!(
            style.status_bar_icons,
            Some(Brightness::Light),
            "the inner one"
        );
        assert_eq!(style.status_bar_color, Some(Color::WHITE), "the outer one");
        assert_eq!(style.navigation_bar_color, Some(Color::WHITE));
        assert_eq!(style.navigation_bar_icons, Some(Brightness::Dark));
    }

    /// **Behind a key, a region still answers.** A key is a transparent wrapper: the walk
    /// asks it, and it has to pass the question on.
    #[test]
    fn a_keyed_region_still_answers() {
        let screen = Flex::<()>::column()
            .child(Keyed::new(
                1u8,
                AnnotatedRegion::new(SystemUiOverlayStyle::for_background(photo()), header()),
            ))
            .child(body());
        assert_eq!(style_of(&screen).status_bar_color, Some(photo()));
    }

    /// **A region replayed from the paint cache still answers.** A frame that reuses a
    /// repaint boundary's recording rebuilds the registries from what was recorded; a
    /// registry left out of the recording is a region that answers on the first frame and
    /// then falls silent — with the bars switching back to the theme's a frame later.
    #[test]
    fn a_region_replayed_from_the_paint_cache_still_answers() {
        let screen = || {
            Flex::<()>::column()
                .child(
                    Container::new()
                        .repaint_boundary()
                        .child(AnnotatedRegion::new(
                            SystemUiOverlayStyle::for_background(photo()),
                            header(),
                        )),
                )
                .child(body())
        };
        let runtime = Runtime::default();
        let theme = Theme::light();
        let first = build_ui(&screen(), SCREEN, &runtime, &theme);
        assert_eq!(
            first.system_ui_style(top(), bottom()).status_bar_color,
            Some(photo())
        );
        let second = build_ui(&screen(), SCREEN, &runtime, &theme);
        let (hits, _) = runtime.paint_cache.borrow().last_frame_stats();
        assert!(
            hits > 0,
            "the fixture: the second frame came from the cache"
        );
        assert_eq!(
            second.system_ui_style(top(), bottom()).status_bar_color,
            Some(photo()),
            "and the region came back with it"
        );
    }

    /// The contrast rule at its ends.
    #[test]
    fn icons_are_read_on_their_bar() {
        assert_eq!(icons_on(Color::WHITE), Brightness::Dark);
        assert_eq!(icons_on(Color::BLACK), Brightness::Light);
    }
}
