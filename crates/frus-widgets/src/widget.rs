//! Le trait [`Widget`], générique sur le type de message émis à l'interaction.

use frus_core::{Rect, Scene, Semantics, Size};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
use crate::theme::Theme;

/// Un widget : un élément d'interface composable.
///
/// `Msg` est le type de message applicatif émis lors d'une interaction (modèle
/// à messages, façon Elm/iced).
pub trait Widget<Msg> {
    /// Style de mise en page (transmis à `frus-layout`).
    fn style(&self) -> Style;

    /// Enfants du widget (éventuellement vide).
    fn children(&self) -> &[Box<dyn Widget<Msg>>];

    /// Peint la décoration propre du widget, selon son statut (survol / focus /
    /// curseur / sélection) et le thème courant.
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene);

    /// Message à émettre au clic (`None` = non cliquable).
    fn on_click(&self) -> Option<Msg>;

    /// Clé d'identité **stable** (indépendante de la position parmi les frères).
    /// `None` = identité positionnelle. Voir [`crate::Keyed`].
    fn key(&self) -> Option<u64> {
        None
    }

    /// Nom **court** du widget (l'équivalent du `runtimeType` Flutter) — pour
    /// l'inspecteur et les dumps diagnostiques. Par défaut : le nom du type
    /// concret, sans chemin de module ni génériques (chaque implémentation
    /// reçoit sa propre copie monomorphisée de ce corps par défaut). Les
    /// wrappers transparents (`Box`, [`crate::Keyed`]…) délèguent au contenu.
    fn debug_name(&self) -> &'static str {
        short_type_name::<Self>()
    }

    /// **Annotation d'accessibilité** du widget (rôle, libellé, valeur, état),
    /// exposée aux technologies d'assistance via l'arbre AccessKit. `None` =
    /// pas de sémantique propre (conteneur de mise en page — ses enfants, eux,
    /// peuvent en avoir). Voir [`frus_core::Semantics`].
    fn semantics(&self) -> Option<Semantics> {
        None
    }

    /// Applique une touche au widget focalisé : mute l'état d'édition
    /// (curseur/sélection) et renvoie un message si la **valeur** change.
    fn on_edit(&self, _edit: &mut Edit, _key: &Key) -> Option<Msg> {
        None
    }

    /// Index de curseur correspondant à une position horizontale locale (px
    /// depuis le bord gauche du widget) — pour placer le curseur au clic.
    ///
    /// `width` = largeur du champ, `scroll_cursor` = curseur d'où recalculer le
    /// **défilement horizontal** courant (le même que le rendu), pour que le clic
    /// tombe juste même quand le texte est défilé. `None` = pas un champ texte.
    fn cursor_at(&self, _local_x: f32, _width: f32, _scroll_cursor: usize) -> Option<usize> {
        None
    }

    /// Texte actuellement sélectionné (pour le copier/couper).
    fn selected_text(&self, _edit: &Edit) -> Option<String> {
        None
    }

    /// Valeur **complète** du champ, si ce widget est un champ texte — pour
    /// fournir le contexte de saisie à l'IME (suggestions). `None` sinon.
    fn text_value(&self) -> Option<&str> {
        None
    }

    /// Plage `(début, fin)` du mot autour de l'index donné (pour le double-clic).
    fn word_at(&self, _index: usize) -> Option<(usize, usize)> {
        None
    }

    /// Si `true`, le widget peut recevoir le focus clavier (clic ou Tab).
    fn focusable(&self) -> bool {
        false
    }

    /// Si `true`, le widget dessine **lui-même** son indicateur de focus (le
    /// pilote ne trace alors pas l'anneau générique). Ex. `TextInput`.
    fn draws_own_focus(&self) -> bool {
        false
    }

    /// Si `true`, le widget répond au glissement du pointeur (curseurs, poignées).
    fn draggable(&self) -> bool {
        false
    }

    /// Message produit lors d'un glissement, `fraction` étant la position
    /// horizontale relative (`0.0..=1.0`) dans les bornes du widget.
    fn on_drag(&self, _fraction: f32) -> Option<Msg> {
        None
    }

    /// Si `true`, le widget est une **pile** : ses enfants sont des couches
    /// superposées (même boîte), rendues dans l'ordre (dernière au-dessus).
    fn stack(&self) -> bool {
        false
    }

    /// Si `true`, le widget s'anime **en continu** (piloté par le temps, pas par
    /// une cible) : le framework continue de redessiner. Ex. `Spinner`.
    fn continuous(&self) -> bool {
        false
    }

    /// Si `true`, ce widget est une **frontière de repaint** : son sous-arbre est
    /// mis en cache (primitives + cartes d'interaction) et **réutilisé tel quel**
    /// tant que sa géométrie et l'état d'interaction de ses descendants ne
    /// changent pas — un widget qui s'anime ailleurs ne le fait plus repeindre.
    /// Voir [`crate::RepaintBoundary`] et `paintcache.rs`.
    fn repaint_boundary(&self) -> bool {
        false
    }

    /// Si le widget est un conteneur défilable, renvoie son contenu.
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }

    /// Si le widget est une **liste virtualisée**, renvoie sa description (nombre
    /// d'éléments, hauteur, fabrique). Seuls les éléments visibles sont construits.
    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        None
    }

    /// Si le widget construit son contenu **à partir de sa boîte réelle** (façon
    /// Flutter `LayoutBuilder`), renvoie la fabrique `taille → widget`. Le contenu
    /// est construit à la volée : pas d'état retenu ni d'overlay (comme un élément
    /// de liste virtualisée).
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        None
    }

    /// Axe(s) de défilement (pour un conteneur défilable).
    fn scroll_axis(&self) -> crate::scroll::Axis {
        crate::scroll::Axis::Vertical
    }

    /// Si le widget est un portail, renvoie son contenu flottant et son placement.
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        None
    }

    /// Message émis au clic sur le voile d'une modale (fermeture), le cas échéant.
    fn overlay_dismiss(&self) -> Option<Msg> {
        None
    }

    /// Cible de la valeur animée propre au widget (p. ex. `1.0` interrupteur on,
    /// `0.0` off). Le runtime fait tendre la valeur retenue vers cette cible et la
    /// restitue via `Status::value`. `None` = pas de valeur animée.
    fn anim_target(&self) -> Option<f32> {
        None
    }

    /// Durée (secondes) de la transition de la valeur animée (`anim_target`) —
    /// façon `duration` de Flutter. Défaut : la durée standard du framework.
    fn anim_duration(&self) -> f32 {
        crate::runtime::ANIM_DURATION
    }

    /// Courbe d'accélération de la transition de la valeur animée (`anim_target`)
    /// — façon `curve` de Flutter (`Curves.easeInOut`…). Défaut : linéaire.
    fn anim_curve(&self) -> frus_core::Curve {
        frus_core::Curve::Linear
    }

    /// Si le widget est un **groupe d'opacité**, renvoie l'opacité (cible) `[0,1]`
    /// appliquée à tout son sous-arbre **d'un bloc** (façon `Opacity` de Flutter) :
    /// la marche de peinture enveloppe ses primitives dans un calque composité à
    /// cette opacité — pas de double-superposition sur les chevauchements. `None`
    /// = pas un groupe. Combiné à `anim_target`, l'opacité **s'anime** (fondu de
    /// groupe, façon `AnimatedOpacity`).
    fn opacity_group(&self) -> Option<f32> {
        None
    }

    /// Couleur de fond **cible** d'un fond animé (`Container::animated_color`) :
    /// le runtime la tween (via `anim_duration`/`anim_curve`) et restitue
    /// l'interpolée dans `Status::anim_color`. `None` = pas de couleur animée.
    fn anim_color(&self) -> Option<frus_core::Color> {
        None
    }

    /// Taille **cible** d'une taille animée (`Container::animated_size`) : le
    /// runtime la tween (via `anim_duration`/`anim_curve`) et la taille interpolée
    /// est injectée **au layout** (voir `effective_style`). `None` = taille fixe.
    fn anim_size(&self) -> Option<Size> {
        None
    }

    /// Rayon de coin **cible** d'un rayon animé (`Container::animated_radius`) :
    /// le runtime le tween et restitue l'interpolé dans `Status::anim_radius`.
    /// `None` = rayon fixe.
    fn anim_radius(&self) -> Option<frus_core::BorderRadius> {
        None
    }

    /// Marge (padding) **cible** d'une marge animée (`Container::animated_padding`)
    /// : le runtime la tween et la marge interpolée est injectée **au layout**
    /// (voir `effective_style`). `None` = marge fixe.
    fn anim_padding(&self) -> Option<frus_core::Insets> {
        None
    }

    /// **Ancrage** de l'unique enfant dans la boîte (façon `Container(alignment:)`
    /// de Flutter) : la marche décale l'enfant dans l'espace libre selon les
    /// fractions de l'ancrage, résolu selon la direction de lecture (physique ou
    /// directionnel — voir [`frus_core::AlignmentGeometry`]). Étant continu, un
    /// `Tween<Alignment>` lu dans `view()` glisse l'enfant. `None` = placement flex
    /// par défaut.
    fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
        None
    }

    /// **Décalage de peinture** `(dx, dy)` appliqué à tout le sous-arbre du widget
    /// (façon `Transform.translate` de Flutter) : le rendu et le hit-test sont
    /// décalés **sans toucher la mise en page** (les frères ne bougent pas, l'enfant
    /// peut déborder sa boîte). Continu → un `Tween` lu dans `view()` glisse le
    /// sous-arbre. `None` = aucun décalage. Voir [`crate::Transform`].
    fn transform_translate(&self) -> Option<(f32, f32)> {
        None
    }

    /// Si le widget est un navigateur d'écrans, renvoie `(progression, push?)`.
    /// Ses enfants (`[écran]` ou `[sortant, entrant]`) sont rendus plein-fenêtre
    /// avec une transition glissée.
    fn navigator(&self) -> Option<(f32, bool)> {
        None
    }

    /// Message émis par un **appui long** (pression maintenue ~500 ms sans
    /// mouvement). L'appui long *évince* le clic : le relâchement qui le suit
    /// n'émet pas `on_click`.
    fn on_long_press(&self) -> Option<Msg> {
        None
    }

    /// Touche reçue pendant la **montée feuille→racine** : le widget focalisé la
    /// reçoit d'abord, puis chaque ancêtre tant que la réponse est `Ignored`.
    /// (Ex. : un `Portal` consomme `Escape` pour se fermer.)
    fn on_key(&self, _key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        crate::interaction::KeyResponse::Ignored
    }

    /// **Mesure sous contraintes** : pour un widget dont la taille dépend de
    /// l'espace offert (paragraphe qui se replie…), renvoie la closure branchée
    /// sur taffy. `None` = taille fixée par `style()`. Contrat : doit être
    /// `Some` **si et seulement si** [`Widget::measure_key`] l'est.
    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        None
    }

    /// Empreinte du **contenu** dont dépend [`Widget::measure`] (texte, style…),
    /// mêlée à l'empreinte de relayout : sans elle, deux contenus différents de
    /// même style seraient confondus par le cache et garderaient une vieille
    /// géométrie. Contrat : `Some` si et seulement si `measure()` l'est.
    fn measure_key(&self) -> Option<u64> {
        None
    }
}

