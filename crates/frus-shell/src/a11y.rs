//! The **accessibility** bridge: maps frus's semantic tree
//! ([`frus_widgets::Ui::semantics`]) onto an [`accesskit::TreeUpdate`], which the
//! `accesskit_winit` adapter pushes to the native screen readers — UIA on Windows,
//! AT-SPI on Linux, macOS.
//!
//! Nothing is reinvented here: frus annotates — role, label, value, state — and
//! AccessKit talks to the assistive technologies. Desktop only; AccessKit's Android
//! provider is a separate piece of work.

use std::sync::{Arc, Mutex};

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Live, Node,
    NodeId, Rect as AkRect, Role as AkRole, Toggled as AkToggled, Tree, TreeId, TreeUpdate,
};
use accesskit_winit::Adapter;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use frus_widgets::{Rect, Role, Semantics, Toggled, WidgetId};

/// The identity of the **root** node, the window, in the AccessKit tree.
const ROOT_ID: NodeId = NodeId(0);

/// The identity of the **live region** node, used for spoken announcements. A
/// reserved id, out of [`WidgetId`] range: those are shifted by `+1` and so are
/// bounded by `u64::MAX - 1`.
const LIVE_ID: NodeId = NodeId(u64::MAX);

/// Converts a frus role into an AccessKit role.
fn to_ak_role(role: Role) -> AkRole {
    match role {
        Role::None => AkRole::GenericContainer,
        Role::Label => AkRole::Label,
        Role::Heading => AkRole::Label, // no distinct Heading role is used here
        Role::Button => AkRole::Button,
        Role::Link => AkRole::Link,
        Role::CheckBox => AkRole::CheckBox,
        Role::Switch => AkRole::Switch,
        Role::RadioButton => AkRole::RadioButton,
        Role::Slider => AkRole::Slider,
        Role::TextInput => AkRole::TextInput,
        Role::Image => AkRole::Image,
        Role::Tab => AkRole::Tab,
        Role::ListItem => AkRole::ListItem,
        Role::ProgressBar => AkRole::ProgressIndicator,
    }
}

/// A widget's AccessKit identity; id `0` is reserved for the root.
fn node_id(id: WidgetId) -> NodeId {
    NodeId(id.as_u64().wrapping_add(1))
}

/// Builds an AccessKit node from a frus annotation and its bounds.
fn to_ak_node(rect: Rect, sem: &Semantics) -> Node {
    let mut node = Node::new(to_ak_role(sem.role));
    node.set_bounds(AkRect {
        x0: rect.x as f64,
        y0: rect.y as f64,
        x1: (rect.x + rect.width) as f64,
        y1: (rect.y + rect.height) as f64,
    });
    if let Some(label) = &sem.label {
        node.set_label(label.clone());
    }
    if let Some(value) = &sem.value {
        node.set_value(value.clone());
    }
    match sem.toggled {
        Toggled::None => {}
        Toggled::False => node.set_toggled(AkToggled::False),
        Toggled::True => node.set_toggled(AkToggled::True),
    }
    if let Some((min, value, max)) = sem.range {
        node.set_numeric_value(value as f64);
        node.set_min_numeric_value(min as f64);
        node.set_max_numeric_value(max as f64);
    }
    if sem.clickable {
        // The screen reader may offer "activate"; the action comes back to the
        // shell, which maps it to a synthetic click on the widget.
        node.add_action(Action::Click);
    }
    node.add_action(Action::Focus);
    node
}

/// Builds the **live region** node: an invisible, polite status — it does not
/// interrupt whatever is being read — whose text the assistive technology speaks
/// **when it changes**, the same contract as `aria-live="polite"`.
fn live_node(message: &str) -> Node {
    let mut node = Node::new(AkRole::Label);
    node.set_live(Live::Polite);
    node.set_label(message.to_string());
    node
}

