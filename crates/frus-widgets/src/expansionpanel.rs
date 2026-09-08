//! [`ExpansionPanelList`]: a column of expansion panels drawn as **one surface** that
//! splits as a panel opens.
//!
//! [`ExpansionTile`](crate::ExpansionTile) opens one row on its own, and a column of them
//! is already an accordion as far as the state goes — each asks to change and the
//! application answers, which is what its own documentation says lets a column of them
//! behave as one. What a column of them is not is **one object**: it is a stack of
//! independent rows, each with its own corners, and nothing about it says the shut ones
//! belong together.
//!
//! That is what this adds, and it is the reference's `MergeableMaterial` rather than its
//! `ExpansionPanelList`: adjacent shut panels are **one card**, divided by a hairline,
//! and an open one is lifted out of the run with a gap either side and corners of its
//! own. A settings screen, an FAQ, a checkout with steps.
//!
//! ```ignore
//! ExpansionPanelList::radio(app.open, Msg::OpenPanel)
//!     .panel(ExpansionPanel::new("Delivery", Text::new("Two parcels.")))
//!     .panel(ExpansionPanel::new("Payment", Text::new("Visa ••4242.")))
//! ```
//!
//! # Two modes, two messages
//!
//! The state is the application's, as everywhere here, and the two modes want two
//! different states — so they say two different things rather than one thing twice:
//!
//! - [`new`](ExpansionPanelList::new) is the free one. Any number open, `on_toggle(index,
//!   now_open)`, and the application keeps a set.
//! - [`radio`](ExpansionPanelList::radio) is the exclusive one. It takes an
//!   `Option<usize>` and its message **carries the resulting state**, not the index that
//!   was pressed: opening the third sends `Some(2)` while the second was open, and
//!   pressing the open one sends `None`. So *only one at a time* is a property of what the
//!   widget sends, which an application cannot get wrong by writing its reducer in the
//!   obvious way.
//!
//! # What is not animated
//!
//! The split is **not tweened**. A panel's body appears in one frame, as `ExpansionTile`'s
//! always has, and the gap and the corners follow it in the same frame — so the surface is
//! never wrong, but it does not grow into place the way the reference's does. The tween
//! belongs to `ExpansionTile` first: a list that animated its gaps around a body that
//! snapped would look worse than one that does neither.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use frus_core::{BorderRadius, Color, Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The gap an open panel is given either side of it, and the radius its card takes.
/// The reference's `kMaterialGap` and a card's corner.
const GAP: f32 = 16.0;
const RADIUS: f32 = 12.0;

/// One panel of an [`ExpansionPanelList`]: a header and what it hides.
pub struct ExpansionPanel<Msg> {
    title: Option<String>,
    header: RefCell<Option<Box<dyn Widget<Msg>>>>,
    subtitle: Option<String>,
    body: RefCell<Option<Box<dyn Widget<Msg>>>>,
}

impl<Msg> ExpansionPanel<Msg> {
    /// A panel with a title, hiding `body`.
    pub fn new(title: impl Into<String>, body: impl Widget<Msg> + 'static) -> Self {
        Self {
            title: Some(title.into()),
            header: RefCell::new(None),
            subtitle: None,
            body: RefCell::new(Some(Box::new(body))),
        }
    }

    /// A panel whose header is a **widget** — the reference's `headerBuilder`, for a row
    /// of chips, a rich span, a badge beside a name.
    ///
    /// The chevron and the tap target stay the list's: what this replaces is the line of
    /// text, not the row it sits in.
    pub fn with_header(header: impl Widget<Msg> + 'static, body: impl Widget<Msg> + 'static) -> Self
    where
        Msg: 'static,
    {
        Self {
            title: None,
            header: RefCell::new(Some(Box::new(header))),
            subtitle: None,
            body: RefCell::new(Some(Box::new(body))),
        }
    }

    /// A second line under the title.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }
}

/// How a list decides what is open, and what it says when that should change.
enum Mode<Msg> {
    /// Any number open at once: the indices, and `(index, now_open)`.
    Free(Vec<usize>, Rc<dyn Fn(usize, bool) -> Msg>),
    /// One at a time: which, and the **resulting** choice.
    Radio(Option<usize>, Rc<dyn Fn(Option<usize>) -> Msg>),
}

