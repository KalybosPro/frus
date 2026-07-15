# Idées tirées de Flutter pour Frus

> Analyse du dépôt **flutter-master** (framework complet, ~570 k lignes Dart dans
> `packages/flutter/lib/src`) en vue d'orienter l'architecture de Frus.
> Objectif : voler ce que Flutter a **prouvé** (moteur de réconciliation, protocole
> de layout, arène de gestes, physique d'animation, scrolling, tokens de thème),
> et **rejeter** ce que l'architecture Elm + taffy + wgpu de Frus rend inutile
> (StatefulWidget, GlobalKey, InheritedWidget, ChangeNotifier comme état d'app).
>
> Les strings d'UI restent en anglais ; ce document est en français (convention projet).

---

## 0. Le fil conducteur (à lire en premier)

Un même principe ressort de **tous** les sous-systèmes analysés. C'est la décision
architecturale porteuse ; tout le reste en découle :

> **Le shell retient l'état ; la `view` pure ne déclare que l'intention ; tout se
> déverse en `Msg`.**

Concrètement :

- **L'état vivant dans le temps** (arène de gestes, contrôleurs d'animation, offsets
  de scroll, focus, curseurs d'édition, nœuds retenus de réconciliation, caches de
  hit-test) vit dans `frus-shell`, **hors** de l'arbre `Box<dyn Widget>`, **clé par
  `child_id`/`Keyed`**. Frus fait déjà ça pour hover/focus/edit/scroll dans
  [`runtime.rs`](../crates/frus-widgets/src/runtime.rs) — il faut généraliser le
  motif, pas l'inventer.
- **La `view` ne porte que des descripteurs déclaratifs** : `on_tap: Msg`,
  `on_drag: fn(Delta) -> Msg`, l'identité stable, l'intention de recevoir tel type
  de reconnaisseur. Jamais l'état de la gesture/animation en cours.
- **La réconciliation par identité est le pivot.** La `view` est reconstruite chaque
  frame ; le shell ré-associe l'état retenu au nouvel arbre par `child_id`. Le mode
  de défaillance est toujours le même (déjà noté dans CLAUDE.md) : un parcours d'ids
  incohérent casse hover/focus/édition/animation/**drag en cours** au réordonnancement.
- **Discipline de phases.** Séparer la frame en passes ordonnées et **indépendamment
  invalidées** — `build → layout → paint → composite` — chacune drainée depuis sa
  propre liste de « dirty ». Un survol ne touche que *paint* ; un changement de texte,
  *layout+paint* ; un changement de thème (Frus se thème au paint), *paint* seul.

Le reste du document décline ce principe sous-système par sous-système, avec les
manques réels de Frus en ligne de mire.

---

## 1. Pipeline de rendu & protocole de layout
*(source : `rendering/object.dart`, `box.dart`, `layer.dart`, `binding.dart` ; `scheduler/{binding,ticker}.dart`)*

### Le protocole
- **Contraintes vers le bas, tailles vers le haut.** Le parent passe un
  `BoxConstraints (min/max W/H)` immuable ; l'enfant choisit sa `size` dedans et la
  remonte. Un parent ne lit la taille d'un enfant que s'il l'a demandé
  (`parentUsesSize`) — ce booléen est ce qui rend le relayout incrémental possible.
- **Frontières de relayout.** Un nœud est frontière ssi
  `!parentUsesSize || sizedByParent || constraints.isTight || parent == None`.
  Quand il devient sale, il s'ajoute **lui-même** à la liste de layout ; sinon il
  remonte jusqu'à la première frontière. Une édition de texte profonde ne relaie que
  son sous-arbre, jamais toute la fenêtre.
- **Parent-data.** L'offset/le flex d'un enfant vivent sur `child.parentData`, pas
  sur l'enfant → l'enfant reste réutilisable et re-parentable (colle à `Keyed`).

### Coexistence avec taffy
Taffy **est** votre `performLayout` pour flex/grid : ne réimplémentez pas ces maths.
Ce qu'il faut voler *au-dessus* de taffy :
1. **Cache de frontière de relayout** : par racine de layout, mémoriser
   `(dernières_contraintes, taille_cachée, needs_layout)` et **ne pas ré-invoquer
   taffy** si contraintes + drapeau inchangés. **Le plus gros gain layout**, et il
   vit hors de taffy.
2. **Tailles intrinsèques** (`min/max intrinsic width`) routées vers la closure de
   mesure de taffy — pour le texte et le contenu peint sur mesure.
3. **Séparation parent-data** pour un réordonnancement bon marché.

### Frontières de repeinture & arbre de layers → wgpu
- Un `RepaintBoundary` possède son propre **batch de dessin retenu** ; quand il
  devient sale, seul lui se re-record, le reste est **réémis par référence**.
- **Analogue wgpu pour Frus** : donner aux frontières de repeinture un fragment
  caché (liste de quads persistée ; ou une **texture wgpu** pour du contenu vraiment
  coûteux, façon `RenderRepaintBoundary.toImage`). Le cas d'école immédiat : **la
  BottomSheet à ressort glissant au-dessus d'un contenu statique** — cachez le
  contenu en une texture, ne réémettez que les quads de la feuille.
- Les « compositing bits » (n'allouer une vraie texture/passe que si clip/opacité<1/
  blur/transform) sont à **différer** : inlinez tout tant qu'il n'y a pas de vrais
  layers matériels.

### Ticker / vsync → piloter l'animation
- Discipline clé : **aucune frame n'est produite si personne n'en demande une.** Le
  ticker est un « one-shot auto-reprogrammé » : tant qu'une animation est active il
  redemande une frame ; à l'arrêt, retour à l'idle → **0 CPU/GPU au repos**.
- **Pour Frus (winit)** : `window.request_redraw()` est votre source vsync.
  À chaque redraw, avant le build, livrez le **timestamp de frame** aux animateurs
  actifs ; puis `if une_animation_active { request_redraw() }`. Au repos,
  `ControlFlow::Wait`. Pilotez par le **timestamp de la frame**, pas par un delta
  mesuré dans le handler, et **clampez les gros deltas** (fenêtre en arrière-plan)
  pour éviter l'explosion des ressorts. Coupez les animateurs hors-écran (lifecycle
  Android que vous gérez déjà) — l'idée `muted`.

### Traduction Rust
- **Arène / slotmap de nœuds clés par `Id`**, liens parents en `Id`. `markNeedsLayout`/
  `markNeedsPaint` deviennent « pousser un `Id` dans un `Vec` dirty » — pas d'aliasing,
  pas d'emprunt mutable vers le haut. Ce design *colle mieux* à Rust qu'à Dart.
- **Pas de GC → durées de vie explicites** des textures/buffers wgpu cachés : liez la
  vie de chaque batch/texture au slot du nœud, libérez à sa suppression (wrapper RAII
  `Drop`, plus sûr que le refcount manuel de Flutter).
- **Les phases séparées imposent la sûreté d'aliasing** : chaque passe prend l'arène
  en `&mut` exclusif → le type system fait gratuitement ce que Flutter vérifie par
  asserts debug.

### À NE PAS copier
Les trois arbres Widget/Element/RenderObject avec `setState` impératif ; les maths
flex/grid (taffy) ; `InheritedWidget` ; le walking `markNeeds*` d'un arbre à pointeurs
parents mutables (→ arène/slotmap à la place).

---

## 2. Modèle Widget/Element & Foundation
*(source : `widgets/framework.dart`, `foundation/{key,change_notifier,diagnostics}.dart`)*

### Ce qu'il faut adopter
- **Deux arbres : config immuable (`Box<dyn Widget>`) + nœud retenu.** La discipline
  de *réutilisation* est le cœur de votre « rebuild pattern ». Le nœud retenu de Frus
  n'a PAS besoin d'être gras comme un Element (qui stocke le `State` utilisateur) :
  en Elm l'app possède l'état logique, donc le nœud ne garde que l'**éphémère**
  (hover/pressed/focus/curseur/horloges d'anim/offset scroll). Appelez-le « nœud de
  paint retenu », pas « Element ».
- **`can_reuse = TypeId + Key`** (l'analogue de `runtimeType + key`) + **l'algo de
  diff de liste en trois phases** (`updateChildren`) : préfixe commun haut→bas, suffixe
  commun, puis map `Key→nœud` du milieu *keyé*. C'est la façon prouvée de rendre
  insert/remove/reorder en O(n) **sans perdre l'état éphémère**. Les enfants du milieu
  *non-keyés* qui ne s'alignent plus positionnellement sont détruits (état perdu) —
  d'où l'importance des clés.
- **Clés = `enum { Index(u32), Value(SmallKey), Unique(u64) }`, `Hash + Eq`.**
  Formalisez `child_id`/`Keyed`. Les clés sont **le** correctif du bug
  « réordonner perd l'état ». Vous avez déjà `WidgetId::child`/`keyed` dans
  [`interaction.rs`](../crates/frus-widgets/src/interaction.rs) — c'est exactement
  la bonne fondation.
- **Trait de dump diagnostique de l'arbre** (`DiagnosticableTree`) : `short()`,
  `props()`, `children()`, avec un `dump_deep()` indenté, derrière
  `#[cfg(debug_assertions)]`. **Investissement minuscule, levier de debug énorme** —
  surtout pour la classe de bugs identité/réordonnancement que vous avez déjà notée.
  Dumpez **les deux** arbres (ce que `view` a produit vs ce qui a été retenu/réutilisé) ;
  ce diff côte-à-côte est l'outil n°1 pour déboguer la stabilité d'identité.
  `#[derive(Debug)]` n'est pas un substitut.

### Ce qu'il faut adapter
- **Contexte ambiant** → une seule struct immuable `Env`
  `{ theme, size_class, text_direction, insets/safe-area, scale }` passée
  **explicitement** vers le bas (dans `paint`, et dans `measure` si besoin), avec un
  *shadowing* par sous-arbre (`Env::with_theme(...)`). Ça **remplace tout le graphe de
  dépendances `InheritedWidget`**. Frus se thème déjà au paint → un changement de thème
  n'a même pas à ré-exécuter `view`, juste repeindre : c'est *strictement plus simple*,
  gardez-le.
- **BuildContext** → jamais un handle vivant vers un arbre mutable. Arène plate +
  `NodeId` : les parcours ancêtre/enfant deviennent des sauts d'index sans conflit
  d'emprunt.

### Ce qu'il faut rejeter (anti-patterns pour Elm)
`StatefulWidget`/`State`/lifecycle ; **`GlobalKey`** (re-parentage cross-arbre via
registre global mutable — cauchemar de durées de vie en Rust ; en Elm l'app déplace
l'état dans `update`, et overlays/portails se modélisent en **couche top-level séparée
dans `view`**) ; `ChangeNotifier`/`ValueNotifier`/`InheritedNotifier` comme état d'app
(graphe observable mutable = antithèse de la source unique via `update`) ;
`updateShouldNotify`/invalidation par dépendants (mort-né puisque Frus repeint au lieu
de rebuild sur changement ambiant).

> **Résumé** : garder le *moteur de réconciliation* de Flutter (split config/retenu,
> canUpdate, diff de liste keyé, clés, diagnostics) ; jeter sa *machinerie d'état et de
> données ambiantes* (StatefulWidget, GlobalKey, InheritedWidget, ChangeNotifier) —
> Elm résout déjà ce pour quoi elles existaient.

---

## 3. Gestes — l'arène (le joyau)
*(source : `gestures/{arena,binding,recognizer,tap,monodrag,long_press,scale,pointer_router,hit_test,events,velocity_tracker}.dart`)*

**État actuel de Frus** : [`interaction.rs`](../crates/frus-widgets/src/interaction.rs)
gère clic/press/hover/focus + édition texte. Pas d'arène, pas de drag/longpress/scale
disambiguation. C'est le plus gros manque structurel côté entrée.

### Le pipeline (4 pièces découplées, tenues par un `GestureBinding` à longue vie)
1. **Hit-test** (`hit_test.dart`) : construit un chemin ordonné de cibles sous le
   pointeur, **du plus interne au plus externe**, et le **cache par id de pointeur au
   down** (réutilisé jusqu'au up — correction *et* perf : les moves routent là où la
   pression a commencé).
2. **PointerRouter** (`pointer_router.dart`) : table d'abonnement par id de pointeur ;
   les reconnaisseurs s'abonnent au flux brut. **Itère sur une copie** pour permettre
   à un callback de se désabonner en plein dispatch.
3. **Arène** (`arena.dart`) : le désambiguïsateur, une par id de pointeur.
4. **Reconnaisseurs** (`recognizer.dart` + tap/drag/longpress/scale) : machines à états
   qui consomment le flux, concourent dans l'arène, émettent les callbacks sémantiques.

### L'arène, règle centrale
> *« Le premier membre à accepter, ou le dernier à ne pas rejeter, gagne. »*

- **Modèle** : liste **ordonnée** de membres (ordre = profondeur hit-test, interne
  d'abord) + drapeaux `open/held/pending_sweep` + `eager_winner`.
- **Cycle** : `add` (tant qu'ouverte) → `close` (le down fini de se dispatcher) →
  `resolve(accepted|rejected)`. Accepté **fermé** = gagne tout de suite (tous les
  autres reçoivent `reject`) ; accepté **ouvert** = `eager_winner` en attente.
- **`sweep`** (au pointer-up) : brise l'égalité — **le premier membre de la liste (le
  plus interne) gagne**, le reste est rejeté. Évite qu'un doigt levé ne déclenche rien.
- **`hold`/`release`** : un reconnaisseur qui doit survivre au up (double-tap, timer de
  long-press) `hold` pour neutraliser le `sweep`, puis `release` le rejoue.
- **Garantie** : chaque membre reçoit **exactement un** `accept` XOR `reject`.

### La disambiguation canonique tap-vs-drag
- **Tap** accepte **passivement** : il attend d'être le dernier debout. Le doigt bouge
  au-delà du slop → `reject` immédiat. Le doigt se lève sur place → tap gagne au `sweep`.
- **Drag** accepte **avidement** : dès que la distance projetée sur son axe dépasse le
  slop → `accept`, ce qui **évince le tap**. Les drags directionnels projettent le delta
  sur leur axe → un drag horizontal cohabite avec un scroll vertical dans la même arène.

### Modèle d'événement pointeur
`Down/Move/Up/Cancel` (+ Hover/pan-zoom à différer). Chaque événement : `pointer`(id),
`position` (globale, px logique), `delta`, `buttons` (bitfield), `kind`, `timestamp`.
**`Cancel` est critique** (app en arrière-plan, gesture volée) : abandonner sans callback
de succès. **Vélocité** : ring buffer des derniers ~100 ms ; commencez par une moyenne
pondérée, passez au fit moindres-carrés (LSQ) plus tard pour des flings précis.

### Traduction Rust (résout la ré-entrance de Dart)
- **`Arena::resolve/close/sweep` PURES** : elles mutent les maps de l'arène et
  **renvoient un `Vec<(MemberId, Disposition)>`** de callbacks à appliquer, au lieu de
  rappeler ré-entramment. Le shell draine ce `Vec` après la fin de l'emprunt → boucle
  explicite au lieu d'une pile d'appels ; ça remplace proprement `scheduleMicrotask`.
- **Ids `Copy` partout** (`MemberId`/`PointerId`/`RouteId`), jamais de références
  possédées. Reconnaisseurs = machines à états renvoyant `Option<GestureEvent>`, ne
  tenant aucune référence vers l'app.
- **Timers** (tap-vs-longpress, hold/release) : `ControlFlow::WaitUntil` de winit ou
  une petite roue de timers ; au tir, injecter un tick synthétique.

### Chemin de migration (par paliers)
- **Palier 0 (fondation, à faire d'abord)** : normaliser l'entrée winit en `PointerEvent`
  (avec `Cancel` explicite) ; hit-tester l'arbre taffy en `Vec<HitEntry>` (interne
  d'abord) caché par id ; un `PointerRouter`. **Non-jetable.**
- **Palier 1 (MVP sans arène complète)** : un reconnaisseur « tap-ou-drag » codé en dur
  (down→*possible* ; mouvement > slop avant up → drag + supprime tap ; up avant slop →
  tap) + long-press par timer. Couvre ~90 % des besoins. **Faites-le parler déjà le
  vocabulaire de l'arène** (« accepte avidement au franchissement de slop » / « accepte
  passivement au up ») pour que le passage au palier 2 soit une substitution, pas une
  réécriture.
- **Palier 2 (vraie arène)** : dès qu'il y a des régions imbriquées indépendamment
  scrollables, ou un draggable dans un scrollable dans une carte tappable. Portez
  `arena.dart` quasi verbatim (versions **pures, renvoyant les outcomes**) +
  `PrimaryPointerGestureRecognizer` (base deadline+slop réutilisée par tap ET longpress).
- **Palier 3 (à différer)** : vélocité LSQ, scale/rotation (pinch-zoom), teams,
  multi-drag, molette, resampling.

---

## 4. Animation & physique
*(source : `animation/{animation,animation_controller,tween,curves,animations}.dart` ; `physics/{simulation,spring_simulation,friction_simulation,clamped_simulation}.dart`)*

**État actuel de Frus** : ressorts codés en dur par-widget (`spring_step`, `scroll_axis`
dans [`runtime.rs`](../crates/frus-widgets/src/runtime.rs)). Ça marche mais ne se
généralise pas. Flutter offre une abstraction unique très portable.

### L'abstraction cœur
- **`Animation<double>`** = valeur observable dans `[0,1]` + `status`
  (`dismissed/forward/reverse/completed`). Le producteur (ticker), le consommateur
  (widget) et le façonneur (tween/courbe) sont **découplés**.
- **`AnimationController`** *est* une `Animation<double>` : il possède un `Ticker` et,
  surtout, **tout ce qu'il fait est exprimé comme une `Simulation`** (fonction pure
  `x(t)`). `forward`/`animateTo` → simulation d'interpolation d'une courbe sur une
  durée ; `fling`/`animateWith` → simulation physique (ressort/friction). **Une seule
  boucle de tick pour tout.** C'est l'idée la plus portable : *un pilote, des fonctions
  temps→valeur enfichables*.

### Tween + Curve
- **`Tween<T>` / `Animatable<T>`** : une méthode `transform(t) -> T`. Un seul contrôleur
  `[0,1]` pilote arbitrairement de valeurs typées (Color, Rect, Offset, opacité…).
- **`Curve.transform(t)`** mappe `[0,1]→[0,1]`. Portez : `Linear`, `Cubic` (Bézier par
  recherche binaire → donne tous les presets `easeInOut` en constantes), et surtout
  **`Interval(begin,end,curve)`** qui déverrouille **gratuitement les animations
  étagées** (staggered) : plusieurs valeurs, un contrôleur, chacune sur une sous-fenêtre.

### Simulations — maths directement portables
Interface : `{ x(t), dx(t), is_done(t), tolerance }` (défaut 1e-3).

**Ressort** — `SpringDescription { mass m, stiffness k, damping c }` ;
`with_damping_ratio` : `c = ratio · 2·√(m·k)`. Soit `x₀ = start − end`, `v₀` vitesse
initiale ; position rapportée = `end + sol.x(t)`. Choix par le discriminant `c² − 4mk` :

```
// Critique (c² − 4mk == 0)
r = −c/(2m); c1 = x₀; c2 = v₀ − r·x₀
x(t)  = (c1 + c2·t)·e^(r·t)
dx(t) = r·(c1 + c2·t)·e^(r·t) + c2·e^(r·t)

// Suramorti (c² − 4mk > 0)
cmk = c² − 4mk; r1 = (−c − √cmk)/(2m); r2 = (−c + √cmk)/(2m)
c2 = (v₀ − r1·x₀)/(r2 − r1); c1 = x₀ − c2
x(t)  = c1·e^(r1·t) + c2·e^(r2·t)
dx(t) = c1·r1·e^(r1·t) + c2·r2·e^(r2·t)

// Sous-amorti (c² − 4mk < 0, oscille)
w = √(4mk − c²)/(2m); r = −c/(2m); c1 = x₀; c2 = (v₀ − r·x₀)/w
x(t)  = e^(r·t)·(c1·cos(w·t) + c2·sin(w·t))
dx(t) = e^(r·t)·(c2·w·cos(w·t) − c1·w·sin(w·t)) + r·e^(r·t)·(c2·sin(w·t) + c1·cos(w·t))
```
`is_done = near_zero(x, tol.dist) && near_zero(dx, tol.vel)`. `fling` utilise un ressort
critique (`ratio 1`, `stiffness 500`) qui s'arrête sur la position seule.

**Friction** (momentum de scroll/fling), `drag ∈ (0,1)` :
```
dx(t) = v₀ · drag^t
x(t)  = x₀ + (v₀/ln(drag))·(drag^t − 1)
finalX (t→∞) = x₀ − v₀/ln(drag)
is_done = |dx(t)| < tol.vel
// through(x0,x1,v0,v1) : drag = e^((v₀−v₁)/(x₀−x₁))
```
`ClampedSimulation` épingle la position dans une plage (la vitesse continue de rapporter)
— pour le scroll.

### Implicite vs explicite = même machine
Les animations implicites (façon `AnimatedContainer`) = un contrôleur *caché* qui, à
chaque rebuild, diffe les props cible et re-cible un `Tween (begin=valeur_courante,
end=nouvelle_cible)` sur `durée+courbe`. **On a l'implicite gratuitement dès qu'on a
l'explicite + un helper diff-et-recible.**

### Mapping Elm — recommandation
Deux options ; **recommandé : B (runtime retenu à côté de la vue pure).**
- **A — animations comme subscriptions émettant des `Msg` tick (pur)** : la vue reste
  pure, mais 60–120 Hz de `Msg` traversent `update`, et l'interruption/re-ciblage
  (drag→fling) force à mettre les objets simulation dans le modèle immuable → douleur.
- **B — recommandé** : petit **registre impératif de contrôleurs dans le shell**, clé
  par identité (`child_id`/`Keyed`). Créés/pilotés par `Command` (`Command::animate(id,
  spec)`, `Command::fling(id, v)`). **La vue lit la valeur courante au paint**
  (`ctx.animation(id).value`) — cohérent avec « les widgets se thèment au paint ».
  **Seules** les transitions de statut (démarré/fini) repassent par `update` en `Msg` ;
  les changements de valeur par-frame déclenchent juste une repeinture.
  Vous avez déjà une feuille à ressort et une boucle frame/command → vous avez *prouvé*
  que vous avez besoin d'état d'animation retenu ; formalisez-le.

### Recommandations classées
1. **Portez la physique maintenant** (maths pures, 0 couplage) : `trait Simulation`,
   `SpringSimulation` (3 cas), `FrictionSimulation` (forme close ; sautez Newton/
   `constantDeceleration`), `ClampedSimulation`. **Généralisez votre ressort de
   BottomSheet en `trait Simulation`** pour que fling, momentum de scroll et feuille
   partagent un seul chemin.
2. **Pilote minimal** : `AnimationController` = valeur bornée + statut + `Box<dyn
   Simulation>` + `tick(elapsed)` (~5 lignes). `forward/reverse/animate_to/fling`.
3. **`Curve` + `Tween`/`Animatable`** : `Linear`, `Cubic`, `Interval` (staggered gratuit).
4. **Implicite** en helper mince une fois l'explicite en place.
5. Câblez le tout en **registre piloté par `Command`, clé par `child_id`**, valeurs lues
   au paint.

**À différer** : splines/`Curve2D`, courbes elastic/bounce, friction desktop à Newton,
scaling « désactiver les animations », `TickerFuture` (un `Msg` de complétion suffit).

---

## 5. Peinture & thème
*(source : `painting/{box_decoration,borders,box_border,box_shadow,edge_insets,alignment,border_radius,gradient,text_style,text_span,text_painter,colors}.dart` ; `material/{theme_data,color_scheme,text_theme}.dart`)*

**État actuel de Frus** : `Scene` n'a que **2 primitives, `Rect` et `Text`**
([`scene.rs`](../crates/frus-core/src/scene.rs)) ; `Theme` est un **sac plat de 11
couleurs + radius + spacing**, dark/light + lerp
([`theme.rs`](../crates/frus-widgets/src/theme.rs)). Deux chantiers : enrichir les
primitives de scène, et structurer le thème en rôles/échelles.

### Vocabulaire de peinture → types cœur Rust (dans `frus-core`, sRGB, px logique)
- **`EdgeInsets` {left,top,right,bottom}** + variante `EdgeInsetsDirectional
  {start,top,end,bottom}` avec **`.resolve(dir) -> EdgeInsets`** (le RTL échange
  start/end — toute l'histoire RTL tient là). Helpers : `all`, `symmetric`,
  `deflate_rect`, `inflate_size`.
- **`Alignment {x,y}`** fraction dans `[-1,1]²` (`(0,0)`=centre) + `within_rect(r)`,
  `inscribe`.
- **`Radius {x,y}`** (elliptique !) + **`BorderRadius`** (4 coins). `to_rrect` **clampe
  les rayons négatifs à 0** avant rendu — faites pareil au bord GPU.
- **`BorderSide {color,width,style,stroke_align}`** : concept porteur = **`stroke_align`
  ∈ [-1 intérieur, 0 centre, 1 extérieur]** (décide si le trait mange le contenu et
  comment inset le fond pour éviter le bleed AI). Bordures uniformes = un `rrect_stroke` ;
  non-uniformes = 4 trapèzes (à différer).
- **`BoxShadow {color,offset,blur_radius,spread_radius}`** ; `sigma ≈ 0.57735·blur +
  0.5`. Implémentez une **primitive de scène « RRect flou analytique »** dans `frus-gpu`
  (shader, pas de vraie passe gaussienne).
- **`Gradient`** enum (Linear/Radial/Sweep) unifié par `colors + stops?(uniforme si
  None) + tile_mode`, **ancres en fractions** (`Alignment`) → indépendant de la taille,
  pixellisé seulement dans `create_shader(rect)`.
- **`Color`** : ajoutez `with_alpha(f32)`, `lerp`, `compute_luminance()` (WCAG, sur
  canaux linéarisés), `from_argb_u32`.

### Modèle de décoration (la clé de voûte peignable)
`BoxDecoration { color?, gradient?, image?, border?, border_radius, shadows, shape }`
avec **ordre de peinture fixe : ombres → fond(couleur/gradient) → image → bordure**.
En immédiat, *lowerez* `BoxDecoration → Vec<Primitive>` au paint (pas de painter retenu),
mais gardez le cache « le shader de gradient dépend du rect ». `content_padding()`
(= dimensions de la bordure) **alimente taffy** pour qu'un conteneur bordé réserve la
place. Différez `ShapeDecoration`/`ShapeBorder` (stadium, superellipse) et
`DecorationImage`/`ImageProvider`.

### Texte
- **`TextStyle`** (color, font, size, weight, italic, letter/word spacing, `height`
  =multiple, decoration, shadows) avec **`merge()` + `inherit`** = la cascade
  (span > DefaultTextStyle > thème). **`TextSpan`** = arbre `{text?, style?, children}`
  → à aplatir en runs `(byte_range, Attrs)` pour cosmic-text.
- **`TextLayout`** mince sur cosmic-text exposant : `size`, `min/max_intrinsic_width`
  (layout à `∞` pour max, `0` pour min → nourrit taffy), `hit_test(p) -> TextPosition`,
  `caret_rect(pos)`, `selection_rects(range)`, `line_metrics()`. cosmic-text fournit
  déjà shaping/line-break/hit/curseur ; le travail = résoudre la cascade `TextStyle →
  Attrs` et exposer intrinsèques + caret/sélection.

### Architecture de thème (Material 3)
- **`ColorScheme` = rôles sémantiques dérivés d'une graine** : `from_seed(seed,
  brightness)` produit ~30 **rôles** (`primary/onPrimary/primaryContainer/…`,
  `surface/onSurface/onSurfaceVariant/surfaceContainer*`, `outline/outlineVariant`,
  `error/…`, `shadow/scrim/inverseSurface/surfaceTint`). Les widgets référencent des
  **rôles**, jamais des couleurs littérales → un swap de thème recolore tout et garantit
  le contraste (paires `X`/`onX`).
- **`TextTheme` = échelle typographique nommée** : 15 slots (`displayLarge…labelSmall`).
  Les widgets choisissent un slot, pas une taille en dur.
- **`Theme` recommandé** (dans `frus-core` ou un `frus-theme`, pour que les widgets se
  thèment sans tirer le shell) :
  `{ brightness, color: ColorScheme, text: TextTheme, shape: ShapeScheme,
  elevation: ElevationScheme, spacing: SpacingScale }`.
- **Livrez un ColorScheme clair/sombre écrit à la main d'abord** ; ajoutez `from_seed`
  (algorithme HCT/`material-color-utilities`, portable en Rust) plus tard.
- Puisque Frus se thème au paint, `paint(theme, status)` résout rôle→couleur selon le
  `Status` (Default/Hover/Press/Disabled/Focus) et applique une **state-layer** (overlay
  `onX` à 8 %/12 %) — **bakez cette règle dans le thème** pour que les widgets restent
  déclaratifs.

### sRGB / linéaire / alpha prémultiplié (bord GPU) — pièges
- Auteur en **sRGB**, conversion linéaire **seulement au bord GPU**, avec la vraie courbe
  (`x≤0.04045 ? x/12.92 : ((x+0.055)/1.055)^2.4`), **pas** `pow(2.2)`.
- **Interpolation** : couleurs d'UI discrètes → lerp en espace gamma (comme CSS/Flutter) ;
  stops de gradient sur GPU → mieux en linéaire. Choisissez et **soyez cohérent** (goldens
  stables).
- **Alpha prémultiplié** : wgpu attend du prémultiplié (`rgb *= a`) **après** passage en
  linéaire ; alignez le blend state (`PREMULTIPLIED_ALPHA_BLENDING`) et l'accord avec
  glyphon/images sous peine de franges.
- **Format de surface** : cible `*Srgb` → le shader sort du **linéaire** (HW encode) ;
  cible `Rgba8Unorm` → encodez vous-même. **Une seule convention `frus-gpu`, documentée,
  centralisée** — c'est la source n°1 des « couleurs délavées/trop sombres ».

### Types à définir, par priorité
1. Helpers `Color` + **convention sRGB↔linéaire + prémultiplié verrouillée dans
   `frus-gpu`** (tout en dépend).
2. `EdgeInsets`(+directional) & `Alignment`.
3. `Radius`/`BorderRadius` (+ clamp négatif).
4. `BoxShadow` + primitive de scène RRect-flou.
5. **`BoxDecoration`** (ordre fixe + `content_padding`→taffy) — clé de voûte.
6. `BorderSide`/`Border` (uniforme d'abord).
7. `Gradient` (Linear d'abord).
8. `TextStyle`(+merge) & `TextSpan`.
9. `TextLayout` sur cosmic-text.
10. **`Theme` = ColorScheme(rôles M3) + TextTheme(15 slots) + Shape + Elevation +
    Spacing** ; clair/sombre à la main, `from_seed` après ; state-layer bakée.

---

## 6. Plateforme, focus, scrolling, navigation & a11y
*(source : `widgets/{focus_manager,focus_traversal,scroll_position,scroll_physics,scroll_activity,scroll_controller,scrollable,overlay,navigator,media_query,safe_area}.dart` ; `services/{hardware_keyboard,text_input,platform_channel,system_channels}.dart` ; `semantics/semantics.dart`)*

### Focus
Arbre `FocusNode` parallèle à l'arbre widget (identité stable → `child_id`), un
`primary_focus` dans le shell, **dispatch des touches feuille→racine** rendant un résultat
à 3 états (`handled/ignored/skipRemaining` ; `ignored` continue de remonter). Une politique
de traversée reading-order/géométrique pour Tab + flèches. **`FocusHighlightMode
{traditional, touch}`** : ne peindre l'anneau de focus que si la dernière interaction était
clavier. Les scopes (piéger le focus dans dialogues/feuilles) peuvent attendre.

### Clavier & saisie — deux voies indépendantes (à garder séparées)
- **(a) Touches matérielles** : modèle régularisé `KeyDown/Up/Repeat` portant
  **physicalKey** (position HID, indépendante du layout) **+ logicalKey** (sens sous le
  layout) **+ character?**, avec un tracker des touches pressées/modificateurs. Alimente
  le focus. **Ignorez le `RawKeyEvent` déprécié.** winit fournit tout côté desktop.
- **(b) Texte/IME** : le texte composé **ne passe pas** par les key events. Un « client »
  possède un `TextEditingValue { text, selection, composing }` — **le `composing` (région
  provisoire IME) est essentiel pour Gboard/CJK**. Le shell possède l'unique connexion
  active. Desktop : winit `Ime::Preedit/Commit` → `composing`. **Android : le clavier
  logiciel ne produit aucune touche matérielle** ; il faut un équivalent `TextInputControl`
  au-dessus du FFI (§ suivant), exposant un `InputConnection`, et **piloter `viewInsets`**
  pour que le contenu monte au-dessus du clavier.

### Scrolling — le sous-système à copier le plus soigneusement
Séparation en 4 pièces à responsabilité unique :
- **`ScrollPosition`** : l'état — `pixels`, `min/maxScrollExtent`, `viewportDimension` ;
  `Listenable` ; **ne démarre aucun mouvement lui-même**, délègue à l'*activity*.
- **`ScrollController`** : la façade que l'app tient — `offset`, `animateTo`, `jumpTo`.
- **`ScrollPhysics`** : chaîne composable — `applyPhysicsToUserOffset` (résistance),
  `applyBoundaryConditions` (clamp aux bords), **`createBallisticSimulation(metrics,
  velocity) -> Simulation?`** (le momentum de fling : renvoie un ressort/friction que le
  ticker échantillonne). Variantes `Clamping`(Android)/`Bouncing`(iOS).
- **`ScrollActivity`** : la machine « comment ça bouge » — `Idle/Hold/Drag/Ballistic/
  Driven`.
- **Adaptation taffy** : taffy layoute **tout** le contenu une fois → `maxScrollExtent =
  contentSize − viewportDimension`. Un viewport Frus = **clip + translation des enfants
  de `-pixels`**. **Pas besoin de slivers paresseux pour la v1** (acceptez le coût de
  layout complet, optimisez après). Pilotez l'activity ballistique depuis votre boucle
  ticker winit. Colle à Elm : `pixels` en état retenu, l'activity/ticker émet des `Msg`.
  Vous avez déjà un ressort de scroll dans `runtime.rs` — c'est le germe de la physics.

### Overlay, Navigator & insets ambiants
- **`Overlay`** = pile de couches flottantes indépendantes (`OverlayEntry`, `opaque`,
  `maintainState`) — substrat de tout ce qui est « au-dessus » (dialogues, feuilles,
  tooltips, feedback de drag). **`OverlayPortal`** ancre un enfant d'overlay à la position
  d'un widget — directement pertinent pour votre BottomSheet.
- **`Navigator`** = pile de `Route` qui **poussent des `OverlayEntry`** ; `ModalRoute`
  ajoute barrière + scope de focus. → **Recastez BottomSheet/modale en overlay entry +
  scope de focus + scrim** ; une pile de routes légère viendra après.
- **Insets ambiants** : distinguez **`padding`** (notch/barres — statique) de
  **`viewInsets`** (clavier — dynamique). `SafeArea` **consomme puis remet à zéro** pour
  les descendants (`removePadding`) → une SafeArea dans une SafeArea ne padde pas deux
  fois. Vous avez déjà `on_insets`/SafeArea (Jalon 51) ; formalisez ces deux valeurs et
  la règle consume-then-zero, et alimentez `viewInsets.bottom` depuis la hauteur du
  clavier Android.

### Frontière native (channels)
Flutter : un `BinaryMessenger` async bidirectionnel sous des façades typées
(`MethodChannel`/`EventChannel`), canaux nommés centralisés (`textinput`, `keyevent`,
`platform`, `lifecycle`, `system`). **Pour Frus** : desktop = winit *est* la frontière
(câblage direct). **Android** = seam JNI/FFI (vous avez déjà `android_main`). Ne bâtissez
pas un bus dynamique string+codec : définissez des **enums Rust typées traversant le FFI,
une par sujet** (textinput, insets/window/lifecycle, system=clipboard/haptics/orientation/
back). Volez la *discipline* : **une frontière étroite, async, bidirectionnelle**, avec
formes app→natif et natif→app.

### Sémantique / accessibilité (planifier, ne pas sur-construire)
Flutter bâtit un **arbre sémantique parallèle** (label/value/hint/flags/role/actions),
batché puis flushé vers Android `AccessibilityNodeInfo`. **Minimal Frus** : une annotation
optionnelle par widget (`role`, `label`, `value`, `flags`, rect, quelques `actions` :
activate/scroll/focus), un arbre plat clé par identité, pont Android via un provider de
vues virtuelles sur le FFI. Réutilisez l'arbre de focus pour l'ordre de lecture au début.
**Bakez le hook `label` dans les widgets dès maintenant** pour éviter un retrofit massif.

### Ordre de construction recommandé (§6)
1. **Arbre de focus + routage des touches** (prérequis de tout).
2. **Modèle clavier régularisé** (winit → physical+logical+character).
3. **Scrolling** (les 4 pièces + viewport clip+translate ; clamping d'abord).
4. **Insets : split `padding`/`viewInsets` + consume-then-zero** (petit refactor de
   SafeArea, débloque l'évitement clavier).
5. **Couche Overlay pour modales/feuilles** (recast BottomSheet).
6. **Saisie/IME** (desktop winit d'abord, puis Android via FFI + `composing`).
7. **Channels FFI typés Android**.
8. **Hooks sémantiques** (annotations maintenant, arbre + pont Android en dernier).

---

## 7. Feuille de route proposée (prochains jalons)

En croisant les 6 briefs avec l'état réel de Frus (61 widgets breadth-first, Scene à
2 primitives, thème plat, ressorts par-widget, pas d'arène/focus/scroll générique), voici
un ordre à fort levier. Frus est **large mais peu profond** : ces jalons ajoutent la
*profondeur de moteur* qui manque sous la vitrine de widgets.

**Bloc A — Fondations moteur (le plus gros levier)**
1. **Phases de frame + listes dirty séparées** (`build→layout→paint→composite`),
   chaque `Msg`/`Command` posant le bit le plus étroit possible. (§1, §0)
2. **Cache de frontière de relayout au-dessus de taffy** `(contraintes, taille, dirty)`.
   (§1)
3. **Pilote d'animation auto-reprogrammé** lié à `request_redraw`, timestamp de frame,
   retour à l'idle au repos ; **généraliser le ressort BottomSheet en `trait
   Simulation`**. (§1, §4)

**Bloc B — Entrée & mouvement**
4. **Palier 0 gestes** : `PointerEvent` normalisé (+`Cancel`), hit-test taffy caché par
   id, `PointerRouter`. (§3)
5. **Palier 1 gestes** : reconnaisseur tap-ou-drag + long-press, parlant déjà le
   vocabulaire d'arène → `on_tap/on_drag/on_long_press` émettant des `Msg`. (§3)
6. **Scrolling** : `ScrollPosition/Controller/Physics/Activity` + viewport clip+translate
   sur taffy, fling via ticker. (§6)

**Bloc C — Système de design & texte**
7. **Types de peinture cœur** : `EdgeInsets`/`Alignment`/`BorderRadius`/`BoxShadow` +
   **primitives de scène** RRect-arrondi/ombre/gradient dans `frus-gpu` (verrouiller la
   convention sRGB/prémultiplié). (§5)
8. **`BoxDecoration`** (ordre fixe, `content_padding`→taffy). (§5)
9. **Thème structuré** : `ColorScheme` (rôles M3, clair/sombre à la main) + `TextTheme`
   (15 slots) + Shape/Elevation/Spacing + state-layer bakée. Migration progressive des
   61 widgets vers les rôles. (§5)
10. **`TextLayout`** sur cosmic-text (intrinsèques→taffy, caret, sélection). (§5)

**Bloc D — Structure & finitions**
11. **Focus + clavier régularisé** (feuille→racine, 3 états, highlight mode). (§6)
12. **Insets `padding`/`viewInsets` + consume-then-zero** ; **Overlay** pour modales
    (recast BottomSheet). (§6)
13. **Clés formalisées** (`enum {Index,Value,Unique}`) + **dump diagnostique** des deux
    arbres (config vs retenu). (§2)
14. **Saisie/IME** (desktop puis Android FFI), **channels FFI typés**, **hooks
    sémantiques** (labels dès maintenant). (§6)

**Palier 2+ / à différer** : vraie arène de gestes (imbrications scrollables), vélocité
LSQ, scale/pinch, slivers paresseux, `from_seed` HCT, `ShapeBorder`/images, compositing
bits, arbre sémantique complet.

---

---
---

# PARTIE II — Ce qui manque pour gagner le marché

> La Partie I (§0–§7) donne le **moteur** : elle assure que Frus n'est pas plafonné
> architecturalement. Mais un moteur excellent ne gagne pas un marché. Ce qui gagne,
> c'est l'**ergonomie pour le dev**, la **portée** (plateformes), le **tooling**, le
> **design par défaut**, et un **positionnement clair face aux vrais concurrents** —
> qui ne sont **pas Flutter**, mais l'écosystème UI Rust (iced, egui, Slint, Dioxus,
> gpui, Xilem…). Cette partie couvre tout ça.
>
> Fil conducteur de la Partie II : **un dev Rust doit se sentir chez lui dès la
> minute 1** — cargo, types, messages exhaustifs, zéro lutte avec le borrow checker,
> compilation supportable, et un défaut magnifique sans config.

---

## 8. Ergonomie Rust — le cœur (les devs Rust doivent se sentir chez eux)

C'est **la** priorité de la demande. Un dev Rust juge un framework en 10 minutes sur :
« est-ce que ça compile vite, est-ce que l'API lit bien, est-ce que je me bats avec le
borrow checker, est-ce que les erreurs sont claires ». Décisions concrètes :

### API : builders qui lisent bien + macros *optionnelles*
- **Évitez `Box::new(...)` partout.** Fournissez des fonctions libres qui renvoient
  `impl Widget<Msg>` et des méthodes builder chaînables. Le sweet spot (celui d'iced) :
  ```rust
  column![
      text("Hello").size(24),
      button("Save").on_press(Msg::Save),
      row![checkbox("Enable", cfg.enabled).on_toggle(Msg::Toggle)].spacing(8),
  ].spacing(12).padding(16)
  ```
  Les macros `row!`/`column!`/`stack!` ne font qu'emballer `vec![...]` en gérant le
  `Box`/`into()` — **jamais** un DSL magique qui casse rust-analyzer. Un dev doit
  pouvoir tout écrire en Rust pur *sans* macro s'il préfère.
- **`into()` implicite** : `impl From<&str> for Text`, `impl From<T: Widget> for Element`
  pour que les enfants s'écrivent sans cérémonie.
- **`#[must_use]`** sur les widgets/commands, **newtypes** partout (`Spacing(f32)`,
  `Radius(f32)`) — les devs Rust adorent que le type raconte l'intention.

### Le générique `Widget<Msg>` : la composition par `.map()` est **le** point vital
C'est la friction n°1 d'Elm en Rust et **le** déverrouilleur de scalabilité. Un
sous-composant émet son propre `ChildMsg` ; le parent le remappe :
```rust
child.view().map(Msg::Child)   // Widget<ChildMsg> -> Widget<Msg>
```
`Element::map` doit être **first-class, zéro-coût perçu, et documenté en page 1**. Sans
lui, tout finit dans un « god enum » `Msg` ingérable. Avec lui, Frus compose comme iced.
Vérifiez que `map` traverse aussi `on_edit`/gestures/subscriptions, pas seulement le clic.

### Zéro lutte avec le borrow checker (design par valeur)
- **La `view` prend `&State` et rend une valeur possédée** ; les widgets sont `Copy`/
  `Clone`-friendly ou construits à la volée. Jamais d'API qui force `Rc<RefCell<>>` côté
  utilisateur.
- **`paint(&self, …)`**, ids `Copy`, état retenu dans le shell (déjà votre modèle) →
  l'utilisateur ne tient jamais de référence mutable dans un arbre. C'est *déjà* le bon
  choix ; protégez-le comme un invariant d'API.
- **Messages = `enum` + `#[derive(Clone, Debug)]`** ; l'exhaustivité du `match` dans
  `update` est un **filet de sécurité que les devs Rust adorent** — le compilateur
  interdit d'oublier un cas.

### Compilation supportable (sinon vous perdez avant de commencer)
- **Découpage en crates** (vous l'avez déjà : core/gpu/layout/text/widgets/shell) →
  recompilation incrémentale ciblée. Gardez `frus-demo` **mince**.
- **Feature flags par famille de widgets** (`features = ["forms", "nav", "data"]`) : un
  dev ne compile que ce qu'il utilise → binaire *et* temps de compile réduits.
- **Mode dev à liaison dynamique** (façon Bevy `dynamic_linking`) : une feature qui
  charge `frus` en `.so`/`.dll` pour couper le temps de link en debug.
- Documentez l'usage du **backend Cranelift** en debug et de `lld`/`mold` comme linker —
  gains de temps de compile immédiats, très appréciés.

### rust-analyzer & découvrabilité
- Types **concrets et nommés**, pas des `impl Trait` opaques partout dans les signatures
  publiques critiques (l'autocomplétion et les messages d'erreur en pâtissent).
- **Doc-tests exécutables** et `examples/` lançables par `cargo run --example gallery`.
  Un dev Rust apprend par l'exemple compilable, pas par la prose.

---

## 9. Composition & Elm à l'échelle (tuer le boilerplate)

Elm est simple sur une petite app et **verbeux** sur une grande si on ne donne pas les
outils. Ce qu'il faut fournir *en tant que framework* (pas laisser chaque dev réinventer) :

- **Le pattern « composant » documenté et outillé** : `{ Model, Msg, update, view }` par
  sous-partie, branché au parent via `child.update(msg)` + `child.view().map(Msg::Child)`.
  Fournissez un **exemple canonique** (une app à 3 écrans) qui montre le remapping des
  Msg et des Command — c'est la référence que les devs copieront.
- **Namespacing des messages** : `enum Msg { Header(header::Msg), List(list::Msg), … }`
  au lieu d'un enum plat de 200 variantes.
- **Mémoïsation (`lazy`/`memo`)** : le talon d'Achille perf d'Elm est que `view` se
  reconstruit entièrement chaque frame. Fournissez un `lazy(deps, || build_subtree())`
  (comme `iced::widget::lazy`) qui **ne reconstruit un sous-arbre que si `deps` change** ;
  combiné à votre réconciliation par `child_id`, ça borne le coût de rebuild. **À prévoir
  tôt** — c'est ce qui rend les grosses listes/tableaux (vous avez déjà `table`, `tree`,
  `list`) tenables.
- **Reconnaître la tension « signals »** : Dioxus/Leptos/floem/Xilem vont vers la
  réactivité fine (signaux) pour la perf et l'ergonomie. Frus a choisi Elm — **assumez-le**
  (débogage prévisible, source unique, tests triviaux) et compensez la perf par
  `lazy`+réconciliation, plutôt que d'hybridiser. Notez-le comme choix explicite, pas
  comme oubli.

---

## 10. Async, effets & `Command` (indispensable aux vraies apps)

Une app réelle fait de l'IO : réseau, disque, timers, sous-processus. Sans un modèle
d'effets propre, Frus reste une démo. Le modèle Elm/iced :

- **`Command<Msg>` intègre le futur** : `Command::perform(future, |result| Msg::…)`
  exécute une `Future` **hors thread UI** et réinjecte un `Msg` au résultat. C'est le
  seul pont autorisé du monde impur vers `update`.
- **Runtime async pluggable** : ne vous mariez pas à tokio en dur. Un trait
  `Executor` (impl pour tokio / async-std / smol / un pool maison) que le dev choisit à
  `run(app, settings)`. Beaucoup de devs Rust ont *déjà* un runtime ; ne leur en imposez
  pas un second.
- **`Subscription<Msg>`** pour les flux longs : websocket, ticks d'horloge, événements
  système, watch de fichiers. Déclarées dans `subscription(state)`, diffées par identité
  (démarre/arrête quand elles apparaissent/disparaissent) — exactement le modèle iced,
  et ça sert *aussi* votre pilote d'animation (§4).
- **Annulation & batching** : `Command::batch([...])`, et une `Command` liée à un `id`
  annulable (une requête qu'on abandonne si l'écran change). Sans annulation, les apps
  fuient des tâches.
- **Règle d'or** : `update` reste **pur et synchrone** ; tout le côté impur vit dans les
  Command/Subscription exécutées par le shell. C'est ce qui garde `update` testable
  (§13) — un avantage Elm que vous ne devez jamais sacrifier.

---

## 11. Portée = marché : desktop, mobile, **web**, embarqué

La portée, c'est l'adressable. Chaque plateforme manquante est un marché perdu.

- **Web (wasm) — le plus gros différenciateur.** wgpu cible **WebGPU avec repli WebGL2**.
  Un `view` Frus qui tourne dans le navigateur *sans réécriture* est un argument massif
  (c'est ce qui a fait décoller Dioxus/Leptos). winit supporte le canvas web. Priorité
  haute : ça multiplie l'audience et fait des démos partageables par URL (adoption).
- **iOS** : vous avez **déjà Android** (rare et précieux — iced/egui y sont faibles).
  Compléter par iOS fait de Frus **le** framework Rust « desktop + mobile beau », un
  créneau quasi vide.
- **Embarquer dans une app existante** : rendu dans une sous-fenêtre winit *ou* rendu
  **offscreen vers une texture** que l'hôte compose. Ça ouvre le marché « une vue Frus
  dans une app Qt/natif/jeu ».
- **Multi-fenêtre** : plusieurs `Window`, une `Application` — nécessaire pour de vrais
  outils desktop (palettes, inspecteurs).
- **Embarqué/`no_std`** : probablement hors scope, mais **c'est là que Slint gagne**
  (microcontrôleurs, IHM industrielles). Décidez consciemment de ne pas y aller — ou d'y
  aller comme axe de conquête distinct.

Chaque cible partage le même `view`/`update` : la portée est surtout un travail de
**frus-shell** (le seam winit/plateforme) et de `frus-gpu` (backends wgpu), pas du code
applicatif. C'est un avantage structurel — capitalisez dessus dans le marketing.

---

## 12. Ingénierie de performance (au-delà du pipeline §1)

Le pipeline de phases (§1) donne le *quand recalculer*. Voici le *comment dessiner vite* —
ce qui fait la fluidité perçue et « démarre/scrolle sans jank » :

- **Batching GPU / minimiser les draw calls.** Regroupez les quads en **un seul buffer
  instancié** par pipeline (rects arrondis, ombres, glyphes). Une UND typique doit tenir
  en une poignée de draw calls, pas un par widget. Vos primitives de scène (§5) doivent
  être conçues pour l'instancing dès le départ.
- **Atlas de glyphes.** glyphon gère déjà un atlas ; assurez-vous de **ne pas re-shaper**
  un texte inchangé (cache par `(texte, style, largeur)` → `TextLayout`). Le texte est
  souvent le poste n°1 en CPU.
- **Régions de dommage (damage/scissor).** Couplé aux frontières de repeinture (§1) :
  ne re-rendez que le **rect sale** via un scissor rect wgpu. Un curseur qui clignote ne
  doit pas re-dessiner l'écran.
- **Zéro alloc par frame** sur le chemin chaud : bump-arena réinitialisée chaque frame
  pour l'arbre `view` (contre le churn Elm noté au but 2), buffers GPU réutilisés.
- **Budget de frame explicite** (16,6 ms @60 Hz, 8,3 ms @120 Hz) et **profilage
  intégré** (`tracy`/`puffin` derrière une feature). Un framework qui gagne se *mesure*.
- **Harnais de benchmark reproductible** : rendu **offscreen + readback pixel** (vous
  avez déjà ce pattern côté WSL/llvmpipe) → golden perf + non-régression en CI.

---

## 13. Tests, tooling & hot-reload — la DX qui fait gagner

C'est souvent ce qui *décide* l'adoption, à moteur égal.

- **`update` pur = tests unitaires triviaux** (avantage Elm massif) : `assert_eq!(
  update(state, Msg::Increment).0, expected)` sans GPU ni fenêtre. **Mettez ça en avant** ;
  c'est un argument que ni egui ni gpui n'ont aussi propre.
- **Tests de rendu headless (golden/snapshot)** : rendu offscreen → comparaison d'image
  de référence, tolérance de pixels. Vous avez déjà l'infra WSL/offscreen — packagez-la
  en `frus-test`.
- **Hot-reload** : la faiblesse identifiée (but 4). Deux leviers, à combiner :
  1. **Rechargement préservant l'état** — Elm rend ça *plus facile* que partout ailleurs :
     l'état est **une struct unique** ; sérialisez-la (serde), rechargez la lib, ré-hydratez.
     Regardez `hot-lib-reloader` et surtout **`subsecond`** (le hot-patching Rust de
     Dioxus) — c'est l'état de l'art et ça marche avec du Rust pur.
  2. **Live preview** de la `view` (façon Slint) : un mode où éditer `view` recharge sans
     relancer. Combiné à (1), vous approchez l'itération Flutter.
- **Inspector runtime** : exposez le **dump diagnostique** (§2) en overlay (arbre + rects
  + ids + état retenu). Un dev qui *voit* pourquoi son identité casse au réordonnancement
  reste. C'est peu de code pour un énorme retour.
- **`cargo`-natif** : `cargo new --template frus-app`, `cargo run`, `cargo test` marchent
  sans outil externe. Le dev Rust ne veut pas d'un CLI propriétaire (le péché de certains
  concurrents). Restez dans cargo autant que possible.

---

## 14. i18n / l10n / RTL / accessibilité (approfondi)

- **RTL & bidi** : la base directionnelle est prévue (§5, `EdgeInsetsDirectional::resolve`).
  Étendez au **miroir de layout** (row inversée en RTL) et au **texte bidi** (cosmic-text
  le gère — exposez-le). Un `TextDirection` dans l'`Env` (§2) propagé au paint.
- **Localisation** : intégrez **Fluent** (`fluent-rs`, le standard i18n Rust de Mozilla)
  pour les messages, et les formats nombres/dates/pluriels par locale. Ne réinventez pas.
- **Accessibilité : adoptez AccessKit — ne réinventez PAS.** `AccessKit` est le standard
  Rust cross-plateforme d'a11y (UIA Windows, AT-SPI Linux, macOS, et un provider Android/
  web) ; **egui, Slint, Xilem et Bevy l'utilisent déjà**. Vous mappez votre annotation
  sémantique par widget (§6 : role/label/value/actions) vers l'arbre AccessKit, et il
  parle aux lecteurs d'écran natifs. C'est *le* raccourci pour une a11y crédible et un
  **argument de conformité** (marché entreprise/public). Bakez le hook `label` dans les
  widgets **dès maintenant** (§6) ; branchez AccessKit ensuite.

---

## 15. Distribution & packaging (la dernière ligne droite avant l'utilisateur)

- **Binaire unique, sans runtime à installer** (contre Electron/Tauri-webview) — argument
  de vente : « un `.exe`/`.app` autonome de quelques Mo ». Assumez le poids wgpu (but 3),
  mais restez sous Flutter.
- **Bundling par plateforme** : `cargo-apk` (déjà, Android), `cargo-bundle`/`cargo-dist`
  pour `.app`/`.msi`/`.deb`/AppImage, wasm-bindgen + trunk pour le web. Documentez une
  commande par cible.
- **Assets embarqués** (polices, images, i18n) via `include_bytes!`/`rust-embed` → **zéro
  fichier externe**, démarrage déterministe (et ça sert le but 3 : ne jamais scanner les
  polices système).
- **Taille** : `opt-level="z"`/`s`, `lto=true`, `strip=true`, `panic="abort"` en release ;
  documentez un profil « minimal ». Publiez les chiffres (un « hello world » à N Mo) —
  la transparence sur la taille rassure.

---

## 16. Positionnement : comment Frus gagne le marché Rust UI

**Le piège** : viser Flutter. Le marché adressable de Frus, ce sont les devs **Rust** qui
choisissent un toolkit UI *aujourd'hui*. Les vrais concurrents :

| Framework | Modèle | Rendu | Forces | Faiblesses (la brèche de Frus) |
|---|---|---|---|---|
| **iced** | **Elm** (le + proche !) | wgpu | Mature, propre, cross-desktop | Austère, **mobile faible**, peu de design system, peu de widgets riches |
| **egui** | Immédiat | wgpu/gl | Ultra-simple, roi des outils/jeux | Non-retenu, style limité, pas « app grand public » |
| **Slint** | DSL `.slint` | maison/Skia | **Live preview**, embarqué, tooling | Langage propriétaire (pas Rust pur), licence commerciale |
| **Dioxus** | React/RSX + signaux | webview/wgpu(Blitz) | **Hot reload**, web/mobile, familier | Rendu natif jeune, pas Elm |
| **gpui** (Zed) | Impératif retenu | GPU maison | **Perf extrême**, prouvé par Zed | Peu documenté/ouvert, courbe raide |
| **Xilem** | Réactif (diff) | Vello/wgpu | Backing Linebender, futur | Expérimental, API mouvante |
| **floem / Makepad / Freya** | Signaux / DSL shader / Skia+Dioxus | divers | Niches (éditeurs, design live) | Petits écosystèmes |

**Le créneau vide que Frus occupe :** *« iced, mais avec un vrai design system Material 3,
des animations physiques, et le mobile qui marche »*. Vous avez **déjà** trois wedges que
peu ont réunis :
1. **Android fonctionnel** (Jalon 50) — iced/egui/Slint-pur y sont faibles.
2. **61 widgets** dont des riches (table, tree, datepicker, carousel, autocomplete) —
   large *avant* les autres.
3. **Theming structuré + animations à ressort** — la voie vers un défaut *magnifique*.

**Ce qu'il faut ajouter pour transformer l'essai (par ordre d'impact sur l'adoption) :**
1. **Un défaut visuellement superbe, sans config** (thème M3 §5 + animations §4). La
   première capture d'écran décide. C'est le levier marketing n°1.
2. **Web (wasm)** (§11) — démos partageables par URL = adoption virale.
3. **DX : hot-reload + compile rapide + `cargo`-natif** (§8, §13) — la friction qui fait
   fuir.
4. **iOS** (§11) — verrouille le créneau « le framework Rust desktop+mobile ».
5. **AccessKit + i18n** (§14) — débloque le marché entreprise/public.
6. **Docs + galerie d'exemples + `cargo new` template** (§8) — la porte d'entrée.

**Honnêteté stratégique** : on ne « gagne » pas en battant *tout le monde partout*. On
gagne en **dominant un créneau** : *belles apps Rust, desktop + mobile + web, sans GC,
démarrage instantané, design par défaut premium.* C'est défendable, c'est vide, et vos
buts 2 et 3 (mémoire, légèreté/démarrage) en sont la preuve technique. Le reste de la
Partie II est la liste de ce qui manque pour *tenir* ce créneau.

---

## 17. Synthèse stratégique — les 3 piliers & la roadmap consolidée

**Les 3 piliers différenciateurs** (à ne jamais diluer) :
- **Pilier A — Performance native honnête** : sans GC, démarrage instantané (polices
  bundlées, init GPU paresseuse), empreinte maîtrisée, 120 Hz sans jank. *(buts 2 & 3 ;
  §1, §12, §15)*
- **Pilier B — Le plus beau des toolkits Rust, sans effort** : design M3 par défaut,
  animations physiques, 61+ widgets polis. *(§4, §5)*
- **Pilier C — Le seul « desktop + mobile + web » Rust confortable** : Android déjà là,
  puis iOS + wasm, sur un `view`/`update` partagé, avec une DX qui ne fait pas fuir.
  *(§8, §10, §11, §13)*

**Ordre consolidé** (fusionne la roadmap §7 avec la Partie II) :
1. **Fondations moteur** (Bloc A du §7) — phases/dirty, cache relayout, pilote d'animation.
2. **DX minimale qui retient** — `.map()` first-class (§8), `lazy`/memo (§9), `Command`
   async + `Subscription` (§10), inspector + tests headless (§13). *Sans ça, personne ne
   reste, quel que soit le moteur.*
3. **Design par défaut premium** — thème M3, primitives de scène (rrect/ombre/gradient),
   `BoxDecoration`, `TextLayout` (§5) + animations généralisées (§4). *Le pilier B, votre
   marketing.*
4. **Entrée & mouvement** — gestes paliers 0→1, scrolling physique (§3, §6).
5. **Portée** — web/wasm d'abord (adoption), puis iOS ; multi-fenêtre, embedding (§11).
6. **Confiance & conformité** — AccessKit, i18n Fluent, focus/clavier (§14, §6).
7. **Distribution** — bundling par cible, template `cargo new`, galerie d'exemples,
   binaire minimal chiffré (§15, §8).
8. **Hot-reload state-preserving** (`subsecond`) — le coup de grâce sur le but 4 (§13).

**À différer sans culpabilité** : vraie arène de gestes (imbrications), signals, slivers
paresseux, `from_seed` HCT, `no_std`/embarqué, arbre sémantique complet au-delà d'AccessKit.

---

## Annexe — à NE PAS copier de Flutter (récapitulatif)
- `StatefulWidget`/`State`/lifecycle, `GlobalKey`, `InheritedWidget` + graphe de
  dépendances, `ChangeNotifier`/`ValueNotifier` comme état d'app → **Elm les remplace**.
- Les maths flex/grid `performLayout` → **taffy**.
- Le walking `markNeeds*` d'un arbre à pointeurs parents mutables → **arène/slotmap +
  listes dirty d'`Id`**.
- La double voie `RawKeyEvent`/`KeyEventManager` (dépréciée) → modèle KeyEvent unique.
- Slivers paresseux → différer jusqu'à ce que le coût du layout-complet-au-scroll morde.
- La ré-entrance `scheduleMicrotask` (arène) → fonctions **pures renvoyant les outcomes**,
  drainées après l'emprunt.
