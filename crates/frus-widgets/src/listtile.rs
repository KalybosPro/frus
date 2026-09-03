//! [`ListTile`]: a row of a list — something in front, one or two lines of text, and
//! something behind.
//!
//! The most reached-for row in the reference's catalogue, and the shape behind a drawer
//! entry, a settings line, a contact, a menu item. It was the largest single gap in this
//! framework's coverage of that catalogue.
//!
//! ```ignore
//! ListTile::new()
//!     .leading(Icon::new(Icons::Star))
//!     .title("Starred")
//!     .subtitle("Everything you kept")
//!     .trailing(Icon::new(Icons::ChevronRight))
//!     .on_tap(Msg::OpenStarred)
//! ```
//!
//! The measurements are the reference's Material 3 defaults: 16 px in front, 24 behind, a
//! 16 px gap either side of the text, and a height of **56** for one line, **72** for two
//! and **88** for three — 48 / 64 / 76 when dense. Every one of them is overridable, and
//! each of the four slots takes a widget when a caller wants something the tile would not
//! have made.

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Insets, Rect, Scene, ShapeBorder, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::constraints::ConstrainedBox;
use crate::disabled::disabled_content;
use crate::expanded::Expanded;
use crate::flex::Flex;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

/// Room in front of the leading slot — the start of the reference's
/// `EdgeInsetsDirectional.only(start: 16, end: 24)`.
pub const LIST_TILE_PADDING_START: f32 = 16.0;
/// Room behind the trailing slot. See [`LIST_TILE_PADDING_START`].
pub const LIST_TILE_PADDING_END: f32 = 24.0;
/// The gap between the leading slot and the text, and between the text and the trailing
/// slot (`horizontalTitleGap`).
pub const LIST_TILE_TITLE_GAP: f32 = 16.0;
/// The narrowest the leading slot may be, so that a column of tiles lines its text up
/// whatever each row put in front of it (`minLeadingWidth`).
pub const LIST_TILE_MIN_LEADING_WIDTH: f32 = 24.0;
/// The least room above and below the text (`minVerticalPadding`).
pub const LIST_TILE_MIN_VERTICAL_PADDING: f32 = 8.0;
/// Heights by line count — one, two, three.
pub const LIST_TILE_HEIGHTS: [f32; 3] = [56.0, 72.0, 88.0];
/// The same, dense. See [`LIST_TILE_HEIGHTS`].
pub const LIST_TILE_DENSE_HEIGHTS: [f32; 3] = [48.0, 64.0, 76.0];

/// A text slot: a string the tile styles itself, or a widget the caller styled.
enum Slot<Msg> {
    Text(String),
    Child(Box<dyn Widget<Msg>>),
}

/// A row of a list: leading, title, subtitle, trailing.
///
/// The reference's Material 3 measurements — 16 px in front, 24 behind, a 16 px gap
/// either side of the text, heights of 56 / 72 / 88 by line count and 48 / 64 / 76 dense —
/// and every one of them replaceable, as is each of the four slots.
pub struct ListTile<Msg> {
    leading: RefCell<Option<Box<dyn Widget<Msg>>>>,
    trailing: RefCell<Option<Box<dyn Widget<Msg>>>>,
    title: RefCell<Option<Slot<Msg>>>,
    subtitle: RefCell<Option<Slot<Msg>>>,
    has_subtitle: bool,
    three_line: bool,
    dense: bool,
    selected: bool,
    enabled: bool,
    on_tap: Option<Msg>,
    padding: Option<Insets>,
    title_gap: Option<f32>,
    min_leading_width: Option<f32>,
    min_height: Option<f32>,
    title_style: Option<TextStyle>,
    subtitle_style: Option<TextStyle>,
    tile_color: Option<Color>,
    selected_color: Option<Color>,
    /// The tile's surface **while it is the chosen one** — the reference's
    /// `selectedTileColor`.
    selected_tile_color: Option<Color>,
    /// The colour of the two slots' icons, over what the selection would give them.
    icon_color: Option<Color>,
    /// The colour of the title and subtitle, over what the selection would give them.
    text_color: Option<Color>,
    /// What shape the tile is — the reference's `shape`. The surface and the ink both
    /// take it, so a rounded tile in a list is rounded all the way through.
    shape: Option<ShapeBorder>,
    /// The assembled row, as the one-element slice [`Widget::children`] hands back.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg> Default for ListTile<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> ListTile<Msg> {
    /// An empty tile. Give it at least a title.
    pub fn new() -> Self {
        Self {
            leading: RefCell::new(None),
            trailing: RefCell::new(None),
            title: RefCell::new(None),
            subtitle: RefCell::new(None),
            has_subtitle: false,
            three_line: false,
            dense: false,
            selected: false,
            enabled: true,
            on_tap: None,
            padding: None,
            title_gap: None,
            min_leading_width: None,
            min_height: None,
            title_style: None,
            subtitle_style: None,
            tile_color: None,
            selected_color: None,
            selected_tile_color: None,
            icon_color: None,
            text_color: None,
            shape: None,
            built: OnceCell::new(),
        }
    }

