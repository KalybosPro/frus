//! [`Card`]: a themed surface — background, radius, shadow or outline — with one child.

use frus_core::{BorderRadius, Color, Insets, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The room a card leaves around itself, in logical pixels, when the caller has not
/// said otherwise — the reference's figure. Cards are usually stacked, and two of them
/// flush against each other read as one.
pub const CARD_MARGIN: f32 = 4.0;
/// The default depth of an [`CardVariant::Elevated`] card. Elevation is a **height**,
/// not a blur radius: [`Card::elevation`] says how far off the surface the card sits,
/// and the shadow is derived from it.
pub const CARD_ELEVATION: f32 = 1.0;
/// The room a card leaves **inside** itself by default. An addition of this framework:
/// the reference's card has no padding and leaves it to the content.
pub const CARD_PADDING: f32 = 16.0;

/// Which of the three cards this is.
///
/// The reference has three, and they are not interchangeable: each says something
/// different about how far the card should stand off what is behind it. This used to be
/// one widget drawing a shadow **and** an outline, which is none of the three.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardVariant {
    /// Lifted off the surface by a shadow, no outline. The default.
    #[default]
    Elevated,
    /// A flatter tonal surface: no shadow, no outline. For a card inside something
    /// already elevated, where a second shadow only adds noise.
    Filled,
    /// An outline and no shadow — the quietest of the three, and the one that survives
    /// on a background a shadow would be invisible against.
    Outlined,
}

/// A card: a surface container, in the theme's colours.
///
/// ```ignore
/// Card::new().child(content)                          // elevated, the default
/// Card::new().outlined().child(content)               // an outline, no shadow
/// Card::new().elevation(6.0).radius(20.0)             // further off the surface
/// Card::new().color(theme.scheme.primary_container)   // its own colour
/// Card::new().margin(0.0)                             // flush, for a caller that
///                                                     // spaces its own children
/// ```
///
/// Everything here is a **default**, not a rule: the colour, the rounding, the depth,
/// the margin and the padding are all the caller's to set.
pub struct Card<Msg> {
    /// `None` = the theme's, then [`CardVariant::Elevated`].
    variant: Option<CardVariant>,
    /// `None` = the theme's, then 16.
    padding: Option<f32>,
    /// `None` = [`CARD_MARGIN`].
    margin: Option<f32>,
    /// `None` = [`CARD_ELEVATION`] for an elevated card, and no shadow for the others.
    elevation: Option<f32>,
    /// `None` = the theme's surface for this variant.
    color: Option<Color>,
    /// `None` = the theme's radius.
    radius: Option<BorderRadius>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Card<Msg> {
    /// Creates an elevated card, with a default padding of 16.
    pub fn new() -> Self {
        Self {
            variant: None,
            padding: None,
            margin: None,
            elevation: None,
            color: None,
            radius: None,
            children: Vec::new(),
        }
    }

    /// Chooses the variant.
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// A flat tonal card: no shadow, no outline.
    pub fn filled(self) -> Self {
        self.variant(CardVariant::Filled)
    }

    /// An outlined card: a hairline, no shadow.
    pub fn outlined(self) -> Self {
        self.variant(CardVariant::Outlined)
    }

    /// Uniform padding **inside** the card. An addition of this framework — the
    /// reference's card has none and leaves it to the content — kept because a card
    /// whose text touches its own edge is the more common mistake.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Uniform room **outside** the card. Defaults to [`CARD_MARGIN`]; `0.0` for a
    /// caller that spaces its children itself.
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = Some(margin);
        self
    }

    /// How far off the surface the card sits. `0.0` removes the shadow; the default is
    /// [`CARD_ELEVATION`] for an elevated card and nothing for the other two.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Overrides the background colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Overrides the corner radii (uniform via `f32`, per corner via [`BorderRadius`]).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// Sets the card's content.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// Which of the three this is: what was asked for, then the theme's, then elevated.
    fn kind(&self, theme: Option<&Theme>) -> CardVariant {
        self.variant
            .or_else(|| theme.and_then(|t| t.widgets.card.variant))
            .unwrap_or_default()
    }

    /// The depth actually used: what was asked for, then the theme's, then the
    /// variant's own.
    fn depth(&self, theme: &Theme) -> f32 {
        self.elevation
            .or(theme.widgets.card.elevation)
            .unwrap_or(match self.kind(Some(theme)) {
                CardVariant::Elevated => CARD_ELEVATION,
                CardVariant::Filled | CardVariant::Outlined => 0.0,
            })
    }

    /// The background actually used.
    fn background(&self, theme: &Theme) -> Color {
        self.color
            .or(theme.widgets.card.color)
            .unwrap_or(match self.kind(Some(theme)) {
                // The two ends of the ladder the reference puts them on
                // (`card.dart:313` and `:348`): a card off the page takes the least
                // emphasis a container has, a filled one the most.
                CardVariant::Elevated => theme.scheme.surface_container_low,
                CardVariant::Filled => theme.scheme.surface_container_highest,
                CardVariant::Outlined => theme.scheme.surface,
            })
    }
}

impl<Msg> Default for Card<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Card<Msg> {
    /// The two spacings, resolved together: caller, then theme, then ours.
    fn spacing(&self, theme: Option<&Theme>) -> (f32, f32) {
        let padding = self
            .padding
            .or_else(|| theme.and_then(|t| t.widgets.card.padding))
            .unwrap_or(CARD_PADDING);
        let margin = self
            .margin
            .or_else(|| theme.and_then(|t| t.widgets.card.margin))
            .unwrap_or(CARD_MARGIN);
        (padding, margin)
    }
}

