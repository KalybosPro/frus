//! [`AnimationHandle`]: an [`AnimationController`] that the frame clock drives
//! (milestone 716).
//!
//! An animation an application drives itself — a spinner, a progress that follows a
//! gesture, a sequence started by a button — used to be three hooks in a function
//! component: a `use_ref` holding the controller, a `use_interval` ticking it at a guessed
//! frame rate, and a `use_state` mirroring its value so that the component rebuilt. The
//! reference gives a controller a ticker that the frames drive. This is that: a handle
//! whose controller is advanced by every frame while it moves, and whose movement asks for
//! the rebuild that shows it. [`BuildContext::use_animation_controller`] keeps one for the
//! life of a component; [`AnimationHandle::new`] makes one for a state to hold.
//!
//! [`BuildContext::use_animation_controller`]: crate::BuildContext::use_animation_controller

use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

use frus_core::{AnimationController, Curve};

use crate::component::request_rebuild;

/// A shared [`AnimationController`], advanced by the frames while it moves.
///
/// Cloning gives another handle to the same controller. Starting it — [`forward`],
/// [`reverse`], [`repeat`], [`animate_to`] — is all there is to do: each frame then moves
/// it and rebuilds what reads [`value`], until it stops. It stops being driven once every
/// handle to it is gone.
///
/// ```
/// use frus_widgets::{component, Button, Column, Curve, Opacity, Text, Widget};
///
/// let fading = component(|cx| {
///     let fade = cx.use_animation_controller(frus_widgets::AnimationController::unit);
///     let start = fade.clone();
///     Box::new(
///         Column::new()
///             .child(Button::new("Fade in").on_press(move || start.forward(0.4, Curve::ease_out())))
///             .child(Opacity::new(fade.value(), Text::new("Hello"))),
///     ) as Box<dyn Widget>
/// });
/// ```
///
/// [`forward`]: AnimationHandle::forward
/// [`reverse`]: AnimationHandle::reverse
/// [`repeat`]: AnimationHandle::repeat
/// [`animate_to`]: AnimationHandle::animate_to
/// [`value`]: AnimationHandle::value
#[derive(Clone)]
pub struct AnimationHandle {
    controller: Rc<RefCell<AnimationController>>,
}

impl AnimationHandle {
    /// A handle to `controller`, driven by the frames whenever it moves.
    pub fn new(controller: AnimationController) -> Self {
        Self {
            controller: Rc::new(RefCell::new(controller)),
        }
    }

    pub(crate) fn from_shared(controller: Rc<RefCell<AnimationController>>) -> Self {
        Self { controller }
    }

    /// Where it is now.
    pub fn value(&self) -> f32 {
        self.controller.borrow().value()
    }

    /// Whether it is moving.
    pub fn is_animating(&self) -> bool {
        self.controller.borrow().is_animating()
    }

    /// Runs to the upper bound over `duration` seconds.
    pub fn forward(&self, duration: f32, curve: Curve) {
        self.change(|c| c.forward(duration, curve));
    }

    /// Runs to the lower bound over `duration` seconds.
    pub fn reverse(&self, duration: f32, curve: Curve) {
        self.change(|c| c.reverse(duration, curve));
    }

    /// Runs to `target` over `duration` seconds.
    pub fn animate_to(&self, target: f32, duration: f32, curve: Curve) {
        self.change(|c| c.animate_to(target, duration, curve));
    }

    /// Runs from bound to bound over `period` seconds, for ever — back and forth when
    /// `reverse`, starting again from the lower bound otherwise.
    pub fn repeat(&self, period: f32, reverse: bool, curve: Curve) {
        self.change(|c| c.repeat(period, reverse, curve));
    }

    /// Stops where it is.
    pub fn stop(&self) {
        self.change(AnimationController::stop);
    }

    /// Jumps to `value`, stopping any movement.
    pub fn set_value(&self, value: f32) {
        self.change(|c| c.set_value(value));
    }

    /// Reads the controller.
    pub fn with<R>(&self, read: impl FnOnce(&AnimationController) -> R) -> R {
        read(&self.controller.borrow())
    }

    /// Changes the controller in any way it allows, and drives it if that set it moving.
    pub fn update(&self, change: impl FnOnce(&mut AnimationController)) {
        self.change(change);
    }

    fn change(&self, change: impl FnOnce(&mut AnimationController)) {
        change(&mut self.controller.borrow_mut());
        if self.controller.borrow().is_animating() {
            drive(Rc::downgrade(&self.controller));
        }
        request_rebuild();
    }
}

impl fmt::Debug for AnimationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimationHandle")
            .field("value", &self.value())
            .field("animating", &self.is_animating())
            .finish()
    }
}

thread_local! {
    /// The controllers moving on this thread.
    static DRIVEN: RefCell<Vec<Weak<RefCell<AnimationController>>>> = const { RefCell::new(Vec::new()) };
}

/// Adds a controller to the ones the frames move, once.
#[inline(never)]
fn drive(controller: Weak<RefCell<AnimationController>>) {
    DRIVEN.with(|driven| {
        let mut driven = driven.borrow_mut();
        if !driven.iter().any(|known| known.ptr_eq(&controller)) {
            driven.push(controller);
        }
    });
}

/// Moves every controller started through an [`AnimationHandle`] on by `dt` seconds;
/// whether one still moves, so that another frame is due. The shell calls it every frame,
/// and a movement asks for the rebuild that shows it.
#[inline(never)]
pub fn tick_animations(dt: f32) -> bool {
    let driven = DRIVEN.with(|driven| std::mem::take(&mut *driven.borrow_mut()));
    if driven.is_empty() {
        return false;
    }
    let mut still = Vec::new();
    for controller in driven {
        let Some(shared) = controller.upgrade() else {
            continue;
        };
        if shared.borrow_mut().tick(dt) {
            still.push(controller);
        }
    }
    request_rebuild();
    let moving = !still.is_empty();
    DRIVEN.with(|driven| {
        let mut driven = driven.borrow_mut();
        // A controller started during the ticks above is already back in the list.
        for controller in still {
            if !driven.iter().any(|known| known.ptr_eq(&controller)) {
                driven.push(controller);
            }
        }
    });
    moving
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frames_move_a_started_controller_to_its_end_and_then_stop() {
        let fade = AnimationHandle::new(AnimationController::unit());
        fade.forward(0.2, Curve::Linear);
        assert!(tick_animations(0.1), "half way, it still moves");
        assert!((fade.value() - 0.5).abs() < 0.01);
        assert!(!tick_animations(0.15), "past its end, nothing moves");
        assert_eq!(fade.value(), 1.0);
        assert!(!tick_animations(0.1), "and it is no longer driven");
    }

    #[test]
    fn a_controller_nobody_holds_is_not_driven() {
        let fade = AnimationHandle::new(AnimationController::unit());
        fade.repeat(1.0, true, Curve::Linear);
        drop(fade);
        assert!(!tick_animations(0.1));
    }

    #[test]
    fn a_change_asks_for_a_rebuild() {
        let fade = AnimationHandle::new(AnimationController::unit());
        let _ = crate::component::take_rebuild_request();
        fade.set_value(0.3);
        assert!(crate::component::take_rebuild_request());
    }
}
