# Jalon 74 — Insets fenêtre : split `padding` / `viewInsets` (évitement clavier)

Le dernier gros item du §6 côté plateforme : distinguer les insets **statiques**
(barres système, encoche) des **dynamiques** (clavier logiciel) — la condition de
l'évitement du clavier sur Android.

## `WindowInsets { padding, view_insets }` (frus-core)

- **`padding`** : zones occupées en permanence par le système.
- **`view_insets`** : zones couvertes par une UI transitoire — convention
  `MediaQuery` de Flutter : `view_insets.bottom` mesure l'occultation **totale
  depuis le bord** (barre comprise), ce qui rend la combinaison par `max`
  correcte. *(Ma première version mesurait le seul excédent clavier — le test de
  zone sûre l'a attrapée : `max(45, 300) = 300` au lieu des 345 réels.)*
- **`safe()`** : la zone à éviter au total — le max côté à côté (le clavier
  recouvre la barre, on n'additionne pas).
- **`from_baseline(référence, courant)`** : la séparation à partir d'une
  référence *sans clavier* (l'excédent bas signale le clavier).

## La référence côté shell (heuristique, auto-corrective)

Android ne livre qu'un `content_rect` brut. Le shell prend comme **référence sans
clavier** la première mesure pour la taille physique courante (une rotation la
réinitialise), et **se corrige vers le bas** si un état plus « nu » apparaît —
couvre le clavier-ouvert-au-démarrage et les barres masquées. Bureau : zéro
partout, inchangé. *(Heuristique documentée, en attendant de vrais insets IME
par FFI — item « channels typés » du §6.)*

## API & adoption

- `Application::on_insets(WindowInsets)` (signature formalisée).
- **Démo** : la zone sûre racine devient `insets.safe()` — quand le clavier
  s'ouvre, tout le contenu (champs de saisie compris) **remonte au-dessus** :
  l'évitement du clavier, au niveau app.

## Validation

- **251 tests**, tout vert — le test cœur épingle : clavier fermé (tout en
  padding), ouvert (`view_insets.bottom` = occultation totale, zone sûre = 345),
  barres masquées (pas de clavier négatif) ; le test démo épingle l'évitement
  (zone sûre du bas qui suit le clavier).
- Build sans avertissement ; démo sans panique. **À valider sur l'appareil
  Android** (le clavier n'existe pas sous WSL) : ouvrir un champ → le contenu
  remonte.

## Non couvert (assumé)

- La règle **consume-then-zero** d'un widget `SafeArea` imbriqué : la démo gère
  les insets à la racine (pas encore de widget SafeArea) — viendra avec le
  contexte ambiant (`Env`) du §2.
- Vrais insets IME via FFI (channels typés Android, §6).

## Suite (§6 restant)

Modèle clavier régularisé (physical + logical + character) — le dernier item
§6 praticable sans FFI Android.
