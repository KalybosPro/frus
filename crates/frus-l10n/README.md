# frus-l10n

Localisation for frus applications, built on [Fluent](https://projectfluent.org/): messages,
plurals and locale fallback.

**Layer:** Foundations. It depends on no other frus crate; messages live in `.ftl` sources
the application embeds. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Localizer`: bundles per locale, a current locale, and a default one as the last resort.
- `args!`: a message's arguments, `args![name: "Ada", n: 3]`.
- `Localizer::set_locale`: negotiates the closest available locale (`fr-CA` finds `fr`).

## Example

```rust
use frus_l10n::{args, Localizer};

let mut l10n = Localizer::new("en");
l10n.add("en", "tasks = { $n ->\n    [one] { $n } task\n   *[other] { $n } tasks\n }");
l10n.add("fr", "tasks = { $n ->\n    [one] { $n } tâche\n   *[other] { $n } tâches\n }");

assert_eq!(l10n.format("tasks", args![n: 1]), "1 task");
l10n.set_locale("fr-CA");
assert_eq!(l10n.format("tasks", args![n: 2]), "2 tâches");
```

## Part of frus

See the [workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT
or Apache-2.0.
