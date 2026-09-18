//! The generic driver: implements [`winit::application::ApplicationHandler`] for
//! any [`Application`].
//!
//! The framework owns the window, the renderer, the [`Runtime`] — the retained
//! interaction state: hover, focus, scroll, editing, animations — input routing by
//! hit test, dragging (scrollbars, selection, handles, the back gesture) and the
//! animation clock. The application supplies only `update`, `view` and friends.

use std::collections::HashMap;
use std::sync::Arc;

use web_time::Instant;

use frus_gpu::Renderer;
use frus_widgets::{
    build_deferred, build_ui, collect_ids, find_by_key, find_path, find_widget,
    nearest_reorder_slot, reflow_reorder_cards, reflow_reorder_columns, reorder_drop_after,
    reorder_siblings, reorderable_owners, subtree_ids, Accessibility, Brightness, Color,
    Cursor as UiCursor, Edit, EditKind, EditSnapshot, FocusDirection, Insets, Key, KeyResponse,
    KeyStroke, MediaQuery, Point, Primitive, Rect, ReorderAxis, Runtime, Scene, ScrollTo,
    Scrollable, SheetTo, ShortcutKey, Size, Theme, Ui, VelocityEstimate, VelocityTracker, Widget,
    WidgetId, WindowInsets,
};
use winit::application::ApplicationHandler;
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, StartCause, TouchPhase, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key as WinitKey, KeyCode, NamedKey, PhysicalKey};
use winit::window::{CursorIcon, Window, WindowId};

use crate::application::{Application, Lifecycle};

/// Everything the platform reports about its user, in one walk.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlatformSettings {
    /// The system's *Font size* slider. `1.0` is normal; Android's own slider reaches
    /// 1.3, and the accessibility sizes go further.
    pub text_scaler: f32,
    /// Whether the system is currently showing a dark interface.
    pub brightness: Brightness,
    /// The accessibility settings, as far as they could be read.
    pub accessibility: Accessibility,
    /// **The reader's preferred languages**, best first, as the platform reports them.
    ///
    /// Empty where the platform said nothing — which is a real answer and not a failure:
    /// the resolution treats it as *no preference* and hands back the application's own
    /// first choice, exactly as the reference does (`app.dart:153`).
    pub locales: Vec<frus_widgets::Locale>,
}

impl Default for PlatformSettings {
    /// What a platform that reports nothing looks like: a user who has changed nothing.
    fn default() -> Self {
        Self {
            text_scaler: 1.0,
            brightness: Brightness::Light,
            accessibility: Accessibility::NONE,
            locales: Vec::new(),
        }
    }
}
use crate::gesture::{PointerEvent, PointerKind, PressRecognizer};

/// What a key asks of the clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClipCommand {
    Copy,
    Cut,
    Paste,
}

/// The clipboard command a key press means, if any: Ctrl+C/X/V, and the Copy, Cut and
/// Paste keys a keyboard may have for them (milestone 509).
///
/// Both halves of the key are asked. A desktop keyboard's Copy key arrives as a named
/// logical key; Android's arrives by **physical code only** — winit gives it no logical
/// key at all — so a check of the logical key alone would ignore it on the one platform
/// that has it as a keycode of its own.
fn clipboard_command(logical: &WinitKey, physical: PhysicalKey, ctrl: bool) -> Option<ClipCommand> {
    match logical {
        WinitKey::Named(NamedKey::Copy) => return Some(ClipCommand::Copy),
        WinitKey::Named(NamedKey::Cut) => return Some(ClipCommand::Cut),
        WinitKey::Named(NamedKey::Paste) => return Some(ClipCommand::Paste),
        WinitKey::Character(c) if ctrl => {
            if c.eq_ignore_ascii_case("c") {
                return Some(ClipCommand::Copy);
            }
            if c.eq_ignore_ascii_case("x") {
                return Some(ClipCommand::Cut);
            }
            if c.eq_ignore_ascii_case("v") {
                return Some(ClipCommand::Paste);
            }
        }
        _ => {}
    }
    match physical {
        PhysicalKey::Code(KeyCode::Copy) => Some(ClipCommand::Copy),
        PhysicalKey::Code(KeyCode::Cut) => Some(ClipCommand::Cut),
        PhysicalKey::Code(KeyCode::Paste) => Some(ClipCommand::Paste),
        _ => None,
    }
}

/// Whether `widget` takes typing, and so wants the software keyboard while it has focus.
///
/// Asked of the widget (milestone 510). It used to be asked of a **caret hit test** at
/// the corner of a field one pixel wide, and a field with a clickable suffix — the ×
/// a field shows once it holds text — answered that no caret goes there, since that
/// pixel is its clear button. So the first letter typed into such a field closed the
/// keyboard and ended its composition, and the keyboard's next update was written
/// beside the first: `F`, then `FFr`.
#[cfg(any(android, test))]
fn wants_keyboard<M>(widget: &dyn Widget<M>) -> bool {
    widget.text_value().is_some() && widget.focusable()
}

/// The clipboard: `arboard` on the desktop platforms, the platform's own on Android
/// (`ClipboardManager`, through the bundled dex — milestone 509, #22), the browser's
/// asynchronous Clipboard API on the Web (milestone 526, #17), and a no-op on iOS
/// (`arboard` does not compile there and is not a dependency).
/// The stub is gated on what is *not* implemented rather than on a list of the other
/// platforms: that is what makes adding a target never leave this type undefined.
/// One uniform API, so the driver's body stays free of `cfg`.
///
/// **A paste is asked for, and answered.** The Web's read is a promise — it may prompt
/// the reader, be refused, or take its time — and the other platforms' reads are
/// immediate. So `Clipboard::paste` names the field that asked and either answers at
/// once or answers later through `Clipboard::take_answered`, which the driver drains
/// every frame; both answers land through the same rule, `Pasted::lands`. Nothing on a
/// synchronous platform pretends to wait, and no widget knows which kind it is on: the
/// shell, not the field, holds the clipboard.
mod clip {
    use frus_widgets::WidgetId;

    /// What the clipboard answered to a paste: its text, and the field that asked.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Pasted {
        pub into: WidgetId,
        pub text: String,
    }

    impl Pasted {
        /// Whether this paste goes into its field: only while that field still has the
        /// focus — on the Web the answer comes back after the key press, and the reader
        /// may have moved on to another field, or to none — and only when there is text.
        /// An empty clipboard is nothing to paste, not an instruction to delete the
        /// selection; the browser answers a clipboard holding no text with `""`.
        pub fn lands(&self, focused: Option<WidgetId>) -> bool {
            focused == Some(self.into) && !self.text.is_empty()
        }
    }

    /// The platforms whose read is immediate answer a paste on the spot.
    #[cfg(not(web))]
    impl Clipboard {
        pub fn paste(
            &mut self,
            into: WidgetId,
            _wake: Option<&std::sync::Arc<winit::window::Window>>,
        ) -> Option<Pasted> {
            self.get_text().map(|text| Pasted { into, text })
        }
        pub fn take_answered(&mut self) -> Option<Pasted> {
            None
        }
    }

    #[cfg(desktop)]
    pub struct Clipboard(Option<arboard::Clipboard>);

    #[cfg(desktop)]
    impl Clipboard {
        pub fn new() -> Self {
            Self(arboard::Clipboard::new().ok())
        }
        pub fn get_text(&mut self) -> Option<String> {
            self.0.as_mut().and_then(|c| c.get_text().ok())
        }
        pub fn set_text(&mut self, text: String) {
            if let Some(c) = self.0.as_mut() {
                let _ = c.set_text(text);
            }
        }
    }

    #[cfg(android)]
    pub struct Clipboard;

    #[cfg(android)]
    impl Clipboard {
        pub fn new() -> Self {
            Self
        }
        pub fn get_text(&mut self) -> Option<String> {
            crate::android_clipboard::get_text()
        }
        pub fn set_text(&mut self, text: String) {
            crate::android_clipboard::set_text(&text);
        }
    }

    /// The pastes a clipboard has answered later and the driver has not taken yet, in the
    /// order the answers came — which need not be the order they were asked in.
    ///
    /// Held by the Web's clipboard; kept out of its `cfg` so that it is tested natively.
    #[cfg(any(web, test))]
    #[derive(Default)]
    pub struct Answers(std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<Pasted>>>);

    #[cfg(any(web, test))]
    impl Answers {
        /// What answers a paste into `into` once the text is there: it queues the paste,
        /// then calls `wake`, so that a frame comes to take it.
        pub fn answer_for(
            &self,
            into: WidgetId,
            wake: impl FnOnce() + 'static,
        ) -> impl FnOnce(String) + 'static {
            let queue = std::rc::Rc::clone(&self.0);
            move |text| {
                queue.borrow_mut().push_back(Pasted { into, text });
                wake();
            }
        }

        /// The oldest answer not taken yet.
        pub fn take(&self) -> Option<Pasted> {
            self.0.borrow_mut().pop_front()
        }
    }

    /// The Web: a read is started by the key press and answered by the browser later,
    /// into a queue the driver drains; the window is asked for a frame so that it does.
    #[cfg(web)]
    pub struct Clipboard {
        answered: Answers,
    }

    #[cfg(web)]
    impl Clipboard {
        pub fn new() -> Self {
            Self {
                answered: Answers::default(),
            }
        }
        pub fn set_text(&mut self, text: String) {
            crate::web_clipboard::set_text(&text);
        }
        pub fn paste(
            &mut self,
            into: WidgetId,
            wake: Option<&std::sync::Arc<winit::window::Window>>,
        ) -> Option<Pasted> {
            let wake = wake.cloned();
            crate::web_clipboard::get_text(self.answered.answer_for(into, move || {
                if let Some(window) = wake {
                    window.request_redraw();
                }
            }));
            None
        }
        pub fn take_answered(&mut self) -> Option<Pasted> {
            self.answered.take()
        }
    }

    #[cfg(not(any(desktop, android, web)))]
    pub struct Clipboard;

    #[cfg(not(any(desktop, android, web)))]
    impl Clipboard {
        pub fn new() -> Self {
            Self
        }
        pub fn get_text(&mut self) -> Option<String> {
            None
        }
        pub fn set_text(&mut self, _text: String) {}
    }
}

/// Browser timers, for the Web: the counterpart of the native subscriptions'
/// `recv_timeout` thread. A **retained** `setInterval`, whose **drop** calls
/// `clearInterval` and releases the closure. That is how an `every` subscription
/// ticks on the Web, where there is no background thread.
#[cfg(web)]
mod web_timer {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;

    /// An active `setInterval`: as long as this handle lives the callback is called
    /// back every `ms`; dropping it cancels that.
    pub(crate) struct Interval {
        id: i32,
        // The JS closure must live as long as the interval does.
        _closure: Closure<dyn FnMut()>,
    }

    impl Interval {
        /// Schedules `f` every `ms` milliseconds, 1 ms at the least. `None` when there
        /// is no window, that is, in a DOM-less context.
        pub(crate) fn new(ms: i32, f: impl FnMut() + 'static) -> Option<Self> {
            let window = web_sys::window()?;
            let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
            let id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    ms.max(1),
                )
                .ok()?;
            Some(Self {
                id,
                _closure: closure,
            })
        }
    }

    impl Drop for Interval {
        fn drop(&mut self) {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(self.id);
            }
        }
    }

    /// Calls `f` once, after `ms` milliseconds — the Web half of
    /// [`Command::after`](crate::Command::after).
    ///
    /// There is no handle to keep, and that is the point. `Closure::once_into_js`
    /// hands ownership of the closure to the JS side, which frees it after the call.
    /// A `Closure` we owned instead would have to be either leaked — one leak per
    /// timer — or dropped here, which would free the callback out from under a
    /// `setTimeout` that has not fired yet.
    pub(crate) fn after(ms: i32, f: impl FnOnce() + 'static) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let callback = wasm_bindgen::closure::Closure::once_into_js(f);
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.unchecked_ref(),
            ms.max(0),
        );
    }
}

/// An active subscription's cancellation handle: **dropping** it stops the
/// subscription.
/// - Native: the executor task's handle — dropping it cancels the task.
/// - Web: a retained `setInterval` (`None` when installing it failed).
#[cfg(not(web))]
type SubHandle = async_executor::Task<()>;
#[cfg(web)]
type SubHandle = Option<web_timer::Interval>;

/// Scroll speed, in pixels per wheel notch.
const SCROLL_SPEED: f32 = 40.0;

/// The distance a **finger** must travel before the framework is confident the
/// gesture is a drag rather than a tap, in logical px.
///
/// 18 px is large for a screen measurement, and deliberately so: a thumb covers a
/// wide contact patch and rolls as it presses, so a smaller threshold turns taps
/// near the edge of a button into aborted drags. It is the value the mature
/// toolkits settled on after starting at 8 and hearing that targets were too hard
/// to hit.
const TOUCH_SLOP: f32 = 18.0;

/// The same threshold for a **precise** pointer — a mouse or a trackpad. It knows
/// exactly where it is, so almost any movement is intentional.
const PRECISE_SLOP: f32 = 1.0;

/// The velocity a release should be flung with, per axis — zero on an axis the
/// gesture did not really travel along.
///
/// Speed alone is not a fling. A finger that twitches fast over three pixels
/// produces a large velocity and no intent; requiring the gesture to have
/// **covered ground** as well is what separates the two, and the distance that says
/// "this was a drag" is the same `slop` that started it.
///
/// The gate is per axis because a scroll is two independent axes: a swipe running
/// down the screen must not fling sideways on the little horizontal wobble a thumb
/// always adds.
fn fling_velocity(estimate: VelocityEstimate, slop: f32) -> (f32, f32) {
    let gate = |velocity: f32, travelled: f32| {
        if travelled.abs() > slop {
            velocity
        } else {
            0.0
        }
    };
    (
        gate(estimate.velocity.x, estimate.offset.0),
        gate(estimate.velocity.y, estimate.offset.1),
    )
}

/// The elastic overshoot allowed past the scroll bounds, in px — the rubber band.
const SCROLL_OVER: f32 = 48.0;

/// The minimum velocity, in px/s, on releasing a pan that starts a fling.
const PAN_FLING_MIN: f32 = 80.0;

/// The width, in physical px, of the edge zone that arms the back gesture.
const BACK_EDGE: f32 = 24.0;

/// The **drag-and-drop** (reorder) preview — paint geometry only. The shadow's
/// *colour* comes from the theme (`theme.scheme.shadow`, as `Button` does); only the
/// geometry lives here, as named constants rather than magic numbers scattered about.
mod drag_preview {
    /// The vertical offset of the ghost's drop shadow, in px.
    pub const SHADOW_OFFSET_Y: f32 = 4.0;
    /// The blur of the ghost's drop shadow, in px.
    pub const SHADOW_BLUR: f32 = 12.0;
    /// The opacity of the ghost's shadow.
    pub const SHADOW_ALPHA: f32 = 0.28;
    /// The opacity of the ghost's `primary` border.
    pub const BORDER_ALPHA: f32 = 0.9;
    /// The thickness of the ghost's `primary` border, in px.
    pub const BORDER_WIDTH: f32 = 1.5;
    /// A slight vertical lift of the ghost during a **horizontal** drag, for columns.
    pub const LIFT_Y: f32 = -2.0;
    /// The thickness of the **insertion line** in the vertical preview, in px.
    pub const INSERT_THICKNESS: f32 = 3.0;
}

/// Whether a frame builds the view again rather than repainting the tree it already has:
/// when something asked for it (`dirty`), while the application's own animations are
/// moving **and in the frame they stop in**, or for a reason of the frame's own
/// (`otherwise`: no tree yet, a switcher in flight).
///
/// The frame they stop in, because the tick that ends a transition has changed the state
/// — a push's page left behind is gone — and reports nothing moving. Without it the tree
/// hit-tested from then on is the transition's last, the page left behind still in it
/// under the finger (milestone 531).
fn frame_needs_build(dirty: bool, animating: bool, was_animating: bool, otherwise: bool) -> bool {
    dirty || animating || was_animating || otherwise
}

/// What a press is still a candidate for once it has chosen its drag: the long press a
/// widget under the finger asks for, and the item it would lift on a hold.
///
/// A long press only while the press was not captured by a drag — a scrollbar, a handle,
/// a selection; a touch scroll that has not moved yet stays a candidate. An item that
/// lifts on a hold is a candidate under a scroll too: the scroll keeps the gesture for
/// now, and the deadline decides, since a finger that stays put was never scrolling.
fn hold_candidates<Msg: Clone>(
    drag: Option<&Drag>,
    ui: Option<&frus_widgets::Ui<Msg>>,
    tree: Option<&dyn frus_widgets::Widget<Msg>>,
    at: Point,
) -> (Option<Msg>, Option<frus_widgets::DragSource>) {
    // A back gesture owns its pointer from the press that starts it, as the reference's
    // pop gesture does: nothing on either page takes a finger that is taking a page away
    // (milestone 531). The page below is not even in the frame yet when the press lands —
    // what is under the finger is whatever the frame on screen holds.
    if matches!(drag, Some(Drag::Back { .. })) {
        return (None, None);
    }
    let Some(ui) = ui else {
        return (None, None);
    };
    let free = matches!(drag, None | Some(Drag::Scroll { moved: false, .. }));
    let long_press = if free { ui.long_press_at(at) } else { None };
    let lift = ui.drag_source_at(at).filter(|source| {
        tree.and_then(|tree| find_widget(tree, source.id))
            .is_some_and(|widget| widget.drag_needs_long_press())
    });
    (long_press, lift)
}

/// The drag a press is left with when its hold's deadline fires, from the drag it had
/// and what the press found to lift.
///
/// A pending touch scroll no longer has any reason to exist — unless the hold was the
/// lift of an item or of a row, in which case the scroll hands the gesture over and what
/// was held is already up: `moved` is true because the hold *was* the threshold, and
/// asking for a movement as well would mean a row that was held and then carried
/// straight out of the list never engaged at all.
fn drag_after_hold(
    drag: Option<Drag>,
    lift: Option<frus_widgets::DragSource>,
    reorder: Option<(WidgetId, usize, Point)>,
    cursor: Point,
) -> Option<Drag> {
    // Never a back gesture's: the hold was not what that finger was doing, and taking the
    // drag from it would leave the application's gesture open with nothing to end it.
    if let Some(Drag::Back { start_x }) = drag {
        return Some(Drag::Back { start_x });
    }
    if let Some(source) = lift {
        Some(Drag::Item {
            source,
            start: cursor,
            moved: true,
            over: None,
        })
    } else if let Some((id, from, start)) = reorder {
        Some(Drag::Reorder {
            id,
            from,
            start,
            moved: true,
            carried: None,
        })
    } else {
        None
    }
}

/// A drag currently under way with the mouse.
enum Drag {
    /// A scrollbar's thumb.
    Scrollbar {
        id: WidgetId,
        vertical: bool,
        grab: f32,
        track_start: f32,
        track_len: f32,
        thumb_len: f32,
        max: f32,
        /// Whether this axis numbers its offsets from the far end, in which case the
        /// thumb's position along the track reads backwards.
        reverse: bool,
    },
    /// A text selection inside a field, with its bounds, for placement.
    TextSelect {
        id: WidgetId,
        rect: frus_widgets::Rect,
    },
    /// A **selection handle**, held (milestone 511). `grab` is the offset from the finger
    /// to the text position the handle stands for, so the selection follows the handle
    /// rather than jumping to the fingertip the moment it moves.
    SelectionHandle {
        id: WidgetId,
        rect: frus_widgets::Rect,
        handle: crate::selection::Handle,
        grab: Point,
    },
    /// A draggable widget — a slider or a handle — dragged along its horizontal axis.
    /// `last_x` is the pointer's last abscissa, so the **delta** can be delivered to
    /// the handles that accumulate, such as a column resize.
    Widget {
        id: WidgetId,
        rect: frus_widgets::Rect,
        last_x: f32,
    },
    /// Reordering a **column**: a header is grabbed (`id`, column `from`) and dropped
    /// onto another. `moved` tells a drag from a plain tap — which stays a sort —
    /// by the `TOUCH_SLOP` threshold measured from `start`.
    Reorder {
        id: WidgetId,
        from: usize,
        start: Point,
        moved: bool,
        /// What is carried and where: the row the grab moves and its box, taken from the
        /// frame the first time the drag is carried and moved with the content since, as a
        /// lifted item's box is. The frame knows a reorderable's box only while it shows, and
        /// only as much of it as shows — and carrying a row to an edge is exactly what
        /// scrolls it out of sight (milestone 527, seen on a phone).
        carried: Option<(WidgetId, Rect)>,
    },
    /// Panning an interactive viewport (`InteractiveViewer`): the pointer pushes the
    /// content. `last` is the previous position, for the delta; `moved` tells a real
    /// pan from a plain tap, by the `TOUCH_SLOP` threshold, so a click on a child can
    /// still get through. `viewport` bounds the pan to the frame. The release
    /// velocity comes from the shell's gesture tracker.
    Pan {
        id: WidgetId,
        last: Point,
        moved: bool,
        viewport: frus_widgets::Rect,
    },
    /// Scrolling a scrollable area with a finger. `moved` tells a real scroll from a
    /// plain tap, movement staying under the `TOUCH_SLOP` threshold.
    Scroll {
        id: WidgetId,
        last: Point,
        moved: bool,
        /// The speed inherited from a fling this press interrupted, per axis, in
        /// px/s — added to the release velocity so repeated swipes build momentum
        /// where the platform does that. Zero when the content was already still.
        carried: (f32, f32),
        /// A dismissible item under the finger, still in the running. Both gestures
        /// start the same way, so neither is chosen at the press: the first movement
        /// past the threshold decides by **direction**, and the loser never sees the
        /// gesture. Cleared once the scroll has won.
        dismiss: Option<frus_widgets::Dismissable>,
        /// The axis this gesture was **claimed by**, decided once at the threshold and
        /// held for the rest of the drag: `true` for vertical.
        ///
        /// A page that scrolls down must not drift sideways at the same time, and a
        /// finger never travels in a straight line. The reference settles this in its
        /// gesture arena — a vertical and a horizontal recogniser compete, the first
        /// past its own slop wins, the loser is out of the gesture entirely — and this
        /// is the same rule for an area that can go both ways. `None` until the
        /// threshold is crossed.
        axis: Option<bool>,
    },
    /// Swiping a [`frus_widgets::Dismissible`] item aside. `last` is the previous
    /// position, for the delta; the item is already past the threshold by the time this
    /// exists, since it is only ever reached by winning the direction test.
    Dismiss {
        item: frus_widgets::Dismissable,
        last: Point,
        /// `false` until the finger has passed the threshold — a press that never
        /// travels is still a tap on the row.
        moved: bool,
    },
    /// Carrying a [`frus_widgets::Draggable`] towards a [`frus_widgets::DragTarget`].
    /// `over` is the target the pointer is on now, when it would accept the payload —
    /// kept here so that entering and leaving one costs a comparison rather than a
    /// second hit test.
    Item {
        source: frus_widgets::DragSource,
        start: Point,
        /// `false` until the finger has passed the threshold: a press that never
        /// travels is a plain click on whatever is inside.
        moved: bool,
        over: Option<WidgetId>,
    },
    /// A [`frus_widgets::DraggableScrollableSheet`] moved by its panel, where nothing
    /// under the finger scrolls — its grabber, its header, a list too short to move
    /// (milestone 515). `available` is the height its shares are shares of, taken at
    /// the press: the panel may shrink to nothing under the finger, and still come back.
    Sheet {
        id: WidgetId,
        last: Point,
        /// `false` until the finger has passed the threshold: a press that never
        /// travels is a tap on whatever is in the sheet.
        moved: bool,
        available: f32,
    },
    /// The "back" gesture: the framework measures the finger's progress and velocity
    /// and passes them to the application, which decides on the navigation.
    Back { start_x: f32 },
}

/// The driver: an `event → frame` loop around an [`Application`].
/// Turns an application's `view` into a tree that is **ready to be read**.
///
/// The one way this shell builds a tree, and it exists because there were two. A
/// `ThemeBuilder` — and everything built on one, an `AppBar` included — has no children
/// until [`build_deferred`] has been over it, so every traversal of a tree that has not
/// been prepared finds nothing inside such a subtree and says nothing about it.
///
/// Both of the shell's build sites read the tree **before** the layout pass would have
/// prepared it: the frame's own path calls `collect_ids` on it for the mount and leave
/// bookkeeping, and the burst path hands it straight to `find_widget`. Neither of those is
/// a mistake to be spotted at a call site; they are why this is one function.
///
/// The surface must already be described — the reader's font setting reaches anything a
/// deferred subtree measures, and a build outside a surface measures at scale 1 while the
/// frame lays it out at whatever the reader asked for (milestone 408).
/// **The ambient answers an application gives**, read afresh every frame.
///
/// Both of these are settings rather than facts: a settings screen may turn scrollbars on
/// or change the reader's language while the application is running, and neither should
/// wait for a restart. Reading them once at start-up is the bug this shape avoids.
///
/// It is one function so that a test can drive it. The frame loop needs a window and an
/// event-loop proxy, so nothing in this repo can call the loop — which is exactly how a
/// setting that never reached production got shipped once already (milestone 408).
fn install_ambient<A: Application>(
    app: &A,
    runtime: &mut frus_widgets::Runtime,
    preferred: &[frus_widgets::Locale],
) {
    runtime.scrollbars = app.scrollbars();
    // **Which language the interface is in**, resolved from what the platform reports
    // against what the application has. Installed before the table, because an
    // application choosing its table by language reads it from here.
    frus_widgets::locale::install(app.resolved_locale(preferred));
    if let Some(table) = app.localizations() {
        frus_widgets::localizations::install(table);
    }
}

fn build_view<A: Application>(
    app: &A,
    theme: &Theme,
    runtime: &frus_widgets::Runtime,
) -> Box<dyn Widget<A::Message>> {
    debug_assert!(
        frus_widgets::MediaQuery::of().is_described(),
        "a view is being built with no surface described: install one first — milestone 416"
    );
    let tree = app.view(theme);
    build_deferred(tree.as_ref(), theme, runtime);
    tree
}

/// Where the messages an effect produces on another thread are sent: the event loop's proxy,
/// or nowhere for a driver no event loop runs.
struct Mailbox<M: 'static>(Option<EventLoopProxy<M>>);

impl<M: 'static> Clone for Mailbox<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<M: 'static> Mailbox<M> {
    /// Sends `message` to the loop. An error when the loop has closed, or when there is
    /// none — either way nobody is left to tell.
    fn send_event(&self, message: M) -> Result<(), ()> {
        match &self.0 {
            Some(proxy) => proxy.send_event(message).map_err(|_| ()),
            None => Err(()),
        }
    }
}

pub struct App<A: Application> {
    /// The application being driven: its state and logic.
    app: A,
    /// The channel that feeds back the messages effects produce, from their threads.
    proxy: Mailbox<A::Message>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    /// A renderer being initialised **asynchronously**, on the Web only: filled in by
    /// the `spawn_local` future and picked up on the first frame, the Web being unable
    /// to block on GPU init.
    #[cfg(web)]
    pending_renderer: std::rc::Rc<std::cell::RefCell<Option<Renderer>>>,
    /// The window's accessibility bridge (AccessKit) — desktop only.
    #[cfg(desktop)]
    a11y: Option<crate::a11y::A11y>,
    /// The last interface built, used for hit testing, focus and scrolling.
    ui: Option<Ui<A::Message>>,
    /// The last widget tree built, used for keyboard and editing routing.
    tree: Option<Box<dyn Widget<A::Message>>>,
    /// The pointer's last known position, in **logical** pixels.
    cursor: Point,
    /// Where the pointer that **hovers** is: a mouse once it has moved, a finger only
    /// while it touches. Kept as a place and asked of each frame — see [`crate::hover`].
    hover: crate::hover::Hover,
    /// The screen's DPI scale factor (physical = logical × scale × density).
    scale: f32,
    /// The last **logical** size handed to the app, to detect breakpoints.
    last_size: Option<(f32, f32)>,
    /// State retained between frames: hover and focus, scroll, caret and selection.
    runtime: Runtime,
    /// **The theme on display and the fade toward the one now asked for** — the
    /// framework's, not the application's, since milestone 452. It resolves the
    /// application's themes against the platform's brightness and the reader's contrast
    /// setting once a frame, and crosses between two of them over time.
    themes: crate::theming::ThemeFade,
    /// Live-reload watching (development, `FRUS_WATCH=1`): relaunch on recompilation.
    reload: Option<crate::reload::ReloadWatcher>,
    /// Overflowing boxes already named on the console, so that a layout that does not
    /// fit is reported once and not sixty times a second.
    reported_overflows: std::cell::RefCell<std::collections::HashSet<u64>>,
    /// Is the runtime inspector on? Toggled by F12, in debug builds only.
    inspector: bool,
    /// A tree dump to print on the next inspected frame.
    inspector_dump: bool,
    /// The current keyboard modifiers.
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    /// The remembered "goal" visual column for Up/Down/PgUp/PgDn: crossing shorter
    /// lines keeps the original column, the way an editor does. Cleared as soon as any
    /// other caret movement happens.
    goal_x: Option<f32>,
    /// Clipboard access: `arboard` on the desktop, the platform's on Android, the
    /// browser's on the web, nothing yet on iOS.
    clipboard: clip::Clipboard,
    /// Has the startup effect (`init`) already run? This keeps it from being replayed
    /// when the surface is recreated, as it is when Android returns from background.
    started: bool,
    /// The last frame's instant, for the animations' dt.
    last_frame: Option<Instant>,
    /// The **build** phase is dirty: the app's state, or the size, changed and the
    /// `view` must be rebuilt. The view is a pure function of `(state, theme, size)` —
    /// never of hover, scroll or focus, which live in the `Runtime` — so a frame that
    /// only animates an interaction merely **repaints** the retained tree (§1: "a hover
    /// touches paint and nothing else").
    build_dirty: bool,
    /// Whether the application's own animations were moving at the last frame. The frame
    /// they stop in is built once more: the tick that ends a transition has changed the
    /// state — the page it left is gone — and says it is no longer moving, so without it
    /// the tree hit-tested from then on is the transition's last (milestone 531).
    app_was_animating: bool,
    /// The mouse drag under way.
    drag: Option<Drag>,
    /// Was the pointer that started the drag under way a finger? A mouse is
    /// precise and needs almost no slop; a finger needs a lot.
    pointer_touch: bool,
    /// The pointer history of the drag under way, and the instant it began.
    ///
    /// One tracker for all of them: at most one drag is active at a time, and every
    /// kind of drag asks the same question on release — how fast was the finger
    /// going. Keeping it here rather than in each [`Drag`] variant means the
    /// gesture's clock and its history start together, in one place.
    gesture_velocity: VelocityTracker,
    gesture_start: Instant,
    /// The pointer's **smoothed** abscissa during a reorder: it springs toward the
    /// real position, giving the columns' sliding a gentle inertia — the background
    /// catches up with the ghost, which sticks to the pointer.
    reorder_x: f32,
    /// The **smoothed** ordinate of the **insertion line** during a **vertical**
    /// reorder, that is, Kanban cards: it springs toward the **chosen** slot edge, the
    /// hovered half, so that the line and the gap *slide* between cards instead of
    /// jumping — the vertical counterpart of the horizontal `reorder_x` spring.
    reorder_y: f32,
    /// The last **announcement** pushed to AccessKit's live region, for the screen
    /// reader. It persists as is: it is re-spoken only on a change, so the same text
    /// carried over every frame does not repeat. Desktop only.
    #[cfg(desktop)]
    announce: String,
    /// The tap-or-long-press recogniser (gesture tier 1).
    press: PressRecognizer,
    /// The pressed target's long-press message, captured on the press.
    long_press_msg: Option<A::Message>,
    /// A **text field** pressed with a finger: a hold there selects the word under it
    /// (milestone 511), where on anything else it is the widget's long press.
    pending_word: Option<WidgetId>,
    /// The focused field's caret blink (milestone 513).
    caret: crate::caret::CaretBlink,
    /// The form last shown to the platform's autofill service, as it was shown
    /// (milestone 512) — what a returned value is routed against, and what a value the
    /// service has not heard is told apart from.
    #[cfg(android)]
    autofill_reported: Vec<crate::autofill::AutofillField>,
    /// The field of that form the service was told has focus.
    #[cfg(android)]
    autofill_focus: Option<WidgetId>,
    /// The [`frus_widgets::AutofillGroup`] that form is, while it is on screen.
    #[cfg(android)]
    autofill_group: Option<WidgetId>,
    /// When the last **recorded** edit happened, for the pause that breaks a run of
    /// typing into two steps of undo. One field and not one per text field, because only
    /// the focused one is being typed into, and leaving a field ends its run anyway.
    last_edit_at: Option<Instant>,
    /// A [`frus_widgets::Draggable`] that asked to be lifted by a **hold**, waiting
    /// for the long-press deadline. Inside a scrollable this is the only way up: the
    /// plain drag belongs to the scroll, and a hold is the one signal it cannot claim.
    pending_lift: Option<frus_widgets::DragSource>,
    /// A **reorderable row** that asked to be lifted by a hold, waiting for the same
    /// deadline: `(the row, its index, where the finger landed)`.
    ///
    /// Its own field rather than a second use of `pending_lift`, because what the two
    /// become at the deadline is different — one carries a payload to a drop target, the
    /// other carries itself to a slot — but the reason they wait is the same, and it is
    /// the reason a list can still be scrolled.
    pending_reorder: Option<(WidgetId, usize, Point)>,
    /// The last click's instant, for double-click detection.
    last_click_time: Option<Instant>,
    /// A counter for the keys of leaving events, which fade out.
    leaving_counter: u64,
    /// The running subscriptions: id → cancellation handle, dropping which stops it.
    running_subs: HashMap<u64, SubHandle>,
    /// Pending focus requests — the keys `Command::focus` produced — resolved against
    /// the **freshly built** tree on the next frame.
    pending_focus: Vec<u64>,
    /// Pending **scroll** requests — the `(key, ScrollTo)` pairs `Command::scroll`
    /// produced — resolved against the frame that follows, and dropped whether or not
    /// the key named anything (see `Command::scroll`).
    pending_scroll: Vec<(u64, ScrollTo)>,
    /// The requests of the frame in progress that the **previous** frame's registry
    /// could not place — a region that has only just appeared. Tried once more against
    /// the registry this frame builds, and then gone: a request gets one frame.
    retry_scroll: Vec<(u64, ScrollTo)>,
    /// Pending **sheet** requests — the `(key, SheetTo)` pairs `Command::sheet` produced —
    /// on the terms of the scroll requests above.
    pending_sheet: Vec<(u64, SheetTo)>,
    /// The sheet requests of the frame in progress that the previous frame's sheets did
    /// not name: tried once more against this frame's.
    retry_sheet: Vec<(u64, SheetTo)>,
    /// The **focus history** of triggers, for returning focus when an overlay closes:
    /// on every focus change the old one, if still present, is pushed; when focus
    /// **vanishes** because a menu or modal closed, we go back to the most recent
    /// entry that is still present.
    focus_history: Vec<WidgetId>,
    /// The previous frame's focus, to detect the transitions worth pushing.
    prev_focus: Option<WidgetId>,
    /// The window is occluded, so rendering is suspended.
    occluded: bool,
    /// Cumulative elapsed time, in seconds, for the continuous animations.
    elapsed: f32,
    /// The last window insets handed to the app — padding plus keyboard — in logical px.
    last_insets: WindowInsets,
    /// What the **platform** last said about the person using it: the font-size slider,
    /// the night setting, the accessibility switches.
    ///
    /// Cached rather than read per frame, because reading it is a walk across the JNI
    /// boundary and the answer changes about once a year. It is refreshed when the
    /// surface appears — which on Android is also when it *reappears*, the activity being
    /// recreated on a font-scale or night change since no `configChanges` is declared —
    /// and on a theme change where the platform sends one.
    platform: PlatformSettings,
    /// The **keyboard-free** inset baseline, along with the physical size it was
    /// taken at — a rotation resets it: whatever bottom inset exceeds it is credited
    /// to the software keyboard.
    inset_baseline: Option<(Insets, (u32, u32))>,
    /// The Android activity's handle, used to query the insets, the keyboard and so on.
    /// The current **lifecycle** state, so the app is notified only on real
    /// **transitions**.
    lifecycle: Lifecycle,
    #[cfg(android)]
    android_app: Option<winit::platform::android::activity::AndroidApp>,
    /// Is the software keyboard being asked for? It follows the text fields' focus.
    #[cfg(android)]
    soft_input_shown: bool,
    /// The system bars as the platform was last told them, so that it is told only of a
    /// change (#46).
    #[cfg(android)]
    system_bars: Option<frus_widgets::SystemBars>,
}

