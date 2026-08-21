# Milestone 380 — A keyboard that knows what it is typing into

The Android input bridge set the same `EditorInfo` for every field in every application:

```java
out.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES;
out.imeOptions = EditorInfo.IME_ACTION_DONE | EditorInfo.IME_FLAG_NO_FULLSCREEN;
```

Sentence-cased prose and a *Done* key. That is a fine default and it was the only thing on
offer, so:

- a phone-number field opened a full QWERTY keyboard;
- an email field had no `@` within reach, and the keyboard capitalised the first letter —
  `Someone@…` is an address that does not work;
- every field in a five-field form said *Done* where the next thing to do was go to the next
  field;
- a field taking several lines had a key that ended the editing instead of adding a line;
- and a **password field told the keyboard it was ordinary prose**.

The last one is not a convenience. `obscure` draws dots on our side; the keyboard, told
nothing, learns what is typed into its personal dictionary and offers it back as a
suggestion later, on whatever screen comes next. The masking was never the part that
protected anything — `TYPE_TEXT_VARIATION_PASSWORD` is.

## The vocabulary

`KeyboardType` says which keys and `TextInputAction` says what the action key does;
together they are `Ime`, which `Widget::ime` hands over when a field takes focus. The
platform layer finds the focused field the same way it finds the caret, so the two answers
cannot disagree about which field is being typed into.

```rust
TextField::new(&app.phone).keyboard_type(KeyboardType::Phone)
TextField::new(&app.query).action(TextInputAction::Search)
```

Untold, a field works it out from what it already is: a masked field is a `Password`, a
multi-line one is `Multiline` with a `Newline` key, and everything else is the text keyboard
with *Done* — which is exactly what was hardcoded, so nothing that says nothing changes.

The order of those two defaults only makes sense one way round. A masked field is a secret
first and a text field second, and a multi-line secret is not a thing anybody types.

## The numbers are computed in Rust

The mapping to Android's `InputType` and `EditorInfo` bit fields lives in
`frus-widgets/src/ime.rs`, and it is **not** behind `cfg(android)`. Two reasons, and they
are the same reason.

A mapping only exercised on a device is a mapping nobody checks. There are eight tests over
it and they run on every platform, on every push: that only `None` is `TYPE_NULL`, that an
email and a URL and a password carry no capitalisation flag, that a newline is
`UNSPECIFIED` rather than an action, that each type names its own class.

And the bridge's dex is **checked in** — rebuilding it needs the Android SDK, which most
contributors will not have. A Java file that only ever copies two integers onto the
`EditorInfo` never has to be rebuilt again for a keyboard type nobody has thought of yet.
It is a pipe, not a policy.

`restartInput` in `startInput` is what makes the second field's keyboard differ from the
first's. Without it the IME keeps the `EditorInfo` it was handed first, and every field
after the first would inherit the first field's keyboard — the bug this change would have
shipped with, had the call not already been there for another reason.

## `IME_FLAG_NO_FULLSCREEN` on every one of them

Android's landscape fullscreen editor replaces the application's own field with a system
one. A framework that draws its own text has a screen to lose and nothing to gain, so the
flag is part of `android_ime_options` rather than something a caller could forget.

## Left

`autocorrect` and `enableSuggestions` as separate switches — Android carries them as more
flags on the same integer, so they are a builder and an arm, not a design. `textCapitalization`
likewise. iOS and the Web have their own spellings of all of this and neither has a text
field yet. And `textAlign`, `inputFormatters` and the cursor's own colour and width remain
what they were before this milestone: absent.
