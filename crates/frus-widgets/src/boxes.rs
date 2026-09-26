//! The **single-purpose boxes**: [`Padding`], [`ColoredBox`], [`DecoratedBox`] and [`ClipRect`].
//!
//! A [`Container`] does all four and more, and is the right widget when a box needs several of
//! them at once. These say one thing each, so that a tree reads as what it does: a padding is a
//! padding, not a container that happens to have one setting.

// `BorderRadius`, `Curve` and `Size` are named by `forward_to_container!`'s expansion.
use frus_core::{
    BorderRadius, BoxDecoration, ClipShape, Color, Curve, Insets, InsetsGeometry, Rect, Scene, Size,
};
use frus_layout::Style;

use crate::container::Container;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// **Insets its child** by `padding` on each side: the box is the child's size plus the
/// padding, and the child is handed what is left of the room once the padding is taken out.
///
/// The padding is physical ([`Insets`](frus_core::Insets), whose left is the left in any
/// script), directional ([`InsetsDirectional`](frus_core::InsetsDirectional), whose `start` is
/// the left in a left-to-right script and the right in a right-to-left one), or one number for
/// all four sides.
///
/// ```
/// use frus_core::{Insets, InsetsDirectional};
/// use frus_widgets::{Padding, Text};
///
/// let _all: Padding<()> = Padding::new(16.0, Text::new("Sixteen all round"));
/// let _sides: Padding<()> = Padding::new(Insets::symmetric(24.0, 8.0), Text::new("Wide"));
/// let _lead: Padding<()> = Padding::new(InsetsDirectional::only_start(72.0), Text::new("Indented"));
/// ```
pub struct Padding<Msg = crate::callback::Callback> {
    padding: InsetsGeometry,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Padding<Msg> {
    /// Insets `child` by `padding`.
    pub fn new(padding: impl Into<InsetsGeometry>, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            padding: padding.into(),
            children: vec![Box::new(child)],
        }
    }
}

impl<Msg> Padding<Msg> {
    /// The insets **in layout space**. The frame is laid out left to right and mirrored as a
    /// whole in a right-to-left script, so a directional inset is resolved as if left to right
    /// (the mirror puts its start on the right), and a physical one is swapped beforehand so
    /// that the mirror puts it back on the side it names.
    fn laid_out(&self, direction: frus_core::TextDirection) -> Insets {
        match self.padding {
            InsetsGeometry::Directional(insets) => insets.resolve(frus_core::TextDirection::Ltr),
            InsetsGeometry::Physical(insets) if direction.is_rtl() => Insets {
                left: insets.right,
                right: insets.left,
                ..insets
            },
            InsetsGeometry::Physical(insets) => insets,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Padding<Msg> {
    fn style(&self) -> Style {
        Style {
            padding: self.laid_out(frus_core::TextDirection::Ltr),
            ..Style::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            padding: self.laid_out(theme.direction),
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure layout widget: it draws nothing of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "Padding"
    }
}

/// **Paints its box in one colour** behind its child — a [`Container`] with a colour and
/// nothing else, for the tree that only wants a fill.
///
/// ```
/// use frus_core::Color;
/// use frus_widgets::{ColoredBox, Text};
///
/// let _band: ColoredBox<()> = ColoredBox::new(Color::rgb(0.95, 0.95, 0.9), Text::new("Note"));
/// ```
pub struct ColoredBox<Msg = crate::callback::Callback> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> ColoredBox<Msg> {
    /// Fills the box behind `child` with `color`.
    pub fn new(color: Color, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Container::new().color(color).child(child),
        }
    }
}

crate::animated::forward_to_container!(ColoredBox);

/// **Paints a decoration** — a fill or a gradient, a border, corners, a shadow — behind its
/// child, or [in front of it](Self::foreground).
///
/// ```
/// use frus_core::{Border, BorderRadius, BoxDecoration, Color};
/// use frus_widgets::{DecoratedBox, Text};
///
/// let frame = BoxDecoration::default()
///     .border(Border::new(1.0, Color::rgb(0.8, 0.8, 0.8)))
///     .radius(BorderRadius::uniform(8.0));
/// let _framed: DecoratedBox<()> = DecoratedBox::new(frame, Text::new("Framed"));
/// ```
pub struct DecoratedBox<Msg = crate::callback::Callback> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> DecoratedBox<Msg> {
    /// Paints `decoration` behind `child`.
    pub fn new(decoration: BoxDecoration, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Container::new().decoration(decoration).child(child),
        }
    }

    /// Paints `decoration` **in front of** `child` instead — a border or a tint over a picture,
    /// which a decoration behind the picture would be hidden by.
    pub fn foreground(decoration: BoxDecoration, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Container::new().foreground(decoration).child(child),
        }
    }
}

crate::animated::forward_to_container!(DecoratedBox);

/// **Clips its child to its box**: whatever the child paints past the box's edges is cut off —
/// a picture larger than its frame, content sliding in from outside, a transform that moves
/// something past the edge. Same layout rules as [`ClipRRect`](crate::ClipRRect), with square
/// corners.
///
/// ```
/// use frus_widgets::{ClipRect, Text};
///
/// let _cropped: ClipRect<()> = ClipRect::new().child(Text::new("Cut at the edge"));
/// ```
pub struct ClipRect<Msg = crate::callback::Callback> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ClipRect<Msg> {
    /// A clip with no child yet.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Sets the clipped child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for ClipRect<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for ClipRect<Msg> {
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