impl<A: Application> App<A> {
    /// Creates the driver around an application and its message channel.
    pub fn new(app: A, proxy: EventLoopProxy<A::Message>) -> Self {
        Self::with_mailbox(app, Mailbox(Some(proxy)))
    }

    /// A driver with no event loop behind it, for a test that feeds it input and frames
    /// by hand: whatever an effect sends back goes nowhere. An event loop cannot be built
    /// on a machine with no display, which is where the continuous integration runs.
    #[cfg(any(test, feature = "testing"))]
    fn detached(app: A) -> Self {
        Self::with_mailbox(app, Mailbox(None))
    }

    fn with_mailbox(app: A, proxy: Mailbox<A::Message>) -> Self {
        // The widget layer knows how to *want* an image over the network and has no way
        // to get one: no runtime, no socket, and no dependency on this crate, since the
        // dependency runs the other way. So the shell says how, on the way up, and the
        // widget layer only asks. Same shape as the image decoder a step earlier.
        #[cfg(feature = "net")]
        frus_widgets::set_image_fetcher(fetch_image_bytes);
        Self {
            app,
            proxy,
            window: None,
            renderer: None,
            #[cfg(web)]
            pending_renderer: std::rc::Rc::new(std::cell::RefCell::new(None)),
            #[cfg(desktop)]
            a11y: None,
            ui: None,
            tree: None,
            cursor: Point::new(0.0, 0.0),
            hover: crate::hover::Hover::default(),
            scale: 1.0,
            last_size: None,
            runtime: Runtime::default(),
            themes: crate::theming::ThemeFade::default(),
            reload: crate::reload::ReloadWatcher::new(),
            reported_overflows: std::cell::RefCell::new(std::collections::HashSet::new()),
            inspector: false,
            inspector_dump: false,
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            goal_x: None,
            clipboard: clip::Clipboard::new(),
            started: false,
            last_frame: None,
            build_dirty: true,
            app_was_animating: false,
            drag: None,
            pointer_touch: false,
            gesture_velocity: VelocityTracker::platform_default(),
            gesture_start: Instant::now(),
            reorder_x: 0.0,
            reorder_y: 0.0,
            #[cfg(desktop)]
            announce: String::new(),
            press: PressRecognizer::new(),
            long_press_msg: None,
            pending_word: None,
            caret: crate::caret::CaretBlink::new(),
            #[cfg(android)]
            autofill_reported: Vec::new(),
            #[cfg(android)]
            autofill_focus: None,
            #[cfg(android)]
            autofill_group: None,
            last_edit_at: None,
            pending_lift: None,
            pending_reorder: None,
            last_click_time: None,
            leaving_counter: 0,
            running_subs: HashMap::new(),
            pending_focus: Vec::new(),
            pending_scroll: Vec::new(),
            retry_scroll: Vec::new(),
            pending_sheet: Vec::new(),
            retry_sheet: Vec::new(),
            focus_history: Vec::new(),
            prev_focus: None,
            occluded: false,
            elapsed: 0.0,
            last_insets: WindowInsets::ZERO,
            platform: PlatformSettings::default(),
            inset_baseline: None,
            // The app starts out detached; `resumed` will move it to `Resumed`.
            lifecycle: Lifecycle::Detached,
            #[cfg(android)]
            android_app: None,
            #[cfg(android)]
            soft_input_shown: false,
            #[cfg(android)]
            system_bars: None,
        }
    }

    /// Replays the actions an assistive technology asked for: an AT click activates
    /// the widget, exactly as a pointer click would, and an AT focus focuses it.
    #[cfg(desktop)]
    fn drain_a11y_actions(&mut self) {
        use crate::a11y::A11yAction;
        let actions = match self.a11y.as_ref() {
            Some(a11y) => a11y.take_actions(),
            None => return,
        };
        for action in actions {
            match action {
                A11yAction::Click(id) => {
                    if let Some(msg) = self.ui.as_ref().and_then(|ui| ui.msg_for(id)) {
                        self.dispatch(msg);
                        self.request_redraw();
                    }
                }
                A11yAction::Focus(id) => {
                    self.runtime.input.focused = Some(id);
                    self.runtime.focus_visible = true;
                    self.request_redraw();
                }
            }
        }
    }

    /// Keeps the **system bars** in step with the frame (#46): their colour and their icons,
    /// from the regions under them and then the theme. The platform is told only when the
    /// answer changes, telling it being a crossing to the Java UI thread — and a request
    /// that did not arrive is asked again next frame rather than remembered as made.
    ///
    /// The two points are the first row of content under the status bar and the last one
    /// above the navigation bar: this window is not drawn behind the bars, so what a bar
    /// stands against is what lies at the edge of the content next to it.
    fn sync_system_bars(&mut self, theme: &Theme) {
        #[cfg(android)]
        {
            let Some(ui) = self.ui.as_ref() else {
                return;
            };
            let Some(window) = self.window.as_ref() else {
                return;
            };
            let scale = self.total_scale();
            let size = window.inner_size();
            let (width, height) = (size.width as f32 / scale, size.height as f32 / scale);
            let edges = self.last_insets.padding;
            let bars = ui
                .system_ui_style(
                    frus_widgets::Point::new(width / 2.0, edges.top + 0.5),
                    frus_widgets::Point::new(width / 2.0, height - edges.bottom - 0.5),
                )
                .resolve(theme);
            if self.system_bars != Some(bars) && crate::android_system_bars::apply(bars) {
                self.system_bars = Some(bars);
            }
        }
        #[cfg(not(android))]
        let _ = theme;
    }

    /// Keeps the **software keyboard** in step with focus: asked for when focus is in
    /// a text field (`wants_keyboard`), closed otherwise. Called at the end of a
    /// frame, since any focus change already triggers a redraw.
    fn sync_soft_input(&mut self) {
        #[cfg(android)]
        {
            let editing = self
                .runtime
                .input
                .focused
                .and_then(|id| {
                    self.tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                })
                .is_some_and(wants_keyboard);
            if editing != self.soft_input_shown {
                self.soft_input_shown = editing;
                self.end_composition();
                if crate::android_ime::installed() {
                    // The InputConnection bridge: the Java view captures the IME.
                    if editing {
                        if let Some(id) = self.runtime.input.focused {
                            self.push_ime_context(id);
                        }
                        crate::android_ime::start_input(self.focused_ime());
                    } else {
                        crate::android_ime::clear_editor_state();
                        crate::android_ime::stop_input();
                    }
                } else if let Some(app) = &self.android_app {
                    // The TYPE_NULL fallback: a plain open, Latin keys only.
                    if editing {
                        app.show_soft_input(true);
                    } else {
                        app.hide_soft_input(false);
                    }
                }
            }
        }
    }

    /// What keyboard the **focused field** asks for.
    ///
    /// Found the same way the caret is — the focused id, resolved against the last
    /// tree built — so the two answers cannot disagree about which field is being
    /// typed into. A focus that resolves to nothing falls back to ordinary text, which
    /// is what every field got before this was asked at all.
    #[cfg(android)]
    fn focused_ime(&self) -> frus_widgets::Ime {
        self.runtime
            .input
            .focused
            .and_then(|id| {
                self.tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), id))
            })
            .map(|widget| widget.ime())
            .unwrap_or_default()
    }

    /// Notifies the app of a lifecycle **change**; never the same state twice in a row.
    fn set_lifecycle(&mut self, state: Lifecycle) {
        if self.lifecycle != state {
            self.lifecycle = state;
            self.app.on_lifecycle(state);
        }
    }

    /// **Explicitly** asks for the software keyboard again, for the focused field —
    /// called when the user **taps in a text field**, even one already focused. This is
    /// indispensable because the keyboard may have been closed by the **system back
    /// button** without the app being told: `soft_input_shown` then stays `true` and
    /// focus does not change on the next tap, so
    /// [`sync_soft_input`](Self::sync_soft_input)'s diff would see no change and never
    /// reopen the keyboard. Here it is reopened unconditionally — the native behaviour:
    /// tapping in a field shows the keyboard.
    fn request_soft_input(&mut self) {
        #[cfg(android)]
        {
            self.soft_input_shown = true;
            self.end_composition();
            if crate::android_ime::installed() {
                if let Some(id) = self.runtime.input.focused {
                    self.push_ime_context(id);
                }
                crate::android_ime::start_input(self.focused_ime());
            } else if let Some(app) = &self.android_app {
                app.show_soft_input(true);
            }
        }
    }

    /// Applies the pending IME operations to the focused field (the §6 bridge). The
    /// composition is materialised **in the field**: each update erases the previous
    /// one, the controlled model having no styled composition region yet — see
    /// docs/milestone-81.md.
    #[cfg(android)]
    fn drain_ime(&mut self) {
        use crate::ime::Step;
        let events = crate::android_ime::drain();
        if events.is_empty() {
            return;
        }
        let Some(focused) = self.runtime.input.focused else {
            return;
        };
        for event in events {
            let edit = self
                .runtime
                .edits
                .get(&focused)
                .copied()
                .unwrap_or_default();
            // Planned against the field as it stands before this operation: its text,
            // its caret and the composition it underlines (milestone 510).
            let text = self
                .tree
                .as_ref()
                .and_then(|tree| find_widget(tree.as_ref(), focused))
                .and_then(|widget| widget.text_value().map(str::to_owned))
                .unwrap_or_default();
            for step in crate::ime::plan(&event, &edit, &text) {
                match step {
                    Step::Place { cursor, anchor } => {
                        let edit = self.runtime.edits.entry(focused).or_default();
                        edit.cursor = cursor;
                        edit.anchor = anchor;
                    }
                    Step::Key(key) => self.apply_key(focused, key),
                    Step::Compose(region) => {
                        if let Some(edit) = self.runtime.edits.get_mut(&focused) {
                            edit.composing = region;
                        }
                    }
                    Step::ComposeTo(start) => {
                        if let Some(edit) = self.runtime.edits.get_mut(&focused) {
                            let end = edit.cursor;
                            edit.composing = (end > start).then_some((start, end));
                        }
                    }
                }
            }
        }
        // Refresh the input context, which the IME queries for its suggestions.
        self.push_ime_context(focused);
        self.request_redraw();
    }

    /// Keeps the platform's **autofill service** in step with the form being edited
    /// (milestone 512). When a field that says what it is for takes focus, its form is
    /// shown to the service and the service is told which field it is in; while it stays,
    /// every value the service has not heard is reported, or what it saves is what the
    /// fields held when it first looked; and when the form's group has left the screen
    /// — submitted, or navigated away from — it is committed, which is the moment a
    /// service may offer to save what was typed.
    #[cfg(android)]
    fn sync_autofill(&mut self) {
        use crate::{android_autofill as platform, autofill};
        if !crate::android_ime::installed() {
            return;
        }
        let scale = self.total_scale();
        let (Some(ui), Some(tree)) = (self.ui.as_ref(), self.tree.as_ref()) else {
            return;
        };
        let fields_of = |id: WidgetId| {
            autofill::structure(&ui.form_of(id), scale, |widget| {
                find_widget(tree.as_ref(), widget).map(|w| {
                    (
                        w.autofill_hints(),
                        w.text_value().unwrap_or_default().to_owned(),
                    )
                })
            })
        };
        // The focused field, when it takes part: it says what it is for.
        let focus = self.runtime.input.focused.filter(|&id| {
            find_widget(tree.as_ref(), id).is_some_and(|w| !w.autofill_hints().is_empty())
        });
        if focus != self.autofill_focus {
            if let Some(left) = self.autofill_focus {
                platform::exit(autofill::virtual_id(left));
            }
            if let Some(entered) = focus {
                let group = ui.form_group(entered);
                // A field of the form already shown — the next step of a wizard — keeps
                // the fields shown before it; any other form starts afresh.
                let shown: &[autofill::AutofillField] =
                    if group.is_some() && group == self.autofill_group {
                        &self.autofill_reported
                    } else {
                        &[]
                    };
                let fields = autofill::with_gone(fields_of(entered), shown);
                platform::publish(&fields);
                platform::enter(autofill::virtual_id(entered));
                self.autofill_reported = fields;
                self.autofill_group = group;
            }
            self.autofill_focus = focus;
        } else if let Some(id) = focus {
            let fields = autofill::with_gone(fields_of(id), &self.autofill_reported);
            for (virtual_id, value) in autofill::changed(&self.autofill_reported, &fields) {
                platform::value_changed(virtual_id, &value);
            }
            self.autofill_reported = fields;
        }
        if let Some(group) = self.autofill_group {
            if ui.group_stops(group).is_empty() {
                platform::commit();
                self.autofill_group = None;
                self.autofill_reported.clear();
            }
        }
    }

    /// Puts the values an autofill service chose into their fields (milestone 512), each
    /// through the message typing it would have produced — the path an undo takes, so a
    /// field that refuses typing refuses this too — with the caret after what arrived.
    #[cfg(android)]
    fn drain_autofill(&mut self) {
        let fills = crate::android_autofill::drain();
        if fills.is_empty() {
            return;
        }
        for (widget, value) in crate::autofill::route(&self.autofill_reported, fills) {
            let message = self
                .tree
                .as_ref()
                .and_then(|tree| find_widget(tree.as_ref(), widget))
                .and_then(|w| w.replace_value(value.clone()));
            if let Some(message) = message {
                self.runtime.edits.insert(
                    widget,
                    Edit {
                        cursor: value.chars().count(),
                        anchor: None,
                        composing: None,
                    },
                );
                self.dispatch_edit(message);
            }
        }
        if let Some(id) = self.runtime.input.focused {
            self.push_ime_context(id);
        }
        self.request_redraw();
    }

    /// Publishes field `id`'s editing state to the bridge — text, caret and selection
    /// — the context the IME reads for its suggestions.
    #[cfg(android)]
    fn push_ime_context(&self, id: WidgetId) {
        let value = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.text_value().map(|s| s.to_string()));
        if let Some(text) = value {
            let edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
            crate::android_ime::set_editor_state(&text, edit.cursor, edit.selection_range());
            // And the keyboard is **told**, as an Android editor tells it on every change:
            // a keyboard that predicts keeps its own model of the field, and one never
            // told of a change drifts from it (milestone 510). In UTF-16 units.
            let len = text.chars().count();
            let at = |i: usize| crate::ime::utf16_index(&text, i.min(len)) as i32;
            let (start, end) = edit.selection_range().unwrap_or((edit.cursor, edit.cursor));
            let (cand_start, cand_end) = edit
                .composing
                .map(|(s, e)| (at(s), at(e)))
                .unwrap_or((-1, -1));
            crate::android_ime::update_selection(at(start), at(end), cand_start, cand_end);
        }
    }

    /// Ends the focused field's composition where it stands: the keyboard is being
    /// started again, and a keyboard started again has forgotten it.
    #[cfg(android)]
    fn end_composition(&mut self) {
        if let Some(id) = self.runtime.input.focused {
            if let Some(edit) = self.runtime.edits.get_mut(&id) {
                edit.composing = None;
            }
        }
    }

    /// Remembers the Android activity's handle, the source of the system insets.
    #[cfg(android)]
    pub(crate) fn set_android_app(
        &mut self,
        android_app: winit::platform::android::activity::AndroidApp,
    ) {
        self.android_app = Some(android_app);
    }

    /// The system insets — the safe area — in **logical** px. On Android they are
    /// derived from the activity's content rect, outside the system bars; zero
    /// elsewhere.
    ///
    /// The content rect is what the system leaves the activity, so it excludes
    /// **any** decoration the theme asks for — a title bar included. An app whose
    /// manifest keeps the default theme therefore reports a top inset of the status
    /// bar *plus* 56dp of action bar that is never drawn, and the shell dutifully
    /// pads it away: a wide empty band above the app bar. The manifests here ask for
    /// `Theme.DeviceDefault.NoActionBar`, and so must any frus app.
    fn compute_insets(&self, phys_w: u32, phys_h: u32, scale: f32) -> Insets {
        #[cfg(android)]
        if let Some(app) = &self.android_app {
            let r = app.content_rect();
            // A degenerate rect, before the first layout, means no inset.
            if r.right > r.left && r.bottom > r.top {
                let left = r.left.max(0) as f32;
                let top = r.top.max(0) as f32;
                let right = (phys_w as i32 - r.right).max(0) as f32;
                let bottom = (phys_h as i32 - r.bottom).max(0) as f32;
                return Insets::new(top / scale, right / scale, bottom / scale, left / scale);
            }
        }
        let _ = (phys_w, phys_h, scale);
        Insets::ZERO
    }

    /// Asks the platform what it says about its user, and remembers the answer.
    ///
    /// Marks the build dirty when something moved: the font size reaches the layout
    /// through [`MediaQuery::scope`], so a changed scaler is a changed geometry for every
    /// widget on the screen, and a frame drawn from the cache would keep the old one.
    fn refresh_platform_settings(&mut self) {
        #[cfg(android)]
        if let Some(app) = &self.android_app {
            let read = crate::android_settings::read(app);
            if read != self.platform {
                self.platform = read;
                self.build_dirty = true;
            }
            return;
        }
        // Desktop and web report the one thing their window system knows. A text scaler
        // is **not** among it: winit exposes no equivalent of Windows'
        // `UISettings.TextScaleFactor` or of GNOME's `text-scaling-factor`, so it stays
        // at 1 there and an application that knows better says so itself. That is a
        // missing wire and is written down rather than papered over.
        let brightness = self
            .window
            .as_ref()
            .and_then(|window| window.theme())
            .map(|theme| match theme {
                winit::window::Theme::Dark => frus_widgets::Brightness::Dark,
                winit::window::Theme::Light => frus_widgets::Brightness::Light,
            })
            .unwrap_or_default();
        if brightness != self.platform.brightness {
            self.platform.brightness = brightness;
            self.build_dirty = true;
        }

        // **The reader's languages**, read once. Changing the display language on a
        // desktop means signing out and back in on Windows, and is a per-process
        // environment variable on Linux — so there is nothing to watch for, and re-reading
        // it every time the window comes back would allocate for an answer that cannot
        // have moved. Android is the platform where it *does* change under a running
        // application, and Android reads it on every walk.
        if self.platform.locales.is_empty() {
            let read = platform_locales();
            if !read.is_empty() {
                self.platform.locales = read;
                self.build_dirty = true;
            }
        }
    }
}

/// **The reader's preferred languages**, best first, from whatever the platform keeps
/// them in. Empty when it says nothing, or when there is no way to ask.
///
/// Android does not come through here: its answer lives on the activity's
/// `Configuration`, which [`crate::android_settings`] already walks.
/// On **Android** there is nothing to ask here: the answer lives on the activity's
/// `Configuration`, and the branch above returns before reaching this. A build with no
/// activity at all has no reader to ask about.
#[cfg(android)]
fn platform_locales() -> Vec<frus_widgets::Locale> {
    Vec::new()
}

#[cfg(not(android))]
fn platform_locales() -> Vec<frus_widgets::Locale> {
    #[cfg(not(web))]
    {
        sys_locale::get_locales()
            .filter_map(|tag| frus_widgets::Locale::parse(&tag))
            .collect()
    }
    #[cfg(web)]
    {
        // `navigator.languages` is the ordered list; `navigator.language` is its first
        // entry and the fallback for a browser that does not offer the list.
        let Some(navigator) = web_sys::window().map(|window| window.navigator()) else {
            return Vec::new();
        };
        let listed: Vec<frus_widgets::Locale> = navigator
            .languages()
            .iter()
            .filter_map(|value| value.as_string())
            .filter_map(|tag| frus_widgets::Locale::parse(&tag))
            .collect();
        if listed.is_empty() {
            return navigator
                .language()
                .and_then(|tag| frus_widgets::Locale::parse(&tag))
                .into_iter()
                .collect();
        }
        listed
    }
}

