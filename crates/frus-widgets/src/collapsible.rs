//! [`ExpansionTile`]: a tile that opens to show more.
//!
//! It is a [`crate::ListTile`] with a chevron and a body underneath — which is what the
//! reference's is, and the reason it is built out of one here rather than painting a
//! header of its own. Everything a tile can carry, this carries: something in front, a
//! subtitle, a trailing widget of your own in place of the chevron, the tile's
//! measurements and its colours.

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Insets, Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::{Container, Icon, ListTile};

/// Which side the chevron sits on — the reference's `ListTileControlAffinity`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ControlAffinity {
    /// In front of the title, where a checkbox would go. What a file tree wants: the
    /// chevrons line up down the left and the indentation reads as a hierarchy.
    Leading,
    /// After it, at the end of the row. The reference's default, and this one's.
    #[default]
    Trailing,
}

/// The chevron's side, matching the reference's `expand_more`.
const CHEVRON: f32 = 24.0;

/// A tile that opens to show more.
///
/// **Controlled**, like everything else here: it is told whether it is open and emits a
/// message asking to change, rather than keeping the answer to itself. That is what lets
/// a column of them behave as an accordion — the application decides only one is open —
/// without the widget knowing anything about its siblings.
///
/// ```
/// # use frus_widgets::{ExpansionTile, Text};
/// # #[derive(Clone)] enum Msg { Toggle }
/// ExpansionTile::new("Delivery", true, Msg::Toggle)
///     .subtitle("Arrives Thursday")
///     .content(Text::new("Two parcels, one signature."));
/// ```
pub struct ExpansionTile<Msg> {
    title: String,
    open: bool,
    on_toggle: Msg,
    subtitle: Option<String>,
    leading: RefCell<Option<Box<dyn Widget<Msg>>>>,
    trailing: RefCell<Option<Box<dyn Widget<Msg>>>>,
    content: RefCell<Option<Box<dyn Widget<Msg>>>>,
    show_trailing_icon: bool,
    affinity: ControlAffinity,
    dense: bool,
    tile_padding: Option<Insets>,
    children_padding: Insets,
    background: Option<Color>,
    collapsed_background: Option<Color>,
    text_color: Option<Color>,
    collapsed_text_color: Option<Color>,
    icon_color: Option<Color>,
    collapsed_icon_color: Option<Color>,
    /// `[tile]`, or `[tile, body]` when it is open and has one. Assembled on the first
    /// walk, as [`crate::ListTile`] assembles its row, so that the order the builders
    /// were called in cannot change what comes out.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> ExpansionTile<Msg> {
    /// A tile with a title, whether it is open, and the message that asks to change that.
    pub fn new(title: impl Into<String>, open: bool, on_toggle: Msg) -> Self {
        Self {
            title: title.into(),
            open,
            on_toggle,
            subtitle: None,
            leading: RefCell::new(None),
            trailing: RefCell::new(None),
            content: RefCell::new(None),
            show_trailing_icon: true,
            affinity: ControlAffinity::default(),
            dense: false,
            tile_padding: None,
            children_padding: Insets::new(0.0, 16.0, 16.0, 16.0),
            background: None,
            collapsed_background: None,
            text_color: None,
            collapsed_text_color: None,
            icon_color: None,
            collapsed_icon_color: None,
            built: OnceCell::new(),
        }
    }

    /// The body, shown only while the tile is open.
    ///
    /// It is not built when the tile is shut — the reference's `maintainState` is what
    /// asks for the other behaviour, and there is nothing here that a closed subtree
    /// would be keeping alive.
    pub fn content(self, content: impl Widget<Msg> + 'static) -> Self {
        *self.content.borrow_mut() = Some(Box::new(content));
        self
    }

    /// A second line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// What goes in front of the title: an icon, an avatar, a checkbox.
    ///
    /// With [`ControlAffinity::Leading`] the chevron takes this place, and whatever is
    /// given here is dropped rather than fought over — the reference does the same, and
    /// two things claiming one slot is a bug the layout cannot report.
    pub fn leading(self, leading: impl Widget<Msg> + 'static) -> Self {
        *self.leading.borrow_mut() = Some(Box::new(leading));
        self
    }

    /// A widget of your own at the end of the row, **in place of** the chevron: a badge,
    /// a count, a switch. It replaces the chevron rather than joining it.
    pub fn trailing(self, trailing: impl Widget<Msg> + 'static) -> Self {
        *self.trailing.borrow_mut() = Some(Box::new(trailing));
        self
    }

    /// Hides the chevron altogether, for a tile whose open state is read from something
    /// else on the row.
    pub fn show_trailing_icon(mut self, show: bool) -> Self {
        self.show_trailing_icon = show;
        self
    }

    /// Which side the chevron sits on; the end of the row by default.
    pub fn control_affinity(mut self, affinity: ControlAffinity) -> Self {
        self.affinity = affinity;
        self
    }

