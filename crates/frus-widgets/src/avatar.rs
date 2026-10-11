//! [`CircleAvatar`]: a circle that stands for a user — their picture, or in its absence
//! their initials, on a colour (milestone 647, the reference's `CircleAvatar`).

use frus_core::{
    BoxDecoration, BoxFit, BoxShape, Color, DecorationImage, ImageHandle, Rect, Scene, Size,
};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The radius when none is given: the size a list tile's leading slot is made for.
pub const AVATAR_RADIUS: f32 = 20.0;

/// How long a change of colour or size takes to arrive — the reference's
/// `kThemeChangeDuration`.
const CHANGE: f32 = 0.2;

/// A circle that represents a user: a picture cropped to the circle, or, without one, a
/// child — usually their initials — centred on a colour.
///
/// ```ignore
/// CircleAvatar::initials("Ada Lovelace")                       // "AL" on the theme's colour
/// CircleAvatar::new().child(Icon::new(icons::PERSON))          // any widget
/// CircleAvatar::new().background_image(photo).radius(32.0)     // a picture
/// ```
///
/// The defaults are the reference's: a radius of 20, the scheme's `primary_container`
/// behind and `on_primary_container` in front; the child's text in the theme's
/// `title_medium`, its icons in the same colour, and its text not scaled with the reader's
/// font size, so it does not spill out of a circle that did not grow. A change of colour
/// or of radius is animated.
///
/// The **foreground** picture is painted over the child and the **background** one under
/// it. A foreground picture still on its way — or that never arrives — leaves the
/// background showing, and a background one the colour, which is how the reference falls
/// back from one to the next.
pub struct CircleAvatar<Msg = crate::callback::Callback> {
    child: Vec<Box<dyn Widget<Msg>>>,
    background_color: Option<Color>,
    foreground_color: Option<Color>,
    background_image: Option<ImageHandle>,
    foreground_image: Option<ImageHandle>,
    radius: Option<f32>,
    min_radius: Option<f32>,
    max_radius: Option<f32>,
}

impl<Msg> Default for CircleAvatar<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> CircleAvatar<Msg> {
    /// An empty avatar: the colour alone, at the default radius.
    pub fn new() -> Self {
        Self {
            child: Vec::new(),
            background_color: None,
            foreground_color: None,
            background_image: None,
            foreground_image: None,
            radius: None,
            min_radius: None,
            max_radius: None,
        }
    }

    /// An avatar showing the **initials** of `name`: the first letters of its first two
    /// words, upper-cased, or its first two letters when it is one word.
    pub fn initials(name: impl Into<String>) -> Self
    where
        Msg: Clone + 'static,
    {
        Self::new().child(crate::text(initials_of(&name.into())))
    }

