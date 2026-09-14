# Milestone 512 — A field that says what it is for

Answers [#45](https://github.com/KalybosPro/frus/issues/45): a field could say which keys
it wanted and never what it held, so the platform's autofill service had nothing to go on.
A saved password was never offered on a sign-in screen, a code arriving by SMS was never
offered above the keyboard, an address was typed out by hand, and a password just created
was never saved. None of it failed loudly; it just never happened.

## The vocabulary

- **`AutofillHint`** — what a field is for: `Username`, `NewUsername`, `Password`,
  `NewPassword`, `OneTimeCode`, `Email`, `Name`, `GivenName`, `FamilyName`,
  `TelephoneNumber`, `StreetAddress`, `PostalAddress`, `PostalCode`, `AddressCity`,
  `AddressState`, `CountryName`, `CreditCardNumber`, `CreditCardExpirationDate`,
  `CreditCardSecurityCode`. The useful part of the platform's catalogue — a sign-in, a
  sign-up, an address, a card payment — not all of it; a hint the list lacks is a variant,
  a name and a test.
- **`TextField::autofill(hints)`** takes a **list**, most telling first, because a field can
  legitimately be more than one thing: a sign-in box that takes an email address is
  `[Username, Email]`.
- **`AutofillGroup`** declares the fields of one form. A credential is a pair, and a service
  shown a password with no username beside it has nothing to save it under. A field in no
  group is a form of one, which is right for a lone code field and wrong for a sign-in.

## The names are Android's, and the tests are not

Each hint carries the string Android's services expect (`AutofillHint::android`), and the
table lives in the widgets crate, next to the keyboard's own numbers, for the same reason:
a mapping only exercised on a device is a mapping nobody checks, and the bridge's dex is
checked in. The names are rarely the obvious ones — an email address is `emailAddress`, a
person's name `personName`, a telephone number `phoneNumber`, a town `addressLocality`, a
one-time code `smsOTPCode` — and a service handed `email` fills nothing, silently.

## How a form reaches the service

A service meets fields that are views. These are drawn, so the bridge view that already
captures the keyboard declares the form as a **virtual structure** — one child per field,
with its hints, its value, its box, and whether it is sensitive — and hands the values the
service chooses back to the native side.

- **The form is the focus registry's.** Every field of a form is a focus stop, and the
  focus registry already survives everything a frame does to its registries — the
  relayout cache's snapshots, their replay, the truncation of a leaving subtree — so a stop
  records the group it sits in, and `Ui::form_of` answers a field's form from there. A
  registry of its own would have had to learn all of that again, and one path forgotten is
  a cached form that silently loses its fields.
- **What is shown is decided in a pure module** (`frus-shell/src/autofill.rs`) and carried
  across by `android_autofill.rs`: only the stops that say what they are for; in the
  form's order; boxes in physical pixels; a secret — a password, a one-time code — marked
  sensitive and still given its value, or there would be nothing to save. A field's id is
  folded from its identity, which is a tree position and so stable across frames: a value
  handed back later still finds its field.
- **When a field takes focus**, its form is published and the service is told which field
  it is in, with the field's box on screen — which is where it anchors its offer. The form
  is built aside on the Java side and swapped whole, so a service asking half-way through
  sees the last form rather than half of this one.
- **While it keeps focus**, every value the service has not heard is reported. A service
  saves the values it was shown, so a password typed after the form was published, and
  never reported, is saved as what it was then: nothing.
- **When the group leaves the screen** — the form submitted, or navigated away from — the
  form is committed, the moment a service may offer to save what was typed. This is the
  reference's default for its group.
- **A value the service chooses** goes into its field through the message typing would have
  produced — the path an undo takes, so a disabled or read-only field refuses it — with the
  caret after it. A value for a field that has gone is dropped.

Every call across is guarded for Android 8 (API 26), where autofill begins; the demo's
minimum is 24, where all of this is simply absent. A failed call clears the Java exception
it left, or the next call across would abort the process, and every string made for a call
lives in a local frame of its own: the thread is attached for good, and a local reference
nobody frees lives as long as the process.

## The demo

The sign-up wizard's fields say what they are for — the name `Name`, the email
`[Username, Email]`, both passwords `NewPassword` — and its steps are **one** group. The
group sits at the same place in the tree whatever the step, so the account's name typed on
the first step and its password on the second are one form, not two.

## Two faults the device found

Every test was green before the demo went onto a phone, and the phone found two faults
no test could have — both in what the service is told, neither in what the widgets
declare.

- **The offer was anchored a status bar too low.** Huawei's service recorded the email
  field's box about fifty pixels below where the field is drawn, and the password's by
  the same amount. The native surface fills the window, so a field's box is measured from
  the window's corner; the bridge view, where that box was moved to the screen's corner,
  sits in the content frame below the status bar, and its position was added on top. The
  box is now moved by the root view's position, and each virtual child's is given
  relative to the bridge view, as a child's must be. On the same phone afterwards, the
  name field's box as the service recorded it runs from 487 to 640 pixels — the field's
  own, the floating label's band above its border included.
- **A form in steps reached the service as half a credential at a time.** The demo's
  wizard asks for the name and the email on its first step and the passwords on its
  second, and the group is the same one on both. But a stop is only in the frame while
  it is on screen, so the service was shown a name and an email with no password, then
  two passwords with no account — and offered to save neither. Now the fields of the form
  already shown stay in what is published when the next step's field takes focus, with
  what they last held and a box of one pixel (`autofill::with_gone`): the way the
  reference reports a field that is not the one being edited. Another form, or a field in
  no group, starts afresh.

## The desktop and the web

Nothing, and on purpose. A desktop has no platform autofill for an application's own
fields — a password manager there fills the browser, or types. The web has `autocomplete`
on an `<input>`, and frus draws its fields rather than making inputs, so there is no
element to put it on; the hint's web name would be the obvious next line in the table the
day there is.

## What this does not do

- **A sign-in screen** in the demo. The wizard is a sign-up, which is what a service
  offers to save; offering a saved credential is the same structure with `Username` and
  `Password`.
- **A value changed by the application** in a field that does not have focus is reported
  only when one of the form's fields next has it.
- **iOS**, which has no shell yet (#19).
- **An offer from a service, seen.** Every step up to it was seen on a device — the form
  shown, answered, changed and committed — and the offer itself was not; see below.

## Verification

- Six tests of the vocabulary: each hint's platform name, the ones that are not the hint's
  own above all; no two hints share a name; the secrets are the passwords and the code; a
  group names itself and changes nothing else; the wrappers that fuse with a field —
  `Keyed`, `Responsive`, `Box` — pass its hints and its group on; a field keeps its hints
  in order.
- One of the frame: a field knows its form — whichever of two fields asks, two groups are
  two forms, a field in no group is a form of one.
- Seven of the rules between the frame and the platform: only the fields that say what they
  are for are shown, in order; boxes in physical pixels; a secret marked sensitive and
  still carrying its value; a field keeps its id and two fields do not share one; a value
  goes to the field it names and nowhere else; only a value the service has not heard is
  reported; a step behind the one showing is still part of the form, placed nowhere, and
  a field on screen is never repeated.
- The structural tripwires of the transparent wrapper caught the first version: a group
  hook added to the trait and not forwarded would have made `Keyed(AutofillGroup(…))` no
  form at all. `autofill_group` is now one of the hooks each wrapper states, like `key`.
- One of the demo: the wizard is one form across its steps, and nothing but its fields is
  in it.

- Fifteen mutations, each failing the test meant for it: an email named for itself; the
  one-time code no secret; `Responsive` dropping the hints; a group leaking to the fields
  after it; stops forgetting their form; a grouped field made a form of one; fields
  without hints shown; boxes in logical pixels; nothing sensitive; ids truncated rather
  than folded; every value to the first field; a field the service never saw not
  reported; a step behind forgotten; a field on screen repeated; a gone field keeping its
  stale box. One of them could not be applied the first time — the line it changes is
  written at both places a stop is registered — and was run by hand on both.

## On the device

Huawei STK-L21, Android 10, SwiftKey; the demo's sign-up wizard, filled with test data.

- **The service is shown the form.** The platform's own history (`dumpsys autofill`)
  records a fill request for the demo each time a field of the form takes focus — for
  Huawei's Keychain service, and for Google's, with the owner's permission, the setting
  switched for the test and put back afterwards. Each request carries the field's virtual
  id, the same for the same field across steps and across runs, and its box on screen.
- **The box is where the field is.** Before the fix the boxes sat a status bar low (748
  for a field drawn from about 697); after it, the name field's box runs 487–640 and the
  email's 676–829, each the field with its floating label's band above.
- **A complete form gets an answer.** With the steps merged, Google's service kept its
  session open on the password step, the form's other fields marked fillable: it had
  answered the structure. The first step alone — a name and an email, no password — ended
  its session at once. Huawei's ended every session at once and logs nothing, so whether
  it declines applications it does not know, or something in this structure, could not
  be told.
- **Changes and the commit reach it.** A password changed within a session is marked
  changed in the session's state, and leaving the wizard recorded the form's context as
  committed in the service's event history.
- **No offer was seen.** Neither service offered to save the account or to fill a field,
  and no dialog appeared. Neither logs anything that says why, so the device cannot tell
  a service's policy from a fault here; the issue's criteria — a saved credential offered
  on a sign-in screen, a one-time code offered above the keyboard — are not met, and #45
  stays open for them.
