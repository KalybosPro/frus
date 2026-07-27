# Jalon 140 — Barre de défilement du champ multi-lignes (+ tactile)

## Analyse

Le jalon 139 a rendu le champ multi-lignes défilable à la **molette**, mais sans
affordance visible ni moyen tactile : au doigt, un appui démarre une **sélection** de
texte, pas un défilement. La pièce manquante est une **barre de défilement** — qui, dans
frus, est déjà glissable à la souris **et au tactile** (le geste `Drag::Scrollbar` est
agnostique de la source). Une barre couvre donc les deux besoins d'un coup.

## Décisions techniques

- **Réutiliser la barre générique.** Là où le champ s'enregistrait comme région
  scrollable (jalon 139), il appelle maintenant aussi `add_scrollbar(id, viewport, …)` —
  exactement comme un `Scroll` ou une liste virtuelle. La barre est dessinée, sa poignée
  enregistrée (`scrollbar_at`), et le shell la fait glisser via `Drag::Scrollbar` au clic
  **comme au doigt** (le `pointer_down` teste `scrollbar_at` pour toute source).

- **La barre épouse la boîte, pas le widget.** La région scrollable et la barre doivent
  courir le long de la **boîte de saisie**, pas du widget entier (qui inclut le label
  flottant au-dessus). Une méthode `Widget::text_viewport(rect)` rend ce cadre (sous le
  label, de la hauteur des `rows`) ; l'enregistrement scrollable et la barre l'emploient,
  si bien que la poignée s'aligne pile sur le texte défilable.

- **Rien de neuf côté interaction.** Molette, inertie, dépassement élastique, glissement
  de poignée : tout vient de la machinerie de défilement existante. Le champ ne fait que
  **s'y déclarer** (région + barre) via `text_metrics` (dépassement) et `text_viewport`
  (cadre).

## Implémentation

- `widget.rs` (+ relais `Box`/`Keyed`/`Responsive`) : méthode `text_viewport`.
- `textinput.rs` : impl `text_viewport` (boîte sous le label, hauteur `field_height`).
- `ui.rs` : le walk enregistre la région **et** ajoute la barre sur ce cadre, avec
  l'offset retenu courant.

## Vérification

- **Rendu à l'œil** : la barre longe le bord droit de la boîte (sous le label « Notes »),
  la poignée reflétant le défilement — golden `multiline_scrolled` régénéré.
- **Non-régression** : suite `frus-widgets` + `frus-test` verte ; le champ court reste
  sans barre (pas de dépassement).
- La poignée réutilise le glissement `Drag::Scrollbar` déjà couvert par les tests de
  défilement (souris et tactile passent par le même `pointer_down`).

## Reste

- **Défilement au doigt directement sur le texte** (fling) : entre toujours en conflit
  avec la sélection ; laissé tel quel (la barre est l'affordance tactile).
- **Auto-masquage** de la barre (n'apparaître qu'au survol/défilement), façon overlay
  Material.
- **Flèches ↑/↓** déplaçant le caret entre lignes (jalon suivant).
