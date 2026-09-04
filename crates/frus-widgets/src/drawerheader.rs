//! [`DrawerHeader`] and [`UserAccountsDrawerHeader`]: the block at the top of a side
//! panel, above the destinations.
//!
//! [`NavigationDrawer`](crate::NavigationDrawer) has had a
//! [header](crate::NavigationDrawer::header) slot since milestone 467 and nothing to put
//! in it. These are the two things the reference puts there: a plain block of a fixed
//! height with a rule under it, and the same block laid out for an account — a picture,
//! a name, an address, and a control for switching.
//!
//! ```ignore
//! NavigationDrawer::new(app.tab, Msg::Go)
//!     .header(
//!         UserAccountsDrawerHeader::new()
//!             .account_name("Ada Lovelace")
//!             .account_email("ada@example.com")
//!             .current_picture(CircleAvatar::new("AL")),
//!     )
//!     .item("in", "Inbox")
//! ```
//!
//! **The header is where the notch lands.** A panel runs to the top of the screen, so the
//! first thing in it is the thing under the status bar: both of these add the top
//! intrusion to their height *and* to their padding, which is what keeps the block the
//! same size below the notch on every device while its background still runs up behind it
//! (`drawer_header.dart:86`, `:90`).

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Insets, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::spacer::Spacer;
use crate::theme::Theme;
use crate::widget::{FillAxes, Widget};

/// The block's height, the rule under it included (`drawer_header.dart:16`). The
/// reference writes it as `160.0 + 1.0`, the `+ 1` being the bottom edge, and that is
/// worth keeping visible: the rule is *inside* the header rather than under it, so a
/// header and its rule together are 161 and the thing below starts at 161.
pub const DRAWER_HEADER_HEIGHT: f32 = 160.0 + 1.0;
/// The room under the header, before whatever comes next (`drawer_header.dart:39`).
const MARGIN: Insets = Insets::new(0.0, 0.0, 8.0, 0.0);
/// The room around a plain header's content (`drawer_header.dart:40`, `fromLTRB`).
const PADDING: Insets = Insets::new(16.0, 16.0, 8.0, 16.0);
/// And around an account header's, which leaves its trailing edge to the pictures
/// (`user_accounts_drawer_header.dart:366`).
const ACCOUNT_PADDING: Insets = Insets::new(16.0, 0.0, 0.0, 16.0);
/// The row at the foot of an account header: the name, the address, the arrow
/// (`user_accounts_drawer_header.dart:208`).
const DETAILS_HEIGHT: f32 = 56.0;
/// The current account's picture (`user_accounts_drawer_header.dart:296`).
const CURRENT_PICTURE: f32 = 72.0;
/// And each of the others (`:297`).
const OTHER_PICTURE: f32 = 40.0;
/// How many of the others are shown (`user_accounts_drawer_header.dart:43`).
const OTHER_PICTURE_LIMIT: usize = 3;
/// Between them.
const PICTURE_GAP: f32 = 8.0;

