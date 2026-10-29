//! [`Badge`]: the small mark that says *there is something here* — a count on an inbox,
//! a dot on a bell.
//!
//! Two shapes, and the reference draws the same distinction. With a label it is a pill
//! carrying a number or a word; without one it is a **dot**, which says the same thing
//! without saying how much. A dot is what most notification marks are, and it is the one
//! this widget could not draw at all before milestone 378.
//!
//! Two ways to place it (milestone 707):
//!
//! - **On its own**, a pill you put where you like — in a row, beside a title.
//! - **On a child**, with [`Badge::on`]: pinned to the child's top-end corner, partly over
//!   it, the way the reference pins it — the count on a mail icon, the dot on a bell. Where on
//!   the child is [`Badge::alignment`] and [`Badge::offset`], each from the theme otherwise,
//!   and mirrored for a right-to-left reading.
//!
//! [`Badge::count`] writes a number, capped as the reference caps it (`999+`), and
//! [`Badge::label_visible`] takes the badge away — not to a dot, away — which is what a count
//! of zero wants.

use frus_core::{Alignment, AlignmentDirectional, AlignmentGeometry, Color, Point, Rect, Scene};
use frus_core::{TextDirection, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The reference's `smallSize`: the diameter of a badge with no label.
const DOT: f32 = 6.0;
/// The reference's `largeSize`: the height of a badge that carries one.
const PILL: f32 = 16.0;
/// The label's room either side, inside the pill.
const PAD_X: f32 = 4.0;
/// The reference's `maxCount`: the largest count written out before it becomes `999+`.
const MAX_COUNT: u32 = 999;
/// The reference's offset for a pinned badge with a label, for a left-to-right reading: a
/// little past the child's end, a little above its top.
const OFFSET: (f32, f32) = (4.0, -4.0);

/// What a count reads as: the number, or `max+` past `max` — the reference's
/// `Badge.count`.
pub(crate) fn count_label(count: u32, max: u32) -> String {
    if count > max {
        format!("{max}+")
    } else {
        count.to_string()
    }
}

/// A small mark: a counted pill, or a dot.
#[derive(Clone, Debug)]
pub struct Badge {
    /// The text on the pill. `None` is a **dot**, which is a shape rather than an empty
    /// label — see [`Badge::dot`].
    label: Option<String>,
    /// A count, written by [`Badge::count`] and capped at `max_count`.
    count: Option<u32>,
    max_count: u32,
    /// Whether the badge is shown at all: the reference's `isLabelVisible`.
    label_visible: bool,
    background_color: Option<Color>,
    text_color: Option<Color>,
    text_style: Option<TextStyle>,
    small_size: Option<f32>,
    large_size: Option<f32>,
    padding: Option<f32>,
    alignment: Option<AlignmentGeometry>,
    offset: Option<(f32, f32)>,
}

impl Badge {
    /// A badge carrying `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            label: Some(text.into()),
            count: None,
            max_count: MAX_COUNT,
            label_visible: true,
            background_color: None,
            text_color: None,
            text_style: None,
            small_size: None,
            large_size: None,
            padding: None,
            alignment: None,
            offset: None,
        }
    }

    /// A badge carrying a **count**: the number, or `999+` past
    /// [`max_count`](Self::max_count). The reference's `Badge.count`.
    pub fn count(count: u32) -> Self {
        let mut badge = Self::new(String::new());
        badge.count = Some(count);
        badge
    }

    /// The largest count written out; past it the badge reads `max+`. 999 by default.
    pub fn max_count(mut self, max: u32) -> Self {
        self.max_count = max.max(1);
        self
    }

    /// A badge with **no** label: a small dot.
    ///
    /// What most notification marks are. *Something happened* is the whole message, and a
    /// count that nobody reads is a number taking up room on a bell.
    pub fn dot() -> Self {
        let mut badge = Self::new(String::new());
        badge.label = None;
        badge
    }

    /// Whether the badge is shown. `false` takes it away — on its own it takes no room and
    /// draws nothing, and on a child only the child is left — so a count of zero needs no
    /// `if` around the widget: `Badge::count(n).label_visible(n > 0)`. The reference's
    /// `isLabelVisible`.
    pub fn label_visible(mut self, visible: bool) -> Self {
        self.label_visible = visible;
        self
    }

    /// The pill's fill; the theme's `error` otherwise, as the reference has it — a badge
    /// is an alert, not an accent.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// The label's colour on that fill; the theme's `on_error` otherwise.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// The label's whole style — size, weight, family. Its colour is
    /// [`text_color`](Badge::text_color)'s.
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// The diameter of a badge with no label. The reference's `smallSize`.
    pub fn small_size(mut self, size: f32) -> Self {
        self.small_size = Some(size);
        self
    }

    /// The height of a badge that carries one. The reference's `largeSize`.
    pub fn large_size(mut self, size: f32) -> Self {
        self.large_size = Some(size);
        self
    }

    /// The label's room either side.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Where on its child a pinned badge goes ([`Badge::on`]): the child's top-end corner
    /// by default, or the theme's. An [`AlignmentDirectional`] follows the reading
    /// direction.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    /// How far a pinned badge with a label is moved from its place, `(across, down)`: a
    /// little past the end and above the top by default — `(4, -4)`, mirrored for a
    /// right-to-left reading — or the theme's. A dot is not moved: it sits in the corner.
    pub fn offset(mut self, across: f32, down: f32) -> Self {
        self.offset = Some((across, down));
        self
    }

    /// Pins this badge to `child`: drawn over it, at its top-end corner unless
    /// [`alignment`](Self::alignment) says otherwise. The child is laid out, and takes
    /// input, exactly as it would alone; the badge takes no room of its own and may reach
    /// past the child's edges.
    pub fn on<Msg>(self, child: impl Widget<Msg> + 'static) -> Badged<Msg> {
        Badged {
            badge: self,
            child: [Box::new(child)],
        }
    }

    fn hidden(&self) -> bool {
        !self.label_visible
    }

    /// The text drawn, or `None` for a dot.
    fn text(&self) -> Option<String> {
        match self.count {
            Some(count) => Some(count_label(count, self.max_count)),
            None => self.label.clone(),
        }
    }

    fn fill(&self, theme: &Theme) -> Color {
        self.background_color
            .or(theme.widgets.badge.background_color)
            .unwrap_or(theme.scheme.error)
    }

    fn ink(&self, theme: &Theme) -> Color {
        self.text_color
            .or(theme.widgets.badge.text_color)
            .unwrap_or(theme.scheme.on_error)
    }

    fn label_style(&self, theme: &Theme) -> TextStyle {
        self.text_style
            .or(theme.widgets.badge.text_style)
            .unwrap_or(TextStyle {
                size: Some(11.0),
                ..theme.text.label_small
            })
    }

    fn dot_size(&self, theme: &Theme) -> f32 {
        self.small_size
            .or(theme.widgets.badge.small_size)
            .unwrap_or(DOT)
    }

    fn pill_height(&self, theme: &Theme) -> f32 {
        self.large_size
            .or(theme.widgets.badge.large_size)
            .unwrap_or(PILL)
    }

    fn pad(&self, theme: &Theme) -> f32 {
        self.padding
            .or(theme.widgets.badge.padding)
            .unwrap_or(PAD_X)
    }

    /// The box the mark itself occupies; nothing when it is hidden.
    fn mark_size(&self, theme: &Theme) -> (f32, f32) {
        if self.hidden() {
            return (0.0, 0.0);
        }
        let Some(text) = self.text() else {
            let d = self.dot_size(theme);
            return (d, d);
        };
        let style = self.label_style(theme);
        let measured = frus_text::measure_style(&text, style);
        let height = self.pill_height(theme);
        // Never narrower than it is tall: a single digit in a wide pill reads as a
        // mistake, and the reference rounds a one-character badge to a circle for the
        // same reason.
        (
            (measured.width + self.pad(theme) * 2.0).max(height).ceil(),
            height,
        )
    }

    /// Where the mark goes when pinned over `child`: the reference's placement.
    fn pinned_rect(&self, child: Rect, theme: &Theme) -> Rect {
        let mark = self.mark_size(theme);
        let labelled = self.text().is_some();
        let direction = theme.direction;
        let alignment = self
            .alignment
            .or(theme.widgets.badge.alignment)
            .unwrap_or(AlignmentGeometry::Directional(
                AlignmentDirectional::TOP_END,
            ))
            .resolve(direction);
        let offset = if labelled {
            self.offset
                .or(theme.widgets.badge.offset)
                .unwrap_or(match direction {
                    TextDirection::Rtl => (-OFFSET.0, OFFSET.1),
                    _ => OFFSET,
                })
        } else {
            (0.0, 0.0)
        };
        // The width a pinned badge is taken to have when it is placed: its height for a
        // pill, its diameter for a dot — the reference's `widthOffset`.
        let width_offset = if labelled {
            self.pill_height(theme)
        } else {
            self.dot_size(theme)
        };
        pinned_at(child, mark, labelled, width_offset, alignment, offset)
    }

    /// Draws the mark in `rect`.
    fn paint_mark(&self, rect: Rect, opacity: f32, theme: &Theme, scene: &mut Scene) {
        scene.draw_rect(
            rect,
            self.fill(theme).fade(opacity),
            rect.height * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        let Some(text) = self.text() else {
            return;
        };
        let style = self.label_style(theme);
        let measured = frus_text::measure_style(&text, style);
        // Centred both ways rather than placed from the corner: the pill is as wide as
        // its label needs *or* as wide as it is tall, whichever is more, so a one-digit
        // badge has room to spare and a corner offset would sit it off to one side.
        scene.text(
            Point::new(
                rect.x + (rect.width - measured.width) / 2.0,
                rect.y + (rect.height - measured.height) / 2.0,
            ),
            text,
            &style.resolved(),
            self.ink(theme).fade(opacity),
        );
    }

    /// Draws the mark pinned over `anchor` — a child's box, or the glyph a navigation
    /// destination hangs its count on.
    pub(crate) fn paint_pinned(
        &self,
        anchor: Rect,
        opacity: f32,
        theme: &Theme,
        scene: &mut Scene,
    ) {
        if self.hidden() {
            return;
        }
        self.paint_mark(self.pinned_rect(anchor, theme), opacity, theme, scene);
    }

    fn describe(&self) -> Option<frus_core::SemanticsProperties> {
        // A dot has nothing to read out. A count does, and a screen reader that never
        // hears it is a reader who never learns there are three unread messages.
        if self.hidden() {
            return None;
        }
        let text = self.text()?;
        Some(frus_core::SemanticsProperties::new(frus_core::Role::Label).label(text))
    }
}

