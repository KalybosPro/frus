//! The generic driver: implements [`winit::application::ApplicationHandler`] for
//! any [`Application`].
//!
//! The framework owns the window, the renderer, the [`Runtime`] — the retained
//! interaction state: hover, focus, scroll, editing, animations — input routing by
//! hit test, dragging (scrollbars, selection, handles, the back gesture) and the
//! animation clock. The application supplies only `update`, `view` and friends.

use std::collections::HashMap;
#[cfg(not(web))]
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::Arc;

use web_time::Instant;

use frus_gpu::{wgpu, Renderer};
use frus_widgets::{
    build_ui, collect_ids, find_by_key, find_path, find_widget, reflow_reorder_cards,
    reflow_reorder_columns, subtree_ids, Color, Cursor as UiCursor, Edit, FocusDirection, Insets,
    Key, KeyResponse, MediaQuery, Point, Primitive, Rect, ReorderAxis, Runtime, Scene, Size, Theme,
    Ui, VelocityEstimate, VelocityTracker, Widget, WidgetId, WindowInsets,
};
use winit::application::ApplicationHandler;
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, StartCause, TouchPhase, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{CursorIcon, Window, WindowId};

use crate::application::{Application, Lifecycle};
use crate::gesture::{PointerEvent, PointerKind, PressRecognizer};

/// The clipboard: `arboard` on the desktop platforms, a no-op **everywhere else**
/// (Android, iOS, Web — `arboard` does not compile there and is not a dependency).
/// The stub is gated on `not(desktop)` rather than on a list of the other platforms:
/// that is what makes adding a target never leave this type undefined.
/// One uniform API, so the driver's body stays free of `cfg`.
mod clip {
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

    #[cfg(not(desktop))]
    pub struct Clipboard;

    #[cfg(not(desktop))]
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
}

/// An active subscription's cancellation handle: **dropping** it stops the
/// subscription.
/// - Native: a `Sender<()>` — the subscription's thread exits at its next wake-up.
/// - Web: a retained `setInterval` (`None` when installing it failed).
#[cfg(not(web))]
type SubHandle = Sender<()>;
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
    },
    /// A text selection inside a field, with its bounds, for placement.
    TextSelect {
        id: WidgetId,
        rect: frus_widgets::Rect,
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
    /// The "back" gesture: the framework measures the finger's progress and velocity
    /// and passes them to the application, which decides on the navigation.
    Back { start_x: f32 },
}

/// The driver: an `event → frame` loop around an [`Application`].
pub struct App<A: Application> {
    /// The application being driven: its state and logic.
    app: A,
    /// The channel that feeds back the messages effects produce, from their threads.
    proxy: EventLoopProxy<A::Message>,
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
    /// The screen's DPI scale factor (physical = logical × scale × density).
    scale: f32,
    /// The last **logical** size handed to the app, to detect breakpoints.
    last_size: Option<(f32, f32)>,
    /// State retained between frames: hover and focus, scroll, caret and selection.
    runtime: Runtime,
    /// Live-reload watching (development, `FRUS_WATCH=1`): relaunch on recompilation.
    reload: Option<crate::reload::ReloadWatcher>,
    /// Is the runtime inspector on? Toggled by F12, in debug builds only.
    inspector: bool,
    /// A tree dump to print on the next inspected frame.
    inspector_dump: bool,
    /// The current keyboard modifiers.
    shift: bool,
    ctrl: bool,
    /// The remembered "goal" visual column for Up/Down/PgUp/PgDn: crossing shorter
    /// lines keeps the original column, the way an editor does. Cleared as soon as any
    /// other caret movement happens.
    goal_x: Option<f32>,
    /// Clipboard access; a no-op on Android.
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
    /// A [`frus_widgets::Draggable`] that asked to be lifted by a **hold**, waiting
    /// for the long-press deadline. Inside a scrollable this is the only way up: the
    /// plain drag belongs to the scroll, and a hold is the one signal it cannot claim.
    pending_lift: Option<frus_widgets::DragSource>,
    /// The last click's instant, for double-click detection.
    last_click_time: Option<Instant>,
    /// A counter for the keys of leaving events, which fade out.
    leaving_counter: u64,
    /// The running subscriptions: id → cancellation handle, dropping which stops it.
    running_subs: HashMap<u64, SubHandle>,
    /// Pending focus requests — the keys `Command::focus` produced — resolved against
    /// the **freshly built** tree on the next frame.
    pending_focus: Vec<u64>,
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
    /// The length, in characters, of the IME **composition** under way in the focused
    /// field; it is replaced on every IME update.
    #[cfg(android)]
    ime_composing: usize,
}

