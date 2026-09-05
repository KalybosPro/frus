# Milestone 454 — The device's language never reached the application

This is the trap of milestone 408 again, and worse: **two** halves were built and neither
was ever connected to the one thing that would have made them work.

- `frus-l10n` has had Fluent and its negotiation for a long time, so an application could
  translate its own messages.
- Milestone 449 gave the framework a `Localizations` table, so it could say *Retour*
  instead of *Back*.

**Nothing ever told either of them what language the device is set to.** Every platform
reports it. Nothing here read it. So an application either hard-coded a language or asked
the reader to pick one — on a device that already knew the answer. This repo's own
demonstration opened in English on a French phone and offered a menu.

Both halves passed every test they had, because a test picks the language it is testing.

## `Locale`

The reference's shape (`locale.dart`): a **required** language, an optional script and an
optional country. `fr` is an answer, `fr-CA` is a better one, and `zh-Hant-TW` needs all
three to be right.

Comparison is exact. Deciding that `fr` will do for `fr-CA` is the resolution's work, and
it needs the whole list to do it well.

`Locale::parse` reads a tag by the **shape** of its subtags, as BCP 47 does: four letters
is a script, two letters or three digits is a region. That is what makes `zh-Hant` and
`zh-TW` both readable without a table of every code in the world. It takes both separators,
because both arrive — a language tag uses `-` and a POSIX `LANG` uses `_` — and drops the
encoding, because `en_US.UTF-8` is the `en-US` locale in a particular byte encoding and the
encoding is not part of which language it is. `C` and `POSIX` name the absence of a
language and read as `None`.

## The negotiation

`locale::resolve` is the reference's `basicLocaleListResolution` (`app.dart:146`), rung for
rung, and the rung worth understanding is the fourth.

A **language-only** match is a weak match: `fr` for a reader who asked for `fr-CA`. From
the reader's **second** choice onward it is remembered rather than returned, so the round
after it gets a chance to do better — a reader listing `[it, fr-BE, fr-CA]` gets `fr-CA`
and not `fr`.

The reader's **first** choice is trusted instead, and that is not an optimisation, it is the
answer: a reader whose list begins with `fr-CA` gets French even where the application
matches their *second* choice exactly, because asking for French first outranks asking for
Canadian English second. I wrote the test asserting the opposite and the implementation
disagreed with me; reading the reference's trace through settled it, and the test says so
now.

The last rung is a **country** with no language match anywhere — a reader is likely to know
a language spoken where they are. It beats the application's default and loses to any
language match, however late that arrives.

Like the reference's, it does not consider how close two languages are to one another.
The reference says so in the same words: German resolves to Chinese over French if Chinese
is listed first.

## The wire, per platform

| | where the answer lives |
|---|---|
| Windows, macOS, Linux, iOS | `sys-locale` — one small crate over four different system APIs, and **already in the tree**: `cosmic-text` pulls it, so naming it adds no dependency at all |
| Android | the activity's `Configuration.getLocales()` (API 24+), over the JNI walk `android_settings` already makes; `configuration.locale` below that |
| Web | `navigator.languages`, falling back to `navigator.language` |

Desktop reads it **once**. Changing the display language means signing out and back in on
Windows and is a per-process environment variable on Linux, so there is nothing to watch
for. Android is the platform where it changes under a running application, and Android
re-reads it on every walk.

## The trait

```rust
fn supported_locales(&self) -> Vec<Locale>              { vec![Locale::default()] }
fn locale(&self) -> Option<Locale>                      { None }
fn resolved_locale(&self, preferred: &[Locale]) -> Locale { /* the rungs */ }
```

`supported_locales`' **order** is not decoration: it decides ties (listing `en-US` before
`en-GB` says which English a reader who asked only for `en` gets) and its first entry is
what a reader whose languages are all unavailable ends up with.

`locale` is what an application's own *Language* setting writes to. It is still **resolved**
rather than obeyed — pinning one the application does not have gives the nearest thing it
does have, not nothing.

## One ordering bug, found by moving the wire

`install_ambient` sat next to the build, which is in time for every widget. It is **one
frame late for the theme**: the direction of the layout follows the language, so an
application whose theme is right-to-left in Arabic asks for its theme before the language
that decides it is installed. The install moved above the theme's own resolution, where it
belongs, and the comment there says why.

## The demonstration lost its hard-coded language

`lang: usize` became `lang: Option<usize>` — `None` follows the device — and the language
menu cycles through the three languages and **back to the device's own**. `theme_of` reads
`lang_of(app)`, which reads `locale::of()` when the reader has not picked. On a French
phone the demonstration now opens in French, and nothing in it knows how that happened.

## The tests

Seven in `locale`, one in the shell:

- `a_language_tag_is_read_by_the_shape_of_its_subtags` — both separators, the encoding
  suffix, a three-digit region, and the tags that name no language.
- `two_locales_are_equal_or_they_are_not`.
- `the_rungs_of_a_locale_resolution` — every rung, including an empty preferred list.
- `a_weak_language_match_waits_for_a_better_one` — the fourth rung, in all four of its
  cases.
- `a_country_is_the_last_thing_tried`.
- `the_application_s_order_decides_which_of_two_will_do`.
- `the_scope_answers_and_restores` — including through a panic.
- `a_shell_installs_the_reader_s_language` — **the guard**. `locale::of()` answers `en`
  with nothing installed, which is what makes the feature safe to add and what would hide
  the wiring being missing. So this drives the shell's own reading of the trait rather than
  calling `resolve`.

With the wire pulled out and the negotiation reduced to *take the reader's first choice*,
**five** of the eight fail: the four about the rungs, and the shell guard. The three that
survive are about reading a tag and restoring a scope, which that revert does not touch.

Three targets were type-checked, not one: `x86_64-unknown-linux-gnu`,
`aarch64-linux-android` and `wasm32-unknown-unknown`. The Android target caught a real
error the desktop build could not see — the desktop tail of `refresh_platform_settings` is
compiled on Android too, and its `platform_locales` was gated away.

**The goldens did not move.**