/// Where a badge `mark` big goes over `child`: `alignment` along the child's box less the
/// badge's `width_offset` across, moved by `offset`, and a labelled badge raised by half its
/// height — the reference's `_RenderBadge`, whose offset for a label also carries the
/// `(0, 8)` it adds so as not to move the badges laid out before its placement changed.
pub(crate) fn pinned_at(
    child: Rect,
    mark: (f32, f32),
    labelled: bool,
    width_offset: f32,
    alignment: Alignment,
    offset: (f32, f32),
) -> Rect {
    let (dx, dy) = if labelled {
        (offset.0, offset.1 + 8.0)
    } else {
        offset
    };
    let x = child.x + alignment.fraction_x() * (child.width - width_offset) + dx;
    let mut y = child.y + alignment.fraction_y() * child.height + dy;
    if labelled {
        y -= mark.1 / 2.0;
    }
    Rect::new(x, y, mark.0, mark.1)
}

impl<Msg> Widget<Msg> for Badge {
    fn style(&self) -> Style {
        <Self as Widget<Msg>>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let (width, height) = self.mark_size(theme);
        Style {
            width: Dimension::Length(width),
            height: Dimension::Length(height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        if self.hidden() {
            return;
        }
        self.paint_mark(bounds, status.opacity, theme, scene);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        self.describe()
    }
}

/// A [`Badge`] pinned to a child: see [`Badge::on`].
pub struct Badged<Msg = crate::callback::Callback> {
    badge: Badge,
    child: [Box<dyn Widget<Msg>>; 1],
}

impl<Msg> Widget<Msg> for Badged<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn paints_over(&self) -> bool {
        !self.badge.hidden()
    }

    fn paint_over(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        self.badge
            .paint_pinned(bounds, status.opacity, theme, scene);
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        self.badge.describe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn painted(badge: &Badge, bounds: Rect, theme: &Theme) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(badge, bounds, Status::default(), theme, &mut scene);
        scene.primitives().to_vec()
    }

    fn texts(primitives: &[Primitive]) -> Vec<String> {
        primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn paints_pill_and_text() {
        let primitives = painted(
            &Badge::new("3"),
            Rect::new(0.0, 0.0, 24.0, 18.0),
            &Theme::default(),
        );
        assert!(primitives
            .iter()
            .any(|p| matches!(p, Primitive::Rect { .. })));
        assert!(primitives
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "3")));
    }

