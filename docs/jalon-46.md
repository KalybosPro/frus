# Jalon 46 — Animation du tiroir (glissement + fondu)

Le `Drawer` du jalon 45 s'ouvrait/fermait instantanément. Il **glisse** désormais
depuis le bord gauche, voile compris, **sans aucun câblage côté application**.

## Principe : animation pilotée par le runtime

Le framework interpole déjà une valeur par widget vers la cible déclarée par
`Widget::anim_target` (mécanisme des interrupteurs, etc.), via
`Runtime::advance_values`, appelé à chaque frame par le shell. On s'y branche :

- `Drawer::anim_target()` renvoie `Some(1.0)` ouvert / `Some(0.0)` fermé. Le
  runtime fait tendre la **progression** `0↔1` vers cette cible et redemande une
  frame tant qu'elle bouge — l'app ne gère aucun ressort.
- Le `Drawer` propose **toujours** son overlay quand un panneau existe ; c'est la
  progression qui décide de l'affichage.

## Application de la progression

Le parcours de `build_ui` lit la progression animée du tiroir
(`Runtime::value_or(id, cible)` — la cible sert de repli au premier rendu, comme
au montage) et la joint à l'overlay. `process_overlays` l'exploite pour le
placement `Left` :

- **Glissement** : `pos.x = -(1 - progression) · largeur` (le panneau entre par la
  gauche) ;
- **Fondu du voile** : opacité `0.5 · progression` (synchronisée avec le
  glissement) ;
- progression ≤ 0 → overlay non émis (ni voile, ni panneau, ni zone de fermeture).

Les autres overlays (menus, tooltips, modales) n'ont pas d'`anim_target` : leur
progression vaut `1.0`, comportement inchangé.

## Nouveautés d'API

- `Runtime::value_or(id, default)` : valeur animée, ou `default` si le widget n'a
  jamais été animé (rendu isolé / montage).
- La pile d'overlays interne transporte une progression `0..=1`.

Aucune signature publique de `Drawer` ne change : `Drawer::new(open)` suffit,
l'animation vient en prime.

## Tests

- `frus-widgets` : `anim_target` reflète l'état ouvert/fermé ; tiroir fermé →
  aucun voile ; **mi-animation** (progression 0.5 injectée) → panneau à moitié
  rentré (bord droit ≈ largeur/2).

## Limites (v1)

- Ressort non utilisé ici : l'interpolation est linéaire (durée fixe partagée avec
  les autres valeurs animées) — suffisant et cohérent visuellement.
- Toujours un seul côté (`Left`).
