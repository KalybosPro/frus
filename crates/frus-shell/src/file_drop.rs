//! **Files dragged onto the window** from outside the application (milestone 695).
//!
//! The system tells the window three things: files are being held over it (one event
//! per file), they went away again, and they were dropped (one event per file again).
//! What it does not reliably say is **where**: the windowing layer reports none of
//! these with a position, and while the system runs the drag the window is sent no
//! pointer movement either. So the position is asked of the system where it can be —
//! on Windows, the cursor's position read directly — and, where it cannot, the drop
//! goes to the one target that takes the files when there is only one, and otherwise to
//! the target under the last position the pointer was seen at.
//!
//! The answer is the same [`frus_widgets::DragTarget`] that an item carried from inside
//! the application lands on: it lights up with `Status::drag_over` while the files are
//! over it, and is handed their paths on the drop.

use std::path::PathBuf;

use frus_widgets::{find_widget, Point, Ui, Widget, WidgetId};

/// Files from outside the application, held over the window or just dropped on it.
#[derive(Debug, Default)]
pub(crate) struct FileDrag {
    /// Every file in the drag, in the order the system listed them.
    pub paths: Vec<PathBuf>,
    /// The target the files would land on now, when one takes them.
    pub over: Option<WidgetId>,
    /// The drop has begun. The system names the files one event at a time, so they are
    /// handed over together once the loop has run out of events — see
    /// `App::deliver_dropped_files`.
    pub dropped: bool,
}

impl FileDrag {
    /// Adds a file the system named for a drag — or for a drop, which starts a fresh
    /// list: the paths dropped are the ones that count, whatever was hovered before.
    pub fn add(&mut self, path: PathBuf, dropping: bool) {
        if dropping && !self.dropped {
            self.dropped = true;
            self.paths.clear();
        }
        if !self.paths.contains(&path) {
            self.paths.push(path);
        }
    }
}

/// The drop target `paths` would land on: the topmost target under `at` if it takes
/// them, when the position is known. When it is not, the one target of the frame that
/// takes them if there is exactly one, and otherwise the topmost under `last`, the last
/// place the pointer was seen.
///
/// A target on top that refuses the files hides the ones beneath it, as it does for an
/// item from inside: what is under the pointer is what answers.
pub(crate) fn target<Msg: Clone>(
    ui: &Ui<Msg>,
    tree: &dyn Widget<Msg>,
    paths: &[PathBuf],
    at: Option<Point>,
    last: Point,
) -> Option<WidgetId> {
    let takes = |id: WidgetId| find_widget(tree, id).is_some_and(|w| w.accepts_files(paths));
    let under = |point: Point| {
        ui.drop_zone_at(point)
            .map(|zone| zone.id)
            .filter(|id| takes(*id))
    };
    if let Some(at) = at {
        return under(at);
    }
    let mut taking = ui
        .drop_zones()
        .iter()
        .map(|zone| zone.id)
        .filter(|id| takes(*id));
    match (taking.next(), taking.next()) {
        (Some(only), None) => Some(only),
        _ => under(last),
    }
}

/// Where the system's cursor is, in the window's logical coordinates, read directly:
/// while the system runs a drag the window hears no pointer movement.
#[cfg(windows)]
pub(crate) fn cursor_in(window: &winit::window::Window, scale: f32) -> Option<Point> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let hwnd = match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(win) => win.hwnd.get() as HWND,
        _ => return None,
    };
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: `point` is a valid out-parameter and the window is alive.
    let ok = unsafe { GetCursorPos(&mut point) != 0 && ScreenToClient(hwnd, &mut point) != 0 };
    ok.then(|| Point::new(point.x as f32 / scale, point.y as f32 / scale))
}

/// Elsewhere the window cannot say where the cursor is during the drag.
#[cfg(not(windows))]
pub(crate) fn cursor_in(_window: &winit::window::Window, _scale: f32) -> Option<Point> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::{build_ui, Container, DragTarget, Flex, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Files(Vec<PathBuf>),
    }

    fn txt(paths: &[PathBuf]) -> bool {
        paths
            .iter()
            .all(|p| p.extension().is_some_and(|e| e == "txt"))
    }

    /// Two targets one above the other, 100 px each: the top takes anything, the
    /// bottom only text files.
    fn two() -> Flex<Msg> {
        Flex::column()
            .width(200.0)
            .child(
                DragTarget::new(Container::new().width(200.0).height(100.0)).on_files(Msg::Files),
            )
            .child(
                DragTarget::new(Container::new().width(200.0).height(100.0))
                    .on_files(Msg::Files)
                    .accepts_files(txt),
            )
    }

    fn ids(ui: &Ui<Msg>) -> (WidgetId, WidgetId) {
        let zones = ui.drop_zones();
        (zones[0].id, zones[1].id)
    }

    #[test]
    fn a_known_position_picks_the_target_under_it() {
        let runtime = Runtime::default();
        let tree = two();
        let ui = build_ui(&tree, Size::new(200.0, 300.0), &runtime, &Theme::dark());
        let (top, bottom) = ids(&ui);
        let paths = [PathBuf::from("a.txt")];
        let far = Point::new(0.0, 0.0);
        assert_eq!(
            target(&ui, &tree, &paths, Some(Point::new(50.0, 50.0)), far),
            Some(top)
        );
        assert_eq!(
            target(&ui, &tree, &paths, Some(Point::new(50.0, 150.0)), far),
            Some(bottom)
        );
        // Below both, nothing — even though the last pointer was over one.
        assert_eq!(
            target(&ui, &tree, &paths, Some(Point::new(50.0, 250.0)), far),
            None
        );
    }

    #[test]
    fn a_target_that_refuses_is_not_offered_the_files() {
        let runtime = Runtime::default();
        let tree = two();
        let ui = build_ui(&tree, Size::new(200.0, 300.0), &runtime, &Theme::dark());
        let paths = [PathBuf::from("a.png")];
        let at = Some(Point::new(50.0, 150.0));
        assert_eq!(target(&ui, &tree, &paths, at, Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn with_no_position_the_only_taker_takes_them() {
        let runtime = Runtime::default();
        let tree = two();
        let ui = build_ui(&tree, Size::new(200.0, 300.0), &runtime, &Theme::dark());
        let (top, bottom) = ids(&ui);
        // A picture: only the top target takes it, wherever the pointer was last seen.
        let picture = [PathBuf::from("a.png")];
        assert_eq!(
            target(&ui, &tree, &picture, None, Point::new(50.0, 150.0)),
            Some(top)
        );
        // Text: both take it, so the last pointer position decides.
        let text = [PathBuf::from("a.txt")];
        assert_eq!(
            target(&ui, &tree, &text, None, Point::new(50.0, 150.0)),
            Some(bottom)
        );
        assert_eq!(
            target(&ui, &tree, &text, None, Point::new(50.0, 250.0)),
            None
        );
    }

    #[test]
    fn a_drop_replaces_what_was_hovered_and_names_each_file_once() {
        let mut drag = FileDrag::default();
        drag.add(PathBuf::from("a"), false);
        drag.add(PathBuf::from("b"), false);
        drag.add(PathBuf::from("a"), false);
        assert_eq!(drag.paths, [PathBuf::from("a"), PathBuf::from("b")]);
        drag.add(PathBuf::from("c"), true);
        drag.add(PathBuf::from("d"), true);
        assert!(drag.dropped);
        assert_eq!(drag.paths, [PathBuf::from("c"), PathBuf::from("d")]);
    }
}
