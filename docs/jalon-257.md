# Jalon 257 — Correctif clavier Android : rouvrir le clavier au ré-appui d'un champ

## Analyse

Bug observé **sur appareil** (Huawei STK-L21) : on tape dans un champ → le clavier logiciel sort ; on
appuie sur le **bouton retour système** → le clavier disparaît ; on retape dans **le même champ** → le
clavier **ne sort plus**.

Cause : le shell pilote le clavier via `sync_soft_input`, qui n'agit **que** sur un **changement**
(`editing != self.soft_input_shown`). Séquence fautive :
1. Appui sur le champ → `editing = true`, `soft_input_shown` passe à `true`, clavier montré.
2. **Retour système** → Android ferme l'IME, mais **l'app n'en est pas notifiée** : `soft_input_shown`
   reste `true` et le **focus ne change pas** (le champ reste focalisé).
3. Nouvel appui sur le champ → focus inchangé, `editing` toujours `true`, `soft_input_shown` toujours
   `true` → le diff ne voit **aucun changement** → le clavier n'est **jamais redemandé**.

## Décision technique

Comportement natif : **taper dans un champ montre le clavier** — inconditionnellement, pas seulement
sur transition. On ajoute `request_soft_input()`, qui **redemande** l'IME pour le champ focalisé
(`start_input` via le pont InputConnection, ou repli `show_soft_input(true)`), appelé quand
l'utilisateur **tape dans un champ texte** (création d'un `Drag::TextSelect`). Indépendant du diff de
`sync_soft_input` : couvre le cas « fermé par le retour système sans notification ».

## Implémentation

- `frus-shell/src/app.rs` : nouvelle méthode `request_soft_input` (corps `#[cfg(target_os = "android")]`) ;
  appel depuis `pointer_down` juste après avoir armé `Drag::TextSelect` (appui **dans** un champ).

## Vérification

- **Desktop** : compile ; shell 27 (la méthode est un no-op hors Android, aucune régression).
- **Appareil** (Huawei STK-L21) : **confirmé** — appui → clavier ; retour système → clavier caché ;
  **ré-appui → le clavier revient**. Séquence auparavant cassée, désormais correcte.

## Notes

- La mémoire projet indiquait « no soft-keyboard/IME » : **périmée** — l'intégration IME
  (`android_ime.rs` : commit/composing/delete/key) et la gestion des **insets** clavier
  (`WindowInsets::from_baseline`, `on_insets`, `reveal_caret`) existent. Ce jalon corrige un défaut
  **ponctuel** de cette intégration, pas une absence.
- Piste : détecter la fermeture **externe** de l'IME (retour système) pour remettre `soft_input_shown`
  à `false` — ceinture et bretelles, si d'autres chemins réouvrent un champ sans passer par l'appui.

## Reste (report des jalons précédents)

- Couverture du réagencement **même-colonne** (chevauchement source/cible → décalage net nul).
- Inertie/ressort **vertical** du coulissement (parité avec l'horizontal).
- Unifier l'ombre de `Card`/`Toast` sur `theme.scheme.shadow`.
