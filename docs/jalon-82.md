# Jalon 82 — Saisie IME palier 3 : composition stylée + contexte des suggestions

## Analyse

Le palier 2 (J81) fournissait une `InputConnection` réelle mais avec deux
limites : le texte en cours de composition n'était pas distingué (pas de
soulignement) et `getTextBeforeCursor` renvoyait une `Editable` locale vide,
donc les IME **désactivaient** composition et suggestions (SwiftKey commitait
caractère par caractère). Objectif : parité Flutter — région de composition
soulignée + suggestions/prédictions pertinentes.

## Architecture

### Région de composition (rendu)
- `Edit.composing: Option<(usize, usize)>` (plage en caractères) →
  `Status.composing` (propagé dans `full_status`) → `TextInput` **souligne**
  cette plage (rectangles fins de 1,5 px sous le texte, `selection_rects`
  réutilisés pour l'étendue).
- Shell (`drain_ime`) : sur `Composing(text)`, remplace la région précédente
  et enregistre la nouvelle plage `[curseur_avant, curseur_avant+n]` dans
  l'`Edit` ; `FinishComposing`/`Commit`/`clear_composing` la remettent à
  `None`.

### Contexte de saisie (suggestions)
- `Widget::text_value() -> Option<&str>` (implémenté par `TextInput`,
  délégué par `Box`/`Keyed`/`Responsive`) : la valeur du champ.
- `android_ime` : `EditorState` partagé (`Mutex`) = texte + curseur +
  sélection ; le shell le pousse (`push_ime_context`) après chaque édition IME
  et à l'ouverture du clavier ; l'efface au blur.
- Trois natives (`nativeTextBeforeCursor`/`After`/`SelectedText`) lisent cet
  état ; la `Connection` Java surcharge `getTextBeforeCursor`,
  `getTextAfterCursor`, `getSelectedText` — et surtout **`getExtractedText`**
  (état complet du champ) : c'est ce qui décide SwiftKey à **composer** plutôt
  qu'à commiter au caractère. EditorInfo : `TYPE_CLASS_TEXT |
  CAP_SENTENCES` (plus de `NO_SUGGESTIONS`).

## Validé sur device (STK-L21, SwiftKey)

Journal IME à l'appui : `Composing("H") → ("He") → ("Hel")` (composition
active, avant : `Commit` par caractère). À l'écran :
- **« Hel » souligné** dans le champ (région de composition) ;
- **barre de suggestions** « Hel | Help | Hello » (le contexte alimente les
  prédictions) ;
- tap sur « Hello » → `Commit("Hello")` + `Commit(" ")`, soulignement effacé,
  puis **prédictions du mot suivant** « how | bro | chef » (contexte continu) ;
- auto-capitalisation du premier caractère (le contexte « début de phrase »
  remonte bien).

## Tests (281 → 283)

- `composing_region_draws_an_underline` (rendu : la plage composée ajoute des
  rectangles fins absents sinon) ; `text_value_exposes_the_field_content`
  (contexte). Modèle : champ `composing` sur `Edit`/`Status` (Copy préservé).

## Limites (au-delà)

- La composition est matérialisée **dans la valeur** (le champ contrôlé n'a
  pas de buffer de composition séparé) : correct pour l'IME, mais un
  `setComposingRegion` sur du texte déjà validé retomberait sur un remplacement
  simple.
- `getExtractedText` renvoie un instantané (pas de `partial` incrémental) —
  suffisant pour un champ mono-ligne.
