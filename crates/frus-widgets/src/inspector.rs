//! L'**inspecteur runtime** (§13) — l'équivalent du Widget Inspector Flutter,
//! palier 1 : contours de tous les widgets, surlignage + fiche d'infos du
//! widget sous le curseur, et **dump texte indenté** de l'arbre (le
//! `dump_deep` du §2).
//!
//! La matière vient de [`crate::build_ui_inspected`] : un [`InspectorNode`]
//! par widget peint, dans l'ordre de peinture (préfixe). Le calque se peint
//! **par-dessus** la scène de l'app ([`paint_overlay`]) — le shell l'active à
//! la demande (F12) sans toucher à l'app.

use frus_core::{Color, Point, Rect, Scene, Size, TextStyle};

use crate::interaction::WidgetId;
use crate::theme::Theme;

/// Un widget observé : son identité, sa boîte peinte, son nom court
/// (`debug_name`) et sa profondeur dans l'arbre.
#[derive(Clone, Copy, Debug)]
pub struct InspectorNode {
    pub id: WidgetId,
    pub rect: Rect,
    pub name: &'static str,
    pub depth: usize,
}

/// Le nœud que le survol désigne : le **plus profond** contenant `point`
/// (à profondeur égale, le dernier peint — celui du dessus).
pub fn node_at(nodes: &[InspectorNode], point: Point) -> Option<&InspectorNode> {
    nodes
        .iter()
        .filter(|n| n.rect.contains(point))
        .max_by_key(|n| n.depth)
}

/// Identité **abrégée** (32 bits bas, 8 hex) — assez pour corréler fiche et
/// dump sans noyer l'affichage sous 16 caractères.
fn short_id(id: WidgetId) -> String {
    format!("#{:08x}", id.as_u64() as u32)
}

/// Dump texte **indenté** de l'arbre observé : une ligne par widget —
/// `Nom  x,y  l×h  #id`. L'outil n°1 pour déboguer identité/réordonnancement.
pub fn dump_tree(nodes: &[InspectorNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        let r = node.rect;
        out.push_str(&format!(
            "{:indent$}{}  {:.0},{:.0}  {:.0}×{:.0}  {}\n",
            "",
            node.name,
            r.x,
            r.y,
            r.width,
            r.height,
            short_id(node.id),
            indent = node.depth * 2,
        ));
    }
    out
}

/// Nuancier des contours, par profondeur (cycle).
const OUTLINE_COLORS: [Color; 4] = [
    Color { r: 0.35, g: 0.65, b: 1.0, a: 0.55 },  // bleu
    Color { r: 0.45, g: 0.85, b: 0.55, a: 0.55 }, // vert
    Color { r: 1.0, g: 0.75, b: 0.35, a: 0.55 },  // orange
    Color { r: 0.85, g: 0.55, b: 0.95, a: 0.55 }, // violet
];

