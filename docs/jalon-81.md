# Jalon 81 — Pont InputConnection Android (§6, palier 2)

## Analyse

Le palier 1 (J80) ouvrait le clavier mais restait en mode `TYPE_NULL` :
NativeActivity ne fournit aucune `InputConnection`, donc les IME n'envoient
que des touches latines — ni composition, ni swipe, ni suggestions, ni CJK,
et certains (SwiftKey) se comportent mal face à une connexion nulle.
L'embedding **Flutter** résout ça avec une View Java offrant une vraie
`InputConnection` reliée au moteur ; on suit exactement ce pas — **sans
Gradle**.

## Architecture

1. **`FrusTextBridge.java`** : une View 1×1 focalisable ajoutée par-dessus le
   contenu natif (`addContentView`), dont `onCreateInputConnection` retourne
   une `BaseInputConnection` réelle. Chaque opération de l'IME — `commitText`,
   `setComposingText`, `finishComposingText`, `deleteSurroundingText`,
   `performEditorAction`, `sendKeyEvent` — est relayée à des méthodes
   `native*`.
2. **Dex embarqué** : compilé une fois (`scripts/build-input-dex.sh`, javac +
   d8) et versionné (`frus-shell/assets/frus_input.dex`, ~5,4 Ko) ; chargé à
   l'exécution via `InMemoryDexClassLoader` + `RegisterNatives` (crate `jni`).
   **Aucun changement de packaging** — cargo-apk n'a jamais besoin de javac.
3. **File + réveil** : les natives (thread UI Java) poussent des `ImeEvent`
   dans une file partagée et réveillent la boucle winit (`AndroidAppWaker`) ;
   le shell draine dans `new_events` et applique au champ focalisé via
   `apply_key` (commit → `Text`, composition → remplacement de la région
   courante, action/`\n` → `Enter`, suppressions → `Backspace`/`Delete`).
4. **Bascule** : `sync_soft_input` passe par le pont quand il est installé
   (`startInput`/`stopInput` : focus Java sur la view pont + IMM), sinon repli
   `TYPE_NULL` du J80. Quand le pont est actif, le chemin d'édition clavier de
   winit est **coupé** (sinon chaque touche matérielle arriverait en double :
   file native + view pont).

## Le bug débusqué en route — l'identité positionnelle, en vrai

Symptôme utilisateur : « le clavier part et revient en boucle ». Diagnostic
(logs de transition) : l'arbre alternait **54 ↔ 53 widgets** — clavier ouvert
→ écran court → la bannière *Tip* (conditionnelle) démonte → tous les ids
positionnels des frères glissent → l'id focalisé résout vers le
`SegmentedControl` → le shell croit le focus sorti d'un champ → referme le
clavier → l'écran regrandit → la Tip remonte → re-focus champ → réouverture…
**C'est la classe de bugs prédite au §2** (« réordonner/conditionner perd
l'état ») — corrigée par le remède canonique : des **clés** (`keyed(...)`) sur
les enfants voisins de la bannière conditionnelle. Le focus survit désormais
au montage/démontage de la Tip.

## Validé sur device (STK-L21, SwiftKey)

- `pont de saisie installé (InputConnection réelle)` au boot ; ouverture
  **explicite** du clavier (`mShowExplicitlyRequested=true`), stable (une
  seule transition en 8 s — la boucle est morte).
- **Frappe réelle sur le clavier tactile** (touches SwiftKey) → `commitText`
  → le texte apparaît dans le champ ; **Entrée bleue** de l'IME
  (`performEditorAction`) → soumission, tâche ajoutée, champ vidé. Vérifié en
  double : l'utilisateur a tapé « This is » à la main pendant la session.

## Limites (palier 3 éventuel)

- La composition est matérialisée dans le champ **sans style** (pas de
  soulignement de la région composée) et `getTextBeforeCursor` renvoie un
  contexte vide (suggestions moins pertinentes). Le vrai palier 3 : synchroniser
  l'état d'édition complet vers la connexion (comme Flutter).
- `deleteSurroundingText` est appliqué autour du curseur sans tenir compte
  d'une région de composition non contiguë.
