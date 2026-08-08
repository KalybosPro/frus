# Jalon 86 — Localisation (i18n/l10n): Fluent

## Analysis

§14 recommends: *integrate **Fluent** (`fluent-rs`, Mozilla's Rust i18n
standard) for messages, plurals and per-locale selections. Do not reinvent it.*
So frus provides a ready-made **localiser**, without imposing a home-grown
format.

## Architecture

A new crate **`frus-l10n`** (a wrapper over `fluent-bundle`):

- `Localizer`: **concurrent** `FluentBundle`s (a memoiser behind a `Mutex` →
  `Send + Sync`, so it can live in a `static`/`OnceLock` on the app side), a
  current locale and a default locale (the ultimate fallback).
- `add(locale, ftl)`: loads a `.ftl` resource (the app bundles them through
  `include_str!`).
- `set_locale(locale)`: **negotiation** written in-house, with no heavy
  dependency — an exact match (`fr-FR`), then by language (`fr-CA` → `fr`), and
  otherwise the default locale.
- `format(key, args![…])` (the current locale) and `format_for(locale, key,
  args)` (an explicit locale, **non-mutating** → ideal for a pure `view`).
- A three-level fallback: the requested locale → the default → **the raw key** (a
  missing message shows up without breaking the UI).
- The `args![name: "Ada", n: 3]` macro: text and number arguments; the numbers
  drive the **CLDR plurals** (`intl_pluralrules`, free with Fluent).

Bidi isolation disabled (`set_use_isolating(false)`): legible and testable
output; RTL direction is handled at layout time (`Theme::direction`, J84), not
through marks in the text.

## Decisions

- **In-house negotiation** rather than `fluent-langneg`: `LanguageIdentifier`
  does not implement `AsRef<Self>` (API friction), and the exact→language rule is
  sufficient and testable in a few lines — one dependency fewer.
- A **concurrent bundle** is mandatory: the simple bundle is `!Sync`, and the app
  wants it in a global `OnceLock`.

## Demo

`frus-demo` bundles `i18n/en.ftl` + `i18n/fr.ftl`, an `OnceLock<Localizer>`
loaded once, an **English ↔ Français** menu action, and localises the AppBar's
title, the filter segments, and the summary (**pluralised** counters: "3 tasks /
3 tâches", "No tasks / Aucune tâche").

## Tests (296 → 302)

- `frus-l10n`: arguments, **per-locale plurals** (en: 1 task / 5 tasks; fr:
  1 tâche / 3 tâches), region→language negotiation (`fr-CA` → `fr`), the
  default-then-key fallback, an unknown locale → the default. + a doctest.
- 23 suites green.

## Validated on the device

The "Français" action → the title **"Mes tâches"**, the filters
**"Toutes / Actives / Terminées"** (Fluent messages resolved in French); back to
"English" → English. ✔

## What's left

- Per-locale **date and number** formatting (Fluent's `DATETIME`/`NUMBER`
  functions) — to be wired up when a consumer calls for it.
- Automatic selection of the system locale at start-up.
