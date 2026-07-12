# Jalon 50 — Premier run sur Android physique

frus tourne désormais sur un **téléphone Android réel** (Huawei STK-L21, arm64,
Android 10, GPU Mali-G51) : rendu Vulkan, tactile (tap + défilement au doigt),
navigation, et toute la bibliothèque de widgets. Même code que le bureau ; seule
la **couche shell** gagne un point d'entrée et une gestion du cycle de vie
spécifiques à Android.

## Ce qu'il a fallu ajouter

### 1. Point d'entrée `android_main`
Android n'appelle pas `main()`. La démo devient une **bibliothèque `cdylib`**
(le `.so` chargé par l'activité native) exposant `android_main(app: AndroidApp)`,
en plus d'un binaire bureau (`src/bin/frus-demo.rs`). Les deux appellent le même
code :
- bureau → `frus_demo::run_desktop()` → `frus_shell::run` ;
- Android → `android_main` → `frus_shell::run_android(app, android_app)`.

`frus-shell` gagne `run_android` (boucle winit construite avec
`.with_android_app(...)`, logs vers logcat via `android_logger`). `winit` reçoit
la feature `android-native-activity` uniquement sur la cible Android. `arboard`
(presse-papier) et `env_logger` deviennent **desktop-only** (ne compilent pas
pour Android) ; le presse-papier est neutralisé par un petit wrapper `clip`.

### 2. Entrées tactiles
Le pilote ne gérait que la souris. Les bras souris sont factorisés en helpers
`pointer_down / pointer_move / pointer_up`, que `WindowEvent::Touch` réutilise
(un doigt = un pointeur). En plus, une variante de glissement `Drag::Scroll`
implémente le **défilement au doigt** : sous `TOUCH_SLOP` (8 px) le geste reste
un tap ; au-delà, il défile la zone scrollable sous le doigt.

### 3. Cycle de vie de la surface
Android **détruit la surface** en arrière-plan. Ajout de `suspended` (relâche
renderer + fenêtre) ; `resumed` les recrée. L'effet de démarrage `init` n'est
joué qu'une fois (drapeau `started`), pas à chaque retour au premier plan.

### 4. Limites GPU réelles (correctif transverse)
`downlevel_defaults()` plafonne la texture max à **2048**, alors que l'écran fait
1080×2340 → `surface.configure` paniquait. Le renderer demande désormais
`downlevel_defaults().using_resolution(adapter.limits())` : compat downlevel mais
résolution réelle de l'adaptateur. (Bénéfique aussi sur desktop haute résolution.)

### 5. Police embarquée (correctif transverse)
`FontSystem::new()` s'appuie sur les polices système ; sur Android, fontdb ne lit
pas `fonts.xml`, donc l'alias « sans-serif » ne résout **aucune** police par
défaut → panic « no default font found » de cosmic-text. `frus-text` embarque
désormais **DejaVu Sans / Sans Mono** (`include_bytes!`) et expose
`new_font_system()` : polices système (repli emoji/scripts) **plus** la police
embarquée fixée comme famille par défaut. `frus-gpu` (rendu glyphon) réutilise ce
même `FontSystem` → rendu texte déterministe sur toute plateforme, façon Flutter.

## Outillage (WSL) & workflow appareil
- Build **dans WSL** (cross-compilation, build scripts Linux — le natif Windows
  reste bloqué par Smart App Control). SDK/NDK (r26d) + `cargo-apk` installés dans
  WSL ; cible `aarch64-linux-android`.
- `cargo apk build -p frus-demo --lib` → APK signé (clé debug auto).
- **Installation via l'`adb.exe` de Windows** (téléphone branché côté Windows,
  aucun usbipd) : copier l'APK sur un chemin `/mnt/...`, puis
  `adb.exe install -r` + `adb.exe shell am start -n com.frus.demo/android.app.NativeActivity`.
- Métadonnées cargo-apk dans `frus-demo/Cargo.toml` (`package = com.frus.demo`,
  `min_sdk 24`, `target_sdk 34`, `build_targets = ["aarch64-linux-android"]`).

## Validation sur appareil
- Rendu : Tasks, Log (liste virtuelle 5000 lignes), Settings (Switch, Slider,
  RadioGroup, Dropdown, Rating, Stepper, DatePicker, Tabs, Breadcrumb, Card).
- Tap → navigation ; swipe → défilement (Row 1 → Row 11) ; chrono qui tourne ;
  aucun crash. Backend Vulkan (Mali-G51).
- Non-régression bureau : workspace bâti, **162 tests** verts, démo lancée.

## Limites (v1)
- Pas d'IME / clavier logiciel : les champs texte reçoivent le focus mais la
  saisie soft-keyboard n'est pas encore branchée.
- L'en-tête de la démo se chevauche en très petite largeur (raffinement de mise
  en page côté démo, pas un défaut du framework).
- Un seul ABI empaqueté (`arm64-v8a`) ; pas d'`armeabi-v7a`/`x86_64`.
- Pas encore de tuiles d'inset système (barre d'état / gestes) : l'UI s'étend
  sous la barre d'état.