impl<A: Application> ApplicationHandler<A::Message> for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // Back in the foreground, or starting up: the surface is (re)born below.
        self.set_lifecycle(Lifecycle::Resumed);
        // And so is what the platform says about its user — a trip to the system
        // settings and back is exactly this event.
        self.refresh_platform_settings();

        let mut attributes = Window::default_attributes()
            .with_title(self.app.title())
            // A sensible minimum size, in logical px, to avoid an absurd UI.
            .with_min_inner_size(winit::dpi::LogicalSize::new(360.0, 280.0));
        // The AccessKit adapter must be created **before** the window is shown, so we
        // create it hidden and reveal it afterwards. Desktop only.
        #[cfg(desktop)]
        {
            attributes = attributes.with_visible(false);
        }
        if let Some((w, h)) = self.app.window_size() {
            attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(w, h));
        }
        // Web: winit creates a `<canvas>` and **appends it** to the document's body.
        #[cfg(web)]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            attributes = attributes.with_append(true);
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("failed to create the window: {err}");
                event_loop.exit();
                return;
            }
        };

        // Web: GPU init is **asynchronous**, blocking being impossible. We start the
        // future, and the ready renderer is picked up on the first frame — see
        // `RedrawRequested`. `init` runs right away, since it does not touch the GPU.
        #[cfg(web)]
        {
            self.scale = window.scale_factor() as f32;
            self.window = Some(window.clone());
            self.build_dirty = true;
            if !self.started {
                self.started = true;
                let command = self.app.init();
                self.run_command(command);
                self.sync_subscriptions();
            }
            let slot = self.pending_renderer.clone();
            let win = window.clone();
            let size = window.inner_size();
            wasm_bindgen_futures::spawn_local(async move {
                match Renderer::new(win.clone(), size.width.max(1), size.height.max(1)).await {
                    Ok(r) => {
                        *slot.borrow_mut() = Some(r);
                        win.request_redraw();
                    }
                    Err(err) => log::error!("failed to initialise the renderer (Web): {err:#}"),
                }
            });
            return;
        }

        #[cfg(not(web))]
        {
            let size = window.inner_size();
            let renderer = pollster::block_on(Renderer::new(
                window.clone(),
                size.width.max(1),
                size.height.max(1),
            ));

            match renderer {
                Ok(renderer) => {
                    self.scale = window.scale_factor() as f32;
                    // The accessibility bridge (AccessKit), created while the window is
                    // still hidden, after which we reveal it. Inert with no screen reader.
                    #[cfg(desktop)]
                    {
                        self.a11y = Some(crate::a11y::A11y::new(event_loop, &window));
                        window.set_visible(true);
                    }
                    self.window = Some(window.clone());
                    // What the input bridge's wakes ask a frame of (milestone 513).
                    #[cfg(android)]
                    crate::android_ime::set_window(Some(window.clone()));
                    self.renderer = Some(renderer);
                    // The surface was (re)created: force a full rebuild on the first frame.
                    self.build_dirty = true;
                    // The startup effect, an initial load and so on: once only, not on
                    // every surface recreation, such as returning from the background.
                    if !self.started {
                        self.started = true;
                        let command = self.app.init();
                        self.run_command(command);
                        self.sync_subscriptions();
                    }
                    window.request_redraw();
                }
                Err(err) => {
                    log::error!("failed to initialise the renderer: {err:#}");
                    event_loop.exit();
                }
            }
        }
    }

    /// Going to the background, on Android: the native surface is destroyed. We
    /// release the renderer and the window, and `resumed` recreates them on the way
    /// back, without replaying `init` — see `started`. Harmless on desktop, where the
    /// event never fires.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Background: the surface is lost, so the app moves to `Paused`.
        self.set_lifecycle(Lifecycle::Paused);
        self.renderer = None;
        self.window = None;
        #[cfg(android)]
        crate::android_ime::set_window(None);
        self.last_frame = None;
    }

    /// The event loop is ending, on close: a final `Detached` notification, where the
    /// app can persist its state before the process disappears.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.set_lifecycle(Lifecycle::Detached);
    }

    /// A message produced by an effect, on a background thread: we apply it and ask
    /// for another frame.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, message: A::Message) {
        self.dispatch(message);
        self.request_redraw();
    }

    /// The loop woke up: if the long-press deadline has been reached the recogniser
    /// **accepts eagerly** — the message is emitted and the release to come will be
    /// swallowed, the long press having evicted the tap.
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) && self.press.poll(Instant::now())
        {
            self.hold_deadline_reached();
        }

        // The caret's turn is due: a frame to show it, or to hide it, in (milestone 513).
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && self.lifecycle == Lifecycle::Resumed
            && self.caret.next_toggle(Instant::now()).is_some()
        {
            self.request_redraw();
        }

        // Pending IME operations, from the Android input bridge.
        #[cfg(android)]
        self.drain_ime();
        // And the values an autofill service chose, from the same bridge.
        #[cfg(android)]
        self.drain_autofill();

        // Actions an assistive technology asked for, through AccessKit.
        #[cfg(desktop)]
        self.drain_a11y_actions();

        // Live reload, in development: the binary was replaced by a recompilation, so
        // capture the state and relaunch the new binary. This does not return.
        if let Some(watcher) = self.reload.as_mut() {
            if watcher.binary_changed() {
                watcher.handoff(self.app.save_state());
            }
        }
        event_loop.set_control_flow(self.idle_control_flow());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // The AccessKit adapter observes the window events: focus, size and so on.
        #[cfg(desktop)]
        if let (Some(a11y), Some(window)) = (self.a11y.as_mut(), self.window.as_ref()) {
            a11y.process_event(window, &event);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            // Gaining or losing focus **in the foreground**: `Resumed` ⇄ `Inactive`. We
            // leave `Paused`/`Detached` — background and closing — alone; those are
            // decided by `suspended` and `exiting`.
            WindowEvent::Focused(focused) => {
                if !matches!(self.lifecycle, Lifecycle::Paused | Lifecycle::Detached) {
                    self.set_lifecycle(if focused {
                        Lifecycle::Resumed
                    } else {
                        Lifecycle::Inactive
                    });
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor as f32;
                // Reconfigure the surface to the current physical size.
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer.resize(size.width, size.height);
                    }
                }
                self.request_redraw();
            }

            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if !occluded {
                    self.request_redraw();
                }
            }

            // Every pointer source, mouse and touch alike, converges on the
            // **normalised** `pointer()` input — gesture tier 0, with an explicit
            // `Cancel`. winit hands us physical px; we work in logical ones, the total
            // scale being DPI × density.
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.total_scale();
                let position = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                self.pointer(
                    event_loop,
                    PointerEvent {
                        kind: PointerKind::Move,
                        position,
                        touch: false,
                    },
                );
            }

            // The mouse left the window: nothing in it is under the pointer any more.
            WindowEvent::CursorLeft { .. } => {
                self.hover.left();
                self.sync_hover();
            }

            WindowEvent::Touch(touch) => {
                let scale = self.total_scale();
                let position = Point::new(
                    touch.location.x as f32 / scale,
                    touch.location.y as f32 / scale,
                );
                let kind = match touch.phase {
                    TouchPhase::Started => PointerKind::Down,
                    TouchPhase::Moved => PointerKind::Move,
                    TouchPhase::Ended => PointerKind::Up,
                    TouchPhase::Cancelled => PointerKind::Cancel,
                };
                self.pointer(
                    event_loop,
                    PointerEvent {
                        kind,
                        position,
                        touch: true,
                    },
                );
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.shift = state.shift_key();
                self.ctrl = state.control_key();
                self.alt = state.alt_key();
                self.meta = state.super_key();
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.pointer(
                event_loop,
                PointerEvent {
                    kind: PointerKind::Down,
                    position: self.cursor,
                    touch: false,
                },
            ),

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.pointer(
                event_loop,
                PointerEvent {
                    kind: PointerKind::Up,
                    position: self.cursor,
                    touch: false,
                },
            ),

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                // A keyboard interaction: the focus ring becomes visible again.
                if !self.runtime.focus_visible {
                    self.runtime.focus_visible = true;
                    self.request_redraw();
                }

                // The **system** back — Android's button or gesture, the browser's back
                // key: it closes the topmost overlay, else pops a screen, else quits.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::BrowserBack) | WinitKey::Named(NamedKey::GoBack)
                ) {
                    if !event.repeat {
                        self.system_back(event_loop);
                    }
                    return;
                }

                // F12 toggles the **inspector**: outlines, a card for the hovered
                // widget, and a tree dump on stderr. Debug builds only.
                if matches!(event.logical_key, WinitKey::Named(NamedKey::F12)) {
                    if cfg!(debug_assertions) && !event.repeat {
                        self.inspector = !self.inspector;
                        self.inspector_dump = self.inspector;
                        self.request_redraw();
                    }
                    return;
                }

                // **Shortcuts.** Placed after the system keys the shell owns outright
                // (back, F12) and before everything an application can bind, so that a
                // binding cannot take the inspector or the back gesture away.
                //
                // A stroke with no Ctrl, Alt or Meta goes to a focused field first: a
                // binding on a bare letter would otherwise make every field under it
                // impossible to type in.
                if let Some(stroke) = self.keystroke(&event) {
                    let typing = !stroke.is_command() && self.runtime.input.focused.is_some();
                    if !typing {
                        let msgs = self
                            .ui
                            .as_ref()
                            .map(|ui| ui.keystroke(stroke, self.runtime.input.focused))
                            .unwrap_or_default();
                        if !msgs.is_empty() {
                            for msg in msgs {
                                self.dispatch(msg);
                            }
                            self.request_redraw();
                            return;
                        }
                    }
                }

                // Tab and Shift+Tab move between focusables, even with nothing focused.
                if matches!(event.logical_key, WinitKey::Named(NamedKey::Tab)) {
                    let forward = !self.shift;
                    let next = self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.focus_next(self.runtime.input.focused, forward));
                    if next.is_some() {
                        self.runtime.input.focused = next;
                        self.reveal_focus();
                        self.request_redraw();
                    }
                    return;
                }

                // Escape walks **leaf to root** from the focused widget — an `OverlayPortal`
                // consumes it to close itself — and failing that closes the topmost
                // overlay, so Escape works with nothing focused.
                if matches!(event.logical_key, WinitKey::Named(NamedKey::Escape)) {
                    // Auto-repeat does not trigger another close.
                    if !event.repeat {
                        self.escape();
                    }
                    return;
                }

                let Some(focused) = self.runtime.input.focused else {
                    return;
                };

                // Arrows navigate focus **geometrically** — except left and right in a
                // text field, where they move the caret; up and down navigate even out
                // of a single-line field.
                let arrow = match event.logical_key {
                    WinitKey::Named(NamedKey::ArrowUp) => Some(FocusDirection::Up),
                    WinitKey::Named(NamedKey::ArrowDown) => Some(FocusDirection::Down),
                    WinitKey::Named(NamedKey::ArrowLeft) => Some(FocusDirection::Left),
                    WinitKey::Named(NamedKey::ArrowRight) => Some(FocusDirection::Right),
                    _ => None,
                };
                // PgUp and PgDn jump a **page** inside a multi-line field, bounded to
                // the field so they never leave it. No effect anywhere else.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::PageUp) | WinitKey::Named(NamedKey::PageDown)
                ) {
                    let down = matches!(event.logical_key, WinitKey::Named(NamedKey::PageDown));
                    if self.move_caret_vertical(focused, down, true) {
                        return;
                    }
                }

                if let Some(direction) = arrow {
                    // In a **multi-line** field, Up and Down move the caret between
                    // lines while it stays in the field; on the first or last line we
                    // fall back to focus navigation and leave the field.
                    if matches!(direction, FocusDirection::Up | FocusDirection::Down) {
                        let down = matches!(direction, FocusDirection::Down);
                        if self.move_caret_vertical(focused, down, false) {
                            return;
                        }
                    }

                    // Left and right arrows are offered to the focused widget first —
                    // a range slider, say — through `on_key`. If it consumes them, focus
                    // does not move.
                    if matches!(direction, FocusDirection::Left | FocusDirection::Right) {
                        let key = if matches!(direction, FocusDirection::Left) {
                            Key::Left {
                                shift: self.shift,
                                word: self.ctrl,
                            }
                        } else {
                            Key::Right {
                                shift: self.shift,
                                word: self.ctrl,
                            }
                        };
                        let widget = self
                            .tree
                            .as_ref()
                            .and_then(|tree| find_widget(tree.as_ref(), focused));
                        // A reorderable header moves its column by one; we capture its
                        // position to announce it, the keyboard being how screen-reader
                        // users reorder.
                        let reorder_from = widget.and_then(|w| w.reorder_index());
                        let handled = widget.map(|w| w.on_key(&key));
                        if let Some(KeyResponse::Handled(message)) = handled {
                            if let Some(message) = message {
                                self.dispatch(message);
                            }
                            if let Some(from) = reorder_from {
                                let to = if matches!(direction, FocusDirection::Left) {
                                    from.wrapping_sub(1)
                                } else {
                                    from + 1
                                };
                                self.set_announcement(format!(
                                    "Column moved to position {}",
                                    to + 1
                                ));
                            }
                            self.request_redraw();
                            return;
                        }
                    }

                    let is_text = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .and_then(|widget| widget.cursor_at(0.0, 0.0, 1.0, 0))
                        .is_some();
                    let navigates =
                        !is_text || matches!(direction, FocusDirection::Up | FocusDirection::Down);
                    if navigates {
                        if let Some(next) = self
                            .ui
                            .as_ref()
                            .and_then(|ui| ui.focus_directional(focused, direction))
                        {
                            self.runtime.input.focused = Some(next);
                            self.reveal_focus();
                            self.request_redraw();
                        }
                        return;
                    }
                }

                // Home and End are offered to the focused widget — a range slider maps
                // them to min and max — before the default action. A text field ignores
                // them here (`on_key` returns Ignored) and falls back to ordinary
                // editing further down.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::Home) | WinitKey::Named(NamedKey::End)
                ) {
                    let key = if matches!(event.logical_key, WinitKey::Named(NamedKey::Home)) {
                        Key::Home {
                            shift: self.shift,
                            doc: self.ctrl,
                        }
                    } else {
                        Key::End {
                            shift: self.shift,
                            doc: self.ctrl,
                        }
                    };
                    let handled = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .map(|widget| widget.on_key(&key));
                    if let Some(KeyResponse::Handled(message)) = handled {
                        if let Some(message) = message {
                            self.dispatch(message);
                        }
                        self.request_redraw();
                        return;
                    }
                }

                // Keyboard activation, Enter or Space, of a clickable focusable: a
                // button, a checkbox, a switch. Text fields, which have no `on_click`,
                // fall back to ordinary editing — Enter submits, Space is a space.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::Enter) | WinitKey::Named(NamedKey::Space)
                ) {
                    let widget = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .filter(|widget| widget.focusable());
                    let message = widget.and_then(|widget| widget.on_click());
                    if let Some(message) = message {
                        // Auto-repeat does not machine-gun the activation: holding
                        // Space on a button is one single click.
                        if !event.repeat {
                            // The spoken announcement of the effect — a sort, a
                            // selection — captured before `dispatch` rebuilds the tree.
                            let announce = widget.and_then(|widget| widget.announce());
                            self.dispatch(message);
                            if let Some(announce) = announce {
                                self.set_announcement(announce);
                            }
                            self.request_redraw();
                        }
                        return;
                    }
                }

                // The clipboard: Ctrl+C/X/V, and a keyboard's own Copy, Cut and Paste.
                if let Some(command) =
                    clipboard_command(&event.logical_key, event.physical_key, self.ctrl)
                {
                    match command {
                        ClipCommand::Copy => self.copy_selection(focused),
                        ClipCommand::Cut => {
                            self.copy_selection(focused);
                            self.apply_key(focused, Key::Backspace);
                            self.request_redraw();
                        }
                        ClipCommand::Paste => {
                            // Answered now, or — on the Web — on a later frame.
                            if let Some(pasted) =
                                self.clipboard.paste(focused, self.window.as_ref())
                            {
                                self.land_paste(pasted);
                            }
                        }
                    }
                    return;
                }

                // The other editing shortcuts, Ctrl+A/Z/Y.
                if self.ctrl {
                    match &event.logical_key {
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("a") => {
                            self.runtime.edits.insert(
                                focused,
                                Edit {
                                    cursor: usize::MAX,
                                    anchor: Some(0),
                                    composing: None,
                                },
                            );
                            // A selection is a caret move: what is typed over it is a step
                            // of its own, not more of whatever was being typed before.
                            self.runtime.close_edit_run(focused);
                            self.request_redraw();
                            return;
                        }
                        // Undo, and redo under both its spellings — Ctrl+Y on Windows,
                        // Ctrl+Shift+Z everywhere else, and people bring the one their
                        // hands already know.
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("z") && !self.shift => {
                            self.step_history(focused, false);
                            self.request_redraw();
                            return;
                        }
                        WinitKey::Character(c)
                            if c.eq_ignore_ascii_case("y")
                                || (c.eq_ignore_ascii_case("z") && self.shift) =>
                        {
                            self.step_history(focused, true);
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                // With the input bridge active, editing goes EXCLUSIVELY through the
                // InputConnection, that is, the IME queue — the bridge view receives the
                // hardware keys too. Without this guard every keystroke would arrive
                // twice, once from winit's native queue and once from the bridge.
                #[cfg(android)]
                if crate::android_ime::installed() {
                    return;
                }

                let shift = self.shift;
                let key = match &event.logical_key {
                    WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
                    WinitKey::Named(NamedKey::Delete) => Some(Key::Delete),
                    // Repeating Enter does not submit again; text and deletion do
                    // repeat normally.
                    WinitKey::Named(NamedKey::Enter) if !event.repeat => Some(Key::Enter),
                    WinitKey::Named(NamedKey::Enter) => None,
                    // With Ctrl, Left/Right jump a word and Home/End bound the field.
                    WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left {
                        shift,
                        word: self.ctrl,
                    }),
                    WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right {
                        shift,
                        word: self.ctrl,
                    }),
                    WinitKey::Named(NamedKey::Home) => Some(Key::Home {
                        shift,
                        doc: self.ctrl,
                    }),
                    WinitKey::Named(NamedKey::End) => Some(Key::End {
                        shift,
                        doc: self.ctrl,
                    }),
                    WinitKey::Named(NamedKey::Space) => Some(Key::Text(" ".to_string())),
                    // Android delivers Enter as `Character("\n")`, through the
                    // KeyCharacterMap, rather than `Named(Enter)`: the same submission,
                    // without inserting a line break into the field.
                    WinitKey::Character(c) if c == "\n" || c == "\r" => {
                        if event.repeat {
                            None
                        } else {
                            Some(Key::Enter)
                        }
                    }
                    _ => event.text.as_ref().map(|text| Key::Text(text.to_string())),
                };

                if let Some(key) = key {
                    self.apply_key(focused, key);
                    self.request_redraw();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let (mut dx, mut dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * SCROLL_SPEED, y * SCROLL_SPEED),
                    // Physical delta → logical.
                    MouseScrollDelta::PixelDelta(pos) => {
                        let scale = self.total_scale();
                        (pos.x as f32 / scale, pos.y as f32 / scale)
                    }
                };
                // With Shift, the wheel scrolls horizontally.
                if self.shift {
                    dx = dy;
                    dy = 0.0;
                }
                // Over an interactive viewport the wheel **zooms**, anchored at the
                // pointer and bounded by the widget's min and max scales.
                if let Some((id, viewport)) = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.interactive_at(self.cursor))
                {
                    let (min, max) = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                        .and_then(|w| w.interactive())
                        .unwrap_or((0.5, 4.0));
                    // Wheel up (`dy > 0`) zooms in, in gentle steps of about 1.1× a notch.
                    let factor = (1.0 + dy * 0.1 / SCROLL_SPEED).clamp(0.2, 5.0);
                    // Zooming cuts off a fling in progress.
                    self.runtime.interactive_velocity.remove(&id);
                    let view = self.runtime.interactive.entry(id).or_default();
                    // Zoom anchored at the pointer, then bounded to the frame.
                    *view = view
                        .zoom_at(factor, self.cursor, min, max)
                        .clamped(viewport);
                    self.request_redraw();
                    return;
                }
                // A wheel asks the same question a finger does, and the reference gates it
                // the same way: an area with everything already in sight takes no offset,
                // and the notch belongs to whatever is behind it.
                let hit = {
                    let scroll = &self.runtime.scroll;
                    self.ui.as_ref().and_then(|ui| {
                        ui.scroll_chain(self.cursor).find(|area| {
                            area.accepts_user_offset(
                                scroll.get(&area.id).copied().unwrap_or((0.0, 0.0)),
                            )
                        })
                    })
                };
                if let Some(area) = hit {
                    // Scrolling with inertia: the wheel pushes the TARGET and the
                    // spring eases across to it. How far past the ends that target
                    // may go is the physics' call — a little elastic overshoot where
                    // the platform bounces, none at all where it does not.
                    let id = area.id;
                    let physics = area.physics_or(self.app.scroll_physics());
                    let over = if physics.allows_overscroll() {
                        SCROLL_OVER
                    } else {
                        0.0
                    };
                    // A notch of the wheel is a new intent: it takes over from the
                    // momentum of the last gesture.
                    self.runtime.stop_scroll_fling(id);
                    let current = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
                    // A screen delta into offsets: the content moves opposite the number,
                    // and a reversed axis counts from the other end. One function, so a
                    // reversed area is right at every one of the places this happens.
                    let (ddx, ddy) = area.offset_delta((dx, dy));
                    let target = self.runtime.scroll_target.entry(id).or_insert(current);
                    let (wanted_x, wanted_y) = (target.0 + ddx, target.1 + ddy);
                    target.0 = wanted_x.clamp(-over, area.max_x + over);
                    target.1 = wanted_y.clamp(-over, area.max_y + over);
                    let refused = (wanted_x - target.0, wanted_y - target.1);
                    self.runtime.scroll_velocity.entry(id).or_insert((0.0, 0.0));
                    // A notch that asks to go past the end deserves the same
                    // acknowledgement a finger gets: the wheel is a gesture too.
                    let cursor = self.cursor;
                    for (refused, vertical, extent) in [
                        (refused.0, false, area.viewport.width),
                        (refused.1, true, area.viewport.height),
                    ] {
                        if refused.abs() < 1e-3 {
                            continue;
                        }
                        let edge = area.refused_edge(vertical, refused);
                        let (offset, cross) =
                            frus_widgets::glow_cross_axis(area.viewport, edge, cursor);
                        self.runtime
                            .glow_pull(id, edge, refused, extent, offset, cross);
                        // A wheel has no "lift off", so the pull is released at once
                        // and simply fades — otherwise it would hang until the hold
                        // timer expired.
                        self.runtime.glow_scroll_end(id);
                    }
                    self.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                // Web: pick up the asynchronously initialised renderer as soon as it
                // is ready; until then, nothing is painted.
                #[cfg(web)]
                if self.renderer.is_none() {
                    match self.pending_renderer.borrow_mut().take() {
                        Some(mut renderer) => {
                            // The renderer was sized from whatever `inner_size()` read
                            // back when it was spawned — often 0×0, before the canvas
                            // had been laid out by CSS. A `Resized` event carrying the
                            // real size may have arrived and been dropped in the
                            // meantime, since `self.renderer` was still `None` then.
                            // Catching up here, against the size right now, is what
                            // makes that race harmless either way.
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                renderer.resize(size.width.max(1), size.height.max(1));
                            }
                            self.renderer = Some(renderer);
                        }
                        None => return,
                    }
                }
                // A paste the browser has answered since the last frame goes in before
                // this one is built. Elsewhere a paste is answered on the spot and this
                // finds nothing.
                while let Some(pasted) = self.clipboard.take_answered() {
                    self.land_paste(pasted);
                }
                // Occluded window: rendering is suspended, resuming on Occluded(false).
                if self.occluded {
                    return;
                }
                let size = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .unwrap_or_default();
                // Minimised, or zero-sized: nothing to draw, which avoids GPU errors.
                if size.width == 0 || size.height == 0 {
                    self.last_frame = None; // no dt jump when it is restored
                    return;
                }
                // The interface is described in **logical** pixels; the GPU output is
                // scaled to physical ones (DPI × density) just before rendering.
                let scale = self.total_scale();
                let (width, height) = (size.width as f32 / scale, size.height as f32 / scale);

                // A logical size change — a resize OR a density change — notifies the
                // app before the view, so it can react to the breakpoint in its logic.
                if self.last_size != Some((width, height)) {
                    self.last_size = Some((width, height));
                    self.build_dirty = true;
                    self.app.on_resize(width, height);
                }

                // Window insets: separates the static **padding** — bars and notch —
                // from the **keyboard**, which is whatever bottom inset exceeds the
                // keyboard-free baseline. The baseline is taken at the first measurement
                // for this physical size (a rotation gives a new one), and corrects
                // downwards should a barer state appear: the keyboard open at startup,
                // the bars hidden, and so on.
                let raw = self.compute_insets(size.width, size.height, scale);
                let phys = (size.width, size.height);
                let mut baseline = match self.inset_baseline {
                    Some((b, s)) if s == phys => b,
                    _ => {
                        self.inset_baseline = Some((raw, phys));
                        raw
                    }
                };
                if raw.bottom < baseline.bottom {
                    baseline = raw;
                    self.inset_baseline = Some((raw, phys));
                }
                let insets = WindowInsets::from_baseline(baseline, raw);
                if self.last_insets != insets {
                    self.last_insets = insets;
                    self.build_dirty = true;
                    self.app.on_insets(insets);
                }

                // The elapsed dt, clamped, for every animation.
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|prev| (now - prev).as_secs_f32().min(0.05))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);

                // A continuous clock, in seconds, for the time-driven animations.
                self.elapsed += dt;
                self.runtime.time = self.elapsed;

                // The caret's blink (milestone 513), on the wall clock: the animation
                // clock above is clamped per frame and stands still between frames, and a
                // caret at rest is exactly when there are none. Solid while the window is
                // not the one in front, where nothing wakes the loop for it.
                let caret_now = Instant::now();
                let caret = if self.lifecycle == Lifecycle::Resumed {
                    self.caret_signature()
                } else {
                    None
                };
                self.caret.observe(caret_now, caret);
                self.runtime.caret_hidden = self.caret.hidden(caret_now);

                // The application advances its own animations: navigation, gesture.
                let mut app_animating = self.app.tick(dt);

                // And the framework advances the **theme**, which is its own work: it
                // resolves the application's themes against the brightness the platform
                // reports and the contrast the reader asked for, and crosses from the one
                // on screen to the one now wanted over `theme_animation_duration`.
                //
                // Both halves of that were the application's before milestone 452, and
                // both were being done by hand in this repo's own demonstration — which
                // is a fair sign of what every application was having to write.
                let settings = self
                    .platform
                    .accessibility
                    .with_overrides(self.app.accessibility());

                // **The ambient scopes, before anything reads them** — which the theme
                // does: the direction of the layout follows the language, so an
                // application whose theme is right-to-left in Arabic needs the language
                // installed before its theme is asked for. This used to sit further down,
                // beside the build, where it was in time for the widgets and one frame
                // late for the theme.
                //
                // Read every frame, all of it: these are the application's answers and
                // they may change while it is running.
                install_ambient(&self.app, &mut self.runtime, &self.platform.locales);

                let theme_moved = self.themes.advance(
                    &self.app,
                    self.platform.brightness,
                    settings.high_contrast,
                    settings.disable_animations,
                    dt,
                );
                // A theme that moved is a rebuild — the view is a pure function of
                // `(state, theme, size)` — and a fade still running is another frame.
                app_animating |= theme_moved | self.themes.animating();
                let theme = self.themes.displayed(&self.app);

                // **The surface, in force for the whole frame** — not just for `view`.
                // A size becomes a number in three places: while the widgets are built,
                // while they are measured and laid out, and while they are painted.
                // Scoping the build alone left the last two at scale 1, so the layout
                // measured one size and the renderer drew another, and the reader's
                // setting reached a device without moving a single pixel (milestone 407).
                //
                // One guard, not two: the description and the font size are installed by
                // the same call and released by the same drop, so they cannot be held for
                // different lengths of time. Holding them separately is the same bug with
                // an extra step (milestone 408). It lives to the end of this frame and
                // puts back what was there, panic or not.
                let _surface = self.media_query(width, height).install();

                // === BUILD phase, conditional ===
                // The `view` is rebuilt only when the app's state or the size changed
                // (`build_dirty`), or when the app is animating, theme, navigation and
                // gesture all altering the state the view reads. A frame that animates
                // interaction alone — hover, scroll, focus, caret — **reuses the
                // retained tree** and merely repaints: the `view` is a pure function of
                // `(state, theme, size)` and never of the `Runtime`.
                // The user's motion setting reaches the runtime before anything is
                // advanced with it, and it is read every frame: a settings screen with a
                // *reduce motion* switch changes it while the application is running.
                self.runtime.still = settings.disable_animations;
                // And where the pointer stands in relation to a bar, from the previous
                // frame's registry — the only one there is at this point in the frame.
                //
                // Near a bar, it holds it open and warms the thumb; a bar that has faded
                // out entirely still answers here, and comes back for the reach. On a
                // touch platform nothing draws a bar in the first place, so the registry
                // is empty and a lingering finger position says nothing.
                self.runtime.scrollbar_hovered = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.scrollbar_near(self.cursor))
                    .map(|bar| bar.id);
                self.runtime.scrollbar_dragged = match self.drag {
                    Some(Drag::Scrollbar { id, .. }) => Some(id),
                    _ => None,
                };
                // A switcher in flight carries each child's progress as a number built into
                // the tree, so it is rebuilt — not only repainted — until the switch settles.
                let need_build = frame_needs_build(
                    self.build_dirty,
                    app_animating,
                    std::mem::replace(&mut self.app_was_animating, app_animating),
                    self.tree.is_none() || self.runtime.switching(),
                );
                if need_build {
                    // No scope of its own: the surface above is already installed, and
                    // covers the layout and the paint that follow as well.
                    //
                    // `build_view`, not `view`: `collect_ids` below reads this tree before
                    // the layout pass has been down it, so an unprepared tree would report
                    // no identities at all inside a deferred subtree — and everything in an
                    // `AppBar` would silently never mount, never fade in and never fade out.
                    let tree = build_view(&self.app, &theme, &self.runtime);
                    let ids = collect_ids(tree.as_ref());
                    let present: std::collections::HashSet<_> = ids.iter().copied().collect();

                    // Leaving: snapshot the widgets present at N-1 but absent at N,
                    // so they can be faded out.
                    let leaving: std::collections::HashSet<u64> = self
                        .runtime
                        .mounted
                        .iter()
                        .filter(|id| !present.contains(id))
                        .map(|id| id.as_u64())
                        .collect();
                    if !leaving.is_empty() {
                        if let Some(ui) = &self.ui {
                            let captured: Vec<_> = ui
                                .scene()
                                .primitives()
                                .iter()
                                .filter(|p| leaving.contains(&p.owner()))
                                .cloned()
                                .collect();
                            if !captured.is_empty() {
                                self.runtime
                                    .leaving
                                    .insert(self.leaving_counter, (captured, 1.0));
                                self.leaving_counter = self.leaving_counter.wrapping_add(1);
                            }
                        }
                    }

                    // Mounting: new widgets start out fading in.
                    for &id in &ids {
                        if self.runtime.mounted.insert(id) {
                            self.runtime.anims.entry(id).or_default().opacity = 0.0;
                        }
                    }
                    self.runtime.mounted.retain(|id| present.contains(id));

                    self.tree = Some(tree);

                    // The configuration may have changed, so invalidate the paint
                    // cache and its repaint boundaries. Entries from a stale generation
                    // no longer *hit*, giving a full repaint this frame and reuse on the
                    // interaction-only frames that follow.
                    self.runtime.paint_cache.borrow_mut().bump_generation();
                }
                self.build_dirty = false;

                // Focus requests (`Command::focus`), resolved against the tree just
                // (re)built — the key names the field wrapped in `keyed(k, …)`. The most
                // recent one that resolves wins, and the focus ring becomes visible
                // again, since we jump to the field as the keyboard would.
                if !self.pending_focus.is_empty() {
                    let keys = std::mem::take(&mut self.pending_focus);
                    let ids: Vec<WidgetId> = self
                        .tree
                        .as_deref()
                        .map(|tree| keys.iter().filter_map(|&k| find_by_key(tree, k)).collect())
                        .unwrap_or_default();
                    if let Some(&id) = ids.last() {
                        self.runtime.input.focused = Some(id);
                        self.runtime.focus_visible = true;
                    }
                }

                // === PAINT phase ===
                // The retained tree is (re)painted. Layout goes through the relayout
                // cache (milestone 55: taffy is called again only when the structure
                // changed); painting goes through the repaint cache (milestone 88: a
                // A row carried past the end of the list: the list comes to meet it.
                // Before the tree is borrowed for the rest of the frame, because it moves
                // an offset the build below is about to read — and this frame, not the
                // next one, or the content would lag a frame behind the finger.
                let autoscrolling = self.autoscroll_carried(dt);

                // The reorder spring, per axis, with a time constant of about 70 ms:
                // - **horizontal** (`Table` columns): the smoothed `reorder_x` catches up
                //   with the pointer — the columns slide with inertia while the ghost
                //   sticks to the real pointer;
                // - **vertical** (Kanban cards): the smoothed `reorder_y` catches up with
                //   the **chosen** slot edge, the hovered half — the insertion line and
                //   the gap *slide* between cards instead of jumping, which is the
                //   vertical counterpart of `reorder_x`;
                // - **a horizontal list's rows**: `reorder_x` catches up with the chosen
                //   slot edge, the same line turned on its side.
                let reorder_animating = self.advance_reorder_springs(dt);

                // static `RepaintBoundary` subtree is replayed without repainting while
                // its geometry and the interaction state hold still).
                let tree = self
                    .tree
                    .as_deref()
                    .expect("the view was built at least once");

                // Scroll and pan inertia; the bounds and viewports come from the
                // previous frame.
                let scroll_regions = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.scroll_regions().to_vec())
                    .unwrap_or_default();
                let scroll_physics = self.app.scroll_physics();
                // The refresh areas of the frame, with the `refreshing` flag each was
                // built with: that flag is what tells a spinning indicator when to stop.
                let refresh_areas = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.refresh_areas().to_vec())
                    .unwrap_or_default();
                let dismissables = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.dismissables().to_vec())
                    .unwrap_or_default();
                // Filled by the dismissal step below, dispatched once the tree is no
                // longer borrowed.
                let mut dismissed: Vec<A::Message> = Vec::new();
                let sheet_areas = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.sheets().to_vec())
                    .unwrap_or_default();
                let interactive_bounds = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.interactive_bounds())
                    .unwrap_or_default();

                // A paged view that has been asked for another page glides across to
                // it. Done before the springs are stepped, so the request is honoured
                // in the frame it arrives rather than the one after.
                self.runtime.sync_pages(&scroll_regions);
                // And a bar whose selected item has moved out of its window slides back
                // to it. Same moment, same reason: the request is honoured in the frame
                // it arrives rather than the one after.
                self.runtime.sync_visible(&scroll_regions);
                // And the scroll requests an application has just returned. Same moment
                // and the same reason, with one addition: a region that has only just
                // appeared is not in the registry above, which was built last frame, so
                // what does not resolve here is tried again below against this frame's.
                let requests = std::mem::take(&mut self.pending_scroll);
                let (retry, scrolled) =
                    apply_scroll_requests(&mut self.runtime, tree, requests, &scroll_regions);
                self.retry_scroll = retry;
                // And the sheets it has asked to move, on the same terms: before the sheets
                // are stepped, so an animated request starts in the frame it arrives in.
                let requests = std::mem::take(&mut self.pending_sheet);
                let (retry, sheeted) =
                    apply_sheet_requests(&mut self.runtime, tree, requests, &sheet_areas);
                self.retry_sheet = retry;

                let animating = scrolled
                    | sheeted
                    | self.runtime.advance(dt)
                    | self.runtime.advance_leaving(dt)
                    | self.runtime.advance_switchers(tree, dt)
                    | self.runtime.advance_values(tree, dt)
                    | self.runtime.advance_colors(tree, dt)
                    | self.runtime.advance_sizes(tree, dt)
                    | self.runtime.advance_radii(tree, dt)
                    | self.runtime.advance_paddings(tree, dt)
                    | self.runtime.advance_offsets(tree, dt)
                    | self.runtime.advance_pins(tree, dt)
                    | self.runtime.advance_fractions(tree, dt)
                    | self.runtime.advance_text_styles(tree, dt)
                    | self.runtime.advance_transforms(tree, dt)
                    | self
                        .runtime
                        .advance_scroll(&scroll_regions, scroll_physics, dt)
                    | self.runtime.advance_glow(dt)
                    | self.runtime.advance_refresh(&refresh_areas, dt)
                    | {
                        // A dismissed item announces itself only once its gap has
                        // finished closing. The messages are *collected* here and
                        // dispatched below: the retained tree is borrowed for the whole
                        // of this frame, and `dispatch` rebuilds it.
                        let (moving, done) = self.runtime.advance_dismiss(&dismissables, dt);
                        dismissed.extend(done.into_iter().filter_map(|(id, direction)| {
                            find_widget(tree, id).and_then(|widget| widget.on_dismissed(direction))
                        }));
                        moving
                    }
                    | {
                        // A sheet lowered to nothing says so once it has arrived there,
                        // collected like a dismissal and for the same reason.
                        let (moving, closed) = self.runtime.advance_sheets(&sheet_areas, dt);
                        dismissed.extend(closed.into_iter().filter_map(|id| {
                            find_widget(tree, id).and_then(|widget| widget.on_sheet_dismissed())
                        }));
                        // A throw that carried a sheet to full height goes on into its
                        // list, under the list's own physics (milestone 520).
                        let mut handed = false;
                        for (list, velocity) in self.runtime.take_sheet_handovers() {
                            if let Some(area) = scroll_regions.iter().find(|a| a.id == list) {
                                let physics = area.physics_or(scroll_physics);
                                handed |=
                                    self.runtime.fling_scroll(*area, physics, (0.0, velocity));
                            }
                        }
                        moving | handed
                    }
                    | self.runtime.advance_interactive(&interactive_bounds, dt)
                    | self.runtime.advance_ink(dt)
                    | reorder_animating
                    | autoscrolling
                    | app_animating;
                // With the inspector on, the same build collects the observed nodes,
                // and the overlay — outlines plus a card for the hovered widget — is
                // painted on top of a copy of the scene.
                let (ui, scene) = if self.inspector {
                    let (ui, nodes) = frus_widgets::build_ui_inspected(
                        tree,
                        Size::new(width, height),
                        &self.runtime,
                        &theme,
                    );
                    if std::mem::take(&mut self.inspector_dump) {
                        eprintln!("{}", frus_widgets::dump_tree(&nodes));
                    }
                    let mut scene = ui.scene().clone();
                    frus_widgets::paint_inspector_overlay(
                        &nodes,
                        Some(self.cursor),
                        Size::new(width, height),
                        &theme,
                        &mut scene,
                    );
                    // Scene: logical → physical (DPI × density).
                    (ui, scene.scaled(scale))
                } else {
                    let ui = build_ui(tree, Size::new(width, height), &self.runtime, &theme);
                    // The preview of a column reorder, or a lifted item, on top of the
                    // scene.
                    let scene = if matches!(self.drag, Some(Drag::Reorder { moved: true, .. })) {
                        let mut scene = ui.scene().clone();
                        self.paint_reorder_preview(&ui, &theme, &mut scene);
                        scene.scaled(scale)
                    } else if matches!(self.drag, Some(Drag::Item { moved: true, .. })) {
                        let mut scene = ui.scene().clone();
                        self.paint_drag_ghost(&ui, &theme, &mut scene);
                        scene.scaled(scale)
                    } else {
                        // Scene: logical → physical (DPI × density), for a crisp render.
                        ui.scene().scaled(scale)
                    };
                    (ui, scene)
                };
                self.report_overflows(&ui);
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(&scene) {
                        frus_gpu::RenderOutcome::Presented => {}
                        frus_gpu::RenderOutcome::NeedsReconfigure => {
                            renderer.reconfigure();
                        }
                        frus_gpu::RenderOutcome::Skipped => {}
                    }
                }

                // A continuously animating widget, a spinner say, forces a redraw. So
                // does an image still on its way: without a frame to draw it in, the
                // frame that would show it never happens.
                //
                // The fetch is asked about **here** rather than inside the tree, and as a
                // count rather than a flag on the widget: showing a placeholder means
                // taking the image out of the tree, so a hook read off `Image` would go
                // quiet at exactly the moment it is needed. Asking here also keeps `Ui`
                // answering for its own widgets alone (milestone 411).
                let wants_animation = ui.wants_animation() || frus_widgets::images_in_flight() > 0;

                // Keep the interface, for hit testing. The tree is already retained.
                self.ui = Some(ui);
                // The tree may have changed under a still pointer — a tap that opened a
                // screen — so what it is over is asked of this frame, as the reference
                // does after every frame. Not during a drag: a slider dragged past its
                // end is still the one being pressed, and the move path leaves it so.
                //
                // Field by field rather than through `sync_hover`, which borrows the whole
                // shell: the frame still holds part of it here. A change is shown the way
                // the reference shows it, on the next frame — the tree is built again,
                // since a tooltip's bubble is decided while the tree is walked.
                let rehovered = self.drag.is_none() && {
                    let hovered = self.ui.as_ref().and_then(|ui| self.hover.target(ui));
                    let changed = hovered != self.runtime.input.hovered;
                    self.runtime.input.hovered = hovered;
                    changed
                };
                if rehovered {
                    self.build_dirty = true;
                }
                let wants_animation = wants_animation || rehovered;

                // The paged views that have just turned a page, read off **this**
                // frame's regions: a page change is worth reporting the moment it
                // reads as one, not a frame later.
                let paged = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.scroll_regions().to_vec())
                    .unwrap_or_default();
                let turned: Vec<A::Message> = self
                    .runtime
                    .page_changes(&paged)
                    .into_iter()
                    .filter_map(|(id, page)| {
                        find_widget(tree, id).and_then(|widget| widget.on_page_changed(page))
                    })
                    .collect();

                // The scroll requests the previous frame's registry could not place:
                // tried once against **this** frame's, which is where a region that has
                // only just appeared turns up. This is the last chance they get.
                let (_, moved_late) = apply_scroll_requests(
                    &mut self.runtime,
                    tree,
                    std::mem::take(&mut self.retry_scroll),
                    &paged,
                );
                // The same for the sheet requests: a sheet shown by the very message that
                // asked it to move is in this frame's sheets and not the last one's.
                let (_, sheeted_late) = {
                    let sheets = self
                        .ui
                        .as_ref()
                        .map(|ui| ui.sheets().to_vec())
                        .unwrap_or_default();
                    apply_sheet_requests(
                        &mut self.runtime,
                        tree,
                        std::mem::take(&mut self.retry_sheet),
                        &sheets,
                    )
                };

                // The regions that have moved since they last said so, read off **this**
                // frame's registry so that the offset and the extents reported together
                // come from the same frame. A region nobody is listening to costs the
                // comparison and nothing else.
                let scrolled: Vec<A::Message> = {
                    let grain =
                        |id| find_widget(tree, id).map_or(0.0, |widget| widget.scroll_grain());
                    self.runtime
                        .scroll_changes(&paged, grain)
                        .into_iter()
                        .filter_map(|(id, position)| {
                            find_widget(tree, id).and_then(|widget| widget.on_scroll(position))
                        })
                        .collect()
                };

                // The rows whose gap has just finished closing, the pages that have
                // turned and the regions that have moved: the tree is no longer
                // borrowed, so the application can be told — and rebuild.
                for message in dismissed.into_iter().chain(turned).chain(scrolled) {
                    self.dispatch(message);
                }
                if moved_late || sheeted_late {
                    // Nothing else this frame knows the offset changed: the springs ran
                    // before the request was placed.
                    self.request_redraw();
                }

                // Focus return: when the focused widget has **vanished**, an overlay
                // having closed, go back to the trigger. Done before the AccessKit
                // announcement, so the focus it publishes is current.
                self.reconcile_focus();

                // Publish the frame's semantic tree to AccessKit.
                #[cfg(desktop)]
                if let Some(a11y) = self.a11y.as_mut() {
                    let focus = self.runtime.input.focused;
                    let title = self.app.title();
                    if let Some(ui) = self.ui.as_ref() {
                        a11y.update(ui.semantics(), focus, &title, &self.announce);
                    }
                }

                // The system bars follow the frame: the regions under them, then the theme.
                self.sync_system_bars(&theme);

                // The Android software keyboard follows the text fields' focus.
                self.sync_soft_input();
                // And the autofill service follows the form being edited.
                #[cfg(android)]
                self.sync_autofill();

                // While an animation is running, ask for another frame.
                if animating || wants_animation {
                    self.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl<A: Application> App<A> {
    /// The total scale: system DPI × app density (physical = logical × this).
    fn total_scale(&self) -> f32 {
        (self.scale * self.app.density()).max(0.1)
    }

    /// The surface description installed around every call to `view`, so that any
    /// widget built there can read it with `MediaQuery::of()` instead of having the
    /// application carry it down by hand.
    ///
    /// Everything in it is already known to the shell — the logical size it is about
    /// to lay out for, the DPI scale, the app's density, and the insets last reported
    /// by the platform. It is assembled here, in one place, rather than at each of the
    /// call sites.
    /// The **platform** answers, and the application overrides what it chose to speak for.
    ///
    /// That order is the one the reference uses and the only one that stays honest: the
    /// settings belong to the person using the device, so the framework asks them first,
    /// and an application with a *reduce motion* switch of its own says only that. An
    /// [`AccessibilityOverrides`] whose fields are all `None` — the default — leaves every
    /// answer to the user, which is what an application that has no settings screen means.
    fn media_query(&self, width: f32, height: f32) -> MediaQuery {
        MediaQuery::new(Size::new(width, height))
            .with_device_pixel_ratio(self.scale)
            .with_density(self.app.density())
            .with_text_scaler(self.platform.text_scaler)
            .with_platform_brightness(self.platform.brightness)
            .with_accessibility(
                self.platform
                    .accessibility
                    .with_overrides(self.app.accessibility()),
            )
            .with_insets(self.last_insets)
    }

    /// The current layout direction; RTL flips both the layout and the gestures.
    fn is_rtl(&self) -> bool {
        self.themes.displayed(&self.app).direction.is_rtl()
    }

    /// The window's **logical** width, in px, for the edge thresholds.
    fn logical_width(&self) -> f32 {
        let scale = self.total_scale();
        self.window
            .as_ref()
            .map(|w| w.inner_size().width as f32 / scale)
            .unwrap_or(1.0)
            .max(1.0)
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// The **normalised** pointer input (gesture tier 0): mouse and touch converge
    /// here, with an explicit `Cancel`. The long-press recogniser is fed along the way,
    /// and the loop is woken at its deadline.
    fn pointer(&mut self, event_loop: &ActiveEventLoop, event: PointerEvent) {
        self.pointer_event(event);
        // Wake the loop exactly at the next deadline, and rest otherwise.
        event_loop.set_control_flow(self.idle_control_flow());
    }

    /// Everything [`pointer`](Self::pointer) does with an event but wake the loop: the one
    /// place a press, a movement and a release are routed, and so what a test drives.
    fn pointer_event(&mut self, event: PointerEvent) {
        self.cursor = event.position;
        self.hover.event(event.kind, event.position, event.touch);
        match event.kind {
            PointerKind::Down => {
                // What is under the pointer **now**: a finger has no move before its
                // press, and a press only shows while it is also the hovered widget.
                self.sync_hover();
                // A pointer interaction: the keyboard focus ring fades away.
                self.runtime.focus_visible = false;
                self.pointer_down(event.touch);
                let (long_press, lift) = hold_candidates(
                    self.drag.as_ref(),
                    self.ui.as_ref(),
                    self.tree.as_deref(),
                    self.cursor,
                );
                self.long_press_msg = long_press;
                self.pending_lift = lift;
                let interested = self.long_press_msg.is_some()
                    || self.pending_lift.is_some()
                    || self.pending_reorder.is_some()
                    || self.pending_word.is_some();
                self.press.down(self.cursor, Instant::now(), interested);
            }
            PointerKind::Move => {
                self.press.moved(self.cursor);
                self.pointer_move();
                // The inspector follows the pointer to highlight, so it redraws even
                // over inert widgets, which have no hover animation.
                if self.inspector {
                    self.request_redraw();
                }
            }
            PointerKind::Up => {
                // The recogniser is always told, but a lifted item owes a drop: the
                // long press that started it must not also eat its ending.
                let swallow = self.press.up();
                self.pending_lift = None;
                self.pending_reorder = None;
                self.pending_word = None;
                // A row lifted by the hold owes its drop for the same reason a lifted
                // item does: the release is what says where it goes.
                if swallow
                    && !matches!(
                        self.drag,
                        Some(Drag::Item { moved: true, .. } | Drag::Reorder { moved: true, .. })
                    )
                {
                    // The long press evicted the tap, so the release is swallowed.
                    self.drag = None;
                    self.runtime.input.pressed = None;
                    self.request_redraw();
                } else {
                    self.pointer_up();
                }
            }
            PointerKind::Cancel => {
                self.press.cancel();
                self.pending_lift = None;
                self.pending_reorder = None;
                self.pending_word = None;
                // A cancelled gesture still owes the offset back, or the region
                // would stay frozen under a finger that is no longer there.
                if let Some(Drag::Dismiss { item, .. }) = self.drag {
                    // A cancelled swipe asks for nothing: zero velocity puts it back.
                    self.runtime.dismiss_release(
                        item.id,
                        0.0,
                        0.0,
                        item.spec.axis,
                        // Unreachable threshold: a cancel never dismisses, however far
                        // the item had already travelled.
                        f32::INFINITY,
                    );
                }
                if let Some(Drag::Scroll { id, .. }) = self.drag {
                    self.runtime.release_scroll(id);
                    self.runtime.glow_scroll_end(id);
                    // A cancelled gesture asks for nothing: the indicator slides back
                    // rather than promising a refresh nobody requested.
                    if let Some(host) = self.refresh_host_of(id) {
                        self.runtime.refresh_cancel(host);
                    }
                }
                // A sheet under a cancelled finger settles from where it was left.
                let sheet = match self.drag {
                    Some(Drag::Sheet { id, available, .. }) => self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.sheet(id))
                        .map(|sheet| (sheet.id, sheet.spec.clone(), available)),
                    Some(Drag::Scroll { id, .. }) => self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.sheet_holding(id))
                        .map(|sheet| (sheet.id, sheet.spec.clone(), sheet.available)),
                    _ => None,
                };
                if let Some((id, spec, available)) = sheet {
                    self.runtime.sheet_release(id, &spec, available, 0.0, None);
                }
                self.drag = None;
                self.runtime.input.pressed = None;
                self.request_redraw();
            }
        }
        // After the release has been routed — it reads the press, not the hover — a
        // finger that has lifted stops hovering (milestone 505).
        if matches!(event.kind, PointerKind::Up | PointerKind::Cancel) {
            self.sync_hover();
        }
    }

    /// A press held still for the long-press delay: what it was a candidate for is decided.
    fn hold_deadline_reached(&mut self) {
        // Two claims on one hold: a widget asking for a message, and an item asking
        // to be lifted. Serving both would do a discrete action *and* start a drag
        // from the same gesture, which is never what anyone meant. **The lift
        // wins** — it changes what the rest of the gesture means, and the message
        // would be acting on something the finger is still holding.
        let lifting = self.pending_lift.is_some() || self.pending_reorder.is_some();
        // A hold in a text field selects the word under the finger (milestone 511),
        // and is that — not also the long press of whatever the field sits in.
        let word = self.pending_word.take();
        if let Some(message) = self.long_press_msg.take() {
            if !lifting && word.is_none() {
                self.dispatch(message);
            }
        }
        if let Some(id) = word {
            self.select_held_word(id);
        }
        let lift = self.pending_lift.take();
        let reorder = self.pending_reorder.take();
        // The scroll hands the gesture over to what the hold lifted.
        if lift.is_some() || reorder.is_some() {
            if let Some(Drag::Scroll { id, .. }) = self.drag {
                self.runtime.release_scroll(id);
            }
        }
        self.drag = drag_after_hold(self.drag.take(), lift, reorder, self.cursor);
        if matches!(self.drag, Some(Drag::Reorder { .. })) {
            self.reorder_x = self.cursor.x;
            self.reorder_y = self.cursor.y;
        }
        self.request_redraw();
    }

    /// The loop's idle policy: wake at the **nearest** deadline — the long press, the
    /// live-reload poll, the caret's next turn — and otherwise wait outright.
    fn idle_control_flow(&self) -> ControlFlow {
        let press = self.press.deadline();
        let reload = self.reload.as_ref().map(|w| w.deadline());
        // Twice a second while a field has the caret and the window is in front, and never
        // otherwise (milestone 513).
        let caret = (self.lifecycle == Lifecycle::Resumed)
            .then(|| self.caret.next_toggle(Instant::now()))
            .flatten();
        match [press, reload, caret].into_iter().flatten().min() {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        }
    }

    /// The focused field's caret, as its blink sees it: which field, where its caret and
    /// selection are, and a digest of its value. `None` when what has focus takes no
    /// typing, which has no caret to blink.
    fn caret_signature(&self) -> Option<crate::caret::Signature> {
        use std::hash::{Hash, Hasher};
        let field = self.runtime.input.focused?;
        let widget = find_widget(self.tree.as_ref()?.as_ref(), field)?;
        let text = widget.text_value()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        Some(crate::caret::Signature {
            field,
            edit: self.runtime.edits.get(&field).copied().unwrap_or_default(),
            value: hasher.finish(),
        })
    }

    /// Pointer movement, mouse or finger: continues a drag under way, and otherwise
    /// updates the hover.
    fn pointer_move(&mut self) {
        if self.drag.is_some() {
            self.handle_drag();
            return;
        }
        let hovered = self.sync_hover();
        // The system cursor follows the hovered sub-region (milestone 205): a hand
        // over a clickable icon, and so on. Recomputed on every move, since the
        // sub-region can change without the hovered widget changing.
        self.update_cursor_icon(hovered);
    }

    /// Asks the frame what is under the hovering pointer, and records it.
    ///
    /// The only way the hover changes, so it can never name a widget of another frame:
    /// an id is a position in the tree, and the same position on the next screen is a
    /// different widget (milestone 505).
    fn sync_hover(&mut self) -> Option<WidgetId> {
        let hovered = self.ui.as_ref().and_then(|ui| self.hover.target(ui));
        if hovered != self.runtime.input.hovered {
            self.runtime.input.hovered = hovered;
            self.request_redraw();
        }
        hovered
    }

    /// Applies the cursor shape the hovered widget asks for at the pointer's local
    /// position (milestone 205), and the default cursor otherwise. Translates
    /// `frus_widgets::Cursor` into winit's.
    fn update_cursor_icon(&mut self, hovered: Option<WidgetId>) {
        let requested = hovered.and_then(|id| {
            let rect = self.ui.as_ref()?.widget_rect(id)?;
            let widget = find_widget(self.tree.as_ref()?.as_ref(), id)?;
            widget.cursor_icon(
                self.cursor.x - rect.x,
                self.cursor.y - rect.y,
                rect.width,
                rect.height,
            )
        });
        let icon = match requested.unwrap_or(UiCursor::Default) {
            UiCursor::Default => CursorIcon::Default,
            UiCursor::Pointer => CursorIcon::Pointer,
            UiCursor::Text => CursorIcon::Text,
        };
        if let Some(window) = &self.window {
            window.set_cursor(icon);
        }
        // Sub-region highlighting (milestone 208): the pointer's position is retained
        // while it hovers an interactive sub-region, that is, while cursor_icon answered.
        // A change repaints — the status hash includes hover_cursor — so the halo
        // follows the pointer or goes away.
        let hover_cursor = requested.map(|_| self.cursor);
        if hover_cursor != self.runtime.input.hover_cursor {
            self.runtime.input.hover_cursor = hover_cursor;
            self.request_redraw();
        }
    }

    /// A pointer press, mouse or finger, at `self.cursor`. `touch` enables finger
    /// scrolling when no other gesture captures the press.
    fn pointer_down(&mut self, touch: bool) {
        self.pointer_touch = touch;
        self.pending_word = None;
        // A selection handle first. It hangs below its line, over whatever is drawn
        // there, so nothing else may claim the press before it; and any other press puts
        // the handles away (milestone 511).
        if let Some(drag) = self.grab_selection_handle() {
            self.drag = Some(drag);
            self.request_redraw();
            return;
        }
        if self.runtime.selection_handles.take().is_some() {
            self.request_redraw();
        }
        // 0) The back gesture: a press on the **leading edge** — left under LTR,
        // right under RTL — if the app allows it.
        let on_back_edge = if self.is_rtl() {
            self.cursor.x > self.logical_width() - BACK_EDGE
        } else {
            self.cursor.x < BACK_EDGE
        };
        if on_back_edge && self.app.can_go_back() {
            self.drag = Some(Drag::Back {
                start_x: self.cursor.x,
            });
            self.begin_gesture();
            self.build_dirty = true;
            self.app.back_gesture(0.0);
            self.request_redraw();
            return;
        }

        // 1) Is this a scrollbar drag?
        if let Some(bar) = self.ui.as_ref().and_then(|ui| ui.scrollbar_at(self.cursor)) {
            let (along, thumb_start) = if bar.vertical {
                (self.cursor.y, bar.thumb.y)
            } else {
                (self.cursor.x, bar.thumb.x)
            };
            self.drag = Some(Drag::Scrollbar {
                id: bar.id,
                vertical: bar.vertical,
                grab: along - thumb_start,
                track_start: bar.track_start,
                track_len: bar.track_len,
                thumb_len: bar.thumb_len,
                max: bar.max,
                reverse: bar.reverse,
            });
            self.request_redraw();
            return;
        }

        // 1b) Is this a draggable widget, a Slider for instance?
        if let Some((id, rect)) = self.ui.as_ref().and_then(|ui| ui.draggable_at(self.cursor)) {
            self.drag = Some(Drag::Widget {
                id,
                rect,
                last_x: self.cursor.x,
            });
            // The bracket opens **before** the first value: an application that means
            // to hold the expensive work until the finger lifts has to know the finger
            // went down first, and a press that jumps the value straight away would
            // otherwise deliver the change before the start.
            self.dispatch_drag_edge(id, rect, Edge::Start);
            // A zero delta on press: only a slider, which takes a fraction, jumps.
            self.apply_widget_drag(id, rect, 0.0);
            self.request_redraw();
            return;
        }

        // 1c) A reorder: a press on a reorderable — a table header, a Kanban card, a
        // list's grip. We do not `return` — focus and `pressed`, where a tap means sort,
        // are settled below; the drag engages only past the threshold, and otherwise the
        // release sorts.
        //
        // A row that asked for a **hold** takes nothing now. The press is left to
        // whatever else wants it, which inside a list is the scroll, and the deadline
        // below decides between them: a finger that stays put was never scrolling.
        if let Some((id, from, hold)) = self.reorderable_at(self.cursor) {
            if hold {
                self.pending_reorder = Some((id, from, self.cursor));
            } else {
                self.drag = Some(Drag::Reorder {
                    id,
                    from,
                    start: self.cursor,
                    moved: false,
                    carried: None,
                });
                self.reorder_x = self.cursor.x; // starts glued to the pointer, with no jerk
                self.reorder_y = self.cursor.y; // likewise for the vertical insertion line
            }
        }

        self.runtime.input.pressed = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        // The ink: a surface that takes it splashes from where the finger landed. The
        // box comes from the frame that was on screen when the finger came down, which
        // is the one the user aimed at.
        if let Some(pressed) = self.runtime.input.pressed {
            if let Some(rect) = self.ui.as_ref().and_then(|ui| ui.ink_box(pressed)) {
                self.runtime.ink_press(
                    pressed,
                    Point::new(self.cursor.x - rect.x, self.cursor.y - rect.y),
                    Size::new(rect.width, rect.height),
                );
                self.request_redraw();
            }
        }
        // 2) Focus and caret placement, and the start of a text selection.
        let previously_focused = self.runtime.input.focused;
        let focus = self.ui.as_ref().and_then(|ui| ui.focus_hit(self.cursor));
        self.runtime.input.focused = focus.map(|(id, _)| id);
        if let Some((id, rect)) = focus {
            let local_x = self.cursor.x - rect.x;
            // The retained vertical scroll, in a multi-line field, is part of the
            // content coordinate: we add it so the click lands on the right line.
            let local_y =
                self.cursor.y - rect.y + self.runtime.scroll.get(&id).map(|s| s.1).unwrap_or(0.0);
            // The scroll shown just before this click, computed from the current caret
            // when the field was already focused, and 0 otherwise.
            let scroll_cursor = if previously_focused == Some(id) {
                self.runtime.edits.get(&id).map(|e| e.cursor).unwrap_or(0)
            } else {
                0
            };
            // Only **text fields** (`cursor_at` → `Some`) start a selection; the other
            // focusables — buttons, checkboxes — keep focus but must NOT capture the
            // click, which would otherwise be swallowed on release as the end of a drag.
            let cursor = self
                .tree
                .as_ref()
                .and_then(|tree| find_widget(tree.as_ref(), id))
                .and_then(|widget| widget.cursor_at(local_x, local_y, rect.width, scroll_cursor));
            if let Some(cursor) = cursor {
                // Placing the caret with the mouse forgets the vertical goal column.
                self.goal_x = None;
                self.runtime.edits.insert(
                    id,
                    Edit {
                        cursor,
                        anchor: None,
                        composing: None,
                    },
                );
                self.drag = Some(Drag::TextSelect { id, rect });
                // A finger that stays put selects the word instead (milestone 511).
                if touch {
                    self.pending_word = Some(id);
                }
                // The caret moved, so the run of typing ends here: typing at one place,
                // then at another, then Ctrl+Z should take back only the second.
                self.runtime.close_edit_run(id);
                // Tapping in a field **reopens** the keyboard, even one the app already
                // considers shown but which the system back closed — see
                // `request_soft_input`.
                self.request_soft_input();

                // A double click selects the word under the pointer.
                let now = Instant::now();
                let double = self
                    .last_click_time
                    .map(|t| (now - t).as_secs_f32() < 0.4)
                    .unwrap_or(false);
                self.last_click_time = Some(now);
                if double {
                    if let Some((start, end)) = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                        .and_then(|widget| widget.word_at(cursor))
                    {
                        self.runtime.edits.insert(
                            id,
                            Edit {
                                cursor: end,
                                anchor: Some(start),
                                composing: None,
                            },
                        );
                        self.drag = None;
                    }
                }
            }
        }

        // 3) Touch: when nothing captured the gesture — no scrollbar, no widget, no
        // text selection — prepare a finger scroll on the area under the finger. A
        // release without movement, under TOUCH_SLOP, stays a tap.
        if touch && self.drag.is_none() {
            // The innermost area that would actually take a finger. An area whose content
            // fits refuses the offset outright — the reference takes its drag recognisers
            // away for exactly this — so it neither swallows the press nor lights an edge,
            // and the page behind it, or a dismissible row under it, gets its turn.
            let landed = {
                let scroll = &self.runtime.scroll;
                self.ui.as_ref().and_then(|ui| {
                    ui.scroll_chain(self.cursor).find(|area| {
                        area.accepts_user_offset(
                            scroll.get(&area.id).copied().unwrap_or((0.0, 0.0)),
                        )
                    })
                })
            };
            if let Some(area) = landed {
                // A finger back on the content catches it: the fling stops where it
                // is rather than sliding under the finger, and hands the next
                // release whatever momentum the platform lets a swipe build on.
                let physics = area.physics_or(self.app.scroll_physics());
                let carried = self.runtime.catch_scroll_fling(area.id, physics);
                // The offset belongs to the finger until it lifts.
                self.runtime.hold_scroll(area.id);
                // And so does the sheet it sits in, if one is still settling: the finger
                // may be about to move it, and a sheet sliding away under a finger that
                // has caught its list is a sheet that ignored the catch.
                if let Some(sheet) = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.sheet_holding(area.id))
                    .map(|sheet| sheet.id)
                {
                    self.runtime.sheet_hold(sheet);
                }
                self.drag = Some(Drag::Scroll {
                    id: area.id,
                    last: self.cursor,
                    moved: false,
                    carried,
                    dismiss: self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.dismissable_at(self.cursor)),
                    axis: None,
                });
                self.begin_gesture();
            }
        }

        // 3a) A sheet's panel with nothing under the finger that scrolls: the finger moves
        // the sheet itself. A pointer too — on a desktop there is no other way to raise one.
        if self.drag.is_none() {
            if let Some((id, available)) = self
                .ui
                .as_ref()
                .and_then(|ui| ui.sheet_at(self.cursor))
                .map(|sheet| (sheet.id, sheet.available))
            {
                self.runtime.sheet_hold(id);
                self.drag = Some(Drag::Sheet {
                    id,
                    last: self.cursor,
                    moved: false,
                    available,
                });
                self.begin_gesture();
            }
        }

        // 3b) A dismissible item with nothing scrollable under it: there is no gesture to
        // arbitrate against, so the swipe is prepared directly. It still waits for the
        // threshold, or a tap on the row would start sliding it.
        if self.drag.is_none() {
            if let Some(item) = self
                .ui
                .as_ref()
                .and_then(|ui| ui.dismissable_at(self.cursor))
            {
                self.drag = Some(Drag::Dismiss {
                    item,
                    last: self.cursor,
                    moved: false,
                });
                self.begin_gesture();
            }
        }

        // 3c) A draggable item, when nothing above has taken the gesture. It comes
        // **after** the touch scroll on purpose: a widget that took every drag inside a
        // list would silently stop that list scrolling, and a list that does not scroll
        // is a worse bug than an item that does not lift. With a pointer there is no
        // touch scroll to lose to, so it lifts everywhere.
        if self.drag.is_none() {
            if let Some(source) = self
                .ui
                .as_ref()
                .and_then(|ui| ui.drag_source_at(self.cursor))
                .filter(|source| {
                    // One that asked for a hold waits for the deadline instead.
                    !self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), source.id))
                        .is_some_and(|widget| widget.drag_needs_long_press())
                })
            {
                self.drag = Some(Drag::Item {
                    source,
                    start: self.cursor,
                    moved: false,
                    over: None,
                });
                self.begin_gesture();
            }
        }

        // 4) An interactive viewport (`InteractiveViewer`) under the pointer: prepare a
        // **pan**, with mouse or finger. Like the touch scroll it engages only past the
        // threshold, so a tap goes through to the child, a button for instance.
        if self.drag.is_none() {
            if let Some((id, viewport)) = self
                .ui
                .as_ref()
                .and_then(|ui| ui.interactive_at(self.cursor))
            {
                // The press stops a fling in progress: we take the content back in hand.
                self.runtime.interactive_velocity.remove(&id);
                self.drag = Some(Drag::Pan {
                    id,
                    last: self.cursor,
                    moved: false,
                    viewport,
                });
                self.begin_gesture();
            }
        }
        self.request_redraw();
    }

    /// A pointer release, mouse or finger: it ends a drag, or commits a click or tap
    /// when the release lands back on the widget that was pressed.
    fn pointer_up(&mut self) {
        let ended = self.drag.take();
        if let Some(Drag::Back { .. }) = ended {
            // The app decides — commit or cancel — from the velocity, which it wants
            // in **fractions of the screen** per second, not pixels.
            let sign = if self.is_rtl() { -1.0 } else { 1.0 };
            let velocity = sign * self.gesture_estimate().velocity.x / self.logical_width();
            self.build_dirty = true;
            self.app.back_gesture_end(velocity);
            self.request_redraw();
            return;
        }
        // A touch scroll, a pan or a swipe that never moved is a plain tap: we let it
        // follow the ordinary click path below.
        let was_tap = gesture_was_a_tap(ended.as_ref());
        // The bracket closes. A press that never moved still gets its end: it changed
        // the value once, and a caller that defers its expensive work until the release
        // would otherwise never be told the release happened.
        if let Some(Drag::Widget { id, rect, .. }) = ended {
            self.dispatch_drag_edge(id, rect, Edge::End);
        }
        // The keyboard is told where a dragged handle left the selection.
        #[cfg(android)]
        if let Some(Drag::SelectionHandle { id, .. }) = ended {
            self.push_ime_context(id);
        }
        // Reordering: on the drop, the target column is the reorderable header under
        // the pointer, and we route the grabbed header's `on_reorder(from, to)`.
        if let Some(Drag::Reorder {
            id,
            from,
            moved: true,
            carried,
            ..
        }) = &ended
        {
            let target = self.reorder_drop_target(*id, *from, *carried);
            let tree = self.tree.as_ref();
            let base = target
                .and_then(|tid| tree.and_then(|t| find_widget(t.as_ref(), tid)))
                .and_then(|widget| widget.reorder_index());
            // The **trailing** half of a hovered target that is inserted at — the lower
            // half down a list, the right half across one (the left, right to left) —
            // means inserting **after** it, index +1: the effective drop slot follows the
            // insertion line that was painted. No effect for `Table` columns, or off target.
            let to = match (base, target) {
                (Some(base), Some(tid)) => {
                    let after = self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.widget_rect(tid))
                        .map(|rect| self.reorder_insert_after(tid, rect))
                        .unwrap_or(false);
                    Some(base + after as usize)
                }
                _ => None,
            };
            // Dropping **onto itself** moves nothing, even in the lower half, where
            // `to = from + 1` would slip past the `to != from` guard — otherwise we would
            // emit a null move and announce it.
            let self_drop = target == Some(*id);
            let message = match to {
                Some(to) if to != *from && !self_drop => tree
                    .and_then(|t| find_widget(t.as_ref(), *id))
                    .and_then(|widget| widget.on_reorder(to)),
                _ => None,
            };
            if let Some(message) = message {
                let to = to.unwrap_or(*from);
                let widget = tree.and_then(|t| find_widget(t.as_ref(), *id));
                let axis = widget
                    .map(|w| w.reorder_axis())
                    .unwrap_or(ReorderAxis::Horizontal);
                // The move is spoken to the screen reader — the ghost's counterpart for
                // a blind user. A widget whose index **is** a position says so itself; a
                // `Kanban` card's is a flat `column × stride + position` that means
                // nothing read aloud, so the fallback speaks of the axis and gives no
                // number at all.
                let announcement = widget
                    .and_then(|w| w.reorder_announcement(to))
                    .unwrap_or_else(|| match axis {
                        ReorderAxis::Horizontal => format!("Column moved to position {}", to + 1),
                        ReorderAxis::Vertical => "Card moved".to_string(),
                    });
                self.dispatch(message);
                self.set_announcement(announcement);
            }
        }
        // Fling: the finger's momentum is handed to the area's physics, which
        // returns the motion that follows — a spline that stops at the edge, or
        // friction that hands over to a spring and bounces.
        // A swiped item: the release velocity decides between flying out and sliding
        // back, using the same fitted estimate a fling uses.
        if let Some(Drag::Dismiss { item, moved, .. }) = &ended {
            if *moved {
                let estimate = self.gesture_estimate();
                let horizontal = item.spec.axis.is_horizontal();
                let (along, across) = if horizontal {
                    (estimate.velocity.x, estimate.velocity.y)
                } else {
                    (estimate.velocity.y, estimate.velocity.x)
                };
                self.runtime.dismiss_release(
                    item.id,
                    along,
                    across,
                    item.spec.axis,
                    item.spec.threshold,
                );
                self.request_redraw();
                return;
            }
        }
        // A sheet let go of by its panel settles from wherever the finger left it — and
        // one only pressed settles too, since the press caught it if it was moving.
        if let Some(Drag::Sheet {
            id,
            moved,
            available,
            ..
        }) = &ended
        {
            let velocity = if *moved {
                -self.fling_velocity(self.gesture_estimate()).1
            } else {
                0.0
            };
            // Thrown by its handle, a sheet with one list in it carries that list on at the
            // top, as the reference's does: there its whole content is the list.
            let sheet = self.ui.as_ref().and_then(|ui| ui.sheet(*id)).map(|sheet| {
                let list = match sheet.areas.as_slice() {
                    [only] => Some(*only),
                    _ => None,
                };
                (sheet.spec.clone(), list)
            });
            if let Some((spec, list)) = sheet {
                self.runtime
                    .sheet_release(*id, &spec, *available, velocity, list);
            }
            if *moved {
                self.request_redraw();
                return;
            }
        }
        if let Some(Drag::Item {
            source,
            moved: true,
            over,
            ..
        }) = &ended
        {
            // The drop, then the source's own answer. In that order: an application
            // that reacts to both should see the thing arrive before it is told the
            // journey is over.
            self.runtime.drag_over = None;
            let accepted = over.is_some();
            let dropped = over.and_then(|id| {
                self.tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), id))
                    .and_then(|widget| widget.on_drop(source.payload))
            });
            let ended_msg = self
                .tree
                .as_ref()
                .and_then(|tree| find_widget(tree.as_ref(), source.id))
                .and_then(|widget| widget.on_dropped(accepted));
            for message in dropped.into_iter().chain(ended_msg) {
                self.dispatch(message);
            }
            self.request_redraw();
            return;
        }
        if let Some(Drag::Item { .. }) = &ended {
            // Lifted but never moved: nothing was dropped, and the click below still
            // has its say.
            self.runtime.drag_over = None;
        }
        // Set when a sheet takes the release of a list inside it, which then does not
        // fling as well (milestone 515).
        let mut sheet_took = false;
        if let Some(Drag::Scroll { id, .. }) = &ended {
            let sheet = self
                .ui
                .as_ref()
                .and_then(|ui| ui.sheet_holding(*id))
                .cloned();
            if let Some(sheet) = sheet {
                // In the sheet's terms, positive growing it: a list's offset grows the
                // same way, on an axis that is not reversed.
                let velocity = match &ended {
                    Some(Drag::Scroll {
                        moved: true,
                        axis: Some(true),
                        carried,
                        ..
                    }) => -self.fling_velocity(self.gesture_estimate()).1 + carried.1,
                    _ => 0.0,
                };
                let offset = self.runtime.scroll.get(id).map_or(0.0, |o| o.1);
                let size = self.runtime.sheet_size(sheet.id, &sheet.spec);
                sheet_took = velocity != 0.0
                    && frus_widgets::sheet_takes_release(
                        velocity,
                        offset,
                        size,
                        &sheet.spec,
                        sheet.available,
                    );
                // Whoever takes it, the sheet is let go of: settled by the throw, or
                // from where it is — which is a stop, whenever the list keeps the throw.
                self.runtime.sheet_release(
                    sheet.id,
                    &sheet.spec,
                    sheet.available,
                    if sheet_took { velocity } else { 0.0 },
                    Some(*id),
                );
            }
        }
        if let Some(Drag::Scroll { id, .. }) = &ended {
            // The finger gives the offset back before anything is flung at it.
            self.runtime.release_scroll(*id);
            // A pull held against an edge now fades at the "let go" rate.
            self.runtime.glow_scroll_end(*id);
            // And a pull-to-refresh gesture is answered: releasing it armed is the
            // whole point of the gesture, so the message goes out now rather than
            // after the indicator has finished settling.
            self.release_refresh(*id);
        }
        if let Some(Drag::Scroll {
            id,
            moved: true,
            carried,
            axis,
            ..
        }) = &ended
        {
            // In **scroll space**: the content moves opposite the finger — and along the
            // axis the gesture was claimed by, or the release would fling the way the drag
            // was not allowed to go.
            let estimate = self.gesture_estimate();
            let velocity = self.fling_velocity(estimate);
            // Through the same conversion the drag went through, so the release carries
            // on the way the finger was going — on a reversed axis too.
            let velocity = match self.ui.as_ref().and_then(|u| u.scroll_region(*id)) {
                Some(area) => area.offset_delta(velocity),
                None => (-velocity.0, -velocity.1),
            };
            // The momentum carried in from an interrupted fling is masked with the rest:
            // a gesture held to one axis must not launch along the other on the strength
            // of what the *previous* one was doing.
            let launch = (velocity.0 + carried.0, velocity.1 + carried.1);
            if sheet_took {
                self.request_redraw();
            } else {
                self.fling(
                    *id,
                    match axis {
                        Some(true) => (0.0, launch.1),
                        Some(false) => (launch.0, 0.0),
                        None => launch,
                    },
                );
            }
        }
        // A pan fling: the momentum launches the content, which `advance_interactive`
        // decelerates and bounds frame by frame.
        if let Some(Drag::Pan {
            id, moved: true, ..
        }) = &ended
        {
            let estimate = self.gesture_estimate();
            let velocity = self.fling_velocity(estimate);
            if velocity.0.hypot(velocity.1) > PAN_FLING_MIN {
                self.runtime.interactive_velocity.insert(*id, velocity);
            }
        }
        if ended.is_some() && !was_tap {
            self.request_redraw();
            return;
        }
        // A click only counts when press and release land on the same widget.
        let released = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        let (message, announce) = match (self.runtime.input.pressed, released) {
            (Some(pressed), Some(released)) if pressed == released => {
                // A **positional** click, on a sub-region such as a field's clickable
                // suffix, takes priority over `on_click`. Local coordinates are the
                // pointer minus the widget's corner.
                let positional = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.widget_rect(released))
                    .and_then(|rect| {
                        self.tree
                            .as_ref()
                            .and_then(|tree| find_widget(tree.as_ref(), released))
                            .and_then(|widget| {
                                widget.positional_click(
                                    self.cursor.x - rect.x,
                                    self.cursor.y - rect.y,
                                    rect.width,
                                    rect.height,
                                )
                            })
                    });
                let message =
                    positional.or_else(|| self.ui.as_ref().and_then(|ui| ui.msg_for(pressed)));
                // The spoken announcement of the effect — a sort, a selection — read off
                // the clicked widget before `dispatch` rebuilds the tree.
                let announce = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), pressed))
                    .and_then(|widget| widget.announce());
                (message, announce)
            }
            _ => (None, None),
        };
        // The tap completed on the widget it started on: its ink finishes growing and
        // fades. Anything else leaves the splash unconfirmed, and `advance_ink` sweeps
        // it away quickly — the finger slid off, and the ink says so.
        if let (Some(pressed), Some(released)) = (self.runtime.input.pressed, released) {
            if pressed == released {
                self.runtime.ink_confirm(pressed);
            }
        }
        self.runtime.input.pressed = None;
        if let Some(message) = message {
            self.dispatch(message);
            if let Some(announce) = announce {
                self.set_announcement(announce);
            }
        }
        self.request_redraw();
    }

    /// Speaks a message aloud through the screen reader's **live region**, for a
    /// column reorder and the like. With no screen reader running it costs nothing. The
    /// text is re-spoken only on a change. Desktop only; a no-op elsewhere.
    #[cfg(desktop)]
    fn set_announcement(&mut self, message: String) {
        self.announce = message;
    }
    #[cfg(not(desktop))]
    fn set_announcement(&mut self, _message: String) {}

    /// Returning focus when an overlay closes: if the focused widget has **vanished**
    /// from the frame, a menu or modal having closed, focus goes back to the
    /// **trigger** — the most recent focusable in the history that is still present.
    /// Every transition is recorded along the way: the old focus, if still present,
    /// becomes a candidate trigger. The focus ring is drawn on the following frame
    /// (`request_redraw` when focus moved).
    fn reconcile_focus(&mut self) {
        let present: std::collections::HashSet<WidgetId> = match self.ui.as_ref() {
            Some(ui) => ui.focusable_ids().collect(),
            None => return,
        };
        let before = self.runtime.input.focused;
        let after = resolve_focus(
            before,
            &present,
            &mut self.focus_history,
            &mut self.prev_focus,
        );
        if after != before {
            self.runtime.input.focused = after;
            self.request_redraw();
        }
    }

    /// Applies a message to the application, runs its effects, then re-evaluates the
    /// subscriptions, the state having possibly changed which ones should run.
    fn dispatch(&mut self, message: A::Message) {
        // The app may have changed state, so the `view` must be rebuilt.
        self.build_dirty = true;
        let command = self.app.update(message);
        self.run_command(command);
        self.sync_subscriptions();
    }

    /// Runs a command, and whatever message it produces comes back into the loop
    /// through the proxy. Focus requests are **queued**, then resolved against the
    /// freshly built tree on the next frame.
    ///
    /// Where each kind of effect goes:
    ///
    /// | | native | Web |
    /// |---|---|---|
    /// | synchronous task | its own thread | a `spawn_local` microtask |
    /// | asynchronous task | the shell's executor | the browser |
    /// | timer | a task on the executor's reactor | `setTimeout` |
    ///
    /// A **synchronous** task keeps its thread: it blocks by definition, and that is
    /// what a thread is for. An **asynchronous** one no longer gets one — it is a
    /// task among others on a pool of four, which is the whole point of milestone
    /// 303. See [`crate::runtime`].
    fn run_command(&mut self, command: crate::command::Command<A::Message>) {
        let parts = command.into_parts();
        self.pending_focus.extend(parts.focus);
        self.pending_scroll.extend(parts.scrolls);
        self.pending_sheet.extend(parts.sheets);
        for task in parts.tasks {
            let proxy = self.proxy.clone();
            #[cfg(not(web))]
            std::thread::spawn(move || {
                if let Some(message) = task() {
                    let _ = proxy.send_event(message);
                }
            });
            #[cfg(web)]
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(message) = task() {
                    let _ = proxy.send_event(message);
                }
            });
        }
        for future in parts.async_tasks {
            let proxy = self.proxy.clone();
            // Native: a task on the shared executor. `detach` because an effect must
            // outlive the handle — nothing here wants to cancel it, unlike a
            // subscription.
            #[cfg(not(web))]
            crate::runtime::spawn(async move {
                if let Some(message) = future.await {
                    let _ = proxy.send_event(message);
                }
            })
            .detach();
            // Web: the browser drives the future, single-threaded — `fetch` and friends
            // genuinely `await`, without blocking the loop.
            #[cfg(web)]
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(message) = future.await {
                    let _ = proxy.send_event(message);
                }
            });
        }
        for (delay, message) in parts.timers {
            let proxy = self.proxy.clone();
            #[cfg(not(web))]
            crate::runtime::spawn(async move {
                crate::runtime::sleep(delay).await;
                let _ = proxy.send_event(message);
            })
            .detach();
            #[cfg(web)]
            web_timer::after(delay.as_millis() as i32, move || {
                let _ = proxy.send_event(message);
            });
        }
    }

    /// Diffs the subscriptions the app declares against those running: starts the new
    /// ones and stops those that vanished, by dropping their `Sender`.
    fn sync_subscriptions(&mut self) {
        let entries = self.app.subscription().into_entries();
        let declared: std::collections::HashSet<u64> = entries.iter().map(|e| e.id).collect();

        // Stop the subscriptions that are no longer declared.
        self.running_subs.retain(|id, _| declared.contains(id));

        // Start the new ones.
        for entry in entries {
            if self.running_subs.contains_key(&entry.id) {
                continue;
            }
            let handle = self.start_subscription(entry.kind);
            self.running_subs.insert(entry.id, handle);
        }
    }

    /// Starts a subscription as a task on the shell's executor, and returns its
    /// cancellation handle — **dropping the handle cancels the task**, which is what
    /// `sync_subscriptions` above relies on when the application stops declaring it.
    ///
    /// This used to be a thread per subscription, parked in `recv_timeout` and woken
    /// by the timeout or by its `Sender` being dropped. A timer is the clearest case
    /// there is of work that only ever waits, so it is the clearest case for not
    /// spending a thread on it.
    #[cfg(not(web))]
    fn start_subscription(&self, kind: crate::subscription::Kind<A::Message>) -> SubHandle {
        let proxy = self.proxy.clone();
        match kind {
            crate::subscription::Kind::Every { interval, make } => {
                crate::runtime::spawn(async move {
                    loop {
                        crate::runtime::sleep(interval).await;
                        // The loop having closed is the other way this ends, and the
                        // application is gone by then, so there is nobody to tell.
                        if proxy.send_event(make(Instant::now())).is_err() {
                            break;
                        }
                    }
                })
            }
        }
    }

    /// Starts a subscription on the **Web**: a browser `setInterval`, with no thread.
    /// Returns its cancellation handle, which calls `clearInterval` on drop. The proxy
    /// feeds the message back into the loop on every tick.
    #[cfg(web)]
    fn start_subscription(&self, kind: crate::subscription::Kind<A::Message>) -> SubHandle {
        let proxy = self.proxy.clone();
        match kind {
            crate::subscription::Kind::Every { interval, make } => {
                let ms = interval.as_millis() as i32;
                web_timer::Interval::new(ms, move || {
                    let _ = proxy.send_event(make(Instant::now()));
                })
            }
        }
    }

    /// Applies the mouse drag under way.
    fn handle_drag(&mut self) {
        let Some(mut drag) = self.drag.take() else {
            return;
        };
        // How far this pointer has to travel before a press counts as a drag.
        let slop = self.hit_slop();
        match &mut drag {
            Drag::Scrollbar {
                id,
                vertical,
                grab,
                track_start,
                track_len,
                thumb_len,
                max,
                reverse,
            } => {
                let along = if *vertical {
                    self.cursor.y
                } else {
                    self.cursor.x
                };
                let travel = (*track_len - *thumb_len).max(1.0);
                let thumb_start = (along - *grab).clamp(*track_start, *track_start + travel);
                let fraction = (thumb_start - *track_start) / travel;
                // A reversed axis numbers its offsets from the far end, so the thumb's
                // position along the track reads backwards.
                let fraction = if *reverse { 1.0 - fraction } else { fraction };
                let offset = (fraction * *max).clamp(0.0, *max);
                let entry = self.runtime.scroll.entry(*id).or_insert((0.0, 0.0));
                if *vertical {
                    entry.1 = offset;
                } else {
                    entry.0 = offset;
                }
                // A precise drag: the target follows the offset and inertia is cut off.
                let synced = *entry;
                self.runtime.scroll_target.insert(*id, synced);
                self.runtime.scroll_velocity.remove(&*id);
            }
            Drag::TextSelect { id, rect } => {
                let local_x = self.cursor.x - rect.x;
                let local_y = self.cursor.y - rect.y
                    + self.runtime.scroll.get(id).map(|s| s.1).unwrap_or(0.0);
                // The field is focused during the drag, so scroll from the current caret.
                let scroll_cursor = self.runtime.edits.get(id).map(|e| e.cursor).unwrap_or(0);
                let cursor = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), *id))
                    .and_then(|widget| {
                        widget.cursor_at(local_x, local_y, rect.width, scroll_cursor)
                    });
                if let Some(cursor) = cursor {
                    let edit = self.runtime.edits.entry(*id).or_default();
                    if edit.anchor.is_none() {
                        edit.anchor = Some(edit.cursor);
                    }
                    edit.cursor = cursor;
                }
            }
            Drag::SelectionHandle {
                id,
                rect,
                handle,
                grab,
            } => {
                // The text position the handle stands for, carried with the finger, and
                // the retained vertical scroll folded in as for a press.
                let local_x = self.cursor.x - rect.x + grab.x;
                let local_y = self.cursor.y - rect.y
                    + grab.y
                    + self.runtime.scroll.get(id).map(|s| s.1).unwrap_or(0.0);
                let edit = self.runtime.edits.get(id).copied().unwrap_or_default();
                let moved = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), *id))
                    .and_then(|widget| widget.cursor_at(local_x, local_y, rect.width, edit.cursor))
                    .and_then(|to| crate::selection::drag(edit, *handle, to));
                if let Some(moved) = moved {
                    self.runtime.edits.insert(*id, moved);
                }
            }
            Drag::Widget { id, rect, last_x } => {
                let dx = self.cursor.x - *last_x;
                *last_x = self.cursor.x;
                self.apply_widget_drag(*id, *rect, dx);
            }
            Drag::Reorder { start, moved, .. } => {
                // Past the threshold this is a real drag, and no longer a sort.
                let dx = self.cursor.x - start.x;
                let dy = self.cursor.y - start.y;
                if !*moved && (dx * dx + dy * dy) > slop * slop {
                    *moved = true;
                }
                // The ghost card follows the pointer, so redraw on every move.
                if *moved {
                    self.request_redraw();
                }
            }
            Drag::Pan {
                id,
                last,
                moved,
                viewport,
            } => {
                let dx = self.cursor.x - last.x;
                let dy = self.cursor.y - last.y;
                if !*moved && (dx * dx + dy * dy) > slop * slop {
                    *moved = true;
                }
                if *moved {
                    let view = self.runtime.interactive.entry(*id).or_default();
                    // The finger pushes the content, bounded to the frame.
                    *view = view.pan(dx, dy).clamped(*viewport);
                    *last = self.cursor;
                    self.track_gesture();
                }
            }
            Drag::Item {
                source,
                start,
                moved,
                over,
            } => {
                let dx = self.cursor.x - start.x;
                let dy = self.cursor.y - start.y;
                // Under the threshold nothing is lifted: the press may still be a click
                // on whatever the draggable wraps.
                if !*moved && (dx * dx + dy * dy) > slop * slop {
                    *moved = true;
                }
                if !*moved {
                    return;
                }
                // The target under the pointer, but only if it would take this payload:
                // a target that refuses is not highlighted, so the answer is visible
                // before the finger lifts rather than after.
                let payload = source.payload;
                let candidate = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.drop_zone_at(self.cursor))
                    .filter(|zone| {
                        self.tree
                            .as_ref()
                            .and_then(|tree| find_widget(tree.as_ref(), zone.id))
                            .is_some_and(|widget| widget.accepts_drag(payload))
                    })
                    .map(|zone| zone.id);
                if *over != candidate {
                    *over = candidate;
                    self.runtime.drag_over = candidate;
                }
                self.request_redraw();
            }
            Drag::Scroll {
                id,
                last,
                moved,
                carried,
                dismiss,
                axis,
            } => {
                let dx = self.cursor.x - last.x;
                let dy = self.cursor.y - last.y;
                // Under the threshold we do not scroll yet; the gesture may be a tap.
                if !*moved && (dx * dx + dy * dy) > slop * slop {
                    *moved = true;
                    // The moment of arbitration. A scroll and a swipe start identically,
                    // so the question is not who is on top but **which way the finger
                    // went**: along the item's swipe axis it is a dismissal, across it a
                    // scroll. Deciding once, here, is what keeps the loser out of the
                    // gesture entirely — a swipe that also scrolled the list would be
                    // worse than either.
                    if let Some(item) = dismiss.take() {
                        let along = if item.spec.axis.is_horizontal() {
                            dx.abs() > dy.abs()
                        } else {
                            dy.abs() > dx.abs()
                        };
                        if along {
                            // The list gives the offset back untouched: it never moved.
                            self.runtime.release_scroll(*id);
                            self.drag = Some(Drag::Dismiss {
                                item,
                                last: *last,
                                moved: true,
                            });
                            self.handle_drag();
                            return;
                        }
                    }
                    // And the same arbitration between the two **axes** — which settles,
                    // with it, *which* of the scrollables under the finger the gesture
                    // belongs to. The reference does both at once in its arena: every
                    // scrollable in the stack enters a recogniser for its own axis, the
                    // one the finger matches wins, and the losers are out of the gesture
                    // entirely. Without the first half a page scrolling down drifts
                    // sideways the whole way, since no finger travels in a straight line;
                    // without the second, a strip of chips that only slides across stops
                    // the page behind it from scrolling at all.
                    let chain: Vec<_> = self
                        .ui
                        .as_ref()
                        .map(|ui| ui.scroll_chain(*last).collect())
                        .unwrap_or_default();
                    let down = dy.abs() >= dx.abs();
                    if let Some(area) = claim_area(&chain, *id, down) {
                        if area.id != *id {
                            // The area under the finger lost: it gives its offset back
                            // untouched, since it never moved. The winner is caught and
                            // held exactly as the press would have done to it.
                            self.runtime.release_scroll(*id);
                            let physics = area.physics_or(self.app.scroll_physics());
                            *carried = self.runtime.catch_scroll_fling(area.id, physics);
                            self.runtime.hold_scroll(area.id);
                            *id = area.id;
                        }
                        *axis = Some(claim_axis(dx, dy, area.max_x > 0.0, area.max_y > 0.0));
                    } else {
                        *axis = Some(down);
                    }
                }
                // Only the claimed axis moves. The other delta is not held back for later:
                // it is not part of this gesture at all.
                let (dx, mut dy) = match *axis {
                    Some(true) => (0.0, dy),
                    Some(false) => (dx, 0.0),
                    None => (dx, dy),
                };
                if *moved && *axis == Some(true) {
                    // A list inside a sheet shares the finger with it (milestone 515).
                    // Every movement is split — the sheet first going up, the list
                    // first going down — so the gesture changes hands in the middle of a
                    // drag, and what the list is left with goes through its physics below
                    // exactly as a movement of its own would.
                    let sheet = self
                        .ui
                        .as_ref()
                        .filter(|ui| ui.scroll_region(*id).is_some_and(|a| !a.reverse_y))
                        .and_then(|ui| ui.sheet_holding(*id))
                        .filter(|sheet| sheet.available > 0.0)
                        .cloned();
                    if let Some(sheet) = sheet {
                        let px = sheet.available;
                        let offset = self.runtime.scroll.get(id).map_or(0.0, |o| o.1);
                        let size = self.runtime.sheet_size(sheet.id, &sheet.spec) * px;
                        let (grown, listed) = frus_widgets::split_sheet_drag(
                            -dy,
                            offset,
                            size,
                            sheet.spec.floor() * px,
                            sheet.spec.max * px,
                        );
                        if grown != 0.0 {
                            self.runtime
                                .sheet_drag(sheet.id, &sheet.spec, grown / px, px);
                        }
                        dy = -listed;
                    }
                }
                if *moved {
                    let area = self.ui.as_ref().and_then(|u| u.scroll_region(*id));
                    let physics = area
                        .map(|a| a.physics_or(self.app.scroll_physics()))
                        .unwrap_or_else(|| self.app.scroll_physics());
                    let cur = self.runtime.scroll.get(id).copied().unwrap_or((0.0, 0.0));
                    // The finger pushes the content, so we follow the delta at once —
                    // but only as far as the physics allows. Past an edge, bouncing
                    // physics resists more and more (the rubber band) while clamping
                    // physics refuses the move outright.
                    // What the physics refuses is exactly the distance the user asked
                    // for and did not get — which is what the glow acknowledges.
                    let axis = |metrics: frus_widgets::ScrollMetrics, delta: f32| {
                        let applied = physics.apply_user_offset(metrics, delta);
                        let proposed = metrics.pixels + applied;
                        let refused = physics.apply_boundary_conditions(metrics, proposed);
                        (proposed - refused, refused)
                    };
                    let (nx, ny, refused) = match area {
                        Some(area) => {
                            let (ddx, ddy) = area.offset_delta((dx, dy));
                            let (nx, rx) = axis(area.metrics_x(cur.0), ddx);
                            let (ny, ry) = axis(area.metrics_y(cur.1), ddy);
                            (nx, ny, Some((area, rx, ry)))
                        }
                        None => (cur.0 - dx, cur.1 - dy, None),
                    };
                    if let Some((area, rx, ry)) = refused {
                        // A pull-to-refresh area listening above this scrollable takes
                        // the **top** edge; the glow takes the other three. Both answer
                        // "there is nothing more that way", and giving both would say it
                        // twice — the indicator being the more useful of the two, since
                        // it leads somewhere.
                        if let Some(host) = area.refresh {
                            self.feed_refresh(host, physics, area.viewport.height, cur.1, ny, ry);
                        }
                        let cursor = self.cursor;
                        for (refused, vertical, extent) in [
                            (rx, false, area.viewport.width),
                            (ry, true, area.viewport.height),
                        ] {
                            if refused.abs() < 1e-3 {
                                continue;
                            }
                            let edge = area.refused_edge(vertical, refused);
                            // The refresh area takes the edge the axis **starts** at,
                            // which is the bottom of a reversed one.
                            if area.refresh.is_some() && edge == area.start_edge(vertical) {
                                continue;
                            }
                            let (offset, cross) =
                                frus_widgets::glow_cross_axis(area.viewport, edge, cursor);
                            self.runtime
                                .glow_pull(area.id, edge, refused, extent, offset, cross);
                        }
                    }
                    self.runtime.scroll.insert(*id, (nx, ny));
                    self.runtime.scroll_target.insert(*id, (nx, ny));
                    self.runtime.scroll_velocity.remove(id);
                    *last = self.cursor;
                    self.track_gesture();
                }
            }
            Drag::Dismiss { item, last, moved } => {
                let dx = self.cursor.x - last.x;
                let dy = self.cursor.y - last.y;
                if !*moved && (dx * dx + dy * dy) > slop * slop {
                    *moved = true;
                }
                if *moved {
                    let delta = if item.spec.axis.is_horizontal() {
                        dx
                    } else {
                        dy
                    };
                    self.runtime
                        .dismiss_drag(item.id, delta, item.extent(), item.spec.axis);
                    *last = self.cursor;
                    self.track_gesture();
                }
            }
            Drag::Sheet {
                id,
                last,
                moved,
                available,
            } => {
                let dx = self.cursor.x - last.x;
                let dy = self.cursor.y - last.y;
                if !*moved && (dx * dx + dy * dy) > slop * slop {
                    *moved = true;
                }
                if *moved && *available > 0.0 {
                    let spec = self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.sheet(*id))
                        .map(|sheet| sheet.spec.clone());
                    if let Some(spec) = spec {
                        // Up grows it: the screen's y runs the other way.
                        self.runtime
                            .sheet_drag(*id, &spec, -dy / *available, *available);
                    }
                    *last = self.cursor;
                    self.track_gesture();
                }
            }
            Drag::Back { start_x } => {
                let width = self.logical_width();
                // Under RTL the finger slides **left** from the right edge, so progress
                // runs the other way.
                let sign = if self.is_rtl() { -1.0 } else { 1.0 };
                let progress = (sign * (self.cursor.x - *start_x) / width).clamp(0.0, 1.0);
                self.track_gesture();
                self.build_dirty = true;
                self.app.back_gesture(progress);
            }
        }
        self.drag = Some(drag);
        self.request_redraw();
    }

    /// The refresh area a scrollable sits inside, if any.
    fn refresh_host_of(&self, scrollable: WidgetId) -> Option<WidgetId> {
        self.ui
            .as_ref()
            .and_then(|ui| ui.scroll_region(scrollable))
            .and_then(|area| area.refresh)
    }

    /// Ends the pull of whichever refresh area holds `scrollable`, and dispatches its
    /// message when the pull was armed.
    fn release_refresh(&mut self, scrollable: WidgetId) {
        let Some(host) = self.refresh_host_of(scrollable) else {
            return;
        };
        if !self.runtime.refresh_release(host) {
            return;
        }
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), host))
            .and_then(|widget| widget.on_refresh());
        if let Some(message) = message {
            self.dispatch(message);
        }
    }

    /// Feeds one move of a drag into the pull of the refresh area `host`.
    ///
    /// The two physics put the overscroll in **different places**, so the signal is
    /// read differently:
    ///
    /// - **Clamping** refuses the movement and pins the offset at the edge, so the
    ///   refused amount is the only trace the gesture leaves. It is incremental, and
    ///   the physics returns nothing at all for a move back towards the content — so
    ///   an eased-off pull holds rather than retracting, which is right: the finger has
    ///   not let go.
    /// - **Bouncing** lets the offset go past the edge, so the *depth* it reached is
    ///   the signal, and the change in that depth is signed. The indicator therefore
    ///   follows the rubber band back in as the finger returns, which is also right:
    ///   there, the content itself is already saying so.
    ///
    /// Leaving the top edge at all ends the pull. The gesture has become an ordinary
    /// scroll, and an indicator still hanging there would be promising something the
    /// release is no longer going to deliver.
    fn feed_refresh(
        &mut self,
        host: WidgetId,
        physics: frus_widgets::ScrollPhysics,
        extent: f32,
        before: f32,
        after: f32,
        refused_y: f32,
    ) {
        if after > 0.5 {
            self.runtime.refresh_cancel(host);
            return;
        }
        let pulled = if physics.allows_overscroll() {
            (-after).max(0.0) - (-before).max(0.0)
        } else {
            -refused_y
        };
        if pulled.abs() > 1e-3 {
            self.runtime.refresh_pull(host, pulled, extent);
        }
    }

    /// How far the pointer must travel before a press becomes a drag, given what
    /// kind of pointer it is.
    fn hit_slop(&self) -> f32 {
        if self.pointer_touch {
            TOUCH_SLOP
        } else {
            PRECISE_SLOP
        }
    }

    /// Seconds since the drag under way began — the clock the velocity tracker's
    /// samples are stamped with.
    fn gesture_now(&self) -> f32 {
        (Instant::now() - self.gesture_start).as_secs_f32()
    }

    /// Starts a fresh gesture: the history of the previous one must not leak into
    /// the next, or a flick left then right would fling the wrong way.
    fn begin_gesture(&mut self) {
        self.gesture_velocity = VelocityTracker::platform_default();
        self.gesture_start = Instant::now();
        self.track_gesture();
    }

    /// Records where the pointer is now.
    fn track_gesture(&mut self) {
        let now = self.gesture_now();
        self.gesture_velocity.add_position(now, self.cursor);
    }

    /// What the gesture tracker makes of the drag as it stands.
    fn gesture_estimate(&self) -> VelocityEstimate {
        self.gesture_velocity
            .estimate(self.gesture_now())
            .unwrap_or(VelocityEstimate::STILL)
    }

    /// [`fling_velocity`], gated on this pointer's slop.
    fn fling_velocity(&self, estimate: VelocityEstimate) -> (f32, f32) {
        fling_velocity(estimate, self.hit_slop())
    }

    /// A scroll fling: projects each axis's ballistic destination, friction in closed
    /// form, bounds it with the elastic overshoot, and primes the scroll spring with
    /// the finger's momentum.
    fn fling(&mut self, id: WidgetId, velocity: (f32, f32)) {
        let Some(area) = self.ui.as_ref().and_then(|ui| ui.scroll_region(id)) else {
            return;
        };
        let physics = area.physics_or(self.app.scroll_physics());
        // The physics decides everything from here: whether there is a fling at all,
        // how far it runs, and what happens at the edges. A release too slow to
        // fling still gets a chance to spring an overscroll back.
        if self.runtime.fling_scroll(area, physics, velocity) {
            self.request_redraw();
        }
    }

    /// The topmost **reorderable** widget under `point`, as `(id, flat index)`. It
    /// uses the reorderables' registry, which is independent of clicking, and so covers
    /// the Kanban cards and drop zones, which are not clickable.
    fn reorderable_at(&self, point: Point) -> Option<(WidgetId, usize, bool)> {
        let id = self.ui.as_ref()?.reorderable_at(point)?;
        let widget = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))?;
        // A target-**only** widget, a drop zone: reorderable but not grabbable, so no
        // drag starts on it. Dropping still aims at it, through `ui.reorderable_at`.
        if !widget.reorder_draggable() {
            return None;
        }
        let from = widget.reorder_index()?;
        Some((id, from, widget.drag_needs_long_press()))
    }

    /// **What is actually moving**, given what was grabbed: its id and its box.
    ///
    /// Usually they are the same widget. They are not when what was grabbed is a **grip**
    /// — a source that is not a target — because a grip is a 40-pixel gutter inside a much
    /// larger row, and everything the preview is made of is the row's: the ghost is the
    /// row lifted out of the frame, the gap that closes behind it is the row's height, and
    /// the band the neighbours are matched against is the row's width. Taking the grip's
    /// box instead would lift an icon and open a slot two centimetres too narrow.
    ///
    /// The row is found by the index the two share, among the reorderables under the
    /// grip's own middle — which is inside its row by construction.
    fn reorder_source(&self, id: WidgetId, from: usize) -> Option<(WidgetId, Rect)> {
        let ui = self.ui.as_ref()?;
        let tree = self.tree.as_ref()?;
        let own = ui.widget_rect(id)?;
        let grabbed = find_widget(tree.as_ref(), id)?;
        if grabbed.reorder_droppable() {
            return Some((id, own));
        }
        let middle = Point::new(own.x + own.width * 0.5, own.y + own.height * 0.5);
        let slot = ui.reorderables_at(middle).find(|other| {
            find_widget(tree.as_ref(), *other)
                .is_some_and(|w| w.reorder_droppable() && w.reorder_index() == Some(from))
        })?;
        // A grip whose row is nowhere to be found still moves something: itself, which is
        // wrong-looking but not a crash, and cannot happen while the two are built
        // together.
        Some((slot, ui.widget_rect(slot).unwrap_or(own)))
    }

    /// The row the reorder under way carries, and its box: the one taken when the drag was
    /// first carried, moved with the content since — or, before any frame has carried it,
    /// the frame's.
    fn reorder_carried(&self) -> Option<(WidgetId, Rect)> {
        match &self.drag {
            Some(Drag::Reorder {
                carried: Some(carried),
                ..
            }) => Some(*carried),
            Some(Drag::Reorder { id, from, .. }) => self.reorder_source(*id, *from),
            _ => None,
        }
    }

    /// Takes the carried row's box from the frame, once: the first frame a reorder is carried
    /// in, before anything has scrolled it. From then on it goes with the content, and the
    /// frame is no longer asked — a row carried to an edge is clipped by it, and a row
    /// scrolled out of sight is not in the frame at all.
    fn keep_carried_box(&mut self) {
        let source = match &self.drag {
            Some(Drag::Reorder {
                id,
                from,
                moved: true,
                carried: None,
                ..
            }) => self.reorder_source(*id, *from),
            _ => return,
        };
        if let Some(Drag::Reorder { carried, .. }) = self.drag.as_mut() {
            *carried = source;
        }
    }

    /// The topmost reorderable under `point` that something can actually be **dropped**
    /// on. It is not always the topmost reorderable: a list row's grip is a source and
    /// not a target, and what is under it is the row the drop is really aimed at.
    fn reorder_target_at(&self, point: Point) -> Option<WidgetId> {
        let ui = self.ui.as_ref()?;
        let tree = self.tree.as_ref()?;
        ui.reorderables_at(point)
            .find(|id| find_widget(tree.as_ref(), *id).is_some_and(|w| w.reorder_droppable()))
    }

    /// Where the reorderable `id`, grabbed at index `from`, is **dropped** if released now:
    /// the target under the pointer, or — over no target at all — the nearest slot of its own
    /// list, for one inserted between its neighbours, along the axis it is reordered on.
    ///
    /// The insertion line and the release both ask this, so the line cannot promise a place
    /// the drop does not keep. The second half is milestone 518: a row carried to the bottom
    /// of a phone's list is over what follows the list, and the release used to put it back.
    /// Milestone 527 turned it on its side, for a list whose rows run across.
    fn reorder_drop_target(
        &self,
        id: WidgetId,
        from: usize,
        carried: Option<(WidgetId, Rect)>,
    ) -> Option<WidgetId> {
        if let Some(target) = self.reorder_target_at(self.cursor) {
            return Some(target);
        }
        let tree = self.tree.as_ref()?;
        let Some(ReorderMotion::Slots(axis)) = find_widget(tree.as_ref(), id).map(reorder_motion)
        else {
            return None;
        };
        // Among the slots of the row that moves, which is not the grip that was grabbed —
        // and which may have been scrolled out of the frame by the carry itself.
        let row = match carried {
            Some((row, _)) => row,
            None => self.reorder_source(id, from)?.0,
        };
        let slots = reorder_siblings(self.ui.as_ref()?, tree.as_ref(), row);
        let boxes: Vec<Rect> = slots.iter().map(|(_, rect)| *rect).collect();
        nearest_reorder_slot(self.cursor, &boxes, axis).map(|index| slots[index].0)
    }

    /// What is being **carried** this frame, where it is now: the box the ghost is drawn
    /// at, for a reorder or for a lifted item. `None` when nothing is engaged.
    ///
    /// It is the ghost's box and not the pointer, because the reference scrolls when the
    /// *item* reaches the edge, and that is the honest moment: a row half off the bottom
    /// is already asking to go further, whatever the finger holding it is over.
    fn carried_rect(&self) -> Option<Rect> {
        match &self.drag {
            Some(Drag::Reorder {
                start, moved: true, ..
            }) => {
                let (_, src) = self.reorder_carried()?;
                // The same offsets the ghost is painted at, and for the same reason a
                // column's ghost only rises: it moves along its own axis.
                let (gx, gy) = ghost_offset(
                    self.dragged_reorder_motion()
                        .unwrap_or(ReorderMotion::Columns),
                    (self.cursor.x - start.x, self.cursor.y - start.y),
                );
                Some(src.translate(gx, gy))
            }
            Some(Drag::Item {
                source,
                start,
                moved: true,
                ..
            }) => Some(
                source
                    .rect
                    .translate(self.cursor.x - start.x, self.cursor.y - start.y),
            ),
            _ => None,
        }
    }

    /// Scrolls the area under the pointer while what is being carried hangs past one of
    /// its edges — the gap `Draggable` has had since milestone 285, and what a list longer
    /// than a screen needs before it can be reordered at all.
    ///
    /// The area is the innermost one under the **pointer** that a user could move by
    /// hand: an area that refuses a finger refuses this too, so a list that fits its
    /// viewport never twitches.
    fn autoscroll_carried(&mut self, dt: f32) -> bool {
        self.keep_carried_box();
        let Some(carried) = self.carried_rect() else {
            return false;
        };
        let area = {
            let scroll = &self.runtime.scroll;
            self.ui.as_ref().and_then(|ui| {
                ui.scroll_chain(self.cursor).find(|area| {
                    area.accepts_user_offset(scroll.get(&area.id).copied().unwrap_or((0.0, 0.0)))
                })
            })
        };
        let Some(area) = area else {
            return false;
        };
        let offset = self
            .runtime
            .scroll
            .get(&area.id)
            .copied()
            .unwrap_or((0.0, 0.0));
        // What is still hidden on either side, in **screen** terms: on a reversed axis
        // the offset is measured from the other end, so the two swap over.
        let room = |offset: f32, max: f32, reverse: bool| {
            if reverse {
                (max - offset, offset)
            } else {
                (offset, max - offset)
            }
        };
        let view = area.viewport;
        let dx = edge_autoscroll(
            (carried.x, carried.x + carried.width),
            (view.x, view.x + view.width),
            room(offset.0, area.max_x, area.reverse_x),
            dt,
        )
        .unwrap_or(0.0);
        let dy = edge_autoscroll(
            (carried.y, carried.y + carried.height),
            (view.y, view.y + view.height),
            room(offset.1, area.max_y, area.reverse_y),
            dt,
        )
        .unwrap_or(0.0);
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        // A movement of the content becomes a change of offset the area's own way, which
        // is the one place the sign of a reversed axis is decided.
        let delta = area.offset_delta((dx, dy));
        let next = (
            (offset.0 + delta.0).clamp(0.0, area.max_x),
            (offset.1 + delta.1).clamp(0.0, area.max_y),
        );
        if next == offset {
            return false;
        }
        self.runtime.scroll.insert(area.id, next);
        // The target follows, or the inertia the area was resting at would spring the
        // list straight back out from under the row.
        self.runtime.scroll_target.insert(area.id, next);
        self.runtime.scroll_velocity.remove(&area.id);
        // What is carried stays under the finger. Its ghost is drawn at the box the frame
        // puts it at, offset by how far the finger has moved since the press; the frame now
        // puts that box where the content went, so the press goes there too. Otherwise the
        // ghost rides up with the list, stops hanging over the edge, and the scroll it
        // started starves itself — seen on a phone, milestone 517.
        if let Some(drag) = self.drag.as_mut() {
            follow_content(
                drag,
                area.offset_delta((next.0 - offset.0, next.1 - offset.1)),
            );
        }
        true
    }

    /// How the reorderable currently **grabbed** moves, if a drag is under way. It is what
    /// keeps the **pointer spring** (`reorder_x` following the pointer) to `Table`'s columns;
    /// the Kanban cards and a list's rows spring their insertion line instead.
    fn dragged_reorder_motion(&self) -> Option<ReorderMotion> {
        let Some(Drag::Reorder { id, .. }) = self.drag else {
            return None;
        };
        self.tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(reorder_motion)
    }

    /// Steps the reorder springs one frame, per gesture — see the frame loop — and says
    /// whether they are still moving.
    fn advance_reorder_springs(&mut self, dt: f32) -> bool {
        let reorder_motion = matches!(self.drag, Some(Drag::Reorder { moved: true, .. }))
            .then(|| self.dragged_reorder_motion())
            .flatten();
        match reorder_motion {
            Some(ReorderMotion::Columns) => {
                self.reorder_x = spring_toward(self.reorder_x, self.cursor.x, dt, 0.07);
                (self.cursor.x - self.reorder_x).abs() > 0.5
            }
            Some(ReorderMotion::Slots(ReorderAxis::Vertical)) => {
                match self.reorder_drop_line(drag_preview::INSERT_THICKNESS) {
                    Some(target) => {
                        self.reorder_y = spring_toward(self.reorder_y, target.y, dt, 0.07);
                        (target.y - self.reorder_y).abs() > 0.5
                    }
                    None => false,
                }
            }
            Some(ReorderMotion::Slots(ReorderAxis::Horizontal)) => {
                match self.reorder_drop_line(drag_preview::INSERT_THICKNESS) {
                    Some(target) => {
                        self.reorder_x = spring_toward(self.reorder_x, target.x, dt, 0.07);
                        (target.x - self.reorder_x).abs() > 0.5
                    }
                    None => false,
                }
            }
            None => false,
        }
    }

    /// Paints the **reorder preview** on top of the scene, unclipped: the source
    /// column dimmed, a **drop indicator** at the target column's insertion edge, and a
    /// **lifted card** — shadow plus a `primary` border — following the pointer. No
    /// effect outside an engaged header drag.
    /// Names, on the console, every box this frame whose children did not fit inside it.
    ///
    /// The reference treats an overflowing flex as an error condition, paints a striped
    /// band along the offending edge and writes to the console; nothing here said anything
    /// at all, which is how a task row's delete button came to be laid out past the window
    /// and stayed there for three milestones — drawn nowhere, hittable nowhere. Half of
    /// that is now here: the words, once per site, with the two ways out the reference
    /// suggests. The striped band is not.
    ///
    /// Once per site, because a layout that does not fit does not fit on every frame, and
    /// sixty lines a second is the same as silence.
    fn report_overflows(&self, ui: &Ui<A::Message>) {
        for o in ui.overflows() {
            // The site: a box, an edge, and the amount rounded to the pixel. Enough to
            // stay quiet while a window is resized past the same defect, and to speak up
            // when the defect changes.
            let mut key = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            (
                o.rect.width.to_bits(),
                o.rect.height.to_bits(),
                o.side as u8,
                o.amount.round() as i32,
            )
                .hash(&mut key);
            if !self.reported_overflows.borrow_mut().insert(key.finish()) {
                continue;
            }
            let side = match o.side {
                frus_widgets::Side::Left => "left",
                frus_widgets::Side::Right => "right",
                frus_widgets::Side::Top => "top",
                frus_widgets::Side::Bottom => "bottom",
            };
            log::warn!(
                "a box {:.0}x{:.0} is overflowed by {:.0} px on the {side}: its children                  do not fit inside it, and what runs past the edge is drawn outside its                  parent — invisible where something clips it, and untappable where it                  leaves the window. Give the child that should give way an `Expanded`,                  or put the content in a `SingleChildScrollView`.",
                o.rect.width,
                o.rect.height,
                o.amount,
            );
        }
    }

    /// Winit's key event as a [`KeyStroke`] — the general vocabulary shortcuts are bound
    /// to, as opposed to [`Key`], which is the one text editing needs.
    ///
    /// `None` for anything with no place in a shortcut: a dead key, a compose sequence, a
    /// modifier pressed on its own.
    fn keystroke(&self, event: &winit::event::KeyEvent) -> Option<KeyStroke> {
        let key = match &event.logical_key {
            WinitKey::Named(NamedKey::Enter) => ShortcutKey::Enter,
            WinitKey::Named(NamedKey::Escape) => ShortcutKey::Escape,
            WinitKey::Named(NamedKey::Tab) => ShortcutKey::Tab,
            WinitKey::Named(NamedKey::Space) => ShortcutKey::Space,
            WinitKey::Named(NamedKey::Backspace) => ShortcutKey::Backspace,
            WinitKey::Named(NamedKey::Delete) => ShortcutKey::Delete,
            WinitKey::Named(NamedKey::ArrowUp) => ShortcutKey::Up,
            WinitKey::Named(NamedKey::ArrowDown) => ShortcutKey::Down,
            WinitKey::Named(NamedKey::ArrowLeft) => ShortcutKey::Left,
            WinitKey::Named(NamedKey::ArrowRight) => ShortcutKey::Right,
            WinitKey::Named(NamedKey::Home) => ShortcutKey::Home,
            WinitKey::Named(NamedKey::End) => ShortcutKey::End,
            WinitKey::Named(NamedKey::PageUp) => ShortcutKey::PageUp,
            WinitKey::Named(NamedKey::PageDown) => ShortcutKey::PageDown,
            WinitKey::Named(NamedKey::F1) => ShortcutKey::F(1),
            WinitKey::Named(NamedKey::F2) => ShortcutKey::F(2),
            WinitKey::Named(NamedKey::F3) => ShortcutKey::F(3),
            WinitKey::Named(NamedKey::F4) => ShortcutKey::F(4),
            WinitKey::Named(NamedKey::F5) => ShortcutKey::F(5),
            WinitKey::Named(NamedKey::F6) => ShortcutKey::F(6),
            WinitKey::Named(NamedKey::F7) => ShortcutKey::F(7),
            WinitKey::Named(NamedKey::F8) => ShortcutKey::F(8),
            WinitKey::Named(NamedKey::F9) => ShortcutKey::F(9),
            WinitKey::Named(NamedKey::F10) => ShortcutKey::F(10),
            WinitKey::Named(NamedKey::F11) => ShortcutKey::F(11),
            WinitKey::Character(text) => ShortcutKey::Char(text.chars().next()?),
            _ => return None,
        };
        Some(KeyStroke {
            key,
            ctrl: self.ctrl,
            shift: self.shift,
            alt: self.alt,
            meta: self.meta,
        })
    }

    fn paint_reorder_preview(&self, ui: &Ui<A::Message>, theme: &Theme, scene: &mut Scene) {
        let Some(Drag::Reorder {
            start, moved: true, ..
        }) = self.drag
        else {
            return;
        };
        // What moves is the row, even when what was grabbed is the grip inside it — at the
        // box it was carried from, which has gone with the content.
        let Some((id, src)) = self.reorder_carried() else {
            return;
        };
        let motion = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(reorder_motion)
            .unwrap_or(ReorderMotion::Columns);
        let dx = self.cursor.x - start.x;
        let dy = self.cursor.y - start.y;

        // The ghost's offset, per motion — see `ghost_offset`.
        let (gx, gy) = ghost_offset(motion, (dx, dy));

        // The owners of the grabbed item's **subtree**: used by the ghost, to capture a
        // rich card's content, which its children paint (milestone 251), **and** by the
        // vertical reflow, to lift the card out of the preview. Fallback: the card alone.
        let owners: std::collections::HashSet<u64> = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(|w| subtree_ids(w, id).iter().map(|i| i.as_u64()).collect())
            .unwrap_or_else(|| std::iter::once(id.as_u64()).collect());

        match motion {
            ReorderMotion::Columns => {
                // Reflow the neighbouring columns: the source's gap closes and the drop
                // slot opens, following the pointer's **smoothed** abscissa — a gentle
                // inertial slide, while the ghost sticks to the real pointer.
                let reflowed =
                    reflow_reorder_columns(scene.primitives(), src, self.reorder_x, id.as_u64());
                scene.clear();
                for primitive in reflowed {
                    scene.push_primitive(primitive);
                }
            }
            ReorderMotion::Slots(axis) => {
                // Reflow the **cards**: the lifted card's gap closes in the source
                // column and a slot opens under the **insertion line** in the target one.
                // Then the line is laid on top, at the chosen edge — the hovered half,
                // milestone 252.
                //
                // A **smoothed** line (milestone 265): the chosen slot's width, abscissa
                // and thickness are kept, but the **ordinate** is replaced by the
                // `reorder_y` spring — the line *and* the gap slide between cards, with
                // vertical inertia, instead of jumping a notch. A horizontal list's line
                // stands on its end, and its **abscissa** is the one that springs.
                let line =
                    self.reorder_drop_line(drag_preview::INSERT_THICKNESS)
                        .map(|r| match axis {
                            ReorderAxis::Vertical => Rect {
                                y: self.reorder_y,
                                ..r
                            },
                            ReorderAxis::Horizontal => Rect {
                                x: self.reorder_x,
                                ..r
                            },
                        });
                // Only what can be reordered makes room — a card, a row, a drop zone, and
                // what each of them paints. The reflow is geometric, and a button floating
                // over the list or the navigation bar under it shares the band without being
                // part of the list: moved with it, the first left its `+` a row above its own
                // disc on a phone (milestone 517).
                let movable = self
                    .tree
                    .as_ref()
                    .map(|tree| reorderable_owners(ui, tree.as_ref()))
                    .unwrap_or_default();
                let reflowed =
                    reflow_reorder_cards(scene.primitives(), src, line, &owners, &movable, axis);
                scene.clear();
                for primitive in reflowed {
                    scene.push_primitive(primitive);
                }
                if let Some(line) = line {
                    // Rounded across its thickness, whichever way it stands.
                    let thickness = match axis {
                        ReorderAxis::Vertical => line.height,
                        ReorderAxis::Horizontal => line.width,
                    };
                    scene.set_clip(Rect::UNBOUNDED);
                    scene.draw_rect(
                        line,
                        theme.primary,
                        theme.radius.min(thickness * 0.5),
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
            }
        }

        // A faithful ghost: the grabbed item's primitives, taken from the original
        // scene, translated and **un-clipped**, since they would otherwise be cropped at
        // the source. In a **rich card** the background is painted by the card but its
        // content — label, tags, the × button — by children, under other owners, so we
        // capture the primitives of the item's **whole subtree**; see `owners` above.
        let ghost: Vec<Primitive> = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| owners.contains(&p.owner()))
            .map(|p| p.translated(gx, gy).with_clip(Rect::UNBOUNDED))
            .collect();
        draw_ghost_card(scene, theme, src.translate(gx, gy), &ghost);
    }

    /// The lifted item: its own primitives dimmed where it sits, and a copy of them
    /// floating under the pointer.
    ///
    /// The copy is taken from the frame rather than built again, so it cannot drift
    /// from what is on screen — a rebuilt "feedback" widget is a second definition of
    /// the same thing, and two definitions is one too many. The same reason the
    /// reorder ghost works this way.
    fn paint_drag_ghost(&self, ui: &Ui<A::Message>, theme: &Theme, scene: &mut Scene) {
        let Some(Drag::Item {
            source,
            start,
            moved: true,
            ..
        }) = &self.drag
        else {
            return;
        };
        let widget = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), source.id));
        // A rich item paints its background itself and its content through children,
        // under other owners, so the whole subtree is captured — the same reason the
        // reorder ghost does.
        let owners: std::collections::HashSet<u64> = widget
            .map(|w| {
                subtree_ids(w, source.id)
                    .iter()
                    .map(|i| i.as_u64())
                    .collect()
            })
            .unwrap_or_else(|| std::iter::once(source.id.as_u64()).collect());
        let opacity = widget.map(|w| w.drag_ghost_opacity()).unwrap_or(1.0);
        let dx = self.cursor.x - start.x;
        let dy = self.cursor.y - start.y;

        // What is left behind, faded in place: the item is being carried, not deleted.
        let original: Vec<Primitive> = scene.primitives().to_vec();
        scene.clear();
        for primitive in &original {
            if owners.contains(&primitive.owner()) {
                scene.push_faded(primitive, opacity);
            } else {
                scene.push_primitive(primitive.clone());
            }
        }

        let ghost: Vec<Primitive> = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| owners.contains(&p.owner()))
            .map(|p| p.translated(dx, dy).with_clip(Rect::UNBOUNDED))
            .collect();
        draw_ghost_card(scene, theme, source.rect.translate(dx, dy), &ghost);
    }

    /// The **insertion** line of the preview: a thin band at the edge of the hovered
    /// reorderable slot, a card, a row or a drop zone — the edge **before** it when the
    /// pointer is in its leading half, the edge **after** it in its trailing half. Across a
    /// vertical list; standing on its end in a horizontal one. `None` when the release would
    /// land nowhere.
    fn reorder_drop_line(&self, thickness: f32) -> Option<Rect> {
        let Some(Drag::Reorder {
            id, from, carried, ..
        }) = self.drag
        else {
            return None;
        };
        // The reorderable slot — card, row or drop zone — the release would land on: the
        // one under the pointer, or the nearest of the list's own.
        let target = self.reorder_drop_target(id, from, carried)?;
        let rect = self.ui.as_ref()?.widget_rect(target)?;
        let axis = match self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), target))
            .map(reorder_motion)
        {
            Some(ReorderMotion::Slots(axis)) => axis,
            _ => ReorderAxis::Vertical,
        };
        Some(drop_insertion_line(
            rect,
            thickness,
            self.reorder_insert_after(target, rect),
            axis,
            self.is_rtl(),
        ))
    }

    /// For a slot a reorderable is **inserted** at, tells whether insertion happens
    /// **after** it (index +1, between it and the next) — the pointer in its trailing half,
    /// which `reorder_drop_after` works out along the list's axis and reading direction.
    /// Always `false` for `Table`'s columns, which keep their drop logic unchanged. This is
    /// the insertion line's counterpart on the **routing** side.
    fn reorder_insert_after(&self, target: WidgetId, rect: Rect) -> bool {
        match self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), target))
            .map(reorder_motion)
        {
            Some(ReorderMotion::Slots(axis)) => {
                reorder_drop_after(self.cursor, rect, axis, self.is_rtl())
            }
            _ => false,
        }
    }

    /// Sends a value drag's **start** or **end** to the widget being dragged.
    ///
    /// The fraction is worked out the same way [`apply_widget_drag`](Self::apply_widget_drag)
    /// works it out, from the same rectangle: an end that disagreed with the last
    /// `on_drag` about where the pointer finished would be a slider that settles
    /// somewhere its own final message never mentioned.
    fn dispatch_drag_edge(&mut self, id: WidgetId, rect: frus_widgets::Rect, edge: Edge) {
        let fraction = drag_fraction(rect, self.cursor.x);
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| match edge {
                Edge::Start => widget.on_drag_start(fraction),
                Edge::End => widget.on_drag_end(fraction),
            });
        if let Some(message) = message {
            self.dispatch(message);
        }
    }

    fn apply_widget_drag(&mut self, id: WidgetId, rect: frus_widgets::Rect, dx: f32) {
        let fraction = drag_fraction(rect, self.cursor.x);
        // An accumulating handle, taking a delta, first; otherwise a slider's absolute
        // fraction.
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| {
                widget
                    .on_drag_delta(dx)
                    .or_else(|| widget.on_drag(fraction))
            });
        if let Some(message) = message {
            self.dispatch(message);
        }
    }

    /// The **system** back — Android's `KEYCODE_BACK`, the browser's back key — and
    /// its native chain: ① the topmost overlay closes, be it a sheet, a drawer, a modal
    /// or a menu; ② failing that a screen is popped, by replaying the **back gesture
    /// already committed**, through the same hooks as the swipe, so it settles with an
    /// animation; ③ failing that, at the root, back **quits the application**, as
    /// Android expects.
    fn system_back(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(message) = self.ui.as_ref().and_then(|ui| ui.top_dismiss()) {
            self.dispatch(message);
            self.request_redraw();
            return;
        }
        if self.app.can_go_back() {
            self.build_dirty = true;
            self.app.back_gesture(0.0);
            // A "committed" momentum: the projection passes the commit threshold and
            // the settle animates the pop.
            self.app.back_gesture_end(5.0);
            self.request_redraw();
            return;
        }
        event_loop.exit();
    }

    /// Routes **Escape**: a leaf-to-root walk from the focused widget, with a
    /// three-state result — `Ignored` keeps walking up, `Handled` consumes, `Skip` stops
    /// with no fallback — then falls back to closing the topmost overlay when nobody
    /// answered, or when nothing is focused.
    fn escape(&mut self) {
        // 1) Walk up the focus path. `Some(None)` means consumed with no message; an
        // outer `None` means the whole path ignored it, so we fall back.
        let outcome: Option<Option<A::Message>> = self.runtime.input.focused.and_then(|focused| {
            let tree = self.tree.as_ref()?;
            let path = find_path(tree.as_ref(), focused);
            for widget in path.iter().rev() {
                match widget.on_key(&Key::Escape) {
                    KeyResponse::Handled(message) => return Some(message),
                    KeyResponse::Skip => return Some(None),
                    KeyResponse::Ignored => {}
                }
            }
            None
        });

        match outcome {
            Some(message) => {
                if let Some(message) = message {
                    self.dispatch(message);
                    self.request_redraw();
                }
            }
            // 2) Nobody on the path, or nothing focused: the topmost overlay.
            None => {
                if let Some(message) = self.ui.as_ref().and_then(|ui| ui.top_dismiss()) {
                    self.dispatch(message);
                    self.request_redraw();
                }
            }
        }
    }

    /// Moves the caret vertically in the focused multi-line field — Up/Down when
    /// `page` is false, PgUp/PgDn otherwise — preserving the **remembered goal column**.
    /// Applies the selection under Shift and reveals the caret. Returns `true` when the
    /// caret moved within the field, `false` otherwise, in which case the shell
    /// navigates focus instead.
    fn move_caret_vertical(&mut self, id: WidgetId, down: bool, page: bool) -> bool {
        let width = self
            .ui
            .as_ref()
            .and_then(|ui| ui.widget_rect(id))
            .map(|r| r.width)
            .unwrap_or(0.0);
        let cursor = self.runtime.edits.get(&id).map(|e| e.cursor).unwrap_or(0);
        let moved = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.caret_vertical(width, cursor, down, page, self.goal_x));
        let Some((new_cursor, goal)) = moved else {
            return false;
        };
        let shift = self.shift;
        let edit = self.runtime.edits.entry(id).or_default();
        if shift {
            if edit.anchor.is_none() {
                edit.anchor = Some(edit.cursor);
            }
        } else {
            edit.anchor = None;
        }
        edit.cursor = new_cursor;
        // Remember the goal column for the next vertical jump.
        self.goal_x = Some(goal);
        self.reveal_caret(id, new_cursor);
        self.request_redraw();
        true
    }

    /// A hold in field `id` selects the word under the finger — the press has already
    /// put the caret there — and gives the selection its handles (milestone 511).
    fn select_held_word(&mut self, id: WidgetId) {
        let cursor = self.runtime.edits.get(&id).map(|e| e.cursor).unwrap_or(0);
        let word = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.word_at(cursor))
            .filter(|(start, end)| start < end);
        let Some((start, end)) = word else {
            return;
        };
        self.runtime.edits.insert(
            id,
            Edit {
                cursor: end,
                anchor: Some(start),
                composing: None,
            },
        );
        self.runtime.selection_handles = Some(id);
        self.runtime.close_edit_run(id);
        #[cfg(android)]
        self.push_ime_context(id);
    }

    /// The selection handle under the pointer, taken: a drag that moves its end of the
    /// focused field's selection. `None` when no handle is showing there.
    fn grab_selection_handle(&self) -> Option<Drag> {
        let id = self.runtime.selection_handles?;
        if self.runtime.input.focused != Some(id) {
            return None;
        }
        let rect = self.ui.as_ref()?.widget_rect(id)?;
        let edit = self.runtime.edits.get(&id).copied()?;
        let scroll_y = self.runtime.scroll.get(&id).map(|s| s.1).unwrap_or(0.0);
        let handles = find_widget(self.tree.as_ref()?.as_ref(), id)?
            .selection_handles(rect.width, &edit, scroll_y)?;
        let local = Point::new(self.cursor.x - rect.x, self.cursor.y - rect.y);
        let handle = crate::selection::grab(&handles, local)?;
        let at = handles[handle.index()].line_center;
        Some(Drag::SelectionHandle {
            id,
            rect,
            handle,
            grab: Point::new(at.x - local.x, at.y - local.y),
        })
    }

    /// Routes a key to the focused field: updates the editing state and applies
    /// whatever message comes out, a value change or a submission.
    fn apply_key(&mut self, id: WidgetId, key: Key) {
        // Any horizontal move, or any keystroke, forgets the vertical goal column.
        self.goal_x = None;
        // Typing puts a touch selection's handles away, as a press does (milestone 511).
        self.runtime.selection_handles = None;
        let was = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let mut edit = was;
        let widget = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id));
        // What the field held before the key: half of an undo step, and the half the
        // application owns.
        let before = widget
            .and_then(|widget| widget.text_value())
            .map(str::to_owned);
        let message = widget.and_then(|widget| widget.on_edit(&mut edit, &key));
        self.runtime.edits.insert(id, edit);
        // In a multi-line field, make the retained scroll follow the caret and reveal it.
        self.reveal_caret(id, edit.cursor);
        if let Some(message) = message {
            self.dispatch_edit(message);
        }
        // The history, recorded on the **evidence** of a changed value rather than on the
        // intent of a key that usually changes one: a filter may have refused the
        // character, a length limit may have swallowed it, a read-only field ignores it,
        // and an Enter submits instead of typing. The value is read again from the tree
        // the dispatch has just rebuilt, so what is recorded is what happened.
        let after = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.text_value())
            .map(str::to_owned);
        match (before, after) {
            (Some(before), Some(after)) if before != after => {
                let kind = EditKind::of(&key, was.selection_range().is_some(), self.composing());
                let since = self.since_last_edit();
                self.runtime
                    .record_edit(id, EditSnapshot::new(before, was), kind, since);
            }
            // A key that moved only the caret ends the run: what is typed next is a step
            // of its own, or one undo would take back two visits to the field at once.
            (Some(_), _) => self.runtime.close_edit_run(id),
            _ => {}
        }
    }

    /// Dispatches a message that changed a field's value, and refreshes the tree at once.
    ///
    /// Keys can arrive in **bursts**, faster than a frame — a software keyboard, `adb
    /// input text`, auto-repeat — and the next one must see the CURRENT value, not the
    /// retained tree's, or it would overwrite the previous keystroke. So the tree is
    /// refreshed right away; `build_dirty` stays raised and the next frame redoes the full
    /// pass: mounts, leaving fades and all.
    fn dispatch_edit(&mut self, message: A::Message) {
        self.dispatch(message);
        if let Some((width, height)) = self.last_size {
            let theme = self.themes.displayed(&self.app);
            // A surface of its own, because this build happens between frames rather than
            // inside one — and the same `build_view` as the frame path, because the next
            // key in the burst reads this tree straight away.
            let tree = self
                .media_query(width, height)
                .scope(|| build_view(&self.app, &theme, &self.runtime));
            self.tree = Some(tree);
        }
    }

    /// Seconds since the last recorded edit, and stamps this one. The first edit is
    /// infinitely far from the one before it, which is to say it starts a run.
    fn since_last_edit(&mut self) -> f32 {
        let now = Instant::now();
        let since = self
            .last_edit_at
            .map(|then| now.duration_since(then).as_secs_f32())
            .unwrap_or(f32::INFINITY);
        self.last_edit_at = Some(now);
        since
    }

    /// Whether an input method is in the middle of composing a word.
    fn composing(&self) -> bool {
        self.runtime
            .input
            .focused
            .and_then(|id| self.runtime.edits.get(&id))
            .is_some_and(|edit| edit.composing.is_some())
    }

    /// Steps a text field's value back one change, or forward again. `true` when something
    /// moved — Ctrl+Z on anything that is not a field, or with nothing left to undo, moves
    /// nothing and says so.
    fn step_history(&mut self, id: WidgetId, forward: bool) -> bool {
        let value = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.text_value())
            .map(str::to_owned);
        let Some(value) = value else {
            return false;
        };
        let edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let current = EditSnapshot::new(value, edit);
        let target = if forward {
            self.runtime.redo_edit(id, current)
        } else {
            self.runtime.undo_edit(id, current)
        };
        let Some(target) = target else {
            return false;
        };
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.replace_value(target.value.clone()));
        // The caret goes back with the text — an undo that restores the value and leaves
        // the caret at the end has done half the job.
        self.runtime.edits.insert(id, target.edit);
        self.reveal_caret(id, target.edit.cursor);
        if let Some(message) = message {
            self.dispatch_edit(message);
        }
        // An undo is not a keystroke: what is typed after it starts a run of its own,
        // however quickly it follows.
        self.last_edit_at = None;
        true
    }

    /// Brings the focused widget **into view**, gliding every scroll region around it.
    ///
    /// Called wherever the keyboard moves the focus, and nowhere else. A click already
    /// landed on something the eye could see, and a focus restored because an overlay
    /// closed should put the page back where the reader left it, not chase whatever the
    /// restoration happened to pick.
    ///
    /// The glide is a **target**, not a jump: the same easing a scrollbar drag or a page
    /// request uses, so Tab through a form reads as one movement rather than a series of
    /// cuts. Anything already moving the offset is let go of — the keyboard has just
    /// overruled it.
    fn reveal_focus(&mut self) {
        let Some(id) = self.runtime.input.focused else {
            return;
        };
        let Some(moves) = self.ui.as_ref().map(|ui| ui.reveal(id, &self.runtime)) else {
            return;
        };
        for (area, offset) in moves {
            self.runtime.scroll_ballistic.remove(&area);
            self.runtime.scroll_velocity.remove(&area);
            self.runtime.scroll_target.insert(area, offset);
        }
    }

    /// Makes a multi-line field's retained scroll **follow the caret**: adjusts
    /// `runtime.scroll[id]` just enough to keep the caret visible, the way an editor
    /// re-centres as you type. A no-op for a field that does not scroll.
    fn reveal_caret(&mut self, id: WidgetId, cursor: usize) {
        let Some(vp) = self.ui.as_ref().and_then(|ui| ui.scrollable_viewport(id)) else {
            return;
        };
        let metrics = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.text_metrics(vp.width, cursor));
        let Some((content_h, visible_h, caret_top, caret_h)) = metrics else {
            return;
        };
        let max_y = (content_h - visible_h).max(0.0);
        let cur = self.runtime.scroll.get(&id).map(|s| s.1).unwrap_or(0.0);
        // The scroll window in which the caret stays visible, from "the caret's bottom
        // is visible" to "the caret's top is visible". We bring the current scroll into it.
        let lo = (caret_top + caret_h - visible_h).max(0.0);
        let hi = caret_top;
        let target = cur.clamp(lo.min(hi), lo.max(hi)).clamp(0.0, max_y);
        self.runtime.scroll.insert(id, (0.0, target));
        self.runtime.scroll_target.insert(id, (0.0, target));
        self.runtime.scroll_velocity.remove(&id);
    }

    /// Copies field `id`'s selected text to the clipboard.
    fn copy_selection(&mut self, id: WidgetId) {
        let edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let text = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.selected_text(&edit));
        if let Some(text) = text {
            self.clipboard.set_text(text);
        }
    }

    /// Types what the clipboard answered into the field that asked for it, if the paste
    /// still lands there — see `clip::Pasted::lands`.
    fn land_paste(&mut self, pasted: clip::Pasted) {
        if pasted.lands(self.runtime.input.focused) {
            self.apply_key(pasted.into, Key::Text(pasted.text));
            self.request_redraw();
        }
    }
}

