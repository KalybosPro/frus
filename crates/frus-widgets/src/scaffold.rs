//! [`Scaffold`] : l'**ossature d'écran** de frus, calquée sur le `Scaffold` de
//! Flutter — le coordinateur central de la structure Material.
//!
//! Le développeur déclare des **slots** (barre haute, corps, navigation, tiroir,
//! FAB, feuille modale) ; le Scaffold les assemble correctement — **barre haute
//! épinglée** en haut, **corps défilant** au milieu, **navigation adaptative**
//! (barre basse en étroit, rail latéral en large), le tout **respectant la zone
//! de sécurité** (insets système). Un seul code, sans brancher sur mobile/desktop.
//!
//! ```ignore
//! Scaffold::new(width, height)
//!     .insets(app.insets)
//!     .background(theme.background)
//!     .app_bar(appbar)                       // épinglé en haut
//!     .body(content)                         // défile
//!     .nav(app.section, Msg::SetSection)     // navigation adaptative
//!     .destination("✔", "Tasks").badge(3)
//!     .destination("▦", "Stats")
//!     .end_drawer(menu, app.drawer_open, Msg::ToggleDrawer)
//!     .fab(button("＋", Msg::AddTodo))        // bouton d'action flottant
//!     .bottom_sheet(sheet, app.sheet_open, Msg::ToggleSheet)
//!     .build()
//! ```

use frus_core::{Color, Insets, SizeClass};
use frus_layout::Justify;

use crate::button::Variant;
use crate::container::Container;
use crate::flex::Flex;
use crate::navrail::{BottomBar, NavRail, BAR_HEIGHT};
use crate::scroll::Scroll;
use crate::stack::Stack;
use crate::widget::Widget;

/// Marge du FAB par rapport au bord (et à la barre basse).
const FAB_MARGIN: f32 = 16.0;

/// Ossature d'écran adaptative. Constructeur fluide terminé par [`Scaffold::build`].
pub struct Scaffold<Msg> {
    width: f32,
    height: f32,
    insets: Insets,
    background: Option<Color>,
    app_bar: Option<Box<dyn Widget<Msg>>>,
    body: Option<Box<dyn Widget<Msg>>>,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<(String, String, Option<u32>)>,
    end_drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    fab: Option<Box<dyn Widget<Msg>>>,
    bottom_sheet: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
}