    /// A dot is a **shape**, not an empty label: no text primitive at all, and a box as
    /// wide as it is tall.
    #[test]
    fn a_dot_carries_no_text() {
        let theme = Theme::default();
        let dot = Badge::dot();
        let primitives = painted(&dot, Rect::new(0.0, 0.0, 6.0, 6.0), &theme);
        assert!(!primitives
            .iter()
            .any(|p| matches!(p, Primitive::Text { .. })));
        let style = Widget::<()>::style_themed(&dot, &theme);
        assert_eq!(style.width, Dimension::Length(DOT));
        assert_eq!(style.height, Dimension::Length(DOT));
    }

    /// **A hidden badge is gone**, as the reference has it: no room, nothing drawn, nothing
    /// read out — not a dot, which would still say *something is here* when the count is
    /// zero (milestone 707; it used to fall back to the dot).
    #[test]
    fn a_hidden_badge_takes_no_room_and_draws_nothing() {
        let theme = Theme::default();
        for hidden in [
            Badge::new("99").label_visible(false),
            Badge::dot().label_visible(false),
            Badge::count(0).label_visible(false),
        ] {
            let style = Widget::<()>::style_themed(&hidden, &theme);
            assert_eq!(style.width, Dimension::Length(0.0));
            assert_eq!(style.height, Dimension::Length(0.0));
            assert!(painted(&hidden, Rect::new(0.0, 0.0, 24.0, 18.0), &theme).is_empty());
            assert!(Widget::<()>::semantics(&hidden).is_none());
        }
    }

