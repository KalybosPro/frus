//! [`CheckboxListTile`], [`RadioListTile`] and [`SwitchListTile`]: a row of a list whose
//! **whole width** works one control.
//!
//! Three of the reference's widgets (`checkbox_list_tile.dart`, `radio_list_tile.dart`,
//! `switch_list_tile.dart`), none of which existed here. The parts did: a
//! [`ListTile`](crate::ListTile) that takes two slots and a tap, and the three controls.
//! What was missing is the thing that says *this row and this control are one control* —
//! which is not a convenience. A settings screen where only the 20-pixel box answers and
//! the label beside it does nothing is a screen where most of the taps go nowhere.

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Insets, Rect, Scene, ShapeBorder, TextStyle};
use frus_layout::{FlexDirection, Style};

use crate::collapsible::ControlAffinity;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::{Checkbox, ListTile, Radio, Switch};

/// Everything the three share, which is nearly all of it: a list tile's row, and the one
/// decision about which side the control sits on.
struct Row<Msg> {
    title: Option<String>,
    subtitle: Option<String>,
    /// The widget in the **other** slot — the reference's `secondary`, usually an icon.
    /// Taken on the first walk, as [`crate::ExpansionTile`] takes its slots.
    secondary: RefCell<Option<Box<dyn Widget<Msg>>>>,
    affinity: ControlAffinity,
    dense: bool,
    three_line: bool,
    selected: bool,
    enabled: bool,
    tile_color: Option<Color>,
    selected_tile_color: Option<Color>,
    shape: Option<ShapeBorder>,
    padding: Option<Insets>,
    title_style: Option<TextStyle>,
    subtitle_style: Option<TextStyle>,
}

impl<Msg> Default for Row<Msg> {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            secondary: RefCell::new(None),
            affinity: ControlAffinity::Trailing,
            dense: false,
            three_line: false,
            selected: false,
            enabled: true,
            tile_color: None,
            selected_tile_color: None,
            shape: None,
            padding: None,
            title_style: None,
            subtitle_style: None,
        }
    }
}

impl<Msg: Clone + 'static> Row<Msg> {
    /// Assembles the tile: the control in the slot its affinity names, the `secondary` in
    /// the other, and **the whole row wired to the same message the control is**.
    ///
    /// That last part is the widget. It is what the reference's three do
    /// (`checkbox_list_tile.dart`), and it is why the label is as good a target as the
    /// box.
    fn tile(&self, control: Box<dyn Widget<Msg>>, on_tap: Option<Msg>) -> ListTile<Msg> {
        let mut tile = ListTile::new();
        if let Some(title) = &self.title {
            tile = tile.title(title.clone());
        }
        if let Some(subtitle) = &self.subtitle {
            tile = tile.subtitle(subtitle.clone());
        }
        if self.three_line {
            tile = tile.three_line();
        }
        if self.dense {
            tile = tile.dense();
        }
        if let Some(color) = self.tile_color {
            tile = tile.tile_color(color);
        }
        if let Some(color) = self.selected_tile_color {
            tile = tile.selected_tile_color(color);
        }
        if let Some(shape) = self.shape {
            tile = tile.shape(shape);
        }
        if let Some(padding) = self.padding {
            tile = tile.padding(padding);
        }
        if let Some(style) = self.title_style {
            tile = tile.title_style(style);
        }
        if let Some(style) = self.subtitle_style {
            tile = tile.subtitle_style(style);
        }
        tile = tile.selected(self.selected).enabled(self.enabled);
        if let Some(message) = on_tap {
            tile = tile.on_tap(message);
        }

        let other = self.secondary.borrow_mut().take();
        let (leading, trailing) = match self.affinity {
            ControlAffinity::Leading => (Some(control), other),
            ControlAffinity::Trailing => (other, Some(control)),
        };
        if let Some(widget) = leading {
            tile = tile.leading(crate::ConstrainedBox::new_boxed(widget));
        }
        if let Some(widget) = trailing {
            tile = tile.trailing(crate::ConstrainedBox::new_boxed(widget));
        }
        tile
    }
}

