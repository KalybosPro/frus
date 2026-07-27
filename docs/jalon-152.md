# Jalon 152 — Autocomplétion : mise en avant du texte & suggestion active

## Analyse

L'`Autocomplete` (jalons antérieurs, largeur réglée au jalon 150) affichait ses
suggestions en texte **uni**, sans deux repères attendus d'un champ Material :

- **Mise en avant** de la portion du libellé qui **correspond** à la requête (le « pourquoi
  ça matche »).
- **Suggestion active** surlignée — celle qui serait choisie, parcourue au clavier.

Le jalon 150 les avait notés dans son « Reste », avec « descente clavier depuis le champ ».

## Décisions techniques

- **Mise en avant par segments.** La suggestion découpe son libellé en trois segments
  `[avant | correspondance | après]` (recherche de sous-chaîne **insensible à la casse**,
  indices de **caractères** — robuste hors ASCII) et dessine la correspondance en couleur
  `primary`, le reste en `on_surface`. Trois appels `text()` calés par mesure de largeur ;
  la correspondance peut être **au milieu** du mot (ex. « gr**ap**e »).

- **Suggestion active, comme le Dropdown.** `active(index)` : la suggestion d'index actif
  reçoit le **fond teinté** `surface.lerp(primary, 0.14)` (survol par-dessus), exactement
  comme l'option sélectionnée d'un `Dropdown` — cohérence visuelle du framework.

- **Descente clavier : déjà là.** Nul besoin de toucher au routage clavier du shell : les
  suggestions sont **focusables**, donc la flèche bas depuis le champ mono-ligne (dont le
  déplacement vertical du curseur retourne `None` en bordure) **navigue le focus** vers la
  première suggestion ; Entrée la choisit (le shell active tout `on_click` focalisé). L'app
  garde le modèle « actif » (index surligné) si elle préfère piloter au clavier sans
  déplacer le focus. Vérifié par un test de cycle de focus.

## Implémentation

- `autocomplete.rs` : helper `match_range` (sous-chaîne insensible à la casse, indices de
  caractères) ; `Suggestion` gagne `query`/`active` et peint en segments ; `Autocomplete`
  gagne `active` + `.active(index)` ; `rebuild` passe `query` (= valeur) et l'actif.
- `goldens.rs` : golden `autocomplete` (champ « ap », liste, 2ᵉ active, « ap » mis en avant).

## Vérification

- **Unitaire** : `match_range` (« Apricot »/« ap » → `(0,2)`, « pineapple »/« APPLE » →
  `(4,9)`, requête vide / absente → `None`) ; la portion correspondante est un **segment**
  de texte à part (« ap » + « ricot » sur « apricot ») ; la suggestion **active** est
  surlignée (rect teinté) ; le champ **puis** une suggestion entrent dans le cycle de focus.
- **Golden** `autocomplete` **inspecté** : « ap » en vert dans chaque suggestion (y compris
  au milieu de « grape »), « apricot » (active) surligné. `cargo test --workspace` **vert**.

## Reste

- **Défilement** de la liste de suggestions quand elle est longue (borne de hauteur +
  `Scroll`).
- **Surbrillance qui suit le focus** : lier `active` au focus clavier réel (aujourd'hui
  l'app pilote l'un ou l'autre) pour un unique repère.
