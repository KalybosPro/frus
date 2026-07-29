# Jalon 192 — Assistant : validation par étape, focus programmatique, mots de passe masqués

## Analyse

L'assistant (jalon 190) laissait passer « Next » sur une étape invalide, affichait les mots de
passe **en clair**, et ses puces d'erreurs ne faisaient que **changer d'étape** sans amener au
champ fautif. Trois manques d'ergonomie que le framework savait déjà couvrir — il fallait les
**câbler**.

## Décisions techniques

- **« Next » gouverné par la validité de l'étape.** `wizard_step_valid(form, step)` interroge le
  `Form` (pur) : Account valide si `name`+`email` passent, Security si `password`+`confirm`
  passent. Le bouton « Next » reçoit `.enabled(...)` (jalon 191) → **grisé et inerte** tant que
  l'étape courante n'est pas remplie. La validité reste une **fonction pure de l'état**, pas un
  drapeau à maintenir.

- **Mots de passe masqués.** Les champs Security passent `.obscure(true)` (déjà offert par
  `TextInput`) : l'affichage devient des points, la valeur éditée reste réelle.

- **Focus programmatique par clé.** Chaque champ est enveloppé dans `keyed(("wizard", i), …)` ;
  cliquer une puce du récapitulatif émet `WizardFocus(étape, champ)` qui **saute à l'étape** puis
  renvoie `Command::focus(("wizard", champ))`. Le shell résout la clé contre l'arbre
  (`keyed`/`Command::focus` hachent la clé à l'identique) et pose le curseur **dans le champ** —
  plus seulement la bonne étape. Aucun mécanisme nouveau : le framework savait déjà focaliser par
  clé.

## Implémentation

- `frus-demo/src/lib.rs` : `Msg::WizardFocus(usize, u8)` (+ arm `reduce` → `Command::focus`) ;
  `wizard_field_of` / `wizard_step_valid` ; `wizard_input` gagne `obscure` et l'enveloppe `keyed` ;
  « Next » `.enabled(wizard_step_valid(...))` ; puces du récapitulatif → `WizardFocus`.
- `goldens.rs` : `wizard_password_step` (étape Security : mots de passe masqués + « Next »
  désactivé).

## Vérification

- **Intégration** (test `wizard_flow_*` étendu) : Account invalide au départ ; `WizardFocus`
  saute à l'étape **et** émet une demande de focus (`!cmd.is_empty()`) ; l'étape passe valide une
  fois remplie. Les 17 tests démo restent **verts**.
- **Golden** `wizard_password_step` **inspecté** : `Steps` (Security), deux champs en points,
  « Back » actif à côté de « Next » grisé.
- `cargo build -p frus-demo` **propre**.

## Reste

- **Marquer les étapes `Steps` par validité** (et pas seulement par position) — cohérent
  maintenant que « Next » est gardé, mais le saut libre via `Steps::on_tap` peut désynchroniser.
- **Révéler le mot de passe** (icône œil `suffix_icon` bascule `obscure`) — petite UX en plus.
