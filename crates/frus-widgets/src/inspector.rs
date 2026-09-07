//! The **runtime inspector** (§13), tier 1: outlines around every widget, a
//! highlight plus an info card for the widget under the cursor, and an
//! **indented text dump** of the tree (the `dump_deep` of §2).
//!
//! The material comes from [`crate::build_ui_inspected`]: one [`InspectorNode`]
//! per painted widget, in paint (prefix) order. The layer paints **on top of**
//! the app's scene ([`paint_overlay`]) — the shell turns it on on demand (F12)
//! without touching the app.

use frus_core::{Color, Point, Rect, Scene, Size, TextStyle};

use crate::interaction::WidgetId;
use crate::theme::Theme;

/// One observed widget: its identity, its painted box, its short name
/// (`debug_name`) and its depth in the tree.
#[derive(Clone, Copy, Debug)]
pub struct InspectorNode {
    pub id: WidgetId,
    pub rect: Rect,
    pub name: &'static str,
    pub depth: usize,
}

/// The node the hover designates: the **deepest** one containing `point`
/// (at equal depth, the last painted — the one on top).
pub fn node_at(nodes: &[InspectorNode], point: Point) -> Option<&InspectorNode> {
    nodes
        .iter()
        .filter(|n| n.rect.contains(point))
        .max_by_key(|n| n.depth)
}

/// An **abbreviated** identity (low 32 bits, 8 hex) — enough to correlate the
/// card with the dump without drowning the display under 16 characters.
fn short_id(id: WidgetId) -> String {
    format!("#{:08x}", id.as_u64() as u32)
}

/// An **indented** text dump of the observed tree: one line per widget —
/// `Name  x,y  w×h  #id`. The first tool to reach for when debugging identity
/// or reordering.
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

/// The outline palette, by depth (it cycles).
const OUTLINE_COLORS: [Color; 4] = [
    Color {
        r: 0.35,
        g: 0.65,
        b: 1.0,
        a: 0.55,
    }, // bleu
    Color {
        r: 0.45,
        g: 0.85,
        b: 0.55,
        a: 0.55,
    }, // vert
    Color {
        r: 1.0,
        g: 0.75,
        b: 0.35,
        a: 0.55,
    }, // orange
    Color {
        r: 0.85,
        g: 0.55,
        b: 0.95,
        a: 0.55,
    }, // violet
];

