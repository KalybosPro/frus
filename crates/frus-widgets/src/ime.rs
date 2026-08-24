//! What the **software keyboard** should be, for the field that has focus.
//!
//! A phone-number field that opens a full QWERTY keyboard is a defect you feel on
//! every use and never see in a screenshot. So is an email field with no `@` key, a
//! form whose every action key says *Done* when the next thing to do is move to the
//! next field, and — the one that matters most — a password field the keyboard
//! believes is an ordinary sentence, learns into its personal dictionary, and offers
//! back as a suggestion later, on somebody else's screen.
//!
//! [`KeyboardType`] says which keys; [`TextInputAction`] says what the action key does.
//! Together they are [`Ime`], which [`Widget::ime`](crate::Widget::ime) hands to the
//! platform when a field takes focus.
//!
//! ```ignore
//! TextField::new(&app.phone).keyboard_type(KeyboardType::Phone)
//! TextField::new(&app.query).action(TextInputAction::Search)
//! ```
//!
//! # The numbers are Android's, and the tests are not
//!
//! The mapping to Android's `InputType` and `EditorInfo` bit fields lives here rather
//! than in the platform layer, and it is not behind `cfg(android)`. Two reasons, and
//! they are the same reason: **a mapping only exercised on a device is a mapping
//! nobody checks**, and the input bridge's dex is *checked in* — rebuilding it needs
//! the Android SDK, so the Java side must never have to change again to learn a new
//! keyboard type. It receives two integers and sets them on the `EditorInfo`. Every
//! type this file will ever grow is a change to this file alone.

/// Which keys the software keyboard should offer.
///
/// The platform decides what that means: a phone keypad on Android and iOS, and
/// nothing at all on a desktop, which has one keyboard and it is already open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyboardType {
    /// Ordinary text, with suggestions and composition. The default.
    #[default]
    Text,
    /// Text over several lines, where the action key is a newline.
    Multiline,
    /// Whole numbers.
    Number,
    /// Numbers with a decimal separator and a sign.
    Decimal,
    /// A telephone keypad: digits, `+`, `*`, `#`.
    Phone,
    /// An email address: an `@` and a `.` within reach, and no auto-capitalisation.
    Email,
    /// A URL: a `/` and a `.com` within reach.
    Url,
    /// A **secret**. The keyboard neither learns it nor suggests it, which is the
    /// whole point and is not something the masking dots can do on their own.
    Password,
    /// A secret the field shows in the clear — a one-time code, a generated
    /// passphrase being checked. Still never learned.
    VisiblePassword,
    /// A person's name: each word capitalised, no sentence casing.
    Name,
    /// A postal address.
    StreetAddress,
    /// A date or a time.
    DateTime,
    /// **No** software keyboard: the field is edited some other way — a picker, a
    /// scanner, a hardware keyboard only.
    None,
}

/// What the keyboard's action key does, and what it says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputAction {
    /// *Done*: finish editing and close the keyboard. The default.
    #[default]
    Done,
    /// *Go*: follow what was typed, usually an address.
    Go,
    /// *Search*.
    Search,
    /// *Send*.
    Send,
    /// *Next*: move to the following field, keeping the keyboard open.
    Next,
    /// *Previous*: move back to the field before.
    Previous,
    /// A **newline**, for a field that takes more than one line. Not an action at
    /// all — the key inserts a line break instead of doing anything.
    Newline,
    /// No action key behaviour at all; the platform picks.
    Unspecified,
}

/// Which letters the software keyboard capitalises on its own.
///
/// A hint, not a rule: it is what the **keyboard** does to what it sends, and a reader
/// who turns it off still types what they meant to. A field that must hold capitals
/// wants [`TextField::input_filter`](crate::TextField::input_filter) as well, which
/// works whatever the keyboard does and works on a desktop, where there is no keyboard
/// to hint to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Capitalization {
    /// Whatever the keyboard type already implies — sentences for ordinary text, words
    /// for a name or an address, nothing for an email address. The default, and right
    /// nearly always, since the type is the better description of the two.
    #[default]
    Auto,
    /// Nothing. The reader capitalises what they mean to capitalise.
    None,
    /// The first letter of each sentence.
    Sentences,
    /// The first letter of each word.
    Words,
    /// Every letter.
    Characters,
}