    /// **A count is written out up to its cap, then `cap+`** — `999+` by default, as the
    /// reference's `Badge.count` writes it.
    #[test]
    fn a_count_is_capped() {
        let theme = Theme::default();
        let text = |badge: Badge| texts(&painted(&badge, Rect::new(0.0, 0.0, 40.0, 16.0), &theme));
        assert_eq!(text(Badge::count(7)), vec!["7"]);
        assert_eq!(text(Badge::count(999)), vec!["999"]);
        assert_eq!(text(Badge::count(1000)), vec!["999+"]);
        assert_eq!(text(Badge::count(150).max_count(99)), vec!["99+"]);
        assert_eq!(count_label(5, 9), "5");
        assert_eq!(count_label(10, 9), "9+");
    }

    /// A one-character badge is never narrower than it is tall: a lone digit in a wide
    /// pill reads as a mistake.
    #[test]
    fn a_single_digit_is_round_rather_than_wide() {
        let theme = Theme::default();
        let one = Badge::new("1");
        let style = Widget::<()>::style_themed(&one, &theme);
        assert_eq!(style.width, style.height);
    }

    /// A long count still grows: the floor is a minimum, not a size.
    #[test]
    fn a_long_count_widens_the_pill() {
        let theme = Theme::default();
        let width = |text: &str| match Widget::<()>::style_themed(&Badge::new(text), &theme).width {
            Dimension::Length(px) => px,
            other => panic!("a stated width, not {other:?}"),
        };
        let (narrow, wide) = (width("1"), width("1024"));
        assert!(wide > narrow, "{wide} should exceed {narrow}");
    }

    /// **Pinned, a labelled badge sits where the reference puts it**: on a 24-pixel icon at
    /// the top-end corner, its left edge 12 pixels in (the icon's width less the pill's
    /// height, plus the offset's 4) and its top 4 above the icon's (the offset's -4 and the
    /// 8 the reference adds, less half the pill); a dot right in the corner; and the
    /// mirror image for a right-to-left reading.
    #[test]
    fn a_pinned_badge_sits_where_the_reference_puts_it() {
        let icon = Rect::new(100.0, 50.0, 24.0, 24.0);
        let mut theme = Theme::default();
        let pill = Badge::new("3").pinned_rect(icon, &theme);
        assert_eq!((pill.x, pill.y), (100.0 + 24.0 - 16.0 + 4.0, 50.0 - 4.0));
        assert_eq!(pill.height, 16.0);
        let dot = Badge::dot().pinned_rect(icon, &theme);
        assert_eq!((dot.x, dot.y, dot.width), (100.0 + 24.0 - 6.0, 50.0, 6.0));

        theme.direction = TextDirection::Rtl;
        let pill = Badge::new("3").pinned_rect(icon, &theme);
        assert_eq!(
            (pill.x, pill.y),
            (100.0 - 4.0, 50.0 - 4.0),
            "at the start, mirrored"
        );
        let dot = Badge::dot().pinned_rect(icon, &theme);
        assert_eq!((dot.x, dot.y), (100.0, 50.0));
    }