/// Which end of a value drag is being announced.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Start,
    End,
}

/// Where along `rect` the pointer sits, as a 0..=1 fraction.
///
/// One function, used by the start, every move and the end alike: three copies of this
/// arithmetic would agree on every slider until one of them did not.
fn drag_fraction(rect: frus_widgets::Rect, x: f32) -> f32 {
    if rect.width > 0.0 {
        ((x - rect.x) / rect.width).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Was the gesture that just ended **still a tap**?
///
/// Every drag here engages only past a threshold, so a press that never crossed it is
/// not a drag at all: it is a tap, and the release owes the widget underneath its
/// click. A drag that *did* move has already been answered — a fling, a drop, a
/// dismissal — and must not also click.
///
/// One list, because forgetting a variant is silent: the widget stays hittable, the
/// press still records it, and only the release quietly does nothing. That is how a
/// dismissible row came to swallow every tap on it (milestone 327).
fn gesture_was_a_tap(ended: Option<&Drag>) -> bool {
    matches!(
        ended,
        Some(
            Drag::Scroll { moved: false, .. }
                | Drag::Pan { moved: false, .. }
                | Drag::Reorder { moved: false, .. }
                | Drag::Item { moved: false, .. }
                | Drag::Dismiss { moved: false, .. }
                | Drag::Sheet { moved: false, .. }
        )
    )
}

/// How fast an area scrolls under a carried item, in pixels per second **per pixel** the
/// item hangs over the edge: the further out it is, the faster the list comes to meet it.
/// The reference's number, and the reference's law.
const AUTOSCROLL_VELOCITY: f32 = 50.0;

/// The overhang that speed is worked out from is capped here, so that carrying a row far
/// past the edge — or off the window entirely — settles at a fast but usable speed instead
/// of one nobody can aim with.
const AUTOSCROLL_MAX_OVERHANG: f32 = 20.0;

/// Places the scroll requests it can against `regions`, and hands back the ones whose
/// region that registry does not name, along with whether anything moved.
///
/// A request names its region by **key**, the way a focus request does: the application
/// wrapped it in `keyed(k, …)` and the tree is what turns that back into an identity.
/// The framework's own identities are hashes of a position in a tree — an application
/// cannot know one, and should not have to.
///
/// Nothing is *stored* here. The request is spent the moment its region is found, and
/// what it leaves behind is an offset in the runtime, indistinguishable from an offset a
/// finger left there. That is what makes it survive a rebuild: there is nothing to
/// survive.
fn apply_scroll_requests<Msg>(
    runtime: &mut Runtime,
    tree: &dyn Widget<Msg>,
    requests: Vec<(u64, ScrollTo)>,
    regions: &[Scrollable],
) -> (Vec<(u64, ScrollTo)>, bool) {
    let mut unplaced = Vec::new();
    let mut moved = false;
    for (key, to) in requests {
        let area = find_by_key(tree, key).and_then(|id| regions.iter().find(|a| a.id == id));
        match area {
            Some(area) => moved |= runtime.scroll_to(area, to),
            None => unplaced.push((key, to)),
        }
    }
    (unplaced, moved)
}

/// Places the sheet requests it can against `sheets`, and hands back the ones whose sheet
/// that registry does not name, along with whether anything moved — the terms of
/// `apply_scroll_requests`, above.
///
/// A request names the key the application wrapped the sheet in; the state belongs to the
/// sheet's panel under that key, which is what the tree is asked for.
fn apply_sheet_requests<Msg>(
    runtime: &mut Runtime,
    tree: &dyn Widget<Msg>,
    requests: Vec<(u64, SheetTo)>,
    sheets: &[frus_widgets::SheetArea],
) -> (Vec<(u64, SheetTo)>, bool) {
    let mut unplaced = Vec::new();
    let mut moved = false;
    for (key, to) in requests {
        let sheet = frus_widgets::find_sheet_by_key(tree, key)
            .and_then(|id| sheets.iter().find(|sheet| sheet.id == id));
        match sheet {
            Some(sheet) => moved |= runtime.sheet_to(sheet.id, &sheet.spec, &to),
            None => unplaced.push((key, to)),
        }
    }
    (unplaced, moved)
}

/// Moves a carried item's press by `shift` — how far its content moved on screen — so that
/// the ghost, drawn at the content's box plus the finger's travel since the press, stays
/// under the finger. A lifted item's box, recorded at the press, goes with it.
///
/// Anything else being dragged is not carried content, and is left alone.
fn follow_content(drag: &mut Drag, shift: (f32, f32)) {
    match drag {
        Drag::Reorder { start, carried, .. } => {
            start.x += shift.0;
            start.y += shift.1;
            if let Some((_, rect)) = carried {
                *rect = rect.translate(shift.0, shift.1);
            }
        }
        Drag::Item { source, start, .. } => {
            start.x += shift.0;
            start.y += shift.1;
            source.rect = source.rect.translate(shift.0, shift.1);
        }
        _ => {}
    }
}

/// One axis of the auto-scroll: how far the **content** has to move this frame so that an
/// item held past a viewport's edge brings the rest of the list into view.
///
/// Everything is in screen coordinates and along one axis: `item` and `view` are
/// `(start, end)` pairs, `room` is what is left to scroll that way (nought at the end of
/// the content). The answer is a movement of the content, which the area then turns into
/// an offset of its own — signs and reversed axes belong to `Scrollable::offset_delta`,
/// not here.
///
/// `None` means *not now*: the item is inside, or the edge it hangs over is the end of the
/// content, where there is nothing left to reveal and the list should stay still rather
/// than fight.
fn edge_autoscroll(item: (f32, f32), view: (f32, f32), room: (f32, f32), dt: f32) -> Option<f32> {
    let before = view.0 - item.0; // above the top edge
    let after = item.1 - view.1; // below the bottom edge
                                 // The leading edge is asked first, and both are never obeyed at once: an item taller
                                 // than the viewport hangs over both, and an item pulled two ways scrolls nowhere.
    let overhang = if before > 0.0 && room.0 > 0.0 {
        before.min(AUTOSCROLL_MAX_OVERHANG)
    } else if after > 0.0 && room.1 > 0.0 {
        -after.min(AUTOSCROLL_MAX_OVERHANG)
    } else {
        return None;
    };
    let travel = overhang * AUTOSCROLL_VELOCITY * dt;
    (travel != 0.0).then_some(travel)
}

/// Moves `current` toward `target` by one **exponential** spring step, with time
/// constant `tau` in seconds, over an interval `dt`: frame-rate independent and free
/// of overshoot. Used to smooth the abscissa during a reorder.
fn spring_toward(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    let k = 1.0 - (-dt / tau.max(1e-4)).exp();
    current + (target - current) * k
}

/// The focus history's maximum depth, for nested overlays' triggers.
const FOCUS_HISTORY_MAX: usize = 8;

/// Resolves focus after a (re)build, from the focusables **present** this frame. When
/// `current` has **vanished**, an overlay having closed, focus returns to the most
/// recent present **trigger** in `history`, which is popped until one is found. The
/// transition is recorded: the old focus (`prev`), if still present and different from
/// the new one, is pushed as a candidate trigger, bounded by [`FOCUS_HISTORY_MAX`].
/// Returns the resolved focus. A **pure** function apart from the mutated `history` and
/// `prev` — testable without a window.
fn resolve_focus(
    current: Option<WidgetId>,
    present: &std::collections::HashSet<WidgetId>,
    history: &mut Vec<WidgetId>,
    prev: &mut Option<WidgetId>,
) -> Option<WidgetId> {
    let mut cur = current;
    if let Some(c) = cur {
        if !present.contains(&c) {
            cur = None;
            while let Some(cand) = history.pop() {
                if present.contains(&cand) {
                    cur = Some(cand);
                    break;
                }
            }
        }
    }
    if *prev != cur {
        if let Some(old) = *prev {
            if present.contains(&old) && Some(old) != cur {
                history.push(old);
                if history.len() > FOCUS_HISTORY_MAX {
                    history.remove(0);
                }
            }
        }
        *prev = cur;
    }
    cur
}

/// Paints the dragged header's **lifted card**, the ghost, into `scene` — unclipped —
/// at `card`: a drop shadow, a **faithful face** from the header's already-translated
/// primitives, or a solid one as a fallback when `ghost` is empty, then a `primary`
/// border. A pure function, testable without a GPU. The neighbours' sliding happens
/// upstream, in `reflow_reorder_columns`.
fn draw_ghost_card(scene: &mut Scene, theme: &Theme, card: Rect, ghost: &[Primitive]) {
    use drag_preview::{BORDER_ALPHA, BORDER_WIDTH, SHADOW_ALPHA, SHADOW_BLUR, SHADOW_OFFSET_Y};
    scene.set_clip(Rect::UNBOUNDED);
    // The shadow colour comes from **the theme** and can be overridden — the same role
    // as `Button`'s shadow; only the geometry, offset, blur and opacity, is a local
    // constant.
    let shadow = theme.scheme.shadow.with_alpha(SHADOW_ALPHA);
    scene.shadow(
        card.translate(0.0, SHADOW_OFFSET_Y),
        shadow,
        theme.radius,
        SHADOW_BLUR,
    );
    let border = theme.primary.fade(BORDER_ALPHA);
    if ghost.is_empty() {
        scene.draw_rect(card, theme.surface, theme.radius, BORDER_WIDTH, border);
    } else {
        scene.draw_rect(card, theme.surface, theme.radius, 0.0, Color::TRANSPARENT);
        for primitive in ghost {
            scene.push_primitive(primitive.clone());
        }
        scene.draw_rect(card, Color::TRANSPARENT, theme.radius, BORDER_WIDTH, border);
    }
}

/// How a reorderable moves while it is carried: the two gestures the shell knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReorderMotion {
    /// A table's column: a continuous slide along x following the pointer, dropped into
    /// the place of the column under it.
    Columns,
    /// A list's row or a board's card: inserted before or after a slot along the axis,
    /// with an insertion line, a gap opening there, and the nearest slot past either end.
    Slots(ReorderAxis),
}

impl ReorderMotion {
    /// Which of the two a reorderable is, from its two answers: every vertical one inserts,
    /// and a horizontal one only when it says so — a list's row does, a table's column does
    /// not.
    fn of(axis: ReorderAxis, inserts: bool) -> Self {
        match axis {
            ReorderAxis::Vertical => Self::Slots(ReorderAxis::Vertical),
            ReorderAxis::Horizontal if inserts => Self::Slots(ReorderAxis::Horizontal),
            ReorderAxis::Horizontal => Self::Columns,
        }
    }
}

/// How `widget` moves while it is carried — [`ReorderMotion::of`] its two answers.
fn reorder_motion<Msg>(widget: &dyn Widget<Msg>) -> ReorderMotion {
    ReorderMotion::of(widget.reorder_axis(), widget.reorder_inserts())
}

/// Where the ghost is drawn relative to the box it was lifted from, given the finger's
/// `travel` since the press.
///
/// - A **column** follows the pointer along x, with a slight lift.
/// - A **card** follows it in both directions: a board carries it across columns.
/// - A **horizontal list's row** follows it along x only. The reference keeps a list's
///   proxy in its lane on either axis; a vertical list here shares the board's freedom
///   because the two are one path, and a horizontal list has no board to share it with.
fn ghost_offset(motion: ReorderMotion, travel: (f32, f32)) -> (f32, f32) {
    match motion {
        ReorderMotion::Columns => (travel.0, drag_preview::LIFT_Y),
        ReorderMotion::Slots(ReorderAxis::Vertical) => travel,
        ReorderMotion::Slots(ReorderAxis::Horizontal) => (travel.0, 0.0),
    }
}

/// The geometry of a reorder preview's **insertion line**: a thin band of thickness
/// `thickness`, centred on the target edge **where the insertion will happen**.
///
/// Along a **vertical** list: the **top** edge (inserting before, the upper half hovered)
/// or the **bottom** one (inserting after, `after = true`, the lower half hovered),
/// spanning the full width. Along a **horizontal** list the band stands on its end,
/// spanning the full height at the left or right edge — and *after* is the right edge
/// only when the list reads left to right: under `rtl` what follows a row is on its left.
/// A pure function, testable without a GPU.
fn drop_insertion_line(
    target: Rect,
    thickness: f32,
    after: bool,
    axis: ReorderAxis,
    rtl: bool,
) -> Rect {
    match axis {
        ReorderAxis::Vertical => {
            let edge = if after {
                target.y + target.height
            } else {
                target.y
            };
            Rect::new(target.x, edge - thickness * 0.5, target.width, thickness)
        }
        ReorderAxis::Horizontal => {
            let edge = if after != rtl {
                target.x + target.width
            } else {
                target.x
            };
            Rect::new(edge - thickness * 0.5, target.y, thickness, target.height)
        }
    }
}

/// Which axis a scroll gesture is **claimed by**, decided once when the finger passes the
/// threshold: `true` for vertical.
///
/// An area that can only go one way claims that one, whatever the finger did — a diagonal
/// flick down a list is a scroll down, not a refusal. An area that can go both ways takes
/// the direction the finger actually went in, ties going to vertical, which is the way a
/// page reads. The loser gets nothing for the rest of the gesture: this is the reference's
/// arena, where a vertical and a horizontal recogniser compete and only one wins.
fn claim_axis(dx: f32, dy: f32, can_x: bool, can_y: bool) -> bool {
    match (can_x, can_y) {
        (true, false) => false,
        (false, true) => true,
        _ => dy.abs() >= dx.abs(),
    }
}

/// Which of the scrollables under the finger a gesture is **for**: the innermost one that
/// can go the way the finger went (`down` = along the vertical), given `chain` innermost
/// first and `under` the area the press landed on.
///
/// A strip that only slides across, sitting in a page that only scrolls down, is not what a
/// drag downwards is about — the page behind it is, and the reference's arena gives it to
/// the page for exactly that reason. Where nothing in the stack can go that way, the area
/// under the finger keeps the gesture and goes its own way: a lone recogniser in an arena
/// still wins.
fn claim_area(
    chain: &[frus_widgets::Scrollable],
    under: WidgetId,
    down: bool,
) -> Option<frus_widgets::Scrollable> {
    chain
        .iter()
        .find(|a| {
            if down {
                // A refresh area listening above takes a pull downwards even with nothing
                // to scroll, which is the reference's one exception to the same rule.
                a.max_y > 0.0 || a.refresh.is_some()
            } else {
                a.max_x > 0.0
            }
        })
        .or_else(|| chain.iter().find(|a| a.id == under))
        .copied()
}

/// Gets the bytes at `url` and hands them to `deliver`, once.
///
/// Registered as the process's [`frus_widgets::ImageFetcher`] when the shell is built with
/// `net`. It spawns and forgets: nothing here waits for the answer, and the store the
/// callback writes into is what a later frame reads.
///
/// The two spawns are the two the rest of this file uses for every other effect — the
/// shared executor natively, the browser on the Web.
///
/// It carries a **deadline**, and that is not politeness. An image still in flight keeps
/// the interface redrawing — that is how the frame showing it ever happens — so a
/// request that never answers would leave the application repainting at full rate for
/// ever, on somebody's battery. A request without an answer has to become one with a
/// failure.
#[cfg(feature = "net")]
fn fetch_image_bytes(
    url: &str,
    deliver: Box<dyn FnOnce(Result<Vec<u8>, String>) + Send + 'static>,
) {
    /// Long enough for a photograph on a slow connection, short enough that a dead
    /// server does not pin a screen redrawing until the user closes the application.
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

    let url = url.to_string();
    let run = async move {
        let result = crate::net::Request::get(url)
            .timeout(DEADLINE)
            .send_bytes()
            .await;
        deliver(result.map_err(|e| e.to_string()));
    };
    #[cfg(not(web))]
    crate::runtime::spawn(run).detach();
    #[cfg(web)]
    wasm_bindgen_futures::spawn_local(run);
}

#[cfg(test)]
mod tests {
    use super::{
        build_view, claim_area, claim_axis, draw_ghost_card, drop_insertion_line, fling_velocity,
        gesture_was_a_tap, ghost_offset, install_ambient, reorder_motion, resolve_focus,
        spring_toward, Drag, Point, Rect, ReorderAxis, ReorderMotion, Scene, Theme,
        VelocityEstimate, PRECISE_SLOP, TOUCH_SLOP,
    };
    use super::{clipboard_command, ClipCommand, KeyCode, PhysicalKey, WinitKey};
    use super::{collect_ids, find_widget, MediaQuery};
    use frus_widgets::Locale;

    /// **A field with a clear button kept the keyboard only until the first letter**
    /// (milestone 510). Whether the focused widget takes typing was asked of a caret
    /// hit test at the corner of a field one pixel wide; a field whose clickable suffix
    /// covers that pixel — the × it shows once it holds text — answered no, and the
    /// keyboard was closed mid-word. Asked of the widget, the answer is the field's.
    #[test]
    fn a_field_with_a_clear_button_still_wants_the_keyboard() {
        use frus_widgets::{Container, Icons, TextField};
        let clearable = TextField::<()>::new("F")
            .suffix_icon(Icons::CLOSE)
            .on_suffix(());
        assert!(
            super::wants_keyboard(&clearable),
            "the field with its clear button"
        );
        assert!(
            !super::wants_keyboard(&TextField::<()>::new("F").enabled(false)),
            "a disabled field takes no typing"
        );
        assert!(
            !super::wants_keyboard(&Container::<()>::new()),
            "and what is not a field wants no keyboard"
        );
    }

    fn character(c: &str) -> WinitKey {
        WinitKey::Character(c.into())
    }

    /// Ctrl+C/X/V are the clipboard, in either case — Caps Lock or Shift is still a
    /// copy — and the same letters without Ctrl are only letters.
    #[test]
    fn ctrl_c_x_v_are_the_clipboard_and_the_letters_alone_are_not() {
        let any = PhysicalKey::Code(KeyCode::KeyQ);
        for (letter, command) in [
            ("c", ClipCommand::Copy),
            ("X", ClipCommand::Cut),
            ("v", ClipCommand::Paste),
        ] {
            assert_eq!(
                clipboard_command(&character(letter), any, true),
                Some(command)
            );
            assert_eq!(
                clipboard_command(&character(letter), any, false),
                None,
                "{letter}"
            );
        }
        assert_eq!(clipboard_command(&character("a"), any, true), None);
    }

    /// **Android's Copy, Cut and Paste keys arrive by physical code alone** — winit gives
    /// them no logical key — and they are the clipboard with no modifier held. A check
    /// of the logical key alone ignored them (milestone 509).
    #[test]
    fn a_keyboards_own_clipboard_keys_work_by_their_physical_code() {
        let unidentified = WinitKey::Unidentified(winit::keyboard::NativeKey::Unidentified);
        for (code, command) in [
            (KeyCode::Copy, ClipCommand::Copy),
            (KeyCode::Cut, ClipCommand::Cut),
            (KeyCode::Paste, ClipCommand::Paste),
        ] {
            assert_eq!(
                clipboard_command(&unidentified, PhysicalKey::Code(code), false),
                Some(command),
                "{code:?}"
            );
        }
        // And the named keys a desktop keyboard reports for the same thing.
        let named = WinitKey::Named(super::NamedKey::Paste);
        let any = PhysicalKey::Code(KeyCode::KeyQ);
        assert_eq!(
            clipboard_command(&named, any, false),
            Some(ClipCommand::Paste)
        );
    }

    /// **A paste lands only in the field that asked, while that field still has the
    /// focus** — on the Web the clipboard answers after the key press, and by then the
    /// reader may have moved to another field or to none. And a clipboard holding no
    /// text pastes nothing, rather than typing an empty string over the selection
    /// (milestone 526).
    #[test]
    fn a_paste_lands_in_the_field_that_asked_only_while_it_has_the_focus() {
        use super::clip::Pasted;
        let field = WidgetId::from_u64(1);
        let other = WidgetId::from_u64(2);
        let pasted = Pasted {
            into: field,
            text: "copied".into(),
        };
        assert!(pasted.lands(Some(field)));
        assert!(
            !pasted.lands(Some(other)),
            "the focus moved to another field"
        );
        assert!(!pasted.lands(None), "the focus left every field");
        let nothing = Pasted {
            into: field,
            text: String::new(),
        };
        assert!(!nothing.lands(Some(field)), "an empty clipboard");
    }

    /// **A paste answered later is queued for its field, and a frame is asked for.** Two
    /// pastes asked in two fields and answered in the other order come out in the order
    /// the answers came, each still naming the field that asked; each answer wakes the
    /// window once; and nothing is taken twice (milestone 526).
    #[test]
    fn a_paste_answered_later_waits_for_the_frame_that_takes_it() {
        use super::clip::{Answers, Pasted};
        use std::cell::Cell;
        use std::rc::Rc;
        let first = WidgetId::from_u64(1);
        let second = WidgetId::from_u64(2);
        let answers = Answers::default();
        let wakes = Rc::new(Cell::new(0));
        let wake = || {
            let wakes = Rc::clone(&wakes);
            move || wakes.set(wakes.get() + 1)
        };
        let to_first = answers.answer_for(first, wake());
        let to_second = answers.answer_for(second, wake());
        assert_eq!(answers.take(), None, "nothing is answered yet");
        assert_eq!(wakes.get(), 0, "asking is not answering");

        to_second("later".into());
        assert_eq!(wakes.get(), 1);
        to_first("sooner".into());
        assert_eq!(wakes.get(), 2);

        assert_eq!(
            answers.take(),
            Some(Pasted {
                into: second,
                text: "later".into()
            })
        );
        assert_eq!(
            answers.take(),
            Some(Pasted {
                into: first,
                text: "sooner".into()
            })
        );
        assert_eq!(answers.take(), None, "each answer is taken once");
    }

    /// An application whose whole interface lives **inside a deferred subtree** — which is
    /// what any application with an `AppBar` is, since a bar defers its own composition
    /// until the theme is known.
    ///
    /// It exists because nothing else in this repo can stand in for one: the shell's own
    /// types need a window and an event-loop proxy, so the only piece of the frame a test
    /// can hold is the part that turns a `view` into a tree. That part is where two
    /// milestones' worth of bugs were.
    struct Deferred;

    impl crate::Application for Deferred {
        type Message = ();

        fn update(&mut self, _message: ()) -> crate::Command<()> {
            crate::Command::none()
        }

        fn view(&self, _theme: &Theme) -> Box<dyn frus_widgets::Widget<()>> {
            Box::new(frus_widgets::ThemeBuilder::new(|_: &Theme| {
                frus_widgets::Flex::column()
                    .width(300.0)
                    .height(80.0)
                    .child(
                        frus_widgets::TextField::new("hi")
                            .width(200.0)
                            .on_input(|_| ()),
                    )
            }))
        }
    }

    /// The surface every frame installs, so a build outside one measures the way a build
    /// inside one does.
    fn surfaced<R>(f: impl FnOnce() -> R) -> R {
        MediaQuery::new(frus_widgets::Size::new(300.0, 80.0)).scope(f)
    }

    /// **A shell installs the reader's language** — the assertion that keeps milestone 454
    /// from being a resolution nobody ever runs.
    ///
    /// `locale::of()` answers `en` with nothing installed, exactly as
    /// `localizations::of()` answers English, and for the same reason: an application that
    /// says nothing must keep working. That default is also what would hide the wiring
    /// being missing, so this drives the shell's own reading of the trait rather than
    /// calling `resolve` directly — milestone 408's bug, and milestone 449's guard against
    /// it.
    #[test]
    fn a_shell_installs_the_reader_s_language() {
        /// An application with three languages, and a switch of its own it may or may not
        /// have been told to use.
        struct Speaks(Option<Locale>);

        impl crate::Application for Speaks {
            type Message = ();

            fn update(&mut self, _message: ()) -> crate::Command<()> {
                crate::Command::none()
            }

            fn view(&self, _theme: &Theme) -> Box<dyn frus_widgets::Widget<()>> {
                Box::new(frus_widgets::Flex::<()>::column())
            }

            fn supported_locales(&self) -> Vec<Locale> {
                vec![
                    Locale::new("en"),
                    Locale::new("fr"),
                    Locale::with_country("fr", "CA"),
                ]
            }

            fn locale(&self) -> Option<Locale> {
                self.0.clone()
            }
        }

        let mut runtime = frus_widgets::Runtime::default();

        // The device speaks Belgian French; the application has French, so it wins.
        install_ambient(
            &Speaks(None),
            &mut runtime,
            &[Locale::with_country("fr", "BE")],
        );
        assert_eq!(frus_widgets::locale::of(), Locale::new("fr"));

        // A platform that reported nothing is not a failure: the application's own first
        // choice stands.
        install_ambient(&Speaks(None), &mut runtime, &[]);
        assert_eq!(frus_widgets::locale::of(), Locale::new("en"));

        // And an application with a language menu of its own outranks the device — still
        // resolved, so asking for one it does not have gives the nearest thing it does.
        install_ambient(
            &Speaks(Some(Locale::with_country("fr", "FR"))),
            &mut runtime,
            &[Locale::new("en")],
        );
        assert_eq!(frus_widgets::locale::of(), Locale::new("fr"));

        // An application that says nothing at all is where it was: English.
        install_ambient(&Deferred, &mut runtime, &[Locale::new("fr")]);
        assert_eq!(
            frus_widgets::locale::of(),
            Locale::new("en"),
            "it supports only English, so French resolves to it"
        );
    }

    /// **A shell installs the application's words** — which is the assertion that keeps
    /// milestone 449 from being a table nobody ever reads.
    ///
    /// `localizations::of` answers English with nothing installed, so every widget test
    /// and every golden would pass whether or not the shell ever called `install`. The
    /// default is what makes the feature safe to add; it is also what would hide the
    /// wiring being missing. So this drives the shell's own reading of the trait.
    #[test]
    fn a_shell_installs_the_application_s_words() {
        struct Fr;

        impl frus_widgets::localizations::Localizations for Fr {
            fn back_button_label(&self) -> &str {
                "Retour"
            }

            fn first_day_of_week_index(&self) -> usize {
                1
            }
        }

        struct Speaks;

        impl crate::Application for Speaks {
            type Message = ();

            fn update(&mut self, _message: ()) -> crate::Command<()> {
                crate::Command::none()
            }

            fn view(&self, _theme: &Theme) -> Box<dyn frus_widgets::Widget<()>> {
                Box::new(frus_widgets::Flex::<()>::column())
            }

            fn localizations(&self) -> Option<std::rc::Rc<dyn frus_widgets::Localizations>> {
                Some(std::rc::Rc::new(Fr))
            }
        }

        let mut runtime = frus_widgets::Runtime::default();
        install_ambient(&Speaks, &mut runtime, &[]);
        assert_eq!(
            frus_widgets::localizations::of().back_button_label(),
            "Retour",
            "the shell read the application's table and installed it"
        );
        assert_eq!(
            frus_widgets::localizations::of().first_day_of_week_index(),
            1
        );

        // And an application that says nothing leaves whatever is in force alone rather
        // than forcing English back on top of a table installed elsewhere.
        install_ambient(&Deferred, &mut runtime, &[]);
        assert_eq!(
            frus_widgets::localizations::of().back_button_label(),
            "Retour",
            "saying nothing is not the same as saying English"
        );
    }

    /// A tree the shell hands to a traversal is **ready to be traversed**.
    ///
    /// Milestones 415 and 416 were the same bug found twice, in the two places this shell
    /// turns a `view` into a tree. A deferred subtree has no children until something has
    /// built it, so an unprepared tree reports **one** identity — its root — and every
    /// widget in the application is invisible to `collect_ids`, to `find_widget`, and to
    /// everything downstream of them: the mount and leave fades, the focus, the caret.
    ///
    /// The failure was silence in both places, which is why it went unnoticed twice. This
    /// is the assertion that would have caught it, and it needs no window.
    #[test]
    fn a_view_is_built_ready_to_be_read() {
        let theme = Theme::default();
        let prepared =
            surfaced(|| build_view(&Deferred, &theme, &frus_widgets::Runtime::default()));
        let ids = collect_ids(prepared.as_ref());
        assert!(
            ids.len() > 1,
            "an application inside a deferred subtree has identities to mount: {ids:?}"
        );
        // And every one of them is reachable, which is what the burst path needs.
        for id in &ids {
            assert!(
                find_widget(prepared.as_ref(), *id).is_some(),
                "{id:?} was counted but cannot be found"
            );
        }
    }

    /// The same view, unprepared, is the state both call sites used to read.
    #[test]
    #[should_panic(expected = "before it was built")]
    fn an_unprepared_view_is_the_state_that_broke() {
        let theme = Theme::default();
        let raw = surfaced(|| crate::Application::view(&Deferred, &theme));
        let _ = collect_ids(raw.as_ref());
    }

    /// A press on a **dismissible** row that never moved is a tap, and the widget under
    /// the finger owes it a click (reported on a device: a task row's avatar opened
    /// nothing, while a control a few hundred pixels away in the same card worked).
    ///
    /// The row registers a swipe on the press, in case the finger is about to slide
    /// sideways. When it does not, the swipe has to stand down. Every other gesture
    /// here already did; this one was left out of the list.
    #[test]
    fn a_swipe_that_never_started_is_still_a_tap() {
        let item = frus_widgets::Dismissable {
            id: WidgetId::from_u64(1),
            rect: Rect::new(0.0, 0.0, 300.0, 62.0),
            spec: frus_widgets::DismissSpec::default(),
        };
        let dismiss = |moved| Drag::Dismiss {
            item,
            last: Point::new(0.0, 0.0),
            moved,
        };
        assert!(
            gesture_was_a_tap(Some(&dismiss(false))),
            "a swipe under the threshold leaves the click alone"
        );
        assert!(
            !gesture_was_a_tap(Some(&dismiss(true))),
            "a swipe that ran has already been answered and must not also click"
        );
        // A scroll, the variant that was already right — and the one that masked this
        // bug, since a list long enough to scroll claims the press before the swipe can.
        let scroll = Drag::Scroll {
            id: WidgetId::from_u64(2),
            last: Point::new(0.0, 0.0),
            moved: false,
            carried: (0.0, 0.0),
            dismiss: None,
            axis: None,
        };
        assert!(gesture_was_a_tap(Some(&scroll)));
        // A press on a sheet's panel likewise: a button in the sheet still clicks, and a
        // sheet that was dragged does not also click what it was dragged by (515).
        let sheet = |moved| Drag::Sheet {
            id: WidgetId::from_u64(4),
            last: Point::new(0.0, 0.0),
            moved,
            available: 800.0,
        };
        assert!(gesture_was_a_tap(Some(&sheet(false))));
        assert!(!gesture_was_a_tap(Some(&sheet(true))));
        // And a gesture that captures on the press, rather than at a threshold, never
        // becomes a tap however still the finger was.
        let select = Drag::TextSelect {
            id: WidgetId::from_u64(3),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        };
        assert!(!gesture_was_a_tap(Some(&select)));
        assert!(!gesture_was_a_tap(None), "no gesture, no verdict to give");
    }

    /// A page that scrolls down must not drift sideways while it does it (reported on a
    /// device, 2026-08-16). No finger travels in a straight line, so an area that can go
    /// both ways has to pick one and keep it.
    #[test]
    fn a_scroll_gesture_is_claimed_by_one_axis() {
        // Both axes available: the direction the finger went in wins, and a little wobble
        // across it changes nothing.
        assert!(claim_axis(6.0, 40.0, true, true), "down the page");
        assert!(!claim_axis(40.0, 6.0, true, true), "across it");
        // A tie reads as vertical, which is the way a page reads.
        assert!(claim_axis(10.0, 10.0, true, true));
        // One axis only: it takes the gesture whatever the finger did, or a diagonal flick
        // down a list would move nothing at all.
        assert!(claim_axis(40.0, 6.0, false, true), "a list only goes down");
        assert!(
            !claim_axis(6.0, 40.0, true, false),
            "a strip only goes across"
        );
    }

    /// An area under the finger, `max_x` by `max_y` of content out of sight.
    fn area(id: u64, max_x: f32, max_y: f32) -> frus_widgets::Scrollable {
        frus_widgets::Scrollable {
            id: WidgetId::from_u64(id),
            viewport: Rect::new(0.0, 0.0, 100.0, 100.0),
            max_x,
            max_y,
            physics: None,
            refresh: None,
            page: None,
            reverse_x: false,
            reverse_y: false,
            host: None,
            keep_visible: None,
        }
    }

    /// A strip that only slides sideways, sitting in a page that only scrolls down, must
    /// not swallow a drag downwards: the page behind it is what that drag is about
    /// (reported on a device, 2026-08-16).
    #[test]
    fn the_gesture_goes_to_the_area_that_can_take_it() {
        let strip = area(1, 400.0, 0.0);
        let page = area(2, 0.0, 900.0);
        let chain = [strip, page]; // innermost first, as the finger meets them
        assert_eq!(
            claim_area(&chain, strip.id, true).map(|a| a.id),
            Some(page.id),
            "down the page, over the strip"
        );
        assert_eq!(
            claim_area(&chain, strip.id, false).map(|a| a.id),
            Some(strip.id),
            "across the strip"
        );
        // Nothing behind it: the strip keeps the gesture and goes its own way, which is
        // what a lone recogniser in an arena does.
        assert_eq!(
            claim_area(&[strip], strip.id, true).map(|a| a.id),
            Some(strip.id)
        );
        // The innermost that can take it wins, not the outermost.
        let inner = area(3, 0.0, 200.0);
        assert_eq!(
            claim_area(&[inner, strip, page], inner.id, true).map(|a| a.id),
            Some(inner.id)
        );
    }

    /// An end-of-content glow on a page with nothing to scroll says something about the
    /// content that is not true (reported on a device, 2026-08-16). The reference takes the
    /// drag recognisers away outright when everything fits.
    #[test]
    fn an_area_whose_content_fits_takes_no_finger() {
        let fits = area(1, 0.0, 0.0);
        assert!(!fits.accepts_user_offset((0.0, 0.0)));
        // Unless it is already displaced — content that shrank under a scrolled offset has
        // to be draggable back into place.
        assert!(fits.accepts_user_offset((0.0, 12.0)));
        // Or a refresh area is listening above it: a list of two items still pulls down.
        let mut pullable = fits;
        pullable.refresh = Some(WidgetId::from_u64(9));
        assert!(pullable.accepts_user_offset((0.0, 0.0)));
        // A single pixel out of sight is content to reveal, and enough.
        assert!(area(1, 0.0, 1.0).accepts_user_offset((0.0, 0.0)));
        assert!(area(1, 1.0, 0.0).accepts_user_offset((0.0, 0.0)));
    }
    use frus_widgets::WidgetId;
    use std::collections::HashSet;

    /// An estimate reading `velocity` px/s after travelling `offset` px.
    fn released(velocity: (f32, f32), offset: (f32, f32)) -> VelocityEstimate {
        VelocityEstimate {
            velocity: frus_widgets::Velocity::new(velocity.0, velocity.1),
            confidence: 1.0,
            duration: 0.05,
            offset,
        }
    }

    #[test]
    fn a_fast_twitch_that_went_nowhere_is_not_a_fling() {
        // 2000 px/s, but the finger covered 3 px: a wobble on lift-off, not a throw.
        let (x, y) = fling_velocity(released((0.0, 2000.0), (0.0, 3.0)), TOUCH_SLOP);
        assert_eq!((x, y), (0.0, 0.0));
        // The same speed over a real distance is.
        let (_, y) = fling_velocity(released((0.0, 2000.0), (0.0, 60.0)), TOUCH_SLOP);
        assert_eq!(y, 2000.0);
    }

    #[test]
    fn the_fling_gate_is_per_axis() {
        // A vertical swipe with the sideways wobble a thumb always adds: the wobble
        // must not fling the content horizontally.
        let (x, y) = fling_velocity(released((300.0, 1500.0), (5.0, 120.0)), TOUCH_SLOP);
        assert_eq!(x, 0.0, "the wobble is not a horizontal fling");
        assert_eq!(y, 1500.0, "the swipe still flings vertically");
    }

    #[test]
    fn a_precise_pointer_needs_almost_no_travel() {
        // The same 3 px that a finger's slop rejects is a deliberate mouse drag.
        let estimate = released((0.0, 900.0), (0.0, 3.0));
        assert_eq!(fling_velocity(estimate, TOUCH_SLOP).1, 0.0);
        assert_eq!(fling_velocity(estimate, PRECISE_SLOP).1, 900.0);
    }

    #[test]
    fn insertion_line_sits_on_the_target_top_edge() {
        // The upper half hovered means inserting **before**: a band of thickness 4
        // centred on the top edge (y=100) of a target 200 wide.
        let line = drop_insertion_line(
            Rect::new(20.0, 100.0, 200.0, 44.0),
            4.0,
            false,
            ReorderAxis::Vertical,
            false,
        );
        assert_eq!(line.x, 20.0, "aligned to the target's left");
        assert_eq!(line.width, 200.0, "the target's full width");
        assert_eq!(line.y, 98.0, "centred on the top edge (100 - 4/2)");
        assert_eq!(line.height, 4.0);
    }

    #[test]
    fn insertion_line_sits_on_the_target_bottom_edge_when_inserting_after() {
        // The lower half hovered means inserting **after**: the band slides to the
        // **bottom** edge (y = 100 + 44 = 144, centred → 142). Same width, same thickness.
        let line = drop_insertion_line(
            Rect::new(20.0, 100.0, 200.0, 44.0),
            4.0,
            true,
            ReorderAxis::Vertical,
            false,
        );
        assert_eq!(line.x, 20.0, "still aligned to the left");
        assert_eq!(line.width, 200.0, "the target's full width");
        assert_eq!(line.y, 142.0, "centred on the bottom edge (144 - 4/2)");
        assert_eq!(line.height, 4.0);
        // A vertical list reads down whichever way its text runs.
        let rtl = drop_insertion_line(
            Rect::new(20.0, 100.0, 200.0, 44.0),
            4.0,
            true,
            ReorderAxis::Vertical,
            true,
        );
        assert_eq!(rtl, line, "right to left changes nothing down a list");
    }

    /// **A horizontal list's line stands on its end**, on the edge the row goes to: the
    /// right one after a row that reads left to right, the left one after a row that reads
    /// right to left — where what follows it is.
    #[test]
    fn a_horizontal_insertion_line_stands_on_the_edge_the_row_goes_to() {
        let chip = Rect::new(100.0, 20.0, 80.0, 48.0);
        let line = |after, rtl| drop_insertion_line(chip, 4.0, after, ReorderAxis::Horizontal, rtl);
        let before = line(false, false);
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (98.0, 20.0, 4.0, 48.0),
            "before a row: its left edge, the full height, 4 wide"
        );
        assert_eq!(
            line(true, false).x,
            178.0,
            "after it: its right edge (180 - 4/2)"
        );
        // Right to left, the two edges trade places.
        assert_eq!(
            line(true, true).x,
            98.0,
            "after, right to left: the left edge"
        );
        assert_eq!(
            line(false, true).x,
            178.0,
            "before, right to left: the right edge"
        );
    }

    /// **Which gesture a reorderable gets.** A column slides; a row or a card is inserted
    /// between slots — and a horizontal reorderable is a row only when it says so, which is
    /// what keeps a table's columns exactly as they were.
    #[test]
    fn a_horizontal_reorderable_inserts_only_when_it_says_so() {
        assert_eq!(
            ReorderMotion::of(ReorderAxis::Horizontal, false),
            ReorderMotion::Columns,
            "a table's column"
        );
        assert_eq!(
            ReorderMotion::of(ReorderAxis::Horizontal, true),
            ReorderMotion::Slots(ReorderAxis::Horizontal),
            "a horizontal list's row"
        );
        // Every vertical reorderable inserts, whatever it answers.
        for inserts in [false, true] {
            assert_eq!(
                ReorderMotion::of(ReorderAxis::Vertical, inserts),
                ReorderMotion::Slots(ReorderAxis::Vertical)
            );
        }
        // A widget that answers nothing keeps the trait's defaults, which are the table's.
        assert_eq!(
            reorder_motion::<()>(&frus_widgets::Container::new()),
            ReorderMotion::Columns
        );
        // And the real rows say which they are, down and across.
        for (axis, expected) in [
            (
                ReorderAxis::Vertical,
                ReorderMotion::Slots(ReorderAxis::Vertical),
            ),
            (
                ReorderAxis::Horizontal,
                ReorderMotion::Slots(ReorderAxis::Horizontal),
            ),
        ] {
            let list = frus_widgets::ReorderableList::new(|_, _| ())
                .axis(axis)
                .row(frus_widgets::Container::new().width(60.0));
            let row = frus_widgets::Widget::children(&list)[0].as_ref();
            assert_eq!(reorder_motion(row), expected, "{axis:?}");
        }
    }

    /// **The ghost stays in its lane.** A column rises and follows x; a card follows the
    /// finger anywhere; a horizontal list's row follows x and nothing else — so a finger
    /// that wanders below the strip does not drag the row off it.
    #[test]
    fn a_horizontal_rows_ghost_follows_the_finger_along_x_only() {
        let travel = (120.0, 35.0);
        assert_eq!(
            ghost_offset(ReorderMotion::Columns, travel),
            (120.0, super::drag_preview::LIFT_Y)
        );
        assert_eq!(
            ghost_offset(ReorderMotion::Slots(ReorderAxis::Vertical), travel),
            (120.0, 35.0)
        );
        assert_eq!(
            ghost_offset(ReorderMotion::Slots(ReorderAxis::Horizontal), travel),
            (120.0, 0.0)
        );
    }

    #[test]
    fn focus_returns_to_trigger_when_overlay_closes() {
        let anchor = WidgetId::from_u64(1);
        let item = WidgetId::from_u64(2);
        let (mut history, mut prev) = (Vec::new(), None);

        // Frame 1: focus on the anchor, which is present.
        let f = resolve_focus(
            Some(anchor),
            &HashSet::from([anchor]),
            &mut history,
            &mut prev,
        );
        assert_eq!(f, Some(anchor));
        assert!(history.is_empty());

        // Frame 2: the menu is open and the item focused; the anchor is pushed as trigger.
        let present = HashSet::from([anchor, item]);
        let f = resolve_focus(Some(item), &present, &mut history, &mut prev);
        assert_eq!(f, Some(item));
        assert_eq!(history, vec![anchor]);

        // Frame 3: the menu closed and the item vanished → back to the trigger, history spent.
        let f = resolve_focus(
            Some(item),
            &HashSet::from([anchor]),
            &mut history,
            &mut prev,
        );
        assert_eq!(f, Some(anchor), "focus returns to the trigger");
        assert!(history.is_empty());
    }

    #[test]
    fn focus_falls_to_none_when_no_trigger_remains() {
        let a = WidgetId::from_u64(1);
        let (mut history, mut prev) = (Vec::new(), Some(a));
        // `a` vanishes and the history is empty → no focus at all.
        let f = resolve_focus(Some(a), &HashSet::new(), &mut history, &mut prev);
        assert_eq!(f, None);
        assert_eq!(prev, None);
    }

    #[test]
    fn spring_approaches_target_monotonically_and_settles() {
        // From 0 toward 100: a monotonic approach with no overshoot, all but reached
        // after several 16 ms steps.
        let mut x = 0.0;
        let mut prev = -1.0;
        for _ in 0..30 {
            x = spring_toward(x, 100.0, 0.016, 0.07);
            assert!(x > prev && x <= 100.0, "monotonic and bounded: {x}");
            prev = x;
        }
        assert!(x > 99.0, "all but reached after ~0.5 s: {x}");
    }

    #[test]
    fn ghost_card_shape() {
        let theme = Theme::default();
        let card = Rect::new(140.0, 0.0, 80.0, 34.0);
        // An empty ghost falls back to a shadow plus a solid bordered card: 2 primitives.
        let mut scene = Scene::new();
        draw_ghost_card(&mut scene, &theme, card, &[]);
        assert_eq!(scene.primitives().len(), 2, "shadow plus solid card");
    }
}