impl Capitalization {
    /// The Android `InputType` bit for this, or zero for none.
    const fn flag(self) -> i32 {
        use input_type as t;
        match self {
            // `Auto` never reaches here; see [`Ime::android_input_type`].
            Self::Auto | Self::None => 0,
            Self::Sentences => t::TEXT_FLAG_CAP_SENTENCES,
            Self::Words => t::TEXT_FLAG_CAP_WORDS,
            Self::Characters => t::TEXT_FLAG_CAP_CHARACTERS,
        }
    }
}

/// The pair the platform is told when a field takes focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ime {
    /// Which keys.
    pub keyboard: KeyboardType,
    /// What the action key does.
    pub action: TextInputAction,
    /// Which letters the keyboard capitalises by itself.
    pub capitalization: Capitalization,
}

impl Ime {
    /// A keyboard of `keyboard` whose action key is *Done*.
    pub const fn new(keyboard: KeyboardType) -> Self {
        Self {
            keyboard,
            action: TextInputAction::Done,
            capitalization: Capitalization::Auto,
        }
    }

    /// The same, capitalising differently from what the type implies.
    pub const fn capitalization(mut self, capitalization: Capitalization) -> Self {
        self.capitalization = capitalization;
        self
    }

    /// The Android `InputType` bit field for this field: the type's, with the
    /// capitalisation overruled where one was asked for.
    ///
    /// Only a **text** class capitalises. A number pad has no letters to capitalise and
    /// the same bits mean other things there (`0x1000` is *signed* on a number class),
    /// so asking a phone field for capitals would quietly turn on a minus key.
    ///
    /// [`Capitalization::Auto`] leaves the type's own bits alone, which is why the
    /// existing types keep answering exactly what they always did.
    pub const fn android_input_type(self) -> i32 {
        use input_type as t;
        let base = self.keyboard.android_input_type();
        if base & t::CLASS_MASK != t::CLASS_TEXT {
            return base;
        }
        match self.capitalization {
            Capitalization::Auto => base,
            other => (base & !t::CAP_MASK) | other.flag(),
        }
    }

    /// The same, with a different action key.
    pub const fn action(mut self, action: TextInputAction) -> Self {
        self.action = action;
        self
    }
}

// ---------------------------------------------------------------------------
// Android's numbers
// ---------------------------------------------------------------------------

/// `android.text.InputType`, the constants this file needs.
mod input_type {
    pub const CLASS_TEXT: i32 = 0x0000_0001;
    pub const CLASS_NUMBER: i32 = 0x0000_0002;
    pub const CLASS_PHONE: i32 = 0x0000_0003;
    pub const CLASS_DATETIME: i32 = 0x0000_0004;

    pub const CLASS_MASK: i32 = 0x0000_000f;

    pub const TEXT_FLAG_CAP_CHARACTERS: i32 = 0x0000_1000;
    pub const TEXT_FLAG_CAP_SENTENCES: i32 = 0x0000_4000;
    pub const TEXT_FLAG_CAP_WORDS: i32 = 0x0000_2000;
    /// Every capitalisation bit, so that asking for one can clear the others.
    pub const CAP_MASK: i32 =
        TEXT_FLAG_CAP_CHARACTERS | TEXT_FLAG_CAP_SENTENCES | TEXT_FLAG_CAP_WORDS;
    pub const TEXT_FLAG_MULTI_LINE: i32 = 0x0002_0000;

    pub const TEXT_VARIATION_URI: i32 = 0x0000_0010;
    pub const TEXT_VARIATION_EMAIL_ADDRESS: i32 = 0x0000_0020;
    pub const TEXT_VARIATION_PASSWORD: i32 = 0x0000_0080;
    pub const TEXT_VARIATION_PERSON_NAME: i32 = 0x0000_0060;
    pub const TEXT_VARIATION_POSTAL_ADDRESS: i32 = 0x0000_0050;
    pub const TEXT_VARIATION_VISIBLE_PASSWORD: i32 = 0x0000_0090;

