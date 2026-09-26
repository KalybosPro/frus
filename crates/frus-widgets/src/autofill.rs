//! What a field is **for**, so that the platform can fill it in.
//!
//! [`KeyboardType`](crate::KeyboardType) says which keys a field wants; it never says
//! what the field holds. That second thing is what a password manager and the platform's
//! autofill service read, and without it nothing happens — not loudly, just never: a
//! saved password is not offered on a sign-in screen, a code arriving by SMS is not
//! offered above the keyboard, an address is typed out by hand, and a password just
//! created is never saved, so the next sign-in fails.
//!
//! ```ignore
//! AutofillGroup::new(
//!     Flex::column()
//!         .child(TextField::new(&app.email).autofill([AutofillHint::Username, AutofillHint::Email]))
//!         .child(TextField::new(&app.password).autofill([AutofillHint::NewPassword])),
//! )
//! ```
//!
//! # A hint is a list, and a form is a group
//!
//! A field can legitimately be more than one thing — a sign-in box that takes an email
//! address is both the username and the email — so a field carries a **list** of hints,
//! in the order of how well each describes it.
//!
//! And a credential is a **pair**. A service that is shown a password with no username
//! beside it has nothing to save under, so the fields of one form are declared together:
//! [`AutofillGroup`] is what says which fields those are. A field outside any group
//! stands alone, which is right for a lone search box and wrong for a sign-in screen.
//!
//! # The names are Android's, and the tests are not
//!
//! Each hint carries the string Android's autofill service expects
//! ([`AutofillHint::android`]), and that mapping lives here rather than in the platform
//! layer — the same reasoning as [`crate::ime`]: a mapping only exercised on a device is
//! a mapping nobody checks, and the bridge's dex is checked in, so the Java side must
//! never have to change to learn a hint. It is handed strings and sets them on the
//! structure.
//!
//! The names are rarely the obvious ones — an email address is `emailAddress`, a person's
//! name is `personName`, a one-time code is `smsOTPCode` — which is exactly why a table
//! nobody can read off the constant's own name is worth writing down once.

use crate::widget::Widget;

/// What a field is for: what an autofill service should put in it, and what it should
/// learn from it when the form is submitted.
///
/// The catalogue is the useful part of the platform's, not all of it: what a sign-in, a
/// sign-up, an address and a card payment need. A hint the list does not carry is a hint
/// to add here, with its platform name and a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutofillHint {
    /// The account's name — an email address often plays this part as well.
    Username,
    /// A username being **created**, so that the service offers to save it rather than
    /// to fill it.
    NewUsername,
    /// An existing password, to be filled.
    Password,
    /// A password being **created**: never filled from what is saved, and offered for
    /// saving once the form is submitted.
    NewPassword,
    /// A one-time code, usually arriving by SMS. The platform offers it above the
    /// keyboard, which is what saves the reader leaving the application to go and read it.
    OneTimeCode,
    /// An email address.
    Email,
    /// A person's whole name.
    Name,
    /// The given (first) name alone.
    GivenName,
    /// The family (last) name alone.
    FamilyName,
    /// A telephone number.
    TelephoneNumber,
    /// The street part of an address — the lines above the town.
    StreetAddress,
    /// A whole postal address in one field.
    PostalAddress,
    /// The postal code.
    PostalCode,
    /// The town or city.
    AddressCity,
    /// The state, region or county.
    AddressState,
    /// The country.
    CountryName,
    /// A payment card's number.
    CreditCardNumber,
    /// Its expiry date, as one field.
    CreditCardExpirationDate,
    /// Its security code (CVC).
    CreditCardSecurityCode,
}