/// The half of a scroll request that belongs to the shell: turning the **name** an
/// application wrote into the region a frame actually has.
#[cfg(test)]
mod sheet_request_tests {
    use super::{apply_sheet_requests, Runtime, Widget};
    use crate::command::Command;
    use frus_widgets::{
        build_ui, keyed, Container, DraggableScrollableSheet, SheetTo, Size, Theme,
    };

    /// A page with one named sheet on it — the key on a wrapper the application wrote, the
    /// state on a panel it never sees.
    fn view() -> Box<dyn Widget<()>> {
        Box::new(
            Container::<()>::new()
                .width(400.0)
                .height(800.0)
                .child(keyed(
                    "places",
                    DraggableScrollableSheet::<()>::new(Container::new()),
                )),
        )
    }

    /// **A request reaches the sheet the view named**, and nothing else: another key, or a
    /// frame without the sheet, hands it back unplaced and moves nothing.
    #[test]
    fn a_request_reaches_the_sheet_the_view_named() {
        let tree = view();
        let mut runtime = Runtime::default();
        let sheets = build_ui(
            tree.as_ref(),
            Size::new(400.0, 800.0),
            &runtime,
            &Theme::default(),
        )
        .sheets()
        .to_vec();
        let sheet = sheets.first().expect("the page has its sheet").clone();

        let requests = Command::<()>::sheet("places", SheetTo::size(0.9))
            .into_parts()
            .sheets;
        let (unplaced, moved) =
            apply_sheet_requests(&mut runtime, tree.as_ref(), requests, &sheets);
        assert!(unplaced.is_empty());
        assert!(moved);
        assert_eq!(runtime.sheet_size(sheet.id, &sheet.spec), 0.9);

        let elsewhere = Command::<()>::sheet("elsewhere", SheetTo::size(0.3))
            .into_parts()
            .sheets;
        let (unplaced, moved) =
            apply_sheet_requests(&mut runtime, tree.as_ref(), elsewhere, &sheets);
        assert_eq!(unplaced.len(), 1);
        assert!(!moved);
        let not_yet = Command::<()>::sheet("places", SheetTo::size(0.3))
            .into_parts()
            .sheets;
        let (unplaced, _) = apply_sheet_requests(&mut runtime, tree.as_ref(), not_yet, &[]);
        assert_eq!(unplaced.len(), 1, "kept for the frame that has the sheet");
        assert_eq!(
            runtime.sheet_size(sheet.id, &sheet.spec),
            0.9,
            "nothing moved"
        );
    }
}