/// A fixed block at the top of a side panel, with a rule under it.
///
/// It sizes and places its content and paints two things: a background, when it was given
/// one, and the rule. Everything else is the child's.
pub struct DrawerHeader<Msg> {
    child: RefCell<Option<Box<dyn Widget<Msg>>>>,
    background: Option<Color>,
    padding: Option<Insets>,
    margin: Option<Insets>,
    height: Option<f32>,
    /// Whether the rule under it is drawn. The reference always draws it; an account
    /// header on a coloured ground does not want it, and says so.
    rule: bool,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> DrawerHeader<Msg> {
    /// An empty header of the default height.
    pub fn new() -> Self {
        Self {
            child: RefCell::new(None),
            background: None,
            padding: None,
            margin: None,
            height: None,
            rule: true,
            built: OnceCell::new(),
        }
    }

    /// What goes in it, inset by the padding.
    pub fn child(self, child: impl Widget<Msg> + 'static) -> Self {
        self.child_boxed(Box::new(child))
    }

    /// [`Self::child`], for a widget already boxed.
    pub fn child_boxed(mut self, child: Box<dyn Widget<Msg>>) -> Self {
        *self.child.borrow_mut() = Some(child);
        self.built.take();
        self
    }

    /// The block's fill (`drawer_header.dart:52`). Unset, nothing — the panel's own
    /// surface shows through, which is what a header that is only a title wants.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The room around the content. Unset, the reference's `16` on three sides and `8`
    /// underneath. **The top intrusion is added to it**, never replaced by it.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The room around the block itself. Unset, `8` underneath and nothing elsewhere.
    pub fn margin(mut self, margin: Insets) -> Self {
        self.margin = Some(margin);
        self
    }

    /// The block's height, before the top intrusion is added. Unset,
    /// [`DRAWER_HEADER_HEIGHT`].
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Whether the rule under it is drawn. Unset, `true`.
    ///
    /// A header with a background of its own is already told apart from what follows it,
    /// and a hairline over a colour is the mash-up [`Card`](crate::Card) was taken apart
    /// for — so [`UserAccountsDrawerHeader`] turns it off.
    pub fn rule(mut self, rule: bool) -> Self {
        self.rule = rule;
        self
    }

    /// The intrusion at the top of the screen, which a header standing at the top of a
    /// panel is under.
    fn safe_top(&self) -> f32 {
        crate::MediaQuery::of().padding.top
    }

    fn sizing(&self) -> Style {
        let safe = self.safe_top();
        let pad = self.padding.unwrap_or(PADDING);
        Style {
            height: Dimension::Length(self.height.unwrap_or(DRAWER_HEADER_HEIGHT) + safe),
            flex_direction: FlexDirection::Column,
            padding: Insets::new(pad.top + safe, pad.right, pad.bottom, pad.left),
            margin: self.margin.unwrap_or(MARGIN),
            ..Default::default()
        }
    }
}

impl<Msg: Clone + 'static> Default for DrawerHeader<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for DrawerHeader<Msg> {
    fn style(&self) -> Style {
        self.sizing()
    }

    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built
            .get_or_init(|| self.child.borrow_mut().take().into_iter().collect())
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        if let Some(background) = self.background {
            scene.fill_rect(bounds, background.fade(o));
        }
        if self.rule {
            // The rule is the header's **last pixel**, not a line under it — which is
            // what the reference's `160.0 + 1.0` says, and why a header and the thing
            // below it never have a gap of exactly one pixel between them.
            let thickness = theme
                .widgets
                .divider
                .thickness
                .unwrap_or(crate::DIVIDER_THICKNESS);
            let color = theme
                .widgets
                .divider
                .color
                .unwrap_or(theme.scheme.outline_variant);
            scene.fill_rect(
                Rect::new(
                    bounds.x,
                    bounds.y + bounds.height - thickness,
                    bounds.width,
                    thickness,
                ),
                color.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A [`DrawerHeader`] laid out for the account the application is signed in to: a
/// picture, up to three others, a name, an address, and a control for switching between
/// them.
pub struct UserAccountsDrawerHeader<Msg> {
    account_name: Option<String>,
    account_email: Option<String>,
    current_picture: RefCell<Option<Box<dyn Widget<Msg>>>>,
    other_pictures: RefCell<Vec<Box<dyn Widget<Msg>>>>,
    current_picture_size: Option<f32>,
    other_pictures_size: Option<f32>,
    background: Option<Color>,
    margin: Option<Insets>,
    arrow_color: Option<Color>,
    name_text_style: Option<TextStyle>,
    email_text_style: Option<TextStyle>,
    on_details_pressed: Option<Msg>,
    open: bool,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> UserAccountsDrawerHeader<Msg> {
    /// An account header with nothing said about the account yet.
    pub fn new() -> Self {
        Self {
            account_name: None,
            account_email: None,
            current_picture: RefCell::new(None),
            other_pictures: RefCell::new(Vec::new()),
            current_picture_size: None,
            other_pictures_size: None,
            background: None,
            margin: None,
            arrow_color: None,
            name_text_style: None,
            email_text_style: None,
            on_details_pressed: None,
            open: false,
            built: OnceCell::new(),
        }
    }

    /// The account's name, on the upper of the two lines.
    pub fn account_name(mut self, name: impl Into<String>) -> Self {
        self.account_name = Some(name.into());
        self.built.take();
        self
    }

    /// The account's address, on the lower one.
    pub fn account_email(mut self, email: impl Into<String>) -> Self {
        self.account_email = Some(email.into());
        self.built.take();
        self
    }

    /// The picture standing for the account, in the upper corner. Usually a
    /// [`CircleAvatar`](crate::CircleAvatar).
    pub fn current_picture(self, picture: impl Widget<Msg> + 'static) -> Self {
        self.current_picture_boxed(Box::new(picture))
    }

    /// [`Self::current_picture`], for a widget already boxed.
    pub fn current_picture_boxed(mut self, picture: Box<dyn Widget<Msg>>) -> Self {
        *self.current_picture.borrow_mut() = Some(picture);
        self.built.take();
        self
    }

    /// Adds a picture for one of the account's *others*, in the opposite corner. **Three
    /// of them are shown** and the rest are dropped, as the reference drops them
    /// (`user_accounts_drawer_header.dart:43`): the corner has room for three, and a
    /// fourth would either shrink the other three or run under the current account's.
    pub fn other_picture(self, picture: impl Widget<Msg> + 'static) -> Self {
        self.other_picture_boxed(Box::new(picture))
    }

    /// [`Self::other_picture`], for a widget already boxed.
    pub fn other_picture_boxed(mut self, picture: Box<dyn Widget<Msg>>) -> Self {
        self.other_pictures.borrow_mut().push(picture);
        self.built.take();
        self
    }

    /// The size of the current account's picture. Unset, `72`.
    pub fn current_picture_size(mut self, size: f32) -> Self {
        self.current_picture_size = Some(size);
        self.built.take();
        self
    }

    /// The size of each of the others. Unset, `40`.
    pub fn other_pictures_size(mut self, size: f32) -> Self {
        self.other_pictures_size = Some(size);
        self.built.take();
        self
    }

    /// The block's fill. Unset, the scheme's `primary`
    /// (`user_accounts_drawer_header.dart:364`).
    pub fn background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.built.take();
        self
    }

    /// The room around the block. Unset, `8` underneath.
    pub fn margin(mut self, margin: Insets) -> Self {
        self.margin = Some(margin);
        self.built.take();
        self
    }

    /// The switching control's colour.
    ///
    /// Unset, `on_primary` — **not** the reference's `Colors.white`
    /// (`user_accounts_drawer_header.dart:301`). White is right for the dark primary a
    /// 2014 palette had and wrong on a light one, and this framework has a role that means
    /// *what goes on primary*. Naming a colour here still wins.
    pub fn arrow_color(mut self, color: Color) -> Self {
        self.arrow_color = Some(color);
        self.built.take();
        self
    }

    /// The name's type. Unset, `bodyLarge`
    /// (`user_accounts_drawer_header.dart:151`).
    pub fn name_text_style(mut self, style: TextStyle) -> Self {
        self.name_text_style = Some(style);
        self.built.take();
        self
    }

    /// The address's type. Unset, `bodyMedium` (`:163`).
    pub fn email_text_style(mut self, style: TextStyle) -> Self {
        self.email_text_style = Some(style);
        self.built.take();
        self
    }

    /// What to emit when the lower half is pressed — the request to show or hide the
    /// other accounts. Without it there is **no control at all**, which is the
    /// reference's rule (`user_accounts_drawer_header.dart:169`): an arrow that opened
    /// nothing would be a promise the header cannot keep.
    pub fn on_details_pressed(mut self, message: Msg) -> Self {
        self.on_details_pressed = Some(message);
        self.built.take();
        self
    }

    /// Whether the other accounts are currently showing, which is what the control points
    /// at. The state lives in the application, as every other piece of state here does.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self.built.take();
        self
    }

    /// **How far the two lines are lifted off the foot of the details row**, so that the
    /// *bottom* one is centred on the control beside it rather than the pair of them being
    /// centred together (`user_accounts_drawer_header.dart:262`).
    ///
    /// The address sits at the middle of the 56-pixel row and the name goes *above* it,
    /// overflowing up into the pictures. That is not a rounding of "centre the two lines"
    /// — it is about ten pixels different — and it is what keeps the address level with
    /// the arrow whether or not there is a name at all. Which is the whole reason the
    /// reference lays this row out by hand instead of using a column.
    fn bottom_lift(&self, theme: &Theme) -> f32 {
        let (name, email) = self.line_heights(theme);
        // Which line is the bottom one: the address when there is one, else the name.
        let bottom = if self.account_email.is_some() {
            email
        } else {
            name
        };
        (DETAILS_HEIGHT * 0.5 - bottom * 0.5).max(0.0)
    }

    /// **How tall the two lines actually lay out**, measured rather than derived from the
    /// type scale's line height.
    ///
    /// The two are not the same number and the difference is under a pixel, which is
    /// exactly the size of difference that costs an afternoon: a row sized from
    /// `line_height` and filled with text laid out from a measurement is a row that
    /// overflows by six tenths of a pixel and says so in a yellow banner across the
    /// header. Measure the thing that will be drawn.
    fn line_heights(&self, theme: &Theme) -> (f32, f32) {
        let scale = crate::theme::type_scale(Some(theme));
        let measured = |text: &Option<String>, style: TextStyle| {
            text.as_ref().map_or(0.0, |text| {
                frus_text::measure_resolved(text, &style.resolved()).height
            })
        };
        (
            measured(
                &self.account_name,
                self.name_text_style.unwrap_or(scale.body_large),
            ),
            measured(
                &self.account_email,
                self.email_text_style.unwrap_or(scale.body_medium),
            ),
        )
    }

    /// The two lines and the control, in a row at least [`DETAILS_HEIGHT`] tall.
    fn details(&self, theme: &Theme) -> Flex<Msg> {
        let scale = crate::theme::type_scale(Some(theme));
        let ink = theme.scheme.on_primary;
        let name_style = self.name_text_style.unwrap_or(scale.body_large);
        let email_style = self.email_text_style.unwrap_or(scale.body_medium);
        let lift = self.bottom_lift(theme);

        let mut lines = Flex::<Msg>::column().padding_each(0.0, 0.0, lift, 0.0);
        if let Some(name) = &self.account_name {
            lines = lines.child(
                crate::Text::styled(name.clone(), name_style)
                    .color(ink)
                    .ellipsis(),
            );
        }
        if let Some(email) = &self.account_email {
            lines = lines.child(
                crate::Text::styled(email.clone(), email_style)
                    .color(ink)
                    .ellipsis(),
            );
        }

        let mut row = Flex::<Msg>::row()
            .align(Align::End)
            // **A strut, not a height.** The row is as tall as [`DETAILS_HEIGHT`] or as
            // tall as its lines, whichever is more — a name above an address that is
            // itself centred on the row's midpoint is taller than 56 by construction, and
            // taller again under a reader who enlarged the type.
            //
            // Saying that as `height(max(56, lines))` means computing what the lines will
            // measure, and a row sized from one arithmetic and filled from another
            // overflows by six tenths of a pixel and says so in a yellow band across the
            // header. A zero-width child of the right height floors the row without
            // anybody predicting anything.
            //
            // The reference has the row at a flat 56 and lets the name overflow up into
            // the pictures, which its hand-written layout does not mind. Here the boxes
            // are real, so the row grows and the pictures give up the difference — every
            // line lands where the reference puts it, with nothing outside its box.
            .child(Flex::<Msg>::column().width(0.0).height(DETAILS_HEIGHT))
            .child(crate::Expanded::new(lines));
        if self.on_details_pressed.is_some() {
            let words = crate::localizations::of();
            // The reference rotates one triangle through half a turn; two chevrons are
            // the two ends of that turn, and this framework has both drawn on the grid.
            let (glyph, label) = if self.open {
                (crate::Icons::ChevronUp, words.hide_accounts_label())
            } else {
                (crate::Icons::ChevronDown, words.show_accounts_label())
            };
            row = row.child(
                crate::Semantics::new(
                    frus_core::SemanticsProperties::new(frus_core::Role::Button).clickable(),
                    Flex::<Msg>::column()
                        .width(DETAILS_HEIGHT)
                        .height(DETAILS_HEIGHT)
                        .justify(frus_layout::Justify::Center)
                        .align(Align::Center)
                        .child(crate::Icon::new(glyph).color(self.arrow_color.unwrap_or(ink))),
                )
                .label(label.to_string()),
            );
        }
        row
    }

    /// The header's subtree, built once under the theme it will be drawn in.
    fn assemble(&self, theme: &Theme) -> Vec<Box<dyn Widget<Msg>>> {
        let current = self.current_picture.borrow_mut().take();
        let others = std::mem::take(&mut *self.other_pictures.borrow_mut());
        let current_size = self.current_picture_size.unwrap_or(CURRENT_PICTURE);
        let other_size = self.other_pictures_size.unwrap_or(OTHER_PICTURE);

        // The pictures: the current account in the leading corner, the others in the
        // trailing one. The reference stacks them at `top: 0` and `end: 0`; a row with a
        // `Spacer` between puts them in the same two places and follows the direction
        // across, so a header in Arabic does not have to be told which corner is which.
        let mut pictures = Flex::<Msg>::row()
            .flex(1.0)
            .align(Align::Start)
            .padding_each(0.0, 16.0, 0.0, 0.0);
        pictures = match current {
            Some(picture) => pictures.child_boxed(Box::new(
                Flex::<Msg>::column()
                    .width(current_size)
                    .height(current_size)
                    .child_boxed(picture),
            )),
            None => pictures.child(Flex::<Msg>::column().width(current_size)),
        };
        pictures = pictures.child(Spacer::new());
        let mut rest = Flex::<Msg>::row().gap(PICTURE_GAP).align(Align::Start);
        for picture in others.into_iter().take(OTHER_PICTURE_LIMIT) {
            rest = rest.child_boxed(Box::new(
                Flex::<Msg>::column()
                    .width(other_size)
                    .height(other_size)
                    .child_boxed(picture),
            ));
        }
        pictures = pictures.child(rest);

        let body = Flex::<Msg>::column()
            .child(pictures)
            .child(self.details(theme));

        // The whole block is **one thing** to a reader, announced as such
        // (`user_accounts_drawer_header.dart:359`) — otherwise a name, an address and an
        // arrow arrive as three unrelated nodes at the top of the panel.
        let words = crate::localizations::of();
        let mut header = DrawerHeader::<Msg>::new()
            .rule(false)
            .padding(ACCOUNT_PADDING)
            .background_color(self.background.unwrap_or(theme.scheme.primary))
            .child(body);
        if let Some(margin) = self.margin {
            header = header.margin(margin);
        }
        let annotated = crate::Semantics::new(
            frus_core::SemanticsProperties::default(),
            match &self.on_details_pressed {
                // The press is the row's, and it is the row the reference makes tappable
                // (`:197`) rather than the arrow: a 56-pixel arrow beside a name that is
                // not tappable is a target nobody finds.
                Some(message) => Box::new(
                    crate::Container::new()
                        .on_click(message.clone())
                        .child(header),
                ) as Box<dyn Widget<Msg>>,
                None => Box::new(header),
            },
        )
        .label(words.signed_in_label().to_string());

        vec![Box::new(annotated)]
    }
}

impl<Msg: Clone + 'static> Default for UserAccountsDrawerHeader<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for UserAccountsDrawerHeader<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes::WIDTH
    }