    /// What goes in front: an icon, an avatar, a checkbox.
    pub fn leading(self, leading: impl Widget<Msg> + 'static) -> Self {
        *self.leading.borrow_mut() = Some(Box::new(leading));
        self
    }

    /// What goes behind: a chevron, a switch, a timestamp.
    pub fn trailing(self, trailing: impl Widget<Msg> + 'static) -> Self {
        *self.trailing.borrow_mut() = Some(Box::new(trailing));
        self
    }

    /// The first line, styled by the tile (`body_large` on `on_surface`).
    pub fn title(self, title: impl Into<String>) -> Self {
        *self.title.borrow_mut() = Some(Slot::Text(title.into()));
        self
    }

    /// The first line as a **widget**, for a caller who wants something the tile would
    /// not have made — a rich span, a row of chips.
    pub fn title_child(self, title: impl Widget<Msg> + 'static) -> Self {
        *self.title.borrow_mut() = Some(Slot::Child(Box::new(title)));
        self
    }

    /// The second line, styled by the tile (`body_medium` on `on_surface_variant`).
    /// Its presence is what makes the tile a two-line one.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        *self.subtitle.borrow_mut() = Some(Slot::Text(subtitle.into()));
        self.has_subtitle = true;
        self
    }

    /// The second line as a **widget**. See [`Self::title_child`].
    pub fn subtitle_child(mut self, subtitle: impl Widget<Msg> + 'static) -> Self {
        *self.subtitle.borrow_mut() = Some(Slot::Child(Box::new(subtitle)));
        self.has_subtitle = true;
        self
    }

    /// Room for three lines rather than two, for a subtitle that wraps.
    pub fn three_line(mut self) -> Self {
        self.three_line = true;
        self
    }

    /// The compact height, for a dense list.
    pub fn dense(mut self) -> Self {
        self.dense = true;
        self
    }

    /// Marks the tile as the chosen one: its text and its two slots take the primary
    /// colour.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Available or not. A disabled tile flattens and stops answering.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The message a tap sends.
    pub fn on_tap(mut self, msg: Msg) -> Self {
        self.on_tap = Some(msg);
        self
    }

    /// Room around the content, replacing the 16 / 24 default.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The gap either side of the text.
    pub fn title_gap(mut self, gap: f32) -> Self {
        self.title_gap = Some(gap);
        self
    }

    /// The narrowest the leading slot may be.
    pub fn min_leading_width(mut self, width: f32) -> Self {
        self.min_leading_width = Some(width);
        self
    }

    /// A height of the caller's choosing, replacing the one the line count implies.
    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    /// The title's text style.
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// The subtitle's text style.
    pub fn subtitle_style(mut self, style: TextStyle) -> Self {
        self.subtitle_style = Some(style);
        self
    }

    /// The tile's own background. Transparent by default, as the reference has it: a
    /// tile takes the colour of whatever it is sitting on.
    pub fn tile_color(mut self, color: Color) -> Self {
        self.tile_color = Some(color);
        self
    }

    /// The colour the text and the two slots take when the tile is the chosen one.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = Some(color);
        self
    }

    /// **The tile's surface while it is the chosen one** — the reference's
    /// `selectedTileColor` (`list_tile.dart`).
    ///
    /// A selected tile used to change nothing but the colour of its words, which is the
    /// weakest possible way to say *this is the one you are on*: it is the difference
    /// between a highlighted row in a navigation list and a row that merely reads
    /// differently. Unset, the tile keeps its ordinary surface, as the reference's does.
    pub fn selected_tile_color(mut self, color: Color) -> Self {
        self.selected_tile_color = Some(color);
        self
    }

    /// **The two slots' icon colour**, over what the selection would give them — the
    /// reference's `iconColor`.
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// **The title's and subtitle's colour**, over what the selection would give them —
    /// the reference's `textColor`.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// **What shape the tile is** — the reference's `shape`. The surface takes it, and so
    /// does the ink, so a rounded tile does not splash square corners over a list.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    /// **What shape the tile paints as**: the caller's, then a plain rectangle — which is
    /// what a row in a list is unless somebody says otherwise.
    fn shape_of(&self) -> ShapeBorder {
        self.shape.unwrap_or(ShapeBorder::rounded(0.0))
    }

    /// The height this tile asks for: the reference's, by line count and density, unless
    /// the caller named one.
    pub fn height(&self) -> f32 {
        if let Some(height) = self.min_height {
            return height;
        }
        let row = if self.three_line {
            2
        } else if self.has_subtitle {
            1
        } else {
            0
        };
        if self.dense {
            LIST_TILE_DENSE_HEIGHTS[row]
        } else {
            LIST_TILE_HEIGHTS[row]
        }
    }

    /// The colour a line of text resolves to. Disabled wins over chosen, as everywhere
    /// else: a tile that cannot be picked does not advertise that it was.
    fn content_color(&self, theme: &Theme, subtitle: bool) -> Color {
        if !self.enabled {
            disabled_content(theme)
        } else if self.selected {
            self.selected_color.unwrap_or(theme.primary)
        } else if subtitle {
            theme.scheme.on_surface_variant
        } else {
            theme.scheme.on_surface
        }
    }
}