    /// What is shown in the circle, centred: usually a text, or an icon.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.child = vec![Box::new(child)];
        self
    }

    /// The colour the circle is filled with; the scheme's `primary_container` otherwise.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// The colour of the child's text and icons; the scheme's `on_primary_container`
    /// otherwise.
    pub fn foreground_color(mut self, color: Color) -> Self {
        self.foreground_color = Some(color);
        self
    }

    /// A picture under the child, covering the circle: the fallback for
    /// [`Self::foreground_image`]. `None` — a picture not yet arrived — shows the colour.
    pub fn background_image(mut self, image: impl Into<Option<ImageHandle>>) -> Self {
        self.background_image = image.into();
        self
    }

    /// A picture over the child, covering the circle: usually the user's photo.
    pub fn foreground_image(mut self, image: impl Into<Option<ImageHandle>>) -> Self {
        self.foreground_image = image.into();
        self
    }

    /// The radius, in logical pixels: the avatar is this big, whatever it holds.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    /// The smallest radius it may have; nought by default once a bound is set.
    pub fn min_radius(mut self, radius: f32) -> Self {
        self.min_radius = Some(radius);
        self
    }

    /// The largest radius it may have; unbounded by default once a bound is set.
    pub fn max_radius(mut self, radius: f32) -> Self {
        self.max_radius = Some(radius);
        self
    }

    /// The size as a **diameter**: [`Self::radius`] of half of it.
    pub fn size(self, diameter: f32) -> Self {
        self.radius(diameter * 0.5)
    }

    /// No bound at all: the reference's default radius on both sides.
    fn unbounded(&self) -> bool {
        self.radius.is_none() && self.min_radius.is_none() && self.max_radius.is_none()
    }

    /// The smallest diameter, the reference's `_minDiameter`.
    fn min_diameter(&self) -> f32 {
        if self.unbounded() {
            return AVATAR_RADIUS * 2.0;
        }
        2.0 * self.radius.or(self.min_radius).unwrap_or(0.0)
    }

    /// The largest diameter, the reference's `_maxDiameter`.
    fn max_diameter(&self) -> f32 {
        if self.unbounded() {
            return AVATAR_RADIUS * 2.0;
        }
        2.0 * self.radius.or(self.max_radius).unwrap_or(f32::INFINITY)
    }

    /// The size it has whatever it holds, when its two bounds agree: what is animated.
    fn fixed(&self) -> Option<f32> {
        let (lo, hi) = (self.min_diameter(), self.max_diameter());
        (lo == hi).then_some(lo)
    }

    fn colors(&self, theme: &Theme) -> (Color, Color) {
        (
            self.background_color
                .unwrap_or(theme.scheme.primary_container),
            self.foreground_color
                .unwrap_or(theme.scheme.on_primary_container),
        )
    }
}

/// The initials of `name`: the first letters of its first two words, upper-cased, or its
/// first two letters when it has no spaces to split at.
fn initials_of(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if initials.is_empty() {
        name.chars().take(2).collect::<String>().to_uppercase()
    } else {
        initials
    }
}

/// A picture covering the circle.
fn covering(image: &ImageHandle) -> DecorationImage {
    DecorationImage::new(image.clone()).fit(BoxFit::Cover)
}

impl<Msg> Widget<Msg> for CircleAvatar<Msg> {
    fn style(&self) -> Style {
        let fixed = self.fixed();
        let (lo, hi) = (self.min_diameter(), self.max_diameter());
        let bound = |d: f32| {
            if d.is_finite() {
                Dimension::Length(d)
            } else {
                Dimension::Auto
            }
        };
        Style {
            width: fixed.map_or(Dimension::Auto, Dimension::Length),
            height: fixed.map_or(Dimension::Auto, Dimension::Length),
            min_width: Dimension::Length(lo),
            min_height: Dimension::Length(lo),
            max_width: bound(hi),
            max_height: bound(hi),
            // The child in the middle, as the reference's `Center`.
            justify: Justify::Center,
            align: Align::Center,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }

    fn debug_name(&self) -> &'static str {
        "CircleAvatar"
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let (background, _) = self.colors(theme);
        let background = status.anim_color.unwrap_or(background);
        let mut decoration = BoxDecoration::filled(background).shape(BoxShape::Circle);
        if let Some(image) = &self.background_image {
            decoration = decoration.image(covering(image));
        }
        decoration.paint_into_in(scene, bounds, status.opacity, theme.direction);
    }

    fn foreground(&self, _theme: &Theme) -> Option<BoxDecoration> {
        self.foreground_image.as_ref().map(|image| {
            BoxDecoration::default()
                .shape(BoxShape::Circle)
                .image(covering(image))
        })
    }