    /// The two type steps and the ink are the theme's, so the subtree cannot be built
    /// before there is one — the idiom every composed widget here uses.
    fn build_themed(&self, theme: &Theme) {
        let _ = self.built.set(self.assemble(theme));
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble(&Theme::default()))
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Details,
    }

    fn scene_of(widget: &dyn Widget<Msg>, bounds: Rect, theme: &Theme) -> Scene {
        let mut scene = Scene::new();
        widget.paint(bounds, Status::default(), theme, &mut scene);
        scene
    }

    /// Every widget in a built subtree whose short type name matches.
    fn find<'a>(node: &'a dyn Widget<Msg>, name: &str, out: &mut Vec<&'a dyn Widget<Msg>>) {
        if node.debug_name() == name {
            out.push(node);
        }
        for child in node.children() {
            find(&**child, name, out);
        }
    }

    /// How many glyphs the subtree draws.
    fn glyph_count(root: &dyn Widget<Msg>) -> usize {
        let mut found = Vec::new();
        find(root, "Icon", &mut found);
        found.len()
    }

    /// Every label a `Semantics` wrapper in the subtree carries.
    fn announced(root: &dyn Widget<Msg>) -> Vec<String> {
        let mut found = Vec::new();
        find(root, "Semantics", &mut found);
        found
            .iter()
            .filter_map(|node| node.describes()?.props.label.clone())
            .collect()
    }

    /// **The notch is added to the block, not taken out of it.** A header is the first
    /// thing in a panel that runs to the top of the screen, so its background has to go up
    /// behind the status bar while its content stays the same 160 pixels tall underneath —
    /// which means the intrusion lands on the height *and* on the padding
    /// (`drawer_header.dart:86`, `:90`). Subtracting it in one place and not the other is
    /// the mistake, and it only shows on a device.
    #[test]
    fn the_notch_is_added_to_the_height_and_to_the_padding() {
        let flat = Widget::<Msg>::style(&DrawerHeader::<Msg>::new());
        assert_eq!(flat.height, Dimension::Length(DRAWER_HEADER_HEIGHT));
        assert_eq!(flat.padding.top, PADDING.top);

        let notched = crate::MediaQuery::new(frus_core::Size::new(360.0, 780.0))
            .with_insets(frus_core::WindowInsets::bars(Insets::new(
                48.0, 0.0, 0.0, 0.0,
            )))
            .scope(|| Widget::<Msg>::style(&DrawerHeader::<Msg>::new()));
        assert_eq!(
            notched.height,
            Dimension::Length(DRAWER_HEADER_HEIGHT + 48.0),
            "the block grows by the intrusion"
        );
        assert_eq!(
            notched.padding.top,
            PADDING.top + 48.0,
            "and the content is pushed clear of it, so the block below the notch is \
             still the same block"
        );
    }

    /// The rule is the header's **last pixel**, which is what `160.0 + 1.0` says.
    #[test]
    fn the_rule_is_the_headers_own_last_pixel() {
        let theme = Theme::default();
        let bounds = Rect::new(0.0, 0.0, 300.0, DRAWER_HEADER_HEIGHT);
        let scene = scene_of(&DrawerHeader::<Msg>::new(), bounds, &theme);
        let rule = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
            .expect("a header draws its rule");
        assert_eq!(rule.0.height, crate::DIVIDER_THICKNESS);
        assert_eq!(rule.0.y, DRAWER_HEADER_HEIGHT - crate::DIVIDER_THICKNESS);
        assert_eq!(rule.1, theme.scheme.outline_variant);

        assert!(
            scene_of(&DrawerHeader::<Msg>::new().rule(false), bounds, &theme)
                .primitives()
                .is_empty(),
            "and a header told not to draw one draws nothing at all"
        );
    }

    /// **No control without something for it to do.** An arrow that opened nothing is a
    /// promise the header cannot keep (`user_accounts_drawer_header.dart:169`).
    #[test]
    fn there_is_no_switch_until_there_is_somewhere_to_switch_to() {
        let theme = Theme::default();
        let quiet = UserAccountsDrawerHeader::<Msg>::new()
            .account_name("Ada Lovelace")
            .account_email("ada@example.com");
        quiet.build_themed(&theme);
        assert_eq!(glyph_count(&quiet), 0);

        let live = UserAccountsDrawerHeader::<Msg>::new()
            .account_name("Ada Lovelace")
            .account_email("ada@example.com")
            .on_details_pressed(Msg::Details);
        live.build_themed(&theme);
        assert_eq!(glyph_count(&live), 1);
    }

    /// And the word on it says what the press **will do**, which is the only kind a reader
    /// can act on — so it is two entries on the table rather than one that flips.
    #[test]
    fn the_switch_is_named_for_what_it_will_do() {
        let theme = Theme::default();
        let header = |open: bool| {
            let header = UserAccountsDrawerHeader::<Msg>::new()
                .account_name("Ada")
                .on_details_pressed(Msg::Details)
                .open(open);
            header.build_themed(&theme);
            announced(&header)
        };
        assert!(header(false).iter().any(|w| w == "Show accounts"));
        assert!(header(true).iter().any(|w| w == "Hide accounts"));
    }

    /// The whole block is **one thing** to a reader (`user_accounts_drawer_header.dart:359`).
    /// Without it, a name, an address and an arrow arrive as three unrelated nodes at the
    /// top of a panel and none of them says what it is the top of.
    #[test]
    fn the_block_announces_itself_once() {
        let theme = Theme::default();
        let header = UserAccountsDrawerHeader::<Msg>::new().account_name("Ada");
        header.build_themed(&theme);
        assert!(announced(&header).iter().any(|w| w == "Signed in"));
    }

    /// **The bottom line is centred on the control, not the pair of lines.**
    ///
    /// The reference places the bottom line's centre at the row's centre and puts the name
    /// above it (`user_accounts_drawer_header.dart:262`), so the address stays level with
    /// the arrow beside it whether or not there is a name. Centring the two lines together
    /// is the obvious reading and lands about ten pixels off.
    #[test]
    fn the_address_is_level_with_the_control() {
        let theme = Theme::default();
        let scale = crate::theme::type_scale(Some(&theme));

        let both = UserAccountsDrawerHeader::<Msg>::new()
            .account_name("Ada Lovelace")
            .account_email("ada@example.com");
        let email_h =
            frus_text::measure_resolved("ada@example.com", &scale.body_medium.resolved()).height;
        assert_eq!(
            both.bottom_lift(&theme),
            DETAILS_HEIGHT * 0.5 - email_h * 0.5,
            "the address sits at the middle of the row"
        );

        // With no address, the **name** is the bottom line and takes its place.
        let name_only = UserAccountsDrawerHeader::<Msg>::new().account_name("Ada Lovelace");
        let name_h =
            frus_text::measure_resolved("Ada Lovelace", &scale.body_large.resolved()).height;
        assert_eq!(
            name_only.bottom_lift(&theme),
            DETAILS_HEIGHT * 0.5 - name_h * 0.5,
        );
        assert_ne!(
            both.bottom_lift(&theme),
            name_only.bottom_lift(&theme),
            "the two steps have to differ for that to mean anything"
        );
    }

    /// Three of the other accounts are shown and the rest are dropped: the corner has room
    /// for three (`user_accounts_drawer_header.dart:43`).
    #[test]
    fn only_three_of_the_other_accounts_are_shown() {
        let theme = Theme::default();
        let header = UserAccountsDrawerHeader::<Msg>::new()
            .account_name("Ada")
            .other_picture(crate::CircleAvatar::new("A"))
            .other_picture(crate::CircleAvatar::new("B"))
            .other_picture(crate::CircleAvatar::new("C"))
            .other_picture(crate::CircleAvatar::new("D"));
        header.build_themed(&theme);
        let mut found = Vec::new();
        find(&header, "CircleAvatar", &mut found);
        assert_eq!(found.len(), 3, "the fourth is dropped, not squeezed in");
    }

    /// The builders are order-independent, the subtree being thrown away by each of them.
    #[test]
    fn saying_it_afterwards_says_it() {
        let theme = Theme::default();
        let after = UserAccountsDrawerHeader::<Msg>::new()
            .account_name("Ada")
            .on_details_pressed(Msg::Details)
            .open(true);
        after.build_themed(&theme);
        assert!(announced(&after).iter().any(|w| w == "Hide accounts"));
    }
}