impl<A: Application> App<A> {
    /// Creates the driver around an application and its message channel.
    pub fn new(app: A, proxy: EventLoopProxy<A::Message>) -> Self {
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
            scale: 1.0,
            last_size: None,
            runtime: Runtime::default(),
            reload: crate::reload::ReloadWatcher::new(),
            inspector: false,
            inspector_dump: false,
            shift: false,
            ctrl: false,
            goal_x: None,
            clipboard: clip::Clipboard::new(),
            started: false,
            last_frame: None,
            build_dirty: true,
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
            pending_lift: None,
            last_click_time: None,
            leaving_counter: 0,
            running_subs: HashMap::new(),
            pending_focus: Vec::new(),
            focus_history: Vec::new(),
            prev_focus: None,
            occluded: false,
            elapsed: 0.0,
            last_insets: WindowInsets::ZERO,
            inset_baseline: None,
            // The app starts out detached; `resumed` will move it to `Resumed`.
            lifecycle: Lifecycle::Detached,
            #[cfg(android)]
            android_app: None,
            #[cfg(android)]
            soft_input_shown: false,
            #[cfg(android)]
            ime_composing: 0,
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

    /// Keeps the **software keyboard** in step with focus: asked for when focus is in
    /// a text field (`cursor_at` → `Some`), closed otherwise. Called at the end of a
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
                .and_then(|widget| widget.cursor_at(0.0, 0.0, 1.0, 0))
                .is_some();
            if editing != self.soft_input_shown {
                self.soft_input_shown = editing;
                self.ime_composing = 0;
                if crate::android_ime::installed() {
                    // The InputConnection bridge: the Java view captures the IME.
                    if editing {
                        if let Some(id) = self.runtime.input.focused {
                            self.push_ime_context(id);
                        }
                        crate::android_ime::start_input();
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
            self.ime_composing = 0;
            if crate::android_ime::installed() {
                if let Some(id) = self.runtime.input.focused {
                    self.push_ime_context(id);
                }
                crate::android_ime::start_input();
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
        use crate::android_ime::ImeEvent;
        let events = crate::android_ime::drain();
        if events.is_empty() {
            return;
        }
        let Some(focused) = self.runtime.input.focused else {
            self.ime_composing = 0;
            return;
        };
        for event in events {
            match event {
                // A `\n` commit, which some IMEs send, means submit, not insert.
                ImeEvent::Commit(text) if text == "\n" || text == "\r" => {
                    self.clear_composing(focused);
                    self.apply_key(focused, Key::Enter);
                }
                ImeEvent::Commit(text) => {
                    self.clear_composing(focused);
                    self.apply_key(focused, Key::Text(text));
                }
                ImeEvent::Composing(text) => {
                    self.clear_composing(focused);
                    let n = text.chars().count();
                    self.ime_composing = n;
                    // The position BEFORE insertion is where the composed region starts.
                    let start = self
                        .runtime
                        .edits
                        .get(&focused)
                        .map(|e| e.cursor)
                        .unwrap_or(0);
                    if !text.is_empty() {
                        self.apply_key(focused, Key::Text(text));
                    }
                    // Record the underlined range; the caret now sits at its end.
                    if let Some(edit) = self.runtime.edits.get_mut(&focused) {
                        edit.composing = if n > 0 {
                            Some((start, start + n))
                        } else {
                            None
                        };
                    }
                }
                ImeEvent::FinishComposing => {
                    self.ime_composing = 0;
                    if let Some(edit) = self.runtime.edits.get_mut(&focused) {
                        edit.composing = None;
                    }
                }
                ImeEvent::Delete { before, after } => {
                    for _ in 0..before {
                        self.apply_key(focused, Key::Backspace);
                    }
                    for _ in 0..after {
                        self.apply_key(focused, Key::Delete);
                    }
                }
                ImeEvent::Action => self.apply_key(focused, Key::Enter),
                ImeEvent::Key { code, unicode } => match code {
                    66 => self.apply_key(focused, Key::Enter),
                    67 => self.apply_key(focused, Key::Backspace),
                    _ => {
                        if let Some(c) = char::from_u32(unicode).filter(|c| !c.is_control()) {
                            self.apply_key(focused, Key::Text(c.to_string()));
                        }
                    }
                },
            }
        }
        // Refresh the input context, which the IME queries for its suggestions.
        self.push_ime_context(focused);
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
        }
    }

    /// Erases the field's current composition, the caret sitting at its end, and
    /// clears the underlined range.
    #[cfg(android)]
    fn clear_composing(&mut self, focused: WidgetId) {
        for _ in 0..self.ime_composing {
            self.apply_key(focused, Key::Backspace);
        }
        self.ime_composing = 0;
        if let Some(edit) = self.runtime.edits.get_mut(&focused) {
            edit.composing = None;
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
}

impl<A: Application> ApplicationHandler<A::Message> for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // Back in the foreground, or starting up: the surface is (re)born below.
        self.set_lifecycle(Lifecycle::Resumed);

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
            // Two claims on one hold: a widget asking for a message, and an item asking
            // to be lifted. Serving both would do a discrete action *and* start a drag
            // from the same gesture, which is never what anyone meant. **The lift
            // wins** — it changes what the rest of the gesture means, and the message
            // would be acting on something the finger is still holding.
            let lifting = self.pending_lift.is_some();
            if let Some(message) = self.long_press_msg.take() {
                if !lifting {
                    self.dispatch(message);
                }
            }
            // A pending touch scroll no longer has any reason to exist — unless the
            // hold was the lift of an item, in which case the scroll hands the gesture
            // over and the item is already up: the hold *was* the threshold.
            if let Some(source) = self.pending_lift.take() {
                if let Some(Drag::Scroll { id, .. }) = self.drag {
                    self.runtime.release_scroll(id);
                }
                self.drag = Some(Drag::Item {
                    source,
                    start: self.cursor,
                    moved: true,
                    over: None,
                });
            } else {
                self.drag = None;
            }
            self.request_redraw();
        }

        // Pending IME operations, from the Android input bridge.
        #[cfg(android)]
        self.drain_ime();

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

                // Tab and Shift+Tab move between focusables, even with nothing focused.
                if matches!(event.logical_key, WinitKey::Named(NamedKey::Tab)) {
                    let forward = !self.shift;
                    let next = self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.focus_next(self.runtime.input.focused, forward));
                    if next.is_some() {
                        self.runtime.input.focused = next;
                        self.request_redraw();
                    }
                    return;
                }

                // Escape walks **leaf to root** from the focused widget — a `Portal`
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

                // The clipboard shortcuts, Ctrl+C/X/V/A.
                if self.ctrl {
                    match &event.logical_key {
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("c") => {
                            self.copy_selection(focused);
                            return;
                        }
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("x") => {
                            self.copy_selection(focused);
                            self.apply_key(focused, Key::Backspace);
                            self.request_redraw();
                            return;
                        }
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("v") => {
                            if let Some(text) = self.clipboard.get_text() {
                                self.apply_key(focused, Key::Text(text));
                                self.request_redraw();
                            }
                            return;
                        }
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("a") => {
                            self.runtime.edits.insert(
                                focused,
                                Edit {
                                    cursor: usize::MAX,
                                    anchor: Some(0),
                                    composing: None,
                                },
                            );
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
                if let Some(area) = self.ui.as_ref().and_then(|ui| ui.scroll_hit(self.cursor)) {
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
                    let target = self.runtime.scroll_target.entry(id).or_insert(current);
                    let (wanted_x, wanted_y) = (target.0 - dx, target.1 - dy);
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
                        let edge = frus_widgets::edge_for(vertical, refused);
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
                        Some(renderer) => self.renderer = Some(renderer),
                        None => return,
                    }
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

                // The application advances its own animations: theme, navigation, gesture.
                let app_animating = self.app.tick(dt);
                let theme = self.app.theme();

                // === BUILD phase, conditional ===
                // The `view` is rebuilt only when the app's state or the size changed
                // (`build_dirty`), or when the app is animating, theme, navigation and
                // gesture all altering the state the view reads. A frame that animates
                // interaction alone — hover, scroll, focus, caret — **reuses the
                // retained tree** and merely repaints: the `view` is a pure function of
                // `(state, theme, size)` and never of the `Runtime`.
                let need_build = self.build_dirty || app_animating || self.tree.is_none();
                if need_build {
                    let tree = self
                        .media_query(width, height)
                        .scope(|| self.app.view(&theme, width, height));
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
                let interactive_bounds = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.interactive_bounds())
                    .unwrap_or_default();

                // The reorder spring, per axis, with a time constant of about 70 ms:
                // - **horizontal** (`Table` columns): the smoothed `reorder_x` catches up
                //   with the pointer — the columns slide with inertia while the ghost
                //   sticks to the real pointer;
                // - **vertical** (Kanban cards): the smoothed `reorder_y` catches up with
                //   the **chosen** slot edge, the hovered half — the insertion line and
                //   the gap *slide* between cards instead of jumping, which is the
                //   vertical counterpart of `reorder_x`.
                let reorder_axis = matches!(self.drag, Some(Drag::Reorder { moved: true, .. }))
                    .then(|| self.dragged_reorder_axis())
                    .flatten();
                let reorder_animating = match reorder_axis {
                    Some(ReorderAxis::Horizontal) => {
                        self.reorder_x = spring_toward(self.reorder_x, self.cursor.x, dt, 0.07);
                        (self.cursor.x - self.reorder_x).abs() > 0.5
                    }
                    Some(ReorderAxis::Vertical) => {
                        match self.reorder_drop_line(drag_preview::INSERT_THICKNESS) {
                            Some(target) => {
                                self.reorder_y = spring_toward(self.reorder_y, target.y, dt, 0.07);
                                (target.y - self.reorder_y).abs() > 0.5
                            }
                            None => false,
                        }
                    }
                    None => false,
                };

                // A paged view that has been asked for another page glides across to
                // it. Done before the springs are stepped, so the request is honoured
                // in the frame it arrives rather than the one after.
                self.runtime.sync_pages(&scroll_regions);

                let animating = self.runtime.advance(dt)
                    | self.runtime.advance_leaving(dt)
                    | self.runtime.advance_values(tree, dt)
                    | self.runtime.advance_colors(tree, dt)
                    | self.runtime.advance_sizes(tree, dt)
                    | self.runtime.advance_radii(tree, dt)
                    | self.runtime.advance_paddings(tree, dt)
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
                    | self.runtime.advance_interactive(&interactive_bounds, dt)
                    | reorder_animating
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
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(&scene) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            renderer.reconfigure();
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("GPU memory exhausted, shutting down.");
                            event_loop.exit();
                        }
                        Err(err) => log::warn!("frame skipped: {err:?}"),
                    }
                }

                // A continuously animating widget, a spinner say, forces a redraw.
                let wants_animation = ui.wants_animation();

                // Keep the interface, for hit testing. The tree is already retained.
                self.ui = Some(ui);

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

                // The rows whose gap has just finished closing, and the pages that have
                // turned: the tree is no longer borrowed, so the application can be told
                // — and rebuild.
                for message in dismissed.into_iter().chain(turned) {
                    self.dispatch(message);
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

                // The Android software keyboard follows the text fields' focus.
                self.sync_soft_input();

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
    fn media_query(&self, width: f32, height: f32) -> MediaQuery {
        MediaQuery::new(Size::new(width, height))
            .with_device_pixel_ratio(self.scale)
            .with_density(self.app.density())
            .with_insets(self.last_insets)
    }

    /// The current layout direction; RTL flips both the layout and the gestures.
    fn is_rtl(&self) -> bool {
        self.app.theme().direction.is_rtl()
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
        self.cursor = event.position;
        match event.kind {
            PointerKind::Down => {
                // A pointer interaction: the keyboard focus ring fades away.
                self.runtime.focus_visible = false;
                self.pointer_down(event.touch);
                // A long-press candidate, but only if the press was not captured by a
                // drag — a scrollbar, a handle, a selection. A touch scroll that has not
                // moved yet stays a candidate.
                let free = matches!(self.drag, None | Some(Drag::Scroll { moved: false, .. }));
                self.long_press_msg = if free {
                    self.ui
                        .as_ref()
                        .and_then(|ui| ui.long_press_at(self.cursor))
                } else {
                    None
                };
                // An item that lifts on a hold: the scroll keeps the gesture for now,
                // and the deadline decides. Nothing is taken from the scroll unless the
                // finger actually stays put, which is not a scroll.
                self.pending_lift = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.drag_source_at(self.cursor))
                    .filter(|source| {
                        self.tree
                            .as_ref()
                            .and_then(|tree| find_widget(tree.as_ref(), source.id))
                            .is_some_and(|widget| widget.drag_needs_long_press())
                    });
                let interested = self.long_press_msg.is_some() || self.pending_lift.is_some();
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
                if swallow && !matches!(self.drag, Some(Drag::Item { moved: true, .. })) {
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
                self.drag = None;
                self.runtime.input.pressed = None;
                self.request_redraw();
            }
        }
        // Wake the loop exactly at the next deadline, and rest otherwise.
        event_loop.set_control_flow(self.idle_control_flow());
    }

    /// The loop's idle policy: wake at the **nearest** deadline — the long press, the
    /// live-reload poll — and otherwise wait outright.
    fn idle_control_flow(&self) -> ControlFlow {
        let press = self.press.deadline();
        let reload = self.reload.as_ref().map(|w| w.deadline());
        match (press, reload) {
            (Some(a), Some(b)) => ControlFlow::WaitUntil(a.min(b)),
            (Some(a), None) => ControlFlow::WaitUntil(a),
            (None, Some(b)) => ControlFlow::WaitUntil(b),
            (None, None) => ControlFlow::Wait,
        }
    }

    /// Pointer movement, mouse or finger: continues a drag under way, and otherwise
    /// updates the hover.
    fn pointer_move(&mut self) {
        if self.drag.is_some() {
            self.handle_drag();
            return;
        }
        let hovered = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        if hovered != self.runtime.input.hovered {
            self.runtime.input.hovered = hovered;
            self.request_redraw();
        }
        // The system cursor follows the hovered sub-region (milestone 205): a hand
        // over a clickable icon, and so on. Recomputed on every move, since the
        // sub-region can change without the hovered widget changing.
        self.update_cursor_icon(hovered);
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
            // A zero delta on press: only a slider, which takes a fraction, jumps.
            self.apply_widget_drag(id, rect, 0.0);
            self.request_redraw();
            return;
        }

        // 1c) A column reorder: a press on a reorderable header. We do not `return` —
        // focus and `pressed`, where a tap means sort, are settled below; the drag
        // engages only past the threshold, and otherwise the release sorts.
        if let Some((id, from)) = self.reorderable_at(self.cursor) {
            self.drag = Some(Drag::Reorder {
                id,
                from,
                start: self.cursor,
                moved: false,
            });
            self.reorder_x = self.cursor.x; // starts glued to the pointer, with no jerk
            self.reorder_y = self.cursor.y; // likewise for the vertical insertion line
        }

        self.runtime.input.pressed = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
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
            if let Some(area) = self.ui.as_ref().and_then(|ui| ui.scroll_hit(self.cursor)) {
                // A finger back on the content catches it: the fling stops where it
                // is rather than sliding under the finger, and hands the next
                // release whatever momentum the platform lets a swipe build on.
                let physics = area.physics_or(self.app.scroll_physics());
                let carried = self.runtime.catch_scroll_fling(area.id, physics);
                // The offset belongs to the finger until it lifts.
                self.runtime.hold_scroll(area.id);
                self.drag = Some(Drag::Scroll {
                    id: area.id,
                    last: self.cursor,
                    moved: false,
                    carried,
                    dismiss: self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.dismissable_at(self.cursor)),
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
        // A touch scroll or a pan that never moved is a plain tap: we let it follow
        // the ordinary click path below.
        let was_tap = matches!(
            ended,
            Some(Drag::Scroll { moved: false, .. })
                | Some(Drag::Pan { moved: false, .. })
                | Some(Drag::Reorder { moved: false, .. })
                | Some(Drag::Item { moved: false, .. })
        );
        // Reordering: on the drop, the target column is the reorderable header under
        // the pointer, and we route the grabbed header's `on_reorder(from, to)`.
        if let Some(Drag::Reorder {
            id,
            from,
            moved: true,
            ..
        }) = &ended
        {
            let target = self
                .ui
                .as_ref()
                .and_then(|ui| ui.reorderable_at(self.cursor));
            let tree = self.tree.as_ref();
            let base = target
                .and_then(|tid| tree.and_then(|t| find_widget(t.as_ref(), tid)))
                .and_then(|widget| widget.reorder_index());
            // The **lower** half of a hovered vertical target means inserting **after**
            // it, index +1: the effective drop slot follows the insertion line that was
            // painted. No effect horizontally, for `Table` columns, or off target.
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
                let axis = tree
                    .and_then(|t| find_widget(t.as_ref(), *id))
                    .map(|w| w.reorder_axis())
                    .unwrap_or(ReorderAxis::Horizontal);
                self.dispatch(message);
                // The move is spoken to the screen reader — the ghost's counterpart for
                // a blind user. The index depends on the axis: horizontally `to` is the
                // **column position**, 1-based; vertically it is a **flat** index
                // (col×STRIDE+pos) that means nothing read aloud, so we announce the move
                // without a number.
                let announcement = match axis {
                    ReorderAxis::Horizontal => format!("Column moved to position {}", to + 1),
                    ReorderAxis::Vertical => "Card moved".to_string(),
                };
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
            ..
        }) = &ended
        {
            // In **scroll space**: the content moves opposite the finger.
            let estimate = self.gesture_estimate();
            let velocity = self.fling_velocity(estimate);
            self.fling(*id, (-velocity.0 + carried.0, -velocity.1 + carried.1));
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

    /// Runs a command: each task goes off in the background — a native thread, or a
    /// `spawn_local` microtask on the Web — and whatever message it produces comes back
    /// into the loop through the proxy. Focus requests are **queued**, then resolved
    /// against the freshly built tree on the next frame.
    ///
    /// The **asynchronous** tasks ([`Command::perform_async`]) are driven **by the
    /// browser** on the Web (`spawn_local`, so a real `fetch` can `await`) and **driven
    /// to completion** on a thread natively (`block_on`).
    fn run_command(&mut self, command: crate::command::Command<A::Message>) {
        let (tasks, async_tasks, focus) = command.into_parts();
        self.pending_focus.extend(focus);
        for task in tasks {
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
        for future in async_tasks {
            let proxy = self.proxy.clone();
            // Native: the future is driven to completion on its own thread
            // (`block_on`). Self-contained futures need no reactor; real network I/O
            // leans on the application's own async runtime.
            #[cfg(not(web))]
            std::thread::spawn(move || {
                if let Some(message) = pollster::block_on(future) {
                    let _ = proxy.send_event(message);
                }
            });
            // Web: the browser drives the future, single-threaded — `fetch` and friends
            // genuinely `await`, without blocking the loop.
            #[cfg(web)]
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(message) = future.await {
                    let _ = proxy.send_event(message);
                }
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

    /// Starts a subscription on a background thread and returns its cancellation
    /// handle; dropping the `Sender` makes the thread exit at its next wake-up.
    #[cfg(not(web))]
    fn start_subscription(&self, kind: crate::subscription::Kind<A::Message>) -> SubHandle {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let proxy = self.proxy.clone();
        match kind {
            crate::subscription::Kind::Every { interval, make } => {
                // The interval elapsing is a timeout; anything else — the Sender
                // dropped, the loop closed — ends the thread.
                std::thread::spawn(move || {
                    while let Err(RecvTimeoutError::Timeout) = rx.recv_timeout(interval) {
                        if proxy.send_event(make(Instant::now())).is_err() {
                            break;
                        }
                    }
                });
            }
        }
        tx
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
            } => {
                let along = if *vertical {
                    self.cursor.y
                } else {
                    self.cursor.x
                };
                let travel = (*track_len - *thumb_len).max(1.0);
                let thumb_start = (along - *grab).clamp(*track_start, *track_start + travel);
                let offset = ((thumb_start - *track_start) / travel * *max).clamp(0.0, *max);
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
                carried: _,
                dismiss,
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
                            let (nx, rx) = axis(area.metrics_x(cur.0), -dx);
                            let (ny, ry) = axis(area.metrics_y(cur.1), -dy);
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
                            let edge = frus_widgets::edge_for(vertical, refused);
                            if area.refresh.is_some() && edge == frus_widgets::GlowEdge::Top {
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
    fn reorderable_at(&self, point: Point) -> Option<(WidgetId, usize)> {
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
        Some((id, from))
    }

    /// The axis of the reorderable currently **grabbed**, if a drag is under way. It
    /// is what keeps the **horizontal spring** (`reorder_x`) to `Table`'s columns; the
    /// Kanban cards, being vertical, reflow with no x smoothing.
    fn dragged_reorder_axis(&self) -> Option<ReorderAxis> {
        let Some(Drag::Reorder { id, .. }) = self.drag else {
            return None;
        };
        self.tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(|w| w.reorder_axis())
    }

    /// Paints the **reorder preview** on top of the scene, unclipped: the source
    /// column dimmed, a **drop indicator** at the target column's insertion edge, and a
    /// **lifted card** — shadow plus a `primary` border — following the pointer. No
    /// effect outside an engaged header drag.
    fn paint_reorder_preview(&self, ui: &Ui<A::Message>, theme: &Theme, scene: &mut Scene) {
        let Some(Drag::Reorder {
            id,
            from: _,
            start,
            moved: true,
        }) = self.drag
        else {
            return;
        };
        let Some(src) = ui.widget_rect(id) else {
            return;
        };
        let axis = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(|w| w.reorder_axis())
            .unwrap_or(ReorderAxis::Horizontal);
        let dx = self.cursor.x - start.x;
        let dy = self.cursor.y - start.y;

        // The ghost's offset, per axis: **horizontal** (Table columns) follows `dx`,
        // with a slight `-2` lift; **vertical** (Kanban cards) follows the pointer in 2D.
        let (gx, gy) = match axis {
            ReorderAxis::Horizontal => (dx, drag_preview::LIFT_Y),
            ReorderAxis::Vertical => (dx, dy),
        };

        // The owners of the grabbed item's **subtree**: used by the ghost, to capture a
        // rich card's content, which its children paint (milestone 251), **and** by the
        // vertical reflow, to lift the card out of the preview. Fallback: the card alone.
        let owners: std::collections::HashSet<u64> = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(|w| subtree_ids(w, id).iter().map(|i| i.as_u64()).collect())
            .unwrap_or_else(|| std::iter::once(id.as_u64()).collect());

        match axis {
            ReorderAxis::Horizontal => {
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
            ReorderAxis::Vertical => {
                // Reflow the **cards**: the lifted card's gap closes in the source
                // column and a slot opens under the **insertion line** in the target one.
                // Then the line is laid on top, at the chosen edge — the hovered half,
                // milestone 252.
                //
                // A **smoothed** line (milestone 265): the chosen slot's width, abscissa
                // and thickness are kept, but the **ordinate** is replaced by the
                // `reorder_y` spring — the line *and* the gap slide between cards, with
                // vertical inertia, instead of jumping a notch.
                let line = self
                    .reorder_drop_line(drag_preview::INSERT_THICKNESS)
                    .map(|r| Rect {
                        y: self.reorder_y,
                        ..r
                    });
                let reflowed = reflow_reorder_cards(scene.primitives(), src, line, &owners);
                scene.clear();
                for primitive in reflowed {
                    scene.push_primitive(primitive);
                }
                if let Some(line) = line {
                    scene.set_clip(Rect::UNBOUNDED);
                    scene.draw_rect(
                        line,
                        theme.primary,
                        theme.radius.min(line.height * 0.5),
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

    /// The **insertion** line of the vertical preview: a thin band at the edge of the
    /// hovered reorderable slot, a card or a drop zone — the **top** edge when the
    /// pointer is in its upper half (inserting **before**), the **bottom** edge in its
    /// lower half (inserting **after**). `None` when the pointer is not over a target.
    fn reorder_drop_line(&self, thickness: f32) -> Option<Rect> {
        // The reorderable slot — card or drop zone — under the pointer, via its registry.
        let target = self.ui.as_ref()?.reorderable_at(self.cursor)?;
        let rect = self.ui.as_ref()?.widget_rect(target)?;
        Some(drop_insertion_line(
            rect,
            thickness,
            self.reorder_insert_after(target, rect),
        ))
    }

    /// For a **vertically** reordered slot, tells whether the pointer is in its
    /// **lower** half — that is, whether insertion happens **after** it (index +1,
    /// between this card and the next). Always `false` on the **horizontal** axis, where
    /// `Table`'s columns keep their drop logic unchanged. This is the insertion line's
    /// counterpart on the **routing** side.
    fn reorder_insert_after(&self, target: WidgetId, rect: Rect) -> bool {
        let vertical = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), target))
            .map(|w| matches!(w.reorder_axis(), ReorderAxis::Vertical))
            .unwrap_or(false);
        vertical && rect.height > 0.0 && self.cursor.y > rect.y + rect.height * 0.5
    }

    fn apply_widget_drag(&mut self, id: WidgetId, rect: frus_widgets::Rect, dx: f32) {
        let fraction = if rect.width > 0.0 {
            ((self.cursor.x - rect.x) / rect.width).clamp(0.0, 1.0)
        } else {
            0.0
        };
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

    /// Routes a key to the focused field: updates the editing state and applies
    /// whatever message comes out, a value change or a submission.
    fn apply_key(&mut self, id: WidgetId, key: Key) {
        // Any horizontal move, or any keystroke, forgets the vertical goal column.
        self.goal_x = None;
        let mut edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.on_edit(&mut edit, &key));
        self.runtime.edits.insert(id, edit);
        // In a multi-line field, make the retained scroll follow the caret and reveal it.
        self.reveal_caret(id, edit.cursor);
        if let Some(message) = message {
            self.dispatch(message);
            // Keys can arrive in **bursts**, faster than a frame — a software
            // keyboard, `adb input text`, auto-repeat — and the next one must see the
            // CURRENT value, not the retained tree's, or it would overwrite the previous
            // keystroke. So we refresh the tree right away; `build_dirty` stays raised
            // and the next frame redoes the full pass: mounts, leaving fades and all.
            if let Some((width, height)) = self.last_size {
                let theme = self.app.theme();
                self.tree = Some(
                    self.media_query(width, height)
                        .scope(|| self.app.view(&theme, width, height)),
                );
            }
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

/// The geometry of a **vertical** reorder preview's **insertion line**: a thin band
/// of thickness `thickness`, centred on the target edge **where the insertion will
/// happen** — the **top** edge (inserting before, the upper half hovered) or the
/// **bottom** one (inserting after, `after = true`, the lower half hovered) — spanning
/// the full width. A pure function, testable without a GPU.
fn drop_insertion_line(target: Rect, thickness: f32, after: bool) -> Rect {
    let edge = if after {
        target.y + target.height
    } else {
        target.y
    };
    Rect::new(target.x, edge - thickness * 0.5, target.width, thickness)
}

#[cfg(test)]
mod tests {
    use super::{
        draw_ghost_card, drop_insertion_line, fling_velocity, resolve_focus, spring_toward, Rect,
        Scene, Theme, VelocityEstimate, PRECISE_SLOP, TOUCH_SLOP,
    };
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
        let line = drop_insertion_line(Rect::new(20.0, 100.0, 200.0, 44.0), 4.0, false);
        assert_eq!(line.x, 20.0, "aligned to the target's left");
        assert_eq!(line.width, 200.0, "the target's full width");
        assert_eq!(line.y, 98.0, "centred on the top edge (100 - 4/2)");
        assert_eq!(line.height, 4.0);
    }

    #[test]
    fn insertion_line_sits_on_the_target_bottom_edge_when_inserting_after() {
        // The lower half hovered means inserting **after**: the band slides to the
        // **bottom** edge (y = 100 + 44 = 144, centred → 142). Same width, same thickness.
        let line = drop_insertion_line(Rect::new(20.0, 100.0, 200.0, 44.0), 4.0, true);
        assert_eq!(line.x, 20.0, "still aligned to the left");
        assert_eq!(line.width, 200.0, "the target's full width");
        assert_eq!(line.y, 142.0, "centred on the bottom edge (144 - 4/2)");
        assert_eq!(line.height, 4.0);
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