    /// The child's type and icons: the theme's `title_medium`, in the foreground colour.
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        if self.child.is_empty() {
            return None;
        }
        let (_, foreground) = self.colors(inherited);
        let mut theme = Box::new(inherited.clone());
        theme.widgets.text = crate::DefaultTextStyle {
            selectable: inherited.widgets.text.selectable,
            ..crate::DefaultTextStyle::from_text_style(
                inherited.text.title_medium.color(foreground),
            )
        };
        theme.widgets.icon.color = Some(foreground);
        Some(theme)
    }

    /// Its text is not scaled with the reader's font size: the circle does not grow.
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        (!self.child.is_empty()).then(|| inherited.with_text_scaler(1.0))
    }

    /// A colour given changes over a moment, as the reference's does.
    fn anim_color(&self) -> Option<Color> {
        self.background_color
    }

    /// A radius given changes over a moment too.
    fn anim_size(&self) -> Option<Size> {
        self.fixed()
            .filter(|_| !self.unbounded())
            .map(|d| Size::new(d, d))
    }

    fn anim_duration(&self) -> f32 {
        CHANGE
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{ImageData, Primitive};

    fn pixels() -> ImageHandle {
        ImageData::from_rgba(2, 2, vec![255; 16]).into_handle()
    }

    fn painted(avatar: &CircleAvatar<()>, theme: &Theme) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            avatar,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Status::default(),
            theme,
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn takes_two_uppercased_initials() {
        assert_eq!(initials_of("ada lovelace"), "AL");
        assert_eq!(initials_of("bob"), "B");
        assert_eq!(initials_of("grace brewster hopper"), "GB");
        assert_eq!(initials_of(""), "");
    }

    /// **The reference's sizes**: a radius of 20 when nothing is said; a radius is both
    /// bounds; one bound alone leaves the other at nought or unbounded.
    #[test]
    fn the_sizes_are_the_reference_s() {
        let size = |a: CircleAvatar<()>| {
            let s = Widget::<()>::style(&a);
            (s.width, s.min_width, s.max_width)
        };
        let l = Dimension::Length;
        assert_eq!(size(CircleAvatar::new()), (l(40.0), l(40.0), l(40.0)));
        assert_eq!(
            size(CircleAvatar::new().radius(12.0)),
            (l(24.0), l(24.0), l(24.0))
        );
        assert_eq!(
            size(CircleAvatar::new().size(30.0)),
            (l(30.0), l(30.0), l(30.0))
        );
        assert_eq!(
            size(CircleAvatar::new().min_radius(10.0)),
            (Dimension::Auto, l(20.0), Dimension::Auto)
        );
        assert_eq!(
            size(CircleAvatar::new().max_radius(30.0)),
            (Dimension::Auto, l(0.0), l(60.0))
        );
        assert_eq!(
            size(CircleAvatar::new().min_radius(10.0).max_radius(10.0)),
            (l(20.0), l(20.0), l(20.0))
        );
        let s = Widget::<()>::style(&CircleAvatar::<()>::new());
        assert_eq!((s.justify, s.align), (Justify::Center, Align::Center));
    }

    /// **The scheme's container colours**, unless given.
    #[test]
    fn the_colours_are_the_container_pair() {
        let theme = Theme::default();
        let Primitive::Rect { color, radius, .. } = &painted(&CircleAvatar::new(), &theme)[0]
        else {
            panic!("the circle");
        };
        assert_eq!(*color, theme.scheme.primary_container);
        assert_eq!(radius.top_left, 20.0, "a circle");
        let red = Color::rgb(1.0, 0.0, 0.0);
        let Primitive::Rect { color, .. } =
            &painted(&CircleAvatar::new().background_color(red), &theme)[0]
        else {
            panic!("the circle");
        };
        assert_eq!(*color, red);
    }

    /// **The child's type and icons** come from the avatar: `title_medium` in the
    /// foreground colour, and no text scaling; with no child there is nothing to style.
    #[test]
    fn the_child_takes_the_title_style_in_the_foreground_colour() {
        let theme = Theme::default();
        let avatar = CircleAvatar::<()>::initials("Ada");
        let inner = Widget::<()>::theme_override(&avatar, &theme).expect("a theme for the child");
        let style = inner.widgets.text.style;
        assert_eq!(style.size, theme.text.title_medium.size);
        assert_eq!(style.color, Some(theme.scheme.on_primary_container));
        assert_eq!(
            inner.widgets.icon.color,
            Some(theme.scheme.on_primary_container)
        );
        let gold = Color::rgb(1.0, 0.8, 0.2);
        let inner =
            Widget::<()>::theme_override(&avatar.foreground_color(gold), &theme).expect("a theme");
        assert_eq!(inner.widgets.text.style.color, Some(gold));
        assert!(Widget::<()>::theme_override(&CircleAvatar::<()>::new(), &theme).is_none());

        let media = crate::MediaQuery::new(Size::new(400.0, 800.0)).with_text_scaler(2.0);
        let avatar = CircleAvatar::<()>::initials("Ada");
        let inside = Widget::<()>::media_override(&avatar, media).expect("unscaled");
        assert_eq!(inside.text_scaler, 1.0);
        assert!(Widget::<()>::media_override(&CircleAvatar::<()>::new(), media).is_none());
    }

    /// **A background picture under, a foreground one over**, both covering the circle.
    #[test]
    fn the_pictures_cover_the_circle() {
        let theme = Theme::default();
        let avatar = CircleAvatar::<()>::new().background_image(pixels());
        let primitives = painted(&avatar, &theme);
        assert!(
            primitives
                .iter()
                .any(|p| matches!(p, Primitive::Layer { clip_shape, .. } if *clip_shape == frus_core::ClipShape::Oval)),
            "the picture in the circle"
        );
        assert!(Widget::<()>::foreground(&avatar, &theme).is_none());
        let front = CircleAvatar::<()>::new().foreground_image(pixels());
        let decoration = Widget::<()>::foreground(&front, &theme).expect("over the child");
        assert_eq!(decoration.shape, BoxShape::Circle);
        assert_eq!(decoration.image.and_then(|i| i.fit), Some(BoxFit::Cover));
        // A picture still on its way is the colour alone.
        let waiting = CircleAvatar::<()>::new().foreground_image(None);
        assert!(Widget::<()>::foreground(&waiting, &theme).is_none());
    }

    /// **The colour on its way is the one painted**: the runtime's, while it moves.
    #[test]
    fn the_colour_in_flight_is_painted() {
        let theme = Theme::default();
        let red = Color::rgb(1.0, 0.0, 0.0);
        let half = Color::rgb(0.5, 0.0, 0.5);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &CircleAvatar::<()>::new().background_color(red),
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Status {
                anim_color: Some(half),
                ..Status::default()
            },
            &theme,
            &mut scene,
        );
        let Primitive::Rect { color, .. } = &scene.primitives()[0] else {
            panic!("the circle");
        };
        assert_eq!(*color, half);
    }

    /// **A selection that spans the texts goes on through the avatar**: the type is the
    /// avatar's, the selection is not its to end.
    #[test]
    fn a_selection_area_reaches_the_initials() {
        let mut theme = Theme::default();
        theme.widgets.text.selectable = true;
        let avatar = CircleAvatar::<()>::initials("Ada");
        let inner = Widget::<()>::theme_override(&avatar, &theme).expect("a theme");
        assert!(inner.widgets.text.selectable);
    }

    /// **A colour or a radius given is animated**; the theme's own are not asked for.
    #[test]
    fn a_given_colour_and_radius_are_animated() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let avatar = CircleAvatar::<()>::new().background_color(red).radius(10.0);
        assert_eq!(Widget::<()>::anim_color(&avatar), Some(red));
        assert_eq!(
            Widget::<()>::anim_size(&avatar),
            Some(Size::new(20.0, 20.0))
        );
        assert_eq!(
            Widget::<()>::anim_duration(&avatar),
            0.2,
            "the reference's 200 ms"
        );
        let plain = CircleAvatar::<()>::new();
        assert_eq!(Widget::<()>::anim_color(&plain), None);
        assert_eq!(Widget::<()>::anim_size(&plain), None);
        assert_eq!(Widget::<()>::anim_size(&plain.min_radius(5.0)), None);
    }
}