impl<Msg: Clone + 'static> ListTile<Msg> {
    /// Assembles the row, once, under the theme in force where the tile sits.
    ///
    /// Not at construction time: a theme reaches text styles and colours, and a tile
    /// built under `Theme::default()` inside a [`crate::Themed`] subtree would come out
    /// in the wrong palette.
    fn row(&self, theme: &Theme) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| {
            let gap = self.title_gap.unwrap_or(LIST_TILE_TITLE_GAP);
            let mut row = Flex::row().align(Align::Center).gap(gap);

            if let Some(leading) = self.leading.borrow_mut().take() {
                let min = self
                    .min_leading_width
                    .unwrap_or(LIST_TILE_MIN_LEADING_WIDTH);
                row = row.child(ConstrainedBox::new_boxed(leading).min_width(min));
            }

            // The text column is the one part that gives way when the row is narrow:
            // the slots either side keep their size and the lines are cut.
            let mut column = Flex::column().align(Align::Start).justify(Justify::Center);
            for (slot, subtitle) in [(&self.title, false), (&self.subtitle, true)] {
                let taken = slot.borrow_mut().take();
                match taken {
                    Some(Slot::Child(child)) => column = column.child_boxed(child),
                    Some(Slot::Text(content)) => {
                        let base = if subtitle {
                            self.subtitle_style.unwrap_or(theme.text.body_medium)
                        } else {
                            self.title_style.unwrap_or(theme.text.body_large)
                        };
                        let mut text =
                            Text::styled(content, base).color(self.content_color(theme, subtitle));
                        // A tile is a fixed height, so a line that wrapped would run out
                        // of the bottom of it — except the subtitle of a three-line tile,
                        // which is what the extra room is for.
                        if subtitle && self.three_line {
                            text = text.wrap();
                        } else {
                            text = text.ellipsis();
                        }
                        column = column.child(text);
                    }
                    None => {}
                }
            }
            row = row.child(Expanded::new(column));

            if let Some(trailing) = self.trailing.borrow_mut().take() {
                row = row.child_boxed(trailing);
            }
            // The row is **`Expanded`**, and both halves of that matter.
            //
            // It **grows** into the tile rather than hugging its slots: left to hug, the
            // tile is the right width and the row inside it is not, so the trailing slot
            // comes to rest against the title instead of at the end of the line — a badge
            // drawn halfway across a row, and the whole reason the text column between
            // them is an `Expanded` in the first place.
            //
            // And it **gives way** rather than running past the edge. Since milestone 349
            // nothing is squeezed unless it says so, so a title too long for the row
            // pushed it straight through the tile: 265 px outside a 200 px box, reported
            // by the overflow band and drawn off the side of the list. `Expanded` says
            // both at once — grow, shrink, and no content-sized floor underneath.
            vec![Box::new(Expanded::new(row)) as Box<dyn Widget<Msg>>]
        })
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ListTile<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    /// It asks to **fill the width it is offered** rather than declaring one.
    ///
    /// See [`Widget::fill_axes`]. A `width: 100%` resolves against the parent's
    /// *resolved* width, which a parent that shrink-wraps does not have yet — so a tile in
    /// a plain column, the most ordinary thing anybody does with one, came out as wide as
    /// its own padding and ellipsised its title to nothing.
    fn style_themed(&self, _theme: &Theme) -> Style {
        Style {
            min_height: Dimension::Length(self.height()),
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            padding: self.padding.unwrap_or(Insets::new(
                LIST_TILE_MIN_VERTICAL_PADDING,
                LIST_TILE_PADDING_END,
                LIST_TILE_MIN_VERTICAL_PADDING,
                LIST_TILE_PADDING_START,
            )),
            ..Default::default()
        }
    }

    /// The width it was offered, not the width its parent came out at — the difference
    /// between a number known on the way **down** and one only known on the way back up.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    fn build_themed(&self, theme: &Theme) {
        self.row(theme);
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get().map(|v| &v[..]).unwrap_or(&[])
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Transparent by default, as the reference has it, so a tile takes the colour of
        // whatever it sits on; a state layer over that when it can be tapped.
        // A selected tile takes its own surface first: the reference's
        // `selectedTileColor` outranks `tileColor` while the tile is the chosen one.
        let base = if self.selected {
            self.selected_tile_color.or(self.tile_color)
        } else {
            self.tile_color
        }
        .unwrap_or(Color::TRANSPARENT);
        let color = if self.enabled && self.on_tap.is_some() {
            theme.state_layer(base, theme.scheme.on_surface, &status)
        } else {
            base
        };
        if color.a > 0.0 {
            scene.draw_shape(bounds, self.shape_of(), color.fade(status.opacity));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.on_tap.clone()).flatten()
    }

    fn ink(&self, theme: &Theme) -> Option<crate::ink::InkStyle> {
        (self.enabled && self.on_tap.is_some()).then(|| {
            let mut ink = crate::ink::InkStyle::of(theme);
            // The ink is clipped to the tile, so it has to know the same shape the surface
            // does — otherwise a rounded tile splashes square corners over the list.
            if let Some((_, radius)) = self
                .shape
                .and_then(|shape| shape.as_rounded(Rect::new(0.0, 0.0, 1000.0, self.height())))
            {
                ink.radius = radius;
            }
            ink
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// **A selected tile has a surface of its own** (`list_tile.dart`'s
    /// `selectedTileColor`), which it did not.
    ///
    /// Being *the one you are on* used to change nothing but the colour of the words — the
    /// weakest possible way to say it, and the difference between a highlighted row in a
    /// navigation list and a row that merely reads differently.
    #[test]
    fn a_selected_tile_has_a_surface_of_its_own() {
        let surface = |tile: &ListTile<Msg>| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                tile,
                Rect::new(0.0, 0.0, 300.0, 56.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            scene.primitives().iter().find_map(|p| match p {
                frus_core::Primitive::Rect { color, radius, .. } => Some((*color, *radius)),
                _ => None,
            })
        };

        // Nothing said: a tile is transparent and paints no surface at all, as before.
        assert_eq!(surface(&ListTile::<Msg>::new().selected(true)), None);

        let chosen = Color::rgb(0.1, 0.2, 0.3);
        assert_eq!(
            surface(
                &ListTile::<Msg>::new()
                    .selected(true)
                    .selected_tile_color(chosen)
            )
            .expect("a surface")
            .0,
            chosen
        );
        // And only while it is the chosen one.
        assert_eq!(
            surface(&ListTile::<Msg>::new().selected_tile_color(chosen)),
            None
        );
        // It outranks the ordinary surface rather than replacing it everywhere.
        let plain = Color::rgb(0.9, 0.9, 0.9);
        assert_eq!(
            surface(
                &ListTile::<Msg>::new()
                    .tile_color(plain)
                    .selected_tile_color(chosen)
            )
            .expect("a surface")
            .0,
            plain,
            "unselected"
        );
        assert_eq!(
            surface(
                &ListTile::<Msg>::new()
                    .selected(true)
                    .tile_color(plain)
                    .selected_tile_color(chosen)
            )
            .expect("a surface")
            .0,
            chosen,
            "selected"
        );
    }

    /// **And a shape**, which the surface and the ink both take — a rounded tile that
    /// splashed square corners would be worse than one that was never rounded.
    #[test]
    fn a_tile_takes_a_shape_and_the_ink_takes_it_too() {
        let tile = ListTile::<Msg>::new()
            .tile_color(Color::rgb(0.5, 0.5, 0.5))
            .on_tap(Msg::Tapped)
            .shape(frus_core::ShapeBorder::rounded(12.0));
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &tile,
            Rect::new(0.0, 0.0, 300.0, 56.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let radius = scene.primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Rect { radius, .. } => Some(*radius),
            _ => None,
        });
        assert_eq!(radius, Some(frus_core::BorderRadius::uniform(12.0)));
        assert_eq!(
            Widget::<Msg>::ink(&tile, &Theme::default())
                .expect("a tappable tile inks")
                .radius,
            frus_core::BorderRadius::uniform(12.0),
            "and the ink is clipped to the same shape"
        );
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Tapped,
    }

    /// The reference's heights, by line count and density.
    #[test]
    fn the_height_follows_the_line_count() {
        let one: ListTile<Msg> = ListTile::new().title("a");
        assert_eq!(one.height(), 56.0);
        let two: ListTile<Msg> = ListTile::new().title("a").subtitle("b");
        assert_eq!(two.height(), 72.0);
        let three: ListTile<Msg> = ListTile::new().title("a").subtitle("b").three_line();
        assert_eq!(three.height(), 88.0);
        let dense: ListTile<Msg> = ListTile::new().title("a").dense();
        assert_eq!(dense.height(), 48.0);
        let asked: ListTile<Msg> = ListTile::new().title("a").min_height(100.0);
        assert_eq!(asked.height(), 100.0);
    }

    /// The four slots are assembled in order, and only those that were given.
    #[test]
    fn it_assembles_the_slots_it_was_given() {
        let theme = Theme::default();
        let bare: ListTile<Msg> = ListTile::new().title("only a title");
        Widget::<Msg>::build_themed(&bare, &theme);
        let row = &Widget::<Msg>::children(&bare)[0];
        assert_eq!(row.children().len(), 1, "just the text column");

        let full: ListTile<Msg> = ListTile::new()
            .leading(crate::Icon::new(crate::Icons::Star))
            .title("title")
            .subtitle("subtitle")
            .trailing(crate::Icon::new(crate::Icons::ChevronRight));
        Widget::<Msg>::build_themed(&full, &theme);
        let row = &Widget::<Msg>::children(&full)[0];
        assert_eq!(row.children().len(), 3, "leading, text, trailing");
    }

    /// A disabled tile does not answer, and says so in its colour before it is asked.
    #[test]
    fn a_disabled_tile_is_inert_and_flattened() {
        let theme = Theme::default();
        let tile: ListTile<Msg> = ListTile::new()
            .title("x")
            .on_tap(Msg::Tapped)
            .enabled(false);
        assert_eq!(Widget::<Msg>::on_click(&tile), None);
        assert_eq!(tile.content_color(&theme, false), disabled_content(&theme));
        assert!(Widget::<Msg>::ink(&tile, &theme).is_none());
    }

    /// Chosen wins over the ordinary roles, and disabled wins over chosen: a tile that
    /// cannot be picked does not advertise that it was.
    #[test]
    fn selected_takes_the_primary_and_disabled_takes_it_back() {
        let theme = Theme::default();
        let chosen: ListTile<Msg> = ListTile::new().title("x").selected(true);
        assert_eq!(chosen.content_color(&theme, false), theme.primary);
        assert_eq!(chosen.content_color(&theme, true), theme.primary);

        let both: ListTile<Msg> = ListTile::new().title("x").selected(true).enabled(false);
        assert_eq!(both.content_color(&theme, false), disabled_content(&theme));
    }

    /// Untouched, the two lines take the reference's roles: `on_surface` for the title,
    /// `on_surface_variant` for the one below it.
    #[test]
    fn the_two_lines_take_their_own_roles() {
        let theme = Theme::default();
        let tile: ListTile<Msg> = ListTile::new().title("x").subtitle("y");
        assert_eq!(tile.content_color(&theme, false), theme.scheme.on_surface);
        assert_eq!(
            tile.content_color(&theme, true),
            theme.scheme.on_surface_variant
        );
    }

    #[test]
    fn a_tap_sends_its_message() {
        let tile: ListTile<Msg> = ListTile::new().title("x").on_tap(Msg::Tapped);
        assert_eq!(Widget::<Msg>::on_click(&tile), Some(Msg::Tapped));
    }
}

#[cfg(test)]
mod fill_tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::Primitive;

    const END: Color = Color::rgb(1.0, 0.0, 0.0);

    /// The box the trailing slot was drawn in, laid out at a definite width.
    fn trailing_rect(width: f32) -> Rect {
        rect_with(width, "Row")
    }

    /// The same, with the title the caller chooses.
    fn rect_with(width: f32, title: &str) -> Rect {
        let tile: ListTile<()> = ListTile::new()
            .title(title.to_string())
            .trailing(Container::<()>::new().width(24.0).height(24.0).color(END));
        let root = Container::<()>::new().width(width).child(tile);
        build_ui(
            &root,
            Size::new(width, 80.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .iter()
        .find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if *color == END => Some(*rect),
            _ => None,
        })
        .expect("the trailing slot is painted")
    }

    /// The trailing slot belongs at the **end** of the row.
    ///
    /// It did not get there for a long time: the tile was the right width and the row
    /// inside it was not, so the row hugged its slots and the `Expanded` text column
    /// between them had nothing to push against — every badge, count and chevron drew
    /// itself halfway across the line. Milestone 368 found it with a chevron; this is the
    /// assertion that keeps it found.
    /// A title too long for the row is **cut**, not carried past the edge.
    ///
    /// The same fix has both halves: a row that only grows fills the tile and then runs
    /// straight through it when its content is wider, because since milestone 349 nothing
    /// is squeezed unless it says so. The trailing slot ended up 111 px outside a 200 px
    /// tile, which is a badge drawn off the side of a list.
    #[test]
    fn a_title_too_long_is_cut_rather_than_carried_past_the_edge() {
        let long = "A title far too long for two hundred pixels of row";
        let rect = rect_with(200.0, long);
        assert!(
            rect.x + rect.width <= 200.0 + 1.0,
            "the slot ends at {} rather than inside the tile",
            rect.x + rect.width
        );
    }

    #[test]
    fn the_trailing_slot_reaches_the_end_of_the_row() {
        for width in [200.0, 320.0, 500.0] {
            let rect = trailing_rect(width);
            assert!(
                (rect.x + rect.width - (width - LIST_TILE_PADDING_END)).abs() <= 1.0,
                "at {width}: the slot ends at {} rather than {}",
                rect.x + rect.width,
                width - LIST_TILE_PADDING_END
            );
        }
    }
    /// **A widget that fills the width does it in a column too**, which is where the
    /// framework's own idiom for it was wrong in fifteen places at once.
    ///
    /// `width: 100%` resolves against the parent's **resolved** width. A parent that
    /// shrink-wraps has not got one yet — it is waiting on this very child — so the
    /// percentage resolves against nothing and the box comes out empty. A list tile in a
    /// plain column, the most ordinary thing anybody does with one, was as wide as its own
    /// padding and ellipsised its title away.
    ///
    /// The fix is to *ask* rather than declare: `fill_axes` is answered by the walk,
    /// which knows the room being offered on the way **down**, where a parent's own width
    /// is only known on the way back up. Both readings are "full width" in English and only
    /// one of them can be computed in time.
    ///
    /// Each widget is checked **alone and in a column** because alone is where the bug
    /// hides: a percentage against a definite parent is right, so every fixture that gave
    /// a width passed and no golden ever caught this.
    #[test]
    fn widgets_that_fill_the_width_do_it_in_a_column_too() {
        use crate::{build_ui_inspected, Runtime};
        use frus_core::Size;
        type W = Box<dyn Widget<()>>;
        type Case = (&'static str, fn() -> W);
        let cases: Vec<Case> = vec![
            ("ListTile", || Box::new(ListTile::new().title("A row"))),
            ("BottomAppBar", || Box::new(crate::BottomAppBar::new())),
            ("BottomSheet", || Box::new(crate::BottomSheet::new(true))),
            ("Drawer", || Box::new(crate::Drawer::new(true))),
            ("Steps", || Box::new(crate::steps::Steps::new(["a", "b"]))),
            // Milestone 405: shells that want **both** axes. They could not say so while
            // the hook answered with one direction, so they kept the percentage that made
            // them vanish.
            ("NavScaffold", || {
                Box::new(crate::NavScaffold::new(
                    frus_core::SizeClass::Expanded,
                    0,
                    |_| (),
                ))
            }),
            ("TwoPane", || {
                Box::new(crate::TwoPane::new(frus_core::SizeClass::Expanded))
            }),
        ];
        for (name, make) in cases {
            for (what, root) in [
                ("alone", make()),
                (
                    "in a column",
                    Box::new(crate::Flex::column().child_boxed(make())) as W,
                ),
            ] {
                let (_, nodes) = build_ui_inspected(
                    root.as_ref(),
                    Size::new(400.0, 300.0),
                    &Runtime::default(),
                    &Theme::default(),
                );
                let width = nodes
                    .iter()
                    .find(|n| n.name == name)
                    .map(|n| n.rect.width)
                    .unwrap_or_else(|| panic!("{name} is not in the tree"));
                assert_eq!(
                    width, 400.0,
                    "{name} {what}: {width} of the 400 it was offered"
                );
            }
        }
    }
}