    fn clip_shape(&self) -> Option<ClipShape> {
        Some(ClipShape::Rect)
    }

    fn debug_name(&self) -> &'static str {
        "ClipRect"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime};
    use frus_core::{Border, InsetsDirectional, Primitive};

    const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

    /// A 20×20 red square.
    fn square() -> Container<()> {
        Container::new().width(20.0).height(20.0).color(RED)
    }

    fn frame(root: &dyn Widget<()>, theme: &Theme) -> Vec<Primitive> {
        build_ui(root, Size::new(100.0, 100.0), &Runtime::default(), theme)
            .scene()
            .primitives()
            .to_vec()
    }

    /// Every filled rectangle of `color`, in painted order, looking inside layers.
    fn rects_of(primitives: &[Primitive], color: Color) -> Vec<Rect> {
        let mut found = Vec::new();
        for p in primitives {
            match p {
                Primitive::Rect { rect, color: c, .. } if *c == color => found.push(*rect),
                Primitive::Layer { primitives, .. } => found.extend(rects_of(primitives, color)),
                _ => {}
            }
        }
        found
    }

    /// A 100×100 box holding `child`.
    fn boxed(child: impl Widget<()> + 'static) -> Container<()> {
        Container::new().width(100.0).height(100.0).child(child)
    }

    /// **The padding moves the child in**: ten all round puts the square at (10, 10), and a
    /// symmetric one puts it at its horizontal and vertical insets.
    #[test]
    fn padding_insets_the_child() {
        let theme = Theme::default();
        let at = rects_of(&frame(&boxed(Padding::new(10.0, square())), &theme), RED);
        assert_eq!(at.first().map(|r| (r.x, r.y)), Some((10.0, 10.0)), "{at:?}");
        let symmetric = Padding::new(Insets::symmetric(24.0, 8.0), square());
        let at = rects_of(&frame(&boxed(symmetric), &theme), RED);
        assert_eq!(at.first().map(|r| (r.x, r.y)), Some((24.0, 8.0)), "{at:?}");
    }

    /// **The style says the padding** when asked without a theme too — left to right, as a
    /// frame with no theme is laid out.
    #[test]
    fn the_style_without_a_theme_carries_the_padding() {
        let padded = Padding::<()>::new(InsetsDirectional::only_start(12.0), square());
        assert_eq!(
            Widget::<()>::style(&padded).padding,
            Insets::new(0.0, 0.0, 0.0, 12.0)
        );
    }

    /// **A directional padding follows the reading direction**: a start inset is on the left in
    /// a left-to-right script and on the right in a right-to-left one. (The centred square shows
    /// where the room left over is: 70 px wide, starting at 30 or at 0.)
    #[test]
    fn a_directional_padding_mirrors_in_rtl() {
        let lead = || {
            boxed(Padding::new(
                InsetsDirectional::only_start(30.0),
                crate::Center::new(square()),
            ))
        };
        let ltr = rects_of(&frame(&lead(), &Theme::default()), RED);
        assert_eq!(ltr.first().map(|r| r.x), Some(55.0), "{ltr:?}");
        let rtl = rects_of(&frame(&lead(), &Theme::default().rtl()), RED);
        assert_eq!(rtl.first().map(|r| r.x), Some(25.0), "{rtl:?}");
    }

