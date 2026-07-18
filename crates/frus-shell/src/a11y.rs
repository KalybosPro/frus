//! Pont **accessibilité** : mappe l'arbre sémantique de frus
//! ([`frus_widgets::Ui::semantics`]) vers un [`accesskit::TreeUpdate`], que
//! l'adaptateur `accesskit_winit` pousse aux lecteurs d'écran natifs (UIA
//! Windows, AT-SPI Linux, macOS).
//!
//! On ne réinvente rien : frus annote (rôle/label/valeur/état), AccessKit
//! parle aux technologies d'assistance. Bureau uniquement (le provider Android
//! d'AccessKit est un chantier distinct).

use std::sync::{Arc, Mutex};

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId,
    Rect as AkRect, Role as AkRole, Toggled as AkToggled, Tree, TreeId, TreeUpdate,
};
use accesskit_winit::Adapter;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use frus_widgets::{Rect, Role, Semantics, Toggled, WidgetId};

/// Identité du nœud **racine** (la fenêtre) dans l'arbre AccessKit.
const ROOT_ID: NodeId = NodeId(0);

/// Convertit un rôle frus en rôle AccessKit.
fn to_ak_role(role: Role) -> AkRole {
    match role {
        Role::None => AkRole::GenericContainer,
        Role::Label => AkRole::Label,
        Role::Heading => AkRole::Label, // pas de rôle Heading distinct utilisé ici
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

/// L'identité AccessKit d'un widget (l'id `0` est réservé à la racine).
fn node_id(id: WidgetId) -> NodeId {
    NodeId(id.as_u64().wrapping_add(1))
}

/// Construit un nœud AccessKit à partir d'une annotation frus et de ses bornes.
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
        // Le lecteur d'écran peut proposer « activer » ; l'action revient au
        // shell (mappée vers un clic synthétique sur le widget).
        node.add_action(Action::Click);
    }
    node.add_action(Action::Focus);
    node
}

/// Construit une mise à jour d'arbre AccessKit complète : une racine `Window`
/// dont les enfants sont les nœuds sémantiques de la frame (ordre de peinture =
/// ordre de lecture). `focus` = le widget focalisé, s'il est dans l'arbre.
pub(crate) fn build_tree_update(
    nodes: &[(WidgetId, Rect, Semantics)],
    focus: Option<WidgetId>,
    title: &str,
) -> TreeUpdate {
    let mut updates: Vec<(NodeId, Node)> = Vec::with_capacity(nodes.len() + 1);

    let mut root = Node::new(AkRole::Window);
    root.set_label(title.to_string());
    root.set_children(nodes.iter().map(|(id, _, _)| node_id(*id)).collect::<Vec<_>>());
    updates.push((ROOT_ID, root));

    for (id, rect, sem) in nodes {
        updates.push((node_id(*id), to_ak_node(*rect, sem)));
    }

    // Le focus AccessKit doit désigner un nœud **présent** dans l'arbre ; sinon
    // on focalise la racine (rien de spécifique).
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

/// Retrouve le [`WidgetId`] frus derrière une action AccessKit sur `node`
/// (inverse de [`node_id`]) — pour router un clic/focus demandé par l'AT.
pub(crate) fn widget_for(node: NodeId) -> Option<WidgetId> {
    (node != ROOT_ID).then(|| WidgetId::from_u64(node.0.wrapping_sub(1)))
}

/// L'instantané sémantique partagé entre la boucle principale (qui l'écrit
/// chaque frame) et l'adaptateur AccessKit (qui le lit sur son propre thread).
#[derive(Default)]
struct Snapshot {
    nodes: Vec<(WidgetId, Rect, Semantics)>,
    focus: Option<WidgetId>,
    title: String,
}

impl Snapshot {
    fn to_update(&self) -> TreeUpdate {
        build_tree_update(&self.nodes, self.focus, &self.title)
    }
}

/// Une action demandée par la technologie d'assistance (activer / focaliser un
/// widget), à rejouer dans la boucle principale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum A11yAction {
    /// Activer (clic) le widget.
    Click(WidgetId),
    /// Donner le focus au widget.
    Focus(WidgetId),
}

type Shared = Arc<Mutex<Snapshot>>;
type Actions = Arc<Mutex<Vec<A11yAction>>>;

/// Fournit l'arbre initial au réveil de l'AT, depuis l'instantané partagé.
struct Activation(Shared);
impl ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().to_update())
    }
}

/// Traduit une action de l'AT en [`A11yAction`] mise en file pour la boucle.
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

/// Rien à libérer : l'instantané se reconstruit à la prochaine frame.
struct Deactivation;
impl DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {}
}

/// Le pont accessibilité **vivant** d'une fenêtre : l'adaptateur `accesskit_winit`
/// + l'instantané partagé + la file d'actions. Bureau uniquement.
pub(crate) struct A11y {
    adapter: Adapter,
    snapshot: Shared,
    actions: Actions,
}

impl A11y {
    /// Crée le pont pour une fenêtre. Sans lecteur d'écran actif, l'adaptateur
    /// reste inerte (aucun coût, aucun crash).
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

    /// Transmet un événement fenêtre à l'adaptateur (focus, déplacement…).
    pub(crate) fn process_event(&mut self, window: &Window, event: &WindowEvent) {
        self.adapter.process_event(window, event);
    }

    /// Publie l'arbre sémantique de la frame ; l'adaptateur ne le pousse que si
    /// une AT est active.
    pub(crate) fn update(
        &mut self,
        nodes: &[(WidgetId, Rect, Semantics)],
        focus: Option<WidgetId>,
        title: &str,
    ) {
        {
            let mut s = self.snapshot.lock().unwrap();
            s.nodes = nodes.to_vec();
            s.focus = focus;
            s.title = title.to_string();
        }
        let snapshot = self.snapshot.clone();
        self.adapter
            .update_if_active(|| snapshot.lock().unwrap().to_update());
    }

    /// Récupère (et vide) les actions demandées par l'AT depuis la dernière frame.
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
            (wid(10), Rect::new(0.0, 0.0, 40.0, 20.0), sem(Role::Button).label("Ok")),
            (
                wid(20),
                Rect::new(0.0, 30.0, 40.0, 20.0),
                sem(Role::CheckBox).toggled(true),
            ),
        ];
        let update = build_tree_update(&nodes, None, "frus");
        // Racine + deux nœuds.
        assert_eq!(update.nodes.len(), 3);
        assert_eq!(update.nodes[0].0, ROOT_ID);
        // La racine référence les deux enfants.
        // (les ids sont décalés de +1 par rapport aux WidgetId).
        assert_eq!(update.focus, ROOT_ID, "aucun focus → racine");
    }

    #[test]
    fn focus_points_at_a_present_node() {
        let focused = wid(10);
        let nodes = vec![(focused, Rect::new(0.0, 0.0, 40.0, 20.0), sem(Role::Button))];
        let update = build_tree_update(&nodes, Some(focused), "frus");
        assert_eq!(update.focus, node_id(focused));
        // Un focus hors arbre retombe sur la racine.
        let absent = wid(999);
        let update2 = build_tree_update(&nodes, Some(absent), "frus");
        assert_eq!(update2.focus, ROOT_ID);
    }

    #[test]
    fn node_id_round_trips() {
        let id = wid(70);
        assert_eq!(widget_for(node_id(id)), Some(id));
        assert_eq!(widget_for(ROOT_ID), None);
    }
}