    /// A **dense** tile: the same shape, less room around it.
    pub fn dense(mut self) -> Self {
        self.dense = true;
        self
    }

    /// The room inside the tile row itself.
    pub fn tile_padding(mut self, padding: Insets) -> Self {
        self.tile_padding = Some(padding);
        self
    }

    /// The room around the body, inside the tile's width.
    pub fn children_padding(mut self, padding: Insets) -> Self {
        self.children_padding = padding;
        self
    }

    /// The row's background **while open**.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The row's background while shut.
    pub fn collapsed_background_color(mut self, color: Color) -> Self {
        self.collapsed_background = Some(color);
        self
    }

    /// The title's colour while open.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// The title's colour while shut.
    pub fn collapsed_text_color(mut self, color: Color) -> Self {
        self.collapsed_text_color = Some(color);
        self
    }

    /// The chevron's colour while open.
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// The chevron's colour while shut.
    pub fn collapsed_icon_color(mut self, color: Color) -> Self {
        self.collapsed_icon_color = Some(color);
        self
    }

    /// The chevron, pointing down when the tile is open and along the reading direction
    /// when it is shut — the same pair the section header drew before this was a tile.
    fn chevron(&self, theme: &Theme) -> Option<Box<dyn Widget<Msg>>> {
        if !self.show_trailing_icon {
            return None;
        }
        let color = if self.open {
            self.icon_color
        } else {
            self.collapsed_icon_color
        }
        .unwrap_or(theme.scheme.on_surface_variant);
        let name = if self.open {
            Icons::ChevronDown
        } else {
            Icons::ChevronRight
        };
        Some(Box::new(Icon::new(name).size(CHEVRON).color(color)) as Box<dyn Widget<Msg>>)
    }

