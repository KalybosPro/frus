# Design notes

This directory is the project's memory: **276 milestone notes** (*jalons*), one per step of
frus's construction. Each records the objective, the alternatives that were weighed, the
decision and its reasoning, the implementation, how it was verified, and what was
deliberately left for later.

When you find yourself asking *"why on earth is it done this way?"*, the answer is almost
always here — along with the option that was rejected and why. `grep` this directory before
opening an issue.

> **These notes are written in French.** Translating them to English is a genuinely valuable
> contribution — see [ROADMAP.md](../ROADMAP.md).

## Other documents

| Document | What it is |
| --- | --- |
| [getting-started.md](getting-started.md) | Write and run your first frus application |
| [cahier des charges.md](cahier%20des%20charges.md) | The original brief: vision, philosophy, working method |
| [prior art](idees-flutter.md) | Ideas from mature UI toolkits, evaluated for porting — what to take, what to fix |
| [etat.html](etat.html) | A visual snapshot of the framework's state |

## Where to start

If you're new to the codebase, these are the notes worth reading first:

| Milestone | Why it matters |
| --- | --- |
| [0](jalon-0.md) – [4](jalon-4.md) | The foundations: a window, the 2D renderer, layout, the widget tree, interactivity |
| [129](jalon-129.md), [131](jalon-131.md) | The web target, and shrinking the wasm payload |
| [267](jalon-267.md), [268](jalon-268.md) | The single entry point (`main!`) and the `frus` facade crate |
| [270](jalon-270.md) – [275](jalon-275.md) | Async effects, `fetch`, `RemoteData`, typed JSON |

## All milestones

