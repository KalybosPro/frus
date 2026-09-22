//! The **accessibility** bridge for the web: maps frus's semantic tree
//! ([`frus_widgets::Ui::semantics`]) onto real DOM elements, so a browser's screen
//! reader (NVDA, JAWS, VoiceOver, Narrator) can read and operate the interface.
//!
//! AccessKit has no web adapter — its own README lists the web as a **planned**
//! platform, not a shipped one, and `accesskit_winit` attaches to nothing on
//! `wasm32`. What the web *does* have, natively, is the accessible-canvas technique
//! the platform itself defines: content nested inside a `<canvas>` element is not
//! painted when the canvas renders (so it never doubles what the GPU already drew),
//! but it is still walked into the accessibility tree, focusable by `Tab`, and
//! reachable by a screen reader's own navigation — exactly the fallback the `<canvas>`
//! element was specified with. This module keeps the DOM in step with the same
//! `(WidgetId, Rect, SemanticsProperties)` list the desktop bridge reads, one real
//! element per node (a `<button>` when the node is clickable, so `Enter` and `Space`
//! activate it without a hand-rolled key handler; a `<div>` otherwise), diffed against
//! last frame's set rather than rebuilt from nothing.
//!
//! What this does **not** do: give a node a bounding box a touch screen reader (mobile
//! VoiceOver, TalkBack) could use for spatial "explore by touch" — canvas fallback
//! content is unpainted, so it is never laid out and `getBoundingClientRect` on it is
//! always empty. A keyboard-driven or virtual-cursor screen reader does not need one;
//! touch exploration is left for a future pass, noted in the ROADMAP.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlCanvasElement, HtmlElement};
use winit::platform::web::WindowExtWebSys;
use winit::window::Window;

use frus_widgets::{Rect, Role, SemanticsProperties, Toggled, WidgetId};

/// The ARIA role for a frus role — the web's counterpart to `a11y::to_ak_role`.
fn aria_role(role: Role) -> &'static str {
    match role {
        Role::None => "group",
        Role::Label => "text",
        Role::Heading => "heading",
        Role::Button => "button",
        Role::Link => "link",
        Role::CheckBox => "checkbox",
        Role::Switch => "switch",
        Role::RadioButton => "radio",
        Role::Slider => "slider",
        Role::TextInput => "textbox",
        Role::Image => "img",
        Role::Tab => "tab",
        Role::ListItem => "listitem",
        Role::ProgressBar => "progressbar",
    }
}

/// An action a screen reader asked for, replayed by the shell's main loop — the same
/// shape as `a11y::A11yAction`, kept as its own type since the two bridges share no
/// code below this file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum A11yAction {
    /// Activate the widget, that is, click it.
    Click(WidgetId),
    /// Give the widget focus.
    Focus(WidgetId),
}

type Actions = Rc<RefCell<Vec<A11yAction>>>;

/// One live DOM node backing a widget's accessibility, and the listeners keeping it
/// wired — dropped together with the element when the widget leaves the tree.
struct Live {
    el: HtmlElement,
    clickable: bool,
    _click: Option<Closure<dyn FnMut(web_sys::Event)>>,
    _focus: Closure<dyn FnMut(web_sys::Event)>,
}

/// Attaches a `click` listener that queues an [`A11yAction::Click`] and **wakes the
/// loop** — the web only ever runs a frame it was asked for (milestone 561 found the
/// same gap in the address bar's own `popstate`), and a screen reader's click arrives
/// with nothing else pending a redraw.
fn click_listener(
    id: WidgetId,
    actions: Actions,
    window: Arc<Window>,
) -> Closure<dyn FnMut(web_sys::Event)> {
    Closure::wrap(Box::new(move |_event: web_sys::Event| {
        actions.borrow_mut().push(A11yAction::Click(id));
        window.request_redraw();
    }) as Box<dyn FnMut(web_sys::Event)>)
}

/// Attaches a `focus` listener that queues an [`A11yAction::Focus`] and wakes the loop
/// — unless the focus just moved here **because we called `.focus()` ourselves**
/// (`programmatic`), in which case it is an echo, not a request, and queuing it would
/// bounce forever.
fn focus_listener(
    id: WidgetId,
    actions: Actions,
    programmatic: Rc<Cell<bool>>,
    window: Arc<Window>,
) -> Closure<dyn FnMut(web_sys::Event)> {
    Closure::wrap(Box::new(move |_event: web_sys::Event| {
        if !programmatic.get() {
            actions.borrow_mut().push(A11yAction::Focus(id));
            window.request_redraw();
        }
    }) as Box<dyn FnMut(web_sys::Event)>)
}