impl<Msg: Clone + 'static> Scaffold<Msg> {
    /// Crée une ossature pour une surface de `width × height` px logiques. La
    /// classe de taille (rail vs barre basse) est déduite de la largeur.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            insets: Insets::ZERO,
            background: None,
            app_bar: None,
            body: None,
            selected: 0,
            on_select: None,
            destinations: Vec::new(),
            end_drawer: None,
            fab: None,
            bottom_sheet: None,
        }
    }

    /// Zone de sécurité (barres système) : le Scaffold en écarte les slots.
    pub fn insets(mut self, insets: Insets) -> Self {
        self.insets = insets;
        self
    }

    /// Couleur de fond, étendue bord à bord (y compris sous les barres système).
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Barre d'application, épinglée en haut.
    pub fn app_bar(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.app_bar = Some(Box::new(widget));
        self
    }

    /// Corps de l'écran : il **défile** dans l'espace entre les barres.
    pub fn body(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.body = Some(Box::new(widget));
        self
    }

    /// Active la navigation adaptative : `selected` = destination active,
    /// `on_select(i)` émis au choix. Ajouter ensuite des [`Scaffold::destination`].
    pub fn nav(mut self, selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.selected = selected;
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Ajoute une destination de navigation (glyphe + libellé).
    pub fn destination(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.destinations.push((icon.into(), label.into(), None));
        self
    }

    /// Compteur de notifications sur la **dernière** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.destinations.last_mut() {
            last.2 = Some(count);
        }
        self
    }

    /// Tiroir latéral (bord droit), modal : `panel` = contenu, `open` = déployé,
    /// `toggle` = message de bascule (bouton + clic sur le voile).
    pub fn end_drawer(mut self, panel: impl Widget<Msg> + 'static, open: bool, toggle: Msg) -> Self {
        self.end_drawer = Some((Box::new(panel), open, toggle));
        self
    }

    /// Bouton d'action flottant, ancré en bas-droite (au-dessus de la barre basse).
    ///
    /// ⚠ **Expérimental** : le FAB est superposé via une couche `Stack` plein
    /// écran ; or une telle couche supérieure **intercepte les clics** de la
    /// moitié basse de l'écran (limite du hit-test des `Stack`). À corriger (overlay
    /// non bloquant) avant usage réel — voir jalon 52c.
    pub fn fab(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.fab = Some(Box::new(widget));
        self
    }

    /// Feuille modale glissant depuis le bas.
    pub fn bottom_sheet(
        mut self,
        panel: impl Widget<Msg> + 'static,
        open: bool,
        toggle: Msg,
    ) -> Self {
        self.bottom_sheet = Some((Box::new(panel), open, toggle));
        self
    }

    /// Assemble l'ossature en un widget prêt à afficher.
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let Scaffold {
            width,
            height,
            insets,
            background,
            app_bar,
            body,
            selected,
            on_select,
            destinations,
            end_drawer,
            fab,
            bottom_sheet,
        } = self;

        let compact = SizeClass::from_width(width) == SizeClass::Compact;
        let bg = background.unwrap_or(Color::TRANSPARENT);
        let has_nav = !destinations.is_empty();
        let body_widget = body.unwrap_or_else(|| Box::new(Container::new()));

        // Navigation : barre basse (étroit) ou rail latéral (large).
        let nav: Option<Box<dyn Widget<Msg>>> = if has_nav {
            let on_select = on_select.expect("nav(selected, on_select) requis avec des destinations");
            if compact {
                let mut bar = BottomBar::new(selected, on_select);
                for (icon, label, badge) in &destinations {
                    bar = bar.item(icon.clone(), label.clone());
                    if let Some(count) = *badge {
                        bar = bar.badge(count);
                    }
                }
                Some(Box::new(bar))
            } else {
                let mut rail = NavRail::new(selected, on_select);
                for (icon, label, badge) in &destinations {
                    rail = rail.item(icon.clone(), label.clone());
                    if let Some(count) = *badge {
                        rail = rail.badge(count);
                    }
                }
                Some(Box::new(rail))
            }
        } else {
            None
        };

        // Corps défilant, insets latéraux appliqués à son contenu.
        let scroll_body =
            Scroll::new().flex(1.0).child(inset_pad(body_widget, 0.0, insets.right, 0.0, insets.left));

        // Ossature épinglée : barre haute · corps · (barre basse | rail).
        let main: Box<dyn Widget<Msg>> = if compact {
            let mut col = Flex::column().width(width).height(height);
            if let Some(bar) = app_bar {
                col = col.child(inset_pad(bar, insets.top, insets.right, 0.0, insets.left));
            }
            col = col.child(scroll_body);
            if let Some(n) = nav {
                col = col.child(inset_pad(n, 0.0, insets.right, insets.bottom, insets.left));
            }
            Box::new(col)
        } else {
            let mut row = Flex::row().width(width).height(height);
            if let Some(n) = nav {
                row = row.child(inset_pad(n, insets.top, 0.0, insets.bottom, insets.left));
            }
            let mut content = Flex::column().flex(1.0);
            if let Some(bar) = app_bar {
                content = content.child(inset_pad(bar, insets.top, insets.right, 0.0, 0.0));
            }
            content = content.child(scroll_body);
            row = row.child(content);
            Box::new(row)
        };

        // FAB ancré en bas-droite, au-dessus de la barre basse et de l'inset.
        let mut content: Box<dyn Widget<Msg>> = main;
        if let Some(fab) = fab {
            let nav_h = if compact && has_nav { BAR_HEIGHT } else { 0.0 };
            let fab_bottom = insets.bottom + nav_h + FAB_MARGIN;
            let fab_layer = Flex::column().width(width).height(height).justify(Justify::End).child(
                Flex::row().justify(Justify::End).child(
                    Container::new()
                        .padding_each(0.0, insets.right + FAB_MARGIN, fab_bottom, 0.0)
                        .child(fab),
                ),
            );
            content = Box::new(
                Stack::new().width(width).height(height).layer(content).layer(fab_layer),
            );
        }

        // Tiroir (modal) puis feuille modale enveloppent l'ossature (overlays).
        if let Some((panel, open, toggle)) = end_drawer {
            content = Box::new(
                crate::Drawer::new(open).on_dismiss(toggle).right().panel(panel).body(content),
            );
        }
        if let Some((panel, open, toggle)) = bottom_sheet {
            content = Box::new(
                crate::BottomSheet::new(open).on_dismiss(toggle).sheet(panel).body(content),
            );
        }

        // Fond plein-fenêtre (bord à bord) donnant une taille définie aux slots.
        Box::new(Container::new().width(width).height(height).color(bg).child(content))
    }
}