/// A column of [`ExpansionPanel`]s drawn as **one surface** that splits as a panel opens.
///
/// A column of [`ExpansionTile`](crate::ExpansionTile)s is already an accordion as far as
/// the state goes. What it is not is one *object*: it is a stack of independent rows, and
/// nothing about it says the shut ones belong together. Here, adjacent shut panels are one
/// card divided by a hairline, and an open one is lifted out of the run with a gap either
/// side and corners of its own — the reference's `MergeableMaterial`.
///
/// Two modes, and they say two different things because they want two different states:
/// [`new`](Self::new) is the free one (`on_toggle(index, now_open)`, any number open), and
/// [`radio`](Self::radio) is the exclusive one, whose message carries the **resulting**
/// choice rather than the index pressed — so *one at a time* is a property of what the
/// widget sends rather than a rule the application has to remember.
///
/// **The split is not tweened.** A panel's body appears in one frame, as `ExpansionTile`'s
/// always has, and the gap and the corners follow it in the same frame; what is missing
/// against the reference is the growing, and it belongs to `ExpansionTile` first.
pub struct ExpansionPanelList<Msg> {
    panels: Vec<ExpansionPanel<Msg>>,
    mode: Mode<Msg>,
    gap: Option<f32>,
    radius: Option<f32>,
    background: Option<Color>,
    divider_color: Option<Color>,
    /// Assembled on the way down, under the theme the list actually sits in — as
    /// [`ExpansionTile`](crate::ExpansionTile) assembles its row.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> ExpansionPanelList<Msg> {
    /// A list where **any number** of panels can be open: the ones that are, and
    /// `on_toggle(index, now_open)` when a header is pressed.
    pub fn new(open: &[usize], on_toggle: impl Fn(usize, bool) -> Msg + 'static) -> Self {
        Self::with_mode(Mode::Free(open.to_vec(), Rc::new(on_toggle)))
    }

    /// A list where **one** panel is open at a time: which one, and `on_open(next)` when a
    /// header is pressed — carrying the resulting choice rather than the index pressed,
    /// so an application that stores what it is handed cannot end up with two open.
    ///
    /// Pressing the open one sends `None`, which shuts it. The reference's radio list
    /// cannot be shut at all; this one can, because a panel a reader has opened and read
    /// is a panel they may want out of the way, and there is no other control for it.
    pub fn radio(open: Option<usize>, on_open: impl Fn(Option<usize>) -> Msg + 'static) -> Self {
        Self::with_mode(Mode::Radio(open, Rc::new(on_open)))
    }

    fn with_mode(mode: Mode<Msg>) -> Self {
        Self {
            panels: Vec::new(),
            mode,
            gap: None,
            radius: None,
            background: None,
            divider_color: None,
            built: OnceCell::new(),
        }
    }

    /// Adds a panel, after the ones already there.
    pub fn panel(mut self, panel: ExpansionPanel<Msg>) -> Self {
        self.panels.push(panel);
        self
    }

