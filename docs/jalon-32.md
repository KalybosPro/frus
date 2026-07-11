# Jalon 32 — Nouveaux widgets (6)

Un lot de six widgets couvrant indicateurs, structure et layering.

## Widgets

- **`ProgressBar::new(value 0..1)`** — barre déterminée (piste `muted` +
  remplissage `primary`, arrondie ; valeur clampée).
- **`Divider::new()`** — fin séparateur horizontal `theme.border` (étiré par le parent).
- **`Badge::new(texte)`** — pastille d'accent (compteur / étiquette).
- **`Stack::new().layer(a).layer(b)`** — couches **superposées** (même boîte,
  dernière au-dessus). Chaque couche remplit la pile ; un positionnement fin se
  fait *dans* la couche (ex. `Flex` aligné). Traitée comme une branche spéciale
  dans `build_ui` (comme `Scroll`/`Navigator`) ; les couches sont des sous-arbres
  complets (overlays/scroll imbriqués permis, car stockées → lifetime `'a`).
- **`Tabs::new(selected, on_select).tab(label, contenu)`** — onglets **contrôlés** ;
  composite `[en-tête, panneau]` (seul le panneau sélectionné est réalisé).
- **`Spinner::new()`** — indicateur d'activité **animé en continu**.

## Animation continue (pour `Spinner`)

Nouveau mécanisme, réutilisable :

- `Widget::continuous(&self) -> bool` (défaut `false`) ; `Spinner` renvoie `true`.
- Le pilote positionne un drapeau `Ui::wants_animation()` dès qu'un widget continu
  est rencontré ; le shell **redessine tant qu'il est vrai**.
- Une **horloge** `Runtime.time` (secondes, avancée par le shell) est exposée aux
  widgets via `Status::time` — le `Spinner` en dérive sa phase de rotation. Base
  pour toute animation pilotée par le temps (pulsation, défilement continu…).

## Démo (intégration)

- En-tête : `Stack(Spinner + Badge)` (pastille du nombre de tâches actives sur le
  spinner) — montre Stack + Spinner + Badge ensemble.
- Carte todo : `Divider` + `ProgressBar` de complétion (terminées / total).
- Écran Réglages : `Tabs` [« Contrôles », « À propos »].

## Tests

- `ProgressBar` : remplissage ∝ valeur (0.5 → 50/100), clamp.
- `Divider` : ligne `border`.
- `Badge` : pastille + texte.
- `Stack` : les couches partagent la même origine (superposées).
- `Tabs` : `[en-tête(N boutons), panneau]` ; pas de panneau si sélection hors bornes.
- `Spinner` : couronne de `DOTS` points, distribution dépendante du temps,
  `continuous() == true`.
- 56 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Spinner` : nombre de points fixe ; couleur d'accent.
- `Stack` : couches plein-cadre (positionnement fin délégué à la couche).
- `ProgressBar` déterminée uniquement (indéterminée = un `Spinner`).
