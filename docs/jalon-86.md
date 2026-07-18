# Jalon 86 — Localisation (i18n/l10n) : Fluent

## Analyse

Le §14 recommande : *intégrez **Fluent** (`fluent-rs`, le standard i18n Rust de
Mozilla) pour les messages, pluriels et sélections par locale. Ne réinventez
pas.* frus fournit donc un **localiseur** clé en main, sans imposer de format
maison.

## Architecture

Nouveau crate **`frus-l10n`** (enveloppe `fluent-bundle`) :

- `Localizer` : des `FluentBundle` **concurrents** (mémoïseur derrière un
  `Mutex` → `Send + Sync`, donc plaçable dans un `static`/`OnceLock` côté app),
  une locale courante et une locale par défaut (le repli ultime).
- `add(locale, ftl)` : charge une ressource `.ftl` (l'app les embarque via
  `include_str!`).
- `set_locale(locale)` : **négociation** maison, sans dépendance lourde —
  correspondance exacte (`fr-FR`) puis par langue (`fr-CA` → `fr`), sinon la
  locale par défaut.
- `format(key, args![…])` (locale courante) et `format_for(locale, key, args)`
  (locale explicite, **non mutant** → idéal pour une `view` pure).
- Repli à trois niveaux : locale demandée → défaut → **la clé brute** (un
  message manquant se voit sans casser l'UI).
- Macro `args![name: "Ada", n: 3]` : arguments texte/nombre ; les nombres
  pilotent les **pluriels CLDR** (`intl_pluralrules`, gratuit avec Fluent).

Isolation bidi désactivée (`set_use_isolating(false)`) : sortie lisible et
testable ; la direction RTL est gérée à la mise en page (`Theme::direction`,
J84), pas par des marques dans le texte.

## Décisions

- **Négociation maison** plutôt que `fluent-langneg` : `LanguageIdentifier`
  n'implémente pas `AsRef<Self>` (friction d'API), et la règle exacte→langue
  suffit et se teste en quelques lignes — une dépendance de moins.
- **Bundle concurrent** obligatoire : le bundle simple est `!Sync`, or l'app le
  veut dans un `OnceLock` global.

## Démo

`frus-demo` embarque `i18n/en.ftl` + `i18n/fr.ftl`, un `OnceLock<Localizer>`
chargé une fois, une action de menu **English ↔ Français**, et localise le
titre de l'AppBar, les segments de filtre, et le résumé (compteurs
**pluralisés** : « 3 tasks / 3 tâches », « No tasks / Aucune tâche »).

## Tests (296 → 302)

- `frus-l10n` : arguments, **pluriels par locale** (en : 1 task / 5 tasks ; fr :
  1 tâche / 3 tâches), négociation région→langue (`fr-CA` → `fr`), repli
  défaut-puis-clé, locale inconnue → défaut. + doctest.
- 23 suites vertes.

## Validé sur l'appareil

Action « Français » → titre **« Mes tâches »**, filtres
**« Toutes / Actives / Terminées »** (messages Fluent résolus en français) ;
retour « English » → anglais. ✔

## Reste

- Formatage **dates/nombres** par locale (fonction `DATETIME`/`NUMBER` Fluent) —
  à brancher quand un consommateur le réclame.
- Sélection auto de la locale système au démarrage.