/// Paints the inspector layer **on top of** an already built scene: an outline
/// around every widget (tinted by depth) and, if `hover` designates a widget, a
/// highlight plus a card (name, size, position, id) near the cursor, kept inside
/// the window.
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

    // Highlight for the designated widget: a primary scrim + a heavier outline.
    scene.draw_rect(
        target.rect,
        theme.primary.with_alpha(0.18),
        0.0,
        2.0,
        theme.primary,
    );

    // The info card, near the widget, clamped to the window.
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
    let title_size = frus_text::measure_styled(&title, 13.0, title_style.resolved().weight, false);
    let detail_size =
        frus_text::measure_styled(&details, 12.0, detail_style.resolved().weight, false);

    const PAD: f32 = 8.0;
    let card_w = title_size.width.max(detail_size.width) + PAD * 2.0;
    let card_h = title_size.height + detail_size.height + PAD * 2.0;
    // Below the widget if there is room, otherwise above; clamped in x.
    let x = r.x.clamp(0.0, (window.width - card_w).max(0.0));
    let below = r.y + r.height + 4.0;
    let y = if below + card_h <= window.height {
        below
    } else {
        (r.y - card_h - 4.0).max(0.0)
    };

    let card = Rect::new(x, y, card_w, card_h);
    scene.draw_rect(
        card,
        theme.scheme.inverse_surface,
        6.0,
        0.0,
        Color::TRANSPARENT,
    );
    let on = theme.scheme.on_inverse_surface;
    scene.text(
        Point::new(x + PAD, y + PAD),
        title,
        &title_style.resolved(),
        on,
    );
    scene.text(
        Point::new(x + PAD, y + PAD + title_size.height),
        details,
        &detail_style.resolved(),
        on,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::Widget;
    use crate::{build_ui_inspected, Container, Flex, Runtime, Text};

    /// A small tree: collection covers every widget, with the right names
    /// (concrete types, transparent wrappers) and the right depths.
    #[test]
    fn collects_names_rects_and_depths() {
        let root: Container<()> = Container::new().width(200.0).height(100.0).child(
            Flex::column()
                .child(Text::new("a"))
                .child(crate::Keyed::new(7, Text::new("b"))),
        );
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (_, nodes) = build_ui_inspected(&root, Size::new(200.0, 100.0), &runtime, &theme);

        let names: Vec<&str> = nodes.iter().map(|n| n.name).collect();
        assert_eq!(
            names,
            ["Container", "Flex", "Text", "Text"],
            "Keyed is transparent"
        );
        let depths: Vec<usize> = nodes.iter().map(|n| n.depth).collect();
        assert_eq!(depths, [0, 1, 2, 2]);
        assert_eq!(
            nodes[0].rect,
            Rect::new(0.0, 0.0, 200.0, 100.0),
            "the root = its box"
        );
        // Without inspection: build_ui collects nothing (the normal path is unchanged).
        let ui = crate::build_ui(&root, Size::new(200.0, 100.0), &runtime, &theme);
        assert!(!ui.scene().is_empty());
    }

    /// Hovering designates the **deepest** widget under the point.
    #[test]
    fn node_at_picks_the_deepest() {
        let root: Container<()> = Container::new()
            .width(200.0)
            .height(100.0)
            .child(Container::new().width(50.0).height(40.0));
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (_, nodes) = build_ui_inspected(&root, Size::new(200.0, 100.0), &runtime, &theme);

        let inner = node_at(&nodes, Point::new(10.0, 10.0)).expect("a widget under the point");
        assert_eq!(inner.depth, 1, "the child, not the root");
        let outer = node_at(&nodes, Point::new(150.0, 80.0)).expect("the root alone here");
        assert_eq!(outer.depth, 0);
        assert!(
            node_at(&nodes, Point::new(500.0, 500.0)).is_none(),
            "outside everything"
        );
    }

    /// The dump is indented by depth and names every widget.
    #[test]
    fn dump_tree_indents_by_depth() {
        let root: Container<()> = Container::new()
            .width(80.0)
            .height(40.0)
            .child(Text::new("x"));
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (_, nodes) = build_ui_inspected(&root, Size::new(80.0, 40.0), &runtime, &theme);
        let dump = dump_tree(&nodes);
        let lines: Vec<&str> = dump.lines().collect();
        assert!(
            lines[0].starts_with("Container"),
            "the root is not indented: {dump}"
        );
        assert!(
            lines[1].starts_with("  Text"),
            "child indented by 2: {dump}"
        );
        assert!(lines[0].contains("80×40"), "geometry in the dump: {dump}");
    }

    /// The layer paints on top of a scene: outlines for every widget, plus a
    /// highlight and a card when a point is designated.
    #[test]
    fn overlay_paints_outlines_and_hover_card() {
        let root: Container<()> = Container::new()
            .width(120.0)
            .height(60.0)
            .child(Text::new("x"));
        let runtime = Runtime::default();
        let theme = Theme::default();
        let (ui, nodes) = build_ui_inspected(&root, Size::new(120.0, 60.0), &runtime, &theme);

        let base = ui.scene().len();
        let mut scene = ui.scene().clone();
        paint_overlay(&nodes, None, Size::new(120.0, 60.0), &theme, &mut scene);
        assert_eq!(scene.len(), base + nodes.len(), "one outline per widget");

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
            "highlight + card + 2 texts on top of the outlines"
        );
    }

    /// `debug_name`: the concrete type, with no path and no generics; `Box` delegates.
    #[test]
    fn debug_names_are_short_and_delegated() {
        let text = Text::new("x");
        assert_eq!(Widget::<()>::debug_name(&text), "Text");
        let boxed: Box<dyn Widget<()>> = Box::new(Text::new("x"));
        assert_eq!(boxed.debug_name(), "Text", "Box delegates to its content");
        let container: Container<()> = Container::new();
        assert_eq!(
            Widget::<()>::debug_name(&container),
            "Container",
            "generics stripped"
        );
    }
}
