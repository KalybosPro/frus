//! [`TextInput`]: a single-line input field, **controlled** (its value comes from
//! the application state), with a caret, navigation and selection.
//!
//! The value is controlled; the **caret / selection** are edit state retained at
//! runtime ([`Edit`]), keyed by widget identity.

use frus_core::{FontWeight, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};
use frus_text::TextLayout;

use crate::icons::IconName;
use crate::interaction::{Key, Status};
use crate::runtime::Edit;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 6.0;

/// Font size of the label (above the field) and of the helper/error text (below)
/// — the decoration's "secondary" typography.
const LABEL_SIZE: f32 = 13.0;
const SUB_SIZE: f32 = 12.0;
/// Vertical gap between the label, the input box and the helper/error line.
const DECO_GAP: f32 = 4.0;
/// Margin around the floating label inside the border's **notch** (`outlined` mode).
const NOTCH_GAP: f32 = 4.0;

/// Side of a prefix/suffix icon (logical px) and the margin around it.
const ICON_SIZE: f32 = 20.0;
const ICON_PAD: f32 = 6.0;
/// Default masking character of a password field.
const OBSCURE_CHAR: char = '•';

/// A single-line text input field, with optional **form decoration** (label, hint,
/// helper text, error). **Validity** stays decided by the application (a pure
/// function of the state); the field only displays its result through [`error`].
///
/// [`error`]: TextInput::error
pub struct TextInput<Msg> {
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
    /// Decorative icon on the left inside the box.
    prefix: Option<IconName>,
    /// Decorative icon on the right inside the box.
    suffix: Option<IconName>,
    /// Message emitted on a **click on the suffix icon** (a clear / reveal button…). Makes
    /// the suffix clickable: a click there emits this message instead of placing the caret.
    suffix_action: Option<Msg>,
    /// **Multi-line** field: Enter inserts a line break (instead of submitting), the box
    /// is `rows` lines tall and scrolls vertically to follow the caret.
    multiline: bool,
    /// Number of visible lines in multi-line mode.
    rows: u16,
    /// **Outlined** style: the floating label sits **on** the top border, which opens a
    /// **notch** behind it. Otherwise (the default), the label floats in a band reserved
    /// above the box.
    outlined: bool,
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

impl<Msg> TextInput<Msg> {
    /// Creates a field displaying `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            size: 18.0,
            width: Dimension::Length(220.0),
            on_input: None,
            on_submit: None,
            label: None,
            placeholder: None,
            helper: None,
            error: None,
            obscure: false,
            prefix: None,
            suffix: None,
            suffix_action: None,
            multiline: false,
            rows: 3,
            outlined: false,
        }
    }

    /// **Outlined** style: the floating label sits on the top border, which opens a
    /// notch behind it.
    pub fn outlined(mut self) -> Self {
        self.outlined = true;
        self
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
    pub fn obscure(mut self, obscure: bool) -> Self {
        self.obscure = obscure;
        self
    }

    /// Decorative icon on the left inside the field.
    pub fn prefix_icon(mut self, icon: IconName) -> Self {
        self.prefix = Some(icon);
        self
    }

    /// Decorative icon on the right inside the field.
    pub fn suffix_icon(mut self, icon: IconName) -> Self {
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
        let zone_left = width - (ICON_SIZE + ICON_PAD * 2.0);
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
        TextLayout::wrapped(
            &self.display(),
            self.size,
            FontWeight::Regular,
            false,
            wrap_width,
        )
    }

    /// Width reserved for the prefix icon (0 if there is none).
    fn prefix_w(&self) -> f32 {
        if self.prefix.is_some() {
            ICON_SIZE + ICON_PAD
        } else {
            0.0
        }
    }

    /// Width reserved for the suffix icon (0 if there is none).
    fn suffix_w(&self) -> f32 {
        if self.suffix.is_some() {
            ICON_SIZE + ICON_PAD
        } else {
            0.0
        }
    }

    /// Text width (between the padding and the icons) for a given widget width.
    fn content_width(&self, width: f32) -> f32 {
        (width - (PAD_X + self.prefix_w()) - PAD_X - self.suffix_w()).max(0.0)
    }

    /// Height reserved for the label above the box (0 if there is no label). In
    /// `outlined` mode the floating label straddles the top border: only its **upper
    /// half** must be reserved (the rest bites into the box), instead of a full band.
    fn label_block(&self) -> f32 {
        if self.label.is_some() {
            if self.outlined {
                (frus_text::line_height(LABEL_SIZE) * 0.5).ceil()
            } else {
                frus_text::line_height(LABEL_SIZE) + DECO_GAP
            }
        } else {
            0.0
        }
    }

    /// Height reserved for the helper/error line below the box (0 if there is none).
    fn sub_block(&self) -> f32 {
        if self.error.is_some() || self.helper.is_some() {
            frus_text::line_height(SUB_SIZE) + DECO_GAP
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
        (frus_text::line_height(self.size) * lines + PAD_Y * 2.0).ceil()
    }
}

impl<Msg: Clone> Widget<Msg> for TextInput<Msg> {
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
        let o = status.opacity;
        let has_error = self.error.is_some();
        let fp = status.focus_progress.clamp(0.0, 1.0);

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
        let content_left = field.x + PAD_X + self.prefix_w();
        // Geometry of the floating label, interpolated between **rest** (inside the box,
        // in the hint's place) and the **floated target**. That target differs with the
        // style: `outlined` → on the top border (with a notch); otherwise → a reserved band.
        let label_geom = self.label.as_ref().map(|label| {
            let rest = Point::new(content_left, field.y + PAD_Y);
            let (fx, fy) = if self.outlined {
                (
                    field.x + PAD_X,
                    field.y - frus_text::line_height(LABEL_SIZE) * 0.5,
                )
            } else {
                (bounds.x, bounds.y)
            };
            let x = rest.x + (fx - rest.x) * float_t;
            let y = rest.y + (fy - rest.y) * float_t;
            let size = self.size + (LABEL_SIZE - self.size) * float_t;
            let color = if has_error {
                theme.error
            } else {
                theme.muted.lerp(theme.focus, fp)
            };
            (label.clone(), x, y, size, color)
        });
        // Helper/error line below the box (the error takes precedence over the helper).
        let sub = self.error.as_ref().or(self.helper.as_ref());
        if let Some(sub) = sub {
            let color = if has_error { theme.error } else { theme.muted };
            scene.text(
                Point::new(bounds.x, field.y + field.height + DECO_GAP),
                sub.clone(),
                SUB_SIZE,
                color.fade(o),
            );
        }

        // Border animated by the focus progress (rest → focus), red when in error.
        let border_color = if has_error {
            theme.error
        } else {
            theme.border.lerp(theme.focus, fp)
        }
        .fade(o);
        let border_width = 1.0 + fp;
        scene.draw_rect(
            field,
            theme.surface.fade(o),
            theme.radius,
            border_width,
            border_color,
        );

        // The label goes **after** the box, in both styles. Floated, it sits above the
        // box and the order would not matter; at rest it sits *inside* it, in the
        // hint's place, over an opaque surface. It used to be painted first and
        // survived only because the renderer drew all text above everything — which it
        // stopped doing in milestone 295, and the golden went blank.
        if !self.outlined {
            if let Some((label, x, y, size, color)) = &label_geom {
                scene.text(Point::new(*x, *y), label.clone(), *size, color.fade(o));
            }
        }

        // Outlined: the label's **notch**. The border segment behind the floating label
        // is masked by a flat surface-coloured fill, then the label is painted on top.
        // The notch only opens as the label rises (`float_t`).
        if self.outlined {
            if let Some((label, x, y, size, color)) = &label_geom {
                if float_t > 0.01 {
                    let label_w = frus_text::measure(label, LABEL_SIZE).width;
                    let notch = Rect::new(
                        *x - NOTCH_GAP,
                        field.y - (border_width + NOTCH_GAP) * 0.5,
                        label_w + NOTCH_GAP * 2.0,
                        border_width + NOTCH_GAP,
                    );
                    scene.fill_rect(notch, theme.surface.fade(o * float_t));
                }
                scene.text(Point::new(*x, *y), label.clone(), *size, color.fade(o));
            }
        }

        // Decorative icons, vertically centred in the box (a discreet colour).
        let icon_color = theme.muted.fade(o);
        let icon_y = field.y + (field.height - ICON_SIZE) * 0.5;
        let icon_scale = ICON_SIZE / 24.0;
        if let Some(prefix) = self.prefix {
            let path = prefix
                .path()
                .scaled(icon_scale)
                .translated(field.x + ICON_PAD, icon_y);
            scene.fill_path(&path, icon_color);
        }
        if let Some(suffix) = self.suffix {
            let x = field.x + field.width - ICON_SIZE - ICON_PAD;
            // Highlight (milestone 208): a discreet halo behind the **clickable** suffix when
            // the pointer hovers it (the absolute position is brought back to local through
            // `bounds`). Purely visual.
            if self.suffix_action.is_some() {
                if let Some(hc) = status.hover_cursor {
                    if self.suffix_hit(hc.x - bounds.x, hc.y - bounds.y, bounds.width) {
                        let halo =
                            Rect::new(x - 4.0, icon_y - 4.0, ICON_SIZE + 8.0, ICON_SIZE + 8.0);
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
            let path = suffix.path().scaled(icon_scale).translated(x, icon_y);
            scene.fill_path(&path, icon_color);
        }

        let len = self.value.chars().count();
        // The content is inset between the prefix/suffix icons, where there are any.
        let left = PAD_X + self.prefix_w();
        let content_x = field.x + left;
        let content_w = self.content_width(field.width);
        let text_y = field.y + PAD_Y;
        // Multi-line: the text **wraps** at the content width — the same `max_width` for
        // the measure (caret/hit) and for the render → identical wraps.
        let wrap = if self.multiline {
            Some(content_w)
        } else {
            None
        };
        let layout = self.layout(wrap);

        // Hint (placeholder): displayed when the field is empty. If there is a label as
        // well, the hint only reveals itself (fading in) once the label has floated —
        // otherwise the two would overlap inside the box.
        if self.value.is_empty() {
            if let Some(placeholder) = &self.placeholder {
                let ph_alpha = if self.label.is_some() { o * fp } else { o };
                if ph_alpha > 0.01 {
                    scene.text(
                        Point::new(content_x, text_y),
                        placeholder.clone(),
                        self.size,
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
        let text_x = content_x - scroll;
        // **Retained** vertical scroll (wheel/scrollbar, caret-following by the shell);
        // clamped to how far the content overflows the box.
        let vscroll = if self.multiline {
            let overflow = (layout.size().height - (field.height - PAD_Y * 2.0)).max(0.0);
            status.scroll_y.clamp(0.0, overflow)
        } else {
            0.0
        };
        // Vertical origin of the content (offset by the multi-line scroll).
        let text_top = text_y - vscroll;

        // Clipped to the content frame (otherwise the text overflows onto its neighbours).
        let content_clip =
            scene
                .current_clip()
                .intersect(Rect::new(content_x, field.y, content_w, field.height));
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
                Some(max_w) => scene.text_wrapped(
                    pos,
                    self.display(),
                    &TextStyle::new(self.size),
                    color,
                    max_w,
                ),
                None => scene.text(pos, self.display(), self.size, color),
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
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                }
                let inserted: Vec<char> = text.chars().collect();
                let n = inserted.len();
                chars.splice(cursor..cursor, inserted);
                cursor += n;
                anchor = None;
                changed = true;
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

        edit.cursor = cursor;
        edit.anchor = anchor;

        if changed {
            let new_value: String = chars.into_iter().collect();
            self.on_input.as_ref().map(|make| make(new_value))
        } else {
            None
        }
    }

    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        scroll_cursor: usize,
    ) -> Option<usize> {
        // A click on the **clickable suffix icon**: do not place a caret (the shell will
        // emit its message through `positional_click`).
        if self.suffix_action.is_some() && self.suffix_hit(local_x, local_y, width) {
            return None;
        }
        // Rebuilds the same content geometry as the render (icon and decoration insets,
        // wrapping, horizontal scroll), for an exact click. The **retained vertical
        // scroll** is already folded into `local_y` by the shell.
        let left = PAD_X + self.prefix_w();
        let content_w = self.content_width(width);
        let layout = self.layout(if self.multiline {
            Some(content_w)
        } else {
            None
        });
        let scroll = (layout.caret_rect(scroll_cursor).x - content_w).max(0.0);
        // `local_*` are relative to the **widget's** top-left corner (label included):
        // the label band and the padding are removed to land inside the text.
        let target_x = local_x - left + scroll;
        let target_y = local_y - self.label_block() - PAD_Y;
        Some(layout.hit_test(Point::new(target_x, target_y)))
    }

    fn text_metrics(&self, width: f32, cursor: usize) -> Option<(f32, f32, f32, f32)> {
        if !self.multiline {
            return None;
        }
        let layout = self.layout(Some(self.content_width(width)));
        let caret = layout.caret_rect(cursor);
        let visible = self.field_height() - PAD_Y * 2.0;
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
            (self.field_height() - PAD_Y * 2.0).max(line_h)
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

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let mut s = frus_core::Semantics::new(frus_core::Role::TextInput).value(self.value.clone());
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
        true
    }

    fn draws_own_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Changed(String),
        Submitted,
    }

    fn input(value: &str) -> TextInput<Msg> {
        TextInput::new(value).on_input(Msg::Changed)
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
            (clip.width - (100.0 - PAD_X * 2.0)).abs() < 0.5,
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
            Widget::<Msg>::cursor_at(&inp, PAD_X, PAD_Y, 80.0, 0),
            Some(0)
        );
        // Scrolled (caret at the end): a click on the right lands on a larger index than
        // a click on the left (the offset is indeed taken into account).
        let end = inp.value.chars().count();
        let at_left = Widget::<Msg>::cursor_at(&inp, PAD_X, PAD_Y, 80.0, end).unwrap();
        let at_right = Widget::<Msg>::cursor_at(&inp, 76.0, PAD_Y, 80.0, end).unwrap();
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
        let inp = TextInput::<Msg>::new("ab\ncd\nef")
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

    #[test]
    fn obscure_masks_the_displayed_text_but_not_the_value() {
        // A password field: the render never contains the value in the clear, only dots;
        // the real value stays reachable (editing, IME).
        let theme = Theme::default();
        let field = TextInput::<Msg>::new("secret").obscure(true);
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
        let with_icon = input("hello world").prefix_icon(IconName::Star);
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
        let at_plain = Widget::<Msg>::cursor_at(&plain, 60.0, PAD_Y, 220.0, 0).unwrap();
        let at_icon = Widget::<Msg>::cursor_at(&with_icon, 60.0, PAD_Y, 220.0, 0).unwrap();
        assert!(
            at_icon < at_plain,
            "the prefix offsets the content ({at_plain} → {at_icon})"
        );
    }

    #[test]
    fn multiline_enter_inserts_a_newline_instead_of_submitting() {
        // In multi-line mode, Enter inserts a "\n" (and emits no submission).
        let inp = TextInput::<Msg>::new("ab")
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
        let inp = TextInput::<Msg>::new(long.trim_end())
            .on_input(Msg::Changed)
            .rows(4)
            .width(160.0);
        let line_h = frus_text::line_height(inp.size);
        let top = Widget::<Msg>::cursor_at(&inp, PAD_X + 2.0, PAD_Y + 1.0, 160.0, 0).unwrap();
        let wrapped =
            Widget::<Msg>::cursor_at(&inp, PAD_X + 2.0, PAD_Y + line_h * 2.0 + 1.0, 160.0, 0)
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
        let inp = TextInput::<Msg>::new("l1\nl2\nl3\nl4\nl5")
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
        let inp = TextInput::<Msg>::new("abc\ndefg\nhi")
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
        let inp = TextInput::<Msg>::new("hello\nhi\nworld")
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
        let inp = TextInput::<Msg>::new("a\nb\nc\nd\ne")
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
        let four = match Widget::<Msg>::style(&TextInput::<Msg>::new("x").rows(4)).height {
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
        let inp = TextInput::<Msg>::new("ab\ncd")
            .on_input(Msg::Changed)
            .rows(3);
        let line_h = frus_text::line_height(inp.size);
        // A top-left click → the 1st line (index ≤ 2).
        let top = Widget::<Msg>::cursor_at(&inp, PAD_X + 1.0, PAD_Y + 1.0, 220.0, 0).unwrap();
        // A click one line lower → the 2nd line (index ≥ 3).
        let below =
            Widget::<Msg>::cursor_at(&inp, PAD_X + 1.0, PAD_Y + line_h + 1.0, 220.0, 0).unwrap();
        assert!(top <= 2, "1st line: {top}");
        assert!(below >= 3, "2nd line: {below}");
    }

    #[test]
    fn floating_label_rests_in_box_then_floats_up() {
        // The label rests inside the box (large, low) while the field is empty and
        // unfocused, and floats above it (small, high) once focused.
        let theme = Theme::default();
        let field = TextInput::<Msg>::new("").label("Name");
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
            let field = TextInput::<Msg>::new(value).placeholder("Type here");
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
        let field = TextInput::new("hello")
            .suffix_icon(IconName::Close)
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
        let deco = TextInput::<Msg>::new("hello")
            .suffix_icon(IconName::Close)
            .width(220.0);
        assert_eq!(
            Widget::<Msg>::positional_click(&deco, x_suffix, y, w, 40.0),
            None
        );
    }

    #[test]
    fn hovering_active_suffix_paints_a_halo() {
        let field = TextInput::new("hello")
            .suffix_icon(IconName::Close)
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
                        (rect.width - (ICON_SIZE + 8.0)).abs() < 0.5
                            && (rect.height - (ICON_SIZE + 8.0)).abs() < 0.5
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
        let field = TextInput::new("hello")
            .suffix_icon(IconName::Close)
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
        let deco = TextInput::<Msg>::new("hello")
            .suffix_icon(IconName::Close)
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
