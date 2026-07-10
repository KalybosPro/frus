# Jalon 19 — Transitions d'état · Geste retour · Overlay avancé

Trois raffinements d'interaction, construits en sous-lots buildés et testés.

## Lot A — Overlay avancé

- **Auto-flip** : un overlay ancré (`Below` / `Tooltip`) qui déborderait d'un
  bord de la fenêtre bascule de l'autre côté de l'ancre (vertical) ou est recalé
  dans la fenêtre (horizontal). Logique dans `Builder::process_overlays`.
- **Voile cliquable** : `Portal::dismiss(msg)` fait émettre `msg` au clic **hors**
  du contenu d'une modale `Center`. Implémenté via un hit plein-écran ajouté
  **avant** le contenu (donc battu par lui au recouvrement), exposé par
  `Widget::overlay_dismiss`.

## Lot B — Transitions d'état animées

- **Glissement du Switch** : la pastille et la couleur de piste interpolent entre
  off/on. Mécanisme générique : `Widget::anim_target() -> Option<f32>` déclare la
  cible ; `Runtime::advance_values` fait tendre une valeur retenue (par identité)
  vers elle et la restitue via `Status::value`. **Pas d'animation au montage**
  (la valeur adopte directement la cible la première fois qu'on voit le widget).
- **Fondu de thème** : `Theme::lerp(other, t)` interpole tous les tokens. Le shell
  capture le thème sortant au basculement et mélange sortant → cible sur ~0,25 s.

## Lot C — Geste retour (swipe), ressenti natif

Glissement depuis le bord gauche (`BACK_EDGE` px) pour dépiler un écran, la
transition suivant le doigt **1:1**, puis une détente physique.

```
Pressed (x < BACK_EDGE, pile non vide)  → Drag::Back ; BackGesture{ progress=0, velocity=0 }
CursorMoved (drag)                      → progress = (x − start)/largeur
                                          velocity = EMA de la vitesse du doigt (fraction/s)
Released                                → projected = progress + velocity·BACK_PROJECT
                                          settling = if projected > 0.5 { 1.0 } else { 0.0 }
Redraw (settling)                       → ressort amorti amorcé par velocity :
                                          a = K·(cible − p) − C·v ; v += a·dt ; p += v·dt
                                          au repos près de la cible → termine (dépile si 1.0)
```

Trois ingrédients du **ressenti natif** :

1. **Vélocité** — un *flick* rapide valide même à mi-course ; un arrêt lent sous
   la moitié annule. Décision par **position projetée** (position + élan), façon iOS.
2. **Détente à ressort** — au relâchement, la transition continue avec l'**élan
   du doigt** en vitesse initiale, via un ressort quasi critique (`K=220`,
   `C=30`) → arrivée douce sans dépassement, pas une rampe linéaire. **Le même
   ressort pilote la navigation par bouton** (amorcé à vitesse 0 → *ease-out*),
   pour un mouvement cohérent partout (`spring_step`).
3. **Parallaxe + profondeur** (`Navigator`, partagé avec push/pop) — l'écran
   arrière se déplace `NAV_PARALLAX=0.3×` moins vite, est rendu **derrière** (ordre
   de profondeur corrigé) et **assombri** proportionnellement à son recouvrement.

La prévisualisation réutilise le `Navigator` (pop : écran courant sortant à
droite, écran inférieur entrant depuis la gauche), **sans modifier la pile de
routes** tant que le geste n'est pas validé ; le dépilement n'a lieu qu'à la fin
de la détente de validation.

## Tests

- `Theme::lerp` atteint ses bornes et diffère au milieu.
- `advance_values` : adopte la cible au montage, anime au changement, oublie les
  widgets disparus.
- Un clic sur le voile d'une modale `Center` renvoie le message de fermeture.
- Pop à mi-course : l'écran arrière est parallaxé (offset comprimé vers 0) et
  rendu derrière l'écran sortant.
- (24 tests antérieurs conservés → **28** au total.)

## Limites (v1)

- Auto-flip : bascule/recadrage simples, pas de repositionnement fin (coins).
- Geste retour : zone de bord fixe en px physiques ; peut chevaucher un contrôle
  très à gauche.
- Le fondu de thème reconstruit un `Theme` mélangé par frame (peu coûteux).
- Mouvement unifié sur `spring_step` (geste + bouton) ; le fondu de thème et les
  transitions hover/focus gardent, eux, leur propre rampe (non ressort).
