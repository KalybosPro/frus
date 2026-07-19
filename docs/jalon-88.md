# Jalon 88 — Phases de frame & cache de frontière de repaint

## Analyse

Le §1/§0 recommandent un pipeline en **phases** (`build → layout → paint →
composite`) où chaque `Msg`/`Command` ne salit que le bit le plus étroit. Frus
avait déjà deux morceaux :

- **Phase BUILD conditionnelle** (pilote) : la `view` n'est reconstruite que si
  l'état/la taille changent (`build_dirty`) ou si l'app anime. Une frame
  d'interaction pure (survol, focus, défilement, curseur) **réutilise l'arbre
  retenu**.
- **Cache de relayout** (jalon 55) : taffy n'est rappelé que si le *style/la
  structure* d'une racine changent — l'« empreinte de layout » **est** le bit
  « layout sale ».

Manquait la **peinture** : même une frame d'interaction repeignait **tout**
l'arbre (reconstruction complète de la `Scene`, reshaping de tout le texte). Ce
jalon ajoute le pendant *peinture* du cache de layout.

## Ce qui est fait

### `RepaintBoundary` (opt-in, façon Flutter)
`Container::repaint_boundary()` marque un conteneur comme **frontière de
repaint** (nouvelle méthode `Widget::repaint_boundary`). Choix opt-in : on ne
paie le cache que là où le contenu est **statique** et le gain réel — comme le
`RepaintBoundary` de Flutter, posé au vu du profilage.

### Cache de peinture (`paintcache.rs`)
Retient par frontière, d'une frame à l'autre, la **sortie peinte** du
sous-arbre : primitives **et** cartes d'interaction (hits, focusables,
sémantique…). Sur un *hit*, on **rejoue** ces primitives déjà formées (découpe
et propriétaire *baked*) sans repeindre — pas de reshaping de texte, pas de
reconstruction de décoration.

Le `Runtime` étant générique-agnostique (pas de `Msg`), la donnée est stockée
**effacée** derrière un `Box<dyn Any>` et redescendue vers son
`BoundaryData<Msg>` concret dans le pilote (une seule instance de `Msg` par app
→ le `downcast` réussit toujours).

### Correction du cache (deux verrous)
1. **Génération** : toute reconstruction de la `view` (état, thème, taille)
   **incrémente une génération** ; une entrée périmée n'est plus un *hit*. Comme
   l'arbre est le **même objet** tant que `build` ne tourne pas, une entrée de
   génération courante ⇒ configuration identique.
2. **Empreinte** : couvre le reste — le `Status` de **chaque** descendant
   (survol, focus, valeur/opacité animées, curseur…) **et** les rectangles
   absolus du sous-arbre. Empreinte + génération inchangées ⇒ la peinture serait
   **bit-à-bit identique** → on rejoue le cache. Le temps est exclu (voir plus
   bas).

### Périmètre sûr (fondation)
Une frontière n'est mise en cache que si son sous-arbre est **plat** : la
frontière et **tous** ses descendants empruntent la branche de parcours par
défaut (enfants en préfixe) — ni défilable, ni navigateur, ni liste
virtualisée, ni `layout_builder`, ni pile, ni overlay, ni animation `continuous`.
Ce cas consomme les rectangles dans l'**ordre exact** du walk, ce qui garantit
une empreinte et une rejoue bit-à-bit correctes. Tout descendant à mise en page
dynamique ⇒ **non cachable**, repli sûr : on repeint intégralement. (Un
sous-arbre qui pousse un overlay ou touche le scope de focus modal n'est pas
mémorisé non plus.)

L'exclusion de `continuous` justifie l'exclusion du **temps** de l'empreinte :
un sous-arbre cachable n'a aucun widget piloté par le temps, donc son rendu ne
dépend pas de l'horloge.

### Pilote & pipeline
Le parcours passe désormais par `walk` (frontière : hit → rejoue ; miss →
`walk_node` + capture) au-dessus de `walk_node` (parcours complet inchangé) —
les frontières **imbriquées** sont donc mises en cache elles aussi. Le pilote
**incrémente la génération** juste après une reconstruction de `view`.

## Démo

La bannière **statique** « Tip » de la carte principale est enveloppée dans une
frontière de repaint : elle est rejouée depuis le cache aux frames
d'interaction pure au lieu d'être repeinte à chaque frame.

## Tests

- `repaint_boundary_reuses_a_static_subtree_bit_identical` : frame 1 = miss
  (capture) ; frame 2 = **hit**, et la scène rejouée est **bit-à-bit identique**
  au repaint complet ; les cartes d'interaction (hits) sont aussi rejouées.
- `repaint_boundary_invalidated_by_generation_bump` : après une reconstruction
  (génération incrémentée) → repaint complet.
- `repaint_boundary_invalidated_by_interaction_change` : un survol animé d'un
  descendant change l'empreinte → repaint ; une fois stabilisé → réutilisation.
- `paintcache` : hit sous génération/empreinte égales, invalidation par
  génération, éviction des frontières disparues en fin de frame, gel des
  compteurs de diagnostic.

Les widgets existants ne posent pas de frontière (`repaint_boundary()` = `false`)
→ `walk` délègue toujours à `walk_node` : comportement **inchangé**, aucune
régression sur les suites existantes.

## Reste

- **Compositing GPU** (calques rendus en texture, réutilisés sans re-upload) :
  différé — validation difficile sous le GPU logiciel de WSL. Ici, seul le walk
  de peinture (CPU) est court-circuité ; la scène est toujours ré-uploadée.
- Frontières **non plates** (défilables/navigateur/pile) : hors périmètre de
  cette fondation.