#[cfg(test)]
mod scroll_request_tests {
    use super::{apply_scroll_requests, Rect, Runtime, Scrollable, Widget, WidgetId};
    use crate::command::Command;
    use frus_widgets::{keyed, Container, ScrollTo, SingleChildScrollView};

    /// A view with one named scroll region in it, and nothing else of interest.
    fn view() -> Box<dyn Widget<()>> {
        Box::new(Container::<()>::new().child(keyed(
            "log",
            SingleChildScrollView::<()>::new().height(200.0),
        )))
    }

    /// The region as the frame would register it: a 200-tall window over 1 000 of content.
    fn region(id: WidgetId) -> Scrollable {
        Scrollable {
            id,
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 200.0,
            },
            max_x: 0.0,
            max_y: 1000.0,
            physics: None,
            refresh: None,
            page: None,
            reverse_x: false,
            reverse_y: false,
            keep_visible: None,
            host: None,
        }
    }

    /// The key a request carries and the key the view declared are the **same hash** —
    /// which is the whole of the identity design, and the one thing that cannot be
    /// checked on either side alone.
    #[test]
    fn a_request_reaches_the_region_the_view_named() {
        let tree = view();
        let requests = Command::<()>::scroll("log", ScrollTo::end().instant())
            .into_parts()
            .scrolls;
        let id = frus_widgets::find_by_key(tree.as_ref(), requests[0].0)
            .expect("the view named it, so the tree knows it");
        let mut runtime = Runtime::default();
        let (unplaced, moved) =
            apply_scroll_requests(&mut runtime, tree.as_ref(), requests, &[region(id)]);
        assert!(unplaced.is_empty());
        assert!(moved);
        assert_eq!(runtime.scroll.get(&id), Some(&(0.0, 1000.0)));
    }

    /// A region that has only just appeared is not in the registry the request is first
    /// tried against — that one was built last frame — so it is handed back rather than
    /// thrown away. Coming back to a screen and being put where you left off depends on
    /// exactly this.
    #[test]
    fn a_region_this_registry_has_not_got_is_handed_back() {
        let tree = view();
        let requests = Command::<()>::scroll("log", ScrollTo::y(120.0))
            .into_parts()
            .scrolls;
        let mut runtime = Runtime::default();
        let (unplaced, moved) = apply_scroll_requests(&mut runtime, tree.as_ref(), requests, &[]);
        assert_eq!(unplaced.len(), 1, "kept for the second attempt");
        assert!(!moved);
        assert!(runtime.scroll_target.is_empty());
    }

    /// A key naming nothing at all leaves no trace: the caller drops what comes back,
    /// so a typo costs one frame's lookup and never accumulates.
    #[test]
    fn a_name_the_view_does_not_use_moves_nothing() {
        let tree = view();
        let requests = Command::<()>::scroll("ledger", ScrollTo::start())
            .into_parts()
            .scrolls;
        let mut runtime = Runtime::default();
        let id = frus_widgets::find_by_key(
            tree.as_ref(),
            Command::<()>::scroll("log", ScrollTo::start())
                .into_parts()
                .scrolls[0]
                .0,
        )
        .expect("the real one is there");
        let (unplaced, moved) =
            apply_scroll_requests(&mut runtime, tree.as_ref(), requests, &[region(id)]);
        assert_eq!(unplaced.len(), 1);
        assert!(!moved);
        assert!(runtime.scroll.is_empty() && runtime.scroll_target.is_empty());
    }

    /// Two requests in one batch, both placed, and the batch is what `Command::batch`
    /// produces — a screen that restores both axes writes two and means two.
    #[test]
    fn a_batch_places_every_request_it_carries() {
        let tree = view();
        let requests = Command::<()>::batch([
            Command::scroll("log", ScrollTo::y(400.0).instant()),
            Command::scroll("log", ScrollTo::y(600.0).instant()),
        ])
        .into_parts()
        .scrolls;
        assert_eq!(requests.len(), 2);
        let id = frus_widgets::find_by_key(tree.as_ref(), requests[0].0).expect("named");
        let mut runtime = Runtime::default();
        let (unplaced, moved) =
            apply_scroll_requests(&mut runtime, tree.as_ref(), requests, &[region(id)]);
        assert!(unplaced.is_empty() && moved);
        assert_eq!(
            runtime.scroll.get(&id),
            Some(&(0.0, 600.0)),
            "the last request is the current statement"
        );
    }
}

