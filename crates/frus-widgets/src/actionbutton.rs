//! [`BackButton`], [`CloseButton`], [`DrawerButton`] and [`EndDrawerButton`]: the four
//! icon buttons whose meaning is fixed and whose **name is not**.
//!
//! The reference calls them action buttons (`action_buttons.dart`) and they are all one
//! idea: an [`IconButton`](crate::IconButton) that already knows its glyph and takes its
//! tooltip from the framework's own words. Nothing here had them — the one back button in
//! the crate was `IconButton::glyph("←")`, a **character**, named by hand at its one call
//! site.

use frus_core::{Color, Rect, Scene};
use frus_layout::Style;

use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::{IconButton, Tooltip};

/// Which of the four this is. The type carries the glyph and the words, so a caller
/// choosing a back button cannot end up with a close button's tooltip.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Kind {
    Back,
    Close,
    Drawer,
    EndDrawer,
}

impl Kind {
    /// The glyph, with the theme's override ahead of the framework's own.
    fn icon(self, theme: &Theme) -> Icons {
        let t = &theme.widgets.action_icons;
        match self {
            Kind::Back => t.back.unwrap_or(default_back_icon()),
            Kind::Close => t.close.unwrap_or(Icons::Close),
            Kind::Drawer => t.drawer.unwrap_or(Icons::Menu),
            Kind::EndDrawer => t.end_drawer.unwrap_or(Icons::Menu),
        }
    }

    /// What the framework calls it, in the reader's language.
    fn label(self) -> String {
        let words = crate::localizations::of();
        match self {
            Kind::Back => words.back_button_label().to_owned(),
            Kind::Close => words.close_button_label().to_owned(),
            // Both drawers, one word. That is the reference's own arrangement
            // (`action_buttons.dart:331` and `:362` return the same string): a reader
            // being told *which edge* the panel comes in from would be told about the
            // layout rather than about the action.
            Kind::Drawer | Kind::EndDrawer => words.open_drawer_label().to_owned(),
        }
    }
}

/// **The back arrow the platform draws.** A chevron where the platform's own back control
/// is a chevron, an arrow everywhere else (`action_buttons.dart:132`).
///
/// Resolved at compile time, like [`ScrollPhysics::platform_default`](crate::ScrollPhysics::platform_default):
/// a binary runs on one platform and the branch it does not take is not a decision it
/// needs to carry.
const fn default_back_icon() -> Icons {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        Icons::ChevronLeft
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        Icons::ArrowLeft
    }
}

/// The body the four share: an icon button, wrapped in a tooltip carrying the same words
/// its semantics carry.
struct Action<Msg> {
    kind: Kind,
    on_press: Option<Msg>,
    color: Option<Color>,
    icon_size: Option<f32>,
    enabled: bool,
    built: std::cell::OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> Action<Msg> {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            on_press: None,
            color: None,
            icon_size: None,
            enabled: true,
            built: std::cell::OnceCell::new(),
        }
    }

    fn assemble(&self, theme: &Theme) -> Box<dyn Widget<Msg>> {
        let label = self.kind.label();
        let mut button = IconButton::new(self.kind.icon(theme))
            .label(label.clone())
            .enabled(self.enabled);
        if let Some(color) = self.color {
            button = button.icon_color(color);
        }
        if let Some(size) = self.icon_size {
            button = button.icon_size(size);
        }
        if let Some(message) = self.on_press.clone() {
            button = button.on_press(message);
        }
        // The words twice, on purpose: once where a reader hears them and once where a
        // pointer sees them. The reference does the same (`action_buttons.dart:54`), and
        // the alternative is a control that is named for one of its two audiences.
        Box::new(Tooltip::new(label).child(button))
    }
}

// The four public faces, **written out** rather than produced by a macro.
//
// They differ only in which `Kind` they carry, and a macro would say them once. That is
// what milestone 465 tried in the control tiles, and the crate's own guard failed on it:
// `every_control_with_an_enabled_flag_honours_all_four` reads the **source** of every
// module carrying an `enabled` flag and checks that each of the four hooks consults it,
// and a hook inside a `macro_rules!` body is text it cannot parse.
//
// A safety net a widget can hide from is a net with a hole in it. So these are typed
// out, and the repetition is the price of staying inside the net.