/// The builders the three share, written once against the field they all carry.
macro_rules! row_builders {
    () => {
        /// The row's first line.
        #[must_use]
        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.row.title = Some(title.into());
            self
        }

        /// Its second.
        #[must_use]
        pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
            self.row.subtitle = Some(subtitle.into());
            self
        }

        /// The widget in the **other** slot — an icon, usually. The control takes the
        /// side [`control_affinity`](Self::control_affinity) names and this takes the
        /// side left over.
        #[must_use]
        pub fn secondary(self, widget: impl Widget<Msg> + 'static) -> Self {
            *self.row.secondary.borrow_mut() = Some(Box::new(widget));
            self
        }

        /// **Which side the control sits on.** Trailing by default, as the reference's
        /// is: a column of labels reads down the leading edge, and a column of controls
        /// down the other.
        #[must_use]
        pub fn control_affinity(mut self, affinity: ControlAffinity) -> Self {
            self.row.affinity = affinity;
            self
        }

        /// The tighter row.
        #[must_use]
        pub fn dense(mut self) -> Self {
            self.row.dense = true;
            self
        }

        /// A row tall enough for a two-line subtitle.
        #[must_use]
        pub fn three_line(mut self) -> Self {
            self.row.three_line = true;
            self
        }

        /// Whether this is the row the reader is on, which is a different question from
        /// whether the control is on.
        #[must_use]
        pub fn selected(mut self, selected: bool) -> Self {
            self.row.selected = selected;
            self
        }

        /// Whether the row and its control can be worked at all. Disabled, **neither**
        /// answers: a row that still reported a tap while its control refused one would
        /// be two controls disagreeing.
        #[must_use]
        pub fn enabled(mut self, enabled: bool) -> Self {
            self.row.enabled = enabled;
            self
        }

        /// The row's surface.
        #[must_use]
        pub fn tile_color(mut self, color: Color) -> Self {
            self.row.tile_color = Some(color);
            self
        }

        /// And its surface while it is the row the reader is on.
        #[must_use]
        pub fn selected_tile_color(mut self, color: Color) -> Self {
            self.row.selected_tile_color = Some(color);
            self
        }

        /// What shape the row is.
        #[must_use]
        pub fn shape(mut self, shape: ShapeBorder) -> Self {
            self.row.shape = Some(shape);
            self
        }

        /// The room kept inside it.
        #[must_use]
        pub fn content_padding(mut self, padding: Insets) -> Self {
            self.row.padding = Some(padding);
            self
        }

        /// The title's type.
        #[must_use]
        pub fn title_style(mut self, style: TextStyle) -> Self {
            self.row.title_style = Some(style);
            self
        }

        /// The subtitle's.
        #[must_use]
        pub fn subtitle_style(mut self, style: TextStyle) -> Self {
            self.row.subtitle_style = Some(style);
            self
        }
    };
}

// ============================================================== the checkbox

/// **A row whose whole width ticks a box.**
///
/// ```
/// # use frus_widgets::CheckboxListTile;
/// # #[derive(Clone)] enum Msg { Notify }
/// CheckboxListTile::new(true, Msg::Notify)
///     .title("Notify me")
///     .subtitle("About replies to my posts");
/// ```
pub struct CheckboxListTile<Msg> {
    row: Row<Msg>,
    value: Option<bool>,
    on_changed: Msg,
    active_color: Option<Color>,
    check_color: Option<Color>,
    checkbox_radius: Option<f32>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> CheckboxListTile<Msg> {
    /// A row carrying a box, ticked or not, and the message a tap on **any part of it**
    /// sends.
    pub fn new(value: bool, on_changed: Msg) -> Self {
        Self {
            row: Row::default(),
            value: Some(value),
            on_changed,
            active_color: None,
            check_color: None,
            checkbox_radius: None,
            built: OnceCell::new(),
        }
    }

    /// The same with a box that can be **neither** — a "select all" over rows that are
    /// partly ticked. `None` is a third answer, not a second no.
    pub fn maybe(value: Option<bool>, on_changed: Msg) -> Self {
        Self {
            value,
            ..Self::new(false, on_changed)
        }
    }

    row_builders!();

    /// The colour of a ticked box, over the theme's and `primary`.
    #[must_use]
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = Some(color);
        self
    }

