# Jalon 80 — Clavier logiciel Android (ouverture du chantier saisie §6)

## Analyse

Constat on-device (session J74+) : le clavier logiciel ne s'ouvre jamais, et
la saisie ne semble pas atteindre les champs. Enquête dans les sources :

- **winit 0.30 Android mappe déjà les caractères** : chaque `KeyEvent` passe
  par le `KeyCharacterMap` du périphérique (JNI, avec combinaison des touches
  mortes) → `Key::Character` — le chemin d'édition existant du shell
  (desktop) est donc **déjà branché** pour les événements clavier Android.
  L'échec observé venait vraisemblablement de l'absence de focus (le champ
  n'avait jamais été touché) — à confirmer sur device.
- **Le clavier ne s'ouvre pas** car personne ne le demande : NativeActivity
  n'a pas d'`InputConnection`, il faut appeler `InputMethodManager` — exposé
  par `android-activity` via `AndroidApp::show_soft_input`/`hide_soft_input`
  (l'approche egui/game-activity).

## Implémentation

`frus-shell` : `sync_soft_input()` appelée en fin de frame (tout changement de
focus redessine déjà) — le clavier est **demandé quand le focus est dans un
champ texte** (`cursor_at` → `Some`, le même critère que les flèches), refermé
sinon (blur, Escape, retour, fermeture de modale…). Transitions dédupliquées
(`soft_input_shown`). No-op compilé hors Android.

## Limites connues (la suite du chantier §6)

- Mode `TYPE_NULL` : sans `InputConnection`, l'IME envoie des **key events**
  — suffisant pour du texte latin (Gboard le gère), mais pas de composition
  (suggestions, swipe, voix, émojis riches, CJK). Le palier suivant est un
  `InputConnection` JNI + événements de composition (le vrai FFI §6).
- L'évitement du clavier (J74, `view_insets`) se validera dans la foulée.

## Validation on-device (STK-L21, IME SwiftKey) — et deux correctifs

- Tap sur le champ → **le clavier monte** et le contenu **s'écarte au-dessus**
  (première preuve réelle de l'évitement clavier du J74). Blur → refermé.
- Deux bugs réels débusqués par la saisie injectée (`adb input text`) :
  1. **Rafales de touches** : des frappes plus rapprochées qu'une frame
     s'appliquaient sur l'arbre **retenu** (valeur du champ en retard d'une
     frame) — chaque frappe écrasait la précédente (« Hello » → « o »).
     Fix : `apply_key` rafraîchit l'arbre (`view`) sitôt le message d'édition
     dispatché ; `build_dirty` reste levé pour la passe complète suivante.
  2. **Entrée Android** arrive en `Character("\n")` (KeyCharacterMap), pas en
     `Named(Enter)` → elle s'insérait comme texte au lieu de soumettre.
     Fix : `"\n"`/`"\r"` mappés sur `Key::Enter` (répétition non re-soumise).
- Après correctifs : « World » arrive entier (majuscule via méta Maj incluse),
  Entrée ajoute la tâche et vide le champ. ✔
- Artefacts d'injection (pas des bugs frus) : clavier **ouvert**, SwiftKey
  consomme une partie des événements injectés (connexion nulle) — la vraie
  saisie utilisateur passe par l'IME lui-même ; et `adb input text` s'arrête
  au premier espace (utiliser `%s`).