/// Builds a complete AccessKit tree update: a `Window` root whose children are the
/// frame's semantic nodes, paint order being reading order. `focus` is the focused
/// widget, when it is in the tree. A non-empty `announce` adds a **live region** that
/// is spoken aloud.
pub(crate) fn build_tree_update(
    nodes: &[(WidgetId, Rect, Semantics)],
    focus: Option<WidgetId>,
    title: &str,
    announce: &str,
) -> TreeUpdate {
    let mut updates: Vec<(NodeId, Node)> = Vec::with_capacity(nodes.len() + 2);

    let mut children: Vec<NodeId> = nodes.iter().map(|(id, _, _)| node_id(*id)).collect();
    if !announce.is_empty() {
        children.push(LIVE_ID);
    }

    let mut root = Node::new(AkRole::Window);
    root.set_label(title.to_string());
    root.set_children(children);
    updates.push((ROOT_ID, root));

    for (id, rect, sem) in nodes {
        updates.push((node_id(*id), to_ak_node(*rect, sem)));
    }
    if !announce.is_empty() {
        updates.push((LIVE_ID, live_node(announce)));
    }

    // AccessKit's focus must name a node **present** in the tree; failing that we
    // focus the root, which means nothing in particular.
    let focus = focus
        .filter(|f| nodes.iter().any(|(id, _, _)| id == f))
        .map(node_id)
        .unwrap_or(ROOT_ID);

    TreeUpdate {
        nodes: updates,
        tree: Some(Tree::new(ROOT_ID)),
        tree_id: TreeId::ROOT,
        focus,
    }
}

/// Finds the frus [`WidgetId`] behind an AccessKit action on `node` — the inverse of
/// [`node_id`] — so a click or focus the AT asked for can be routed.
pub(crate) fn widget_for(node: NodeId) -> Option<WidgetId> {
    (node != ROOT_ID).then(|| WidgetId::from_u64(node.0.wrapping_sub(1)))
}

/// The semantic snapshot shared between the main loop, which writes it every frame,
/// and the AccessKit adapter, which reads it on its own thread.
#[derive(Default)]
struct Snapshot {
    nodes: Vec<(WidgetId, Rect, Semantics)>,
    focus: Option<WidgetId>,
    title: String,
    /// The last announcement message, for the live region. It persists as is:
    /// AccessKit re-speaks it only on a **change**, so unchanged text does not repeat.
    announce: String,
}

impl Snapshot {
    fn to_update(&self) -> TreeUpdate {
        build_tree_update(&self.nodes, self.focus, &self.title, &self.announce)
    }
}

/// An action the assistive technology asked for — activate or focus a widget — to be
/// replayed in the main loop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum A11yAction {
    /// Activate the widget, that is, click it.
    Click(WidgetId),
    /// Give the widget focus.
    Focus(WidgetId),
}

type Shared = Arc<Mutex<Snapshot>>;
type Actions = Arc<Mutex<Vec<A11yAction>>>;

/// Supplies the initial tree when the AT wakes up, from the shared snapshot.
struct Activation(Shared);
impl ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().to_update())
    }
}

/// Turns an AT action into an [`A11yAction`] queued for the loop.
struct ActionForwarder(Actions);
impl ActionHandler for ActionForwarder {
    fn do_action(&mut self, request: ActionRequest) {
        let Some(id) = widget_for(request.target_node) else {
            return;
        };
        let action = match request.action {
            Action::Click => A11yAction::Click(id),
            Action::Focus => A11yAction::Focus(id),
            _ => return,
        };
        self.0.lock().unwrap().push(action);
    }
}

/// Nothing to release: the snapshot rebuilds itself on the next frame.
struct Deactivation;
impl DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {}
}

/// A window's **live** accessibility bridge: the `accesskit_winit` adapter, the
/// shared snapshot and the action queue. Desktop only.
pub(crate) struct A11y {
    adapter: Adapter,
    snapshot: Shared,
    actions: Actions,
}

impl A11y {
    /// Creates the bridge for a window. With no screen reader running the adapter
    /// stays inert: no cost, no crash.
    pub(crate) fn new(event_loop: &ActiveEventLoop, window: &Window) -> Self {
        let snapshot: Shared = Arc::new(Mutex::new(Snapshot::default()));
        let actions: Actions = Arc::new(Mutex::new(Vec::new()));
        let adapter = Adapter::with_direct_handlers(
            event_loop,
            window,
            Activation(snapshot.clone()),
            ActionForwarder(actions.clone()),
            Deactivation,
        );
        Self {
            adapter,
            snapshot,
            actions,
        }
    }