/// Nom de type **court** : sans chemin de module ni paramètres génériques
/// (`frus_widgets::text::Text` → `Text`, `Container<Msg>` → `Container`).
pub(crate) fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    let no_generics = full.split('<').next().unwrap_or(full);
    no_generics.rsplit("::").next().unwrap_or(no_generics)
}

/// Permet de composer un widget **déjà boxé** là où un `impl Widget` est attendu
/// (p. ex. `Flex::child`). Délègue tout au widget contenu.
impl<Msg> Widget<Msg> for Box<dyn Widget<Msg>> {
    fn style(&self) -> Style {
        (**self).style()
    }
    fn debug_name(&self) -> &'static str {
        (**self).debug_name()
    }
    fn semantics(&self) -> Option<Semantics> {
        (**self).semantics()
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        (**self).children()
    }
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        (**self).paint(bounds, status, theme, scene)
    }
    fn on_click(&self) -> Option<Msg> {
        (**self).on_click()
    }
    fn key(&self) -> Option<u64> {
        (**self).key()
    }
    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        (**self).on_edit(edit, key)
    }
    fn cursor_at(&self, local_x: f32, width: f32, scroll_cursor: usize) -> Option<usize> {
        (**self).cursor_at(local_x, width, scroll_cursor)
    }
    fn selected_text(&self, edit: &Edit) -> Option<String> {
        (**self).selected_text(edit)
    }
    fn text_value(&self) -> Option<&str> {
        (**self).text_value()
    }
    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        (**self).word_at(index)
    }
    fn focusable(&self) -> bool {
        (**self).focusable()
    }
    fn draws_own_focus(&self) -> bool {
        (**self).draws_own_focus()
    }
    fn draggable(&self) -> bool {
        (**self).draggable()
    }
    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        (**self).on_drag(fraction)
    }
    fn stack(&self) -> bool {
        (**self).stack()
    }
    fn continuous(&self) -> bool {
        (**self).continuous()
    }
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        (**self).scroll_content()
    }
    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        (**self).virtual_list()
    }
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        (**self).layout_builder()
    }
    fn scroll_axis(&self) -> Axis {
        (**self).scroll_axis()
    }
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        (**self).overlay()
    }
    fn overlay_dismiss(&self) -> Option<Msg> {
        (**self).overlay_dismiss()
    }
    fn anim_target(&self) -> Option<f32> {
        (**self).anim_target()
    }
    fn anim_duration(&self) -> f32 {
        (**self).anim_duration()
    }
    fn anim_curve(&self) -> frus_core::Curve {
        (**self).anim_curve()
    }
    fn opacity_group(&self) -> Option<f32> {
        (**self).opacity_group()
    }
    fn anim_color(&self) -> Option<frus_core::Color> {
        (**self).anim_color()
    }
    fn anim_size(&self) -> Option<Size> {
        (**self).anim_size()
    }
    fn anim_radius(&self) -> Option<frus_core::BorderRadius> {
        (**self).anim_radius()
    }
    fn anim_padding(&self) -> Option<frus_core::Insets> {
        (**self).anim_padding()
    }
    fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
        (**self).alignment_geometry()
    }
    fn transform_translate(&self) -> Option<(f32, f32)> {
        (**self).transform_translate()
    }
    fn navigator(&self) -> Option<(f32, bool)> {
        (**self).navigator()
    }
    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        (**self).measure()
    }
    fn measure_key(&self) -> Option<u64> {
        (**self).measure_key()
    }
    fn on_long_press(&self) -> Option<Msg> {
        (**self).on_long_press()
    }
    fn on_key(&self, key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        (**self).on_key(key)
    }
}