#[cfg(test)]
mod autoscroll_tests {
    use super::{
        edge_autoscroll, follow_content, Drag, Point, Rect, WidgetId, AUTOSCROLL_MAX_OVERHANG,
        AUTOSCROLL_VELOCITY,
    };

    /// A viewport from 100 to 500, with room to scroll either way.
    const VIEW: (f32, f32) = (100.0, 500.0);
    const ROOM: (f32, f32) = (1000.0, 1000.0);
    /// A sixtieth of a second, which is the frame this is asked on.
    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn a_row_inside_the_viewport_scrolls_nothing() {
        assert_eq!(edge_autoscroll((200.0, 260.0), VIEW, ROOM, DT), None);
        // Touching the edge exactly is still inside: the row has not asked for anything.
        assert_eq!(edge_autoscroll((440.0, 500.0), VIEW, ROOM, DT), None);
    }

    /// The law: pixels per second per pixel of overhang, so ten pixels out is ten times
    /// the speed of one pixel out — the row is not dragged along by a fixed nudge.
    #[test]
    fn the_speed_follows_how_far_out_the_row_is() {
        let near = edge_autoscroll((460.0, 501.0), VIEW, ROOM, DT).expect("one pixel out");
        let far = edge_autoscroll((460.0, 510.0), VIEW, ROOM, DT).expect("ten pixels out");
        assert!(
            near < 0.0,
            "the content moves up to reveal what is below: {near}"
        );
        assert!(
            (far / near - 10.0).abs() < 0.01,
            "ten times as far, ten times as fast: {near} vs {far}"
        );
        assert!(
            (near + 1.0 * AUTOSCROLL_VELOCITY * DT).abs() < 1e-4,
            "one pixel out, at the stated velocity: {near}"
        );
    }