/// Écarte un slot des barres système, **sans wrapper superflu** : si tous les
/// insets sont nuls, renvoie le widget tel quel (préserve l'étirement du parent) ;
/// sinon l'enveloppe d'un `Container` de padding.
fn inset_pad<Msg: Clone + 'static>(
    widget: Box<dyn Widget<Msg>>,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> Box<dyn Widget<Msg>> {
    if top == 0.0 && right == 0.0 && bottom == 0.0 && left == 0.0 {
        widget
    } else {
        Box::new(Container::new().padding_each(top, right, bottom, left).child(widget))
    }
}

/// Un bouton d'action flottant conventionnel (rond, accent), à passer à
/// [`Scaffold::fab`]. Sucre pour `button(label, msg)` stylé en primaire.
pub fn fab_button<Msg: Clone + 'static>(label: impl Into<String>, message: Msg) -> crate::Button<Msg> {
    crate::Button::new(label).variant(Variant::Primary).size(24.0).on_press(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, dsl::button, dsl::text, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
        Drawer,
        Add,
    }

    fn scaffold(width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        Scaffold::new(width, height)
            .insets(Insets::new(40.0, 0.0, 30.0, 0.0))
            .background(Color::rgb(0.1, 0.1, 0.1))
            .app_bar(text("Title").size(20.0))
            .body(text("Body").size(16.0))
            .nav(0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Stats")
            .end_drawer(text("Menu"), false, Msg::Drawer)
            .fab(button("＋", Msg::Add))
            .build()
    }

    #[test]
    fn assembles_at_compact_and_expanded_without_panic() {
        for w in [400.0_f32, 1000.0_f32] {
            let s = scaffold(w, 800.0);
            let ui = build_ui(s.as_ref(), Size::new(w, 800.0), &Runtime::default(), &Theme::default());
            assert!(!ui.scene().primitives().is_empty(), "scène vide pour width={w}");
        }
    }

    #[test]
    fn compact_pins_bottom_bar_near_the_bottom() {
        // La barre basse est épinglée en bas : des primitives sont peintes dans la
        // bande basse (y ≥ 700), au-dessus de l'inset bas (30), pas au milieu.
        let s = scaffold(400.0, 800.0);
        let ui = build_ui(s.as_ref(), Size::new(400.0, 800.0), &Runtime::default(), &Theme::default());
        let pinned_low = ui.scene().primitives().iter().any(|p| match p {
            frus_core::Primitive::Rect { rect, .. } => rect.y >= 700.0 && rect.height < 100.0,
            frus_core::Primitive::Text { position, .. }
            | frus_core::Primitive::RichText { position, .. } => position.y >= 700.0,
        });
        assert!(pinned_low, "la barre basse doit être épinglée dans la bande basse");
    }
}