    pub const NUMBER_FLAG_SIGNED: i32 = 0x0000_1000;
    pub const NUMBER_FLAG_DECIMAL: i32 = 0x0000_2000;
}

/// `android.view.inputmethod.EditorInfo`, likewise.
mod editor_info {
    pub const IME_ACTION_UNSPECIFIED: i32 = 0;
    pub const IME_ACTION_GO: i32 = 2;
    pub const IME_ACTION_SEARCH: i32 = 3;
    pub const IME_ACTION_SEND: i32 = 4;
    pub const IME_ACTION_NEXT: i32 = 5;
    pub const IME_ACTION_DONE: i32 = 6;
    pub const IME_ACTION_PREVIOUS: i32 = 7;
    pub const IME_FLAG_NO_FULLSCREEN: i32 = 0x0200_0000;
}

impl KeyboardType {
    /// The Android `InputType` bit field for this keyboard.
    ///
    /// [`None`](Self::None) is `TYPE_NULL`, which is zero — the value that tells
    /// Android there is nothing to type into. It is also what the bridge produced for
    /// *every* field before it had an `InputType` at all, which is why a keyboard used
    /// to arrive with no composition, no swipe and no CJK.
    pub const fn android_input_type(self) -> i32 {
        use input_type as t;
        match self {
            Self::Text => t::CLASS_TEXT | t::TEXT_FLAG_CAP_SENTENCES,
            Self::Multiline => t::CLASS_TEXT | t::TEXT_FLAG_CAP_SENTENCES | t::TEXT_FLAG_MULTI_LINE,
            Self::Number => t::CLASS_NUMBER,
            Self::Decimal => t::CLASS_NUMBER | t::NUMBER_FLAG_DECIMAL | t::NUMBER_FLAG_SIGNED,
            Self::Phone => t::CLASS_PHONE,
            // No capitalisation flag: an address that arrives as `Someone@…` is an
            // address that does not work, and the keyboard would supply the capital.
            Self::Email => t::CLASS_TEXT | t::TEXT_VARIATION_EMAIL_ADDRESS,
            Self::Url => t::CLASS_TEXT | t::TEXT_VARIATION_URI,
            Self::Password => t::CLASS_TEXT | t::TEXT_VARIATION_PASSWORD,
            Self::VisiblePassword => t::CLASS_TEXT | t::TEXT_VARIATION_VISIBLE_PASSWORD,
            Self::Name => t::CLASS_TEXT | t::TEXT_VARIATION_PERSON_NAME | t::TEXT_FLAG_CAP_WORDS,
            Self::StreetAddress => {
                t::CLASS_TEXT | t::TEXT_VARIATION_POSTAL_ADDRESS | t::TEXT_FLAG_CAP_WORDS
            }
            Self::DateTime => t::CLASS_DATETIME,
            Self::None => 0,
        }
    }

    /// Whether the keyboard must never learn what is typed into it.
    ///
    /// Android infers this from the variation bits, so nothing else has to be sent —
    /// but the question is worth asking in this vocabulary too, because it is the
    /// reason [`Password`](Self::Password) exists as a *type* rather than as a way of
    /// drawing dots.
    pub const fn is_secret(self) -> bool {
        matches!(self, Self::Password | Self::VisiblePassword)
    }
}

