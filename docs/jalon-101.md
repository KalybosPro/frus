# Jalon 101 — Animations explicites : `repeat` / `stop` / `reset`

## Analyse

Contrairement aux animations **implicites** (J95→100 : le framework interpole
seul vers une cible déclarée), une animation **explicite** est **pilotée par
l'app** : elle décide quand démarrer, inverser, répéter, arrêter.

L'infrastructure était **déjà en place** :

- [`AnimationController`] (frus-core) — valeur bornée `[lower, upper]`, `value()`/
  `velocity()`/`status()`, `forward`/`reverse`/`animate_to`/`fling`/`spring_to`/
  `drive`, et `tick(dt) -> bool`. Exporté publiquement.
- Le **hook de frame** : `Application::tick(&mut self, dt) -> bool`, appelé par le
  shell **à chaque frame** ; tant qu'il renvoie `true`, le shell redemande une
  frame. C'est le « ticker » de l'Elm/iced.
- La **démo l'utilise déjà** (transition de navigation, détente de geste).

Le motif app-owned est donc complet : l'app détient un contrôleur dans son état,
l'avance dans `tick()`, lit `value()` dans `view()` pour piloter un widget.

**Le manque** : la **répétition** (boucle). Flutter a
`AnimationController.repeat(reverse:)` — omniprésent (pulsation, halo, indicateur
piloté). Le contrôleur de frus ne pouvait que jouer un cycle puis se reposer.

## Décisions techniques

- **`repeat(period, reverse, curve)`.** Chaque cycle dure `period`, façonné par
  `curve`. `reverse` = aller-retour (`0→1→0…`) ; sinon sawtooth (`0→1`, saut à 0,
  `0→1…`). Implémenté **dans le `tick`** : à la fin d'un cycle, au lieu de se
  reposer, le contrôleur **relance** un cycle (bord opposé si `reverse`, sinon
  repart du bas). `is_animating()` reste vrai — donc `tick()` continue de renvoyer
  `true` et les frames coulent.

- **Démarrage partagé.** `animate_to` a été scindé : un `start_interpolation`
  interne (ne touche pas au mode boucle) sert **et** `animate_to` (à un coup, qui
  annule `repeat`) **et** le redémarrage de cycle. Les méthodes à un coup
  (`forward`/`reverse`/`fling`/`spring_to`/`drive`/`set_value`) **annulent** une
  boucle en cours — passer à une animation ponctuelle sort naturellement de la
  boucle.

- **`stop()` / `reset()`.** `stop` fige la valeur et met fin à la boucle ; `reset`
  ramène à la borne basse (`set_value(lower)`). Complètent le contrôle explicite.

## Implémentation

`frus-core/animation/controller.rs` : champ `repeat: Option<Repeat>` ;
`repeat`/`stop`/`reset` ; `animate_to` refactoré autour de `start_interpolation` ;
`tick` relance un cycle si une boucle est active ; annulation de boucle dans les
départs à un coup.

## Tests

- `repeat_never_settles_and_restarts` (sawtooth) : sur ~16 cycles, reste toujours
  animé, atteint le haut, et **retombe** (la valeur chute → nouveau cycle).
- `repeat_reverse_ping_pongs` : la valeur **monte puis redescend** (aller-retour).
- `stop_and_reset_end_a_repeat` : `stop` met fin à la boucle (plus rien à ticker) ;
  `reset` l'arrête et ramène à `0`.
- Suite complète verte : les animations existantes (à un coup) sont inchangées.

## Motif d'usage (rappel)

```rust
struct App { pulse: AnimationController }           // état

fn init(&mut self) -> Command<_> {
    self.pulse.repeat(1.0, true, Curve::ease_in_out()); // boucle aller-retour
    Command::none()
}
fn tick(&mut self, dt: f32) -> bool { self.pulse.tick(dt) } // avance, redemande une frame
fn view(&self, ..) -> Box<dyn Widget<_>> {
    Container::new().opacity(0.4 + 0.6 * self.pulse.value()) /* … */
}
```

## Reste

- Un widget/adaptateur nommé pour les cas courants (p. ex. relier un contrôleur à
  une propriété) — l'implicite couvre déjà l'essentiel.
- `Tween::animate(controller)` typé (frus-core a `Tween`) pour mapper la valeur.
- Démo dédiée d'une boucle `repeat`.
