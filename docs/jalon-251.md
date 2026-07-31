# Jalon 251 — Fantôme de glisser incluant le contenu d'une carte riche

## Analyse

Le fantôme du glisser-déposer (jalons 248, 250) recopie, translatées et dé-découpées, les primitives
de l'élément saisi. Le filtre retenait les primitives dont le **propriétaire** est exactement la carte
saisie (`p.owner() == id`). Cela suffit pour une carte **texte** (la carte peint elle-même son fond et
son libellé), mais **pas** pour une carte **riche** : son contenu (libellé, étiquettes, bouton ×) est
peint par des **enfants**, sous d'autres propriétaires. Résultat : le fantôme d'une carte riche ne
montrait qu'une **tuile vide** — limitation notée au jalon 250.

## Décisions techniques

- **Fantôme = primitives de tout le sous-arbre.** Plutôt que la seule carte, le shell rassemble les
  primitives de **tous** les widgets du sous-arbre de l'élément saisi (la carte **et** ses enfants).
  L'ordre de peinture est préservé (fond d'abord, contenu ensuite), donc le fantôme est fidèle.

- **Nouvel utilitaire `subtree_ids(widget, root_id)`** dans `frus-widgets` : les identités du
  sous-arbre enraciné en `root_id`, dérivées par le **même schéma positionnel** que `collect_ids`
  (`child_id(id, index, child)`). Le shell en fait un ensemble de propriétaires et filtre la scène
  dessus.

- **Repli sûr** : si l'arbre est indisponible, on retombe sur l'ancien comportement (propriétaire =
  la carte seule).

## Implémentation

- `frus-widgets/src/ui.rs` : `pub fn subtree_ids` (+ export dans `lib.rs`). Test
  `subtree_ids_covers_a_widget_and_its_descendants` (depuis la racine ≡ `collect_ids` ; depuis un
  enfant : commence par son identité, sous-ensemble strict de l'arbre).
- `frus-shell/src/app.rs` : `paint_reorder_preview` calcule l'ensemble des propriétaires via
  `subtree_ids` et filtre les primitives du fantôme dessus.

## Vérification

- **Widgets 387** ; **shell 26**. Le mécanisme (`subtree_ids`) est couvert par test unitaire :
  l'ensemble des propriétaires du fantôme contient bien la carte **et** ses descendants.
- **Non-régression** : aucun rendu statique modifié — le fantôme n'existe que pendant un glisser
  engagé, hors des goldens ; goldens inchangés.

## Notes

- Le fantôme reste un état **runtime** (glisser en cours), non inspecté au GPU dans cet environnement.
  Ce jalon corrige la **composition** du fantôme (source des primitives) ; sa vérification porte sur
  la logique de collecte du sous-arbre.

## Reste

- Indicateur d'insertion **inter-cartes** (au-dessus/au-dessous selon la moitié survolée).