impl TextInputAction {
    /// The Android `imeOptions` bit field for this action key.
    ///
    /// `IME_FLAG_NO_FULLSCREEN` is on every one of them: the fullscreen editor Android
    /// offers in landscape replaces the application's own field with a system one, and
    /// a framework that draws its own text has nothing to gain and a screen to lose.
    pub const fn android_ime_options(self) -> i32 {
        use editor_info as e;
        let action = match self {
            Self::Done => e::IME_ACTION_DONE,
            Self::Go => e::IME_ACTION_GO,
            Self::Search => e::IME_ACTION_SEARCH,
            Self::Send => e::IME_ACTION_SEND,
            Self::Next => e::IME_ACTION_NEXT,
            Self::Previous => e::IME_ACTION_PREVIOUS,
            // A newline is not an action: the key has to fall through to the editor,
            // and `UNSPECIFIED` with the multi-line input flag is how Android says so.
            Self::Newline | Self::Unspecified => e::IME_ACTION_UNSPECIFIED,
        };
        action | e::IME_FLAG_NO_FULLSCREEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pair a field with nothing said is what the bridge used to hardcode for
    /// every field: sentence-cased text, a *Done* key, no fullscreen editor. Nothing
    /// that says nothing changes.
    #[test]
    fn the_default_is_what_was_hardcoded_before() {
        let ime = Ime::default();
        assert_eq!(ime.keyboard, KeyboardType::Text);
        assert_eq!(ime.action, TextInputAction::Done);
        // TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_CAP_SENTENCES
        assert_eq!(ime.keyboard.android_input_type(), 1 | 0x4000);
        // IME_ACTION_DONE | IME_FLAG_NO_FULLSCREEN
        assert_eq!(ime.action.android_ime_options(), 6 | 0x0200_0000);
    }

    /// Asking for a capitalisation **replaces** the type's own rather than adding to it:
    /// two capitalisation bits at once is a keyboard being told two things.
    #[test]
    fn a_capitalisation_replaces_the_types_own() {
        const CAP_MASK: i32 = 0x0000_7000;
        let bits = |c: Capitalization| Ime::new(KeyboardType::Text).capitalization(c);
        // Ordinary text capitalises sentences by itself.
        assert_eq!(
            Ime::new(KeyboardType::Text).android_input_type() & CAP_MASK,
            0x0000_4000
        );
        assert_eq!(
            bits(Capitalization::Words).android_input_type() & CAP_MASK,
            0x0000_2000
        );
        assert_eq!(
            bits(Capitalization::Characters).android_input_type() & CAP_MASK,
            0x0000_1000
        );
        assert_eq!(
            bits(Capitalization::None).android_input_type() & CAP_MASK,
            0
        );
        // And the rest of the field is untouched: still a text class.
        assert_eq!(bits(Capitalization::None).android_input_type() & 0xf, 1);
    }

    /// `Auto` is the absence of an opinion, so every type answers exactly what it always
    /// did — a name still capitalises words, an email address still capitalises nothing.
    #[test]
    fn auto_leaves_every_type_as_it_was() {
        for keyboard in [
            KeyboardType::Text,
            KeyboardType::Multiline,
            KeyboardType::Name,
            KeyboardType::Email,
            KeyboardType::Password,
            KeyboardType::Number,
            KeyboardType::Phone,
        ] {
            assert_eq!(
                Ime::new(keyboard).android_input_type(),
                keyboard.android_input_type(),
                "{keyboard:?}"
            );
        }
    }

    /// Only a text class capitalises. `0x1000` is *signed* on a number class, so asking
    /// a keypad for capitals would quietly turn on a minus key.
    #[test]
    fn a_keypad_is_never_told_to_capitalise() {
        for keyboard in [
            KeyboardType::Number,
            KeyboardType::Decimal,
            KeyboardType::Phone,
            KeyboardType::DateTime,
        ] {
            let asked = Ime::new(keyboard).capitalization(Capitalization::Characters);
            assert_eq!(
                asked.android_input_type(),
                keyboard.android_input_type(),
                "{keyboard:?}"
            );
        }
    }

    /// Each type carries its own class. A phone field on a text class is a QWERTY
    /// keyboard with a wrong label.
    #[test]
    fn each_type_names_its_own_class() {
        const CLASS_MASK: i32 = 0x0000_000f;
        let class = |k: KeyboardType| k.android_input_type() & CLASS_MASK;
        assert_eq!(class(KeyboardType::Text), 1);
        assert_eq!(class(KeyboardType::Multiline), 1);
        assert_eq!(class(KeyboardType::Number), 2);
        assert_eq!(class(KeyboardType::Decimal), 2);
        assert_eq!(class(KeyboardType::Phone), 3);
        assert_eq!(class(KeyboardType::DateTime), 4);
        assert_eq!(class(KeyboardType::Email), 1);
        assert_eq!(class(KeyboardType::Url), 1);
    }

    /// `None` is `TYPE_NULL`, and it is the only zero. A type that fell through to
    /// zero by accident would silently be *no keyboard at all*, which is the one
    /// failure a user cannot work around.
    #[test]
    fn only_none_is_null() {
        let every = [
            KeyboardType::Text,
            KeyboardType::Multiline,
            KeyboardType::Number,
            KeyboardType::Decimal,
            KeyboardType::Phone,
            KeyboardType::Email,
            KeyboardType::Url,
            KeyboardType::Password,
            KeyboardType::VisiblePassword,
            KeyboardType::Name,
            KeyboardType::StreetAddress,
            KeyboardType::DateTime,
        ];
        for keyboard in every {
            assert_ne!(
                keyboard.android_input_type(),
                0,
                "{keyboard:?} must not be TYPE_NULL"
            );
        }
        assert_eq!(KeyboardType::None.android_input_type(), 0);
    }

    /// A secret carries a password **variation**, which is what stops the keyboard
    /// learning it. Masking dots are drawn by us and tell the keyboard nothing.
    #[test]
    fn a_secret_carries_a_password_variation() {
        const VARIATION_MASK: i32 = 0x0000_0ff0;
        assert_eq!(
            KeyboardType::Password.android_input_type() & VARIATION_MASK,
            0x80
        );
        assert_eq!(
            KeyboardType::VisiblePassword.android_input_type() & VARIATION_MASK,
            0x90
        );
        assert!(KeyboardType::Password.is_secret());
        assert!(KeyboardType::VisiblePassword.is_secret());
        assert!(!KeyboardType::Text.is_secret());
    }

    /// An email address is never auto-capitalised: `Someone@…` is an address that
    /// does not work, and the keyboard would supply the capital unasked.
    #[test]
    fn an_email_is_not_capitalised() {
        const CAP_MASK: i32 = 0x0000_7000;
        assert_eq!(KeyboardType::Email.android_input_type() & CAP_MASK, 0);
        assert_eq!(KeyboardType::Url.android_input_type() & CAP_MASK, 0);
        assert_eq!(KeyboardType::Password.android_input_type() & CAP_MASK, 0);
        // Ordinary prose is, and a name is capitalised per word rather than per
        // sentence.
        assert_ne!(KeyboardType::Text.android_input_type() & CAP_MASK, 0);
        assert_eq!(
            KeyboardType::Name.android_input_type() & CAP_MASK,
            0x0000_2000
        );
    }

    /// Multiline carries the multi-line flag, which is what lets the action key be a
    /// newline at all.
    #[test]
    fn multiline_carries_the_multi_line_flag() {
        const MULTI_LINE: i32 = 0x0002_0000;
        assert_ne!(KeyboardType::Multiline.android_input_type() & MULTI_LINE, 0);
        assert_eq!(KeyboardType::Text.android_input_type() & MULTI_LINE, 0);
    }

    /// A newline is `UNSPECIFIED`, not an action: the key has to reach the editor.
    #[test]
    fn a_newline_is_not_an_action() {
        const ACTION_MASK: i32 = 0x0000_00ff;
        assert_eq!(
            TextInputAction::Newline.android_ime_options() & ACTION_MASK,
            0
        );
        assert_eq!(
            TextInputAction::Search.android_ime_options() & ACTION_MASK,
            3
        );
        assert_eq!(TextInputAction::Next.android_ime_options() & ACTION_MASK, 5);
    }

    /// Every action refuses the fullscreen editor: it replaces the application's own
    /// field with a system one, and a framework that draws its own text loses a screen
    /// and gains nothing.
    #[test]
    fn no_action_accepts_the_fullscreen_editor() {
        const NO_FULLSCREEN: i32 = 0x0200_0000;
        for action in [
            TextInputAction::Done,
            TextInputAction::Go,
            TextInputAction::Search,
            TextInputAction::Send,
            TextInputAction::Next,
            TextInputAction::Previous,
            TextInputAction::Newline,
            TextInputAction::Unspecified,
        ] {
            assert_ne!(
                action.android_ime_options() & NO_FULLSCREEN,
                0,
                "{action:?} must refuse the fullscreen editor"
            );
        }
    }
}
