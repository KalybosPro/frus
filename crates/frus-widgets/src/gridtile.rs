//! [`GridTile`] and [`GridTileBar`]: a cell of an image list, and the strip of words
//! **over** it.
//!
//! ```ignore
//! GridTile::new(Image::new(photo))
//!     .footer(GridTileBar::new().title(Text::new("Cliffs")).subtitle(Text::new("Ada")))
//! ```
//!
//! The whole idea is in the word *over*. A caption under a picture is a column, and
//! nobody needs a widget for a column. A grid of pictures with their names on them is a
//! different thing: the tile is the picture's size, the strip is laid on top of it, and
//! the grid's rows stay even because the words took no room. That is why the header and
//! the footer are layers and not siblings (`grid_tile.dart:49`), and why a tile with
//! neither is simply its child — there is nothing to stack.
//!
//! ## The bar reads light on dark, whatever the application is
//!
//! A [`GridTileBar`] stands on a photograph, and a photograph has no brightness the theme
//! knows about. So the strip takes a **dark scheme for its own subtree** and white content
//! over it (`grid_tile_bar.dart:75`), which is the one place in this framework where a
//! widget overrules the application's colours rather than resolving against them. The
//! alternative is a caption that is legible until someone switches to the light theme.
//!
//! It is still overridable, and by more than the reference allows:
//! [`GridTileBarTheme`](crate::GridTileBarTheme) and the builders answer the colours, the
//! type and the two heights, and a caller who wants their own scheme wraps the bar in a
//! [`Themed`](crate::Themed) of their own.

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Rect, Scene, TextOverflow, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::themed::Themed;
use crate::widget::Widget;

/// A bar carrying one line (`grid_tile_bar.dart:79`).
pub const GRID_TILE_BAR_HEIGHT: f32 = 48.0;

/// A bar carrying a title **and** a subtitle.
pub const GRID_TILE_BAR_TWO_LINE_HEIGHT: f32 = 68.0;

/// The room at an end with something in it (`grid_tile_bar.dart:70`).
const TIGHT_PADDING: f32 = 8.0;

/// The room at an end with nothing in it. A glyph carries its own margin; a word does not.
const LOOSE_PADDING: f32 = 16.0;

/// Between the slots.
const GAP: f32 = 8.0;

/// A cell of an image list: a child, with a strip laid over its top or its bottom.
pub struct GridTile<Msg> {
    child: RefCell<Option<Box<dyn Widget<Msg>>>>,
    header: RefCell<Option<Box<dyn Widget<Msg>>>>,
    footer: RefCell<Option<Box<dyn Widget<Msg>>>>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> GridTile<Msg> {
    /// A tile filled by `child` — typically a picture (`grid_tile.dart:41`).
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self::new_boxed(Box::new(child))
    }

    /// The same, for a child that is already boxed.
    pub fn new_boxed(child: Box<dyn Widget<Msg>>) -> Self {
        Self {
            child: RefCell::new(Some(child)),
            header: RefCell::new(None),
            footer: RefCell::new(None),
            built: OnceCell::new(),
        }
    }

    /// A strip across the **top** of the tile, over the child (`grid_tile.dart:31`).
    /// Typically a [`GridTileBar`].
    #[must_use]
    pub fn header(self, header: impl Widget<Msg> + 'static) -> Self {
        *self.header.borrow_mut() = Some(Box::new(header));
        self
    }

    /// A strip across the **bottom** (`grid_tile.dart:36`).
    ///
    /// The reference notes that a tile does not usually have both this and a header, and
    /// that is a note rather than a rule: nothing here stops it, because a picture with a
    /// name at the top and a date at the bottom is a thing people build.
    #[must_use]
    pub fn footer(self, footer: impl Widget<Msg> + 'static) -> Self {
        *self.footer.borrow_mut() = Some(Box::new(footer));
        self
    }

