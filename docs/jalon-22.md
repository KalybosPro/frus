# Jalon 22 — Barre de navigation (`NavBar`) + titres animés

Un widget de barre de navigation persistante, qui **glisse et fond avec son
écran** pendant les transitions du `Navigator`.

## `NavBar`

```rust
NavBar::new("Mes tâches")             // barre racine (titre seul)
NavBar::new("Réglages").on_back(Msg::Pop)  // avec bouton retour
```

- Hauteur fixe (56 px), **titre centré** peint dans les bornes (indépendant du
  bouton), **séparateur** bas fin.
- Bouton retour optionnel à gauche (un `Button` interne) émettant le message
  fourni. Sa marge gauche (`PAD_LEFT = 28`) le place **au-delà de la zone du
  geste retour** (`BACK_EDGE = 24`), pour qu'il reste cliquable sans déclencher
  le swipe.
- Le titre fond via `Status::opacity` (fondu de montage).

## Titres animés — « gratuit »

La `NavBar` est **dans l'arbre de chaque écran**. Elle hérite donc de la
transition du `Navigator` (glissement + parallaxe + assombrissement de J19) :
lors d'un push/pop/geste, le titre de l'écran sortant glisse, celui de l'entrant
arrive — synchronisé avec le contenu, **sans moteur d'animation dédié**. C'est le
bénéfice de garder la barre par-écran plutôt qu'une barre globale.

## Démo

L'écran **Réglages** (poussé) commence désormais par une `NavBar` avec retour, en
remplacement de l'ancien `screen_header` bricolé. L'**accueil** garde son en-tête
riche (titre + bascule de thème + « Réglages → ») : une barre racine avec actions
à droite dépasse la v1 de `NavBar`.

## Tests

- `root_bar_has_no_back_button` : la barre racine n'a pas de bouton.
- `back_button_emits_message` : un clic sur le retour renvoie le message.
- `bar_paints_title_and_divider` : le titre est peint.
- Total : **33 tests frus-widgets** + tests frus-demo + doctest.

## Limites (v1)

- Titre mono-ligne, pas de *large title* iOS ni de sous-titre.
- Pas d'actions à droite dans `NavBar` (l'accueil garde un en-tête custom).
- Crossfade « par écran », pas une barre globale qui morph un seul titre.