/// A window's **live** accessibility bridge, projected into the canvas's own fallback
/// content.
pub(crate) struct A11y {
    canvas: HtmlCanvasElement,
    live_region: HtmlElement,
    nodes: HashMap<WidgetId, Live>,
    actions: Actions,
    /// Set for the duration of a `.focus()` call we made ourselves, so the `focus`
    /// listener it fires is not queued back as an assistive-technology request.
    programmatic_focus: Rc<Cell<bool>>,
    focus: Option<WidgetId>,
    announce: String,
    /// Cloned into every listener, so it can wake the loop (`request_redraw`) — the
    /// web only runs a frame it was asked for, and a screen reader's action is not
    /// otherwise anything winit knows to redraw for.
    window: Arc<Window>,
}

impl A11y {
    /// Creates the bridge over `window`'s canvas. `None` if winit has not attached one
    /// (should not happen on the web target, but nothing here is worth a panic over).
    pub(crate) fn new(window: &Arc<Window>) -> Option<Self> {
        let canvas = window.canvas()?;
        canvas.set_attribute("role", "application").ok();
        let live_region = new_live_region(&canvas)?;
        Some(Self {
            canvas,
            live_region,
            nodes: HashMap::new(),
            actions: Rc::new(RefCell::new(Vec::new())),
            programmatic_focus: Rc::new(Cell::new(false)),
            focus: None,
            announce: String::new(),
            window: window.clone(),
        })
    }

    /// Publishes the frame's semantic tree: creates the elements for new widgets,
    /// updates the ones already there, and removes the ones no longer present.
    pub(crate) fn update(
        &mut self,
        nodes: &[(WidgetId, Rect, SemanticsProperties)],
        focus: Option<WidgetId>,
        title: &str,
        announce: &str,
    ) {
        self.canvas.set_attribute("aria-label", title).ok();

        let present: std::collections::HashSet<WidgetId> =
            nodes.iter().map(|(id, _, _)| *id).collect();
        self.nodes.retain(|id, live| {
            let keep = present.contains(id);
            if !keep {
                let _ = self.canvas.remove_child(&live.el);
            }
            keep
        });

        for (id, _rect, sem) in nodes {
            match self.nodes.get(id) {
                Some(live) if live.clickable == sem.clickable => {
                    apply(&live.el, sem);
                }
                _ => {
                    // Either new, or its element was built for the wrong tag (a
                    // clickable widget needs the `<button>`, a native `Enter`/`Space`
                    // activator a `<div>` cannot offer).
                    if let Some(old) = self.nodes.remove(id) {
                        let _ = self.canvas.remove_child(&old.el);
                    }
                    if let Some(live) = self.spawn(*id, sem) {
                        apply(&live.el, sem);
                        self.nodes.insert(*id, live);
                    }
                }
            }
        }

        if focus != self.focus {
            self.focus = focus;
            if let Some(id) = focus {
                if let Some(live) = self.nodes.get(&id) {
                    self.programmatic_focus.set(true);
                    let _ = live.el.focus();
                    self.programmatic_focus.set(false);
                }
            }
        }

        if announce != self.announce {
            self.announce = announce.to_string();
            self.live_region.set_text_content(Some(announce));
        }
    }

    /// Builds the DOM element for a new widget and attaches its listeners.
    fn spawn(&self, id: WidgetId, sem: &SemanticsProperties) -> Option<Live> {
        let document = self.canvas.owner_document()?;
        let tag = if sem.clickable { "button" } else { "div" };
        let el: HtmlElement = document
            .create_element(tag)
            .ok()?
            .dyn_into::<HtmlElement>()
            .ok()?;
        if sem.clickable {
            el.set_attribute("type", "button").ok();
        } else {
            // Not a Tab stop (only interactive nodes are), but still a target the
            // shell can move real focus onto when its own model says so.
            el.set_attribute("tabindex", "-1").ok();
        }
        let click = sem.clickable.then(|| {
            let closure = click_listener(id, self.actions.clone(), self.window.clone());
            let _ = (el.as_ref() as &web_sys::EventTarget)
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
            closure
        });
        let focus_closure = focus_listener(
            id,
            self.actions.clone(),
            self.programmatic_focus.clone(),
            self.window.clone(),
        );
        let _ = (el.as_ref() as &web_sys::EventTarget)
            .add_event_listener_with_callback("focus", focus_closure.as_ref().unchecked_ref());
        self.canvas.append_child(&el).ok()?;
        Some(Live {
            el,
            clickable: sem.clickable,
            _click: click,
            _focus: focus_closure,
        })
    }