    /// The room an open panel is given either side of it — what makes it read as lifted
    /// out of the card rather than merely coloured differently. 16 by default.
    #[must_use]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap.max(0.0));
        self
    }

    /// The corner radius of a card — the run's outer corners, and all four of an open
    /// panel's.
    #[must_use]
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius.max(0.0));
        self
    }

    /// The surface the panels are drawn on.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The hairline between two shut panels of the same card.
    #[must_use]
    pub fn divider_color(mut self, color: Color) -> Self {
        self.divider_color = Some(color);
        self
    }

    /// Whether panel `index` is showing.
    fn is_open(&self, index: usize) -> bool {
        match &self.mode {
            Mode::Free(open, _) => open.contains(&index),
            Mode::Radio(open, _) => *open == Some(index),
        }
    }

    /// What pressing panel `index`'s header asks for.
    fn message(&self, index: usize) -> Msg {
        let open = self.is_open(index);
        match &self.mode {
            Mode::Free(_, toggle) => toggle(index, !open),
            Mode::Radio(_, choose) => choose(match open {
                true => None,
                false => Some(index),
            }),
        }
    }

    /// **Where the card splits.** There is a gap between two panels when **either** of
    /// them is open: an open panel is a card of its own, so it parts from both its
    /// neighbours. None at the ends, where there is nothing to part from.
    fn gap_before(&self, index: usize) -> bool {
        index > 0 && (self.is_open(index) || self.is_open(index - 1))
    }

    fn assemble(&self, theme: &Theme) -> Vec<Box<dyn Widget<Msg>>> {
        let t = &theme.widgets.expansion_panel_list;
        let gap = self.gap.or(t.gap).unwrap_or(GAP);
        let radius = self.radius.or(t.radius).unwrap_or(RADIUS);
        let background = self
            .background
            .or(t.background)
            .unwrap_or(theme.scheme.surface_container_low);
        let divider = self
            .divider_color
            .or(t.divider_color)
            .unwrap_or(theme.scheme.outline_variant);

        let mut column = crate::Flex::column();
        for (index, panel) in self.panels.iter().enumerate() {
            let open = self.is_open(index);
            let split_above = self.gap_before(index);
            let split_below = index + 1 == self.panels.len() || self.gap_before(index + 1);
            if split_above {
                column = column.child(crate::Container::<Msg>::new().height(gap));
            } else if index > 0 {
                // Two shut panels of one card: a hairline, which is what says they are
                // separate rows of the same thing rather than one long row.
                column = column.child(crate::Divider::new().color(divider));
            }

            // The **head** of a run rounds its top, the **foot** rounds its bottom, and a
            // panel that is both — an open one, or an only one — is a whole card.
            let corners = BorderRadius {
                top_left: if split_above || index == 0 {
                    radius
                } else {
                    0.0
                },
                top_right: if split_above || index == 0 {
                    radius
                } else {
                    0.0
                },
                bottom_right: if split_below { radius } else { 0.0 },
                bottom_left: if split_below { radius } else { 0.0 },
            };

            let mut tile = crate::ExpansionTile::new(
                panel.title.clone().unwrap_or_default(),
                open,
                self.message(index),
            );
            if let Some(header) = panel.header.borrow_mut().take() {
                tile = tile.title_child_boxed(header);
            }
            if let Some(subtitle) = &panel.subtitle {
                tile = tile.subtitle(subtitle.clone());
            }
            if let Some(body) = panel.body.borrow_mut().take() {
                tile = tile.content(crate::ConstrainedBox::new_boxed(body));
            }
            column = column.child(
                crate::Container::new()
                    .color(background)
                    .radius(corners)
                    .clip()
                    .child(tile),
            );
        }
        vec![Box::new(column) as Box<dyn Widget<Msg>>]
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ExpansionPanelList<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

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

    fn debug_name(&self) -> &'static str {
        "ExpansionPanelList"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Text};
    use frus_core::{Primitive, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle(usize, bool),
        Open(Option<usize>),
    }

    fn three(list: ExpansionPanelList<Msg>) -> ExpansionPanelList<Msg> {
        list.panel(ExpansionPanel::new("One", Text::new("body one")))
            .panel(ExpansionPanel::new("Two", Text::new("body two")))
            .panel(ExpansionPanel::new("Three", Text::new("body three")))
    }

    fn ui(list: &ExpansionPanelList<Msg>) -> crate::Ui<Msg> {
        build_ui(
            list,
            Size::new(320.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    /// Every primitive drawn, layers unwrapped — each panel is a clipped card, so
    /// everything a reader sees is one level down.
    fn flat(ui: &crate::Ui<Msg>) -> Vec<Primitive> {
        fn walk(from: &[Primitive], out: &mut Vec<Primitive>) {
            for p in from {
                match p {
                    Primitive::Layer { primitives, .. } => walk(primitives, out),
                    other => out.push(other.clone()),
                }
            }
        }
        let mut out = Vec::new();
        walk(ui.scene().primitives(), &mut out);
        out
    }

    /// The words drawn, in paint order.
    fn texts(ui: &crate::Ui<Msg>) -> Vec<String> {
        flat(ui)
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Presses panel `index`'s header and returns what it asked for.
    fn press(list: &ExpansionPanelList<Msg>, title: &str) -> Msg {
        let ui = ui(list);
        let row = flat(&ui)
            .iter()
            .find_map(|p| match p {
                Primitive::Text { text, position, .. } if text == title => Some(*position),
                _ => None,
            })
            .expect("the header is drawn");
        ui.hit(frus_core::Point::new(row.x + 4.0, row.y + 4.0))
            .and_then(|id| ui.msg_for(id))
            .expect("the header answers a press")
    }

    /// **Opening the third closes the second**, in the exclusive mode — and it is the
    /// message that says so, not a rule the application has to remember. Driven end to
    /// end: press, apply what came back the obvious way, and look at what is drawn.
    #[test]
    fn in_radio_mode_opening_one_shuts_the_other() {
        let mut open = Some(1);
        let list = three(ExpansionPanelList::radio(open, Msg::Open));
        assert!(texts(&ui(&list)).contains(&"body two".to_string()));

        let asked = press(&list, "Three");
        assert_eq!(
            asked,
            Msg::Open(Some(2)),
            "the resulting choice, not the index"
        );
        let Msg::Open(next) = asked else {
            unreachable!()
        };
        open = next;

        let after = three(ExpansionPanelList::radio(open, Msg::Open));
        let drawn = texts(&ui(&after));
        assert!(drawn.contains(&"body three".to_string()), "{drawn:?}");
        assert!(
            !drawn.contains(&"body two".to_string()),
            "the second shut itself: {drawn:?}"
        );
    }

    /// **And leaves it open in the free mode**, where the two panels have nothing to do
    /// with each other.
    #[test]
    fn in_free_mode_opening_one_leaves_the_other() {
        let mut open = vec![1];
        let list = three(ExpansionPanelList::new(&open, Msg::Toggle));
        let asked = press(&list, "Three");
        assert_eq!(asked, Msg::Toggle(2, true));
        let Msg::Toggle(index, now) = asked else {
            unreachable!()
        };
        if now {
            open.push(index);
        }

        let after = three(ExpansionPanelList::new(&open, Msg::Toggle));
        let drawn = texts(&ui(&after));
        assert!(drawn.contains(&"body two".to_string()), "{drawn:?}");
        assert!(drawn.contains(&"body three".to_string()), "{drawn:?}");
    }

    /// **Pressing the open one shuts it.** The reference's radio list cannot be shut at
    /// all; a panel a reader has read is one they may want out of the way.
    #[test]
    fn pressing_the_open_panel_shuts_it() {
        let list = three(ExpansionPanelList::radio(Some(1), Msg::Open));
        assert_eq!(press(&list, "Two"), Msg::Open(None));
    }

    /// **Shut panels are one card and an open one is lifted out of it.** The gap is the
    /// whole of what makes the set read as one object that splits, rather than as three
    /// rows that happen to be stacked.
    #[test]
    fn the_card_splits_only_where_a_panel_is_open() {
        let shut = three(ExpansionPanelList::new(&[], Msg::Toggle));
        assert!(!shut.gap_before(1) && !shut.gap_before(2), "one card");

        let second = three(ExpansionPanelList::radio(Some(1), Msg::Open));
        assert!(second.gap_before(1), "parted from the one above it");
        assert!(second.gap_before(2), "and from the one below");

        let first = three(ExpansionPanelList::radio(Some(0), Msg::Open));
        assert!(
            first.gap_before(1),
            "the open first panel parts from the second"
        );
        assert!(!first.gap_before(2), "and the two shut ones stay one card");
    }

    /// The gap is real room on the screen, not only a number: the second panel's card
    /// starts lower once the first is open.
    #[test]
    fn the_gap_is_room_on_the_screen() {
        let card_tops = |list: &ExpansionPanelList<Msg>| -> Vec<f32> {
            let ui = ui(list);
            let mut tops: Vec<f32> = flat(&ui)
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, radius, .. } if radius.top_left > 0.0 => Some(rect.y),
                    _ => None,
                })
                .collect();
            tops.dedup();
            tops
        };
        let shut = card_tops(&three(ExpansionPanelList::new(&[], Msg::Toggle)));
        let open = card_tops(&three(ExpansionPanelList::radio(Some(0), Msg::Open)));
        assert!(
            open.len() > shut.len(),
            "an open panel makes a second card: {shut:?} against {open:?}"
        );
    }
}