    /// Forwards a window event to the adapter: focus, a move, and so on.
    pub(crate) fn process_event(&mut self, window: &Window, event: &WindowEvent) {
        self.adapter.process_event(window, event);
    }

    /// Publishes the frame's semantic tree; the adapter pushes it only when an AT is
    /// running.
    pub(crate) fn update(
        &mut self,
        nodes: &[(WidgetId, Rect, Semantics)],
        focus: Option<WidgetId>,
        title: &str,
        announce: &str,
    ) {
        {
            let mut s = self.snapshot.lock().unwrap();
            s.nodes = nodes.to_vec();
            s.focus = focus;
            s.title = title.to_string();
            s.announce = announce.to_string();
        }
        let snapshot = self.snapshot.clone();
        self.adapter
            .update_if_active(|| snapshot.lock().unwrap().to_update());
    }

    /// Takes the actions the AT asked for since the last frame, emptying the queue.
    pub(crate) fn take_actions(&self) -> Vec<A11yAction> {
        std::mem::take(&mut self.actions.lock().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sem(role: Role) -> Semantics {
        Semantics::new(role)
    }

    fn wid(raw: u64) -> WidgetId {
        WidgetId::from_u64(raw)
    }

    #[test]
    fn roles_map_to_accesskit() {
        assert_eq!(to_ak_role(Role::Button), AkRole::Button);
        assert_eq!(to_ak_role(Role::CheckBox), AkRole::CheckBox);
        assert_eq!(to_ak_role(Role::Slider), AkRole::Slider);
        assert_eq!(to_ak_role(Role::ProgressBar), AkRole::ProgressIndicator);
    }

    #[test]
    fn tree_update_has_root_plus_children() {
        let nodes = vec![
            (
                wid(10),
                Rect::new(0.0, 0.0, 40.0, 20.0),
                sem(Role::Button).label("Ok"),
            ),
            (
                wid(20),
                Rect::new(0.0, 30.0, 40.0, 20.0),
                sem(Role::CheckBox).toggled(true),
            ),
        ];
        let update = build_tree_update(&nodes, None, "frus", "");
        // The root plus two nodes.
        assert_eq!(update.nodes.len(), 3);
        assert_eq!(update.nodes[0].0, ROOT_ID);
        // The root references both children.
        // (the ids are shifted by +1 relative to the WidgetIds).
        assert_eq!(update.focus, ROOT_ID, "no focus → the root");
    }

    #[test]
    fn announcement_adds_a_polite_live_region() {
        let nodes = vec![(wid(10), Rect::new(0.0, 0.0, 40.0, 20.0), sem(Role::Button))];
        // With no announcement there is no live node.
        let quiet = build_tree_update(&nodes, None, "frus", "");
        assert!(
            quiet.nodes.iter().all(|(id, _)| *id != LIVE_ID),
            "no live region"
        );
        // With one: a polite live node carrying the message, a child of the root.
        let loud = build_tree_update(&nodes, None, "frus", "Column moved to position 2");
        let live = loud
            .nodes
            .iter()
            .find(|(id, _)| *id == LIVE_ID)
            .expect("the live region is present");
        assert_eq!(
            live.1.label().as_deref(),
            Some("Column moved to position 2")
        );
        assert_eq!(live.1.live(), Some(Live::Polite));
        let (_, root) = &loud.nodes[0];
        assert!(
            root.children().contains(&LIVE_ID),
            "the root references the live region"
        );
    }

    #[test]
    fn focus_points_at_a_present_node() {
        let focused = wid(10);
        let nodes = vec![(focused, Rect::new(0.0, 0.0, 40.0, 20.0), sem(Role::Button))];
        let update = build_tree_update(&nodes, Some(focused), "frus", "");
        assert_eq!(update.focus, node_id(focused));
        // A focus outside the tree falls back to the root.
        let absent = wid(999);
        let update2 = build_tree_update(&nodes, Some(absent), "frus", "");
        assert_eq!(update2.focus, ROOT_ID);
    }

    #[test]
    fn node_id_round_trips() {
        let id = wid(70);
        assert_eq!(widget_for(node_id(id)), Some(id));
        assert_eq!(widget_for(ROOT_ID), None);
    }
}