    /// Takes the actions the screen reader asked for since the last frame, emptying
    /// the queue.
    pub(crate) fn take_actions(&self) -> Vec<A11yAction> {
        std::mem::take(&mut self.actions.borrow_mut())
    }
}

/// Creates the **live region**: an invisible, polite status the assistive technology
/// speaks when its text **changes** — `aria-live="polite"`, the same contract the
/// desktop bridge gives AccessKit's `Live::Polite`.
///
/// **Not** canvas fallback content, unlike every other node this module builds.
/// Found testing #18 in a real browser: a live region nested inside an unpainted
/// `<canvas>` child never reached Chrome's accessibility tree at all — its role and
/// interactive nodes did, but the live-region machinery, which watches the *render*
/// tree for mutations, has nothing to watch in a subtree the canvas never paints. This
/// one is a real, laid-out sibling instead, hidden the ordinary "visually hidden but
/// present" way (a 1×1 box, clipped, never `display: none`) rather than by being
/// outside the render tree altogether.
fn new_live_region(canvas: &HtmlCanvasElement) -> Option<HtmlElement> {
    let document = canvas.owner_document()?;
    let body = document.body()?;
    let el: HtmlElement = document
        .create_element("div")
        .ok()?
        .dyn_into::<HtmlElement>()
        .ok()?;
    el.set_attribute("aria-live", "polite").ok();
    el.set_attribute("role", "status").ok();
    el.set_attribute(
        "style",
        "position:absolute;width:1px;height:1px;overflow:hidden;\
         clip:rect(0,0,0,0);white-space:nowrap;",
    )
    .ok();
    body.append_child(&el).ok()?;
    Some(el)
}

/// Writes a frus annotation onto an already-built element: role, name, state — every
/// frame, since any of them may have changed.
fn apply(el: &HtmlElement, sem: &SemanticsProperties) {
    let element: &Element = el.as_ref();
    element.set_attribute("role", aria_role(sem.role)).ok();
    if let Some(label) = &sem.label {
        element.set_attribute("aria-label", label).ok();
    } else {
        element.remove_attribute("aria-label").ok();
    }
    match sem.toggled {
        Toggled::None => {
            element.remove_attribute("aria-checked").ok();
        }
        Toggled::False => {
            element.set_attribute("aria-checked", "false").ok();
        }
        Toggled::True => {
            element.set_attribute("aria-checked", "true").ok();
        }
        Toggled::Mixed => {
            element.set_attribute("aria-checked", "mixed").ok();
        }
    }
    if sem.disabled {
        element.set_attribute("aria-disabled", "true").ok();
        if sem.clickable {
            element.set_attribute("disabled", "").ok();
        }
    } else {
        element.remove_attribute("aria-disabled").ok();
        if sem.clickable {
            element.remove_attribute("disabled").ok();
        }
    }
    if let Some((min, value, max)) = sem.range {
        element
            .set_attribute("aria-valuemin", &min.to_string())
            .ok();
        element
            .set_attribute("aria-valuenow", &value.to_string())
            .ok();
        element
            .set_attribute("aria-valuemax", &max.to_string())
            .ok();
    } else {
        element.remove_attribute("aria-valuemin").ok();
        element.remove_attribute("aria-valuenow").ok();
        element.remove_attribute("aria-valuemax").ok();
    }
    // A role with no dedicated ARIA value slot (a badge on a button, a caption under a
    // step) carries its `value` as a description read after the label; a field's
    // contents are its accessible **value**, read through the element's own text
    // rather than a made-up attribute, matching how a browser resolves a native
    // `role="textbox"` with no editable child.
    match (&sem.value, sem.role) {
        (Some(value), Role::TextInput) => el.set_text_content(Some(value)),
        (Some(value), Role::Slider | Role::ProgressBar) => {
            element.set_attribute("aria-valuetext", value).ok();
        }
        (Some(value), _) => {
            element.set_attribute("aria-description", value).ok();
        }
        (None, Role::TextInput) => el.set_text_content(None),
        (None, _) => {
            element.remove_attribute("aria-description").ok();
            element.remove_attribute("aria-valuetext").ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_aria() {
        assert_eq!(aria_role(Role::Button), "button");
        assert_eq!(aria_role(Role::CheckBox), "checkbox");
        assert_eq!(aria_role(Role::Slider), "slider");
        assert_eq!(aria_role(Role::ProgressBar), "progressbar");
        assert_eq!(aria_role(Role::TextInput), "textbox");
    }
}
