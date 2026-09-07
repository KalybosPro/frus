//! [`TextField`]: a single-line input field, **controlled** (its value comes from
//! the application state), with a caret, navigation and selection.
//!
//! The value is controlled; the **caret / selection** are edit state retained at
//! runtime ([`Edit`]), keyed by widget identity.

use frus_core::{Point, Rect, ResolvedTextStyle, Scene, TextAlign, TextStyle};
use frus_layout::{Dimension, Style};
use frus_text::TextLayout;

use crate::disabled::DISABLED_CONTENT_OPACITY;
use crate::icons::IconData;
use crate::ime::{Capitalization, Ime, KeyboardType, TextInputAction};
use crate::interaction::{Key, Status};
use crate::runtime::Edit;
use crate::theme::Theme;
use crate::widget::Widget;

/// Padding either side of the content, both variants.
pub const FIELD_PADDING_X: f32 = 12.0;
/// Padding above and below the content in a [`filled`](TextField::filled) field.
///
/// The reference's Material 3 content padding is `(12, 8, 12, 8)` filled and
/// `(12, 20, 12, 12)` outlined. The asymmetry is not decoration: an outlined field's
/// floating label sits **on** the top border, so the top has to give it room, while a
/// filled one floats its label inside the box.
pub const FIELD_PADDING_Y: f32 = 8.0;
/// Padding above the content in an [`outlined`](TextField::outlined) field.
pub const FIELD_OUTLINED_PADDING_TOP: f32 = 20.0;
/// Padding below the content in an outlined field.
pub const FIELD_OUTLINED_PADDING_BOTTOM: f32 = 12.0;

/// Padding above and below the content in a [`dense`](TextField::dense) filled field —
/// the reference's `(12, 4, 12, 4)`.
pub const FIELD_DENSE_PADDING_Y: f32 = 4.0;
/// Padding above the content in a dense **outlined** field: the reference's
/// `(12, 16, 12, 8)`, which keeps room for the label on the border while giving back
/// everything else.
pub const FIELD_DENSE_OUTLINED_PADDING_TOP: f32 = 16.0;
/// Padding below the content in a dense outlined field.
pub const FIELD_DENSE_OUTLINED_PADDING_BOTTOM: f32 = 8.0;

/// Size of the value and of the resting label — the reference's `body_large`.
pub const FIELD_TEXT_SIZE: f32 = 16.0;
/// What the label shrinks to once it floats. The reference scales it rather than naming
/// a second size, so a field given a larger type keeps the same proportion.
pub const FIELD_LABEL_SCALE: f32 = 0.75;
/// Size of the helper and error line — the reference's `body_small`.
pub const FIELD_SUB_SIZE: f32 = 12.0;
/// Vertical gap between the box and the helper/error line, and above a floating label.
pub const FIELD_GAP: f32 = 4.0;
/// Margin either side of the floating label inside the border's **notch**, which is the
/// reference's `OutlineInputBorder.gapPadding`.
pub const FIELD_NOTCH_GAP: f32 = 4.0;

/// Corner radius of the box, and of a filled field's two top corners.
pub const FIELD_RADIUS: f32 = 4.0;
/// Border weight at rest, and once focused — the reference widens rather than recolours
/// alone, which is what makes focus readable without colour.
pub const FIELD_BORDER_WIDTH: f32 = 1.0;
/// Border weight of a focused field.
pub const FIELD_FOCUSED_BORDER_WIDTH: f32 = 2.0;

/// Side of a prefix/suffix icon (logical px) and the margin around it.
pub const FIELD_ICON_SIZE: f32 = 24.0;
const ICON_PAD: f32 = 6.0;
/// What a disabled field keeps of its colours.
///
/// An alias, kept for the name: the number is the framework's one disabled content
/// opacity, [`DISABLED_CONTENT_OPACITY`], and a field's 38 % moving independently of
/// every other control's would be a bug rather than a setting.
pub const FIELD_DISABLED_OPACITY: f32 = DISABLED_CONTENT_OPACITY;
/// Default masking character of a password field.
const OBSCURE_CHAR: char = '•';

/// Which of the reference's two fields this is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TextFieldVariant {
    /// A box the caller can see all the way round, the floating label notching its top
    /// border. The default here.
    #[default]
    Outlined,
    /// A tinted container with a single line under it, the label floating **inside** the
    /// box. The line carries the state; the fill carries the affordance.
    Filled,
    /// **No container at all** — the reference's `InputBorder.none`. No fill, no box, no
    /// line: only the value, the hint and whatever icons were asked for.
    ///
    /// This is the field a widget puts *inside* something that is already a container. A
    /// [`SearchBar`](crate::SearchBar) is a raised stadium with a field in it, and a field
    /// that drew its own box inside that would be two containers deep with the outer one's
    /// corners cut by the inner one's. The reference reaches for it in exactly those
    /// places (`search_anchor.dart:1810`).
    ///
    /// It lays out like [`Filled`](Self::Filled) — the label floats inside rather than
    /// notching a border, there being no border to notch — and simply paints nothing
    /// behind the content.
    None,
}

/// Everything a [`TextField`] paints, each answer the caller's, the theme's, or the
/// framework's — resolved in that order.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextFieldStyle {
    /// Container fill. A filled field defaults to the theme's high surface container; an
    /// outlined one to nothing at all.
    pub fill: Option<frus_core::Color>,
    /// The border, or the underline, at rest.
    pub border_color: Option<frus_core::Color>,
    /// The border once focused.
    pub focused_border_color: Option<frus_core::Color>,
    /// Border, label and helper colour while an error is showing.
    pub error_color: Option<frus_core::Color>,
    /// The same, **under the pointer**: an errored field deepens on hover
    /// (`input_decorator.dart:5981`). Unset, the scheme's `on_error_container`.
    pub error_hover_color: Option<frus_core::Color>,
    /// The value's colour.
    pub text_color: Option<frus_core::Color>,
    /// The label and the hint, at rest.
    pub label_color: Option<frus_core::Color>,
    /// The label once focused.
    pub focused_label_color: Option<frus_core::Color>,
    /// The helper line.
    pub helper_color: Option<frus_core::Color>,
    /// The prefix and suffix icons.
    pub icon_color: Option<frus_core::Color>,
    /// Corner radius.
    pub radius: Option<f32>,
    /// Border weight at rest.
    pub border_width: Option<f32>,
    /// Border weight once focused.
    pub focused_border_width: Option<f32>,
    /// Side of the prefix/suffix icons.
    pub icon_size: Option<f32>,
    /// Padding either side of the content.
    pub padding_x: Option<f32>,
}

/// A single-line text input field, with optional **form decoration** (label, hint,
/// helper text, error). **Validity** stays decided by the application (a pure
/// function of the state); the field only displays its result through [`error`].
///
/// [`error`]: TextField::error
pub struct TextField<Msg> {
    value: String,
    size: f32,
    width: Dimension,
    on_input: Option<Box<dyn Fn(String) -> Msg>>,
    on_submit: Option<Msg>,
    /// Label displayed above the field.
    label: Option<String>,
    /// Hint displayed **inside** the field while the value is empty.
    placeholder: Option<String>,
    /// Helper text below the field (hidden when an error is present).
    helper: Option<String>,
    /// Error message: when present, the border and the label switch to the error
    /// colour and this text replaces the helper below the field.
    error: Option<String>,
    /// Masks the value (a password field): every character is rendered as
    /// [`OBSCURE_CHAR`]. Editing acts on the real value; only the display changes.
    obscure: bool,
    /// Where the text sits inside the field. Single-line only; see
    /// [`TextField::text_align`].
    text_align: TextAlign,
    /// Which keys the software keyboard offers; `None` = worked out from the field
    /// (a masked field is a secret, a multi-line one takes newlines).
    keyboard: Option<KeyboardType>,
    /// What its action key does; `None` = likewise.
    action: Option<TextInputAction>,
    /// Decorative icon on the left inside the box.
    prefix: Option<IconData>,
    /// Decorative icon on the right inside the box.
    suffix: Option<IconData>,
    /// Message emitted on a **click on the suffix icon** (a clear / reveal button…). Makes
    /// the suffix clickable: a click there emits this message instead of placing the caret.
    suffix_action: Option<Msg>,
    /// **Multi-line** field: Enter inserts a line break (instead of submitting), the box
    /// is `rows` lines tall and scrolls vertically to follow the caret.
    multiline: bool,
    /// Number of visible lines in multi-line mode.
    rows: u16,
    /// Which of the reference's two fields to draw.
    variant: TextFieldVariant,
    /// A field that is shown but cannot be edited: greyed out, and inert. It still
    /// displays its value — the reference's disabled field is readable, not hidden.
    enabled: bool,
    /// A **dense** field: the same shape, less room around the content. What a table's
    /// inline cell editor wants, and what a form does not.
    dense: bool,
    /// The most characters the field will hold; see [`TextField::max_length`].
    max_length: Option<usize>,
    /// What a typed character becomes before it reaches the value; see
    /// [`TextField::input_filter`].
    input_filter: Option<Box<dyn Fn(char) -> Option<char>>>,
    /// Which letters the software keyboard capitalises; see
    /// [`TextField::capitalization`].
    capitalization: Capitalization,
    /// A field whose value can be read, selected and copied but not changed; see
    /// [`TextField::read_only`].
    read_only: bool,
    /// Per-call overrides; everything unset falls to the theme, then the framework.
    style: TextFieldStyle,
}

/// A "word" character (letter/digit/`_`) for word jumps (Ctrl+Arrow).
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Next word boundary **to the left** of `cursor`: skips the separators, then the word
/// (editor behaviour — it stops at the **start** of the previous word).
fn word_boundary_left(chars: &[char], cursor: usize) -> usize {
    let mut i = cursor;
    while i > 0 && !is_word(chars[i - 1]) {
        i -= 1;
    }
    while i > 0 && is_word(chars[i - 1]) {
        i -= 1;
    }
    i
}

/// Next word boundary **to the right**: skips the separators, then the word — it stops
/// **after** the next word.
fn word_boundary_right(chars: &[char], cursor: usize) -> usize {
    let len = chars.len();
    let mut i = cursor;
    while i < len && !is_word(chars[i]) {
        i += 1;
    }
    while i < len && is_word(chars[i]) {
        i += 1;
    }
    i
}

/// Start of the **logical line** (after the previous `\n`, or 0) containing `cursor`.
fn line_start(chars: &[char], cursor: usize) -> usize {
    let mut i = cursor;
    while i > 0 && chars[i - 1] != '\n' {
        i -= 1;
    }
    i
}

/// End of the **logical line** (before the next `\n`, or the end) containing `cursor`.
fn line_end(chars: &[char], cursor: usize) -> usize {
    let len = chars.len();
    let mut i = cursor;
    while i < len && chars[i] != '\n' {
        i += 1;
    }
    i
}

/// Moves the caret to `target`, handling the selection anchor according to Shift.
fn move_cursor(cursor: &mut usize, anchor: &mut Option<usize>, target: usize, shift: bool) {
    if shift {
        if anchor.is_none() {
            *anchor = Some(*cursor);
        }
    } else {
        *anchor = None;
    }
    *cursor = target;
}

