//! **Sémantique d'accessibilité** : l'annotation par widget (rôle, libellé,
//! valeur, état) que le framework expose aux technologies d'assistance.
//!
//! Type frus-natif, zéro-dépendance, **mappable** vers `accesskit` au bord
//! plateforme (le shell construit un arbre AccessKit à partir de ces nœuds).
//! On suit le conseil du §14 : *baker le libellé dans les widgets dès
//! maintenant, brancher AccessKit ensuite.*

/// Le **rôle** sémantique d'un élément (sous-ensemble aligné sur les rôles
/// AccessKit/ARIA : ce qu'un lecteur d'écran annonce).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Role {
    /// Élément sans rôle propre (conteneur de mise en page).
    #[default]
    None,
    /// Texte statique (étiquette, paragraphe).
    Label,
    /// En-tête / titre.
    Heading,
    /// Bouton actionnable.
    Button,
    /// Lien.
    Link,
    /// Case à cocher (état `checked`).
    CheckBox,
    /// Interrupteur (état `checked`).
    Switch,
    /// Bouton radio (état `checked`).
    RadioButton,
    /// Curseur de valeur continue (`value`/`min`/`max`).
    Slider,
    /// Champ de saisie de texte (`value` = contenu).
    TextInput,
    /// Image / icône décrite par son `label`.
    Image,
    /// Onglet.
    Tab,
    /// Élément de liste.
    ListItem,
    /// Barre de progression (`value`).
    ProgressBar,
}

/// L'état coché d'un contrôle à bascule (case, interrupteur, radio).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Toggled {
    /// Non applicable (le contrôle n'est pas à bascule).
    #[default]
    None,
    /// Décoché.
    False,
    /// Coché.
    True,
}

/// L'annotation sémantique **résolue** d'un widget, pour l'accessibilité.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Semantics {
    /// Rôle annoncé.
    pub role: Role,
    /// Nom accessible (ce que le lecteur d'écran lit).
    pub label: Option<String>,
    /// Valeur textuelle (contenu d'un champ, position d'un curseur…).
    pub value: Option<String>,
    /// État coché (cases/interrupteurs/radios).
    pub toggled: Toggled,
    /// Actionnable au clic (le lecteur d'écran propose « activer »).
    pub clickable: bool,
    /// Désactivé (grisé, non interactif).
    pub disabled: bool,
    /// Bornes numériques `(min, value, max)` pour curseurs/progressions.
    pub range: Option<(f32, f32, f32)>,
}

impl Semantics {
    /// Une annotation de rôle donné, sans autre attribut.
    pub fn new(role: Role) -> Self {
        Self {
            role,
            ..Default::default()
        }
    }

    /// Fixe le nom accessible.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Fixe la valeur textuelle.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Marque l'état coché.
    pub fn toggled(mut self, on: bool) -> Self {
        self.toggled = if on { Toggled::True } else { Toggled::False };
        self
    }

    /// Marque l'élément actionnable.
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self
    }

    /// Marque l'élément désactivé.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fixe les bornes numériques `(min, value, max)`.
    pub fn range(mut self, min: f32, value: f32, max: f32) -> Self {
        self.range = Some((min, value, max));
        self
    }

    /// `true` si ce nœud porte une information utile à l'assistance (un rôle
    /// non nul ou un libellé) — les conteneurs vides sont ignorés de l'arbre.
    pub fn is_meaningful(&self) -> bool {
        self.role != Role::None || self.label.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_compose() {
        let s = Semantics::new(Role::CheckBox)
            .label("Notifications")
            .toggled(true);
        assert_eq!(s.role, Role::CheckBox);
        assert_eq!(s.label.as_deref(), Some("Notifications"));
        assert_eq!(s.toggled, Toggled::True);
        assert!(s.is_meaningful());
    }

    #[test]
    fn empty_is_not_meaningful() {
        assert!(!Semantics::default().is_meaningful());
        // Un simple libellé suffit à être exposé.
        assert!(Semantics::default().label("x").is_meaningful());
    }
}