/// **The back arrow**, named in the reader's language.
///
/// Its glyph follows the platform: a chevron where the platform's own back control is a
/// chevron, an arrow everywhere else (`action_buttons.dart:132`).
pub struct BackButton<Msg>(Action<Msg>);

impl<Msg: Clone + 'static> BackButton<Msg> {
    /// A back button, inert until it is given a message.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Action::new(Kind::Back))
    }

    /// What it does. Without a message it is **inert**: it draws, it is named, and it
    /// answers nothing. Disabled is the other thing, and says so.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.0.on_press = Some(message);
        self
    }

    /// The glyph's colour.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.0.color = Some(color);
        self
    }

    /// The glyph's size.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.0.icon_size = Some(size);
        self
    }

    /// Whether it can be pressed.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for BackButton<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.0.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    /// Assembled on the first walk, under the theme it will be drawn in: the glyph is a
    /// theme's to override, and a button built by a builder never sees one.
    fn build_themed(&self, theme: &Theme) {
        let _ = self.0.built.set(vec![self.0.assemble(theme)]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The button inside answers, not this — including the `enabled` flag, which it was
    /// built with. A second click here would send the message twice for one press.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// **The cross that dismisses something**, named in the reader's language.
pub struct CloseButton<Msg>(Action<Msg>);

impl<Msg: Clone + 'static> CloseButton<Msg> {
    /// A close button, inert until it is given a message.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Action::new(Kind::Close))
    }

    /// What it does. Without a message it is **inert**: it draws, it is named, and it
    /// answers nothing. Disabled is the other thing, and says so.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.0.on_press = Some(message);
        self
    }

    /// The glyph's colour.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.0.color = Some(color);
        self
    }

    /// The glyph's size.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.0.icon_size = Some(size);
        self
    }

    /// Whether it can be pressed.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for CloseButton<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.0.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    /// Assembled on the first walk, under the theme it will be drawn in: the glyph is a
    /// theme's to override, and a button built by a builder never sees one.
    fn build_themed(&self, theme: &Theme) {
        let _ = self.0.built.set(vec![self.0.assemble(theme)]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The button inside answers, not this — including the `enabled` flag, which it was
    /// built with. A second click here would send the message twice for one press.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// **The bars that open the panel on the leading edge**, named in the reader's
/// language.
pub struct DrawerButton<Msg>(Action<Msg>);

impl<Msg: Clone + 'static> DrawerButton<Msg> {
    /// A drawer button, inert until it is given a message.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Action::new(Kind::Drawer))
    }

    /// What it does. Without a message it is **inert**: it draws, it is named, and it
    /// answers nothing. Disabled is the other thing, and says so.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.0.on_press = Some(message);
        self
    }

    /// The glyph's colour.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.0.color = Some(color);
        self
    }

    /// The glyph's size.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.0.icon_size = Some(size);
        self
    }

    /// Whether it can be pressed.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for DrawerButton<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.0.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    /// Assembled on the first walk, under the theme it will be drawn in: the glyph is a
    /// theme's to override, and a button built by a builder never sees one.
    fn build_themed(&self, theme: &Theme) {
        let _ = self.0.built.set(vec![self.0.assemble(theme)]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The button inside answers, not this — including the `enabled` flag, which it was
    /// built with. A second click here would send the message twice for one press.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// **The bars that open the panel on the trailing edge.**
///
/// It takes the *same* words as [`DrawerButton`], which is the reference's own
/// arrangement: a reader told which edge a panel comes in from is being told about the
/// layout rather than about the action.
pub struct EndDrawerButton<Msg>(Action<Msg>);

impl<Msg: Clone + 'static> EndDrawerButton<Msg> {
    /// An end-drawer button, inert until it is given a message.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Action::new(Kind::EndDrawer))
    }

    /// What it does. Without a message it is **inert**: it draws, it is named, and it
    /// answers nothing. Disabled is the other thing, and says so.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.0.on_press = Some(message);
        self
    }

    /// The glyph's colour.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.0.color = Some(color);
        self
    }

    /// The glyph's size.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.0.icon_size = Some(size);
        self
    }

    /// Whether it can be pressed.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for EndDrawerButton<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.0.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    /// Assembled on the first walk, under the theme it will be drawn in: the glyph is a
    /// theme's to override, and a button built by a builder never sees one.
    fn build_themed(&self, theme: &Theme) {
        let _ = self.0.built.set(vec![self.0.assemble(theme)]);
    }

    fn paint(&self, _b: Rect, _s: Status, _t: &Theme, _scene: &mut Scene) {}

    /// The button inside answers, not this — including the `enabled` flag, which it was
    /// built with. A second click here would send the message twice for one press.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use std::rc::Rc;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pop,
    }

    /// A table in another language, to prove the words travel.
    struct French;
    impl crate::localizations::Localizations for French {
        fn back_button_label(&self) -> &str {
            "Retour"
        }
        fn open_drawer_label(&self) -> &str {
            "Ouvrir le menu"
        }
    }

    fn labels<W: Widget<Msg>>(widget: &W) -> Vec<String> {
        let ui = build_ui(
            widget,
            Size::new(200.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.semantics()
            .iter()
            .filter_map(|(_, _, node)| node.label.clone())
            .collect()
    }

    /// **The one back button in the crate was a character.** `NavigationBar::on_back`
    /// built `IconButton::glyph("←")` — a codepoint, at the mercy of whatever font is
    /// loaded, with no control over its weight and none over its size relative to the
    /// other glyphs beside it. These four carry a **drawn** icon.
    #[test]
    fn a_back_button_carries_a_drawn_glyph_and_not_a_character() {
        let button = BackButton::<Msg>::new().on_press(Msg::Pop);
        let ui = build_ui(
            &button,
            Size::new(200.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let paths = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| matches!(p, frus_core::Primitive::Path { .. }))
            .count();
        assert!(paths >= 1, "a path, not a text run");
    }

    /// **They are named, and named in the reader's language.** An icon has no words of
    /// its own, so a back arrow with nothing said about it is a shape a screen reader
    /// cannot announce — and a back arrow announced as "Back" to a French reader is a
    /// shape announced wrongly.
    ///
    /// The words go in **twice**, as the reference's do: once where a reader hears them
    /// and once where a pointer sees them. A control named for only one of its two
    /// audiences is named for neither.
    #[test]
    fn the_four_are_named_in_the_reader_s_language() {
        assert!(labels(&BackButton::<Msg>::new().on_press(Msg::Pop))
            .iter()
            .any(|l| l.contains("Back")));

        crate::localizations::scope(Rc::new(French), || {
            let said = labels(&BackButton::<Msg>::new().on_press(Msg::Pop));
            assert!(
                said.iter().any(|l| l.contains("Retour")),
                "the reader's own word: {said:?}"
            );
            let drawer = labels(&DrawerButton::<Msg>::new().on_press(Msg::Pop));
            assert!(
                drawer.iter().any(|l| l.contains("Ouvrir le menu")),
                "{drawer:?}"
            );
        });
    }

    /// **Both drawers take the same word**, which is the reference's own arrangement
    /// (`action_buttons.dart:331` against `:362`): a reader told which *edge* a panel
    /// comes in from is being told about the layout rather than about the action.
    #[test]
    fn both_drawer_buttons_say_the_same_thing() {
        assert_eq!(Kind::Drawer.label(), Kind::EndDrawer.label());
    }

    /// **A theme can replace the glyphs** — the reference's `ActionIconTheme`. An
    /// application with its own icon set should not have to leave four of the framework's
    /// showing through.
    #[test]
    fn a_theme_can_replace_the_glyphs() {
        let mut theme = Theme::default();
        theme.widgets.action_icons.back = Some(Icons::ChevronLeft);
        assert_eq!(Kind::Back.icon(&theme), Icons::ChevronLeft);
        assert_eq!(
            Kind::Back.icon(&Theme::default()),
            default_back_icon(),
            "and the framework's own is the platform's"
        );
    }

    /// A button with nothing to say is **inert**, not disabled: it draws and it is named,
    /// and it answers nothing. Disabled is the other thing, and says so.
    #[test]
    fn a_button_with_no_message_answers_nothing() {
        let ui = build_ui(
            &BackButton::<Msg>::new(),
            Size::new(200.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(ui
            .hit(frus_core::Point::new(20.0, 20.0))
            .and_then(|id| ui.msg_for(id))
            .is_none());
    }
}