impl<Msg> TextField<Msg> {
    /// Creates a field displaying `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            size: FIELD_TEXT_SIZE,
            width: Dimension::Length(220.0),
            on_input: None,
            on_submit: None,
            label: None,
            placeholder: None,
            helper: None,
            error: None,
            obscure: false,
            text_align: TextAlign::Start,
            keyboard: None,
            action: None,
            prefix: None,
            suffix: None,
            suffix_action: None,
            multiline: false,
            rows: 3,
            variant: TextFieldVariant::Outlined,
            enabled: true,
            dense: false,
            max_length: None,
            input_filter: None,
            capitalization: Capitalization::Auto,
            read_only: false,
            style: TextFieldStyle::default(),
        }
    }

    /// **Outlined**: a box the caller can see all the way round, the floating label
    /// notching its top border. The default.
    pub fn outlined(mut self) -> Self {
        self.variant = TextFieldVariant::Outlined;
        self
    }

    /// **Filled**: a tinted container with a single line under it, the label floating
    /// inside the box rather than on its edge.
    /// **No container**: no fill, no box, no line — the reference's `InputBorder.none`.
    /// For a field inside something that is already a container. See
    /// [`TextFieldVariant::None`].
    pub fn borderless(mut self) -> Self {
        self.variant = TextFieldVariant::None;
        self
    }

    pub fn filled(mut self) -> Self {
        self.variant = TextFieldVariant::Filled;
        self
    }

    /// Chooses the variant directly, for a caller holding one in a variable.
    pub fn variant(mut self, variant: TextFieldVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Greys the field out and makes it inert. A disabled field still **shows** its
    /// value: the reference dims it rather than hiding it, since it is usually the answer
    /// to why the rest of a form is the way it is.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Overrides part of the styling for this field alone. Anything left `None` falls to
    /// the theme's [`TextFieldTheme`](crate::widgettheme::TextFieldTheme), then to the
    /// framework's defaults.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.style = style;
        self
    }

    /// Tightens the field: the reference's `isDense`, which trades the comfortable
    /// padding for a control that fits inside a table row. The shape, the label and the
    /// border are unchanged — only the room around the content.
    pub fn dense(mut self, dense: bool) -> Self {
        self.dense = dense;
        self
    }

    /// Whether this field is outlined (a box) rather than filled (a container and a line).
    fn is_outlined(&self) -> bool {
        self.variant == TextFieldVariant::Outlined
    }

    /// Whether this field paints **no container** — see [`TextFieldVariant::None`].
    fn is_borderless(&self) -> bool {
        self.variant == TextFieldVariant::None
    }

    /// Padding either side of the content.
    fn pad_x(&self) -> f32 {
        self.style.padding_x.unwrap_or(FIELD_PADDING_X)
    }

    /// Padding above the content: an outlined field gives its floating label room on the
    /// border, a filled one floats it inside the box and needs none.
    fn pad_top(&self) -> f32 {
        match (self.is_outlined(), self.dense) {
            (true, false) => FIELD_OUTLINED_PADDING_TOP,
            (true, true) => FIELD_DENSE_OUTLINED_PADDING_TOP,
            (false, false) => FIELD_PADDING_Y,
            (false, true) => FIELD_DENSE_PADDING_Y,
        }
    }

    /// Padding below the content.
    fn pad_bottom(&self) -> f32 {
        match (self.is_outlined(), self.dense) {
            (true, false) => FIELD_OUTLINED_PADDING_BOTTOM,
            (true, true) => FIELD_DENSE_OUTLINED_PADDING_BOTTOM,
            (false, false) => FIELD_PADDING_Y,
            (false, true) => FIELD_DENSE_PADDING_Y,
        }
    }

    /// The field's type, **resolved once**: the reader's font setting applied.
    ///
    /// Everything that measures, shapes, hit-tests, places a caret or paints the field's
    /// text goes through this one number. A caret placed from an unresolved size and
    /// glyphs drawn from a resolved one land in different places, and the field is then
    /// wrong for exactly the readers who most needed it to be right.
    fn text_style(&self) -> ResolvedTextStyle {
        TextStyle::new(self.size).resolved()
    }

    /// The helper line under the field — helper text, error, counter. See [`Self::text_style`].
    fn sub_style(&self) -> ResolvedTextStyle {
        TextStyle::new(FIELD_SUB_SIZE).resolved()
    }

    /// Size the label shrinks to once it floats — a proportion of the field's own type,
    /// not a second number, so a field given larger text keeps the relationship.
    fn label_size(&self) -> f32 {
        self.text_style().size * FIELD_LABEL_SCALE
    }

    /// Side of the prefix/suffix icons.
    fn icon_size(&self) -> f32 {
        self.style.icon_size.unwrap_or(FIELD_ICON_SIZE)
    }

    /// Distance from the top of the box to the first line of text: the padding, plus the
    /// room a filled field reserves for its floating label.
    fn text_top(&self) -> f32 {
        self.pad_top() + self.floating_label_height()
    }

    /// Room a **filled** field reserves above its content for the floating label. The
    /// reference computes it as `4 + 0.75 × label size` rather than reserving a band
    /// outside the box, which is why a filled field is taller than its text.
    fn floating_label_height(&self) -> f32 {
        if self.is_outlined() || self.label.is_none() {
            0.0
        } else {
            FIELD_GAP + self.label_size()
        }
    }

    /// This field's settings, resolved `caller ?? theme ?? framework`.
    fn resolved(&self, theme: &Theme) -> TextFieldStyle {
        let t = theme.widgets.text_field;
        let pick = |a: Option<frus_core::Color>,
                    b: Option<frus_core::Color>,
                    c: frus_core::Color| { Some(a.or(b).unwrap_or(c)) };
        TextFieldStyle {
            fill: pick(
                self.style.fill,
                t.fill,
                if self.is_outlined() || self.is_borderless() {
                    frus_core::Color::TRANSPARENT
                } else {
                    // `input_decorator.dart:5968` — a filled field takes the most
                    // emphasis a container has.
                    theme.scheme.surface_container_highest
                },
            ),
            border_color: pick(
                self.style.border_color,
                t.border_color,
                if self.is_outlined() {
                    theme.scheme.outline
                } else {
                    theme.scheme.on_surface_variant
                },
            ),
            focused_border_color: pick(
                self.style.focused_border_color,
                t.focused_border_color,
                theme.scheme.primary,
            ),
            error_color: pick(self.style.error_color, t.error_color, theme.scheme.error),
            error_hover_color: pick(
                self.style.error_hover_color,
                t.error_hover_color,
                theme.scheme.on_error_container,
            ),
            text_color: pick(self.style.text_color, t.text_color, theme.scheme.on_surface),
            label_color: pick(
                self.style.label_color,
                t.label_color,
                theme.scheme.on_surface_variant,
            ),
            focused_label_color: pick(
                self.style.focused_label_color,
                t.focused_label_color,
                theme.scheme.primary,
            ),
            helper_color: pick(
                self.style.helper_color,
                t.helper_color,
                theme.scheme.on_surface_variant,
            ),
            icon_color: pick(
                self.style.icon_color,
                t.icon_color,
                theme.scheme.on_surface_variant,
            ),
            radius: Some(self.style.radius.or(t.radius).unwrap_or(FIELD_RADIUS)),
            border_width: Some(
                self.style
                    .border_width
                    .or(t.border_width)
                    .unwrap_or(FIELD_BORDER_WIDTH),
            ),
            focused_border_width: Some(
                self.style
                    .focused_border_width
                    .or(t.focused_border_width)
                    .unwrap_or(FIELD_FOCUSED_BORDER_WIDTH),
            ),
            icon_size: Some(self.icon_size()),
            padding_x: Some(self.pad_x()),
        }
    }

    /// Switches the field to **multi-line**: Enter inserts a line break (instead of
    /// submitting), and the box shows `rows` lines (see [`rows`](Self::rows)).
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Number of visible lines in multi-line mode (at least 1). Implies
    /// [`multiline`](Self::multiline).
    pub fn rows(mut self, rows: u16) -> Self {
        self.multiline = true;
        self.rows = rows.max(1);
        self
    }

    /// Masks the value (a password field): every character becomes a dot. Editing
    /// stays normal; only the display is masked.
    ///
    /// It also changes what the **software keyboard** is told, unless
    /// [`keyboard_type`](Self::keyboard_type) says otherwise: a masked field is a
    /// [`Password`](KeyboardType::Password), which is what stops the keyboard learning
    /// what is typed into it. The dots are drawn by us and tell the keyboard nothing.
    pub fn obscure(mut self, obscure: bool) -> Self {
        self.obscure = obscure;
        self
    }

    /// Where the text sits inside the field: an amount to the right, a code centred.
    ///
    /// [`Start`](TextAlign::Start) — the default — follows the reading direction, so a
    /// field is left-aligned in English and right-aligned in Arabic.
    /// [`Left`](TextAlign::Left) and [`Right`](TextAlign::Right) do not: a column of
    /// figures wants the same edge whatever the prose around it is doing.
    ///
    /// **Single-line fields only.** A multi-line one stays at the start, and that is a
    /// boundary rather than an oversight: aligning wrapped text means moving each line
    /// by its own width, and the caret and the click would then have to be told about
    /// an offset that differs line by line. That belongs inside the text layout, where
    /// the caret and the hit test already live, and not in a widget nudging a block
    /// sideways behind their backs.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.text_align = align;
        self
    }

    /// How far the alignment pushes the text inside its content box.
    ///
    /// Zero unless the text is **narrower** than the box: there is nothing to
    /// distribute otherwise, and a right-aligned line longer than its field would be
    /// shoved off the left edge — the one edge whose text must stay put, since that
    /// is where reading starts and where the horizontal scroll brings the caret back to.
    ///
    /// One function, used by the paint, the caret and the hit test alike. Three copies
    /// of it would agree on every field until one of them did not, and the failure
    /// would be a click landing several characters from where it was aimed.
    ///
    /// It takes **no direction**, and that is deliberate rather than forgotten.
    /// [`Widget::cursor_at`] is handed a rectangle and nothing else — no theme, so no
    /// reading direction — so a push that depended on one could be applied by the paint
    /// and not by the click. Every field in a right-to-left application would then take
    /// its caret several characters from the tap, including every field that never asked
    /// to be aligned at all. A push both sides can compute is worth more than one that is
    /// right in a place the other cannot reach.
    ///
    /// So `Start` is the left edge here and `End` the right. Following the reading
    /// direction is the **text layout's** job, one layer down, where the caret and the
    /// hit test already live and cannot disagree with the glyphs.
    fn align_offset(&self, content_w: f32, text_w: f32) -> f32 {
        if self.multiline {
            return 0.0;
        }
        let slack = (content_w - text_w).max(0.0);
        match self.text_align {
            // Justification stretches the spaces between words, which a single line of
            // input has no business doing: it would move the glyphs under the caret
            // every time a space was typed.
            TextAlign::Start | TextAlign::Left | TextAlign::Justify => 0.0,
            TextAlign::End | TextAlign::Right => slack,
            TextAlign::Center => slack / 2.0,
        }
    }

    /// Which keys the software keyboard should offer — a phone keypad, an email
    /// address, a number.
    ///
    /// Untold, it is worked out from the field: a masked field is a secret, a
    /// multi-line one takes newlines, and anything else is ordinary text.
    pub fn keyboard_type(mut self, keyboard: KeyboardType) -> Self {
        self.keyboard = Some(keyboard);
        self
    }

    /// What the keyboard's action key does, and what it says — *Next* in a form,
    /// *Search* over a query, *Send* on a message.
    ///
    /// Untold, a multi-line field takes a newline and every other field says *Done*.
    pub fn action(mut self, action: TextInputAction) -> Self {
        self.action = Some(action);
        self
    }

    /// The keyboard this field asks for, once the defaults have had their say.
    fn ime_options(&self) -> Ime {
        let keyboard = self.keyboard.unwrap_or({
            // The order matters and only one way round makes sense: a masked field is
            // a secret first and a text field second, and a multi-line secret is not a
            // thing anybody types.
            if self.obscure {
                KeyboardType::Password
            } else if self.multiline {
                KeyboardType::Multiline
            } else {
                KeyboardType::Text
            }
        });
        let action = self.action.unwrap_or(match keyboard {
            // A field that takes several lines needs the key to *insert* one. Saying
            // *Done* there is a keyboard that cannot type what the field is for.
            KeyboardType::Multiline => TextInputAction::Newline,
            _ => TextInputAction::Done,
        });
        Ime {
            keyboard,
            action,
            capitalization: self.capitalization,
        }
    }

    /// Decorative icon on the left inside the field.
    pub fn prefix_icon(mut self, icon: IconData) -> Self {
        self.prefix = Some(icon);
        self
    }

    /// Decorative icon on the right inside the field.
    pub fn suffix_icon(mut self, icon: IconData) -> Self {
        self.suffix = Some(icon);
        self
    }

    /// Makes the **suffix icon clickable**: a click on it emits `message` (a clear button,
    /// a reveal-password button…) instead of placing the caret. Implies a suffix icon
    /// (to be set through [`suffix_icon`](Self::suffix_icon)).
    pub fn on_suffix(mut self, message: Msg) -> Self {
        self.suffix_action = Some(message);
        self
    }

    /// Does the local point `(x, y)` land on the **suffix's clickable zone** (on the right of
    /// the input box)? Used both to route the click and to keep the caret out of it.
    fn suffix_hit(&self, local_x: f32, local_y: f32, width: f32) -> bool {
        if self.suffix.is_none() {
            return false;
        }
        let zone_left = width - (self.icon_size() + ICON_PAD * 2.0);
        let top = self.label_block();
        let bottom = top + self.field_height();
        local_x >= zone_left && local_y >= top && local_y <= bottom
    }

    /// Label displayed above the field.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Hint displayed inside the field while it is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Helper text below the field (replaced by the error, if present).
    pub fn helper(mut self, helper: impl Into<String>) -> Self {
        self.helper = Some(helper.into());
        self
    }

    /// Marks the field **in error**: red border and label, `message` displayed below
    /// the field. Chain it only when the application's validation fails.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the font size, in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// The most characters the field will hold.
    ///
    /// It is **enforced**, as the reference's default is: a keystroke past the limit does
    /// not reach the value, and neither does the tail of a paste that would cross it —
    /// the part that fits still lands, because dropping a whole paste for being one
    /// character too long loses work the user can see they had.
    ///
    /// It also puts a counter — `5/10` — at the end of the line below the box, where the
    /// reference puts one, and reserves that line even when there is no helper text to
    /// share it with.
    ///
    /// The limit is in **characters**, not bytes: "é" is one, and a name in an alphabet
    /// that is not Latin is counted the way its writer would count it.
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// What each typed character becomes before it reaches the value: `None` drops it,
    /// `Some(c)` puts `c` in instead of what was typed.
    ///
    /// The reference's `inputFormatters` reshape the **whole value** after every
    /// keystroke, which buys grouping (spaces every four digits of a card number) and
    /// costs a caret: the formatter has to say where it went, and getting that wrong
    /// puts the cursor in the middle of a group the reader did not touch. A filter that
    /// works one character at a time cannot group, and cannot lose the caret either
    /// — a dropped character simply never arrives, and a substituted one takes the same
    /// place. That covers digits-only, letters-only, no-spaces and forced case, which is
    /// nearly every field that filters at all.
    ///
    /// It applies to what is **typed or pasted**, and a paste keeps whatever it can
    /// rather than being refused whole — the same rule
    /// [`max_length`](TextField::max_length) follows, for the same reason.
    ///
    /// A value the **caller** supplied is left alone: it is the application's state, not
    /// something typed, and rewriting it would be editing a value nobody edited.
    ///
    /// ```ignore
    /// TextField::new(&app.code).input_filter(|c| c.to_uppercase().next())
    /// TextField::new(&app.tag).input_filter(|c| (!c.is_whitespace()).then_some(c))
    /// ```
    pub fn input_filter(mut self, filter: impl Fn(char) -> Option<char> + 'static) -> Self {
        self.input_filter = Some(Box::new(filter));
        self
    }

    /// Digits, and nothing else.
    ///
    /// It also asks for a **number keypad**, unless one was named already: a field that
    /// refuses everything but digits and then opens a QWERTY keyboard is a field whose
    /// keys mostly do nothing. Naming a keyboard before or after this leaves that
    /// choice standing, so the two builders can be written in either order.
    pub fn digits_only(mut self) -> Self {
        self.input_filter = Some(Box::new(|c| c.is_ascii_digit().then_some(c)));
        self.keyboard = self.keyboard.or(Some(KeyboardType::Number));
        self
    }

    /// Which letters the software keyboard capitalises by itself.
    ///
    /// A **hint**: it is what the keyboard does to what it sends, and a reader who turns
    /// it off still types what they meant to. A field that must hold capitals wants
    /// [`input_filter`](TextField::input_filter) as well, which works whatever the
    /// keyboard does and works on a desktop, where there is no keyboard to hint to.
    pub fn capitalization(mut self, capitalization: Capitalization) -> Self {
        self.capitalization = capitalization;
        self
    }

    /// A field whose value can be read, selected and copied — but not changed.
    ///
    /// It is **not** [`enabled(false)`](TextField::enabled), and the difference matters.
    /// A disabled field is greyed out and inert: out of the tab order, no caret, nothing
    /// to select. A read-only one looks and behaves like any other field except that
    /// typing does nothing — you can focus it, move the caret through it, select a
    /// reference number and copy it. That is what an identifier the application generated
    /// wants, and greying it out would suggest it is unavailable rather than fixed.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Closure producing a message from the field's new value.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Message emitted on submission (the Enter key), without changing the value.
    pub fn on_submit(mut self, message: Msg) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// The **displayed** string: masked (one dot per character) for a password field,
    /// the value as is otherwise. Same character count as the value → the caret and
    /// the hit-test stay aligned index for index.
    fn display(&self) -> String {
        if self.obscure {
            OBSCURE_CHAR.to_string().repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    /// The **displayed** string shaped once: caret by index, hit-test, selection — one
    /// consistent geometry (kerning). `wrap_width` = the soft-wrap width (multi-line) or
    /// `None` (single-line: only explicit `\n` break).
    fn layout(&self, wrap_width: Option<f32>) -> TextLayout {
        let style = self.text_style();
        TextLayout::wrapped(
            &self.display(),
            style.size,
            style.weight,
            style.italic,
            wrap_width,
        )
    }

    /// Width reserved for the prefix icon (0 if there is none).
    fn prefix_w(&self) -> f32 {
        if self.prefix.is_some() {
            self.icon_size() + ICON_PAD
        } else {
            0.0
        }
    }

    /// Width reserved for the suffix icon (0 if there is none).
    fn suffix_w(&self) -> f32 {
        if self.suffix.is_some() {
            self.icon_size() + ICON_PAD
        } else {
            0.0
        }
    }

    /// Text width (between the padding and the icons) for a given widget width.
    fn content_width(&self, width: f32) -> f32 {
        (width - (self.pad_x() + self.prefix_w()) - self.pad_x() - self.suffix_w()).max(0.0)
    }

    /// Height reserved for the label above the box (0 if there is no label). In
    /// `outlined` mode the floating label straddles the top border: only its **upper
    /// half** must be reserved (the rest bites into the box), instead of a full band.
    fn label_block(&self) -> f32 {
        if self.label.is_some() {
            if self.is_outlined() {
                (frus_text::line_height(self.label_size()) * 0.5).ceil()
            } else {
                frus_text::line_height(self.label_size()) + FIELD_GAP
            }
        } else {
            0.0
        }
    }

    /// The counter shown at the end of the line below the box, when there is a limit.
    fn counter(&self) -> Option<String> {
        self.max_length
            .map(|max| format!("{}/{}", self.value.chars().count(), max))
    }

    /// Height reserved for the helper/error line below the box (0 if there is none).
    fn sub_block(&self) -> f32 {
        if self.error.is_some() || self.helper.is_some() || self.max_length.is_some() {
            // The helper style's **own** line, not one recomputed from its size: another
            // of milestone 412's survivors, written against a bare constant rather than a
            // style, which is the one formulation that sweep could not find.
            self.sub_style().line_height() + FIELD_GAP
        } else {
            0.0
        }
    }

    /// Height of the input box itself (decoration excluded): one line in single-line
    /// mode, `rows` lines in multi-line mode.
    fn field_height(&self) -> f32 {
        let lines = if self.multiline {
            self.rows.max(1) as f32
        } else {
            1.0
        };
        (self.text_style().line_height() * lines + self.text_top() + self.pad_bottom()).ceil()
    }
}

impl<Msg: Clone> Widget<Msg> for TextField<Msg> {
    fn style(&self) -> Style {
        let height = self.label_block() + self.field_height() + self.sub_block();
        Style {
            width: self.width,
            height: Dimension::Length(height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let s = self.resolved(theme);
        // A disabled field keeps 38% of everything, following the reference, and never
        // shows a focus: it cannot be focused, so a focus ring left over from before it
        // was disabled would be a lie about what Tab will do.
        let o = if self.enabled {
            status.opacity
        } else {
            status.opacity * FIELD_DISABLED_OPACITY
        };
        let has_error = self.error.is_some() && self.enabled;
        let fp = if self.enabled {
            status.focus_progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        // **An errored field deepens under the pointer** (`input_decorator.dart:5981`,
        // `:6004`, `:6053`): `error` at rest, `on_error_container` while hovered — and
        // back to `error` once focused, because the reference tests focus **before**
        // hover and a focused field is already saying everything it can.
        //
        // Continuous where the reference is discrete, which is this framework's habit
        // with the pointer: `hover_progress` is a progression, not a flag.
        let error_ink = s.error_color.unwrap().lerp(s.error_hover_color.unwrap(), {
            let hover = if self.enabled {
                status.hover_progress.clamp(0.0, 1.0)
            } else {
                0.0
            };
            hover * (1.0 - fp)
        });

        // Decoration: label above, input box in the middle, helper/error below. The
        // box is the sub-rectangle where all the editing lives.
        let label_block = self.label_block();
        let field = Rect::new(
            bounds.x,
            bounds.y + label_block,
            bounds.width,
            self.field_height(),
        );

        // **Floating** label: at rest (an empty, unfocused field) it occupies the box,
        // in the hint's place; focused OR filled, it floats above, shrunk. The
        // position/size progress follows that "floating" (`t`); the **colour** follows
        // the focus (`fp`) — a filled but unfocused field keeps a discreet label, not
        // yet accented.
        let float_t = if self.value.is_empty() { fp } else { 1.0 };
        let content_left = field.x + self.pad_x() + self.prefix_w();
        // Geometry of the floating label, interpolated between **rest** (inside the box,
        // in the hint's place) and the **floated target**. That target differs with the
        // style: `outlined` → on the top border (with a notch); otherwise → a reserved band.
        let label_geom = self.label.as_ref().map(|label| {
            let rest = Point::new(content_left, field.y + self.text_top());
            let (fx, fy) = if self.is_outlined() {
                // On the top border, which the notch opens behind it.
                (
                    field.x + self.pad_x(),
                    field.y - frus_text::line_height(self.label_size()) * 0.5,
                )
            } else {
                // Inside the container, in the room reserved above the content — a filled
                // field never spills outside its own box.
                (content_left, field.y + self.pad_top())
            };
            let x = rest.x + (fx - rest.x) * float_t;
            let y = rest.y + (fy - rest.y) * float_t;
            let resolved = self.text_style().size;
            let size = resolved + (self.label_size() - resolved) * float_t;
            let color = if has_error {
                error_ink
            } else {
                s.label_color
                    .unwrap()
                    .lerp(s.focused_label_color.unwrap(), fp)
            };
            (label.clone(), x, y, size, color)
        });
        // Helper/error line below the box (the error takes precedence over the helper).
        let sub = self.error.as_ref().or(self.helper.as_ref());
        if let Some(sub) = sub {
            // The message itself does **not** deepen: `errorStyle` is `error` in every
            // state (`input_decorator.dart:6100`). It is a sentence, not a control, and
            // a sentence that changed colour under the pointer would be claiming to be
            // one.
            let color = if has_error {
                s.error_color.unwrap()
            } else {
                s.helper_color.unwrap()
            };
            scene.text(
                Point::new(bounds.x, field.y + field.height + FIELD_GAP),
                sub.clone(),
                &self.sub_style(),
                color.fade(o),
            );
        }
        // The counter shares that line, pushed to its far end — the reference's place for
        // it, and the one that leaves the helper text where it was. It takes the helper's
        // colour rather than the error's even while an error is showing: it is a fact
        // about length, not a second complaint.
        if let Some(counter) = self.counter() {
            let width = frus_text::measure_resolved(&counter, &self.sub_style()).width;
            scene.text(
                Point::new(
                    bounds.x + bounds.width - width,
                    field.y + field.height + FIELD_GAP,
                ),
                counter,
                &self.sub_style(),
                s.helper_color.unwrap().fade(o),
            );
        }

        // The border follows the focus in **two** ways, as the reference does: the colour
        // moves to the accent and the weight goes from 1 to 2. Either alone is legible;
        // both together is what makes a focused field unmistakable without relying on
        // colour, which not every reader has.
        let border_color = if has_error {
            error_ink
        } else {
            s.border_color
                .unwrap()
                .lerp(s.focused_border_color.unwrap(), fp)
        }
        .fade(o);
        let rest_w = s.border_width.unwrap();
        let border_width = rest_w + (s.focused_border_width.unwrap() - rest_w) * fp;
        let radius = s.radius.unwrap();
        let fill = s.fill.unwrap();
        if self.is_borderless() {
            // **Nothing.** The container belongs to whatever this field was put inside.
        } else if self.is_outlined() {
            scene.draw_rect(field, fill.fade(o), radius, border_width, border_color);
        } else {
            // Filled: a container with its **top** corners rounded, and a single line
            // under it. A box all the way round would be the other variant wearing a fill.
            scene.draw_rect(
                field,
                fill.fade(o),
                frus_core::BorderRadius {
                    top_left: radius,
                    top_right: radius,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                0.0,
                frus_core::Color::TRANSPARENT,
            );
            scene.fill_rect(
                Rect::new(
                    field.x,
                    field.y + field.height - border_width,
                    field.width,
                    border_width,
                ),
                border_color,
            );
        }

        // The label goes **after** the box, in both styles. Floated, it sits above the
        // box and the order would not matter; at rest it sits *inside* it, in the
        // hint's place, over an opaque surface. It used to be painted first and
        // survived only because the renderer drew all text above everything — which it
        // stopped doing in milestone 295, and the golden went blank.
        if !self.is_outlined() {
            if let Some((label, x, y, size, color)) = &label_geom {
                // `exact`: `size` is already an interpolation between two **resolved**
                // numbers, so resolving it again would apply the reader's setting twice.
                let style = ResolvedTextStyle::exact(*size);
                scene.text(Point::new(*x, *y), label.clone(), &style, color.fade(o));
            }
        }

        // Outlined: the label's **notch**. The border segment behind the floating label
        // is masked by a flat surface-coloured fill, then the label is painted on top.
        // The notch only opens as the label rises (`float_t`).
        if self.is_outlined() {
            if let Some((label, x, y, size, color)) = &label_geom {
                if float_t > 0.01 {
                    let label_w = frus_text::measure(label, self.label_size()).width;
                    let notch = Rect::new(
                        *x - FIELD_NOTCH_GAP,
                        field.y - (border_width + FIELD_NOTCH_GAP) * 0.5,
                        label_w + FIELD_NOTCH_GAP * 2.0,
                        border_width + FIELD_NOTCH_GAP,
                    );
                    scene.fill_rect(notch, theme.surface.fade(o * float_t));
                }
                scene.text(
                    Point::new(*x, *y),
                    label.clone(),
                    &ResolvedTextStyle::exact(*size),
                    color.fade(o),
                );
            }
        }

        // Decorative icons, vertically centred in the box (a discreet colour).
        let icon_color = s.icon_color.unwrap().fade(o);
        let icon_y = field.y + (field.height - self.icon_size()) * 0.5;
        if let Some(prefix) = self.prefix {
            let path = prefix.placed(
                self.icon_size(),
                field.x + ICON_PAD,
                icon_y,
                theme.direction,
            );
            scene.fill_path(&path, icon_color);
        }
        if let Some(suffix) = self.suffix {
            let x = field.x + field.width - self.icon_size() - ICON_PAD;
            // Highlight (milestone 208): a discreet halo behind the **clickable** suffix when
            // the pointer hovers it (the absolute position is brought back to local through
            // `bounds`). Purely visual.
            if self.suffix_action.is_some() {
                if let Some(hc) = status.hover_cursor {
                    if self.suffix_hit(hc.x - bounds.x, hc.y - bounds.y, bounds.width) {
                        let halo = Rect::new(
                            x - 4.0,
                            icon_y - 4.0,
                            self.icon_size() + 8.0,
                            self.icon_size() + 8.0,
                        );
                        scene.draw_rect(
                            halo,
                            theme.muted.fade(o * 0.18),
                            halo.height * 0.5,
                            0.0,
                            frus_core::Color::TRANSPARENT,
                        );
                    }
                }
            }
            let path = suffix.placed(self.icon_size(), x, icon_y, theme.direction);
            scene.fill_path(&path, icon_color);
        }

        let len = self.value.chars().count();
        // The content is inset between the prefix/suffix icons, where there are any.
        let left = self.pad_x() + self.prefix_w();
        let content_x = field.x + left;
        let content_w = self.content_width(field.width);
        let text_y = field.y + self.text_top();
        // Multi-line: the text **wraps** at the content width — the same `max_width` for
        // the measure (caret/hit) and for the render → identical wraps.
        let wrap = if self.multiline {
            Some(content_w)
        } else {
            None
        };
        let layout = self.layout(wrap);
        // The alignment's push, worked out once and used by the placeholder, the text,
        // the selection, the underline and the caret — every one of which is drawn from
        // `text_x` below.
        let align = self.align_offset(content_w, layout.size().width);

        // Hint (placeholder): displayed when the field is empty. If there is a label as
        // well, the hint only reveals itself (fading in) once the label has floated —
        // otherwise the two would overlap inside the box.
        if self.value.is_empty() {
            if let Some(placeholder) = &self.placeholder {
                let ph_alpha = if self.label.is_some() { o * fp } else { o };
                if ph_alpha > 0.01 {
                    // The placeholder sits where the text will: a centred field whose
                    // hint hugs the left edge jumps the moment the first key lands.
                    let style = self.text_style();
                    let hint_w = frus_text::measure_resolved(placeholder, &style).width;
                    let hint_align = self.align_offset(content_w, hint_w);
                    scene.text(
                        Point::new(content_x + hint_align, text_y),
                        placeholder.clone(),
                        &style,
                        theme.muted.fade(ph_alpha),
                    );
                }
            }
        }

        // Scrolling to keep the caret visible: horizontal (always) and vertical
        // (multi-line). Recomputed from the cursor, as on click (`cursor_at`).
        let cursor = status.cursor.unwrap_or(len).min(len);
        let caret = layout.caret_rect(cursor);
        let scroll = if status.focused {
            (caret.x - content_w).max(0.0)
        } else {
            0.0
        };
        let text_x = content_x + align - scroll;
        // **Retained** vertical scroll (wheel/scrollbar, caret-following by the shell);
        // clamped to how far the content overflows the box.
        let vscroll = if self.multiline {
            let overflow = (layout.size().height
                - (field.height - self.text_top() - self.pad_bottom()))
            .max(0.0);
            status.scroll_y.clamp(0.0, overflow)
        } else {
            0.0
        };
        // Vertical origin of the content (offset by the multi-line scroll).
        let text_top = text_y - vscroll;

        // Clipped to the **content** frame — inside the padding, not the whole box.
        // Clipping to the box let scrolled multi-line text ride up onto the top border,
        // which is where an outlined field's floating label lives: the third line of a
        // scrolled field was painted straight over the label naming it. It only became
        // visible once the padding grew to the reference's, which is the useful kind of
        // regression — the old 6 px hid it rather than avoided it.
        let content_clip = scene.current_clip().intersect(Rect::new(
            content_x,
            field.y + self.text_top(),
            content_w,
            (field.height - self.text_top() - self.pad_bottom()).max(0.0),
        ));
        scene.set_clip(content_clip);

        // Selection highlight (below the text).
        if status.focused {
            if let Some((start, end)) = status.selection {
                for r in layout.selection_rects(start.min(len), end.min(len)) {
                    scene.fill_rect(
                        Rect::new(text_x + r.x, text_top + r.y, r.width, r.height),
                        theme.selection.fade(o),
                    );
                }
            }
        }

        if !self.value.is_empty() {
            let pos = Point::new(text_x, text_top);
            let color = theme.on_surface.fade(o);
            match wrap {
                // Multi-line: the render wraps just as the measure did.
                Some(max_w) => {
                    scene.text_wrapped(pos, self.display(), &self.text_style(), color, max_w)
                }
                None => scene.text(pos, self.display(), &self.text_style(), color),
            }
        }

        // The IME **composition** region: underlined (provisional text). Drawn over the
        // text, below the baseline.
        if status.focused {
            if let Some((start, end)) = status.composing {
                let (start, end) = (start.min(len), end.min(len));
                for r in layout.selection_rects(start, end) {
                    scene.fill_rect(
                        Rect::new(text_x + r.x, text_top + r.y + r.height - 1.5, r.width, 1.5),
                        theme.on_surface.fade(o * 0.7),
                    );
                }
            }
        }

        // Curseur.
        if status.focused {
            scene.fill_rect(
                Rect::new(text_x + caret.x, text_top + caret.y, 2.0, caret.height),
                theme.on_surface.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn positional_click(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        _height: f32,
    ) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        if self.suffix_action.is_some() && self.suffix_hit(local_x, local_y, width) {
            self.suffix_action.clone()
        } else {
            None
        }
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        _height: f32,
    ) -> Option<crate::interaction::Cursor> {
        // A hand over the **active** suffix icon (clear / reveal); elsewhere, no opinion.
        if self.suffix_action.is_some() && self.suffix_hit(local_x, local_y, width) {
            Some(crate::interaction::Cursor::Pointer)
        } else {
            None
        }
    }

    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        // A disabled field cannot be focused, so this should be unreachable — but a key
        // arriving from a stale focus must not edit a value the caller has declared
        // untouchable.
        if !self.enabled {
            return None;
        }
        let mut chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let mut cursor = edit.cursor.min(len);
        let mut anchor = edit.anchor.map(|a| a.min(len));
        let selection = anchor
            .map(|a| (a.min(cursor), a.max(cursor)))
            .filter(|(s, e)| s < e);

        let mut changed = false;

        match key {
            Key::Text(text) => {
                // The filter, one character at a time: a rejected one never arrives and
                // a substituted one takes the same place, so the caret arithmetic below
                // is the arithmetic it always was.
                let inserted: Vec<char> = match &self.input_filter {
                    Some(filter) => text.chars().filter_map(filter).collect(),
                    None => text.chars().collect(),
                };
                // Nothing typed and nothing selected is nothing done: emitting the value
                // unchanged would rebuild the tree for a keystroke that was refused.
                // A selection **is** something done, even when what replaces it is
                // empty, since the reference filters the value after the replacement.
                if selection.is_some() || !inserted.is_empty() {
                    if let Some((s, e)) = selection {
                        chars.drain(s..e);
                        cursor = s;
                    }
                    let n = inserted.len();
                    chars.splice(cursor..cursor, inserted);
                    cursor += n;
                    anchor = None;
                    changed = true;
                }
            }
            Key::Backspace => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                    changed = true;
                } else if cursor > 0 {
                    chars.remove(cursor - 1);
                    cursor -= 1;
                    changed = true;
                }
                anchor = None;
            }
            Key::Delete => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                    changed = true;
                } else if cursor < len {
                    chars.remove(cursor);
                    changed = true;
                }
                anchor = None;
            }
            Key::Left { shift, word } => {
                let target = if *word {
                    word_boundary_left(&chars, cursor)
                } else {
                    cursor.saturating_sub(1)
                };
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            Key::Right { shift, word } => {
                let target = if *word {
                    word_boundary_right(&chars, cursor)
                } else {
                    (cursor + 1).min(len)
                };
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            // Ctrl (`doc`): the bounds of the whole field; otherwise the bounds of the
            // logical line (identical in a single-line field).
            Key::Home { shift, doc } => {
                let target = if *doc { 0 } else { line_start(&chars, cursor) };
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            Key::End { shift, doc } => {
                let target = if *doc { len } else { line_end(&chars, cursor) };
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            // Escape is none of editing's business (routed leaf→root by the shell).
            Key::Escape => {}
            Key::Enter if self.multiline => {
                // Multi-line: Enter **inserts a line break** (no submission).
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                }
                chars.insert(cursor, '\n');
                cursor += 1;
                anchor = None;
                changed = true;
            }
            Key::Enter => {
                // Single-line: Enter submits — it does not alter the value, it emits the submission.
                edit.cursor = cursor;
                edit.anchor = anchor;
                return self.on_submit.clone();
            }
        }

        // Read-only: everything that only **moves** has already happened -- the caret,
        // the selection, the word jumps -- and is kept. What is refused is the change,
        // which is the whole of the difference between this and a disabled field, where
        // there is no caret to move in the first place.
        if self.read_only && changed {
            return None;
        }

        // The limit, enforced the way the reference enforces it: what fits lands and the
        // rest does not. Dropping a whole paste for being one character too long would
        // lose work the user can see they had, and refusing the keystroke that crosses
        // the limit is the same rule seen one character at a time.
        //
        // A value the **caller** supplied over the limit is left alone: it is the
        // application's state, not something typed, and silently shortening it would be
        // editing a value nobody edited. The counter shows it over.
        if changed {
            if let Some(max) = self.max_length {
                if chars.len() > max {
                    chars.truncate(max);
                    cursor = cursor.min(max);
                    anchor = anchor.map(|a| a.min(max));
                }
            }
        }

        edit.cursor = cursor;
        edit.anchor = anchor;

        if changed {
            let new_value: String = chars.into_iter().collect();
            self.on_input.as_ref().map(|make| make(new_value))
        } else {
            None
        }
    }

    fn ime(&self) -> Ime {
        self.ime_options()
    }

    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        scroll_cursor: usize,
    ) -> Option<usize> {
        // A disabled field takes no caret: it is readable, not editable, and a caret
        // sitting in it would promise otherwise.
        if !self.enabled {
            return None;
        }
        // A click on the **clickable suffix icon**: do not place a caret (the shell will
        // emit its message through `positional_click`).
        if self.suffix_action.is_some() && self.suffix_hit(local_x, local_y, width) {
            return None;
        }
        // Rebuilds the same content geometry as the render (icon and decoration insets,
        // wrapping, horizontal scroll), for an exact click. The **retained vertical
        // scroll** is already folded into `local_y` by the shell.
        let left = self.pad_x() + self.prefix_w();
        let content_w = self.content_width(width);
        let layout = self.layout(if self.multiline {
            Some(content_w)
        } else {
            None
        });
        let scroll = (layout.caret_rect(scroll_cursor).x - content_w).max(0.0);
        // The same push the paint applied, from the same function. Without it a click on
        // centred or right-aligned text lands wherever the glyphs *would* have been
        // against the left edge, which is a caret appearing several characters from the
        // tap.
        let align = self.align_offset(content_w, layout.size().width);
        // `local_*` are relative to the **widget's** top-left corner (label included):
        // the label band and the padding are removed to land inside the text.
        let target_x = local_x - left - align + scroll;
        let target_y = local_y - self.label_block() - self.text_top();
        Some(layout.hit_test(Point::new(target_x, target_y)))
    }

    fn text_metrics(&self, width: f32, cursor: usize) -> Option<(f32, f32, f32, f32)> {
        if !self.multiline {
            return None;
        }
        let layout = self.layout(Some(self.content_width(width)));
        let caret = layout.caret_rect(cursor);
        let visible = self.field_height() - self.text_top() - self.pad_bottom();
        Some((layout.size().height, visible, caret.y, caret.height))
    }

    fn text_viewport(&self, rect: Rect) -> Option<Rect> {
        if !self.multiline {
            return None;
        }
        // The input box: below the label, as tall as the `rows`.
        Some(Rect::new(
            rect.x,
            rect.y + self.label_block(),
            rect.width,
            self.field_height(),
        ))
    }

    fn caret_vertical(
        &self,
        width: f32,
        cursor: usize,
        down: bool,
        page: bool,
        goal_x: Option<f32>,
    ) -> Option<(usize, f32)> {
        if !self.multiline {
            return None;
        }
        let layout = self.layout(Some(self.content_width(width)));
        let caret = layout.caret_rect(cursor);
        let line_h = caret.height;
        // Wanted column: the remembered target, otherwise the current column.
        let x = goal_x.unwrap_or(caret.x);
        // A step of one line, or of one page (the field's visible height, at least 1 line).
        let step = if page {
            (self.field_height() - self.text_top() - self.pad_bottom()).max(line_h)
        } else {
            line_h
        };
        let center = caret.y + line_h * 0.5;
        let target_y = if down { center + step } else { center - step };
        let full = layout.size().height;
        if page {
            // Page: clamped to the field (it is never left).
            let clamped = target_y.clamp(0.0, (full - line_h * 0.5).max(0.0));
            Some((layout.hit_test(Point::new(x, clamped)), x))
        } else {
            // Line: already on the first/last one → the shell moves the focus.
            if target_y < 0.0 || target_y >= full {
                return None;
            }
            Some((layout.hit_test(Point::new(x, target_y)), x))
        }
    }

    fn text_value(&self) -> Option<&str> {
        Some(&self.value)
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let mut s = frus_core::SemanticsProperties::new(frus_core::Role::TextInput)
            .value(self.value.clone());
        // The label names the field; an error is appended to it so it gets read out.
        let label = match (&self.label, &self.error) {
            (Some(l), Some(e)) => Some(format!("{l}, {e}")),
            (Some(l), None) => Some(l.clone()),
            (None, Some(e)) => Some(e.clone()),
            (None, None) => None,
        };
        if let Some(label) = label {
            s = s.label(label);
        }
        if !self.enabled {
            s = s.disabled(true);
        }
        Some(s)
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let cursor = edit.cursor.min(len);
        let anchor = edit.anchor?.min(len);
        let (start, end) = (anchor.min(cursor), anchor.max(cursor));
        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        if len == 0 {
            return Some((0, 0));
        }
        let i = index.min(len - 1);
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        if !is_word(chars[i]) {
            return Some((i, (i + 1).min(len)));
        }
        let mut start = i;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = i + 1;
        while end < len && is_word(chars[end]) {
            end += 1;
        }
        Some((start, end))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn draws_own_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::Icons;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Changed(String),
        Submitted,
    }

    fn input(value: &str) -> TextField<Msg> {
        TextField::new(value).on_input(Msg::Changed)
    }

    /// Every rectangle a field paints, in order.
    fn rects(field: &TextField<Msg>, theme: &Theme, w: f32, h: f32) -> Vec<frus_core::Primitive> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            field,
            Rect::new(0.0, 0.0, w, h),
            Status::default(),
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, frus_core::Primitive::Rect { .. }))
            .cloned()
            .collect()
    }

    /// The reference's content padding is `(12, 8, 12, 8)` filled and `(12, 20, 12, 12)`
    /// outlined, and the outlined asymmetry is the room its floating label takes on the
    /// border. A field measured any other way is a field that does not line up beside
    /// anything else drawn to the same specification.
    #[test]
    fn a_field_is_the_references_size() {
        let line = input("x").text_style().line_height();
        let outlined = input("x");
        assert_eq!(
            outlined.field_height(),
            (line + FIELD_OUTLINED_PADDING_TOP + FIELD_OUTLINED_PADDING_BOTTOM).ceil()
        );
        // A filled field is tighter — until it carries a label, which it floats *inside*
        // the box and therefore has to make room for.
        let filled = input("x").filled();
        assert_eq!(filled.field_height(), (line + FIELD_PADDING_Y * 2.0).ceil());
        let labelled = input("x").filled().label("Name");
        assert_eq!(
            labelled.field_height() - filled.field_height(),
            (FIELD_GAP + FIELD_TEXT_SIZE * FIELD_LABEL_SCALE).ceil()
        );
        // The label shrinks by a proportion, not to a fixed size: a field given larger
        // text keeps the relationship.
        assert_eq!(input("x").size(32.0).label_size(), 24.0);
    }

    /// A field is 56 px tall, which is right for a form and wrong inside a table row.
    /// `dense` is the reference's answer, and it gives back the padding without changing
    /// the shape, the label or the border.
    #[test]
    fn a_dense_field_gives_back_its_padding() {
        let line = input("x").text_style().line_height();
        assert_eq!(
            input("x").dense(true).field_height(),
            (line + FIELD_DENSE_OUTLINED_PADDING_TOP + FIELD_DENSE_OUTLINED_PADDING_BOTTOM).ceil()
        );
        assert_eq!(
            input("x").filled().dense(true).field_height(),
            (line + FIELD_DENSE_PADDING_Y * 2.0).ceil()
        );
        // It is only the room: a dense field is shorter than its roomy twin in both
        // variants, and never taller.
        for dense in [input("x").dense(true), input("x").filled().dense(true)] {
            let roomy = if dense.is_outlined() {
                input("x")
            } else {
                input("x").filled()
            };
            assert!(dense.field_height() < roomy.field_height());
        }
    }

    /// A filled field is a container with **one line under it**; an outlined one is a box.
    /// Drawing a fill inside a stroked box on all four sides would be the other variant
    /// wearing a tint, which is the mistake this variant exists to avoid.
    #[test]
    fn a_filled_field_has_a_line_not_a_box() {
        let theme = Theme::default();
        let stroked = |field: &TextField<Msg>| {
            rects(field, &theme, 220.0, 80.0)
                .iter()
                .filter(|p| {
                    matches!(p, frus_core::Primitive::Rect { border_width, .. } if *border_width > 0.0)
                })
                .count()
        };
        assert_eq!(
            stroked(&input("x").outlined()),
            1,
            "outlined: one stroked box"
        );
        assert_eq!(stroked(&input("x").filled()), 0, "filled: nothing stroked");
        // ...and the line is a filled rectangle the width of the field, at its foot.
        let filled = input("x").filled();
        let height = filled.field_height();
        let has_underline = rects(&filled, &theme, 220.0, 80.0).iter().any(|p| {
            matches!(
                p,
                frus_core::Primitive::Rect { rect, border_width, .. }
                    if *border_width == 0.0
                        && (rect.width - 220.0).abs() < 0.5
                        && (rect.height - FIELD_BORDER_WIDTH).abs() < 0.5
                        && (rect.y + rect.height - height).abs() < 0.5
            )
        });
        assert!(has_underline, "a filled field is underlined");
    }

    /// A disabled field keeps 38% of its colours and shows no focus — it cannot be
    /// focused, so a ring left over from before would say the wrong thing about Tab.
    #[test]
    fn a_disabled_field_dims_and_never_looks_focused() {
        let theme = Theme::default();
        let focused = Status {
            focus_progress: 1.0,
            ..Default::default()
        };
        let border = |field: &TextField<Msg>, status: Status| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                field,
                Rect::new(0.0, 0.0, 220.0, 80.0),
                status,
                &theme,
                &mut scene,
            );
            scene.primitives().iter().find_map(|p| match p {
                frus_core::Primitive::Rect {
                    border_width,
                    border_color,
                    ..
                } if *border_width > 0.0 => Some((*border_width, *border_color)),
                _ => None,
            })
        };
        let (live_w, live_c) = border(&input("x"), focused).unwrap();
        assert_eq!(
            live_w, FIELD_FOCUSED_BORDER_WIDTH,
            "focus widens the border"
        );
        assert_eq!(live_c.a, 1.0, "and it is fully opaque");
        let (dead_w, dead_c) = border(&input("x").enabled(false), focused).unwrap();
        assert_eq!(dead_w, FIELD_BORDER_WIDTH, "disabled: no focus weight");
        assert!(
            (dead_c.a - FIELD_DISABLED_OPACITY).abs() < 0.01,
            "disabled: dimmed to the reference's 38%, got {}",
            dead_c.a
        );
    }

    /// Scrolled multi-line text must stay **inside the padding**, not merely inside the
    /// box: the top border is where an outlined field's floating label lives, and text
    /// clipped only to the box was painted straight over the label naming it.
    #[test]
    fn scrolled_text_is_clipped_below_the_label() {
        let theme = Theme::default();
        let field = input("one\ntwo\nthree\nfour\nfive\nsix")
            .rows(3)
            .label("Notes");
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &field,
            Rect::new(0.0, 0.0, 220.0, 140.0),
            Status {
                scroll_y: 40.0,
                ..Default::default()
            },
            &theme,
            &mut scene,
        );
        let box_top = field.label_block();
        let clip = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                // The label and the helper are painted unclipped; the value is not, and
                // its clip is the one under test.
                frus_core::Primitive::Text { clip, .. } if clip.y > -1.0e6 => Some(*clip),
                _ => None,
            })
            .expect("the value is clipped");
        assert!(
            clip.y >= box_top + field.text_top() - 0.5,
            "the clip starts below the padding, not at the border: {} < {}",
            clip.y,
            box_top + field.text_top()
        );
    }

    /// Greying a field out is not enough: it has to be out of the tab order, refuse a
    /// caret, and ignore a key that reaches it from a focus set before it was disabled.
    #[test]
    fn a_disabled_field_is_inert() {
        let live = input("hello");
        let dead = input("hello").enabled(false);
        assert!(Widget::<Msg>::focusable(&live));
        assert!(!Widget::<Msg>::focusable(&dead), "out of the tab order");
        assert!(
            Widget::<Msg>::cursor_at(&live, 40.0, 30.0, 220.0, 0).is_some(),
            "a live field takes a caret"
        );
        assert_eq!(
            Widget::<Msg>::cursor_at(&dead, 40.0, 30.0, 220.0, 0),
            None,
            "a disabled field takes none"
        );
        let mut edit = Edit::default();
        assert!(
            Widget::on_edit(&dead, &mut edit, &Key::Text("x".into())).is_none(),
            "and no key edits it"
        );
        let semantics = Widget::<Msg>::semantics(&dead).unwrap();
        assert!(semantics.disabled, "a reader is told it is disabled");
    }

    /// `caller ?? theme ?? framework`, on a field that has all three to choose from.
    #[test]
    fn the_caller_outranks_the_theme_which_outranks_the_framework() {
        let radius = |field: &TextField<Msg>, theme: &Theme| {
            rects(field, theme, 220.0, 80.0)
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect {
                        border_width,
                        radius,
                        ..
                    } if *border_width > 0.0 => Some(radius.top_left),
                    _ => None,
                })
                .unwrap()
        };
        let plain = Theme::default();
        assert_eq!(radius(&input("x"), &plain), FIELD_RADIUS);
        let mut themed = Theme::default();
        themed.widgets.text_field.radius = Some(10.0);
        assert_eq!(radius(&input("x"), &themed), 10.0);
        let overridden = input("x").style(TextFieldStyle {
            radius: Some(2.0),
            ..Default::default()
        });
        assert_eq!(radius(&overridden, &themed), 2.0);
    }

    #[test]
    fn text_is_clipped_to_content_box() {
        // Text longer than the field must be clipped to its content width (otherwise it
        // overflows onto the neighbouring widgets).
        let inp = input("a very long value that overflows the field width").width(100.0);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &inp,
            Rect::new(0.0, 0.0, 100.0, 30.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let clip = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { clip, .. } => Some(*clip),
                _ => None,
            })
            .expect("text primitive");
        assert!(
            (clip.width - (100.0 - FIELD_PADDING_X * 2.0)).abs() < 0.5,
            "clip = {clip:?}"
        );
    }

    #[test]
    fn composing_region_draws_an_underline() {
        // The composed region adds thin rectangles (the underline) below the text,
        // absent when no composition is in progress.
        let inp = input("konnichiwa");
        let base = Status {
            focused: true,
            cursor: Some(5),
            ..Default::default()
        };
        let composing = Status {
            composing: Some((0, 5)),
            ..base
        };

        let count_thin_rects = |status: Status| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &inp,
                Rect::new(0.0, 0.0, 220.0, 30.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.height <= 2.0 && rect.width > 2.0))
                .count()
        };
        let plain = count_thin_rects(base);
        let underlined = count_thin_rects(composing);
        assert!(
            underlined > plain,
            "the composition must underline ({plain} → {underlined})"
        );
    }

    #[test]
    fn text_value_exposes_the_field_content() {
        // The input context (IME suggestions) reads the field's value.
        let inp = input("hello");
        assert_eq!(Widget::<Msg>::text_value(&inp), Some("hello"));
    }

    #[test]
    fn cursor_at_accounts_for_scroll() {
        // A narrow field + long text → scrolling once the caret is at the end.
        let inp = input("0123456789 abcdefghij");
        // Without scrolling (cursor 0): a click on the left edge → index 0.
        assert_eq!(
            Widget::<Msg>::cursor_at(&inp, FIELD_PADDING_X, FIELD_OUTLINED_PADDING_TOP, 80.0, 0),
            Some(0)
        );
        // Scrolled (caret at the end): a click on the right lands on a larger index than
        // a click on the left (the offset is indeed taken into account).
        let end = inp.value.chars().count();
        let at_left =
            Widget::<Msg>::cursor_at(&inp, FIELD_PADDING_X, FIELD_OUTLINED_PADDING_TOP, 80.0, end)
                .unwrap();
        let at_right =
            Widget::<Msg>::cursor_at(&inp, 76.0, FIELD_OUTLINED_PADDING_TOP, 80.0, end).unwrap();
        assert!(at_left > 0, "scrolled: the left edge is no longer index 0");
        assert!(at_right > at_left, "a click on the right → a larger index");
    }

    #[test]
    fn insert_at_cursor() {
        let inp = input("ac");
        let mut edit = Edit {
            cursor: 1,
            anchor: None,
            composing: None,
        };
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Text("b".to_string())),
            Some(Msg::Changed("abc".to_string()))
        );
        assert_eq!(edit.cursor, 2);
    }

    #[test]
    fn shift_arrow_selects_then_delete() {
        let inp = input("hello");
        // Caret at the end, Shift+Left twice -> selects "lo".
        let mut edit = Edit {
            cursor: 5,
            anchor: None,
            composing: None,
        };
        inp.on_edit(
            &mut edit,
            &Key::Left {
                shift: true,
                word: false,
            },
        );
        inp.on_edit(
            &mut edit,
            &Key::Left {
                shift: true,
                word: false,
            },
        );
        assert_eq!(edit.selection_range(), Some((3, 5)));
        // Backspace deletes the selection.
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Backspace),
            Some(Msg::Changed("hel".to_string()))
        );
        assert_eq!(edit.cursor, 3);
        assert_eq!(edit.anchor, None);
    }

    #[test]
    fn home_end_bounds() {
        let inp = input("abc");
        let mut edit = Edit {
            cursor: 1,
            anchor: None,
            composing: None,
        };
        inp.on_edit(
            &mut edit,
            &Key::End {
                shift: false,
                doc: false,
            },
        );
        assert_eq!(edit.cursor, 3);
        inp.on_edit(
            &mut edit,
            &Key::Home {
                shift: false,
                doc: false,
            },
        );
        assert_eq!(edit.cursor, 0);
    }

    #[test]
    fn ctrl_arrow_jumps_by_word() {
        let inp = input("foo bar baz");
        // From the end, Ctrl+Left jumps to the start of "baz", then "bar", then "foo".
        let mut edit = Edit {
            cursor: 11,
            anchor: None,
            composing: None,
        };
        inp.on_edit(
            &mut edit,
            &Key::Left {
                shift: false,
                word: true,
            },
        );
        assert_eq!(edit.cursor, 8, "start of \"baz\"");
        inp.on_edit(
            &mut edit,
            &Key::Left {
                shift: false,
                word: true,
            },
        );
        assert_eq!(edit.cursor, 4, "start of \"bar\"");
        // Ctrl+Right jumps to the end of "bar", then "baz".
        inp.on_edit(
            &mut edit,
            &Key::Right {
                shift: false,
                word: true,
            },
        );
        assert_eq!(edit.cursor, 7, "fin de \"bar\"");
        inp.on_edit(
            &mut edit,
            &Key::Right {
                shift: false,
                word: true,
            },
        );
        assert_eq!(edit.cursor, 11, "fin de \"baz\"");
    }

    #[test]
    fn home_end_are_line_relative_but_ctrl_spans_the_field() {
        // "ab\ncd\nef": caret in the middle of the 2nd line (index 4, between c and d).
        let inp = TextField::<Msg>::new("ab\ncd\nef")
            .on_input(Msg::Changed)
            .rows(3);
        let mut edit = Edit {
            cursor: 4,
            anchor: None,
            composing: None,
        };
        // Plain Home → the start of the current line (index 3), not of the field.
        inp.on_edit(
            &mut edit,
            &Key::Home {
                shift: false,
                doc: false,
            },
        );
        assert_eq!(edit.cursor, 3, "start of the 2nd line");
        // Plain End → the end of the current line (index 5).
        inp.on_edit(
            &mut edit,
            &Key::End {
                shift: false,
                doc: false,
            },
        );
        assert_eq!(edit.cursor, 5, "end of the 2nd line");
        // Ctrl+Home / Ctrl+End → the bounds of the whole field.
        inp.on_edit(
            &mut edit,
            &Key::Home {
                shift: false,
                doc: true,
            },
        );
        assert_eq!(edit.cursor, 0, "start of the field");
        inp.on_edit(
            &mut edit,
            &Key::End {
                shift: false,
                doc: true,
            },
        );
        assert_eq!(edit.cursor, 8, "end of the field");
    }

    #[test]
    fn selected_text_reads_range() {
        let inp = input("hello");
        let edit = Edit {
            cursor: 5,
            anchor: Some(2),
            composing: None,
        };
        assert_eq!(inp.selected_text(&edit), Some("llo".to_string()));
    }

    #[test]
    fn enter_submits_without_changing_value() {
        let inp = input("acheter du lait").on_submit(Msg::Submitted);
        let mut edit = Edit {
            cursor: 3,
            anchor: None,
            composing: None,
        };
        // Enter: emits the submission, returns no value change.
        assert_eq!(inp.on_edit(&mut edit, &Key::Enter), Some(Msg::Submitted));
        assert_eq!(edit.cursor, 3); // caret unchanged
    }

    #[test]
    fn enter_without_submit_is_noop() {
        let inp = input("x");
        let mut edit = Edit {
            cursor: 1,
            anchor: None,
            composing: None,
        };
        assert_eq!(inp.on_edit(&mut edit, &Key::Enter), None);
    }

    /// Where the text was actually drawn: the x of the field's own text primitive.
    fn drawn_text_x(field: &TextField<()>, width: f32) -> f32 {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            field,
            Rect::new(0.0, 0.0, width, 56.0),
            Status {
                focused: false,
                ..Default::default()
            },
            &Theme::default(),
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { position, text, .. } if text == "42" => {
                    Some(position.x)
                }
                _ => None,
            })
            .expect("the value is drawn")
    }

    /// A field says nothing, and the text starts where it always did.
    #[test]
    fn a_field_that_says_nothing_starts_at_the_left() {
        let plain: TextField<()> = TextField::new("42");
        let started: TextField<()> = TextField::new("42").text_align(TextAlign::Start);
        assert_eq!(drawn_text_x(&plain, 300.0), drawn_text_x(&started, 300.0));
    }

    /// Centred and right-aligned text moves, and moves the right way round.
    #[test]
    fn the_text_sits_where_it_was_told_to() {
        let left = drawn_text_x(&TextField::<()>::new("42"), 300.0);
        let centre = drawn_text_x(
            &TextField::<()>::new("42").text_align(TextAlign::Center),
            300.0,
        );
        let right = drawn_text_x(
            &TextField::<()>::new("42").text_align(TextAlign::Right),
            300.0,
        );
        assert!(left < centre, "centred is further right than left-aligned");
        assert!(centre < right, "right-aligned is further right again");
        // Centred is halfway: the two gaps either side match.
        assert!(
            ((centre - left) - (right - centre)).abs() < 0.5,
            "halfway: {left} {centre} {right}"
        );
    }

    /// **The click has to agree with the paint.** A caret placed from the unaligned
    /// geometry would appear several characters from the tap, and it is the kind of
    /// wrongness nobody reports precisely — it just feels broken.
    #[test]
    fn a_click_lands_where_the_glyphs_are() {
        let width = 300.0;
        for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
            let field: TextField<()> = TextField::new("42").text_align(align);
            let x = drawn_text_x(&field, width);
            // A tap a hair to the left of the first glyph belongs before it\u2026
            let before = Widget::<()>::cursor_at(&field, x - 1.0, 20.0, width, 0);
            assert_eq!(before, Some(0), "{align:?}: before the first character");
            // \u2026and one past the last glyph belongs after it.
            let text_w = frus_text::measure("42", field.size).width;
            let after = Widget::<()>::cursor_at(&field, x + text_w + 1.0, 20.0, width, 0);
            assert_eq!(after, Some(2), "{align:?}: after the last character");
        }
    }

    /// Nothing to distribute, nothing to push. A line wider than its field keeps its
    /// left edge, which is where reading starts and where the horizontal scroll brings
    /// the caret back to.
    #[test]
    fn text_wider_than_the_field_is_not_pushed_off_it() {
        let long: TextField<()> = TextField::new("42").text_align(TextAlign::Right);
        // A box far too narrow for the value: the slack is negative, so the push is nil.
        assert_eq!(long.align_offset(4.0, 40.0), 0.0);
        assert_eq!(long.align_offset(40.0, 40.0), 0.0);
    }

    /// A multi-line field stays at the start. Aligning wrapped text means moving each
    /// line by its own width, which the caret and the click would have to be told about
    /// line by line — that belongs inside the text layout, not in a widget nudging a
    /// block sideways behind their backs.
    #[test]
    fn a_multiline_field_is_not_aligned() {
        let note: TextField<()> = TextField::new("42")
            .multiline()
            .text_align(TextAlign::Center);
        assert_eq!(note.align_offset(300.0, 40.0), 0.0);
        // The same field on one line would have moved.
        let one_line: TextField<()> = TextField::new("42").text_align(TextAlign::Center);
        assert_eq!(one_line.align_offset(300.0, 40.0), 130.0);
    }

    /// The placeholder sits where the text will. A centred field whose hint hugs the
    /// left edge jumps the moment the first key lands.
    #[test]
    fn the_placeholder_is_aligned_like_the_value() {
        let hint_x = |align: TextAlign| {
            let field: TextField<()> = TextField::new("").placeholder("Amount").text_align(align);
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &field,
                Rect::new(0.0, 0.0, 300.0, 56.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { position, text, .. } if text == "Amount" => {
                        Some(position.x)
                    }
                    _ => None,
                })
                .expect("the hint is drawn")
        };
        assert!(hint_x(TextAlign::Right) > hint_x(TextAlign::Center));
        assert!(hint_x(TextAlign::Center) > hint_x(TextAlign::Left));
    }

    /// A filtered field drops what it will not take, and keeps the rest of a paste.
    #[test]
    fn a_filter_drops_what_it_will_not_take() {
        let field: TextField<String> = TextField::new("").digits_only().on_input(|v| v);
        let mut edit = Edit::default();
        // A paste of a formatted number: the digits land, the spaces and dashes do not.
        let out = Widget::on_edit(&field, &mut edit, &Key::Text("12 34-56".into()));
        assert_eq!(out.as_deref(), Some("123456"));
        assert_eq!(edit.cursor, 6, "the caret is past what actually arrived");
    }

    /// A refused keystroke is not an edit: emitting the value unchanged would rebuild
    /// the tree for a key that did nothing.
    #[test]
    fn a_refused_keystroke_says_nothing() {
        let field: TextField<String> = TextField::new("42").digits_only().on_input(|v| v);
        let mut edit = Edit {
            cursor: 2,
            ..Default::default()
        };
        assert_eq!(
            Widget::on_edit(&field, &mut edit, &Key::Text("x".into())),
            None
        );
        assert_eq!(edit.cursor, 2, "and the caret has not moved");
    }

    /// A **selection** replaced by nothing is still an edit: the selection is gone.
    #[test]
    fn a_refused_keystroke_over_a_selection_still_clears_it() {
        let field: TextField<String> = TextField::new("1234").digits_only().on_input(|v| v);
        let mut edit = Edit {
            cursor: 3,
            anchor: Some(1),
            ..Default::default()
        };
        let out = Widget::on_edit(&field, &mut edit, &Key::Text("x".into()));
        assert_eq!(out.as_deref(), Some("14"));
        assert_eq!(edit.cursor, 1);
    }

    /// A filter can **substitute** as well as drop, and the caret arithmetic is the same
    /// because a substituted character takes the place of the one typed.
    #[test]
    fn a_filter_can_substitute() {
        let field: TextField<String> = TextField::new("")
            .input_filter(|c| c.to_uppercase().next())
            .on_input(|v| v);
        let mut edit = Edit::default();
        let out = Widget::on_edit(&field, &mut edit, &Key::Text("ab".into()));
        assert_eq!(out.as_deref(), Some("AB"));
        assert_eq!(edit.cursor, 2);
    }

    /// A value the **caller** supplied is left alone: it is the application's state, not
    /// something typed.
    #[test]
    fn a_filter_does_not_rewrite_what_the_caller_gave() {
        let field: TextField<String> = TextField::new("a1b2").digits_only().on_input(|v| v);
        let mut edit = Edit {
            cursor: 4,
            ..Default::default()
        };
        let out = Widget::on_edit(&field, &mut edit, &Key::Text("3".into()));
        assert_eq!(
            out.as_deref(),
            Some("a1b23"),
            "only the keystroke is filtered"
        );
    }

    /// Digits ask for a keypad, and a keyboard named either side of it stands.
    #[test]
    fn digits_only_asks_for_a_keypad_without_overruling_one() {
        let plain: TextField<()> = TextField::new("").digits_only();
        assert_eq!(Widget::<()>::ime(&plain).keyboard, KeyboardType::Number);
        let before: TextField<()> = TextField::new("")
            .keyboard_type(KeyboardType::Phone)
            .digits_only();
        assert_eq!(Widget::<()>::ime(&before).keyboard, KeyboardType::Phone);
        let after: TextField<()> = TextField::new("")
            .digits_only()
            .keyboard_type(KeyboardType::Phone);
        assert_eq!(Widget::<()>::ime(&after).keyboard, KeyboardType::Phone);
    }

    /// The capitalisation reaches the keyboard the field asks for.
    #[test]
    fn the_capitalisation_travels_with_the_field() {
        let field: TextField<()> = TextField::new("").capitalization(Capitalization::Characters);
        let ime = Widget::<()>::ime(&field);
        assert_eq!(ime.capitalization, Capitalization::Characters);
        assert_eq!(ime.android_input_type() & 0x0000_7000, 0x0000_1000);
    }

    /// A plain field asks for the plain keyboard, which is what every field used to
    /// get whether it suited or not.
    #[test]
    fn a_plain_field_asks_for_the_plain_keyboard() {
        let field: TextField<()> = TextField::new("hello");
        assert_eq!(
            Widget::<()>::ime(&field),
            Ime {
                keyboard: KeyboardType::Text,
                action: TextInputAction::Done,
                capitalization: Capitalization::Auto,
            }
        );
    }

    /// **A masked field is a secret, and the dots cannot say so.** `obscure` draws
    /// dots on our side; the keyboard, told nothing, treated the field as ordinary
    /// prose — learning the password into its personal dictionary and offering it
    /// back as a suggestion later, on whatever screen came next.
    #[test]
    fn an_obscured_field_tells_the_keyboard_it_is_a_secret() {
        let field: TextField<()> = TextField::new("hunter2").obscure(true);
        let ime = Widget::<()>::ime(&field);
        assert_eq!(ime.keyboard, KeyboardType::Password);
        assert!(ime.keyboard.is_secret());
        // TYPE_TEXT_VARIATION_PASSWORD, which is what stops the learning.
        assert_eq!(ime.keyboard.android_input_type() & 0x0000_0ff0, 0x80);
    }

    /// A field that takes several lines needs the action key to **insert** one.
    /// *Done* there is a keyboard that cannot type what the field is for.
    #[test]
    fn a_multiline_field_takes_a_newline_key() {
        let field: TextField<()> = TextField::new("a note").multiline();
        let ime = Widget::<()>::ime(&field);
        assert_eq!(ime.keyboard, KeyboardType::Multiline);
        assert_eq!(ime.action, TextInputAction::Newline);
        // `rows` implies multiline, so it implies the same keyboard.
        let rowed: TextField<()> = TextField::new("a note").rows(4);
        assert_eq!(Widget::<()>::ime(&rowed).keyboard, KeyboardType::Multiline);
    }

    /// What the caller says beats what the field guessed — both halves, separately.
    #[test]
    fn what_the_caller_says_wins() {
        let field: TextField<()> = TextField::new("")
            .obscure(true)
            .keyboard_type(KeyboardType::VisiblePassword);
        assert_eq!(
            Widget::<()>::ime(&field).keyboard,
            KeyboardType::VisiblePassword
        );

        // A search box asks for Search without ceasing to be a text field.
        let query: TextField<()> = TextField::new("").action(TextInputAction::Search);
        let ime = Widget::<()>::ime(&query);
        assert_eq!(ime.keyboard, KeyboardType::Text);
        assert_eq!(ime.action, TextInputAction::Search);

        // And a multi-line field can still be told to say Done.
        let note: TextField<()> = TextField::new("").multiline().action(TextInputAction::Done);
        assert_eq!(Widget::<()>::ime(&note).action, TextInputAction::Done);
    }

    /// A field that says what it is gets the keyboard for it — a keypad for a phone
    /// number, an `@` for an address.
    #[test]
    fn a_typed_field_gets_the_keyboard_for_it() {
        let phone: TextField<()> = TextField::new("").keyboard_type(KeyboardType::Phone);
        // TYPE_CLASS_PHONE
        assert_eq!(Widget::<()>::ime(&phone).keyboard.android_input_type(), 3);

        let email: TextField<()> = TextField::new("").keyboard_type(KeyboardType::Email);
        let bits = Widget::<()>::ime(&email).keyboard.android_input_type();
        // TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_EMAIL_ADDRESS, and no capitalisation:
        // `Someone@` is an address that does not work.
        assert_eq!(bits, 1 | 0x20);
    }

    #[test]
    fn obscure_masks_the_displayed_text_but_not_the_value() {
        // A password field: the render never contains the value in the clear, only dots;
        // the real value stays reachable (editing, IME).
        let theme = Theme::default();
        let field = TextField::<Msg>::new("secret").obscure(true);
        let mut scene = Scene::new();
        let status = Status {
            focused: true,
            cursor: Some(6),
            ..Default::default()
        };
        Widget::<Msg>::paint(
            &field,
            Rect::new(0.0, 0.0, 220.0, 30.0),
            status,
            &theme,
            &mut scene,
        );
        let drawn: Vec<&str> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            drawn.iter().all(|t| !t.contains("secret")),
            "the value must not leak"
        );
        assert!(
            drawn.iter().any(|t| t.chars().all(|c| c == '•')),
            "dots are drawn"
        );
        // The real value stays exposed to the input context.
        assert_eq!(Widget::<Msg>::text_value(&field), Some("secret"));
    }

    #[test]
    fn prefix_icon_draws_a_path_and_shifts_the_hit_test() {
        // A prefix icon draws a path and offsets the content to the right: the same
        // click lands on a smaller index than it would without one.
        let theme = Theme::default();
        let with_icon = input("hello world").prefix_icon(Icons::STAR);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &with_icon,
            Rect::new(0.0, 0.0, 220.0, 30.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, frus_core::Primitive::Path { .. })),
            "the prefix icon draws a path"
        );
        let plain = input("hello world");
        let at_plain =
            Widget::<Msg>::cursor_at(&plain, 60.0, FIELD_OUTLINED_PADDING_TOP, 220.0, 0).unwrap();
        let at_icon =
            Widget::<Msg>::cursor_at(&with_icon, 60.0, FIELD_OUTLINED_PADDING_TOP, 220.0, 0)
                .unwrap();
        assert!(
            at_icon < at_plain,
            "the prefix offsets the content ({at_plain} → {at_icon})"
        );
    }

    #[test]
    fn multiline_enter_inserts_a_newline_instead_of_submitting() {
        // In multi-line mode, Enter inserts a "\n" (and emits no submission).
        let inp = TextField::<Msg>::new("ab")
            .on_input(Msg::Changed)
            .multiline();
        let mut edit = Edit {
            cursor: 2,
            anchor: None,
            composing: None,
        };
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Enter),
            Some(Msg::Changed("ab\n".to_string()))
        );
        assert_eq!(edit.cursor, 3);
        // Single line: Enter submits (unchanged behaviour).
        let single = input("ab").on_submit(Msg::Submitted);
        let mut edit = Edit {
            cursor: 2,
            anchor: None,
            composing: None,
        };
        assert_eq!(single.on_edit(&mut edit, &Key::Enter), Some(Msg::Submitted));
    }

    #[test]
    fn multiline_wraps_long_lines_to_the_width() {
        // A long line **without** any `\n` wraps onto several visual lines: a click well
        // below the 1st line places the caret further into the text (a wrapped line under
        // the first one), not at index 0.
        let long = "word ".repeat(30); // 150 characters, no explicit break
        let inp = TextField::<Msg>::new(long.trim_end())
            .on_input(Msg::Changed)
            .rows(4)
            .width(160.0);
        let line_h = inp.text_style().line_height();
        let top = Widget::<Msg>::cursor_at(
            &inp,
            FIELD_PADDING_X + 2.0,
            FIELD_OUTLINED_PADDING_TOP + 1.0,
            160.0,
            0,
        )
        .unwrap();
        let wrapped = Widget::<Msg>::cursor_at(
            &inp,
            FIELD_PADDING_X + 2.0,
            FIELD_OUTLINED_PADDING_TOP + line_h * 2.0 + 1.0,
            160.0,
            0,
        )
        .unwrap();
        assert!(top < 10, "1st line: {top}");
        assert!(
            wrapped > top,
            "a wrapped line further down → an index further on ({top} → {wrapped})"
        );
    }

    #[test]
    fn multiline_reports_overflow_and_scrolls_content() {
        // Five lines in a box of two: the content overflows…
        let inp = TextField::<Msg>::new("l1\nl2\nl3\nl4\nl5")
            .on_input(Msg::Changed)
            .rows(2)
            .width(200.0);
        let (content_h, visible_h, _, _) =
            Widget::<Msg>::text_metrics(&inp, 200.0, 0).expect("a multi-line field");
        assert!(
            content_h > visible_h + 1.0,
            "5 lines > a box of 2 ({content_h} vs {visible_h})"
        );

        // …and a retained scroll shifts the text upwards (a smaller position.y).
        let text_top = |scroll: f32| {
            let mut scene = Scene::new();
            let status = Status {
                focused: true,
                cursor: Some(0),
                scroll_y: scroll,
                ..Default::default()
            };
            Widget::<Msg>::paint(
                &inp,
                Rect::new(0.0, 0.0, 200.0, 80.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, position, .. } if text.contains("l1") => {
                        Some(position.y)
                    }
                    _ => None,
                })
                .expect("the field's text")
        };
        assert!(
            text_top(20.0) < text_top(0.0),
            "scrolling raises the content"
        );
    }

    #[test]
    fn multiline_arrows_move_the_caret_between_lines() {
        // "abc\ndefg\nhi": line 0 = indices 0..3, line 1 = 4..8, line 2 = 9..11.
        let inp = TextField::<Msg>::new("abc\ndefg\nhi")
            .on_input(Msg::Changed)
            .rows(3)
            .width(200.0);
        // From the 1st line (index 1), moving down lands on the 2nd line.
        let (down, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 1, true, false, None).unwrap();
        assert!(
            (4..=8).contains(&down),
            "moves down to the 2nd line: {down}"
        );
        // 1st line, moving up → impossible (the shell moves the focus).
        assert_eq!(
            Widget::<Msg>::caret_vertical(&inp, 200.0, 1, false, false, None),
            None
        );
        // Last line (index 10), moving down → impossible.
        assert_eq!(
            Widget::<Msg>::caret_vertical(&inp, 200.0, 10, true, false, None),
            None
        );
        // 2nd line (index 5), moving up lands on the 1st.
        let (up, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 5, false, false, None).unwrap();
        assert!(up <= 3, "moves up to the 1st line: {up}");
        // A single-line field: never any vertical movement.
        assert_eq!(
            Widget::<Msg>::caret_vertical(&input("abc"), 200.0, 1, true, false, None),
            None
        );
    }

    #[test]
    fn multiline_goal_column_survives_a_short_line() {
        // "hello\nhi\nworld": line 0 = 0..5, line 1 = 6..8 (short), line 2 = 9..14.
        // Starting from column 5 (the end of "hello") and moving down: the 2nd line "hi"
        // is too short (so the caret is clamped to its end), but the goal column returned
        // stays ~the starting one, so moving down again lands back on column 5.
        let inp = TextField::<Msg>::new("hello\nhi\nworld")
            .on_input(Msg::Changed)
            .rows(3)
            .width(200.0);
        let (mid, goal) = Widget::<Msg>::caret_vertical(&inp, 200.0, 5, true, false, None).unwrap();
        assert!(
            (6..=8).contains(&mid),
            "a short line, clamped to its end: {mid}"
        );
        // A second jump reusing the goal column: without memory the caret would have
        // stayed clamped to the end of "hi" (col. ~2); here it lands far into "world"
        // (col. ~5), proving the original column survives the short line.
        let (low, _) =
            Widget::<Msg>::caret_vertical(&inp, 200.0, mid, true, false, Some(goal)).unwrap();
        assert!(
            (12..=14).contains(&low),
            "goal column preserved into \"world\": {low}"
        );
    }

    #[test]
    fn multiline_page_jump_is_clamped_to_the_field() {
        // Page up/down never leave the multi-line field: at the bounds the caret settles
        // at the start / at the end and returns `Some` (not `None` → the field is kept).
        let inp = TextField::<Msg>::new("a\nb\nc\nd\ne")
            .on_input(Msg::Changed)
            .rows(2)
            .width(200.0);
        // From the last line, PgDn settles at the bottom (the field is not left).
        let (bottom, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 8, true, true, None).unwrap();
        assert!(
            bottom >= 7,
            "PgDn clamped to the bottom of the field: {bottom}"
        );
        // From the 1st line, PgUp settles at the top.
        let (top, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 0, false, true, None).unwrap();
        assert!(top <= 1, "PgUp clamped to the top of the field: {top}");
    }

    #[test]
    fn multiline_reserves_rows_of_height() {
        // `rows(n)` reserves n lines; that is taller than a one-line field.
        let one = match Widget::<Msg>::style(&input("x")).height {
            Dimension::Length(h) => h,
            _ => panic!("a fixed height"),
        };
        let four = match Widget::<Msg>::style(&TextField::<Msg>::new("x").rows(4)).height {
            Dimension::Length(h) => h,
            _ => panic!("a fixed height"),
        };
        assert!(
            four > one * 2.0,
            "4 lines much taller than one ({one} → {four})"
        );
    }

    #[test]
    fn multiline_hit_test_uses_the_click_line() {
        // A click on the 2nd line places the caret inside "cd" (indices ≥ 3), not inside
        // the "ab" of the 1st line.
        let inp = TextField::<Msg>::new("ab\ncd")
            .on_input(Msg::Changed)
            .rows(3);
        let line_h = inp.text_style().line_height();
        // A top-left click → the 1st line (index ≤ 2).
        let top = Widget::<Msg>::cursor_at(
            &inp,
            FIELD_PADDING_X + 1.0,
            FIELD_OUTLINED_PADDING_TOP + 1.0,
            220.0,
            0,
        )
        .unwrap();
        // A click one line lower → the 2nd line (index ≥ 3).
        let below = Widget::<Msg>::cursor_at(
            &inp,
            FIELD_PADDING_X + 1.0,
            FIELD_OUTLINED_PADDING_TOP + line_h + 1.0,
            220.0,
            0,
        )
        .unwrap();
        assert!(top <= 2, "1st line: {top}");
        assert!(below >= 3, "2nd line: {below}");
    }

    #[test]
    fn floating_label_rests_in_box_then_floats_up() {
        // The label rests inside the box (large, low) while the field is empty and
        // unfocused, and floats above it (small, high) once focused.
        let theme = Theme::default();
        let field = TextField::<Msg>::new("").label("Name");
        let label_geo = |status: Status| -> (f32, f32) {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &field,
                Rect::new(0.0, 30.0, 220.0, 60.0),
                status,
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text {
                        text,
                        size,
                        position,
                        ..
                    } if text == "Name" => Some((*size, position.y)),
                    _ => None,
                })
                .expect("a painted label")
        };
        let (rest_size, rest_y) = label_geo(Status::default());
        let (float_size, float_y) = label_geo(Status {
            focused: true,
            focus_progress: 1.0,
            ..Default::default()
        });
        assert!(
            rest_size > float_size,
            "at rest the label is larger ({rest_size} → {float_size})"
        );
        assert!(
            float_y < rest_y,
            "once focused the label rises ({rest_y} → {float_y})"
        );
    }

    /// **An errored field deepens under the pointer** (milestone 439).
    ///
    /// `error` at rest, `on_error_container` while hovered, and `error` again once
    /// focused — the reference tests focus **before** hover
    /// (`input_decorator.dart:5977`), because a focused field is already saying
    /// everything it can. The message below it does not move: `errorStyle` is `error` in
    /// every state (`:6100`), it being a sentence rather than a control.
    ///
    /// It is the first thing here to ask the scheme for `on_error_container`, which
    /// arrived in milestone 429 with nothing wanting it.
    #[test]
    fn an_errored_field_deepens_under_the_pointer() {
        let theme = Theme::default();
        let field = input("x").label("Name").error("Required").outlined();
        let painted = |status: Status| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &field,
                Rect::new(0.0, 0.0, 220.0, 90.0),
                status,
                &theme,
                &mut scene,
            );
            scene
        };
        // The border is the only stroked rectangle a field paints.
        let border = |status: Status| {
            painted(status)
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect {
                        border_color,
                        border_width,
                        ..
                    } if *border_width > 0.0 => Some(*border_color),
                    _ => None,
                })
                .expect("an outlined field draws a border")
        };
        let message = |status: Status| {
            painted(status)
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, color, .. } if text == "Required" => {
                        Some(*color)
                    }
                    _ => None,
                })
                .expect("the message is painted")
        };
        let rest = Status {
            opacity: 1.0,
            ..Default::default()
        };
        let hovered = Status {
            hover_progress: 1.0,
            ..rest
        };
        let both = Status {
            focused: true,
            focus_progress: 1.0,
            ..hovered
        };

        assert_eq!(border(rest), theme.scheme.error);
        assert_eq!(
            border(hovered),
            theme.scheme.on_error_container,
            "it deepens under the pointer"
        );
        assert_eq!(
            border(both),
            theme.scheme.error,
            "and comes back once focused, which the reference tests first"
        );
        assert_ne!(
            theme.scheme.error, theme.scheme.on_error_container,
            "the two have to differ for any of the above to mean anything"
        );

        for status in [rest, hovered, both] {
            assert_eq!(
                message(status),
                theme.scheme.error,
                "the message is a sentence, not a control"
            );
        }
    }

    #[test]
    fn decoration_grows_height_for_label_and_error() {
        // A bare field reserves only the box; a label + an error add one line above and
        // one below → the style is taller.
        let bare_h = match Widget::<Msg>::style(&input("x")).height {
            Dimension::Length(h) => h,
            _ => panic!("a fixed height was expected"),
        };
        let decorated = input("x").label("Email").error("Required");
        let deco_h = match Widget::<Msg>::style(&decorated).height {
            Dimension::Length(h) => h,
            _ => panic!("a fixed height was expected"),
        };
        assert!(
            deco_h > bare_h,
            "label + error grow the field ({bare_h} → {deco_h})"
        );
    }

    #[test]
    fn error_paints_the_border_in_the_error_color() {
        // In error, the box's border switches to the theme's error colour.
        let theme = Theme::default();
        let field = input("x").error("bad");
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &field,
            Rect::new(0.0, 0.0, 220.0, 60.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        let has_error_border = scene.primitives().iter().any(|p| {
            matches!(
                p,
                frus_core::Primitive::Rect { border_color, border_width, .. }
                    if *border_width > 0.0 && *border_color == theme.error
            )
        });
        assert!(has_error_border, "the border must be in the error colour");
    }

    #[test]
    fn placeholder_shows_only_when_empty() {
        // The hint only appears while the value is empty.
        let theme = Theme::default();
        let count_texts = |value: &str| {
            let field = TextField::<Msg>::new(value).placeholder("Type here");
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &field,
                Rect::new(0.0, 0.0, 220.0, 30.0),
                Status::default(),
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(
                    |p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "Type here"),
                )
                .count()
        };
        assert_eq!(count_texts(""), 1, "empty: the hint shows");
        assert_eq!(count_texts("hi"), 0, "rempli : pas d'indice");
    }

    #[test]
    fn clickable_suffix_emits_and_blocks_caret() {
        let field = TextField::new("hello")
            .suffix_icon(Icons::CLOSE)
            .on_suffix(Msg::Submitted)
            .width(220.0);
        let (w, y) = (220.0, 12.0);
        let x_suffix = w - 8.0; // near the right edge (the suffix's zone)

        // A click on the suffix: emits the message, and does NOT place a caret.
        assert_eq!(
            Widget::<Msg>::positional_click(&field, x_suffix, y, w, 40.0),
            Some(Msg::Submitted)
        );
        assert_eq!(Widget::<Msg>::cursor_at(&field, x_suffix, y, w, 0), None);
        // A click in the body: no suffix, a caret is placed.
        assert_eq!(
            Widget::<Msg>::positional_click(&field, 20.0, y, w, 40.0),
            None
        );
        assert!(Widget::<Msg>::cursor_at(&field, 20.0, y, w, 0).is_some());
        // Without `on_suffix` the icon stays decorative (no positional click).
        let deco = TextField::<Msg>::new("hello")
            .suffix_icon(Icons::CLOSE)
            .width(220.0);
        assert_eq!(
            Widget::<Msg>::positional_click(&deco, x_suffix, y, w, 40.0),
            None
        );
    }

    #[test]
    fn hovering_active_suffix_paints_a_halo() {
        let field = TextField::new("hello")
            .suffix_icon(Icons::CLOSE)
            .on_suffix(Msg::Submitted)
            .width(220.0);
        let bounds = Rect::new(0.0, 0.0, 220.0, 40.0);
        // Counts the ~28x28 rectangles (the halo behind the suffix icon).
        let halos = |status: Status| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(&field, bounds, status, &Theme::default(), &mut scene);
            scene
                .primitives()
                .iter()
                .filter(|p| match p {
                    frus_core::Primitive::Rect { rect, .. } => {
                        (rect.width - (FIELD_ICON_SIZE + 8.0)).abs() < 0.5
                            && (rect.height - (FIELD_ICON_SIZE + 8.0)).abs() < 0.5
                    }
                    _ => false,
                })
                .count()
        };
        // Pointer over the suffix → one halo; elsewhere (the body) or without hover → none.
        let over_suffix = Status {
            hover_cursor: Some(Point::new(212.0, 20.0)),
            ..Default::default()
        };
        let over_body = Status {
            hover_cursor: Some(Point::new(20.0, 20.0)),
            ..Default::default()
        };
        assert_eq!(halos(over_suffix), 1, "a halo on the hovered suffix");
        assert_eq!(halos(over_body), 0, "no halo in the body");
        assert_eq!(halos(Status::default()), 0, "no halo without hover");
    }

    #[test]
    fn cursor_icon_is_pointer_over_active_suffix() {
        use crate::interaction::Cursor;
        let field = TextField::new("hello")
            .suffix_icon(Icons::CLOSE)
            .on_suffix(Msg::Submitted)
            .width(220.0);
        let (w, h, y) = (220.0, 40.0, 12.0);
        // A hand over the suffix, nothing in the body.
        assert_eq!(
            Widget::<Msg>::cursor_icon(&field, w - 8.0, y, w, h),
            Some(Cursor::Pointer)
        );
        assert_eq!(Widget::<Msg>::cursor_icon(&field, 20.0, y, w, h), None);
        // A decorative suffix (no on_suffix): no hand.
        let deco = TextField::<Msg>::new("hello")
            .suffix_icon(Icons::CLOSE)
            .width(220.0);
        assert_eq!(Widget::<Msg>::cursor_icon(&deco, w - 8.0, y, w, h), None);
    }

    #[test]
    fn word_at_finds_word_bounds() {
        let inp = input("hello there world");
        // Any index inside a word yields that word's bounds; separators are excluded.
        assert_eq!(inp.word_at(2), Some((0, 5))); // "hello"
        assert_eq!(inp.word_at(13), Some((12, 17))); // "world"
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;
    use crate::runtime::Edit;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Changed(String),
    }

    fn field(value: &str) -> TextField<Msg> {
        TextField::new(value).on_input(Msg::Changed)
    }

    /// Types `text` at the end of the field's value and returns what came back.
    fn typed(field: &TextField<Msg>, text: &str) -> (Option<Msg>, Edit) {
        let end = field.value.chars().count();
        let mut edit = Edit {
            cursor: end,
            anchor: None,
            ..Default::default()
        };
        let msg = Widget::on_edit(field, &mut edit, &Key::Text(text.to_string()));
        (msg, edit)
    }

    fn value_of(msg: Option<Msg>) -> Option<String> {
        msg.map(|Msg::Changed(v)| v)
    }

    /// A keystroke past the limit does not reach the value.
    #[test]
    fn a_keystroke_past_the_limit_goes_nowhere() {
        let full = field("abcde").max_length(5);
        assert_eq!(value_of(typed(&full, "f").0), Some("abcde".into()));
        let room = field("abcd").max_length(5);
        assert_eq!(value_of(typed(&room, "e").0), Some("abcde".into()));
    }

    /// A paste that crosses the limit lands the part that fits rather than being dropped
    /// whole: losing work the user can see they had is the worse of the two answers.
    #[test]
    fn a_paste_that_crosses_the_limit_lands_what_fits() {
        let f = field("ab").max_length(5);
        let (msg, edit) = typed(&f, "cdefgh");
        assert_eq!(value_of(msg), Some("abcde".into()));
        assert_eq!(edit.cursor, 5, "the caret comes to rest at the end");
    }

    /// The limit is in **characters**, not bytes.
    #[test]
    fn the_limit_counts_characters_not_bytes() {
        let f = field("").max_length(3);
        assert_eq!(value_of(typed(&f, "éàü").0), Some("éàü".into()));
        let full = field("éàü").max_length(3);
        assert_eq!(value_of(typed(&full, "e").0), Some("éàü".into()));
    }

    /// A value the caller supplied over the limit is left alone — it is the application's
    /// state, not something typed — and the counter says so.
    #[test]
    fn a_value_the_caller_set_over_the_limit_is_left_alone() {
        let over = field("abcdefg").max_length(5);
        assert_eq!(over.counter().as_deref(), Some("7/5"));
        assert_eq!(over.value, "abcdefg");
    }

    /// The counter reserves the line below the box even with no helper to share it with.
    #[test]
    fn the_counter_reserves_the_line_it_sits_on() {
        assert_eq!(
            field("ab").sub_block(),
            0.0,
            "nothing to say, no room taken"
        );
        assert!(field("ab").max_length(5).sub_block() > 0.0);
        assert_eq!(field("ab").max_length(5).counter().as_deref(), Some("2/5"));
    }

    /// Read-only refuses the **change** and keeps everything that only moves.
    #[test]
    fn read_only_refuses_the_change_and_keeps_the_caret() {
        let f = field("hello").read_only();
        assert_eq!(typed(&f, "x").0, None, "typing goes nowhere");

        let mut edit = Edit {
            cursor: 5,
            anchor: None,
            ..Default::default()
        };
        assert_eq!(
            Widget::on_edit(
                &f,
                &mut edit,
                &Key::Left {
                    shift: false,
                    word: false
                }
            ),
            None
        );
        assert_eq!(edit.cursor, 4, "but the caret still moves");

        let mut edit = Edit {
            cursor: 5,
            anchor: None,
            ..Default::default()
        };
        Widget::on_edit(
            &f,
            &mut edit,
            &Key::Home {
                shift: true,
                doc: false,
            },
        );
        assert_eq!(
            (edit.cursor, edit.anchor),
            (0, Some(5)),
            "and a selection can still be made, so it can be copied"
        );
    }

    /// Backspace is a change, so it is refused too — and leaves the caret alone.
    #[test]
    fn read_only_refuses_a_deletion() {
        let f = field("hello").read_only();
        let mut edit = Edit {
            cursor: 5,
            anchor: None,
            ..Default::default()
        };
        assert_eq!(Widget::on_edit(&f, &mut edit, &Key::Backspace), None);
        assert_eq!(edit.cursor, 5);
    }

    /// It is not a disabled field: it keeps its caret and its place in the tab order.
    #[test]
    fn read_only_is_not_disabled() {
        let read = field("REF-4417").read_only();
        assert!(Widget::<Msg>::focusable(&read), "still in the tab order");
        assert!(
            Widget::<Msg>::cursor_at(&read, 20.0, 10.0, 200.0, 0).is_some(),
            "and it still takes a caret, so a reference can be selected and copied"
        );

        let dead = field("REF-4417").enabled(false);
        assert!(!Widget::<Msg>::focusable(&dead));
        assert!(Widget::<Msg>::cursor_at(&dead, 20.0, 10.0, 200.0, 0).is_none());
    }
}
