// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **A text field on the Web is edited through a real one** (milestone 711).
//!
//! The canvas is pixels: a browser has nothing in it to type into. Keys reached a field as
//! winit's key events, and that was all — no software keyboard on a phone, since nothing
//! editable ever had the focus; no composition, so no Chinese, Japanese or Korean; no
//! dictation, no autocorrect, no password manager; and an assistive technology that sets a
//! field's value had nowhere to set it. The browser does every one of those, for an element
//! it can edit.
//!
//! So while a field has the focus, the shell keeps an `<input>` (or a `<textarea>`, for a
//! field of several lines) **over it**, transparent and out of the pointer's way, with the
//! field's text, its selection, its name and its keyboard ([`frus_widgets::Ime::web`]), and
//! gives it the browser's focus. What the browser does to it comes back: after every change
//! the element's value, selection and composition are read and put in the field — through the
//! message typing would produce, as an autofill does, so a field that refuses typing refuses
//! this too — and what the application then holds is put back in the element.
//!
//! **Not inside the canvas.** The accessibility bridge's nodes are the canvas's fallback
//! content, which is never laid out — and an `<input>` with no layout takes the focus and its
//! keys, and no text: none typed, none composed, not even a selection set from a script. It
//! was tried first. The element is a sibling of the canvas.
//!
//! **Which keys are whose.** The element is outside the canvas, so winit, which listens on
//! the canvas, hears nothing typed into it. The browser keeps what only it does well or what
//! must stay native: characters (and dead keys, AltGr, composition), deletion, and the
//! clipboard's keys — a paste the browser makes itself asks for no permission. Every other key
//! is cancelled on the element and handed on to the canvas, where the shell treats it as it
//! always has: the arrows, Home and End, the page keys and Enter move and edit by the
//! application's own layout, which is what is on the screen; Tab moves the focus; Escape,
//! undo and the application's shortcuts are the application's. [`browser_edits`] says which.

use frus_widgets::Edit;

/// The editing element as the browser left it. Offsets are UTF-16 code units, the DOM's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Element {
    /// Its value.
    pub value: String,
    /// The selection's start.
    pub start: u32,
    /// The selection's end.
    pub end: u32,
    /// The selection was made backwards: the caret is at `start`.
    pub backward: bool,
    /// The text of an open composition, which ends where the selection does.
    pub composing: Option<String>,
}

/// The character index of UTF-16 offset `utf16` in `text`. An offset inside a surrogate pair
/// counts the pair as not yet reached; one past the end is the end.
pub(crate) fn char_index(text: &str, utf16: u32) -> usize {
    let mut units = 0;
    for (index, c) in text.chars().enumerate() {
        units += c.len_utf16() as u32;
        if units > utf16 {
            return index;
        }
    }
    text.chars().count()
}

/// What the field becomes from the element: its text, and where its caret, selection and
/// composition are, in characters.
pub(crate) fn read(element: &Element) -> (String, Edit) {
    let text = &element.value;
    let start = char_index(text, element.start.min(element.end));
    let end = char_index(text, element.start.max(element.end));
    let (cursor, anchor) = if element.backward {
        (start, end)
    } else {
        (end, start)
    };
    // The reference reads a composition the same way: it is the text being composed, and it
    // ends at the caret.
    let composing = element.composing.as_deref().and_then(|composing| {
        let length = composing.chars().count();
        (length > 0 && length <= end).then(|| (end - length, end))
    });
    let edit = Edit {
        affinity: Default::default(),
        cursor,
        anchor: (anchor != cursor).then_some(anchor),
        composing,
    };
    (text.clone(), edit)
}

/// Where the element's selection goes for a field holding `text` with `edit`: start, end, and
/// whether it runs backwards, in UTF-16 units. Indices past the text are its end — the field
/// may have refused what was typed, and the caret that came with it.
pub(crate) fn selection(text: &str, edit: &Edit) -> (u32, u32, bool) {
    // `take` stops at the end of the text, which is where an index past it belongs.
    let at = |index: usize| text.chars().take(index).map(char::len_utf16).sum::<usize>() as u32;
    let cursor = edit.cursor;
    match edit.anchor {
        Some(anchor) if at(anchor) != at(cursor) => (
            at(cursor.min(anchor)),
            at(cursor.max(anchor)),
            cursor < anchor,
        ),
        _ => (at(cursor), at(cursor), false),
    }
}