    /// **A physical padding stays on the side it names** in either direction: a left inset is
    /// on the left in a right-to-left script too.
    #[test]
    fn a_physical_padding_stays_put_in_rtl() {
        let left = || {
            boxed(Padding::new(
                Insets::new(0.0, 0.0, 0.0, 30.0),
                crate::Center::new(square()),
            ))
        };
        for theme in [Theme::default(), Theme::default().rtl()] {
            let at = rects_of(&frame(&left(), &theme), RED);
            assert_eq!(at.first().map(|r| r.x), Some(55.0), "{at:?}");
        }
    }

    /// **A coloured box paints its whole box, behind the child** — as big as its child, like
    /// any box, and the whole room when the child asks for it, as a centred child does.
    #[test]
    fn a_colored_box_fills_behind_its_child() {
        let hugging = frame(&boxed(ColoredBox::new(BLUE, square())), &Theme::default());
        let fill = rects_of(&hugging, BLUE);
        assert_eq!(
            fill.first().map(|r| r.width),
            Some(20.0),
            "as wide as the square: {fill:?}"
        );
        let centred = boxed(ColoredBox::new(BLUE, crate::Center::new(square())));
        let primitives = frame(&centred, &Theme::default());
        let fill = rects_of(&primitives, BLUE);
        assert_eq!(fill, vec![Rect::new(0.0, 0.0, 100.0, 100.0)]);
        let fill_at = primitives
            .iter()
            .position(|p| matches!(p, Primitive::Rect { color, .. } if *color == BLUE));
        let child_at = primitives
            .iter()
            .position(|p| matches!(p, Primitive::Rect { color, .. } if *color == RED));
        assert!(fill_at < child_at, "the fill is behind the child");
    }

    /// **A decoration is painted behind the child, or in front of it** when it is a
    /// foreground.
    #[test]
    fn a_decoration_goes_behind_or_in_front() {
        let frame_of = |decorated: DecoratedBox<()>| frame(&boxed(decorated), &Theme::default());
        let order = |primitives: &[Primitive]| {
            let blue = primitives
                .iter()
                .position(|p| matches!(p, Primitive::Rect { color, .. } if *color == BLUE));
            let red = primitives
                .iter()
                .position(|p| matches!(p, Primitive::Rect { color, .. } if *color == RED));
            (blue.expect("the decoration"), red.expect("the child"))
        };
        let (decoration, child) = order(&frame_of(DecoratedBox::new(
            BoxDecoration::filled(BLUE),
            square(),
        )));
        assert!(decoration < child, "behind");
        let tint = BoxDecoration::filled(BLUE);
        let (decoration, child) = order(&frame_of(DecoratedBox::foreground(tint, square())));
        assert!(decoration > child, "in front");
        // A border is part of a decoration too.
        let bordered = BoxDecoration::default().border(Border::new(2.0, BLUE));
        assert!(!frame_of(DecoratedBox::new(bordered, square())).is_empty());
    }

    /// **A clip cuts the child at the box's edges**: a square larger than its box is painted
    /// in a layer clipped to the box.
    #[test]
    fn a_clip_rect_cuts_at_the_box() {
        let big = Container::<()>::new().width(200.0).height(200.0).color(RED);
        let clipped = Container::<()>::new()
            .width(50.0)
            .height(50.0)
            .child(ClipRect::new().child(big));
        let primitives = frame(&clipped, &Theme::default());
        let layer = primitives.iter().find_map(|p| match p {
            Primitive::Layer {
                clip, primitives, ..
            } if !rects_of(primitives, RED).is_empty() => Some(*clip),
            _ => None,
        });
        let clip = layer.expect("the child is painted in a clipped layer");
        assert!(clip.width <= 50.0 && clip.height <= 50.0, "{clip:?}");
    }
}
