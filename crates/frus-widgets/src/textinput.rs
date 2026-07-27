//! [`TextInput`] : un champ de saisie mono-ligne, **contrôlé** (sa valeur vient
//! de l'état applicatif), avec curseur, navigation et sélection.
//!
//! La valeur est contrôlée ; le **curseur / la sélection** sont un état d'édition
//! retenu au runtime ([`Edit`]), clé par identité de widget.

use frus_core::{FontWeight, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};
use frus_text::TextLayout;

use crate::icons::IconName;
use crate::interaction::{Key, Status};
use crate::runtime::Edit;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 6.0;

/// Taille de police du label (au-dessus du champ) et du texte d'aide/erreur (en
/// dessous) — la typographie « annexe » de la décoration.
const LABEL_SIZE: f32 = 13.0;
const SUB_SIZE: f32 = 12.0;
/// Espace vertical entre le label, la boîte de saisie et la ligne d'aide/erreur.
const DECO_GAP: f32 = 4.0;

/// Côté d'une icône préfixe/suffixe (px logiques) et marge autour d'elle.
const ICON_SIZE: f32 = 20.0;
const ICON_PAD: f32 = 6.0;
/// Caractère de masquage par défaut d'un champ mot de passe.
const OBSCURE_CHAR: char = '•';

/// Un champ de saisie de texte sur une ligne, avec **décoration de formulaire**
/// optionnelle (label, indice, texte d'aide, erreur) — le pendant frus de
/// l'`InputDecoration` de Flutter. La **validité** reste décidée par l'application
/// (fonction pure de l'état) ; le champ n'en affiche que le résultat via [`error`].
///
/// [`error`]: TextInput::error
pub struct TextInput<Msg> {
    value: String,
    size: f32,
    width: Dimension,
    on_input: Option<Box<dyn Fn(String) -> Msg>>,
    on_submit: Option<Msg>,
    /// Étiquette affichée au-dessus du champ.
    label: Option<String>,
    /// Indice affiché **dans** le champ quand la valeur est vide (façon *hint*).
    placeholder: Option<String>,
    /// Texte d'aide sous le champ (masqué quand une erreur est présente).
    helper: Option<String>,
    /// Message d'erreur : quand présent, la bordure et le label passent en couleur
    /// d'erreur et ce texte remplace l'aide sous le champ.
    error: Option<String>,
    /// Masque la valeur (champ mot de passe) : chaque caractère est rendu par
    /// [`OBSCURE_CHAR`]. L'édition porte sur la vraie valeur ; seul l'affichage change.
    obscure: bool,
    /// Icône décorative à gauche dans la boîte (façon `prefixIcon`).
    prefix: Option<IconName>,
    /// Icône décorative à droite dans la boîte (façon `suffixIcon`).
    suffix: Option<IconName>,
    /// Champ **multi-lignes** : Entrée insère un saut de ligne (au lieu de soumettre),
    /// la boîte fait `rows` lignes de haut et défile verticalement pour suivre le caret.
    multiline: bool,
    /// Nombre de lignes visibles en mode multi-lignes.
    rows: u16,
}

/// Déplace le curseur vers `target`, en gérant l'ancre de sélection selon Shift.
fn move_cursor(cursor: &mut usize, anchor: &mut Option<usize>, target: usize, shift: bool) {
    if shift {
        if anchor.is_none() {
            *anchor = Some(*cursor);
        }
    } else {
        *anchor = None;
    }
    *cursor = target;
}

