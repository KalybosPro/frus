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
    Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, Affine,
    DeactivationHandler, Live, Node, NodeId, Rect as AkRect, Role as AkRole, Toggled as AkToggled,
    TreeId, TreeInfo, TreeUpdate,
};
use accesskit_winit::Adapter;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use frus_widgets::{Rect, Role, SemanticsProperties, Toggled, WidgetId};

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
        // AccessKit **does** have one, and this threw it away with a comment saying it
        // did not — true of an older version, false since, and nothing had gone back to
        // look. A heading is how a screen reader's user jumps through a screen; announced
        // as a label it is one more piece of text among the rest.
        Role::Heading => AkRole::Heading,
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
fn to_ak_node(rect: Rect, sem: &SemanticsProperties) -> Node {
    let mut node = Node::new(to_ak_role(sem.role));
    node.set_bounds(AkRect {
        x0: rect.x as f64,
        y0: rect.y as f64,
        x1: (rect.x + rect.width) as f64,
        y1: (rect.y + rect.height) as f64,
    });
    if sem.role == Role::Label {
        // Static text speaks its **value**: that is where AccessKit reads a label node's
        // words from, on every platform — given as a `label`, the text was there and said
        // nothing (milestone 700).
        if let Some(words) = sem.value.as_ref().or(sem.label.as_ref()) {
            node.set_value(words.clone());
        }
    } else {
        if let Some(label) = &sem.label {
            node.set_label(label.clone());
        }
        if let Some(value) = &sem.value {
            node.set_value(value.clone());
        }
    }
    match sem.toggled {
        Toggled::None => {}
        Toggled::False => node.set_toggled(AkToggled::False),
        Toggled::True => node.set_toggled(AkToggled::True),
        Toggled::Mixed => node.set_toggled(AkToggled::Mixed),
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
    if sem.role == Role::TextInput && !sem.disabled {
        // And a field can be **filled**: dictation, a reader's own editing commands, a tool
        // that types for the person (milestone 710). It goes through the message typing
        // would have produced, so a field that refuses typing refuses this too.
        node.add_action(Action::SetValue);
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
    // A label node's words are its value (see `to_ak_node`).
    node.set_value(message.to_string());
    node
}

/// The tree of an interface drawn from the window's corner, as most are.
#[cfg(test)]
pub(crate) fn build_tree_update(
    nodes: &[(WidgetId, Rect, SemanticsProperties)],
    focus: Option<WidgetId>,
    title: &str,
    announce: &str,
    scale: f64,
) -> TreeUpdate {
    build_tree_update_at(nodes, focus, title, announce, scale, (0.0, 0.0))
}

/// Builds a complete AccessKit tree update: a `Window` root whose children are the
/// frame's semantic nodes, paint order being reading order. `focus` is the focused
/// widget, when it is in the tree. A non-empty `announce` adds a **live region** that
/// is spoken aloud.
///
/// The nodes' bounds are the layout's, in **logical** pixels, and the system reads
/// **physical** ones: `scale` goes on the root as a transform, which AccessKit applies to
/// every node under it. Without it a reader's highlight and a magnifier were drawn at two
/// thirds of the size, off their place, on a 150 % screen. `origin` is where the interface's
/// corner is in the window, in physical pixels: the window's own corner, or a simulated
/// device's screen inside its outline (milestone 726), which the root then moves the tree to.
pub(crate) fn build_tree_update_at(
    nodes: &[(WidgetId, Rect, SemanticsProperties)],
    focus: Option<WidgetId>,
    title: &str,
    announce: &str,
    scale: f64,
    origin: (f64, f64),
) -> TreeUpdate {
    let mut updates: Vec<(NodeId, Node)> = Vec::with_capacity(nodes.len() + 2);

    let mut children: Vec<NodeId> = nodes.iter().map(|(id, _, _)| node_id(*id)).collect();
    if !announce.is_empty() {
        children.push(LIVE_ID);
    }

    let mut root = Node::new(AkRole::Window);
    root.set_label(title.to_string());
    root.set_children(children);
    // None at 1, as AccessKit asks of an identity; a scale not yet known (0) is taken as 1
    // rather than collapsing the tree to a point.
    let scale = if scale > 0.0 { scale } else { 1.0 };
    if scale != 1.0 || origin != (0.0, 0.0) {
        root.set_transform(Affine::translate(origin) * Affine::scale(scale));
    }
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
        tree: Some(TreeInfo::new(ROOT_ID)),
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
    nodes: Vec<(WidgetId, Rect, SemanticsProperties)>,
    focus: Option<WidgetId>,
    title: String,
    /// The last announcement message, for the live region. It persists as is:
    /// AccessKit re-speaks it only on a **change**, so unchanged text does not repeat.
    announce: String,
    /// The window's scale factor: logical pixels to physical ones.
    scale: f64,
    /// Where the interface's origin is in the window, in physical pixels.
    origin: (f64, f64),
}

impl Snapshot {
    fn to_update(&self) -> TreeUpdate {
        build_tree_update_at(
            &self.nodes,
            self.focus,
            &self.title,
            &self.announce,
            self.scale,
            self.origin,
        )
    }
}

/// An action the assistive technology asked for — activate or focus a widget — to be
/// replayed in the main loop.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum A11yAction {
    /// Activate the widget, that is, click it.
    Click(WidgetId),
    /// Give the widget focus.
    Focus(WidgetId),
    /// Put this text in the field (milestone 710).
    SetValue(WidgetId, String),
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

/// Turns an AT action into an [`A11yAction`] queued for the loop, and wakes the loop.
///
/// The adapter calls this whenever the assistive technology asks, outside the event loop's
/// own events: an action only queued waited for the next frame something else asked for, so
/// on an idle window a screen reader's press did nothing (milestone 710). A redraw asked of
/// the window brings the loop round, and the queue is drained at its start.
struct ActionForwarder(Actions, Arc<Window>);
impl ActionHandler for ActionForwarder {
    fn do_action(&mut self, request: ActionRequest) {
        let Some(action) = forwarded(request) else {
            return;
        };
        self.0.lock().unwrap().push(action);
        self.1.request_redraw();
    }
}

/// The [`A11yAction`] an AT's request is, if it is one the shell answers.
fn forwarded(request: ActionRequest) -> Option<A11yAction> {
    let id = widget_for(request.target_node)?;
    Some(match (request.action, request.data) {
        (Action::Click, _) => A11yAction::Click(id),
        (Action::Focus, _) => A11yAction::Focus(id),
        (Action::SetValue, Some(ActionData::Value(value))) => {
            A11yAction::SetValue(id, value.to_string())
        }
        _ => return None,
    })
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
    pub(crate) fn new(event_loop: &ActiveEventLoop, window: &Arc<Window>) -> Self {
        let snapshot: Shared = Arc::new(Mutex::new(Snapshot::default()));
        let actions: Actions = Arc::new(Mutex::new(Vec::new()));
        let adapter = Adapter::with_direct_handlers(
            event_loop,
            window,
            Activation(snapshot.clone()),
            ActionForwarder(actions.clone(), window.clone()),
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

    /// Publishes the frame's semantic tree, laid out at `scale` physical pixels to the
    /// logical one, its origin at `origin` physical pixels in the window; the adapter pushes it
    /// only when an AT is running.
    pub(crate) fn update(
        &mut self,
        nodes: &[(WidgetId, Rect, SemanticsProperties)],
        focus: Option<WidgetId>,
        title: &str,
        announce: &str,
        scale: f64,
        origin: (f64, f64),
    ) {
        {
            let mut s = self.snapshot.lock().unwrap();
            s.nodes = nodes.to_vec();
            s.focus = focus;
            s.title = title.to_string();
            s.announce = announce.to_string();
            s.scale = scale;
            s.origin = origin;
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

    fn sem(role: Role) -> SemanticsProperties {
        SemanticsProperties::new(role)
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
        let update = build_tree_update(&nodes, None, "frus", "", 1.0);
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
        let quiet = build_tree_update(&nodes, None, "frus", "", 1.0);
        assert!(
            quiet.nodes.iter().all(|(id, _)| *id != LIVE_ID),
            "no live region"
        );
        // With one: a polite live node carrying the message, a child of the root.
        let loud = build_tree_update(&nodes, None, "frus", "Column moved to position 2", 1.0);
        let live = loud
            .nodes
            .iter()
            .find(|(id, _)| *id == LIVE_ID)
            .expect("the live region is present");
        assert_eq!(live.1.value(), Some("Column moved to position 2"));
        assert_eq!(live.1.label(), None, "a label node speaks its value");
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
        let update = build_tree_update(&nodes, Some(focused), "frus", "", 1.0);
        assert_eq!(update.focus, node_id(focused));
        // A focus outside the tree falls back to the root.
        let absent = wid(999);
        let update2 = build_tree_update(&nodes, Some(absent), "frus", "", 1.0);
        assert_eq!(update2.focus, ROOT_ID);
    }

    #[test]
    fn node_id_round_trips() {
        let id = wid(70);
        assert_eq!(widget_for(node_id(id)), Some(id));
        assert_eq!(widget_for(ROOT_ID), None);
    }

    /// **Static text speaks its value**: AccessKit names a label node from its value, so a
    /// text's words go there — as a `label` they were in the tree and read as nothing. Every
    /// other role keeps its words as its label.
    #[test]
    fn a_text_node_carries_its_words_as_its_value() {
        let rect = Rect::new(0.0, 0.0, 80.0, 20.0);
        let text = to_ak_node(rect, &sem(Role::Label).label("Water the plants"));
        assert_eq!(text.value(), Some("Water the plants"));
        assert_eq!(text.label(), None);
        let button = to_ak_node(rect, &sem(Role::Button).label("Attach…"));
        assert_eq!(button.label(), Some("Attach…"));
        assert_eq!(button.value(), None);
        let heading = to_ak_node(rect, &sem(Role::Heading).label("Task"));
        assert_eq!(heading.label(), Some("Task"));
    }

    /// **An editable field can be filled; nothing else can** (milestone 710): a field offers
    /// set value, a disabled one does not, and a label or a button never does.
    #[test]
    fn an_editable_field_offers_set_value() {
        let rect = Rect::new(0.0, 0.0, 120.0, 30.0);
        let field = to_ak_node(rect, &sem(Role::TextInput).label("Name"));
        assert!(field.supports_action(Action::SetValue));
        let disabled = to_ak_node(rect, &sem(Role::TextInput).disabled(true));
        assert!(!disabled.supports_action(Action::SetValue));
        for role in [Role::Label, Role::Button, Role::CheckBox] {
            assert!(!to_ak_node(rect, &sem(role)).supports_action(Action::SetValue));
        }
    }

    fn request(action: Action, data: Option<ActionData>) -> ActionRequest {
        ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: node_id(WidgetId::from_u64(41)),
            data,
        }
    }

    /// **A request becomes the shell's action**: activate a click, focus a focus, set value
    /// with its text the text — and set value with no text, the root, or an action the shell
    /// does not answer, nothing.
    #[test]
    fn a_request_becomes_the_shells_action() {
        let id = WidgetId::from_u64(41);
        assert_eq!(
            forwarded(request(Action::Click, None)),
            Some(A11yAction::Click(id))
        );
        assert_eq!(
            forwarded(request(Action::Focus, None)),
            Some(A11yAction::Focus(id))
        );
        assert_eq!(
            forwarded(request(
                Action::SetValue,
                Some(ActionData::Value("Ada".into()))
            )),
            Some(A11yAction::SetValue(id, "Ada".into()))
        );
        assert_eq!(forwarded(request(Action::SetValue, None)), None);
        assert_eq!(forwarded(request(Action::ScrollDown, None)), None);
        let mut at_root = request(Action::Click, None);
        at_root.target_node = ROOT_ID;
        assert_eq!(forwarded(at_root), None);
    }

    /// **The tree is handed over in physical pixels**: the layout's logical bounds stay on
    /// the nodes, and the window's scale goes on the root, so a 56 px button at 150 % is
    /// 84 px where the system draws its highlight.
    #[test]
    fn the_root_scales_logical_bounds_to_physical_ones() {
        let nodes = vec![(
            wid(10),
            Rect::new(461.0, 307.5, 56.0, 56.0),
            sem(Role::Button).label("Done"),
        )];
        let root = |update: &TreeUpdate| update.nodes[0].1.clone();
        let button = |update: &TreeUpdate| update.nodes[1].1.clone();

        let scaled = build_tree_update(&nodes, None, "frus", "", 1.5);
        let transform = *root(&scaled).transform().expect("a scale on the root");
        let physical = transform.transform_rect_bbox(button(&scaled).bounds().unwrap());
        assert_eq!(
            (physical.x0, physical.y0, physical.x1, physical.y1),
            (691.5, 461.25, 775.5, 545.25)
        );
        assert_eq!(
            button(&scaled).transform(),
            None,
            "only the root carries it"
        );

        // The snapshot the adapter reads carries the scale it was last given to the tree.
        let snapshot = Snapshot {
            nodes: nodes.clone(),
            scale: 1.5,
            ..Snapshot::default()
        };
        assert_eq!(
            root(&snapshot.to_update()).transform(),
            Some(&Affine::scale(1.5))
        );

        // At 1 — and before the scale is known — no transform: the identity is left unset.
        for scale in [1.0, 0.0] {
            let update = build_tree_update(&nodes, None, "frus", "", scale);
            assert_eq!(root(&update).transform(), None, "scale {scale}");
        }
    }

    /// **A simulated device's screen is where the system looks** (milestone 726): its
    /// interface is drawn from the screen's corner, inside the device's outline, so the root
    /// carries that move as well as the scale, and a reader's highlight lands on the button.
    #[test]
    fn the_root_moves_the_tree_to_a_simulated_screen() {
        let nodes = vec![(
            wid(10),
            Rect::new(20.0, 40.0, 56.0, 56.0),
            sem(Role::Button).label("Done"),
        )];
        let update = build_tree_update_at(&nodes, None, "frus", "", 2.0, (607.0, 148.0));
        let transform = *update.nodes[0]
            .1
            .transform()
            .expect("a transform on the root");
        let physical = transform.transform_rect_bbox(update.nodes[1].1.bounds().unwrap());
        assert_eq!(
            (physical.x0, physical.y0, physical.x1, physical.y1),
            (647.0, 228.0, 759.0, 340.0)
        );
        // A move alone is still a transform.
        let moved = build_tree_update_at(&nodes, None, "frus", "", 1.0, (10.0, 0.0));
        assert!(moved.nodes[0].1.transform().is_some());
    }
}
