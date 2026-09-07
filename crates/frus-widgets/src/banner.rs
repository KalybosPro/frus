//! [`MaterialBanner`]: a **message across the top of the screen**, with the actions that
//! answer it, staying until one of them is taken.
//!
//! The third of the three the reference has for saying something to the reader, and each
//! is a different promise about how long it stays:
//!
//! | | how long | where |
//! |---|---|---|
//! | [`SnackBar`](crate::SnackBar) | a few seconds | over the bottom |
//! | `MaterialBanner` | until an action is taken | in the flow, above the content |
//! | [`AlertDialog`](crate::AlertDialog) | until it is answered | over everything, with a barrier |
//!
//! It is the one [`Alert`](crate::Alert) resembles without being: an `Alert` has kinds and
//! an accent bar and no actions, which is a callout. A banner **requires** actions —
//! `required this.actions` (`banner.dart:109`) — because a message that stays until it is
//! dismissed and offers no way to dismiss it stays for ever.
//!
//! ```ignore
//! MaterialBanner::new("You have unsaved changes.")
//!     .leading(Icon::new(Icons::WARNING))
//!     .action(button("Discard", Msg::Discard))
//!     .action(button("Save", Msg::Save))
//! ```

use frus_core::{Color, Insets, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The banner's own elevation. `1` in Material 3 (`banner.dart:503`).
pub const BANNER_ELEVATION: f32 = 1.0;
/// The shortest an actions bar gets (`banner.dart:123`).
pub const BANNER_MIN_ACTION_BAR_HEIGHT: f32 = 52.0;
/// The gap between two actions, and the padding around their bar (`banner.dart:363`).
const ACTION_GAP: f32 = 8.0;
/// What the leading slot is held off the content by (`banner.dart:358`).
const LEADING_GAP: f32 = 16.0;
/// The margin under a banner that is off the page at all (`banner.dart:377`).
const RAISED_MARGIN: f32 = 10.0;

/// The banner's padding, which depends on **where the actions went**
/// (`banner.dart:352`): tucked into the row beside the content, or on a line of their own.
fn default_padding(single_row: bool) -> Insets {
    if single_row {
        Insets::new(2.0, 0.0, 0.0, 16.0)
    } else {
        Insets::new(24.0, 16.0, 4.0, 16.0)
    }
}

/// What the banner says: `bodyMedium` (`banner.dart:519`).
fn content_style(over: Option<TextStyle>, theme: Option<&Theme>) -> TextStyle {
    over.or(theme.and_then(|t| t.widgets.banner.content_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_medium)
}

/// A message across the top of the screen with the actions that answer it.
///
/// It sits **in the flow**, not over it: a banner is put above the content it is about,
/// usually as the first child of a screen's column. Nothing hides it on a timer, which is
/// the whole difference from a snack bar — an action does.
pub struct MaterialBanner<Msg> {
    content: Option<Box<dyn Widget<Msg>>>,
    content_text_style: Option<TextStyle>,
    leading: Option<Box<dyn Widget<Msg>>>,
    actions: Vec<Box<dyn Widget<Msg>>>,
    background: Option<Color>,
    surface_tint: Option<Color>,
    shadow_color: Option<Color>,
    divider_color: Option<Color>,
    elevation: Option<f32>,
    padding: Option<Insets>,
    margin: Option<Insets>,
    leading_padding: Option<Insets>,
    force_actions_below: bool,
    min_action_bar_height: f32,
}

impl<Msg: Clone + 'static> MaterialBanner<Msg> {
    /// A banner saying `content`. **Add at least one action**: a banner stays until one is
    /// taken, and the reference makes them required for that reason.
    pub fn new(content: impl Into<String>) -> Self {
        Self::with_content(crate::Text::new(content))
    }

    /// The same, with a widget in place of the text.
    pub fn with_content(content: impl Widget<Msg> + 'static) -> Self {
        Self {
            content: Some(Box::new(content)),
            content_text_style: None,
            leading: None,
            actions: Vec::new(),
            background: None,
            surface_tint: None,
            shadow_color: None,
            divider_color: None,
            elevation: None,
            padding: None,
            margin: None,
            leading_padding: None,
            force_actions_below: false,
            min_action_bar_height: BANNER_MIN_ACTION_BAR_HEIGHT,
        }
    }

    /// A glyph before the message.
    #[must_use]
    pub fn leading(mut self, leading: impl Widget<Msg> + 'static) -> Self {
        self.leading = Some(Box::new(leading));
        self
    }

    /// Adds an action. **One** action sits on the message's own line; two or more take a
    /// line of their own (`banner.dart:348`).
    #[must_use]
    pub fn action(mut self, action: impl Widget<Msg> + 'static) -> Self {
        self.actions.push(Box::new(action));
        self
    }

    /// Puts the actions on a line of their own even when there is only one.
    #[must_use]
    pub fn force_actions_below(mut self, force: bool) -> Self {
        self.force_actions_below = force;
        self
    }

    /// What the message is set in. `bodyMedium` by default.
    #[must_use]
    pub fn content_text_style(mut self, style: TextStyle) -> Self {
        self.content_text_style = Some(style);
        self
    }

    /// The banner's colour. A low container tone by default.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// What the surface is tinted towards for its elevation. Unset, nothing is — the
    /// reference's Material 3 answer (`banner.dart:513`).
    #[must_use]
    pub fn surface_tint(mut self, color: Color) -> Self {
        self.surface_tint = Some(color);
        self
    }

    /// The shadow's colour. Unset, there is none.
    #[must_use]
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = Some(color);
        self
    }

    /// The rule along the bottom, which is drawn **only when the banner is flat** — see
    /// [`Self::elevation`].
    #[must_use]
    pub fn divider_color(mut self, color: Color) -> Self {
        self.divider_color = Some(color);
        self
    }

    /// How far off the page it sits. `1` by default.
    ///
    /// It decides two other things, which is why it is worth naming: a banner off the page
    /// keeps a 10-pixel margin under it so its shadow has room, and a banner **flat** on
    /// the page draws a rule along its bottom edge instead (`banner.dart:425`). Height or
    /// a line, never both — the same either-or `Card` makes between a shadow and a
    /// hairline.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// The padding around the message row.
    #[must_use]
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The margin around the banner.
    #[must_use]
    pub fn margin(mut self, margin: Insets) -> Self {
        self.margin = Some(margin);
        self
    }

    /// What the leading slot is held off the message by.
    #[must_use]
    pub fn leading_padding(mut self, padding: Insets) -> Self {
        self.leading_padding = Some(padding);
        self
    }

    /// The shortest the actions bar gets. `52` by default.
    #[must_use]
    pub fn min_action_bar_height(mut self, height: f32) -> Self {
        self.min_action_bar_height = height;
        self
    }

    /// Assembles the banner.
    ///
    /// Deferred to a [`ThemeBuilder`](crate::ThemeBuilder): whether the actions sit on the
    /// message's line decides **which children exist and in what order**, and the padding
    /// and the rule are the theme's to answer, so the composition cannot happen before the
    /// theme is known.
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        Box::new(crate::ThemeBuilder::boxed(move |theme| {
            self.build_with(theme)
        }))
    }

    fn build_with(self, theme: &Theme) -> Box<dyn Widget<Msg>> {
        let t = theme.widgets.banner;
        let MaterialBanner {
            content,
            content_text_style,
            leading,
            actions,
            background,
            surface_tint,
            shadow_color,
            divider_color,
            elevation,
            padding,
            margin,
            leading_padding,
            force_actions_below,
            min_action_bar_height,
        } = self;

        // **One action goes on the message's line; two do not.** The reference's rule, and
        // the reason its padding has two shapes: the tucked-in bar needs almost no top
        // padding, the one below needs the message to breathe above it.
        let single_row = actions.len() == 1 && !force_actions_below;
        let padding = padding.or(t.padding).unwrap_or(default_padding(single_row));
        let leading_padding = leading_padding.or(t.leading_padding).unwrap_or(Insets::new(
            0.0,
            LEADING_GAP,
            0.0,
            0.0,
        ));
        let elevation = elevation.or(t.elevation).unwrap_or(BANNER_ELEVATION);

        let actions_bar = |actions: Vec<Box<dyn Widget<Msg>>>| {
            let mut row = crate::Flex::row().justify(Justify::End).gap(ACTION_GAP);
            for action in actions {
                row = row.child_boxed(action);
            }
            // A **floor**, not a height: the reference constrains the bar's minimum
            // (`banner.dart:361`) so a taller action is not squeezed into 52 pixels. An
            // imposed height would clip one, which is the failure a floor is written to
            // avoid.
            crate::ConstrainedBox::new(
                crate::Flex::row()
                    .justify(Justify::End)
                    .align(Align::Center)
                    .padding_each(0.0, ACTION_GAP, 0.0, ACTION_GAP)
                    .child(row),
            )
            .min_height(min_action_bar_height)
        };

        // The message row: the leading slot, then the message taking what is left, then
        // the actions when there is one of them.
        let mut message = crate::Flex::row().align(Align::Center);
        if let Some(leading) = leading {
            message = message.child(
                crate::Flex::row()
                    .padding_each(
                        leading_padding.top,
                        leading_padding.right,
                        leading_padding.bottom,
                        leading_padding.left,
                    )
                    .child_boxed(leading),
            );
        }
        if let Some(content) = content {
            message = message.child(
                crate::Expanded::new(crate::Themed::tweak(
                    {
                        let style = content_style(content_text_style, Some(theme));
                        move |t: &mut Theme| t.widgets.text.style = style
                    },
                    content,
                ))
                .flex(1.0),
            );
        }
        let mut actions = actions;
        if single_row {
            message = message.child(actions_bar(std::mem::take(&mut actions)));
        }

        // **The padding is a column, not a row.** A row's child sits on its main axis and
        // takes the width of its own content, so the message row inside would hug and the
        // `Expanded` in it would have nothing to expand into — the action would land beside
        // the words instead of at the far end of the line. A column stretches its child
        // across, which is what puts the row on the banner's full width.
        let mut column = crate::Flex::column().child(
            crate::Flex::column()
                .padding_each(padding.top, padding.right, padding.bottom, padding.left)
                .child(message),
        );
        if !actions.is_empty() {
            column = column.child(actions_bar(actions));
        }
        // **A rule only when the banner is flat.** Off the page it casts a shadow instead,
        // and a banner with both would be saying its height twice.
        if elevation == 0.0 {
            // The reference's `Divider(height: 0)` means the line with no air around
            // it. Here the height **is** the room the separator takes, so zero takes none
            // and draws nothing: the flush hairline is `height == thickness`.
            let mut divider = crate::Divider::new().height(crate::DIVIDER_THICKNESS);
            if let Some(color) = divider_color.or(t.divider_color) {
                divider = divider.color(color);
            }
            column = column.child(divider);
        }

        let margin = margin.or(t.margin).unwrap_or(if elevation > 0.0 {
            Insets::new(0.0, 0.0, RAISED_MARGIN, 0.0)
        } else {
            Insets::ZERO
        });

        Box::new(BannerSurface {
            children: vec![Box::new(column)],
            background: background.or(t.color),
            surface_tint: surface_tint.or(t.surface_tint),
            shadow_color: shadow_color.or(t.shadow_color),
            elevation,
            margin,
        })
    }
}

