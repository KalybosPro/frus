# Jalon 16 — Bibliothèque de widgets nommés (themés)

Ajoute des composants applicatifs prêts à l'emploi, tous **themés** (lisent le
thème au paint) et animés.

## Widgets livrés

| Widget | Rôle |
|---|---|
| **`Button`** | bouton themé, variantes `Primary` / `Secondary` / `Danger`, hover/pressé, ombre |
| **`Checkbox`** | case à cocher **contrôlée** (case + ✓ + libellé) ; `on_toggle(bool)` |
| **`Switch`** | interrupteur pilule **contrôlé** ; `on_toggle(bool)` |
| **`RadioGroup`** | groupe radio à sélection unique ; `on_select(index)` |
| **`Dropdown`** | liste déroulante **contrôlée** (déploiement en place) ; `on_toggle` + `on_select(index)` |
| **`Slider`** | curseur `0..=1` **glissable** ; `on_change(f32)` |
| **`Card`** | surface themée (fond, bordure, rayon, ombre) avec un enfant |

## Nouveau mécanisme — drag générique de widget

Pour le `Slider` (et futures poignées), on généralise le glissement :

```rust
trait Widget { … fn draggable(&self) -> bool { false }
                  fn on_drag(&self, fraction: f32) -> Option<Msg> { None } }
Ui::draggable_at(point) -> Option<(WidgetId, Rect)>
```

Le shell : `MouseDown` sur un draggable → `Drag::Widget{id, rect}` (et applique
tout de suite) ; `CursorMoved` → `fraction = (x − rect.x)/rect.width` →
`find_widget(id).on_drag(fraction)` → `Msg` → `update`. Même schéma que la barre
de défilement, mais réutilisable par n'importe quel widget.

## Modèle

- Les widgets **contrôlés** (Checkbox/Switch/Slider/Dropdown/Radio) tirent leur
  valeur de l'état applicatif et émettent un message ; l'app met à jour l'état.
- `RadioGroup`/`Dropdown` sont des **conteneurs** (colonne d'options) : chaque
  option est un enfant cliquable → identité propre → l'ouverture/fermeture du
  Dropdown bénéficie gratuitement des **fondus** d'apparition/disparition (J13/J14).

## Démo

Une `Card` de réglages regroupe une checkbox « Terminé », un switch
« Notifications », un slider de volume, un `RadioGroup` de tailles et un
`Dropdown` — le tout themé (clair/sombre) et animé.

## Tests

- `Button::on_click`, `Checkbox::on_click` (renvoie `on_toggle(!checked)`),
  `Slider::on_drag` (fraction → valeur, bornée).

## Limites (v1)

- `Dropdown` se déploie **en place** (pas d'overlay flottant au-dessus du reste).
- Pas encore de `Switch`/`Slider` animés en transition d'état (position instantanée).