    /// The colour of the tick itself.
    #[must_use]
    pub fn check_color(mut self, color: Color) -> Self {
        self.check_color = Some(color);
        self
    }

    /// The box's corner.
    #[must_use]
    pub fn checkbox_radius(mut self, radius: f32) -> Self {
        self.checkbox_radius = Some(radius);
        self
    }

    fn assemble(&self) -> Box<dyn Widget<Msg>> {
        let mut checkbox = Checkbox::<Msg>::maybe(self.value).enabled(self.row.enabled);
        if let Some(color) = self.active_color {
            checkbox = checkbox.fill_color(color);
        }
        if let Some(color) = self.check_color {
            checkbox = checkbox.check_color(color);
        }
        if let Some(radius) = self.checkbox_radius {
            checkbox = checkbox.radius(radius);
        }
        // The box answers with the row's message, ignoring the value it computed: the
        // caller said what a change means, and a control tile's control is not the one
        // deciding what the next value is.
        if self.row.enabled {
            let message = self.on_changed.clone();
            checkbox = checkbox.on_change(move |_| message.clone());
        }
        let on_tap = self.row.enabled.then(|| self.on_changed.clone());
        Box::new(self.row.tile(Box::new(checkbox), on_tap))
    }
}

// ================================================================= the radio

/// **A row whose whole width chooses one option.**
///
/// It carries `selected` rather than a value and a group value: this framework's
/// [`Radio`] reports being pressed and says nothing about what the answer becomes, so the
/// application already holds the choice.
pub struct RadioListTile<Msg> {
    row: Row<Msg>,
    selected: bool,
    on_changed: Msg,
    active_color: Option<Color>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> RadioListTile<Msg> {
    /// A row carrying a radio, chosen or not, and the message a tap on **any part of it**
    /// sends.
    pub fn new(selected: bool, on_changed: Msg) -> Self {
        Self {
            row: Row::default(),
            selected,
            on_changed,
            active_color: None,
            built: OnceCell::new(),
        }
    }

    row_builders!();

    /// The ring and the dot while it is the chosen one, over the theme's and `primary`.
    #[must_use]
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = Some(color);
        self
    }

    fn assemble(&self) -> Box<dyn Widget<Msg>> {
        let mut radio = Radio::<Msg>::new(self.selected).enabled(self.row.enabled);
        if let Some(color) = self.active_color {
            radio = radio.selected_color(color);
        }
        if self.row.enabled {
            radio = radio.on_select(self.on_changed.clone());
        }
        let on_tap = self.row.enabled.then(|| self.on_changed.clone());
        Box::new(self.row.tile(Box::new(radio), on_tap))
    }
}

// ================================================================ the switch

/// **A row whose whole width flips a switch.** The commonest shape a settings screen has.
pub struct SwitchListTile<Msg> {
    row: Row<Msg>,
    value: bool,
    on_changed: Msg,
    active_color: Option<Color>,
    track_color: Option<Color>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> SwitchListTile<Msg> {
    /// A row carrying a switch, on or off, and the message a tap on **any part of it**
    /// sends.
    pub fn new(value: bool, on_changed: Msg) -> Self {
        Self {
            row: Row::default(),
            value,
            on_changed,
            active_color: None,
            track_color: None,
            built: OnceCell::new(),
        }
    }

    row_builders!();

    /// The thumb while it is on, over the theme's.
    #[must_use]
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = Some(color);
        self
    }

    /// The track while it is on.
    #[must_use]
    pub fn active_track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    fn assemble(&self) -> Box<dyn Widget<Msg>> {
        let mut switch = Switch::<Msg>::new(self.value).enabled(self.row.enabled);
        if let Some(color) = self.active_color {
            switch = switch.thumb_color(color);
        }
        if let Some(color) = self.track_color {
            switch = switch.track_color(color);
        }
        if self.row.enabled {
            let message = self.on_changed.clone();
            switch = switch.on_toggle(move |_| message.clone());
        }
        let on_tap = self.row.enabled.then(|| self.on_changed.clone());
        Box::new(self.row.tile(Box::new(switch), on_tap))
    }
}