    /// **Alignment and offset are the instance's, then the theme's**: a bottom-start badge
    /// with no offset sits on the child's bottom-start corner, raised by half its height.
    #[test]
    fn alignment_and_offset_are_the_instances_then_the_themes() {
        let icon = Rect::new(0.0, 0.0, 24.0, 24.0);
        let mut theme = Theme::default();
        let placed = Badge::new("3")
            .alignment(Alignment::new(-1.0, 1.0))
            .offset(0.0, -8.0)
            .pinned_rect(icon, &theme);
        assert_eq!((placed.x, placed.y), (0.0, 24.0 - 8.0));

        theme.widgets.badge.alignment =
            Some(AlignmentGeometry::Physical(Alignment::new(-1.0, 1.0)));
        theme.widgets.badge.offset = Some((0.0, -8.0));
        let themed = Badge::new("3").pinned_rect(icon, &theme);
        assert_eq!((themed.x, themed.y), (0.0, 24.0 - 8.0));
    }

    /// **On a child, the child is the box and the badge is drawn over it**: the child is the
    /// one child, the badge paints after the subtree and reaches past the child's corner, and
    /// a hidden badge leaves the child alone.
    #[test]
    fn on_a_child_the_badge_is_drawn_over_it() {
        let theme = Theme::default();
        let badged: Badged<()> = Badge::count(3).on(crate::Spacer::default());
        assert_eq!(Widget::<()>::children(&badged).len(), 1);
        assert!(badged.paints_over());
        let mut scene = Scene::new();
        badged.paint_over(
            Rect::new(0.0, 0.0, 24.0, 24.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        let pill = scene.primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, .. } => Some(*rect),
            _ => None,
        });
        let pill = pill.expect("the pill");
        assert!(
            pill.y < 0.0 && pill.x + pill.width > 24.0,
            "past the corner: {pill:?}"
        );
        assert_eq!(texts(scene.primitives()), vec!["3"]);

        let hidden: Badged<()> = Badge::count(0)
            .label_visible(false)
            .on(crate::Spacer::default());
        assert!(!hidden.paints_over());
        assert!(Widget::<()>::semantics(&hidden).is_none());
    }

    /// The colours resolve **instance, then theme, then the scheme's role** — the chain
    /// every widget since `Chip` has followed.
    #[test]
    fn the_colours_are_the_instances_then_the_themes() {
        let mut theme = Theme::default();
        let default_fill = |primitives: &[Primitive]| match primitives.first() {
            Some(Primitive::Rect { color, .. }) => *color,
            _ => panic!("a pill"),
        };
        let plain = painted(&Badge::new("3"), Rect::new(0.0, 0.0, 24.0, 18.0), &theme);
        assert_eq!(
            default_fill(&plain),
            theme.scheme.error,
            "the scheme's role"
        );

        theme.widgets.badge.background_color = Some(Color::rgb(0.0, 1.0, 0.0));
        let themed = painted(&Badge::new("3"), Rect::new(0.0, 0.0, 24.0, 18.0), &theme);
        assert_eq!(
            default_fill(&themed),
            Color::rgb(0.0, 1.0, 0.0),
            "the theme"
        );

        let overridden = painted(
            &Badge::new("3").background_color(Color::rgb(0.0, 0.0, 1.0)),
            Rect::new(0.0, 0.0, 24.0, 18.0),
            &theme,
        );
        assert_eq!(
            default_fill(&overridden),
            Color::rgb(0.0, 0.0, 1.0),
            "the instance wins"
        );
    }

    /// A count is read out. A dot is not: there is nothing to say beyond the mark itself,
    /// and announcing an empty label would interrupt a reader with nothing.
    #[test]
    fn a_count_is_announced_and_a_dot_is_not() {
        let counted = Widget::<()>::semantics(&Badge::new("3")).expect("announced");
        assert_eq!(counted.label.as_deref(), Some("3"));
        let counted = Widget::<()>::semantics(&Badge::count(1200)).expect("announced");
        assert_eq!(counted.label.as_deref(), Some("999+"));
        assert!(Widget::<()>::semantics(&Badge::dot()).is_none());
    }
}