impl AutofillHint {
    /// The string Android's autofill services expect for this hint.
    ///
    /// Several are not what the hint is called: `personName`, `emailAddress`,
    /// `phoneNumber`, `addressLocality` for a town, `addressRegion` for a state,
    /// `addressCountry` for a country, `smsOTPCode` for a one-time code.
    pub const fn android(self) -> &'static str {
        match self {
            Self::Username => "username",
            Self::NewUsername => "newUsername",
            Self::Password => "password",
            Self::NewPassword => "newPassword",
            Self::OneTimeCode => "smsOTPCode",
            Self::Email => "emailAddress",
            Self::Name => "personName",
            Self::GivenName => "personGivenName",
            Self::FamilyName => "personFamilyName",
            Self::TelephoneNumber => "phoneNumber",
            Self::StreetAddress => "streetAddress",
            Self::PostalAddress => "postalAddress",
            Self::PostalCode => "postalCode",
            Self::AddressCity => "addressLocality",
            Self::AddressState => "addressRegion",
            Self::CountryName => "addressCountry",
            Self::CreditCardNumber => "creditCardNumber",
            Self::CreditCardExpirationDate => "creditCardExpirationDate",
            Self::CreditCardSecurityCode => "creditCardSecurityCode",
        }
    }

    /// Is what this field holds a **secret**, so that it must never be shown in a
    /// screenshot of the structure, nor learned by a keyboard?
    ///
    /// The field's [`KeyboardType`](crate::KeyboardType) already stops the keyboard
    /// learning it; this is the same question asked of the *hint*, for the paths that
    /// only ever see one — reporting a value to the autofill service, above all.
    pub const fn is_secret(self) -> bool {
        matches!(self, Self::Password | Self::NewPassword | Self::OneTimeCode)
    }
}

/// The fields of **one form**, declared together so that the platform can fill them and
/// save them as a set.
///
/// A transparent wrapper: it changes nothing about how its subtree is laid out or
/// painted. What it does is answer, for every field inside it, *which form is this part
/// of* — and a password with no username beside it is a credential a service cannot save.
///
/// It is the reference's `AutofillGroup`.
pub struct AutofillGroup<Msg = crate::callback::Callback> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> AutofillGroup<Msg> {
    /// Declares `inner`'s fields to be one form.
    pub fn new(inner: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// A group is not a box: its child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(AutofillGroup {
    /// The one thing it does: name itself as the form its fields belong to.
    fn autofill_group(&self) -> bool {
        true
    }
    /// Forwarded: the area's bar is the area's, whatever wraps it.
    fn area_toolbar(
        &self,
        context: crate::ToolbarContext,
    ) -> Option<Option<&dyn crate::widget::Widget<Msg>>> {
        self.inner.area_toolbar(context)
    }

    /// A form is not an identity: the child's key, if it has one.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }

    /// Forwarded, as for every transparent wrapper: a form is not a place, not a theme,
    /// and not a surface.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, TextField};

    /// Every hint this crate offers, for the tests that must cover all of them.
    const EVERY: [AutofillHint; 19] = [
        AutofillHint::Username,
        AutofillHint::NewUsername,
        AutofillHint::Password,
        AutofillHint::NewPassword,
        AutofillHint::OneTimeCode,
        AutofillHint::Email,
        AutofillHint::Name,
        AutofillHint::GivenName,
        AutofillHint::FamilyName,
        AutofillHint::TelephoneNumber,
        AutofillHint::StreetAddress,
        AutofillHint::PostalAddress,
        AutofillHint::PostalCode,
        AutofillHint::AddressCity,
        AutofillHint::AddressState,
        AutofillHint::CountryName,
        AutofillHint::CreditCardNumber,
        AutofillHint::CreditCardExpirationDate,
        AutofillHint::CreditCardSecurityCode,
    ];

    /// A hint whose platform name is wrong is a field the service silently declines to
    /// fill, so each one is written down and checked. The names that are **not** the
    /// hint's own are the ones worth the test: a service reading `email` or `name` fills
    /// nothing at all.
    #[test]
    fn each_hint_carries_the_platforms_own_name() {
        assert_eq!(AutofillHint::Email.android(), "emailAddress");
        assert_eq!(AutofillHint::Name.android(), "personName");
        assert_eq!(AutofillHint::GivenName.android(), "personGivenName");
        assert_eq!(AutofillHint::FamilyName.android(), "personFamilyName");
        assert_eq!(AutofillHint::TelephoneNumber.android(), "phoneNumber");
        assert_eq!(AutofillHint::OneTimeCode.android(), "smsOTPCode");
        assert_eq!(AutofillHint::AddressCity.android(), "addressLocality");
        assert_eq!(AutofillHint::AddressState.android(), "addressRegion");
        assert_eq!(AutofillHint::CountryName.android(), "addressCountry");
        // And the ones that are their own name are still named, so that adding a hint
        // without a mapping cannot pass by defaulting to something plausible.
        assert_eq!(AutofillHint::Username.android(), "username");
        assert_eq!(AutofillHint::NewPassword.android(), "newPassword");
    }

    /// No two hints may share a name — two fields carrying the same one is a form the
    /// service fills twice with the same value.
    #[test]
    fn no_two_hints_share_a_name() {
        let mut seen: Vec<&str> = EVERY.iter().map(|h| h.android()).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "a name is used twice");
        assert!(EVERY.iter().all(|h| !h.android().is_empty()));
    }