/// Peint le calque inspecteur **par-dessus** une scène déjà construite :
/// contours de chaque widget (teinte par profondeur), et si `hover` désigne un
/// widget, surlignage + fiche (nom, taille, position, id) près du curseur,
/// maintenue dans la fenêtre.
pub fn paint_overlay(
    nodes: &[InspectorNode],
    hover: Option<Point>,
    window: Size,
    theme: &Theme,
    scene: &mut Scene,
) {
    scene.set_clip(Rect::UNBOUNDED);
    scene.set_owner(0);

    for node in nodes {
        let color = OUTLINE_COLORS[node.depth % OUTLINE_COLORS.len()];
        scene.draw_rect(node.rect, Color::TRANSPARENT, 0.0, 1.0, color);
    }

    let Some(target) = hover.and_then(|point| node_at(nodes, point)) else {
        return;
    };

    // Surlignage du widget désigné : voile primaire + contour appuyé.
    scene.draw_rect(
        target.rect,
        theme.primary.with_alpha(0.18),
        0.0,
        2.0,
        theme.primary,
    );

    // Fiche d'infos, près du widget, bornée à la fenêtre.
    let r = target.rect;
    let title = target.name.to_string();
    let details = format!(
        "{:.0}×{:.0} @ {:.0},{:.0}  {}",
        r.width,
        r.height,
        r.x,
        r.y,
        short_id(target.id)
    );
    let title_style = TextStyle::new(13.0).weight(frus_core::FontWeight::Bold);
    let detail_style = TextStyle::new(12.0);
    let title_size = frus_text::measure_styled(&title, 13.0, title_style.weight, false);
    let detail_size = frus_text::measure_styled(&details, 12.0, detail_style.weight, false);

    const PAD: f32 = 8.0;
    let card_w = title_size.width.max(detail_size.width) + PAD * 2.0;
    let card_h = title_size.height + detail_size.height + PAD * 2.0;
    // Sous le widget si la place le permet, sinon au-dessus ; bornée en X.
    let x = r.x.clamp(0.0, (window.width - card_w).max(0.0));
    let below = r.y + r.height + 4.0;
    let y = if below + card_h <= window.height {
        below
    } else {
        (r.y - card_h - 4.0).max(0.0)
    };

    let card = Rect::new(x, y, card_w, card_h);
    scene.draw_rect(card, theme.scheme.inverse_surface, 6.0, 0.0, Color::TRANSPARENT);
    let on = theme.scheme.on_inverse_surface;
    scene.text_styled(Point::new(x + PAD, y + PAD), title, &title_style, on);
    scene.text_styled(
        Point::new(x + PAD, y + PAD + title_size.height),
        details,
        &detail_style,
        on,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::Widget;
    use crate::{build_ui_inspected, Container, Flex, Runtime, Text};

    /// Un petit arbre : la collecte couvre chaque widget, avec les bons noms
    /// (types concrets, wrappers transparents) et les bonnes profondeurs.
    #[test]
    fn collects_names_rects_and_depths() {
        let root: Container<()> = Container::new().width(200.0).height(100.0).child(
            Flex::column()
                .child(Text::new("a"))
                .child(crate::Keyed::new(7, Text::new("b"))),
        );
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (_, nodes) =
            build_ui_inspected(&root, Size::new(200.0, 100.0), &runtime, &theme);

        let names: Vec<&str> = nodes.iter().map(|n| n.name).collect();
        assert_eq!(names, ["Container", "Flex", "Text", "Text"], "Keyed est transparent");
        let depths: Vec<usize> = nodes.iter().map(|n| n.depth).collect();
        assert_eq!(depths, [0, 1, 2, 2]);
        assert_eq!(nodes[0].rect, Rect::new(0.0, 0.0, 200.0, 100.0), "racine = sa boîte");
        // Sans inspection : build_ui ne collecte rien (chemin normal inchangé).
        let ui = crate::build_ui(&root, Size::new(200.0, 100.0), &runtime, &theme);
        assert!(!ui.scene().is_empty());
    }

    /// Le survol désigne le widget le **plus profond** sous le point.
    #[test]
    fn node_at_picks_the_deepest() {
        let root: Container<()> = Container::new()
            .width(200.0)
            .height(100.0)
            .child(Container::new().width(50.0).height(40.0));
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (_, nodes) =
            build_ui_inspected(&root, Size::new(200.0, 100.0), &runtime, &theme);

        let inner = node_at(&nodes, Point::new(10.0, 10.0)).expect("un widget sous le point");
        assert_eq!(inner.depth, 1, "l'enfant, pas la racine");
        let outer = node_at(&nodes, Point::new(150.0, 80.0)).expect("la racine seule ici");
        assert_eq!(outer.depth, 0);
        assert!(node_at(&nodes, Point::new(500.0, 500.0)).is_none(), "hors de tout");
    }

    /// Le dump est indenté par profondeur et nomme chaque widget.
    #[test]
    fn dump_tree_indents_by_depth() {
        let root: Container<()> =
            Container::new().width(80.0).height(40.0).child(Text::new("x"));
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (_, nodes) = build_ui_inspected(&root, Size::new(80.0, 40.0), &runtime, &theme);
        let dump = dump_tree(&nodes);
        let lines: Vec<&str> = dump.lines().collect();
        assert!(lines[0].starts_with("Container"), "racine sans indentation : {dump}");
        assert!(lines[1].starts_with("  Text"), "enfant indenté de 2 : {dump}");
        assert!(lines[0].contains("80×40"), "géométrie dans le dump : {dump}");
    }

    /// Le calque se peint par-dessus une scène : contours pour chaque widget,
    /// surlignage + fiche quand un point est désigné.
    #[test]
    fn overlay_paints_outlines_and_hover_card() {
        let root: Container<()> =
            Container::new().width(120.0).height(60.0).child(Text::new("x"));
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (ui, nodes) =
            build_ui_inspected(&root, Size::new(120.0, 60.0), &runtime, &theme);

        let base = ui.scene().len();
        let mut scene = ui.scene().clone();
        paint_overlay(&nodes, None, Size::new(120.0, 60.0), &theme, &mut scene);
        assert_eq!(scene.len(), base + nodes.len(), "un contour par widget");

        let mut scene = ui.scene().clone();
        paint_overlay(
            &nodes,
            Some(Point::new(5.0, 5.0)),
            Size::new(120.0, 60.0),
            &theme,
            &mut scene,
        );
        assert!(
            scene.len() > base + nodes.len() + 2,
            "surlignage + carte + 2 textes en plus des contours"
        );
    }

    /// `debug_name` : type concret, sans chemin ni génériques ; `Box` délègue.
    #[test]
    fn debug_names_are_short_and_delegated() {
        let text = Text::new("x");
        assert_eq!(Widget::<()>::debug_name(&text), "Text");
        let boxed: Box<dyn Widget<()>> = Box::new(Text::new("x"));
        assert_eq!(boxed.debug_name(), "Text", "Box délègue au contenu");
        let container: Container<()> = Container::new();
        assert_eq!(Widget::<()>::debug_name(&container), "Container", "génériques retirés");
    }
}