    /// The layers, built once: the child, then whichever strips were given.
    ///
    /// The child is **not** positioned, so it is the child that decides how big the tile
    /// is; the strips are, so they take none of that decision and no room in it. That is
    /// the difference between this and a column, and it is the whole widget.
    fn layers(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| {
            let mut layers: Vec<Box<dyn Widget<Msg>>> = Vec::with_capacity(3);
            if let Some(child) = self.child.borrow_mut().take() {
                layers.push(child);
            }
            if let Some(header) = self.header.borrow_mut().take() {
                layers.push(Box::new(
                    crate::Positioned::new(header).top(0.0).left(0.0).right(0.0),
                ));
            }
            if let Some(footer) = self.footer.borrow_mut().take() {
                layers.push(Box::new(
                    crate::Positioned::new(footer)
                        .bottom(0.0)
                        .left(0.0)
                        .right(0.0),
                ));
            }
            layers
        })
    }

    /// Whether anything is laid over the child. With nothing, the tile **is** the child
    /// and is not a stack at all — which is the reference's early return
    /// (`grid_tile.dart:45`) and matters here for more than a spare node: a stack answers
    /// the walk's structural questions differently, and a tile that claimed to be one
    /// while holding a single child would lay that child out loose in a box of its own.
    fn stacked(&self) -> bool {
        self.layers().len() > 1
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for GridTile<Msg> {
    /// The child's box. A tile is the size of what fills it; the strips are over it.
    fn style(&self) -> Style {
        match self.layers().first() {
            Some(child) => Widget::<Msg>::style(child),
            None => Style::default(),
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        match self.layers().first() {
            Some(child) => Widget::<Msg>::style_themed(child, theme),
            None => Style::default(),
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.layers()
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        self.stacked()
    }

    /// Loose: a layer that was not told where to go keeps its own size. The child is that
    /// layer, and it is the one that sized the tile.
    fn stack_loose(&self) -> bool {
        true
    }
}

/// The strip of words laid over a [`GridTile`]: up to two lines, with a slot at each end.
pub struct GridTileBar<Msg> {
    background_color: Option<Color>,
    foreground_color: Option<Color>,
    title_style: Option<TextStyle>,
    subtitle_style: Option<TextStyle>,
    height: Option<f32>,
    leading: RefCell<Option<Box<dyn Widget<Msg>>>>,
    title: RefCell<Option<Box<dyn Widget<Msg>>>>,
    subtitle: RefCell<Option<Box<dyn Widget<Msg>>>>,
    trailing: RefCell<Option<Box<dyn Widget<Msg>>>>,
    has_title: bool,
    has_subtitle: bool,
    has_leading: bool,
    has_trailing: bool,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> Default for GridTileBar<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> GridTileBar<Msg> {
    /// An empty bar. Every slot is optional, the reference's included — a strip carrying
    /// only a trailing glyph is a legitimate thing to lay over a picture.
    pub fn new() -> Self {
        Self {
            background_color: None,
            foreground_color: None,
            title_style: None,
            subtitle_style: None,
            height: None,
            leading: RefCell::new(None),
            title: RefCell::new(None),
            subtitle: RefCell::new(None),
            trailing: RefCell::new(None),
            has_title: false,
            has_subtitle: false,
            has_leading: false,
            has_trailing: false,
            built: OnceCell::new(),
        }
    }

    /// What is drawn behind the words (`grid_tile_bar.dart:41`). Untold, **nothing** — the
    /// picture shows through, which is the arrangement the widget is for.
    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// The colour of the words and glyphs in the bar. Untold, white
    /// (`grid_tile_bar.dart:83`).
    ///
    /// Not in the reference, which hardcodes it; here because a bar over a pale photograph
    /// wants the opposite and there was nowhere to say so.
    #[must_use]
    pub fn foreground_color(mut self, color: Color) -> Self {
        self.foreground_color = Some(color);
        self
    }

    /// A widget before the title — a glyph, usually (`grid_tile_bar.dart:46`).
    #[must_use]
    pub fn leading(mut self, leading: impl Widget<Msg> + 'static) -> Self {
        *self.leading.borrow_mut() = Some(Box::new(leading));
        self.has_leading = true;
        self
    }

    /// The bar's main line (`grid_tile_bar.dart:51`). Typically a
    /// [`Text`](crate::Text), which takes the bar's type and colour without being told.
    #[must_use]
    pub fn title(mut self, title: impl Widget<Msg> + 'static) -> Self {
        *self.title.borrow_mut() = Some(Box::new(title));
        self.has_title = true;
        self
    }

    /// A second line under the title (`grid_tile_bar.dart:56`). **Its presence is what
    /// makes the bar 68 rather than 48**, so this is a size decision as much as a content
    /// one.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Widget<Msg> + 'static) -> Self {
        *self.subtitle.borrow_mut() = Some(Box::new(subtitle));
        self.has_subtitle = true;
        self
    }

    /// A widget after the title (`grid_tile_bar.dart:61`).
    #[must_use]
    pub fn trailing(mut self, trailing: impl Widget<Msg> + 'static) -> Self {
        *self.trailing.borrow_mut() = Some(Box::new(trailing));
        self.has_trailing = true;
        self
    }

    /// The bar's height, over the two the content would have chosen.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The title's type. Untold, the scale's `title_medium` (`grid_tile_bar.dart:95`).
    #[must_use]
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// The subtitle's. Untold, `body_small` (`grid_tile_bar.dart:101`).
    #[must_use]
    pub fn subtitle_style(mut self, style: TextStyle) -> Self {
        self.subtitle_style = Some(style);
        self
    }

    /// How tall the bar is: two lines make it 68, anything else 48.
    fn resolved_height(&self, theme: &Theme) -> f32 {
        if let Some(told) = self.height {
            return told;
        }
        let t = &theme.widgets.grid_tile_bar;
        if self.has_title && self.has_subtitle {
            t.two_line_height.unwrap_or(GRID_TILE_BAR_TWO_LINE_HEIGHT)
        } else {
            t.height.unwrap_or(GRID_TILE_BAR_HEIGHT)
        }
    }

    /// The room at each end. An end with a glyph in it takes less, because a glyph is
    /// already smaller than the box it is drawn in (`grid_tile_bar.dart:70`).
    fn room(&self) -> (f32, f32) {
        (
            if self.has_leading {
                TIGHT_PADDING
            } else {
                LOOSE_PADDING
            },
            if self.has_trailing {
                TIGHT_PADDING
            } else {
                LOOSE_PADDING
            },
        )
    }

    /// One line of the bar, wrapped in the type it takes.
    fn line(
        slot: Box<dyn Widget<Msg>>,
        own: Option<TextStyle>,
        pick: fn(&Theme) -> TextStyle,
        themed: fn(&Theme) -> Option<TextStyle>,
    ) -> Box<dyn Widget<Msg>> {
        Box::new(Themed::tweak(
            move |theme| {
                // The colour comes from the bar's own tweak, which has already run: this
                // one answers the *type* and leaves the ink where it found it, so the two
                // decisions stay apart and either can be overridden alone.
                let ink = theme.widgets.text.style.color;
                let mut style = own.or_else(|| themed(theme)).unwrap_or_else(|| pick(theme));
                style.color = style.color.or(ink);
                theme.widgets.text.style = style;
            },
            slot,
        ))
    }

    fn assemble(&self) -> Vec<Box<dyn Widget<Msg>>> {
        let mut row = Flex::<Msg>::row().flex(1.0).align(Align::Center).gap(GAP);
        if let Some(leading) = self.leading.borrow_mut().take() {
            row = row.child_boxed(leading);
        }
        let title = self.title.borrow_mut().take();
        let subtitle = self.subtitle.borrow_mut().take();
        let own_title = self.title_style;
        let own_subtitle = self.subtitle_style;
        let lines: Option<Box<dyn Widget<Msg>>> = match (title, subtitle) {
            (Some(title), Some(subtitle)) => Some(Box::new(
                Flex::<Msg>::column()
                    .justify(Justify::Center)
                    .align(Align::Start)
                    .child_boxed(Self::line(
                        title,
                        own_title,
                        |theme| crate::theme::type_scale(Some(theme)).title_medium,
                        |theme| theme.widgets.grid_tile_bar.title_style,
                    ))
                    .child_boxed(Self::line(
                        subtitle,
                        own_subtitle,
                        |theme| crate::theme::type_scale(Some(theme)).body_small,
                        |theme| theme.widgets.grid_tile_bar.subtitle_style,
                    )),
            )),
            // One line on its own takes the **title**'s type whichever slot it came from
            // (`grid_tile_bar.dart:115`): a subtitle with no title above it is not a
            // second line, it is the only line.
            (Some(only), None) | (None, Some(only)) => Some(Self::line(
                only,
                own_title,
                |theme| crate::theme::type_scale(Some(theme)).title_medium,
                |theme| theme.widgets.grid_tile_bar.title_style,
            )),
            (None, None) => None,
        };
        if let Some(lines) = lines {
            row = row.child(crate::Expanded::new(lines));
        }
        if let Some(trailing) = self.trailing.borrow_mut().take() {
            row = row.child_boxed(trailing);
        }

        let own_ink = self.foreground_color;
        vec![Box::new(Themed::tweak(
            move |theme| {
                // **A dark scheme for this subtree, whatever the application is.** The bar
                // stands on a photograph and the theme cannot know how bright one is; the
                // reference installs `ThemeData.dark()` for the same reason
                // (`grid_tile_bar.dart:75`). What is kept is everything that is not a
                // colour — the typographic scale, the reading direction, the spacing, and
                // the application's per-widget defaults — so a bar in a themed application
                // still reads as part of it.
                let mut dark = Theme::dark();
                dark.direction = theme.direction;
                dark.text = theme.text;
                dark.radius = theme.radius;
                dark.spacing = theme.spacing;
                dark.tap_target = theme.tap_target;
                dark.widgets = theme.widgets.clone();
                *theme = dark;

                let ink = own_ink
                    .or(theme.widgets.grid_tile_bar.foreground_color)
                    .unwrap_or(Color::WHITE);
                theme.widgets.text.style.color = Some(ink);
                theme.widgets.icon.color = Some(ink);
                // A caption over a picture does not wrap and does not grow: it cuts.
                // Two lines of a name would cover the face the tile is showing.
                theme.widgets.text.soft_wrap = Some(false);
                theme.widgets.text.overflow = Some(TextOverflow::Ellipsis);
                theme.widgets.text.max_lines = Some(1);
            },
            row,
        )) as Box<dyn Widget<Msg>>]
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for GridTileBar<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let (start, end) = self.room();
        Style {
            height: Dimension::Length(self.resolved_height(theme)),
            width: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Row,
            align: Align::Stretch,
            padding: frus_core::Insets::new(0.0, end, 0.0, start),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble())
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let fill = self
            .background_color
            .or(theme.widgets.grid_tile_bar.background_color);
        // Untold, nothing at all — not a transparent rectangle, which would still be a
        // primitive and would still be a decision. The picture is the background.
        if let Some(fill) = fill.filter(|c| c.a > 0.0) {
            scene.fill_rect(bounds, fill.fade(status.opacity));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Icon, Icons, Runtime, Size, Text};
    use frus_core::Primitive;

    fn picture() -> Container<()> {
        Container::<()>::new()
            .width(120.0)
            .height(120.0)
            .color(Color::rgb(0.2, 0.4, 0.8))
    }

    fn scene_of(tile: impl Widget<()> + 'static) -> Vec<Primitive> {
        let root = crate::flex::Flex::<()>::column()
            .width(200.0)
            .height(200.0)
            .child(tile);
        build_ui(
            &root,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .to_vec()
    }

    /// A tile with nothing over it is its child, and is **not** a stack: a stack lays its
    /// layers out loose in a box of their own, which is not what one child in flow does.
    #[test]
    fn a_tile_with_no_strips_is_its_child() {
        let bare = GridTile::new(picture());
        assert!(!Widget::<()>::stack(&bare));
        assert_eq!(Widget::<()>::children(&bare).len(), 1);
        let with_footer = GridTile::new(picture()).footer(GridTileBar::new().title(Text::new("A")));
        assert!(Widget::<()>::stack(&with_footer));
        assert_eq!(Widget::<()>::children(&with_footer).len(), 2);
    }

    /// The tile is the size of what fills it, whatever is laid over it: the strips are
    /// positioned, so they take no part in the box.
    #[test]
    fn the_strips_do_not_change_the_tile() {
        let bare = Widget::<()>::style(&GridTile::new(picture()));
        let dressed = Widget::<()>::style(
            &GridTile::new(picture())
                .header(GridTileBar::new().title(Text::new("A")))
                .footer(GridTileBar::new().title(Text::new("B"))),
        );
        assert_eq!(bare.width, dressed.width);
        assert_eq!(bare.height, dressed.height);
        assert_eq!(bare.width, Dimension::Length(120.0));
    }

    /// A header sits at the top of the tile and a footer at the bottom — both over the
    /// child, both the full width.
    #[test]
    fn a_header_is_at_the_top_and_a_footer_at_the_bottom() {
        let ink = Color::rgb(1.0, 0.0, 0.0);
        let paper = Color::rgb(0.0, 1.0, 0.0);
        let tile = || {
            GridTile::new(picture())
                .header(
                    GridTileBar::new()
                        .background_color(ink)
                        .title(Text::new("H")),
                )
                .footer(
                    GridTileBar::new()
                        .background_color(paper)
                        .title(Text::new("F")),
                )
        };
        let strips = scene_of(tile());
        let rect_of = |c: Color| {
            strips
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == c => Some(*rect),
                    _ => None,
                })
                .expect("the strip is drawn")
        };
        let header = rect_of(ink);
        let footer = rect_of(paper);
        assert_eq!(header.y, 0.0, "the header is at the top");
        assert_eq!(header.height, GRID_TILE_BAR_HEIGHT);
        assert_eq!(
            footer.y + footer.height,
            120.0,
            "and the footer at the bottom of the tile, not of the page"
        );
        assert_eq!(header.width, 120.0, "both run the tile's full width");
        assert_eq!(footer.width, 120.0);
    }

    /// Two lines make the bar 68; anything else 48. It is the subtitle that decides,
    /// which makes adding one a size decision.
    #[test]
    fn a_subtitle_makes_the_bar_taller() {
        let theme = Theme::default();
        let height = |bar: GridTileBar<()>| match Widget::<()>::style_themed(&bar, &theme).height {
            Dimension::Length(h) => h,
            other => panic!("a bar has a height, not {other:?}"),
        };
        assert_eq!(height(GridTileBar::new().title(Text::new("A"))), 48.0);
        assert_eq!(
            height(
                GridTileBar::new()
                    .title(Text::new("A"))
                    .subtitle(Text::new("B"))
            ),
            68.0
        );
        // A subtitle on its own is the only line, so it is one line tall.
        assert_eq!(height(GridTileBar::new().subtitle(Text::new("B"))), 48.0);
        assert_eq!(height(GridTileBar::new().height(90.0)), 90.0);
    }

    /// An end with a glyph in it takes less room than an end with nothing.
    #[test]
    fn a_slot_at_an_end_tightens_it() {
        let theme = Theme::default();
        let padding = |bar: GridTileBar<()>| Widget::<()>::style_themed(&bar, &theme).padding;
        let plain = padding(GridTileBar::new().title(Text::new("A")));
        assert_eq!((plain.left, plain.right), (LOOSE_PADDING, LOOSE_PADDING));
        let dressed = padding(
            GridTileBar::new()
                .leading(Icon::new(Icons::CHECK))
                .title(Text::new("A")),
        );
        assert_eq!(
            (dressed.left, dressed.right),
            (TIGHT_PADDING, LOOSE_PADDING),
            "only the end with something in it"
        );
    }

    /// The bar reads light on dark whatever the application is, because it stands on a
    /// picture and the theme cannot know how bright one is.
    #[test]
    fn the_bar_is_light_on_the_picture() {
        let colours: Vec<Color> = scene_of(
            GridTile::new(picture()).footer(GridTileBar::new().title(Text::new("Cliffs"))),
        )
        .into_iter()
        .filter_map(|p| match p {
            Primitive::Text { color, .. } => Some(color),
            _ => None,
        })
        .collect();
        assert_eq!(colours, vec![Color::WHITE]);
        // And a caller who wants the other way round has somewhere to say so.
        let told = Color::rgb(0.1, 0.1, 0.1);
        let mine: Vec<Color> = scene_of(
            GridTile::new(picture()).footer(
                GridTileBar::new()
                    .foreground_color(told)
                    .title(Text::new("Cliffs")),
            ),
        )
        .into_iter()
        .filter_map(|p| match p {
            Primitive::Text { color, .. } => Some(color),
            _ => None,
        })
        .collect();
        assert_eq!(mine, vec![told]);
    }

    /// Untold, the bar draws **nothing** behind the words: the picture is the background,
    /// and a transparent rectangle would still be a primitive and still be a decision.
    #[test]
    fn a_bar_with_no_colour_paints_nothing() {
        let backgrounds = |bar: GridTileBar<()>| {
            scene_of(GridTile::new(picture()).footer(bar))
                .into_iter()
                .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.height == 48.0))
                .count()
        };
        assert_eq!(backgrounds(GridTileBar::new().title(Text::new("A"))), 0);
        let told = Color::rgb(0.0, 0.0, 0.0);
        assert_eq!(
            backgrounds(
                GridTileBar::new()
                    .background_color(told)
                    .title(Text::new("A"))
            ),
            1
        );
    }

    /// The theme answers what the caller did not, for the heights as for the colours.
    #[test]
    fn the_theme_answers_what_the_caller_did_not() {
        let mut theme = Theme::default();
        theme.widgets.grid_tile_bar.height = Some(40.0);
        theme.widgets.grid_tile_bar.two_line_height = Some(80.0);
        let one = GridTileBar::<()>::new().title(Text::new("A"));
        assert_eq!(
            Widget::<()>::style_themed(&one, &theme).height,
            Dimension::Length(40.0)
        );
        let two = GridTileBar::<()>::new()
            .title(Text::new("A"))
            .subtitle(Text::new("B"));
        assert_eq!(
            Widget::<()>::style_themed(&two, &theme).height,
            Dimension::Length(80.0)
        );
    }
}
