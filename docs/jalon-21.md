# Jalon 21 — Séparation framework / application (`run(app)`)

Extraction d'une **API d'hébergement** : `frus-shell` devient un framework pur,
générique sur un trait [`Application`] ; l'app todo devient un **consommateur
externe** vivant dans `frus-demo`. Motivé par J20, qui avait rendu le couplage
concret (l'app était codée *dans* le shell).

## Le trait

```rust
pub trait Application {
    type Message: Clone;
    fn update(&mut self, message: Self::Message);
    fn view(&self, theme: &Theme, w: f32, h: f32) -> Box<dyn Widget<Self::Message>>;
    fn theme(&self) -> Theme { Theme::dark() }      // défaut
    fn tick(&mut self, _dt: f32) -> bool { false }  // anims propres à l'app
    fn title(&self) -> String { "frus".into() }
    fn can_go_back(&self) -> bool { false }          // active le geste retour
    fn back_gesture(&mut self, _progress: f32) {}
    fn back_gesture_end(&mut self, _velocity: f32) {}
}

frus_shell::run(MyApp::default())?; // ouvre la fenêtre et pilote la boucle
```

Une app minimale n'implémente que `update` + `view` (le reste a des défauts).

## Partage des responsabilités

| Framework (`frus-shell`) | Application (consommateur) |
|---|---|
| Fenêtre, renderer, boucle d'événements | État (`State`), `update`, `view` |
| `Runtime` (survol/focus/scroll/édition/anims) | Thème + fondu (`theme`) |
| Hit-test, routage clic/clavier, presse-papier | Transitions d'écran, pile de routes |
| Glissement (barres, sélection, poignées) | Détente des animations (`tick`) |
| **Mesure** du geste retour (bord, progression, vélocité) | **Décision** du geste (`can_go_back`, `back_gesture*`) |
| Fondus montage/démontage | — |

Point clé — le **geste retour** : le framework mesure (zone de bord `BACK_EDGE`,
progression, vélocité lissée) et appelle des *hooks* ; l'app décide (préview via
`view`, validation/annulation par projection de la vélocité, détente à ressort
dans `tick`). Le framework reste **ignorant des `Route`**.

## Util partagé

`frus_widgets::spring_step(p, v, target, dt, K, C) -> (p, v, au_repos)` : un pas
de ressort amorti réutilisable (transitions d'écran, gestes). Les constantes de
raideur/amortissement et de projection sont **politique de l'app**.

## Arborescence

- `frus-shell` : `application.rs` (trait) + `app.rs` (`App<A>` générique) +
  `run<A>(app)`. **Zéro code métier.**
- `frus-widgets` : `spring_step` public.
- `frus-demo` : `TodoApp: Application` (tout l'ex-code démo y migre) ; dépend
  désormais de `frus-widgets`.

## Tests

- Migrés dans `frus-demo` : add/trim, toggle/delete/clear, rendu non vide, et un
  nouveau **`back_gesture_flick_commits_pop`** (le flick rapide valide le retour,
  piloté sans souris via les hooks + `tick`).
- 30 tests `frus-widgets` + 4 `frus-demo` + doctest de `run`.

## Ce que ça débloque

- Écrire une app frus **sans toucher au framework**.
- Plusieurs apps/exemples possibles côte à côte.
- Base saine pour les prochains jalons (barre de nav, inertie de défilement).

## Limites (v1)

- `view` reconstruit tout l'arbre à chaque frame (pas de diff/mémoïsation).
- Pas encore de sous-commandes/effets (pas de `Command`/async depuis `update`).
- Navigation encore « à la main » dans l'app (pile de routes) — un routeur
  first-class reste un candidat futur.