/// Whether the **browser** acts on this key in the editing element, rather than the shell.
///
/// `key` is the DOM's `KeyboardEvent.key`. `mac` says that Option makes characters, as it does
/// on a Mac — elsewhere Alt with a letter is a shortcut and types nothing.
pub(crate) fn browser_edits(
    key: &str,
    ctrl: bool,
    alt: bool,
    meta: bool,
    shift: bool,
    composing: bool,
    mac: bool,
) -> bool {
    // A key that belongs to an open composition, or that starts one, is the input method's.
    if composing || matches!(key, "Process" | "Dead" | "Unidentified" | "Compose") {
        return true;
    }
    let printable = key.chars().count() == 1;
    // AltGr arrives as Ctrl and Alt together on Windows, and it types: € is AltGr+E.
    let alt_gr = ctrl && alt && !meta;
    if (ctrl || meta) && !alt_gr {
        // The clipboard's keys, under both their spellings, and deleting a word.
        return matches!(
            key.to_ascii_lowercase().as_str(),
            "c" | "x" | "v" | "insert" | "backspace" | "delete"
        );
    }
    if alt && !alt_gr && !mac {
        return false;
    }
    // Shift+Insert pastes, Shift+Delete cuts.
    printable || matches!(key, "Backspace" | "Delete") || (shift && key == "Insert")
}

/// What the browser did to a field's value, as the field's own typing: the stretch of `old`
/// it replaced — `(start, end)` in characters — and the text it put there, found as the
/// longest common start and end of the two values.
pub(crate) fn splice(old: &str, new: &str) -> (usize, usize, String) {
    let old: Vec<char> = old.chars().collect();
    let new: Vec<char> = new.chars().collect();
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let room = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    let inserted = new[prefix..new.len() - suffix].iter().collect();
    (prefix, old.len() - suffix, inserted)
}

/// The message that puts what the browser left in the element, `value` with the caret state
/// `browser`, into `field` — and the caret state the field is then left with (milestone 749).
///
/// **Through the field's own typing.** The change is replayed as the text typed over the
/// stretch it replaced, so a field's filter, its length limit and its refusals apply to it as
/// they do to a key: a field that takes upper case only gets upper case, whatever the
/// browser's keyboard sent. The browser's caret, selection and composition are kept when the
/// field took the change as it came; when it took something else — fewer characters, none —
/// the caret is where the field's typing left it, and the next frame puts the field's own
/// text back in the element.
pub(crate) fn browser_change<Msg>(
    field: &dyn frus_widgets::Widget<Msg>,
    value: &str,
    browser: Edit,
) -> (Option<Msg>, Edit) {
    let Some(old) = field.text_value() else {
        return (None, browser);
    };
    if old == value {
        return (None, browser);
    }
    if !field.takes_typing() {
        return (None, Edit::default());
    }
    let (start, end, inserted) = splice(old, value);
    let typed = inserted.chars().count();
    let mut edit = Edit {
        cursor: end,
        anchor: (start < end).then_some(start),
        composing: None,
    };
    let key = if inserted.is_empty() {
        frus_widgets::Key::Backspace
    } else {
        frus_widgets::Key::Text(inserted)
    };
    let message = field.on_edit(&mut edit, &key);
    // Taken as it came — a substitution keeps the length — so the browser's caret, its
    // selection and its composition are the field's too.
    let as_it_came = message.is_some() && edit.cursor == start + typed && edit.anchor.is_none();
    (message, if as_it_came { browser } else { edit })
}