| # | Title |
| --- | --- |
| [0](jalon-0.md) | Une fenêtre + un quad coloré |
| [1](jalon-1.md) | Moteur de rendu 2D minimal (primitives) |
| [2](jalon-2.md) | Moteur de mise en page (flexbox via taffy) |
| [3](jalon-3.md) | Arbre de widgets déclaratif |
| [4](jalon-4.md) | Interactivité : événements + état |
| [5](jalon-5.md) | Texte |
| [6](jalon-6.md) | Identité des widgets + états d'interaction |
| [7](jalon-7.md) | Style : coins arrondis, bordures, alignements, marges par côté |
| [8](jalon-8.md) | Saisie de texte + focus clavier |
| [9](jalon-9.md) | Scroll vertical + clipping |
| [10](jalon-10.md) | Curseur, navigation, sélection et presse-papier |
| [11](jalon-11.md) | Animations (transitions implicites) |
| [12](jalon-12.md) | Ombres, dégradés, scroll horizontal, barre & drag, animations de focus |
| [13](jalon-13.md) | Opacité + apparition en fondu |
| [14](jalon-14.md) | Disparition en fondu (rétention des sortants) |
| [15](jalon-15.md) | Système de thèmes |
| [16](jalon-16.md) | Bibliothèque de widgets nommés (themés) |
| [17](jalon-17.md) | Overlay / portail (menus flottants, tooltips, modales) |
| [18](jalon-18.md) | Navigation (pile d'écrans + transitions glissées) |
| [19](jalon-19.md) | Transitions d'état · Geste retour · Overlay avancé |
| [20](jalon-20.md) | App exemple réelle : liste de tâches (todo) |
| [21](jalon-21.md) | Séparation framework / application (`run(app)`) |
| [22](jalon-22.md) | Barre de navigation (`NavBar`) + titres animés |
| [23](jalon-23.md) | Défilement avec inertie (ressort + rebond) |
| [24](jalon-24.md) | `Command` / effets depuis `update` |
| [25](jalon-25.md) | DPI / facteur d'échelle (HiDPI) |
| [26](jalon-26.md) | Subscriptions (sources continues de messages) |
| [27](jalon-27.md) | DX / ergonomie (écrire une UI plus vite) |
| [28](jalon-28.md) | Reconciliation par clé (identité stable) |
| [29](jalon-29.md) | Navigation clavier / accessibilité |
| [30](jalon-30.md) | Robustesse fenêtre |
| [31](jalon-31.md) | Liste virtualisée (`List`) |
| [32](jalon-32.md) | Nouveaux widgets (6) |
| [33](jalon-33.md) | Nouveaux widgets : Collapsible, Menu, Chip |
| [34](jalon-34.md) | Nouveaux widgets : Avatar, Stepper, Rating |
| [35](jalon-35.md) | Layout : grille (`Grid`) |
| [36](jalon-36.md) | Nouveaux widgets : Table, SegmentedControl, Toast |
| [37](jalon-37.md) | Nouveaux widgets : Breadcrumb, Pagination, Skeleton |
| [38](jalon-38.md) | Nouveaux widgets : Tree, ColorPicker, Timeline |
| [39](jalon-39.md) | Correctif clic + nouveaux widgets : DatePicker, Carousel, Alert |
| [40](jalon-40.md) | Nouveaux widgets : Popover, Autocomplete, Kbd |
| [41](jalon-41.md) | Colorimétrie sRGB / linéaire |
| [42](jalon-42.md) | Responsivité par défaut |
| [43](jalon-43.md) | Layout adaptatif (navigation & maître-détail) |
| [44](jalon-44.md) | Échelle & taille dynamiques |
| [45](jalon-45.md) | Responsivité widgets avancée |
| [46](jalon-46.md) | Animation du tiroir (glissement + fondu) |
| [47](jalon-47.md) | Tiroir droit & tiroir permanent |
| [48](jalon-48.md) | Glissement du tiroir en courbe de ressort |
| [49](jalon-49.md) | Feuille modale (`BottomSheet`) |
| [50](jalon-50.md) | Premier run sur Android physique |
| [51](jalon-51.md) | Insets système (zone de sécurité / SafeArea) |
| [52a](jalon-52a.md) | Jalon 52a — AppBar adaptative (barre d'application Material) |
| [52b](jalon-52b.md) | Jalon 52b — `Scaffold` unifié (ossature d'écran Material) |
| [53](jalon-53.md) | Physique unifiée (`trait Simulation`) |
| [54](jalon-54.md) | Couche d'animation atteignable + transitions du démo dessus |
| [55](jalon-55.md) | Cache de frontière de relayout (layout retenu au-dessus de taffy) |
| [56](jalon-56.md) | Phases de frame : build conditionnel (build → paint) |
| [57](jalon-57.md) | `BoxDecoration` : le modèle de décoration de boîte (§5) |
| [58](jalon-58.md) | Thème : state-layers Material bakées + rôles M3 étendus |
| [59](jalon-59.md) | Généralisation des state-layers Material |
| [60](jalon-60.md) | Typographie : `TextStyle` + `TextTheme` (graisse et italique rendus) |
| [61](jalon-61.md) | AppBar/NavBar entièrement personnalisables |
| [62](jalon-62.md) | `TextSpan` : texte riche, de l'arbre stylé au GPU |
| [63](jalon-63.md) | `TextLayout` : caret, hit-test et sélection sur cosmic-text |
| [64](jalon-64.md) | Mesure sous contraintes (closures taffy) + paragraphe à retour à la ligne |
| [65](jalon-65.md) | `RichText::wrap()` : le paragraphe riche replié |
| [66](jalon-66.md) | `BorderRadius` : rayons d'arrondi **par coin** (SDF) |
| [67](jalon-67.md) | Adoption du par-coin (feuille, segments) + la bordure réserve sa place |
| [68](jalon-68.md) | `ColorScheme` : les rôles consolidés (source de vérité unique) |
| [69](jalon-69.md) | Gestes, paliers 0+1 : entrée normalisée + appui long |
| [70](jalon-70.md) | Focus : anneau clavier-seul + navigation aux flèches (géométrique) |
| [71](jalon-71.md) | Touches feuille→racine (3 états) : Échap ferme partout |
| [72](jalon-72.md) | Scopes de focus : la modale piège Tab, les flèches et le clic |
| [73](jalon-73.md) | Fling tactile : le momentum de défilement (balistique) |
| [74](jalon-74.md) | Insets fenêtre : split `padding` / `viewInsets` (évitement clavier) |
| [75](jalon-75.md) | Décorations de texte (soulignement, barré, surlignement) |
| [76](jalon-76.md) | `from_seed` : thème généré depuis une couleur graine (HCT) |
| [77](jalon-77.md) | `frus-test` : rendu headless, snapshots et goldens (ouverture du §13) |
| [78](jalon-78.md) | Inspecteur runtime (§13, palier 1) |
| [79](jalon-79.md) | Live-reload préservant l'état (§13) |
| [80](jalon-80.md) | Clavier logiciel Android (ouverture du chantier saisie §6) |
| [81](jalon-81.md) | Pont InputConnection Android (§6, palier 2) |
| [82](jalon-82.md) | Saisie IME palier 3 : composition stylée + contexte des suggestions |
| [83](jalon-83.md) | Démarrage en une commande (`cargo generate`) : clôture du §13 |
| [84](jalon-84.md) | RTL : direction de lecture et miroir de mise en page (§14, ouverture) |
| [85](jalon-85.md) | Accessibilité : annotation sémantique + pont AccessKit |
| [86](jalon-86.md) | Localisation (i18n/l10n) : Fluent |
| [87](jalon-87.md) | Écriture arabe (bidi) : rendu du script + correctif RTL hors-écran |
| [88](jalon-88.md) | Phases de frame & cache de frontière de repaint |
| [89](jalon-89.md) | Chemins vectoriels & icônes |
| [90](jalon-90.md) | Images & textures |
| [91](jalon-91.md) | Décodage d'images (PNG/JPEG) |
| [92](jalon-92.md) | Compositing par calques & précompilation des pipelines |
| [93](jalon-93.md) | Anti-aliasing (MSAA) |
| [94](jalon-94.md) | Réutilisation GPU des textures de calque |
| [95](jalon-95.md) | Animations implicites : courbe & durée par widget |
| [96](jalon-96.md) | Opacité de groupe & `AnimatedOpacity` |
| [97](jalon-97.md) | `AnimatedContainer` : couleur de fond animée |
| [98](jalon-98.md) | `AnimatedContainer` : taille animée (au layout) |
| [99](jalon-99.md) | `AnimatedContainer` : rayon de coin animé |
| [100](jalon-100.md) | Widgets nommés : `Opacity`, `AnimatedOpacity`, `AnimatedContainer` |
| [101](jalon-101.md) | Animations explicites : `repeat` / `stop` / `reset` |
| [102](jalon-102.md) | `AnimatedContainer` : marge (padding) animée |
| [103](jalon-103.md) | `Animatable` : le pont explicite → valeur typée vivante |
| [104](jalon-104.md) | `Animatable` composés : `TweenSequence` + tweens de boîte |
| [105](jalon-105.md) | Container : `alignment` + `decoration` composite |
| [106](jalon-106.md) | `Alignment` fractionnel + `Tween<Alignment>` (placement manuel) |
| [107](jalon-107.md) | Ancrage : listes virtualisées + `AlignmentDirectional` |
| [108](jalon-108.md) | `AlignmentGeometry` : l'ancrage unifié |
| [109](jalon-109.md) | Container : marge extérieure (`margin`) |
| [110](jalon-110.md) | `AspectRatio` : boîte à rapport largeur/hauteur |
| [111](jalon-111.md) | `FractionallySizedBox` : taille en fraction du parent |
| [112](jalon-112.md) | `Transform` : décalage de peinture (`translate`) |
| [113](jalon-113.md) | `Transform` : échelle de peinture (`scale`) |
| [114](jalon-114.md) | `Transform` : rotation (calque composité tourné) |
| [115](jalon-115.md) | `Transform` : échelle non uniforme (`scale_xy`) |
| [116](jalon-116.md) | `Transform` : composition (translate + échelle + rotation) |
| [117](jalon-117.md) | `Transform` : matrice affine unifiée |
| [118](jalon-118.md) | `Transform` : focus/a11y suivent l'échelle (cas aligné) |
| [119](jalon-119.md) | Vitrine animée : `frus-transforms` |
| [120](jalon-120.md) | Tests au pixel du pipeline de transformation |
| [121](jalon-121.md) | Découpe en forme : `ClipRRect` / `ClipOval` |
| [122](jalon-122.md) | `InteractiveViewer` : déplacer (pan) + zoomer (pinch/molette) |
| [123](jalon-123.md) | Vitrine enrichie : Clip + InteractiveViewer |
| [124](jalon-124.md) | `FittedBox` + `RotatedBox` : transformations qui affectent la mise en page |
| [125](jalon-125.md) | Découpe arrondie **par coin** (`ClipRRect` + `BorderRadius`) |
| [126](jalon-126.md) | `InteractiveViewer` : inertie (fling) + bornage du pan |
| [127](jalon-127.md) | `ClipPath` : découpe à un chemin arbitraire (pipeline de masque) |
| [128](jalon-128.md) | Vitrine : ClipPath + RotatedBox + FittedBox |
| [129](jalon-129.md) | Cible Web (wasm + WebGPU) |
| [130](jalon-130.md) | Effets & souscriptions au Web |
| [131](jalon-131.md) | Amincir le `.wasm` |
| [132](jalon-132.md) | Champ de formulaire décoré (label, indice, aide, erreur) |
| [133](jalon-133.md) | Champ mot de passe (masquage) + icônes préfixe/suffixe |
| [134](jalon-134.md) | Label flottant animé (façon Material) |
| [135](jalon-135.md) | Validation de formulaire groupée (pure, côté app) |
| [136](jalon-136.md) | Focus programmatique (rendre `first_invalid` actionnable) |
| [137](jalon-137.md) | Champ multi-lignes |
| [138](jalon-138.md) | Repli automatique du texte (word-wrap) |
| [139](jalon-139.md) | Défilement du champ multi-lignes (molette) |
| [140](jalon-140.md) | Barre de défilement du champ multi-lignes (+ tactile) |
| [141](jalon-141.md) | Flèches Haut/Bas dans le champ multi-lignes |
| [142](jalon-142.md) | Colonne cible mémorisée + Page préc./suiv. |
| [143](jalon-143.md) | Saut de mot (Ctrl+Flèches) & bornes de champ (Ctrl+Début/Fin) |
| [144](jalon-144.md) | Encoche du label (style `outlined`) |
| [145](jalon-145.md) | Tableau : en-tête triable & lignes sélectionnables |
| [146](jalon-146.md) | Sélecteur d'heure (`TimePicker`) |
| [147](jalon-147.md) | Flux date + heure, minutes fines & 12 h AM/PM |
| [148](jalon-148.md) | Tableau : sélection multiple & colonnes à largeur variable |
| [149](jalon-149.md) | Tableau : « tout cocher » indéterminé & tri au clavier |
| [150](jalon-150.md) | Audit `Dropdown` / `Autocomplete` : mise au niveau attendu |
| [151](jalon-151.md) | Tableau : redimensionnement de colonnes à la souris |
| [152](jalon-152.md) | Autocomplétion : mise en avant du texte & suggestion active |
| [153](jalon-153.md) | Tableau : réordonnancement des colonnes (glisser un en-tête) |
| [154](jalon-154.md) | Autocomplétion : liste de suggestions défilante |
| [155](jalon-155.md) | Réordonnancement des colonnes : aperçu glissant |
| [156](jalon-156.md) | Curseur de plage (deux poignées) |
| [157](jalon-157.md) | Curseur de plage : poignée collante & pas discret |
| [158](jalon-158.md) | Réordonnancement : fantôme fidèle (texte compris) |
| [159](jalon-159.md) | Réordonnancement : coulissement des colonnes voisines |
| [160](jalon-160.md) | Curseur de plage : infobulle de valeur & clavier |
| [161](jalon-161.md) | Réordonnancement : clavier & coulissement continu |
| [162](jalon-162.md) | Curseur de plage : survol, clic-piste & Début/Fin |
| [163](jalon-163.md) | Réordonnancement : inertie douce & en-têtes annoncés |
| [164](jalon-164.md) | Tableau : cellules-widgets (au-delà du texte) |
| [165](jalon-165.md) | Accessibilité : annonces vocales (région live) |
| [166](jalon-166.md) | Tableau : hauteur de rangée adaptative |
| [167](jalon-167.md) | Accessibilité : annonces de tri et de sélection |
| [168](jalon-168.md) | Tableau : en-têtes à icône (+ tri de colonnes-widgets) |
| [169](jalon-169.md) | Accessibilité : sélection de ligne annoncée |
| [170](jalon-170.md) | Tableau : widget d'action dans l'en-tête |
| [171](jalon-171.md) | Tableau : en-tête entièrement widget |
| [172](jalon-172.md) | Tableau : menu de colonne au clavier |
| [173](jalon-173.md) | Tableau : lignes virtualisées |
| [174](jalon-174.md) | Piège de focus des menus ouverts |
| [175](jalon-175.md) | Retour du focus à la fermeture d'un overlay |
| [176](jalon-176.md) | Tableau virtualisé : rangées-widgets |
| [177](jalon-177.md) | Tableau virtualisé : sélection multiple |
| [178](jalon-178.md) | Tableau : colonnes gelées |
| [179](jalon-179.md) | Colonnes gelées : ombre de séparation & gel à droite |
| [180](jalon-180.md) | Formulaires : validation croisée & récapitulatif d'erreurs |
| [181](jalon-181.md) | Formulaires : récapitulatif d'erreurs cliquable |
| [182](jalon-182.md) | Formulaire multi-étapes : indicateur `Steps` |
| [183](jalon-183.md) | Indicateur `Steps` : marqueurs cliquables |
| [184](jalon-184.md) | DatePicker : sélection d'une plage de dates |
| [185](jalon-185.md) | Snackbar : action + file d'attente |
| [186](jalon-186.md) | DatePicker : calendrier double (longues plages) |
| [187](jalon-187.md) | TimePicker : plage horaire (créneau début → fin) |
| [188](jalon-188.md) | ToastHost : positionnement, empilement, transition |
| [189](jalon-189.md) | DateTimeRange : plage date + heure |
| [190](jalon-190.md) | Assistant d'inscription intégré (démo bout en bout) |
| [191](jalon-191.md) | Button : état désactivé |
| [192](jalon-192.md) | Assistant : validation par étape, focus programmatique, mots de passe masqués |
| [193](jalon-193.md) | Snackbar : sortie animée + file branchée |
| [194](jalon-194.md) | Assistant : révéler le mot de passe |
| [195](jalon-195.md) | Steps : état « terminé » par validité |
| [196](jalon-196.md) | Table : édition en ligne des cellules |
| [197](jalon-197.md) | Grille éditable : câblage interactif |
| [198](jalon-198.md) | TextInput : suffixe cliquable (clic positionnel) |
| [199](jalon-199.md) | Charts : graphique à barres |
| [200](jalon-200.md) | Charts : graphique en lignes (LineChart) |
| [201](jalon-201.md) | Grille éditable : navigation clavier + lignes |
| [202](jalon-202.md) | Icône œil + révélation du mot de passe dans le champ |
| [203](jalon-203.md) | Charts : axe des ordonnées + grille (partagé) |
| [204](jalon-204.md) | Grille : tri par en-tête + validation par cellule |
| [205](jalon-205.md) | Curseur système par sous-région |
| [206](jalon-206.md) | Charts : aire remplie sous la courbe |
| [207](jalon-207.md) | Grille : soumission gardée par la validation |
| [208](jalon-208.md) | Surbrillance de sous-région au survol |
| [209](jalon-209.md) | Charts : séries multiples + légende |
| [210](jalon-210.md) | Grille : Save désactivé + accès à la première faute |
| [211](jalon-211.md) | Charts : infobulle de sous-région au survol |
| [212](jalon-212.md) | BarChart au niveau de LineChart : groupées + légende + infobulle |
| [213](jalon-213.md) | LineChart : aires empilées |
| [214](jalon-214.md) | Grille : cycle entre les fautes |
| [215](jalon-215.md) | Charts : légende cliquable + séries masquables |
| [216](jalon-216.md) | BarChart : barres empilées |
| [217](jalon-217.md) | Charts : halo pulsant animé au survol |
| [218](jalon-218.md) | Démo : écran « Charts » à légende cliquable |
| [219](jalon-219.md) | Démo Charts : sélecteur de type |
| [220](jalon-220.md) | Démo Charts : graphique compagnon partageant la visibilité |
| [221](jalon-221.md) | Clic sur un point de graphique → détail épinglé |
| [222](jalon-222.md) | Clic sur une barre : détail épinglé (BarChart::on_point) |
| [223](jalon-223.md) | Point/barre épinglé mis en évidence (halo + anneau persistants) |
| [224](jalon-224.md) | Empilage 100 % (proportions normalisées) |
| [225](jalon-225.md) | Désépinglage au re-clic |
| [226](jalon-226.md) | Pourcentages dans l'infobulle en mode 100 % |
| [227](jalon-227.md) | Libellé de part (%) dans chaque strate (barres 100 %) |
| [228](jalon-228.md) | Total au sommet des colonnes empilées absolues |
| [229](jalon-229.md) | Valeur dans chaque strate (barres empilées absolues) |
| [230](jalon-230.md) | Valeur/part dans chaque bande (aires empilées) |
| [231](jalon-231.md) | DatePicker borné (jours désactivés hors [min, max]) |
| [232](jalon-232.md) | DataTable auto-triant (widget réutilisable) |
| [233](jalon-233.md) | Pagination interne du DataTable |
| [234](jalon-234.md) | DatePicker plage bornée (range + fenêtre [min, max]) |
| [235](jalon-235.md) | Jours blackout / prédicat de sélection (DatePicker) |
| [236](jalon-236.md) | DataTable : taille de page + libellé « N–M of T » |
| [237](jalon-237.md) | Démo : écran Tableau de données (DataTable câblé) |
| [238](jalon-238.md) | Démo : calendrier filtré (week-ends grisés) |
| [239](jalon-239.md) | DataTable : ligne sélectionnée (traduction index source ↔ position affichée) |
| [240](jalon-240.md) | DataTable : clé de tri personnalisée par colonne |
| [241](jalon-241.md) | DataTable : sélection multiple (cases à cocher) |
| [242](jalon-242.md) | DataTable : recherche/filtre |
| [243](jalon-243.md) | DataTable : barre d'actions groupées |
| [244](jalon-244.md) | DataTable : état vide (« No results ») |
| [245](jalon-245.md) | Démo : confirmation avant suppression groupée |
| [247](jalon-247.md) | Kanban : colonnes + cartes, glisser-déposer inter-colonnes |
| [248](jalon-248.md) | Kanban : aperçu de dépôt vertical |
| [249](jalon-249.md) | Kanban : cartes riches + ajout/suppression |
| [250](jalon-250.md) | Registre des réordonnables (glisser des cartes fonctionnel) |
| [251](jalon-251.md) | Fantôme de glisser incluant le contenu d'une carte riche |
| [252](jalon-252.md) | Indicateur d'insertion inter-cartes (moitié survolée) |
| [253](jalon-253.md) | Décalage des cartes voisines à l'insertion verticale (le « trou ») |
| [254](jalon-254.md) | Revue transverse du glisser-déposer : correctifs Table + Kanban |
| [255](jalon-255.md) | Peinture du glisser-déposer portée sur le thème / constantes nommées |
| [256](jalon-256.md) | Consolidation : registres transformés (ui.rs) + facteur de réagencement partagé |
| [257](jalon-257.md) | Correctif clavier Android : rouvrir le clavier au ré-appui d'un champ |
| [258](jalon-258.md) | Respect du viewport : board Kanban défilable + texte enroulé (fin du débordement) |
| [259](jalon-259.md) | Contrat de cycle de vie de l'application |
| [260](jalon-260.md) | Défilement Kanban : axe horizontal intentionnel (fin du pan 2D) |
| [261](jalon-261.md) | Finitions DnD : ombres `Card`/`Toast` thémées + test de réagencement même-colonne |
| [262](jalon-262.md) | Balayage overflow des écrans (tables défilables + textes enroulés + corps verticaux) |
| [263](jalon-263.md) | Défilement vertical par colonne : blocage layout + garde-fou réordonnables-dans-Scroll |
| [264](jalon-264.md) | Défilement vertical par colonne (façon Trello), via hauteur explicite |
| [265](jalon-265.md) | Inertie verticale du glisser (ligne d'insertion à ressort) |
| [266](jalon-266.md) | Fill-then-scroll : défilement vertical par colonne **sans hauteur explicite** |
| [267](jalon-267.md) | Point d'entrée **unique**, une entrée par plateforme |
| [268](jalon-268.md) | Crate-façade `frus` : **une seule dépendance** |
| [269](jalon-269.md) | `compute_scroll` **remplit l'axe contraint** (fin du conteneur remplisseur) |
| [270](jalon-270.md) | Effets **asynchrones** (`perform_async` / `run_async`) |
| [271](jalon-271.md) | Helper `fetch` cross-plateforme (feature `net`) |
| [272](jalon-272.md) | `Request` : POST, en-têtes et timeout sur `fetch` (feature `net`) |
| [273](jalon-273.md) | Exemple réseau de bout en bout (`frus-fetch-example`) |
| [274](jalon-274.md) | `RemoteData<T, E>` : l'idiome Elm pour une donnée asynchrone |
| [275](jalon-275.md) | JSON typé sur `Request` (feature `json`) |
