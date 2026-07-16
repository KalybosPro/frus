# Jalon 72 — Scopes de focus : la modale piège Tab, les flèches et le clic

Suite du §6 : les **scopes de focus** (le brief les listait « peuvent attendre » —
ils ne pouvaient plus : Échap fermait la modale du dessus depuis le jalon 71,
mais **Tab s'en échappait** vers l'interface d'arrière-plan voilée).

## Le mécanisme

Pendant le rendu des overlays, chaque overlay **modal** (voilé : `Center`, tiroirs
`Left`/`Right`, feuille `Bottom`) marque l'index de son premier focusable —
`focus_scope_start`. Le dernier rendu (le plus au-dessus, portails imbriqués
compris) l'emporte. Les overlays ancrés (`Below`, `Tooltip`) ne piègent pas.

Le **pool de focus participants** (`focus_pool`) devient alors la tranche des
focusables du scope, et les trois portes d'entrée du focus la respectent :

- **Tab** (`focus_next`) : boucle **dans** la modale ; un focus courant hors
  scope (pris avant l'ouverture) est traité comme « aucun » — Tab **entre** dans
  le piège.
- **Flèches** (`focus_directional`) : les candidats sont piégés ; le point de
  départ peut être hors scope (le premier coup de flèche fait entrer).
- **Focus au clic** (`focus_hit`) : un clic sur le voile ne focalise plus un
  champ d'arrière-plan.

Sans modale, `focus_scope_start` est `None` : tous les focusables participent —
comportement historique strictement inchangé (les tests Tab existants passent
tels quels).

## Validation

- **249 tests**, tout vert — le nouveau test épingle : Tab entre dans la modale,
  y circule et y **boucle** ; les flèches ne sortent pas du scope ; le clic sur
  la zone d'un bouton d'arrière-plan ne focalise rien ; sans modale, Tab commence
  au fond comme avant.
- Build sans avertissement ; démo sans panique. La modale de confirmation, le
  tiroir et la feuille de la démo piègent désormais le clavier — combiné à
  Échap (J71) et aux anneaux clavier-seul (J70), le clavier des modales est
  complet.

## Suite (§6)

Modèle clavier régularisé (physical + logical + character), scrolling en 4
pièces (`Position/Controller/Physics/Activity` — la rencontre avec la couche
`Simulation` du jalon 53), split `padding`/`viewInsets`.