/// Where the editing element's top edge goes, in the page's CSS pixels, for a field whose top
/// is at `field_top` and whose height is `height`, on a canvas from `canvas_top` to
/// `canvas_bottom` (milestone 749).
///
/// Over the field, where an input method's candidates come up — except on a touch screen, where
/// a keyboard comes up from below and covers the bottom of the page. A browser that sees the
/// element it edits through about to be covered **pans the whole page** up to keep it in sight,
/// and the application's top bar went off the top of the screen with it. The shell answers the
/// keyboard itself, with an inset, and scrolls the field into view in its own layout; the
/// element must not give the browser a reason to as well. So on a touch screen it stays in the
/// top quarter of the canvas until the keyboard is up (`keyboard`, the CSS pixels it covers, is
/// not zero), and then within what the keyboard leaves visible.
pub(crate) fn element_top(
    field_top: f64,
    height: f64,
    canvas_top: f64,
    canvas_bottom: f64,
    keyboard: f64,
    touch: bool,
) -> f64 {
    if !touch {
        return field_top;
    }
    let lowest = if keyboard > 0.0 {
        canvas_bottom - keyboard - height
    } else {
        canvas_top + (canvas_bottom - canvas_top) / 4.0
    };
    field_top.min(lowest).max(canvas_top)
}

#[cfg(web)]
pub(crate) use web::{Editor, Field};

#[cfg(web)]
mod web {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    use frus_widgets::{Edit, Rect, WebInput, WidgetId};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{
        CompositionEvent, Document, Event, HtmlCanvasElement, HtmlElement, HtmlInputElement,
        HtmlTextAreaElement, KeyboardEvent, KeyboardEventInit,
    };
    use winit::platform::web::WindowExtWebSys;
    use winit::window::Window;

    use super::{browser_edits, selection, Element};