    /// What must never be reported in the clear, nor learned: the passwords and the
    /// one-time code. An email address is not a secret, and treating it as one would
    /// stop a service ever offering it.
    #[test]
    fn the_secrets_are_the_passwords_and_the_code() {
        for hint in [
            AutofillHint::Password,
            AutofillHint::NewPassword,
            AutofillHint::OneTimeCode,
        ] {
            assert!(hint.is_secret(), "{hint:?}");
        }
        for hint in [
            AutofillHint::Username,
            AutofillHint::Email,
            AutofillHint::CreditCardNumber,
        ] {
            assert!(!hint.is_secret(), "{hint:?}");
        }
    }

    /// A group says what it is and delegates the rest; the fields inside keep their own
    /// hints, which is what the shell reads.
    #[test]
    fn a_group_names_itself_and_changes_nothing_else() {
        let field = TextField::<()>::new("someone@example.com")
            .autofill([AutofillHint::Username, AutofillHint::Email]);
        let group = AutofillGroup::new(Container::<()>::new().child(field));
        assert!(Widget::<()>::autofill_group(&group));
        assert_eq!(Widget::<()>::children(&group).len(), 1);
        // And nothing that is not a group answers that it is.
        assert!(!Widget::<()>::autofill_group(&Container::<()>::new()));
    }

    /// A wrapper that fuses with its field shares its identity, so it is the wrapper the
    /// shell asks what the field is for — and one that forgot to pass the question on
    /// would take the field out of every form without a word. `Responsive` is written by
    /// hand, and has already cost four silent bugs of exactly this kind.
    #[test]
    fn the_wrappers_that_fuse_with_a_field_pass_its_hints_on() {
        let field = || TextField::<()>::new("").autofill([AutofillHint::OneTimeCode]);
        let expected = [AutofillHint::OneTimeCode];
        let keyed = crate::Keyed::new(1, field());
        let responsive = crate::Responsive::new(crate::SizeClass::Compact).compact(field());
        let boxed: Box<dyn Widget<()>> = Box::new(field());
        assert_eq!(Widget::<()>::autofill_hints(&keyed), &expected);
        assert_eq!(Widget::<()>::autofill_hints(&responsive), &expected);
        assert_eq!(boxed.autofill_hints(), &expected);
        // And a group behind a selector, a key or a box is still a group.
        let grouped = crate::Responsive::new(crate::SizeClass::Compact)
            .compact(AutofillGroup::new(Container::<()>::new()));
        assert!(Widget::<()>::autofill_group(&grouped));
        let keyed = crate::Keyed::new(2, AutofillGroup::new(Container::<()>::new()));
        assert!(Widget::<()>::autofill_group(&keyed));
        let boxed: Box<dyn Widget<()>> = Box::new(AutofillGroup::new(Container::<()>::new()));
        assert!(boxed.autofill_group());
    }

    /// A field carries its hints in the order it was given them: the first is what the
    /// field mostly is, and a service that understands only one reads that one.
    #[test]
    fn a_field_keeps_the_hints_it_was_given_in_order() {
        let field =
            TextField::<()>::new("").autofill([AutofillHint::Username, AutofillHint::Email]);
        assert_eq!(
            Widget::<()>::autofill_hints(&field),
            &[AutofillHint::Username, AutofillHint::Email]
        );
        // A field nobody described carries none, and is not part of any structure.
        assert!(Widget::<()>::autofill_hints(&TextField::<()>::new("")).is_empty());
    }
}