    /// Assembles the row and, when it is open, the body under it.
    fn assemble(&self, theme: &Theme) -> Vec<Box<dyn Widget<Msg>>> {
        let chevron = self.chevron(theme);
        let given_leading = self.leading.borrow_mut().take();
        let given_trailing = self.trailing.borrow_mut().take();

        // The chevron claims one slot, and whichever slot it claims wins: a widget given
        // for the same side is dropped rather than stacked, which is what the reference
        // does and the only answer a row of fixed slots can give.
        let (leading, trailing) = match self.affinity {
            ControlAffinity::Leading => (chevron.or(given_leading), given_trailing),
            ControlAffinity::Trailing => (given_leading, given_trailing.or(chevron)),
        };

        let mut tile = ListTile::new()
            .title(self.title.clone())
            .on_tap(self.on_toggle.clone());
        if let Some(subtitle) = &self.subtitle {
            tile = tile.subtitle(subtitle.clone());
        }
        if let Some(leading) = leading {
            tile = tile.leading(crate::ConstrainedBox::new_boxed(leading));
        }
        if let Some(trailing) = trailing {
            tile = tile.trailing(crate::ConstrainedBox::new_boxed(trailing));
        }
        if self.dense {
            tile = tile.dense();
        }
        if let Some(padding) = self.tile_padding {
            tile = tile.padding(padding);
        }
        if let Some(color) = if self.open {
            self.background
        } else {
            self.collapsed_background
        } {
            tile = tile.tile_color(color);
        }
        if let Some(color) = if self.open {
            self.text_color
        } else {
            self.collapsed_text_color
        } {
            let mut style = theme.text.body_large;
            style.color = Some(color);
            tile = tile.title_style(style);
        }

        let mut out: Vec<Box<dyn Widget<Msg>>> = vec![Box::new(tile)];
        if self.open {
            if let Some(content) = self.content.borrow_mut().take() {
                let pad = self.children_padding;
                out.push(Box::new(
                    Container::new()
                        .padding_each(pad.top, pad.right, pad.bottom, pad.left)
                        .child(crate::ConstrainedBox::new_boxed(content)),
                ));
            }
        }
        out
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ExpansionTile<Msg> {
    fn style(&self) -> Style {
        Style {
            // **No** width of its own: a cross axis left at `Auto` is stretched by the
            // column above it, and an explicit percentage would overrule that stretch
            // and resolve against a parent that is not definite yet — which comes out
            // as a hug, with the chevron against the last letter of the title.
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// Assembled on the way down, under the theme the tile actually sits in: it reaches
    /// for text styles and colours, and a tile built at construction time inside a
    /// [`crate::Themed`] subtree would come out in the wrong palette.
    fn build_themed(&self, theme: &Theme) {
        self.built.get_or_init(|| self.assemble(theme));
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get().map(|v| &v[..]).unwrap_or(&[])
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size, Text};
    use frus_core::{Color, Primitive};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle,
        Other,
    }

    /// A tile as the walk sees it: assembled under a theme, children resolved.
    fn assembled(tile: &ExpansionTile<Msg>) -> &[Box<dyn Widget<Msg>>] {
        Widget::build_themed(tile, &Theme::default());
        Widget::children(tile)
    }

    /// Shut, only the row is built — the body is not a subtree kept alive out of sight.
    #[test]
    fn a_shut_tile_builds_only_its_row() {
        let tile = ExpansionTile::new("Title", false, Msg::Toggle).content(Text::new("hidden"));
        assert_eq!(assembled(&tile).len(), 1);
    }

    /// Open, the body follows the row and the row asks to close.
    #[test]
    fn an_open_tile_carries_its_body_and_the_row_toggles() {
        let tile = ExpansionTile::new("Title", true, Msg::Toggle).content(Text::new("visible"));
        let children = assembled(&tile);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].on_click(), Some(Msg::Toggle));
    }

    /// It is a `ListTile`, so it is the reference's height rather than a header's guess.
    #[test]
    fn the_row_is_a_list_tile() {
        let one = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle);
        Widget::build_themed(&one, &Theme::default());
        let row = &Widget::children(&one)[0];
        assert_eq!(
            row.style_themed(&Theme::default()).min_height,
            frus_layout::Dimension::Length(crate::LIST_TILE_HEIGHTS[0]),
            "one line"
        );

        let two = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle).subtitle("Second line");
        Widget::build_themed(&two, &Theme::default());
        let row = &Widget::children(&two)[0];
        assert_eq!(
            row.style_themed(&Theme::default()).min_height,
            frus_layout::Dimension::Length(crate::LIST_TILE_HEIGHTS[1]),
            "two lines"
        );
    }

    /// The number of slots the row filled, leading first.
    fn slots(tile: &ExpansionTile<Msg>) -> usize {
        let row = &assembled(tile)[0];
        // ListTile assembles a single row; its children are the filled slots.
        row.build_themed(&Theme::default());
        row.children()
            .first()
            .map(|inner| inner.children().len())
            .unwrap_or(0)
    }

    /// A chevron on its own fills the trailing slot; a leading widget adds a second.
    #[test]
    fn the_chevron_takes_the_trailing_slot() {
        let bare = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle);
        let with_leading =
            ExpansionTile::<Msg>::new("Title", false, Msg::Toggle).leading(Text::new("*"));
        assert_eq!(slots(&with_leading), slots(&bare) + 1);
    }

    /// A trailing widget **replaces** the chevron: a row whose end carries a switch is
    /// not also carrying an arrow.
    #[test]
    fn a_trailing_widget_replaces_the_chevron() {
        let bare = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle);
        let replaced =
            ExpansionTile::<Msg>::new("Title", false, Msg::Toggle).trailing(Text::new("3"));
        assert_eq!(slots(&replaced), slots(&bare), "one end slot either way");
    }

    /// Asked for nothing at the end, there is one slot fewer than with a chevron.
    #[test]
    fn the_chevron_can_be_hidden() {
        let bare = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle);
        let none = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle).show_trailing_icon(false);
        assert_eq!(slots(&none) + 1, slots(&bare));
    }

    /// Leading affinity moves the chevron to the front, and the widget that wanted that
    /// slot is dropped rather than stacked — two widgets in one slot is a bug a row of
    /// fixed slots cannot report.
    #[test]
    fn leading_affinity_gives_the_chevron_the_front_slot() {
        let bare = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle)
            .control_affinity(ControlAffinity::Leading);
        let contested = ExpansionTile::<Msg>::new("Title", false, Msg::Toggle)
            .control_affinity(ControlAffinity::Leading)
            .leading(Text::new("*"));
        assert_eq!(
            slots(&contested),
            slots(&bare),
            "the chevron keeps the slot"
        );
    }

    /// The colours are asked for, not assumed — and the open and shut states differ.
    #[test]
    fn the_row_takes_the_colour_it_was_given() {
        const OPEN: Color = Color::rgb(0.0, 0.6, 0.3);
        const SHUT: Color = Color::rgb(0.9, 0.2, 0.1);

        let painted = |open: bool| {
            let tile = ExpansionTile::<Msg>::new("Title", open, Msg::Toggle)
                .background_color(OPEN)
                .collapsed_background_color(SHUT);
            let ui = build_ui(
                &tile,
                Size::new(300.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene().primitives().iter().any(|p| match p {
                Primitive::Rect { color, .. } => *color == if open { OPEN } else { SHUT },
                _ => false,
            })
        };
        assert!(painted(true), "the open background is painted");
        assert!(
            painted(false),
            "and the shut one, which is a different colour"
        );
    }

    /// The message is the caller's, not a shape the widget invented.
    #[test]
    fn the_row_emits_the_message_it_was_handed() {
        let tile = ExpansionTile::new("Title", false, Msg::Other);
        assert_eq!(assembled(&tile)[0].on_click(), Some(Msg::Other));
    }
}
