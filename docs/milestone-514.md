# Milestone 514 — A sign-up form that opened a sentence keyboard for an email address

Seen on a phone while checking milestone 512: typing an address into the demo's sign-up
wizard, SwiftKey capitalised its first letter and put a space after each word it suggested.
The keyboard was doing exactly what it was asked, which was nothing in particular.

## Why

The framework has said which keyboard a field wants since `KeyboardType` existed, and a
masked field already asks for a secret's keyboard on its own. The wizard said nothing else:
its fields told autofill what they were for (512) and never told the keyboard, so the name
and the email both opened ordinary sentence text — capitals at the start, suggestions with a
space after them. Nothing in the framework was wrong; the demo, which is what an application
author reads to learn how a form is written, showed a form written wrong.

## The fix

Each wizard field now asks for its keyboard, through `wizard_keyboard(field, obscure)`:

| field | keyboard |
|---|---|
| Full name | `Name` — each word capitalised, no sentence casing |
| Email | `Email` — an `@` within reach, no capitalisation, no learning |
| Password, Confirm password | `Password` while masked |
| the same, revealed by the eye | `VisiblePassword` |

The last row is the one that was wrong in a way nobody would have seen. A masked field got a
secret's keyboard by default; but the eye turns the masking off, and a field that is neither
masked nor told otherwise is ordinary text — so **revealing a password handed it to a keyboard
that learns what it is given**, into a personal dictionary that offers it back later. Showing
a password in the clear is for the reader to check it, not for the keyboard to keep it, so the
revealed fields ask for `VisiblePassword`: shown, never learned.

The framework's own default is left as it is. A field that is not masked is not known to be
a secret, and guessing from an autofill hint would be a second way of saying the same thing
that could disagree with the first; the reference keeps the two apart as well.

## Verification

**One test**, reading the keyboard off the tree the view builds — each focus stop in the form,
looked up in the tree and asked for its `ime()`, which is what the shell hands the platform —
rather than off the helper alone: name and email on the first step, two secrets on the
second, and two visible secrets once the eye is pressed.

**Mutations**, four, each failing it: no keyboard asked for; the email opening text; the
secret branches swapped; the reveal ignored.
