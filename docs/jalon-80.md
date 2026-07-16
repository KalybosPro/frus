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

## Validation

- Workspace : 18 suites vertes ; APK construit. **Validation on-device en
  attente** (téléphone débranché au moment du jalon) : tap sur le champ →
  clavier ouvert (`dumpsys input_method` : `mInputShown=true`), saisie
  `adb input text` → texte dans le champ, Entrée → tâche ajoutée, blur →
  clavier refermé.
