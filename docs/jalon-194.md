# Jalon 194 — Assistant : révéler le mot de passe

## Analyse

Les champs mot de passe de l'assistant (jalon 192) étaient toujours masqués : impossible de
**vérifier** ce qu'on tape — source d'erreurs et de frustration. Il manquait la bascule
« afficher / masquer » classique.

## Décisions techniques

- **Composition, pas de nouveau mécanisme.** `TextInput::obscure(bool)` existait déjà (jalon
  antérieur) ; il suffit de piloter son argument par un état applicatif `wizard_reveal` et
  d'ajouter un bouton bascule. Une seule bascule révèle **les deux** champs de l'étape Security
  (mot de passe + confirmation), cohérent (on compare deux saisies).

- **Bascule texte plutôt qu'icône œil.** Le jeu d'icônes est *rempli* (sans contour) : un « œil »
  reconnaissable y est coûteux, et une icône **dans** le champ demanderait un routage de clic
  positionnel côté shell (le trait `Widget` n'expose pas de clic positionnel). Un bouton
  « Show password » / « Hide password » sous les champs est clair, offre une grande cible, et
  reste 100 % composable.

## Implémentation

- `frus-demo/src/lib.rs` : état `wizard_reveal` ; `Msg::WizardToggleReveal` (+ arm `reduce`) ;
  l'étape Security passe `obscure = !wizard_reveal` aux deux champs et ajoute le bouton bascule.
- `goldens.rs` : `wizard_password_revealed` (mots de passe visibles + « Hide password »).

## Vérification

- **Golden** `wizard_password_revealed` **inspecté** : « secret12 » lisible dans les deux champs,
  bouton « Hide password ». (L'état masqué reste couvert par `wizard_password_step`, jalon 192.)
- Les 18 tests démo restent **verts** ; `cargo build -p frus-demo` **propre**.

## Reste

- **Icône œil dans le champ** (`suffix_icon` cliquable) : demande une icône œil (contour) et un
  routage de clic positionnel du suffixe côté shell — extension framework distincte.