/// The three `Widget` implementations, **written out** rather than produced by a macro.
///
/// They are three near-identical copies and a macro would say them once, which is what
/// this module did until the crate's own guard failed on it: milestone 322's
/// `every_control_with_an_enabled_flag_honours_all_four` reads the **source** of every
/// module carrying an `enabled` flag and checks that each of the four hooks consults it.
/// A hook inside a `macro_rules!` body is text the guard cannot parse, so the macro put
/// this module in the one place the net does not reach.
///
/// A safety net a widget can hide from is a net with a hole in it. Three copies is the
/// cheaper of the two prices.
impl<Msg: Clone + 'static> Widget<Msg> for CheckboxListTile<Msg> {
    /// A column of one, which is what the tile is. Not a default `Style`: a wrapper that
    /// forgets to say *column* turns its child sideways, milestone 425's rule.
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    /// Assembled on the first walk, as [`crate::ExpansionTile`]'s row is, so that **the
    /// order the builders were called in cannot change what comes out**.
    fn build_themed(&self, _theme: &Theme) {
        let _ = self.built.set(vec![self.assemble()]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The row answers, not this. Wiring a click here as well would send the message
    /// twice for one tap, and the row already refuses while disabled.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for RadioListTile<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    fn build_themed(&self, _theme: &Theme) {
        let _ = self.built.set(vec![self.assemble()]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The row answers, not this.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for SwitchListTile<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    fn build_themed(&self, _theme: &Theme) {
        let _ = self.built.set(vec![self.assemble()]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The row answers, not this.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Icon, Icons, Point as P, Runtime, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle,
        Other,
    }

    fn frame(widget: &dyn Widget<Msg>) -> crate::Ui<Msg> {
        build_ui(
            widget,
            Size::new(320.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    /// **The whole row is the control.** This is the widget: a settings screen where only
    /// the 20-pixel box answers, and the label beside it does nothing, is a screen where
    /// most of the taps go nowhere. The reference's three do exactly this
    /// (`checkbox_list_tile.dart`), and the parts to do it with have been here all along.
    #[test]
    fn a_tap_anywhere_on_the_row_works_the_control() {
        for tile in [
            Box::new(CheckboxListTile::new(false, Msg::Toggle).title("Notify me"))
                as Box<dyn Widget<Msg>>,
            Box::new(RadioListTile::new(false, Msg::Toggle).title("Notify me")),
            Box::new(SwitchListTile::new(false, Msg::Toggle).title("Notify me")),
        ] {
            let ui = frame(tile.as_ref());
            // Far from the control, on the words: the middle of the row's leading half.
            let hit = ui.hit(P::new(80.0, 24.0)).expect("the row is hittable");
            assert_eq!(
                ui.msg_for(hit),
                Some(Msg::Toggle),
                "the label answers, not only the box"
            );
        }
    }

    /// The control **and** the row send the same message, so a tap on either does the
    /// same thing. A row wired to one message and a control wired to another would be two
    /// controls in one row disagreeing about what they are.
    #[test]
    fn the_control_and_the_row_say_the_same_thing() {
        let tile = CheckboxListTile::new(false, Msg::Toggle).title("Notify me");
        let ui = frame(&tile);
        // The trailing edge, where the box is.
        let box_hit = ui.hit(P::new(300.0, 24.0)).expect("the box is hittable");
        assert_eq!(ui.msg_for(box_hit), Some(Msg::Toggle));
    }

    /// **Disabled, neither answers.** A row that still reported a tap while its control
    /// refused one would be two controls disagreeing, and the one a reader can see is the
    /// one that looks unavailable.
    #[test]
    fn a_disabled_row_answers_nowhere() {
        let tile = SwitchListTile::new(true, Msg::Toggle)
            .title("Notify me")
            .enabled(false);
        let ui = frame(&tile);
        for x in [80.0, 300.0] {
            if let Some(hit) = ui.hit(P::new(x, 24.0)) {
                assert_ne!(ui.msg_for(hit), Some(Msg::Toggle), "at x = {x}");
            }
        }
    }

    /// **Which side the control sits on**, and the other slot going to `secondary`. The
    /// reference's `controlAffinity` (`checkbox_list_tile.dart`), with trailing the
    /// default: a column of labels reads down the leading edge and a column of controls
    /// down the other.
    #[test]
    fn the_control_takes_the_side_it_was_given_and_the_icon_the_other() {
        let ends = |tile: &dyn Widget<Msg>| {
            let ui = frame(tile);
            // Where the row's own painted marks are: the leftmost and rightmost thing
            // drawn inside it that is not the row's surface.
            let mut xs: Vec<f32> = Vec::new();
            fn walk(primitives: &[frus_core::Primitive], xs: &mut Vec<f32>) {
                for p in primitives {
                    match p {
                        frus_core::Primitive::Rect { rect, .. } if rect.width < 100.0 => {
                            xs.push(rect.x)
                        }
                        frus_core::Primitive::Path { .. } => {}
                        frus_core::Primitive::Layer { primitives, .. } => walk(primitives, xs),
                        _ => {}
                    }
                }
            }
            walk(ui.scene().primitives(), &mut xs);
            xs
        };

        let trailing = CheckboxListTile::new(true, Msg::Toggle)
            .title("Notify me")
            .secondary(Icon::new(Icons::Add));
        let leading = CheckboxListTile::new(true, Msg::Toggle)
            .title("Notify me")
            .secondary(Icon::new(Icons::Add))
            .control_affinity(ControlAffinity::Leading);

        let (right, left) = (ends(&trailing), ends(&leading));
        assert!(!right.is_empty() && !left.is_empty(), "both drew a box");
        let far_right = right.iter().cloned().fold(f32::MIN, f32::max);
        let far_left = left.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            far_right > far_left,
            "a trailing box sits further along the row than a leading one: \
             {far_right} vs {far_left}"
        );
    }

    /// **The order the builders were written in cannot change what comes out.** The row
    /// is assembled on the first walk, as [`crate::ExpansionTile`]'s is — the trap
    /// milestone 458 found in `BottomSheet`, where the panel is built by one particular
    /// method and anything said after it is dropped.
    #[test]
    fn the_builder_order_does_not_change_what_comes_out() {
        let first = SwitchListTile::new(true, Msg::Toggle)
            .dense()
            .title("Notify me")
            .tile_color(Color::rgb(0.2, 0.3, 0.4));
        let last = SwitchListTile::new(true, Msg::Toggle)
            .tile_color(Color::rgb(0.2, 0.3, 0.4))
            .title("Notify me")
            .dense();
        let paint = |tile: &SwitchListTile<Msg>| {
            let ui = frame(tile);
            format!("{:?}", ui.scene().primitives())
        };
        assert_eq!(paint(&first), paint(&last));
    }

    /// A row that is the one the reader is on says so, which is a different question from
    /// whether the control is on. Both travel to the tile.
    #[test]
    fn being_the_chosen_row_is_not_being_on() {
        let tile = RadioListTile::new(false, Msg::Toggle)
            .title("Every day")
            .selected(true)
            .selected_tile_color(Color::rgb(0.6, 0.4, 0.2));
        let ui = frame(&tile);
        fn find(primitives: &[frus_core::Primitive], want: Color) -> bool {
            primitives.iter().any(|p| match p {
                frus_core::Primitive::Rect { color, .. } => *color == want,
                frus_core::Primitive::Layer { primitives, .. } => find(primitives, want),
                _ => false,
            })
        }
        assert!(
            find(ui.scene().primitives(), Color::rgb(0.6, 0.4, 0.2)),
            "the selected surface is painted"
        );
        assert_ne!(Msg::Other, Msg::Toggle);
    }
}