    /// Carrying a row right off the window is not a request to scroll a thousand times
    /// faster; past the cap the speed stops growing.
    #[test]
    fn the_overhang_is_capped() {
        let far = edge_autoscroll((460.0, 520.0), VIEW, ROOM, DT).expect("twenty out");
        let absurd = edge_autoscroll((460.0, 5000.0), VIEW, ROOM, DT).expect("off the window");
        assert!((far - absurd).abs() < 1e-4, "{far} vs {absurd}");
        assert!((absurd + AUTOSCROLL_MAX_OVERHANG * AUTOSCROLL_VELOCITY * DT).abs() < 1e-4);
    }

    /// Above the top the content moves the other way, and the sign is the whole of the
    /// difference between the two edges.
    #[test]
    fn above_the_top_the_content_comes_down() {
        let up = edge_autoscroll((90.0, 150.0), VIEW, ROOM, DT).expect("ten above");
        assert!(up > 0.0, "{up}");
    }

    /// **What is carried stays under the finger while its list scrolls.**
    ///
    /// Seen on a phone: a row carried to the bottom edge started the list scrolling, and
    /// the ghost rode up with the content — it is drawn at the row's box, and the box had
    /// moved — until it no longer hung over the edge and the scroll stopped by itself,
    /// 95 px in. The press moves with the content instead.
    #[test]
    fn a_carried_item_follows_its_content() {
        // The list scrolled on by 95 px: its content went 95 px up the screen.
        let shift = (0.0, -95.0);
        let pressed = Point::new(900.0, 1330.0);
        let row_then = Rect::new(105.0, 1250.0, 700.0, 160.0);
        let row_now = row_then.translate(0.0, -95.0);
        let finger = Point::new(900.0, 2030.0);

        let mut reorder = Drag::Reorder {
            id: WidgetId::from_u64(1),
            from: 0,
            start: pressed,
            moved: true,
            carried: Some((WidgetId::from_u64(1), row_then)),
        };
        follow_content(&mut reorder, shift);
        let Drag::Reorder { start, carried, .. } = reorder else {
            unreachable!("still a reorder");
        };
        // The box it was carried from went with the content, as the row did — and the frame's
        // own box is not asked again, since the frame clips a row carried to an edge and loses
        // one scrolled out of sight (milestone 527).
        let (_, carried) = carried.expect("the box it was carried from");
        assert_eq!(carried, row_now, "the carried box went with the content");
        // Drawn the way the shell draws it: the carried box, plus the travel.
        let ghost = carried.translate(finger.x - start.x, finger.y - start.y);
        assert_eq!(
            ghost,
            row_then.translate(finger.x - pressed.x, finger.y - pressed.y),
            "the ghost is where it would be had nothing scrolled"
        );

        let mut item = Drag::Item {
            source: frus_widgets::DragSource {
                id: WidgetId::from_u64(1),
                rect: row_then,
                payload: 7,
            },
            start: pressed,
            moved: true,
            over: None,
        };
        follow_content(&mut item, shift);
        let Drag::Item { source, start, .. } = item else {
            unreachable!("still an item");
        };
        assert_eq!(
            source.rect, row_now,
            "the box taken at the press went with the content"
        );
        assert_eq!(start, Point::new(900.0, 1235.0), "and so did the press");
    }

    /// At the end of the content there is nothing left to reveal, so the list stays where
    /// it is instead of straining against its own end.
    #[test]
    fn an_exhausted_edge_stays_still() {
        assert_eq!(
            edge_autoscroll((460.0, 520.0), VIEW, (1000.0, 0.0), DT),
            None
        );
        assert_eq!(
            edge_autoscroll((90.0, 150.0), VIEW, (0.0, 1000.0), DT),
            None
        );
    }

    /// A row taller than the viewport hangs over both edges at once. Pulled two ways it
    /// would scroll nowhere, or jitter between them; the leading edge decides.
    #[test]
    fn a_row_taller_than_the_viewport_follows_its_leading_edge() {
        let both = edge_autoscroll((50.0, 900.0), VIEW, ROOM, DT).expect("over both edges");
        assert!(both > 0.0, "the top edge wins: {both}");
    }
}

/// **A back gesture the page below cannot take** (milestone 531).
///
/// Seen on a phone: a back gesture from the left edge, at the height of a task row of the
/// page below, did not slide the page on top. Half a second in, the row was lifted instead —
/// drawn over the page on top in its green-outlined ghost, and following the finger — and it
/// went away when the finger did. At a height with no row the gesture worked.
///
/// The press that starts the gesture is tested against the frame on screen, and after a push
/// that frame is the push's last: an application's animation that has just settled asks for
/// no rebuild, so the page the push left is still in the frame, parallaxed to the left and
/// under the finger. The press found the row's hold, the long press armed, and its deadline
/// replaced the gesture with a lift.
#[cfg(test)]
mod back_gesture_tests {
    use super::{
        drag_after_hold, frame_needs_build, hold_candidates, Drag, Point, Theme, WidgetId,
    };
    use frus_widgets::{build_ui, column, Container, DragSource, Draggable, Navigator};
    use frus_widgets::{Rect, Runtime, Size, Ui, Widget};

    /// A phone, in logical pixels.
    const W: f32 = 392.0;
    const H: f32 = 850.0;
    /// Where the row sits on the page below, and the finger's press on the edge at its
    /// height.
    const ROW_TOP: f32 = 480.0;
    const ROW_HEIGHT: f32 = 66.0;
    const ON_THE_ROW: Point = Point::new(3.0, 509.0);
    const ABOVE_THE_ROW: Point = Point::new(3.0, 200.0);
    /// The long press the row asks for.
    const HELD: u8 = 1;

    /// A page with one row that lifts on a hold and says something when held, below a
    /// stretch with nothing to take.
    fn home() -> impl Widget<u8> {
        column![
            Container::<u8>::new().width(W).height(ROW_TOP),
            Draggable::new(
                Container::<u8>::new()
                    .width(W)
                    .height(ROW_HEIGHT)
                    .on_long_press(HELD)
            )
            .payload(7)
            .long_press(),
        ]
    }

    /// The frame on screen after a push from `home` has settled: the push's last, with the
    /// page it left still in it.
    fn after_a_push() -> Navigator<u8> {
        Navigator::new("data", Container::<u8>::new().width(W).height(H))
            .size(W, H)
            .from("home", home(), 0.999, true)
    }

    fn frame(root: &dyn Widget<u8>) -> Ui<u8> {
        build_ui(
            root,
            Size::new(W, H),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    fn back(at: Point) -> Drag {
        Drag::Back { start_x: at.x }
    }

    fn still_scroll() -> Drag {
        Drag::Scroll {
            id: WidgetId::from_u64(9),
            last: ON_THE_ROW,
            moved: false,
            carried: (0.0, 0.0),
            dismiss: None,
            axis: None,
        }
    }

    fn row_source() -> DragSource {
        DragSource {
            id: WidgetId::from_u64(3),
            rect: Rect::new(-117.0, ROW_TOP, W, ROW_HEIGHT),
            payload: 7,
        }
    }

    /// **Why the row was under the finger at all**: the frame a transition ends on is built
    /// again. The tick that ends a push takes the page it left out of the state and reports
    /// nothing moving, so a loop that builds only while something moves hit-tests the
    /// push's last frame from then on — the page left behind still in it, where a tap or a
    /// hold can reach what is no longer on screen. The demo's
    /// `the_last_frame_of_a_push_still_holds_the_page_it_left` shows that frame on its own
    /// page, at the phone's coordinates.
    #[test]
    fn the_frame_an_animation_settles_in_is_built_again() {
        assert!(
            frame_needs_build(false, false, true, false),
            "the frame the application's animation stops in builds the view again"
        );
        assert!(
            frame_needs_build(false, true, true, false),
            "as does every frame while it moves"
        );
        assert!(
            !frame_needs_build(false, false, false, false),
            "and a still frame after a still one repaints the tree it has"
        );
        assert!(frame_needs_build(true, false, false, false), "unless asked");
        assert!(
            frame_needs_build(false, false, false, true),
            "or the frame has its own reason"
        );
    }

    /// The reproduction: the frame really has a row to take under the finger, and a back
    /// gesture's press takes nothing from it.
    #[test]
    fn a_back_gesture_claims_nothing_under_its_finger() {
        let root = after_a_push();
        let ui = frame(&root);
        let (held, lift) = hold_candidates(None, Some(&ui), Some(&root), ON_THE_ROW);
        assert!(
            lift.is_some() && held == Some(HELD),
            "the frame the phone had: a press there, with no gesture, finds the row's hold"
        );

        let gesture = back(ON_THE_ROW);
        let (held, lift) = hold_candidates(Some(&gesture), Some(&ui), Some(&root), ON_THE_ROW);
        assert!(
            lift.is_none(),
            "a back gesture's press arms no lift of the row under it"
        );
        assert_eq!(held, None, "nor its long press");
    }

    /// Whatever a press found, the deadline never takes the pointer from a back gesture:
    /// the gesture goes on, and keeps its start.
    #[test]
    fn a_hold_never_takes_the_pointer_from_a_back_gesture() {
        let reorder = Some((WidgetId::from_u64(4), 2, ON_THE_ROW));
        for (lift, reorder) in [
            (Some(row_source()), None),
            (None, reorder),
            (Some(row_source()), reorder),
            (None, None),
        ] {
            let after = drag_after_hold(Some(back(ON_THE_ROW)), lift, reorder, ON_THE_ROW);
            assert!(
                matches!(after, Some(Drag::Back { start_x }) if start_x == ON_THE_ROW.x),
                "the back gesture is still the drag (lift {}, reorder {})",
                lift.is_some(),
                reorder.is_some()
            );
        }
    }

    /// At a height with nothing to take — where the gesture always worked — it still does.
    #[test]
    fn a_back_gesture_where_nothing_is_under_it_still_owns_the_pointer() {
        let root = after_a_push();
        let ui = frame(&root);
        let gesture = back(ABOVE_THE_ROW);
        let (held, lift) = hold_candidates(Some(&gesture), Some(&ui), Some(&root), ABOVE_THE_ROW);
        assert!(held.is_none() && lift.is_none());
        assert!(matches!(
            drag_after_hold(Some(gesture), None, None, ABOVE_THE_ROW),
            Some(Drag::Back { .. })
        ));
    }

    /// No regression: a hold on the row with no back gesture — the list's scroll still
    /// waiting to see whether the finger moves — lifts it, or its reorder, and says what
    /// the row asks when held.
    #[test]
    fn a_hold_on_a_row_without_a_back_gesture_still_lifts_it() {
        let root = home();
        let ui = frame(&root);
        let at = Point::new(40.0, ROW_TOP + 10.0);
        let scroll = still_scroll();
        let (held, lift) = hold_candidates(Some(&scroll), Some(&ui), Some(&root), at);
        assert_eq!(
            held,
            Some(HELD),
            "the long press is armed under a still scroll"
        );
        assert!(lift.is_some(), "and so is the lift");
        let (held, lift) = hold_candidates(None, Some(&ui), Some(&root), at);
        assert!(held.is_some() && lift.is_some(), "and with no drag at all");

        let cursor = Point::new(41.0, ROW_TOP + 11.0);
        let lifted = drag_after_hold(Some(still_scroll()), Some(row_source()), None, cursor);
        assert!(
            matches!(lifted, Some(Drag::Item { moved: true, start, .. }) if start == cursor),
            "the hold lifts the item where the finger is"
        );
        let reorder = Some((WidgetId::from_u64(4), 2, at));
        let reordering = drag_after_hold(Some(still_scroll()), None, reorder, cursor);
        assert!(
            matches!(reordering, Some(Drag::Reorder { moved: true, from: 2, start, .. }) if start == at),
            "or picks the row up for a reorder, from where it was pressed"
        );
        assert!(
            drag_after_hold(Some(still_scroll()), None, None, cursor).is_none(),
            "a hold with nothing to lift ends the still scroll: the long press had it"
        );
    }
}

/// A driver with no window and no event loop: input and frames fed by hand, through the
/// shell's own paths.
#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub mod testing {
    use super::*;
    use crate::gesture::{PointerEvent, PointerKind, LONG_PRESS_DELAY};

    /// The shell around an application, on a surface of a stated logical size.
    pub struct Driver<A: Application> {
        shell: App<A>,
        size: Size,
        /// Where the last frame drew a reorder's ghost, when it drew one.
        ghost: Option<Rect>,
    }

    impl<A: Application> Driver<A> {
        /// A driver for `app` on a `width` by `height` logical surface.
        pub fn new(app: A, width: f32, height: f32) -> Self {
            Self {
                shell: App::detached(app),
                size: Size::new(width, height),
                ghost: None,
            }
        }

        /// Where what is carried is drawn this frame — a reorder's ghost or a lifted item —
        /// or `None` when nothing is.
        pub fn carried(&self) -> Option<Rect> {
            self.shell.carried_rect()
        }

        /// Where the last frame drew a reorder's ghost — the outline around it — or `None`
        /// when it drew none.
        pub fn ghost(&self) -> Option<Rect> {
            self.ghost
        }

        /// Every scroll region of the last frame, as its viewport and its offset.
        pub fn offsets(&self) -> Vec<(Rect, (f32, f32))> {
            let s = &self.shell;
            s.ui.as_ref()
                .map(|ui| {
                    ui.scroll_regions()
                        .iter()
                        .map(|a| {
                            (
                                a.viewport,
                                s.runtime.scroll.get(&a.id).copied().unwrap_or((0.0, 0.0)),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        /// The application, as the shell holds it.
        pub fn app(&self) -> &A {
            &self.shell.app
        }

        /// Hands the application a message, as the shell does.
        pub fn update(&mut self, message: A::Message) {
            self.shell.dispatch(message);
        }

        /// Frames at sixty a second for `seconds`.
        pub fn run(&mut self, seconds: f32) {
            let n = (seconds * 60.0).round() as usize;
            for _ in 0..n.max(1) {
                self.frame(1.0 / 60.0);
            }
        }

        /// A finger down at `at`.
        pub fn press(&mut self, at: Point) {
            self.pointer(PointerKind::Down, at);
        }

        /// The finger moved to `at`.
        pub fn move_to(&mut self, at: Point) {
            self.pointer(PointerKind::Move, at);
        }

        /// The finger lifted at `at`.
        pub fn release(&mut self, at: Point) {
            self.pointer(PointerKind::Up, at);
        }

        /// The long-press deadline, delivered as the loop's wake delivers it. Whether it
        /// fired: a press that moved past the slop, or was never waiting, has none.
        pub fn hold_deadline(&mut self) -> bool {
            let fired = self.shell.press.poll(Instant::now() + LONG_PRESS_DELAY);
            if fired {
                self.shell.hold_deadline_reached();
            }
            fired
        }

        fn pointer(&mut self, kind: PointerKind, at: Point) {
            self.shell.pointer_event(PointerEvent {
                kind,
                position: at,
                touch: true,
            });
        }

        /// One frame, the real frame's steps in the real frame's order, less the GPU.
        pub fn frame(&mut self, dt: f32) {
            let (width, height) = (self.size.width, self.size.height);
            let s = &mut self.shell;
            if s.last_size != Some((width, height)) {
                s.last_size = Some((width, height));
                s.build_dirty = true;
                s.app.on_resize(width, height);
            }
            let mut app_animating = s.app.tick(dt);
            let settings = s
                .platform
                .accessibility
                .with_overrides(s.app.accessibility());
            let theme_moved = s.themes.advance(
                &s.app,
                s.platform.brightness,
                settings.high_contrast,
                settings.disable_animations,
                dt,
            );
            app_animating |= theme_moved | s.themes.animating();
            let theme = s.themes.displayed(&s.app);
            install_ambient(&s.app, &mut s.runtime, &s.platform.locales);
            let _surface = s.media_query(width, height).install();
            s.runtime.still = settings.disable_animations;
            let was = std::mem::replace(&mut s.app_was_animating, app_animating);
            if frame_needs_build(
                s.build_dirty,
                app_animating,
                was,
                s.tree.is_none() || s.runtime.switching(),
            ) {
                s.tree = Some(build_view(&s.app, &theme, &s.runtime));
            }
            s.build_dirty = false;
            s.autoscroll_carried(dt);
            s.advance_reorder_springs(dt);
            let regions =
                s.ui.as_ref()
                    .map(|ui| ui.scroll_regions().to_vec())
                    .unwrap_or_default();
            let physics = s.app.scroll_physics();
            s.runtime.sync_pages(&regions);
            s.runtime.sync_visible(&regions);
            s.runtime.advance(dt);
            s.runtime.advance_scroll(&regions, physics, dt);
            let tree = s.tree.as_deref().expect("the view was built");
            let ui = build_ui(tree, Size::new(width, height), &s.runtime, &theme);
            let mut scene = ui.scene().clone();
            let ghost = if matches!(s.drag, Some(Drag::Reorder { moved: true, .. })) {
                s.paint_reorder_preview(&ui, &theme, &mut scene);
                // The ghost's outline is the last thing the preview draws, on the ghost's box.
                scene.primitives().last().map(|p| p.bounds())
            } else {
                None
            };
            let paged = ui.scroll_regions().to_vec();
            let scrolled: Vec<A::Message> = {
                let grain = |id| find_widget(tree, id).map_or(0.0, |w| w.scroll_grain());
                s.runtime
                    .scroll_changes(&paged, grain)
                    .into_iter()
                    .filter_map(|(id, position)| {
                        find_widget(tree, id).and_then(|w| w.on_scroll(position))
                    })
                    .collect()
            };
            s.ui = Some(ui);
            for message in scrolled {
                s.dispatch(message);
            }
            self.ghost = ghost;
        }
    }
}

/// **What is carried stays carried, however far its list scrolls** (milestone 527, seen on
/// a phone).
///
/// A label held on the Kanban screen's strip and carried towards the strip's right edge was
/// not drawn lifted, the strip scrolled away under it, and the release put it somewhere the
/// finger had never been. The carried row's box came from the frame, and the frame keeps a
/// reorderable's box only while it is on screen, clipped to what shows: once auto-scroll had
/// pushed the row against the edge the box stopped moving with the content while the press
/// kept moving, the ghost ran ahead of the finger, the scroll fed on it, and a row scrolled
/// out of sight had no box at all.
///
/// These drive the shell's own input path and its frame, on the screen's shape: a bar, a strip
/// of ten 96-px labels that lift on a hold, and a board of cards, each in a horizontal scroll.
#[cfg(test)]
mod carried_row_tests {
    use super::testing::Driver;
    use crate::{Application, Command};
    use frus_widgets::{
        column, text, Axis, Button, CellFn, Container, Flex, Kanban, Point, Rect, ReorderAxis,
        ReorderGrab, ReorderableList, SingleChildScrollView, Theme, Widget,
    };

    /// A phone, in logical pixels.
    const W: f32 = 392.7;
    const H: f32 = 850.9;
    /// The height of the labels' middle, under the 56-px bar and the strip's 12 px of padding.
    const STRIP_Y: f32 = 88.0;
    /// How wide the strip's content is: ten labels, nine gaps, and 24 px at either end.
    const STRIP_CONTENT: f32 = 24.0 + 10.0 * 96.0 + 9.0 * 8.0 + 24.0;

    #[derive(Clone, Debug, PartialEq)]
    enum Board {
        Label(usize, usize),
        Card(usize, usize, usize, usize),
        Delete(usize, usize),
        Add(usize),
        Row(usize, usize),
    }

    /// The Kanban screen, in the shape that matters.
    struct BoardApp {
        labels: Vec<usize>,
        cards: Vec<Vec<String>>,
        log: Vec<Board>,
    }

    impl Default for BoardApp {
        fn default() -> Self {
            let cols: [&[&str]; 3] = [
                &["Design API", "Write spec", "Triage bugs"],
                &["Build widget"],
                &["Kickoff", "Research"],
            ];
            Self {
                labels: (0..10).collect(),
                cards: cols
                    .iter()
                    .map(|c| c.iter().map(|s| s.to_string()).collect())
                    .collect(),
                log: Vec::new(),
            }
        }
    }

    impl Application for BoardApp {
        type Message = Board;

        fn update(&mut self, message: Board) -> Command<Board> {
            self.log.push(message.clone());
            match message {
                Board::Label(from, to) => {
                    let label = self.labels.remove(from);
                    self.labels.insert(to, label);
                }
                Board::Card(from_col, from_pos, to_col, to_pos) => {
                    let card = self.cards[from_col].remove(from_pos);
                    let to_pos = to_pos.min(self.cards[to_col].len());
                    self.cards[to_col].insert(to_pos, card);
                }
                _ => {}
            }
            Command::none()
        }

        fn view(&self, theme: &Theme) -> Box<dyn Widget<Board>> {
            let mut strip = ReorderableList::new(Board::Label)
                .axis(ReorderAxis::Horizontal)
                .grab(ReorderGrab::LongPress)
                .gap(8.0);
            for &label in &self.labels {
                strip = strip.keyed_row(
                    label as u64,
                    Container::new()
                        .width(96.0)
                        .padding(12.0)
                        .color(theme.surface)
                        .child(text(format!("label {label}")).size(14.0)),
                );
            }
            let strip = SingleChildScrollView::new()
                .axis(Axis::Horizontal)
                .width(W)
                .child(
                    Container::new()
                        .padding_each(12.0, 24.0, 0.0, 24.0)
                        .child(strip),
                );
            let mut board = Kanban::new(Board::Card)
                .on_add(Board::Add)
                .scrollable_columns();
            for (col, title) in ["To do", "Doing", "Done"].iter().enumerate() {
                let cards: Vec<CellFn<Board>> = self.cards[col]
                    .iter()
                    .enumerate()
                    .map(|(pos, label)| {
                        let label = label.clone();
                        Box::new(move || {
                            Box::new(
                                Flex::row()
                                    .child(text(label.clone()).size(14.0))
                                    .child(Flex::row().flex(1.0))
                                    .child(Button::new("x").on_press(Board::Delete(col, pos))),
                            ) as Box<dyn Widget<Board>>
                        }) as CellFn<Board>
                    })
                    .collect();
                board = board.column_widgets(*title, cards);
            }
            let board = SingleChildScrollView::new()
                .axis(Axis::Horizontal)
                .width(W)
                .flex(1.0)
                .child(Container::new().padding(24.0).child(board));
            Box::new(
                Container::new().width(W).height(H).child(
                    column![
                        Container::<Board>::new().width(W).height(56.0),
                        strip,
                        board
                    ]
                    .flex(1.0),
                ),
            )
        }
    }

    /// Twenty 60-px rows that lift on a hold, in a list taller than the window.
    #[derive(Default)]
    struct ListApp {
        log: Vec<Board>,
    }

    impl Application for ListApp {
        type Message = Board;

        fn update(&mut self, message: Board) -> Command<Board> {
            self.log.push(message);
            Command::none()
        }

        fn view(&self, _theme: &Theme) -> Box<dyn Widget<Board>> {
            let mut list = ReorderableList::new(Board::Row)
                .grab(ReorderGrab::LongPress)
                .width(W);
            for row in 0..20u64 {
                list = list.keyed_row(
                    row,
                    Container::new()
                        .height(60.0)
                        .child(text(format!("row {row}")).size(14.0)),
                );
            }
            Box::new(
                SingleChildScrollView::new()
                    .axis(Axis::Vertical)
                    .width(W)
                    .height(H)
                    .child(list),
            )
        }
    }

    fn driver<A: Application>(app: A) -> Driver<A> {
        let mut driver = Driver::new(app, W, H);
        driver.run(0.2);
        driver
    }

    /// A finger held still at `at` until the long press has lifted what is under it.
    fn hold<A: Application>(driver: &mut Driver<A>, at: Point) {
        driver.press(at);
        driver.run(0.5);
        assert!(
            driver.hold_deadline(),
            "the hold lifts what is under {at:?}"
        );
        driver.run(0.05);
    }

    /// The finger through `points`, a tenth of a second apart, as the phone's were.
    fn carry<A: Application>(driver: &mut Driver<A>, points: impl IntoIterator<Item = Point>) {
        for at in points {
            driver.move_to(at);
            driver.run(0.1);
        }
    }

    /// Frame by frame for `seconds` with the finger still at `finger`: the carried box keeps
    /// the place it had under the finger — `offset` from it and `size` — and the region
    /// `region` keeps scrolling the way it started. Hands back that region's last offset.
    fn held_under_the_finger<A: Application>(
        driver: &mut Driver<A>,
        seconds: f32,
        finger: Point,
        offset: (f32, f32),
        size: (f32, f32),
        region: usize,
    ) -> (f32, f32) {
        let mut last = driver.offsets()[region].1;
        for frame in 0..(seconds * 60.0) as usize {
            driver.run(1.0 / 60.0);
            let now = driver.offsets()[region].1;
            let carried: Rect = driver.carried().unwrap_or_else(|| {
                panic!("frame {frame}: nothing is carried any more, the list at {now:?}")
            });
            assert!(
                (carried.x - finger.x - offset.0).abs() < 0.5
                    && (carried.y - finger.y - offset.1).abs() < 0.5
                    && (carried.width - size.0).abs() < 0.5
                    && (carried.height - size.1).abs() < 0.5,
                "frame {frame}: carried at {carried:?}, the finger at {finger:?}, the list at {now:?}"
            );
            assert!(
                now.0 >= last.0 - 0.01 && now.1 >= last.1 - 0.01,
                "frame {frame}: the list went back, {last:?} then {now:?}"
            );
            // And it is drawn there: the ghost the finger sees is the box being carried.
            let ghost = driver
                .ghost()
                .unwrap_or_else(|| panic!("frame {frame}: no ghost is drawn"));
            assert!(
                (ghost.x - carried.x).abs() < 1.0
                    && (ghost.y - carried.y).abs() < 1.0
                    && (ghost.width - carried.width).abs() < 2.0,
                "frame {frame}: the ghost is drawn at {ghost:?}, what is carried is at {carried:?}"
            );
            last = now;
        }
        last
    }

    /// **The reproduction.** A label held and carried past the strip's right edge stays under
    /// the finger for as long as it is held there, whole, while the strip scrolls on to its
    /// end; and the release drops it where the finger is, past the last label.
    #[test]
    fn a_label_carried_past_the_strips_edge_stays_under_the_finger() {
        let mut driver = driver(BoardApp::default());
        hold(&mut driver, Point::new(155.0, STRIP_Y));
        let lifted = driver.carried().expect("the label is carried");
        assert_eq!(
            lifted,
            Rect::new(128.0, 68.0, 96.0, 41.0),
            "Feature, lifted"
        );
        let finger = Point::new(370.0, STRIP_Y);
        carry(
            &mut driver,
            [200.0, 260.0, 320.0, finger.x].map(|x| Point::new(x, STRIP_Y)),
        );
        let strip = held_under_the_finger(
            &mut driver,
            1.5,
            finger,
            (lifted.x - 155.0, lifted.y - STRIP_Y),
            (96.0, 41.0),
            0,
        );
        assert!(
            (strip.0 - (STRIP_CONTENT - W)).abs() < 1.0,
            "the strip scrolled on to its end: {strip:?}"
        );
        driver.release(finger);
        driver.run(0.2);
        assert_eq!(
            driver.app().log,
            [Board::Label(1, 9)],
            "past the last label, it goes last; no card moves"
        );
    }

    /// **The phone's own gesture**, where the label never reaches an edge: held on Feature,
    /// carried along x as the injected moves went, let go over Design's right half.
    #[test]
    fn a_hold_on_a_label_then_a_carry_along_x_reorders_the_labels_and_moves_no_card() {
        let mut driver = driver(BoardApp::default());
        hold(&mut driver, Point::new(155.0, STRIP_Y));
        carry(
            &mut driver,
            [167.0, 189.0, 218.0, 247.0, 276.0, 306.0].map(|x| Point::new(x, STRIP_Y)),
        );
        driver.run(0.4);
        assert!(driver.carried().is_some(), "still carried");
        driver.release(Point::new(306.0, STRIP_Y));
        driver.run(0.2);
        assert_eq!(driver.app().log, [Board::Label(1, 2)]);
        assert_eq!(driver.app().labels[..4], [0, 2, 1, 3]);
        assert_eq!(
            driver.app().cards,
            BoardApp::default().cards,
            "no card moved"
        );
        assert_eq!(driver.offsets()[0].1, (0.0, 0.0), "nothing scrolled");
        assert_eq!(driver.offsets()[1].1, (0.0, 0.0), "the board neither");
    }

    /// **A card still crosses the board**: Design API, pressed and carried onto the upper
    /// half of Build widget, goes to the top of Doing; no label moves.
    #[test]
    fn a_card_carried_across_the_board_moves_the_card() {
        let mut driver = driver(BoardApp::default());
        driver.press(Point::new(100.0, 190.0));
        carry(
            &mut driver,
            [130.0, 180.0, 240.0, 300.0].map(|x| Point::new(x, 185.0)),
        );
        driver.release(Point::new(300.0, 185.0));
        driver.run(0.2);
        assert_eq!(driver.app().log, [Board::Card(0, 0, 1, 0)]);
        assert_eq!(driver.app().labels, (0..10).collect::<Vec<_>>());
    }

    /// **The board has the same edge.** A card carried to the board's right edge scrolls
    /// the board and stays under the finger, although the column it came from scrolls away.
    #[test]
    fn a_card_carried_to_the_boards_edge_stays_under_the_finger() {
        let mut driver = driver(BoardApp::default());
        let press = Point::new(100.0, 190.0);
        driver.press(press);
        let finger = Point::new(385.0, 190.0);
        carry(
            &mut driver,
            [130.0, 200.0, 280.0, 340.0, finger.x].map(|x| Point::new(x, 190.0)),
        );
        let card = driver.carried().expect("the card is carried");
        let board = held_under_the_finger(
            &mut driver,
            1.0,
            finger,
            (card.x - finger.x, card.y - finger.y),
            (card.width, card.height),
            1,
        );
        assert!(
            board.0 > 250.0,
            "the board scrolled past the column the card came from: {board:?}"
        );
    }

    /// **A list that runs down has the same edge**: row 0 held and carried to the bottom
    /// stays under the finger after its own place has scrolled off the top, and lands low in
    /// the list.
    #[test]
    fn a_row_carried_past_the_bottom_of_a_long_list_stays_under_the_finger() {
        let mut driver = driver(ListApp::default());
        hold(&mut driver, Point::new(100.0, 30.0));
        let lifted = driver.carried().expect("the row is carried");
        let finger = Point::new(100.0, 840.0);
        carry(
            &mut driver,
            [200.0, 400.0, 600.0, 800.0, finger.y].map(|y| Point::new(100.0, y)),
        );
        let list = held_under_the_finger(
            &mut driver,
            1.5,
            finger,
            (lifted.x - 100.0, lifted.y - 30.0),
            (lifted.width, lifted.height),
            0,
        );
        assert!(
            list.1 > 200.0,
            "the list scrolled past the row's own place: {list:?}"
        );
        driver.release(finger);
        driver.run(0.2);
        assert!(
            matches!(driver.app().log[..], [Board::Row(0, to)] if to > 15),
            "row 0 lands near the end: {:?}",
            driver.app().log
        );
    }
}