    /// The field being edited, as this frame has it.
    pub(crate) struct Field<'a> {
        /// The field.
        pub id: WidgetId,
        /// Where it is, in the layout's pixels.
        pub rect: Rect,
        /// What the browser is told about it.
        pub web: WebInput,
        /// Its name, for an assistive technology.
        pub label: Option<&'a str>,
        /// What it holds.
        pub text: &'a str,
        /// Its caret, selection and composition.
        pub edit: Edit,
    }

    type Changes = Rc<RefCell<Vec<(WidgetId, Element)>>>;
    type Listener = Closure<dyn FnMut(Event)>;

    /// The element over the field being edited.
    struct Live {
        id: WidgetId,
        web: WebInput,
        el: HtmlElement,
        composing: Rc<RefCell<Option<String>>>,
        _listeners: Vec<Listener>,
    }

    /// The editing element's keeper: one element at a time, over the field with the focus.
    pub(crate) struct Editor {
        document: Document,
        canvas: HtmlCanvasElement,
        window: Arc<Window>,
        live: Option<Live>,
        changes: Changes,
        mac: bool,
        /// The page is used with a finger: a keyboard will come up over it.
        touch: bool,
        _selection: Listener,
    }

    impl Editor {
        /// The keeper for `window`'s canvas; `None` when there is no canvas or no document.
        pub(crate) fn new(window: &Arc<Window>) -> Option<Self> {
            let canvas = window.canvas()?;
            let document = canvas.owner_document()?;
            let changes: Changes = Rc::default();
            let mac = web_sys::window()
                .and_then(|w| w.navigator().platform().ok())
                .is_some_and(|platform| platform.starts_with("Mac"));
            let touch = web_sys::window()
                .and_then(|w| w.match_media("(pointer: coarse)").ok().flatten())
                .is_some_and(|query| query.matches());
            // A selection moved by the browser — a Shift+arrow it kept, a double-click it
            // made, an assistive technology's — is a change like any other. Asked of the
            // document, which every browser fires it on.
            let selection = {
                let changes = changes.clone();
                let document = document.clone();
                let window = window.clone();
                Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                    let Some(active) = document.active_element() else {
                        return;
                    };
                    let Some(id) = active
                        .get_attribute("data-frus-field")
                        .and_then(|id| id.parse::<u64>().ok())
                    else {
                        return;
                    };
                    let Ok(el) = active.dyn_into::<HtmlElement>() else {
                        return;
                    };
                    changes
                        .borrow_mut()
                        .push((WidgetId::from_u64(id), state(&el, None)));
                    window.request_redraw();
                })
            };
            document
                .add_event_listener_with_callback(
                    "selectionchange",
                    selection.as_ref().unchecked_ref(),
                )
                .ok()?;
            Some(Self {
                document,
                canvas,
                window: window.clone(),
                live: None,
                changes,
                mac,
                touch,
                _selection: selection,
            })
        }

        /// The field being edited through the element, if one is.
        pub(crate) fn editing(&self) -> Option<WidgetId> {
            self.live.as_ref().map(|live| live.id)
        }

        /// The changes the browser made since the last call, oldest first.
        pub(crate) fn take_changes(&self) -> Vec<(WidgetId, Element)> {
            std::mem::take(&mut self.changes.borrow_mut())
        }

        /// Keeps the element in step with `field` — made, moved, filled and focused — or
        /// takes it away when no field is being edited. `css` is how many CSS pixels a
        /// layout pixel is.
        pub(crate) fn sync(&mut self, field: Option<Field>, css: f32) {
            let keep = matches!(
                (&self.live, &field),
                (Some(live), Some(field)) if live.id == field.id && live.web == field.web
            );
            if !keep {
                if let Some(live) = self.live.take() {
                    self.remove(live, field.is_none());
                }
            }
            let Some(field) = field else {
                return;
            };
            let fresh = self.live.is_none();
            if fresh {
                self.live = self.spawn(&field);
            }
            let Some(live) = self.live.as_ref() else {
                return;
            };
            let bounds = self.canvas.get_bounding_client_rect();
            let height = (field.rect.height * css).max(1.0);
            let top = super::element_top(
                bounds.top() + f64::from(field.rect.y * css),
                f64::from(height),
                bounds.top(),
                bounds.bottom(),
                crate::web_page::keyboard_cover_of(&self.canvas),
                self.touch,
            );
            let style = format!(
                "{STYLE}left:{}px;top:{}px;width:{}px;height:{}px;",
                bounds.left() + f64::from(field.rect.x * css),
                top,
                (field.rect.width * css).max(1.0),
                height,
            );
            if live.el.get_attribute("style").as_deref() != Some(style.as_str()) {
                live.el.set_attribute("style", &style).ok();
            }
            match field.label {
                Some(label) => live.el.set_attribute("aria-label", label).ok(),
                None => live.el.remove_attribute("aria-label").ok(),
            };
            // While the browser composes, the element is the input method's: writing to it
            // would end the composition under the person's fingers.
            if live.composing.borrow().is_none() {
                if value(&live.el) != field.text {
                    set_value(&live.el, field.text);
                }
                let (start, end, backward) = selection(field.text, &field.edit);
                let now = state(&live.el, None);
                if (now.start, now.end, now.backward && now.start != now.end)
                    != (start, end, backward)
                {
                    set_selection(&live.el, start, end, backward);
                }
            }
            // The element takes the browser's focus when the field takes the shell's, and
            // again when the focus has gone nowhere in particular: to the canvas, which winit
            // focuses on every press, so a tap in the field would leave it; to the page; to
            // the field's own node in the accessibility tree. Never from anything else — an
            // assistive technology that moved to a button of the selection bar meant to. And
            // never while this document does not have the focus at all: in a frame, that is the
            // page around it taking the focus, which from in here looks like the focus having
            // gone to the body. A press in the canvas focuses the canvas, and so the document,
            // first, so a press in the field still brings the focus back.
            let active = self.document.active_element();
            let ours = self.document.has_focus().unwrap_or(true);
            if fresh
                || (ours
                    && active
                        .as_ref()
                        .is_none_or(|active| self.adrift(active, &live.el)))
            {
                let _ = live.el.focus();
            }
        }

        /// Whether the browser's focus, on `active`, has gone nowhere in particular — and
        /// the editing element `el` should have it back.
        fn adrift(&self, active: &web_sys::Element, el: &HtmlElement) -> bool {
            if active == AsRef::<web_sys::Element>::as_ref(el) {
                return false;
            }
            let canvas: &web_sys::Element = self.canvas.as_ref();
            let body = self.document.body();
            active == canvas
                || body.is_some_and(|body| active == AsRef::<web_sys::Element>::as_ref(&body))
                || (self.canvas.contains(Some(active))
                    && active.get_attribute("role").as_deref() == Some("textbox"))
        }

        /// Builds the element for `field` and its listeners, and adds it beside the canvas.
        fn spawn(&self, field: &Field) -> Option<Live> {
            let web = field.web;
            let el: HtmlElement = if web.multiline {
                self.document
                    .create_element("textarea")
                    .ok()?
                    .dyn_into()
                    .ok()?
            } else {
                let input: HtmlInputElement = self
                    .document
                    .create_element("input")
                    .ok()?
                    .dyn_into()
                    .ok()?;
                input.set_type(web.input_type);
                input.into()
            };
            el.set_attribute("data-frus-field", &field.id.as_u64().to_string())
                .ok();
            el.set_attribute("inputmode", web.inputmode).ok();
            if let Some(hint) = web.enterkeyhint {
                el.set_attribute("enterkeyhint", hint).ok();
            }
            el.set_attribute("autocapitalize", web.autocapitalize).ok();
            let corrects = if web.corrects { "on" } else { "off" };
            el.set_attribute("autocorrect", corrects).ok();
            el.set_attribute("spellcheck", if web.corrects { "true" } else { "false" })
                .ok();
            // The page's own suggestions would come up over a field drawn elsewhere.
            el.set_attribute("autocomplete", "off").ok();

            let id = field.id;
            let composing: Rc<RefCell<Option<String>>> = Rc::default();
            let mut listeners = Vec::new();
            let mut listen = |kind: &str, listener: Listener| {
                let _ =
                    el.add_event_listener_with_callback(kind, listener.as_ref().unchecked_ref());
                listeners.push(listener);
            };
            let changed = {
                let changes = self.changes.clone();
                let composing = composing.clone();
                let window = self.window.clone();
                move |el: &HtmlElement| {
                    let open = composing.borrow().clone();
                    changes.borrow_mut().push((id, state(el, open)));
                    window.request_redraw();
                }
            };
            {
                let changed = changed.clone();
                listen(
                    "input",
                    Closure::new(move |event: Event| {
                        if let Some(el) = target(&event) {
                            changed(&el);
                        }
                    }),
                );
            }
            {
                let composing = composing.clone();
                listen(
                    "compositionstart",
                    Closure::new(move |_: Event| {
                        *composing.borrow_mut() = Some(String::new());
                    }),
                );
            }
            {
                let composing = composing.clone();
                listen(
                    "compositionupdate",
                    Closure::new(move |event: Event| {
                        let data = event
                            .dyn_ref::<CompositionEvent>()
                            .and_then(CompositionEvent::data)
                            .unwrap_or_default();
                        *composing.borrow_mut() = Some(data);
                    }),
                );
            }
            {
                let composing = composing.clone();
                let changed = changed.clone();
                listen(
                    "compositionend",
                    Closure::new(move |event: Event| {
                        *composing.borrow_mut() = None;
                        if let Some(el) = target(&event) {
                            changed(&el);
                        }
                    }),
                );
            }
            for kind in ["keydown", "keyup"] {
                let canvas = self.canvas.clone();
                let mac = self.mac;
                listen(
                    kind,
                    Closure::new(move |event: Event| {
                        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
                            return;
                        };
                        let browsers = browser_edits(
                            &key.key(),
                            key.ctrl_key(),
                            key.alt_key(),
                            key.meta_key(),
                            key.shift_key(),
                            key.is_composing(),
                            mac,
                        );
                        if browsers {
                            return;
                        }
                        // The shell's: the browser must not act on it as well — move its own
                        // caret, submit, leave the element on a Tab.
                        key.prevent_default();
                        forward(&canvas, kind, key);
                    }),
                );
            }
            if let Some(parent) = self.canvas.parent_node() {
                parent.append_child(&el).ok()?;
            } else {
                self.document.body()?.append_child(&el).ok()?;
            }
            Some(Live {
                id,
                web,
                el,
                composing,
                _listeners: listeners,
            })
        }

        /// Takes `live` away. When no field follows it and it had the browser's focus, the
        /// canvas gets it back — or the keys that follow would reach no one.
        fn remove(&self, live: Live, last: bool) {
            let had_focus = self
                .document
                .active_element()
                .is_some_and(|active| active == *AsRef::<web_sys::Element>::as_ref(&live.el));
            live.el.remove();
            if last && had_focus {
                let _ = self.canvas.focus();
            }
        }
    }

    /// The element's look: nothing. It is over the field and carries its caret for the
    /// browser — where a composition's candidates come up — and draws none of it.
    const STYLE: &str = "position:fixed;margin:0;padding:0;border:0;outline:none;\
        background:transparent;color:transparent;caret-color:transparent;\
        text-shadow:none;resize:none;overflow:hidden;pointer-events:none;\
        opacity:0;z-index:1;";

    fn target(event: &Event) -> Option<HtmlElement> {
        event.target()?.dyn_into::<HtmlElement>().ok()
    }

    fn value(el: &HtmlElement) -> String {
        if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
            input.value()
        } else if let Some(area) = el.dyn_ref::<HtmlTextAreaElement>() {
            area.value()
        } else {
            String::new()
        }
    }

    fn set_value(el: &HtmlElement, text: &str) {
        if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
            input.set_value(text);
        } else if let Some(area) = el.dyn_ref::<HtmlTextAreaElement>() {
            area.set_value(text);
        }
    }

    fn set_selection(el: &HtmlElement, start: u32, end: u32, backward: bool) {
        let direction = if backward { "backward" } else { "forward" };
        if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
            let _ = input.set_selection_range_with_direction(start, end, direction);
        } else if let Some(area) = el.dyn_ref::<HtmlTextAreaElement>() {
            let _ = area.set_selection_range_with_direction(start, end, direction);
        }
    }

    /// The element's state, with the composition `composing` holds open.
    fn state(el: &HtmlElement, composing: Option<String>) -> Element {
        let (start, end, direction) = if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
            (
                input.selection_start().ok().flatten(),
                input.selection_end().ok().flatten(),
                input.selection_direction().ok().flatten(),
            )
        } else if let Some(area) = el.dyn_ref::<HtmlTextAreaElement>() {
            (
                area.selection_start().ok().flatten(),
                area.selection_end().ok().flatten(),
                area.selection_direction().ok().flatten(),
            )
        } else {
            (None, None, None)
        };
        let value = value(el);
        let length = value.encode_utf16().count() as u32;
        let end = end.unwrap_or(length);
        Element {
            value,
            start: start.unwrap_or(end),
            end,
            backward: direction.as_deref() == Some("backward"),
            composing,
        }
    }

    /// Hands a key the element will not act on to the canvas, where winit reads it as it
    /// reads every key the canvas gets.
    fn forward(canvas: &HtmlCanvasElement, kind: &str, key: &KeyboardEvent) {
        let init = KeyboardEventInit::new();
        init.set_key(&key.key());
        init.set_code(&key.code());
        init.set_location(key.location());
        init.set_repeat(key.repeat());
        init.set_ctrl_key(key.ctrl_key());
        init.set_shift_key(key.shift_key());
        init.set_alt_key(key.alt_key());
        init.set_meta_key(key.meta_key());
        init.set_bubbles(true);
        init.set_cancelable(true);
        if let Ok(event) = KeyboardEvent::new_with_keyboard_event_init_dict(kind, &init) {
            let _ = canvas.dispatch_event(&event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(value: &str, start: u32, end: u32) -> Element {
        Element {
            value: value.to_owned(),
            start,
            end,
            ..Element::default()
        }
    }

    /// The DOM counts in UTF-16 units; a field counts characters. An emoji is two units and
    /// one character, and an offset between its halves has not reached it.
    #[test]
    fn utf16_offsets_become_characters() {
        let text = "a😀b";
        assert_eq!(char_index(text, 0), 0);
        assert_eq!(char_index(text, 1), 1);
        assert_eq!(char_index(text, 2), 1, "inside the pair");
        assert_eq!(char_index(text, 3), 2);
        assert_eq!(char_index(text, 4), 3);
        assert_eq!(char_index(text, 99), 3, "past the end is the end");
    }

    /// A caret, a selection made forwards and one made backwards.
    #[test]
    fn the_selection_comes_back_with_its_direction() {
        let (text, edit) = read(&element("hello", 2, 2));
        assert_eq!(text, "hello");
        assert_eq!((edit.cursor, edit.anchor, edit.composing), (2, None, None));

        let (_, forwards) = read(&element("hello", 1, 4));
        assert_eq!((forwards.cursor, forwards.anchor), (4, Some(1)));

        let backwards = Element {
            backward: true,
            ..element("hello", 1, 4)
        };
        let (_, backwards) = read(&backwards);
        assert_eq!((backwards.cursor, backwards.anchor), (1, Some(4)));
    }

    /// A composition is the text being composed, ending at the caret — the probe's own
    /// sequence: "ni" composed after "a", in "ab".
    #[test]
    fn a_composition_ends_at_the_caret() {
        let composing = Element {
            composing: Some("ni".into()),
            ..element("anib", 3, 3)
        };
        let (text, edit) = read(&composing);
        assert_eq!(text, "anib");
        assert_eq!(edit.composing, Some((1, 3)));
        // Committed: no composition left.
        let (_, committed) = read(&element("a你b", 2, 2));
        assert_eq!(committed.composing, None);
        // A composition that has just started has nothing in it yet.
        let started = Element {
            composing: Some(String::new()),
            ..element("ab", 1, 1)
        };
        assert_eq!(read(&started).1.composing, None);
    }

    /// The way back: characters into UTF-16 units, the direction kept, and what is past the
    /// end of a text the field refused brought back to its end.
    #[test]
    fn the_field_selection_goes_to_the_element() {
        let caret = Edit {
            cursor: 2,
            ..Edit::default()
        };
        assert_eq!(selection("a😀b", &caret), (3, 3, false));
        let backwards = Edit {
            affinity: Default::default(),
            cursor: 0,
            anchor: Some(2),
            composing: None,
        };
        assert_eq!(selection("a😀b", &backwards), (0, 3, true));
        let forwards = Edit {
            affinity: Default::default(),
            cursor: 3,
            anchor: Some(1),
            composing: None,
        };
        assert_eq!(selection("a😀b", &forwards), (1, 4, false));
        let past = Edit {
            affinity: Default::default(),
            cursor: 9,
            anchor: Some(7),
            composing: None,
        };
        assert_eq!(
            selection("ab", &past),
            (2, 2, false),
            "both ends at the end"
        );
    }

    fn plain(key: &str) -> bool {
        browser_edits(key, false, false, false, false, false, false)
    }

    fn ctrl(key: &str) -> bool {
        browser_edits(key, true, false, false, false, false, false)
    }

    /// What is typed, deleted, composed or pasted is the browser's.
    #[test]
    fn typing_deleting_and_the_clipboard_are_the_browsers() {
        for key in [
            "a",
            "Z",
            "é",
            " ",
            "7",
            "Backspace",
            "Delete",
            "Dead",
            "Process",
        ] {
            assert!(plain(key), "{key}");
        }
        for key in ["c", "x", "v", "V", "Backspace", "Delete", "Insert"] {
            assert!(ctrl(key), "Ctrl+{key}");
        }
        // Command on a Mac.
        assert!(browser_edits("v", false, false, true, false, false, true));
        // Shift+Insert pastes.
        assert!(browser_edits(
            "Insert", false, false, false, true, false, false
        ));
        // A key that comes in the middle of a composition, whatever it is.
        assert!(browser_edits(
            "Enter", false, false, false, false, true, false
        ));
        assert!(browser_edits(
            "ArrowLeft",
            false,
            false,
            false,
            false,
            true,
            false
        ));
    }

    /// What moves, submits, leaves or commands is the shell's: its layout is what is on the
    /// screen, its undo is the application's, and its shortcuts are the application's.
    #[test]
    fn moving_submitting_and_shortcuts_are_the_shells() {
        for key in [
            "ArrowLeft",
            "ArrowRight",
            "ArrowUp",
            "ArrowDown",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Enter",
            "Tab",
            "Escape",
            "F2",
            "ContextMenu",
            "Shift",
            "Control",
        ] {
            assert!(!plain(key), "{key}");
        }
        for key in ["a", "z", "y", "s", "k", "ArrowLeft"] {
            assert!(!ctrl(key), "Ctrl+{key}");
        }
    }

    /// AltGr types (Ctrl and Alt together, as Windows reports it); Alt with a letter is a
    /// shortcut, except on a Mac, where Option makes characters.
    #[test]
    fn alt_gr_types_and_alt_is_a_shortcut_but_on_a_mac() {
        assert!(browser_edits("€", true, true, false, false, false, false));
        assert!(!browser_edits("f", false, true, false, false, false, false));
        assert!(browser_edits("ƒ", false, true, false, false, false, true));
    }

    /// On a touch screen the element stays where the coming keyboard will not cover it, so
    /// that the browser does not pan the page (milestone 749).
    #[test]
    fn the_element_keeps_out_of_the_keyboard_s_way_on_a_touch_screen() {
        // The phone: a canvas from 0 to 788, a field at 690, 50 high.
        assert_eq!(element_top(690.0, 50.0, 0.0, 788.0, 0.0, false), 690.0);
        // A finger, no keyboard yet: the top quarter.
        assert_eq!(element_top(690.0, 50.0, 0.0, 788.0, 0.0, true), 197.0);
        // A field already there stays over its field.
        assert_eq!(element_top(120.0, 50.0, 0.0, 788.0, 0.0, true), 120.0);
        // The keyboard up, 466 high: anywhere above it, and over the field once it is there.
        assert_eq!(element_top(690.0, 50.0, 0.0, 788.0, 466.0, true), 272.0);
        assert_eq!(element_top(200.0, 50.0, 0.0, 788.0, 466.0, true), 200.0);
        // Never above the canvas.
        assert_eq!(element_top(-80.0, 50.0, 0.0, 788.0, 466.0, true), 0.0);
    }

    #[test]
    fn a_change_is_the_stretch_it_replaced() {
        assert_eq!(splice("abc", "abXc"), (2, 2, "X".into()));
        assert_eq!(splice("abc", "ac"), (1, 2, String::new()));
        assert_eq!(splice("hello", "help"), (3, 5, "p".into()));
        assert_eq!(splice("", "é😀"), (0, 0, "é😀".into()));
        // A repeated letter: the common end never overlaps the common start.
        assert_eq!(splice("aa", "aaa"), (2, 2, "a".into()));
        assert_eq!(splice("same", "same"), (4, 4, String::new()));
    }

    /// What the browser types goes through the field's filter and its length limit, as a key
    /// does (milestone 749). It went in as is: the gallery's upper-case code field held
    /// lower case on a phone.
    #[test]
    fn the_browser_s_typing_goes_through_the_field_s_filter_and_limit() {
        use frus_widgets::TextField;
        let upper = TextField::new("X")
            .on_input(|s: String| s)
            .input_filter(|c| c.is_ascii_alphanumeric().then(|| c.to_ascii_uppercase()));
        let typed = Edit {
            cursor: 3,
            anchor: None,
            composing: Some((1, 3)),
        };
        let (message, edit) = browser_change(&upper, "Xyz", typed);
        assert_eq!(message.as_deref(), Some("XYZ"));
        // Taken with the same length: the browser's caret and composition stand.
        assert_eq!(edit, typed);

        // A character the filter drops: the field's own caret.
        let (message, edit) = browser_change(
            &upper,
            "X-",
            Edit {
                cursor: 2,
                ..Edit::default()
            },
        );
        assert_eq!(message, None);
        assert_eq!(edit.cursor, 1);

        // The length limit, on a paste.
        let short = TextField::new("ab").on_input(|s: String| s).max_length(3);
        let (message, edit) = browser_change(
            &short,
            "abcdef",
            Edit {
                cursor: 6,
                ..Edit::default()
            },
        );
        assert_eq!(message.as_deref(), Some("abc"));
        assert_eq!(edit.cursor, 3);

        // A deletion, and a field that holds what it was given.
        let (message, _) = browser_change(
            &short,
            "a",
            Edit {
                cursor: 1,
                ..Edit::default()
            },
        );
        assert_eq!(message.as_deref(), Some("a"));
        let (message, _) = browser_change(
            &short,
            "ab",
            Edit {
                cursor: 2,
                ..Edit::default()
            },
        );
        assert_eq!(message, None);
    }
}