impl<Msg> TextInput<Msg> {
    /// Crée un champ affichant `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            size: 18.0,
            width: Dimension::Length(220.0),
            on_input: None,
            on_submit: None,
            label: None,
            placeholder: None,
            helper: None,
            error: None,
            obscure: false,
            prefix: None,
            suffix: None,
            multiline: false,
            rows: 3,
        }
    }

    /// Passe le champ en **multi-lignes** : Entrée insère un saut de ligne (au lieu de
    /// soumettre), et la boîte affiche `rows` lignes (voir [`rows`](Self::rows)).
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Nombre de lignes visibles en mode multi-lignes (au moins 1). Implique
    /// [`multiline`](Self::multiline).
    pub fn rows(mut self, rows: u16) -> Self {
        self.multiline = true;
        self.rows = rows.max(1);
        self
    }

    /// Masque la valeur (champ mot de passe) : chaque caractère devient un point.
    /// L'édition reste normale ; seul l'affichage est masqué.
    pub fn obscure(mut self, obscure: bool) -> Self {
        self.obscure = obscure;
        self
    }

    /// Icône décorative à gauche dans le champ (`prefixIcon`).
    pub fn prefix_icon(mut self, icon: IconName) -> Self {
        self.prefix = Some(icon);
        self
    }

    /// Icône décorative à droite dans le champ (`suffixIcon`).
    pub fn suffix_icon(mut self, icon: IconName) -> Self {
        self.suffix = Some(icon);
        self
    }

    /// Étiquette affichée au-dessus du champ.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Indice affiché dans le champ tant qu'il est vide.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Texte d'aide sous le champ (remplacé par l'erreur, si présente).
    pub fn helper(mut self, helper: impl Into<String>) -> Self {
        self.helper = Some(helper.into());
        self
    }

    /// Marque le champ **en erreur** : bordure et label rouges, `message` affiché
    /// sous le champ. À ne chaîner que lorsque la validation applicative échoue.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la taille de police, en pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Closure produisant un message depuis la nouvelle valeur du champ.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Message émis à la validation (touche Entrée), sans modifier la valeur.
    pub fn on_submit(mut self, message: Msg) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// La chaîne **affichée** : masquée (un point par caractère) pour un champ mot
    /// de passe, la valeur telle quelle sinon. Même nombre de caractères que la
    /// valeur → le caret et le hit-test restent alignés index pour index.
    fn display(&self) -> String {
        if self.obscure {
            OBSCURE_CHAR.to_string().repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    /// La chaîne **affichée** shapée une fois : caret par index, hit-test, sélection
    /// — géométrie cohérente (kerning). `wrap_width` = largeur de repli doux
    /// (multi-lignes) ou `None` (mono-ligne : seuls les `\n` explicites coupent).
    fn layout(&self, wrap_width: Option<f32>) -> TextLayout {
        TextLayout::wrapped(&self.display(), self.size, FontWeight::Regular, false, wrap_width)
    }

    /// Largeur réservée à l'icône de préfixe (0 si aucune).
    fn prefix_w(&self) -> f32 {
        if self.prefix.is_some() {
            ICON_SIZE + ICON_PAD
        } else {
            0.0
        }
    }

    /// Largeur réservée à l'icône de suffixe (0 si aucune).
    fn suffix_w(&self) -> f32 {
        if self.suffix.is_some() {
            ICON_SIZE + ICON_PAD
        } else {
            0.0
        }
    }

    /// Largeur du texte (entre padding et icônes) pour une largeur de widget donnée.
    fn content_width(&self, width: f32) -> f32 {
        (width - (PAD_X + self.prefix_w()) - PAD_X - self.suffix_w()).max(0.0)
    }

    /// Hauteur réservée au label au-dessus de la boîte (0 si aucun label).
    fn label_block(&self) -> f32 {
        if self.label.is_some() {
            frus_text::line_height(LABEL_SIZE) + DECO_GAP
        } else {
            0.0
        }
    }

    /// Hauteur réservée à la ligne d'aide/erreur sous la boîte (0 si aucune).
    fn sub_block(&self) -> f32 {
        if self.error.is_some() || self.helper.is_some() {
            frus_text::line_height(SUB_SIZE) + DECO_GAP
        } else {
            0.0
        }
    }

    /// Hauteur de la boîte de saisie elle-même (hors décoration) : une ligne en
    /// simple, `rows` lignes en multi-lignes.
    fn field_height(&self) -> f32 {
        let lines = if self.multiline { self.rows.max(1) as f32 } else { 1.0 };
        (frus_text::line_height(self.size) * lines + PAD_Y * 2.0).ceil()
    }
}

impl<Msg: Clone> Widget<Msg> for TextInput<Msg> {
    fn style(&self) -> Style {
        let height = self.label_block() + self.field_height() + self.sub_block();
        Style {
            width: self.width,
            height: Dimension::Length(height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let has_error = self.error.is_some();
        let fp = status.focus_progress.clamp(0.0, 1.0);

        // Décoration : label au-dessus, boîte de saisie au milieu, aide/erreur
        // en dessous. La boîte est le sous-rectangle où vit toute l'édition.
        let label_block = self.label_block();
        let field = Rect::new(
            bounds.x,
            bounds.y + label_block,
            bounds.width,
            self.field_height(),
        );

        // Label **flottant** (façon Material) : au repos (champ vide et non focalisé)
        // il occupe la boîte, à la place de l'indice ; focalisé OU rempli, il flotte
        // au-dessus, réduit. La progression de position/taille suit ce « flottement »
        // (`t`) ; la **couleur** suit la focalisation (`fp`) — un champ rempli mais non
        // focalisé garde un label discret, pas encore accentué.
        let float_t = if self.value.is_empty() { fp } else { 1.0 };
        let content_left = field.x + PAD_X + self.prefix_w();
        if let Some(label) = &self.label {
            let rest = Point::new(content_left, field.y + PAD_Y);
            let x = rest.x + (bounds.x - rest.x) * float_t;
            let y = rest.y + (bounds.y - rest.y) * float_t;
            let size = self.size + (LABEL_SIZE - self.size) * float_t;
            let color = if has_error {
                theme.error
            } else {
                theme.muted.lerp(theme.focus, fp)
            };
            scene.text(Point::new(x, y), label.clone(), size, color.fade(o));
        }

        // Ligne d'aide/erreur sous la boîte (l'erreur prend le pas sur l'aide).
        let sub = self.error.as_ref().or(self.helper.as_ref());
        if let Some(sub) = sub {
            let color = if has_error { theme.error } else { theme.muted };
            scene.text(
                Point::new(bounds.x, field.y + field.height + DECO_GAP),
                sub.clone(),
                SUB_SIZE,
                color.fade(o),
            );
        }

        // Bordure animée selon la progression de focus (repos → focus), rouge en erreur.
        let border_color = if has_error {
            theme.error
        } else {
            theme.border.lerp(theme.focus, fp)
        }
        .fade(o);
        let border_width = 1.0 + fp;
        scene.draw_rect(field, theme.surface.fade(o), theme.radius, border_width, border_color);

        // Icônes décoratives, centrées verticalement dans la boîte (couleur discrète).
        let icon_color = theme.muted.fade(o);
        let icon_y = field.y + (field.height - ICON_SIZE) * 0.5;
        let icon_scale = ICON_SIZE / 24.0;
        if let Some(prefix) = self.prefix {
            let path = prefix.path().scaled(icon_scale).translated(field.x + ICON_PAD, icon_y);
            scene.fill_path(&path, icon_color);
        }
        if let Some(suffix) = self.suffix {
            let x = field.x + field.width - ICON_SIZE - ICON_PAD;
            let path = suffix.path().scaled(icon_scale).translated(x, icon_y);
            scene.fill_path(&path, icon_color);
        }

        let len = self.value.chars().count();
        // Le contenu est inséré entre les icônes de préfixe/suffixe (le cas échéant).
        let left = PAD_X + self.prefix_w();
        let content_x = field.x + left;
        let content_w = self.content_width(field.width);
        let text_y = field.y + PAD_Y;
        // Multi-lignes : le texte se **replie** à la largeur de contenu — même
        // `max_width` pour la mesure (caret/hit) et pour le rendu → replis identiques.
        let wrap = if self.multiline { Some(content_w) } else { None };
        let layout = self.layout(wrap);

        // Indice (placeholder) : affiché quand le champ est vide. S'il y a aussi un
        // label, l'indice ne se révèle (en fondu) que lorsque le label a flotté —
        // sinon les deux se chevaucheraient dans la boîte.
        if self.value.is_empty() {
            if let Some(placeholder) = &self.placeholder {
                let ph_alpha = if self.label.is_some() { o * fp } else { o };
                if ph_alpha > 0.01 {
                    scene.text(
                        Point::new(content_x, text_y),
                        placeholder.clone(),
                        self.size,
                        theme.muted.fade(ph_alpha),
                    );
                }
            }
        }

        // Défilement pour garder le caret visible : horizontal (toujours) et vertical
        // (multi-lignes). Recalculé depuis le curseur, comme au clic (`cursor_at`).
        let cursor = status.cursor.unwrap_or(len).min(len);
        let caret = layout.caret_rect(cursor);
        let scroll = if status.focused { (caret.x - content_w).max(0.0) } else { 0.0 };
        let text_x = content_x - scroll;
        // Défilement vertical **retenu** (molette/barre, suivi du caret par le shell) ;
        // borné au dépassement du contenu sur la boîte.
        let vscroll = if self.multiline {
            let overflow = (layout.size().height - (field.height - PAD_Y * 2.0)).max(0.0);
            status.scroll_y.clamp(0.0, overflow)
        } else {
            0.0
        };
        // Origine verticale du contenu (décalée par le défilement multi-lignes).
        let text_top = text_y - vscroll;

        // Découpe au cadre de contenu (sinon le texte déborde sur les voisins).
        let content_clip = scene
            .current_clip()
            .intersect(Rect::new(content_x, field.y, content_w, field.height));
        scene.set_clip(content_clip);

        // Surbrillance de sélection (sous le texte).
        if status.focused {
            if let Some((start, end)) = status.selection {
                for r in layout.selection_rects(start.min(len), end.min(len)) {
                    scene.fill_rect(
                        Rect::new(text_x + r.x, text_top + r.y, r.width, r.height),
                        theme.selection.fade(o),
                    );
                }
            }
        }

        if !self.value.is_empty() {
            let pos = Point::new(text_x, text_top);
            let color = theme.on_surface.fade(o);
            match wrap {
                // Multi-lignes : le rendu se replie comme la mesure.
                Some(max_w) => {
                    scene.text_wrapped(pos, self.display(), &TextStyle::new(self.size), color, max_w)
                }
                None => scene.text(pos, self.display(), self.size, color),
            }
        }

        // Région de **composition** IME : soulignée (texte provisoire, façon
        // Material/Flutter). Dessinée par-dessus le texte, sous la ligne de base.
        if status.focused {
            if let Some((start, end)) = status.composing {
                let (start, end) = (start.min(len), end.min(len));
                for r in layout.selection_rects(start, end) {
                    scene.fill_rect(
                        Rect::new(
                            text_x + r.x,
                            text_top + r.y + r.height - 1.5,
                            r.width,
                            1.5,
                        ),
                        theme.on_surface.fade(o * 0.7),
                    );
                }
            }
        }

        // Curseur.
        if status.focused {
            scene.fill_rect(
                Rect::new(text_x + caret.x, text_top + caret.y, 2.0, caret.height),
                theme.on_surface.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        let mut chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let mut cursor = edit.cursor.min(len);
        let mut anchor = edit.anchor.map(|a| a.min(len));
        let selection = anchor
            .map(|a| (a.min(cursor), a.max(cursor)))
            .filter(|(s, e)| s < e);

        let mut changed = false;

        match key {
            Key::Text(text) => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                }
                let inserted: Vec<char> = text.chars().collect();
                let n = inserted.len();
                chars.splice(cursor..cursor, inserted);
                cursor += n;
                anchor = None;
                changed = true;
            }
            Key::Backspace => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                    changed = true;
                } else if cursor > 0 {
                    chars.remove(cursor - 1);
                    cursor -= 1;
                    changed = true;
                }
                anchor = None;
            }
            Key::Delete => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                    changed = true;
                } else if cursor < len {
                    chars.remove(cursor);
                    changed = true;
                }
                anchor = None;
            }
            Key::Left { shift } => {
                let target = cursor.saturating_sub(1);
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            Key::Right { shift } => {
                let target = (cursor + 1).min(len);
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            Key::Home { shift } => move_cursor(&mut cursor, &mut anchor, 0, *shift),
            Key::End { shift } => move_cursor(&mut cursor, &mut anchor, len, *shift),
            // Échap ne concerne pas l'édition (routé feuille→racine par le shell).
            Key::Escape => {}
            Key::Enter if self.multiline => {
                // Multi-lignes : Entrée **insère un saut de ligne** (pas de soumission).
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                }
                chars.insert(cursor, '\n');
                cursor += 1;
                anchor = None;
                changed = true;
            }
            Key::Enter => {
                // Simple : Entrée valide — n'altère pas la valeur, émet la soumission.
                edit.cursor = cursor;
                edit.anchor = anchor;
                return self.on_submit.clone();
            }
        }

        edit.cursor = cursor;
        edit.anchor = anchor;

        if changed {
            let new_value: String = chars.into_iter().collect();
            self.on_input.as_ref().map(|make| make(new_value))
        } else {
            None
        }
    }

    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        scroll_cursor: usize,
    ) -> Option<usize> {
        // Recompose la même géométrie de contenu que le rendu (insets d'icônes et de
        // décoration, repli, défilement horizontal), pour un clic exact. Le
        // **défilement vertical retenu** est déjà intégré à `local_y` par le shell.
        let left = PAD_X + self.prefix_w();
        let content_w = self.content_width(width);
        let layout = self.layout(if self.multiline { Some(content_w) } else { None });
        let scroll = (layout.caret_rect(scroll_cursor).x - content_w).max(0.0);
        // `local_*` sont relatifs au coin haut-gauche du **widget** (label compris) :
        // on retire la bande du label et le padding pour tomber dans le texte.
        let target_x = local_x - left + scroll;
        let target_y = local_y - self.label_block() - PAD_Y;
        Some(layout.hit_test(Point::new(target_x, target_y)))
    }

    fn text_metrics(&self, width: f32, cursor: usize) -> Option<(f32, f32, f32, f32)> {
        if !self.multiline {
            return None;
        }
        let layout = self.layout(Some(self.content_width(width)));
        let caret = layout.caret_rect(cursor);
        let visible = self.field_height() - PAD_Y * 2.0;
        Some((layout.size().height, visible, caret.y, caret.height))
    }

    fn text_viewport(&self, rect: Rect) -> Option<Rect> {
        if !self.multiline {
            return None;
        }
        // La boîte de saisie : sous le label, de la hauteur des `rows`.
        Some(Rect::new(rect.x, rect.y + self.label_block(), rect.width, self.field_height()))
    }

    fn caret_vertical(
        &self,
        width: f32,
        cursor: usize,
        down: bool,
        page: bool,
        goal_x: Option<f32>,
    ) -> Option<(usize, f32)> {
        if !self.multiline {
            return None;
        }
        let layout = self.layout(Some(self.content_width(width)));
        let caret = layout.caret_rect(cursor);
        let line_h = caret.height;
        // Colonne visée : la cible mémorisée, sinon la colonne courante.
        let x = goal_x.unwrap_or(caret.x);
        // Pas d'une ligne, ou d'une page (hauteur visible du champ, au moins 1 ligne).
        let step = if page {
            (self.field_height() - PAD_Y * 2.0).max(line_h)
        } else {
            line_h
        };
        let center = caret.y + line_h * 0.5;
        let target_y = if down { center + step } else { center - step };
        let full = layout.size().height;
        if page {
            // Page : on borne au champ (on n'en sort pas).
            let clamped = target_y.clamp(0.0, (full - line_h * 0.5).max(0.0));
            Some((layout.hit_test(Point::new(x, clamped)), x))
        } else {
            // Ligne : déjà à la première/dernière → le shell navigue le focus.
            if target_y < 0.0 || target_y >= full {
                return None;
            }
            Some((layout.hit_test(Point::new(x, target_y)), x))
        }
    }

    fn text_value(&self) -> Option<&str> {
        Some(&self.value)
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let mut s = frus_core::Semantics::new(frus_core::Role::TextInput).value(self.value.clone());
        // Le label annonce le champ ; une erreur s'y ajoute pour être lue.
        let label = match (&self.label, &self.error) {
            (Some(l), Some(e)) => Some(format!("{l}, {e}")),
            (Some(l), None) => Some(l.clone()),
            (None, Some(e)) => Some(e.clone()),
            (None, None) => None,
        };
        if let Some(label) = label {
            s = s.label(label);
        }
        Some(s)
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let cursor = edit.cursor.min(len);
        let anchor = edit.anchor?.min(len);
        let (start, end) = (anchor.min(cursor), anchor.max(cursor));
        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        if len == 0 {
            return Some((0, 0));
        }
        let i = index.min(len - 1);
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        if !is_word(chars[i]) {
            return Some((i, (i + 1).min(len)));
        }
        let mut start = i;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = i + 1;
        while end < len && is_word(chars[end]) {
            end += 1;
        }
        Some((start, end))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn draws_own_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Changed(String),
        Submitted,
    }

    fn input(value: &str) -> TextInput<Msg> {
        TextInput::new(value).on_input(Msg::Changed)
    }

    #[test]
    fn text_is_clipped_to_content_box() {
        // Un texte plus long que le champ doit être découpé à sa largeur de contenu
        // (sinon il déborde sur les widgets voisins).
        let inp = input("a very long value that overflows the field width").width(100.0);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &inp,
            Rect::new(0.0, 0.0, 100.0, 30.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let clip = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { clip, .. } => Some(*clip),
                _ => None,
            })
            .expect("primitive de texte");
        assert!((clip.width - (100.0 - PAD_X * 2.0)).abs() < 0.5, "découpe = {clip:?}");
    }

    #[test]
    fn composing_region_draws_an_underline() {
        // La région composée ajoute des rectangles fins (soulignement) sous le
        // texte, absents quand aucune composition n'est en cours.
        let inp = input("konnichiwa");
        let base = Status { focused: true, cursor: Some(5), ..Default::default() };
        let composing = Status { composing: Some((0, 5)), ..base };

        let count_thin_rects = |status: Status| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(&inp, Rect::new(0.0, 0.0, 220.0, 30.0), status, &Theme::default(), &mut scene);
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.height <= 2.0 && rect.width > 2.0))
                .count()
        };
        let plain = count_thin_rects(base);
        let underlined = count_thin_rects(composing);
        assert!(underlined > plain, "la composition doit souligner ({plain} → {underlined})");
    }

    #[test]
    fn text_value_exposes_the_field_content() {
        // Le contexte de saisie (suggestions IME) lit la valeur du champ.
        let inp = input("hello");
        assert_eq!(Widget::<Msg>::text_value(&inp), Some("hello"));
    }

    #[test]
    fn cursor_at_accounts_for_scroll() {
        // Champ étroit + texte long → défilement quand le curseur est en fin.
        let inp = input("0123456789 abcdefghij");
        // Sans défilement (curseur 0) : clic au bord gauche → index 0.
        assert_eq!(
            Widget::<Msg>::cursor_at(&inp, PAD_X, PAD_Y, 80.0, 0),
            Some(0)
        );
        // Défilé (curseur en fin) : clic à droite tombe sur un index plus grand
        // qu'un clic à gauche (le décalage est bien pris en compte).
        let end = inp.value.chars().count();
        let at_left = Widget::<Msg>::cursor_at(&inp, PAD_X, PAD_Y, 80.0, end).unwrap();
        let at_right = Widget::<Msg>::cursor_at(&inp, 76.0, PAD_Y, 80.0, end).unwrap();
        assert!(at_left > 0, "défilé : le bord gauche n'est plus l'index 0");
        assert!(at_right > at_left, "clic à droite → index plus grand");
    }

    #[test]
    fn insert_at_cursor() {
        let inp = input("ac");
        let mut edit = Edit { cursor: 1, anchor: None, composing: None };
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Text("b".to_string())),
            Some(Msg::Changed("abc".to_string()))
        );
        assert_eq!(edit.cursor, 2);
    }

    #[test]
    fn shift_arrow_selects_then_delete() {
        let inp = input("hello");
        // Curseur en fin, Shift+Left deux fois -> sélectionne "lo".
        let mut edit = Edit { cursor: 5, anchor: None, composing: None };
        inp.on_edit(&mut edit, &Key::Left { shift: true });
        inp.on_edit(&mut edit, &Key::Left { shift: true });
        assert_eq!(edit.selection_range(), Some((3, 5)));
        // Backspace supprime la sélection.
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Backspace),
            Some(Msg::Changed("hel".to_string()))
        );
        assert_eq!(edit.cursor, 3);
        assert_eq!(edit.anchor, None);
    }

    #[test]
    fn home_end_bounds() {
        let inp = input("abc");
        let mut edit = Edit { cursor: 1, anchor: None, composing: None };
        inp.on_edit(&mut edit, &Key::End { shift: false });
        assert_eq!(edit.cursor, 3);
        inp.on_edit(&mut edit, &Key::Home { shift: false });
        assert_eq!(edit.cursor, 0);
    }

    #[test]
    fn selected_text_reads_range() {
        let inp = input("hello");
        let edit = Edit { cursor: 5, anchor: Some(2), composing: None };
        assert_eq!(inp.selected_text(&edit), Some("llo".to_string()));
    }

    #[test]
    fn enter_submits_without_changing_value() {
        let inp = input("acheter du lait").on_submit(Msg::Submitted);
        let mut edit = Edit { cursor: 3, anchor: None, composing: None };
        // Entrée : émet la soumission, ne renvoie pas de changement de valeur.
        assert_eq!(inp.on_edit(&mut edit, &Key::Enter), Some(Msg::Submitted));
        assert_eq!(edit.cursor, 3); // curseur inchangé
    }

    #[test]
    fn enter_without_submit_is_noop() {
        let inp = input("x");
        let mut edit = Edit { cursor: 1, anchor: None, composing: None };
        assert_eq!(inp.on_edit(&mut edit, &Key::Enter), None);
    }

    #[test]
    fn obscure_masks_the_displayed_text_but_not_the_value() {
        // Champ mot de passe : le rendu ne contient jamais la valeur en clair, mais
        // des points ; la valeur réelle reste accessible (édition, IME).
        let theme = Theme::default();
        let field = TextInput::<Msg>::new("secret").obscure(true);
        let mut scene = Scene::new();
        let status = Status { focused: true, cursor: Some(6), ..Default::default() };
        Widget::<Msg>::paint(&field, Rect::new(0.0, 0.0, 220.0, 30.0), status, &theme, &mut scene);
        let drawn: Vec<&str> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(drawn.iter().all(|t| !t.contains("secret")), "la valeur ne doit pas fuiter");
        assert!(drawn.iter().any(|t| t.chars().all(|c| c == '•')), "des points sont dessinés");
        // La vraie valeur reste exposée au contexte de saisie.
        assert_eq!(Widget::<Msg>::text_value(&field), Some("secret"));
    }

    #[test]
    fn prefix_icon_draws_a_path_and_shifts_the_hit_test() {
        // Une icône de préfixe dessine un chemin et décale le contenu vers la droite :
        // un même clic tombe sur un index plus petit qu'en son absence.
        let theme = Theme::default();
        let with_icon = input("hello world").prefix_icon(IconName::Star);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(&with_icon, Rect::new(0.0, 0.0, 220.0, 30.0), Status::default(), &theme, &mut scene);
        assert!(
            scene.primitives().iter().any(|p| matches!(p, frus_core::Primitive::Path { .. })),
            "l'icône de préfixe dessine un chemin"
        );
        let plain = input("hello world");
        let at_plain = Widget::<Msg>::cursor_at(&plain, 60.0, PAD_Y, 220.0, 0).unwrap();
        let at_icon = Widget::<Msg>::cursor_at(&with_icon, 60.0, PAD_Y, 220.0, 0).unwrap();
        assert!(at_icon < at_plain, "le préfixe décale le contenu ({at_plain} → {at_icon})");
    }

    #[test]
    fn multiline_enter_inserts_a_newline_instead_of_submitting() {
        // En multi-lignes, Entrée insère « \n » (et n'émet pas la soumission).
        let inp = TextInput::<Msg>::new("ab").on_input(Msg::Changed).multiline();
        let mut edit = Edit { cursor: 2, anchor: None, composing: None };
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Enter),
            Some(Msg::Changed("ab\n".to_string()))
        );
        assert_eq!(edit.cursor, 3);
        // Simple ligne : Entrée soumet (comportement inchangé).
        let single = input("ab").on_submit(Msg::Submitted);
        let mut edit = Edit { cursor: 2, anchor: None, composing: None };
        assert_eq!(single.on_edit(&mut edit, &Key::Enter), Some(Msg::Submitted));
    }

    #[test]
    fn multiline_wraps_long_lines_to_the_width() {
        // Une longue ligne **sans** `\n` se replie sur plusieurs lignes visuelles :
        // un clic bien plus bas que la 1re ligne place le curseur plus loin dans le
        // texte (une ligne repliée sous la première), pas à l'index 0.
        let long = "word ".repeat(30); // 150 caractères, aucune coupure explicite
        let inp = TextInput::<Msg>::new(long.trim_end()).on_input(Msg::Changed).rows(4).width(160.0);
        let line_h = frus_text::line_height(inp.size);
        let top = Widget::<Msg>::cursor_at(&inp, PAD_X + 2.0, PAD_Y + 1.0, 160.0, 0).unwrap();
        let wrapped =
            Widget::<Msg>::cursor_at(&inp, PAD_X + 2.0, PAD_Y + line_h * 2.0 + 1.0, 160.0, 0).unwrap();
        assert!(top < 10, "1re ligne : {top}");
        assert!(wrapped > top, "une ligne repliée plus bas → index plus loin ({top} → {wrapped})");
    }

    #[test]
    fn multiline_reports_overflow_and_scrolls_content() {
        // Cinq lignes dans une boîte de deux : le contenu déborde…
        let inp = TextInput::<Msg>::new("l1\nl2\nl3\nl4\nl5").on_input(Msg::Changed).rows(2).width(200.0);
        let (content_h, visible_h, _, _) =
            Widget::<Msg>::text_metrics(&inp, 200.0, 0).expect("champ multi-lignes");
        assert!(content_h > visible_h + 1.0, "5 lignes > boîte de 2 ({content_h} vs {visible_h})");

        // …et un défilement retenu décale le texte vers le haut (position.y plus petite).
        let text_top = |scroll: f32| {
            let mut scene = Scene::new();
            let status = Status { focused: true, cursor: Some(0), scroll_y: scroll, ..Default::default() };
            Widget::<Msg>::paint(&inp, Rect::new(0.0, 0.0, 200.0, 80.0), status, &Theme::default(), &mut scene);
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, position, .. } if text.contains("l1") => {
                        Some(position.y)
                    }
                    _ => None,
                })
                .expect("texte du champ")
        };
        assert!(text_top(20.0) < text_top(0.0), "défiler remonte le contenu");
    }

    #[test]
    fn multiline_arrows_move_the_caret_between_lines() {
        // "abc\ndefg\nhi" : ligne 0 = indices 0..3, ligne 1 = 4..8, ligne 2 = 9..11.
        let inp = TextInput::<Msg>::new("abc\ndefg\nhi").on_input(Msg::Changed).rows(3).width(200.0);
        // Depuis la 1re ligne (index 1), descendre tombe sur la 2e ligne.
        let (down, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 1, true, false, None).unwrap();
        assert!((4..=8).contains(&down), "descend sur la 2e ligne : {down}");
        // 1re ligne, monter → impossible (le shell navigue le focus).
        assert_eq!(Widget::<Msg>::caret_vertical(&inp, 200.0, 1, false, false, None), None);
        // Dernière ligne (index 10), descendre → impossible.
        assert_eq!(Widget::<Msg>::caret_vertical(&inp, 200.0, 10, true, false, None), None);
        // 2e ligne (index 5), monter tombe sur la 1re.
        let (up, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 5, false, false, None).unwrap();
        assert!(up <= 3, "monte sur la 1re ligne : {up}");
        // Champ mono-ligne : jamais de mouvement vertical.
        assert_eq!(Widget::<Msg>::caret_vertical(&input("abc"), 200.0, 1, true, false, None), None);
    }

    #[test]
    fn multiline_goal_column_survives_a_short_line() {
        // "hello\nhi\nworld" : ligne 0 = 0..5, ligne 1 = 6..8 (courte), ligne 2 = 9..14.
        // Partant de la colonne 5 (fin de "hello"), on descend : la 2e ligne "hi" est
        // trop courte (bornée à sa fin), mais la colonne cible rendue reste ~celle de
        // départ, si bien qu'on retombe sur la colonne 5 en descendant encore.
        let inp =
            TextInput::<Msg>::new("hello\nhi\nworld").on_input(Msg::Changed).rows(3).width(200.0);
        let (mid, goal) = Widget::<Msg>::caret_vertical(&inp, 200.0, 5, true, false, None).unwrap();
        assert!((6..=8).contains(&mid), "ligne courte, bornée à sa fin : {mid}");
        // Deuxième saut en réutilisant la colonne cible : sans mémoire on serait resté
        // borné à la fin de "hi" (col. ~2) ; ici on retombe loin dans "world" (col. ~5),
        // preuve que la colonne d'origine est préservée par-dessus la ligne courte.
        let (low, _) =
            Widget::<Msg>::caret_vertical(&inp, 200.0, mid, true, false, Some(goal)).unwrap();
        assert!((12..=14).contains(&low), "colonne cible préservée dans \"world\" : {low}");
    }

    #[test]
    fn multiline_page_jump_is_clamped_to_the_field() {
        // Page haut/bas ne quittent jamais le champ multi-lignes : aux bornes, le
        // curseur se cale au début / à la fin et rend `Some` (pas `None` → on reste).
        let inp =
            TextInput::<Msg>::new("a\nb\nc\nd\ne").on_input(Msg::Changed).rows(2).width(200.0);
        // Depuis la dernière ligne, PgDn se cale en bas (on ne quitte pas le champ).
        let (bottom, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 8, true, true, None).unwrap();
        assert!(bottom >= 7, "PgDn borné au bas du champ : {bottom}");
        // Depuis la 1re ligne, PgUp se cale en haut.
        let (top, _) = Widget::<Msg>::caret_vertical(&inp, 200.0, 0, false, true, None).unwrap();
        assert!(top <= 1, "PgUp borné au haut du champ : {top}");
    }

    #[test]
    fn multiline_reserves_rows_of_height() {
        // `rows(n)` réserve n lignes ; c'est plus haut qu'un champ d'une ligne.
        let one = match Widget::<Msg>::style(&input("x")).height {
            Dimension::Length(h) => h,
            _ => panic!("hauteur fixe"),
        };
        let four = match Widget::<Msg>::style(&TextInput::<Msg>::new("x").rows(4)).height {
            Dimension::Length(h) => h,
            _ => panic!("hauteur fixe"),
        };
        assert!(four > one * 2.0, "4 lignes bien plus hautes qu'une ({one} → {four})");
    }

    #[test]
    fn multiline_hit_test_uses_the_click_line() {
        // Un clic sur la 2e ligne place le curseur dans « cd » (indices ≥ 3), pas
        // dans « ab » de la 1re ligne.
        let inp = TextInput::<Msg>::new("ab\ncd").on_input(Msg::Changed).rows(3);
        let line_h = frus_text::line_height(inp.size);
        // Clic haut-gauche → 1re ligne (indice ≤ 2).
        let top = Widget::<Msg>::cursor_at(&inp, PAD_X + 1.0, PAD_Y + 1.0, 220.0, 0).unwrap();
        // Clic une ligne plus bas → 2e ligne (indice ≥ 3).
        let below =
            Widget::<Msg>::cursor_at(&inp, PAD_X + 1.0, PAD_Y + line_h + 1.0, 220.0, 0).unwrap();
        assert!(top <= 2, "1re ligne : {top}");
        assert!(below >= 3, "2e ligne : {below}");
    }

    #[test]
    fn floating_label_rests_in_box_then_floats_up() {
        // Le label repose dans la boîte (grand, bas) quand le champ est vide et non
        // focalisé, et flotte au-dessus (petit, haut) une fois focalisé.
        let theme = Theme::default();
        let field = TextInput::<Msg>::new("").label("Name");
        let label_geo = |status: Status| -> (f32, f32) {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(&field, Rect::new(0.0, 30.0, 220.0, 60.0), status, &theme, &mut scene);
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, size, position, .. } if text == "Name" => {
                        Some((*size, position.y))
                    }
                    _ => None,
                })
                .expect("label dessiné")
        };
        let (rest_size, rest_y) = label_geo(Status::default());
        let (float_size, float_y) = label_geo(Status { focused: true, focus_progress: 1.0, ..Default::default() });
        assert!(rest_size > float_size, "au repos le label est plus grand ({rest_size} → {float_size})");
        assert!(float_y < rest_y, "focalisé le label monte ({rest_y} → {float_y})");
    }

    #[test]
    fn decoration_grows_height_for_label_and_error() {
        // Un champ nu ne réserve que la boîte ; un label + une erreur ajoutent une
        // ligne au-dessus et une en dessous → le style est plus haut.
        let bare_h = match Widget::<Msg>::style(&input("x")).height {
            Dimension::Length(h) => h,
            _ => panic!("hauteur fixe attendue"),
        };
        let decorated = input("x").label("Email").error("Required");
        let deco_h = match Widget::<Msg>::style(&decorated).height {
            Dimension::Length(h) => h,
            _ => panic!("hauteur fixe attendue"),
        };
        assert!(deco_h > bare_h, "label + erreur agrandissent le champ ({bare_h} → {deco_h})");
    }

    #[test]
    fn error_paints_the_border_in_the_error_color() {
        // En erreur, la bordure de la boîte passe en couleur d'erreur du thème.
        let theme = Theme::default();
        let field = input("x").error("bad");
        let mut scene = Scene::new();
        Widget::<Msg>::paint(&field, Rect::new(0.0, 0.0, 220.0, 60.0), Status::default(), &theme, &mut scene);
        let has_error_border = scene.primitives().iter().any(|p| matches!(
            p,
            frus_core::Primitive::Rect { border_color, border_width, .. }
                if *border_width > 0.0 && *border_color == theme.error
        ));
        assert!(has_error_border, "la bordure doit être en couleur d'erreur");
    }

    #[test]
    fn placeholder_shows_only_when_empty() {
        // L'indice n'apparaît que si la valeur est vide.
        let theme = Theme::default();
        let count_texts = |value: &str| {
            let field = TextInput::<Msg>::new(value).placeholder("Type here");
            let mut scene = Scene::new();
            Widget::<Msg>::paint(&field, Rect::new(0.0, 0.0, 220.0, 30.0), Status::default(), &theme, &mut scene);
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "Type here"))
                .count()
        };
        assert_eq!(count_texts(""), 1, "vide : l'indice s'affiche");
        assert_eq!(count_texts("hi"), 0, "rempli : pas d'indice");
    }

    #[test]
    fn word_at_finds_word_bounds() {
        let inp = input("bonjour le monde");
        // index 10 tombe dans "monde" (indices 10..15) — non, "le" est 8..10.
        assert_eq!(inp.word_at(2), Some((0, 7))); // "bonjour"
        assert_eq!(inp.word_at(11), Some((11, 16))); // "monde"
    }
}
