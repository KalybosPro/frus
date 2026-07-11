# Jalon 48 — Glissement du tiroir en courbe de ressort

Le tiroir glissait **linéairement** (durée fixe des valeurs animées). Il suit
désormais une **courbe en ressort**, pour une arrivée douce cohérente avec les
transitions d'écran de l'app.

## `spring_ease(t)` — réponse indicielle critique

Une fonction fermée qui remappe la progression linéaire `t ∈ [0,1]` :

```text
y(τ) = 1 − e^(−ω·τ)·(1 + ω·τ)     (ω = 8, renormalisée pour f(1) = 1)
```

C'est la **réponse indicielle d'un ressort en amortissement critique** : départ
au repos (pente nulle), montée franche, décélération douce, **sans dépassement**
(`f(0) = 0`, `f(1) = 1`, monotone). Contrairement à `spring_step` (intégration
pas à pas avec vélocité, utilisée pour les gestes/écrans), c'est une forme
**fermée** — pas d'état de vélocité à conserver, idéale pour remapper une
progression déjà interpolée par le runtime.

Pas de dépassement : essentiel pour un panneau accosté à un bord (un dépassement
laisserait apparaître un interstice au bord de la fenêtre).

## Application

`process_overlays` applique `spring_ease` à la progression des seuls tiroirs
(`Placement::Left` / `Right`) avant d'en déduire le décalage de glissement et
l'opacité du voile. Les autres overlays (menus, tooltips, modales) gardent leur
progression brute.

Le runtime continue de piloter la progression **linéaire** `0↔1` (aucune
animation à câbler côté app, jalon 46) ; la courbe n'intervient qu'au **rendu**.

## Tests

- `frus-widgets` : `spring_ease` — `f(0)=0`, `f(1)=1`, croissante, bornée à `≤ 1`
  (aucun dépassement), déjà bien avancée à mi-parcours, bornée hors domaine.
- Le test de mi-animation du tiroir dérive désormais son attendu de la courbe
  (`spring_ease(0.5)·largeur`).

## Limites (v1)

- Courbe unique (amortissement critique) ; ni raideur ni rebond réglables.