/// The banner's surface: a flat block of colour with the margin around it, and a shadow
/// when it is off the page.
struct BannerSurface<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    background: Option<Color>,
    surface_tint: Option<Color>,
    shadow_color: Option<Color>,
    elevation: f32,
    margin: Insets,
}

impl<Msg> BannerSurface<Msg> {
    /// A banner sits just off the page, on the ladder's low rung
    /// (`banner.dart:510`).
    fn background(&self, theme: &Theme) -> Color {
        let base = self
            .background
            .unwrap_or(theme.scheme.surface_container_low);
        match self.surface_tint {
            Some(tint) => base.surface_tint(tint, self.elevation),
            None => base,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for BannerSurface<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            height: Dimension::Auto,
            margin: self.margin,
            ..Default::default()
        }
    }

    /// The width it was **offered**, not the width its parent came out at.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        if self.elevation > 0.0 {
            if let Some(shadow) = self.shadow_color.filter(|c| c.a > 0.0) {
                let blur = self.elevation * 4.0 + 8.0;
                scene.shadow(
                    Rect::new(
                        bounds.x - blur,
                        bounds.y + self.elevation * 2.0 - blur,
                        bounds.width + 2.0 * blur,
                        bounds.height + 2.0 * blur,
                    ),
                    shadow.fade(o),
                    frus_core::BorderRadius::ZERO.inflate(blur),
                    blur,
                );
            }
        }
        scene.fill_rect(bounds, self.background(theme).fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, dsl::button, Runtime, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Save,
        Discard,
    }

    fn rects(banner: &dyn Widget<Msg>) -> Vec<(frus_core::Rect, Color)> {
        build_ui(
            banner,
            Size::new(400.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            frus_core::Primitive::Rect { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
    }

    /// **One action rides on the message's line; two take a line of their own.** The
    /// reference's rule (`banner.dart:348`), and the reason its padding has two shapes.
    #[test]
    fn a_second_action_moves_the_bar_onto_its_own_line() {
        let height = |actions: usize| {
            let mut banner = MaterialBanner::<Msg>::new("You have unsaved changes.")
                .action(button("Save", Msg::Save));
            if actions > 1 {
                banner = banner.action(button("Discard", Msg::Discard));
            }
            let tree = banner.build();
            build_ui(
                tree.as_ref(),
                Size::new(400.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            )
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, .. } if rect.width > 300.0 => Some(rect.height),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
        };
        assert!(
            height(2) > height(1),
            "two actions did not take a line of their own: {} vs {}",
            height(2),
            height(1)
        );
    }

    /// The same switch, said the other way round: forcing the actions below makes a single
    /// action behave like two.
    #[test]
    fn a_single_action_can_be_forced_onto_its_own_line() {
        let height = |forced: bool| {
            let tree = MaterialBanner::<Msg>::new("You have unsaved changes.")
                .action(button("Save", Msg::Save))
                .force_actions_below(forced)
                .build();
            build_ui(
                tree.as_ref(),
                Size::new(400.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            )
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, .. } if rect.width > 300.0 => Some(rect.height),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
        };
        assert!(height(true) > height(false));
    }

    /// **A rule only when the banner is flat.** Off the page it casts a shadow instead,
    /// and one with both would be saying its height twice (`banner.dart:425`).
    #[test]
    fn the_rule_is_drawn_only_where_there_is_no_height_to_show() {
        let rule = |elevation: f32| {
            let tree = MaterialBanner::<Msg>::new("Message")
                .action(button("Save", Msg::Save))
                .elevation(elevation)
                .divider_color(Color::rgb8(255, 0, 255))
                .build();
            rects(tree.as_ref())
                .into_iter()
                .any(|(_, c)| c == Color::rgb8(255, 0, 255))
        };
        assert!(rule(0.0), "a flat banner draws no rule");
        assert!(
            !rule(1.0),
            "a raised banner drew a rule as well as a shadow"
        );
    }

    /// The message takes the room the leading and the actions leave, rather than the width
    /// of its own words — which is what `Expanded` is for and what a banner needs, since
    /// the single action sits at the **far end** of the message's line.
    #[test]
    fn the_message_takes_what_is_left_of_the_line() {
        const WIDTH: f32 = 400.0;
        let tree = MaterialBanner::<Msg>::new("A message")
            .action(button("Save", Msg::Save))
            .build();
        let ui = build_ui(
            tree.as_ref(),
            Size::new(WIDTH, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let box_of = |wanted: &str| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, bounds, .. } if text == wanted => {
                        Some(*bounds)
                    }
                    _ => None,
                })
                .expect("drawn")
        };
        let message = box_of("A message");
        let action = box_of("Save");
        assert!(
            message.x + message.width < action.x,
            "the action is not past the message: {message:?} {action:?}"
        );
        // And it is at the far end: within the bar's own padding of the banner's edge,
        // rather than sitting wherever a hugging message left it.
        assert!(
            action.x + action.width > WIDTH - 48.0,
            "the action did not reach the trailing edge: {action:?}"
        );
    }
}
