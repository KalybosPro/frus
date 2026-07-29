# Jalon 195 — Steps : état « terminé » par validité

## Analyse

`Steps` (jalons 182–183) marquait une étape « terminée » (coche) **uniquement par position**
(`i < current`). Or dans un assistant gardé (jalon 192, « Next » bloqué tant qu'une étape est
invalide), une étape *passée* peut redevenir **invalide** (retour en arrière + saut libre via
`on_tap`) : l'indicateur mentait alors en la montrant cochée. Il fallait marquer les étapes par
**validité**, pas par position.

## Décisions techniques

- **Un masque « terminé » explicite, optionnel.** `Steps::completed([bool, …])` fixe, par étape,
  si elle est terminée — typiquement la validité calculée par le `Form`. Sans cet appel, on garde
  la règle par défaut `i < current` : **tous les usages existants et leurs goldens sont
  inchangés** (rétrocompatible).

- **Un seul point de décision.** Toute la peinture (coche vs numéro, connecteur franchi ou non)
  passe par `is_done(i)` : masque s'il est fourni, `i < current` sinon. L'**étape courante**
  affiche toujours son **numéro** (même si valide) — on ne coche que les *autres* étapes
  terminées, comme le `Stepper` de Material.

## Implémentation

- `steps.rs` : champ `completed` (+ builder `completed`) ; `is_done` (dans le bloc `impl<Msg>`
  sans borne, appelé depuis `paint`) ; connecteur et marqueur utilisent `is_done` au lieu de
  `i < current`.
- `frus-demo/src/lib.rs` : l'assistant passe `.completed([valide_0, valide_1, tout_valide])`
  (mêmes prédicats que le garde « Next »).

## Vérification

- **Unitaire** : `completed_mask_overrides_position` — sans masque, `is_done = i < current` ;
  avec masque, indépendant de la position (étape 0 invalide non cochée bien que `i < current`,
  étape 2 valide cochée bien que `i > current`) ; masque plus court → manquants non terminés.
  Les tests des jalons 182–183 restent **verts**.
- **Golden** `wizard_password_revealed` (jalon 194) **inspecté** : l'étape Account apparaît
  **cochée** via `completed`, l'étape courante (Security) en numéro. Les goldens `form_wizard` /
  `wizard_*` (sans `completed`) sont **inchangés**.

## Reste

- **Verrouiller le saut vers une étape non atteinte** (au-delà du marquage visuel) — le garde
  « Next » couvre l'avance séquentielle, mais `on_tap` autorise encore le saut libre.
