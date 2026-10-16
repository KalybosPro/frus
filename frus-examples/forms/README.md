# forms

A "Create your account" form, built the way a real one is: every common kind of input,
validation that shows at the right moment, a submit that waits for a server, and a second
page that shows what was saved.

## What it teaches

- **Inputs.** Text fields with labels, icons, helper text and keyboard types (email,
  phone); a password with a show/hide eye and a strength meter; a multi-line bio with a
  character counter (`max_length`); a date picked from a calendar in a `Dialog`; a
  filtering `DropdownMenu`; a `RadioGroup`; filter `Chip`s in a `Wrap`; a `Switch`; a
  `Checkbox`.
- **Validation with `frus::form`.** `Form` and `Rule` check every field in one pure
  function: `required`, `email`, `min_len`/`max_len`, `Rule::all` to chain them, a
  cross-field check (`field_with`: the confirmation must match the password), and rules
  over values that are not text (a date, a choice, a ticked box). The rules are unit-tested
  with no window.
- **When errors show.** Nothing is red while the form is being filled in for the first time;
  after the first submit attempt, every field shows its error, an `ErrorSummary` lists them
  at the top (each line moves the focus to its field), and the focus jumps to the first
  field in error (`host::focus` + `keyed`). A field stops being in error the moment its
  value is fixed, because the report is recomputed from the values on every build.
- **Submitting.** The button turns into a disabled spinner while the request is in flight
  (`host::after` stands in for the network), so it cannot be pressed twice. The pretend
  server turns down `taken@example.com`, and that error lands on the email field like any
  other.
- **`TextEditingController`.** The text fields' values live in controllers held by the page's
  state, so "Fill with sample data" (the wand in the app bar) can write into them.
- **Autofill.** The fields carry autofill hints (`Name`, `Email`, `NewPassword`,
  `TelephoneNumber`) inside one `AutofillGroup`, so a password manager can fill them and
  save the new account.
- **Two routes.** `GoRouter` with `/` (the form) and `/summary`; the submitted profile
  travels with the navigation (`push_with_extra`), and Back returns to the filled form.
- **Layout.** The content is a centred column, full width on a phone and capped at 560 px
  on a desktop.

## Running it

```sh
cargo run                   # desktop
cargo run -- "/?errors"     # the form as it looks after a failed submit
cargo run -- /summary       # the summary page, with sample data
cargo apk run --lib         # Android
```

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/forms.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

On the Web, `controllers/clock.rs` does not read today's date (the standard library has no
clock there), so the minimum-age rule is skipped; `js_sys::Date` is how you would add it.

## The files

```
src/
  lib.rs                          the module list and frus::main!
  common/
    routes.rs                     "/" and "/summary"
    routers.rs                    the GoRouter, and the start location from the command line
    colors.rs                     the strength meter's colours, the success tick
    shared.rs                     the centred page column, the error line under a control
  models/
    profile.rs                    Profile, Date, Plan, the countries and interests
  features/
    signup/
      pages/signuppage.rs         the form: one method per field, the dialog, the submit
      states/signupstate.rs       controllers and flags, and every way they change (tested)
      controllers/
        validation.rs             the rules and the password strength (tested)
        signup_service.rs         the pretend server
        clock.rs                  today's date, for the age rule
      widgets/
        form_section.rs           a titled card around a group of fields
        password_strength.rs      the meter under the password
        interest_chips.rs         the filter chips
        birth_date_calendar.rs    the year switcher and calendar in the date dialog
        submit_button.rs          "Create account", or a spinner while submitting
    summary/
      pages/summarypage.rs        what was saved
```

`cargo test` runs the rules and the state's behaviour without opening a window.