impl<Msg> Widget<Msg> for Card<Msg> {
    fn style(&self) -> Style {
        let (padding, margin) = self.spacing(None);
        Style {
            padding: Insets::uniform(padding),
            margin: Insets::uniform(margin),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let (padding, margin) = self.spacing(Some(theme));
        Style {
            padding: Insets::uniform(padding),
            margin: Insets::uniform(margin),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let radius = self
            .radius
            .or(theme.widgets.card.radius)
            .unwrap_or_else(|| theme.radius.into());
        let depth = self.depth(theme);

        // The shadow, when the card is off the surface at all. The blur grows with the
        // depth and the drop is half of it: a card 1 px up casts a tight shadow under
        // its own edge, one 6 px up casts a wide soft one below it.
        if depth > 0.0 {
            let blur = depth * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                radius.inflate(blur),
                blur,
            );
        }

        // The outline belongs to exactly one of the three. A shadow *and* a hairline is
        // the mash-up this widget used to be.
        let border = if self.kind(Some(theme)) == CardVariant::Outlined {
            (1.0, theme.scheme.outline_variant.fade(o))
        } else {
            (0.0, Color::TRANSPARENT)
        };
        scene.draw_rect(
            bounds,
            self.background(theme).fade(o),
            radius,
            border.0,
            border.1,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    fn primitives(card: &Card<Msg>) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            card,
            Rect::new(0.0, 0.0, 200.0, 100.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    fn shadows(card: &Card<Msg>) -> usize {
        primitives(card)
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { blur, .. } if *blur > 0.0))
            .count()
    }

    fn border_width(card: &Card<Msg>) -> f32 {
        primitives(card)
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    blur, border_width, ..
                } if *blur == 0.0 => Some(*border_width),
                _ => None,
            })
            .next()
            .expect("the card paints its own surface")
    }

    #[test]
    fn the_three_cards_are_three_different_things() {
        // This used to be one card drawing a shadow *and* an outline, which is neither
        // of the reference's three. Each now says one thing.
        let elevated = Card::<Msg>::new();
        assert_eq!(shadows(&elevated), 1, "elevated: a shadow");
        assert_eq!(border_width(&elevated), 0.0, "elevated: and no outline");

        let filled = Card::<Msg>::new().filled();
        assert_eq!(shadows(&filled), 0, "filled: neither");
        assert_eq!(border_width(&filled), 0.0);

        let outlined = Card::<Msg>::new().outlined();
        assert_eq!(shadows(&outlined), 0, "outlined: no shadow");
        assert!(border_width(&outlined) > 0.0, "outlined: an outline");
    }

    #[test]
    fn the_three_cards_sit_on_three_different_tones() {
        let theme = Theme::default();
        let fill = |card: &Card<Msg>| {
            primitives(card).iter().find_map(|p| match p {
                Primitive::Rect { blur, color, .. } if *blur == 0.0 => Some(*color),
                _ => None,
            })
        };
        assert_eq!(
            fill(&Card::new()),
            Some(theme.scheme.surface_container_low),
            "elevated"
        );
        assert_eq!(
            fill(&Card::new().filled()),
            Some(theme.scheme.surface_container_highest),
            "filled"
        );
        assert_eq!(fill(&Card::new().outlined()), Some(theme.scheme.surface));
    }

    #[test]
    fn elevation_is_a_height_and_zero_removes_the_shadow() {
        assert_eq!(shadows(&Card::<Msg>::new().elevation(0.0)), 0);
        assert_eq!(shadows(&Card::<Msg>::new().filled().elevation(3.0)), 1);
        // Higher card, wider and lower shadow.
        let extent = |depth: f32| {
            primitives(&Card::<Msg>::new().elevation(depth))
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { blur, rect, .. } if *blur > 0.0 => Some((*blur, rect.y)),
                    _ => None,
                })
                .expect("a shadow")
        };
        let (low_blur, low_y) = extent(1.0);
        let (high_blur, high_y) = extent(6.0);
        assert!(high_blur > low_blur, "a taller card blurs wider");
        // The rectangle handed to `shadow` is the card's box **grown by the blur** on
        // every side, so its own `y` runs the wrong way as the blur grows. The drop is
        // what is left once that growth is taken back off.
        assert!(
            high_y + high_blur > low_y + low_blur,
            "and drops further: {} against {}",
            high_y + high_blur,
            low_y + low_blur
        );
    }

    #[test]
    fn a_card_leaves_room_around_itself_unless_told_not_to() {
        // The reference's card carries its own margin; two flush against each other
        // read as one surface.
        assert_eq!(
            Widget::<Msg>::style(&Card::new()).margin,
            Insets::uniform(CARD_MARGIN)
        );
        assert_eq!(
            Widget::<Msg>::style(&Card::<Msg>::new().margin(0.0)).margin,
            Insets::uniform(0.0)
        );
    }

    #[test]
    fn the_colour_and_the_rounding_are_the_callers() {
        let mine = Color::rgb8(10, 200, 90);
        let card = Card::<Msg>::new().color(mine).radius(20.0);
        let found = primitives(&card).iter().find_map(|p| match p {
            Primitive::Rect {
                blur,
                color,
                radius,
                ..
            } if *blur == 0.0 => Some((*color, *radius)),
            _ => None,
        });
        assert_eq!(found, Some((mine, BorderRadius::uniform(20.0))));
    }
}
