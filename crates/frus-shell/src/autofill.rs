//! Autofill (milestone 512): which fields of a form the platform's autofill service is
//! shown, under which ids, and where the values it hands back go.
//!
//! Pure, like `ime.rs`: the platform half (`android_autofill.rs`) only carries these
//! answers across, so every rule is testable away from a device. The geometry and the
//! form come from the frame ([`frus_widgets::Ui::form_of`]); what each field is for and
//! what it holds come from the widgets, through the `describe` a caller passes.

use frus_widgets::{AutofillHint, Rect, WidgetId};

/// One field of a form, as the autofill service is shown it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AutofillField {
    /// The field it stands for.
    pub widget: WidgetId,
    /// The id the platform knows it by — see [`virtual_id`].
    pub virtual_id: i32,
    /// What it is for, in the platform's names, most telling first.
    pub hints: Vec<&'static str>,
    /// Its box in **physical** pixels from the window content's corner: left, top,
    /// width, height. The platform anchors its offer to the focused one.
    pub bounds: [i32; 4],
    /// What it holds now.
    pub value: String,
    /// A secret — a password, a one-time code. The service is still given the value, or
    /// there would be nothing to save; it is told the value is sensitive, which keeps it
    /// out of anything the platform logs or shows about the structure.
    pub sensitive: bool,
}

/// The id the platform knows a field by: a non-negative `i32` folded from its identity.
///
/// Identities are positions in the tree, stable from one frame to the next, so the same
/// field keeps the same id for as long as it stays where it is — which is what lets a
/// value handed back several frames later still find it. Folded rather than truncated,
/// so that two identities differing only in their high half do not collide.
pub(crate) fn virtual_id(id: WidgetId) -> i32 {
    let raw = id.as_u64();
    ((raw ^ (raw >> 32)) & 0x7fff_ffff) as i32
}

/// The form as the service is shown it: of `stops` — the form's focus stops in tree
/// order, each with its logical box — the ones that say what they are for, at `scale`
/// physical pixels per logical one.
///
/// `describe` answers a stop's hints and its current value, `None` for a stop that is
/// not in the tree. A stop with no hints takes no part: a service shown a field it knows
/// nothing about guesses, and a guess in a sign-in form is a password put in the wrong box.
pub(crate) fn structure<'a>(
    stops: &[(WidgetId, Rect)],
    scale: f32,
    describe: impl Fn(WidgetId) -> Option<(&'a [AutofillHint], String)>,
) -> Vec<AutofillField> {
    stops
        .iter()
        .filter_map(|&(widget, rect)| {
            let (hints, value) = describe(widget)?;
            if hints.is_empty() {
                return None;
            }
            let px = |v: f32| (v * scale).round() as i32;
            Some(AutofillField {
                widget,
                virtual_id: virtual_id(widget),
                hints: hints.iter().map(|h| h.android()).collect(),
                bounds: [
                    px(rect.x),
                    px(rect.y),
                    px(rect.width).max(1),
                    px(rect.height).max(1),
                ],
                value,
                sensitive: hints.iter().any(|h| h.is_secret()),
            })
        })
        .collect()
}

/// The values the service handed back, each routed to the field it names. A value for
/// an id the form no longer has is dropped: the field it was meant for has gone, and
/// putting it anywhere else would be worse than putting it nowhere.
pub(crate) fn route(
    fields: &[AutofillField],
    values: Vec<(i32, String)>,
) -> Vec<(WidgetId, String)> {
    values
        .into_iter()
        .filter_map(|(id, value)| {
            fields
                .iter()
                .find(|f| f.virtual_id == id)
                .map(|f| (f.widget, value))
        })
        .collect()
}

/// The form to publish: the fields on screen, then those of the **same form** shown before
/// and gone since — the step of a wizard behind the one showing — with the value each last
/// held, and a box of one pixel at the corner, since it is not on screen anywhere.
///
/// A form in steps shows its fields a few at a time, and a service handed each step alone
/// sees half a credential at a time: a name and an email with no password, then two
/// passwords with no account. It saves neither half. Seen on a device, with the demo's
/// sign-up wizard; the reference reports fields that are not the one being edited the same
/// way, with what they last held and one pixel for a box.
pub(crate) fn with_gone(
    mut current: Vec<AutofillField>,
    shown: &[AutofillField],
) -> Vec<AutofillField> {
    for field in shown {
        if current.iter().all(|f| f.virtual_id != field.virtual_id) {
            current.push(AutofillField {
                bounds: [0, 0, 1, 1],
                ..field.clone()
            });
        }
    }
    current
}

/// The fields whose value is not what the service was last told, with what they hold
/// now — what `notifyValueChanged` must hear. A service remembers the values it was
/// shown, and saves those: a password typed after the structure was taken, and never
/// reported, is saved as what it was then, which is nothing.
pub(crate) fn changed(reported: &[AutofillField], now: &[AutofillField]) -> Vec<(i32, String)> {
    now.iter()
        .filter(|field| {
            reported
                .iter()
                .find(|r| r.virtual_id == field.virtual_id)
                .is_none_or(|r| r.value != field.value)
        })
        .map(|field| (field.virtual_id, field.value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(raw: u64) -> WidgetId {
        WidgetId::from_u64(raw)
    }

    /// A sign-in form and a stop that is not a field of it, as a frame would give them.
    fn form() -> Vec<(WidgetId, Rect)> {
        vec![
            (id(11), Rect::new(16.0, 100.0, 300.0, 56.0)),
            (id(12), Rect::new(16.0, 170.0, 300.0, 56.0)),
            (id(13), Rect::new(16.0, 240.0, 120.0, 40.0)),
        ]
    }

    const USERNAME: [AutofillHint; 2] = [AutofillHint::Username, AutofillHint::Email];
    const PASSWORD: [AutofillHint; 1] = [AutofillHint::Password];

    /// What the tree would answer: a username, a password, and a button with no hints.
    fn describe(widget: WidgetId) -> Option<(&'static [AutofillHint], String)> {
        match widget.as_u64() {
            11 => Some((&USERNAME, "someone@example.com".into())),
            12 => Some((&PASSWORD, "hunter2".into())),
            13 => Some((&[], String::new())),
            _ => None,
        }
    }

    #[test]
    fn only_the_fields_that_say_what_they_are_for_are_shown() {
        let fields = structure(&form(), 1.0, describe);
        assert_eq!(
            fields.iter().map(|f| f.widget).collect::<Vec<_>>(),
            vec![id(11), id(12)],
            "the button is not a field of the form, and the order is the form's"
        );
        assert_eq!(fields[0].hints, vec!["username", "emailAddress"]);
        assert_eq!(fields[1].hints, vec!["password"]);
        // A stop the tree no longer has is not shown either.
        let gone = [(id(99), Rect::new(0.0, 0.0, 10.0, 10.0))];
        assert!(structure(&gone, 1.0, describe).is_empty());
    }

    /// The platform anchors its offer in physical pixels; a box handed over in logical
    /// ones puts the suggestion a third of the screen away on a dense phone.
    #[test]
    fn a_box_is_given_in_physical_pixels() {
        let fields = structure(&form(), 2.625, describe);
        assert_eq!(fields[1].bounds, [42, 446, 788, 147]);
        // And never smaller than a pixel, which a service may take for invisible.
        let thin = [(id(11), Rect::new(0.0, 0.0, 0.1, 0.0))];
        assert_eq!(structure(&thin, 1.0, describe)[0].bounds[2..], [1, 1]);
    }

    /// A password is marked sensitive and still carries its value — there would be
    /// nothing to save otherwise. A username is neither.
    #[test]
    fn a_secret_is_marked_sensitive_and_still_carries_its_value() {
        let fields = structure(&form(), 1.0, describe);
        assert!(!fields[0].sensitive);
        assert!(fields[1].sensitive);
        assert_eq!(fields[1].value, "hunter2");
    }

    #[test]
    fn a_field_keeps_its_id_and_two_fields_do_not_share_one() {
        assert_eq!(virtual_id(id(11)), virtual_id(id(11)));
        assert_ne!(virtual_id(id(11)), virtual_id(id(12)));
        // Identities that differ only in their high half are still two fields.
        assert_ne!(virtual_id(id(5)), virtual_id(id(5 | (1 << 40))));
        // Never negative, whatever the identity.
        assert!(virtual_id(id(u64::MAX)) >= 0);
    }

    #[test]
    fn a_value_goes_to_the_field_it_names_and_nowhere_else() {
        let fields = structure(&form(), 1.0, describe);
        let routed = route(
            &fields,
            vec![
                (virtual_id(id(12)), "from the vault".into()),
                (virtual_id(id(11)), "saved@example.com".into()),
                (virtual_id(id(42)), "for a field that has gone".into()),
            ],
        );
        assert_eq!(
            routed,
            vec![
                (id(12), "from the vault".to_string()),
                (id(11), "saved@example.com".to_string()),
            ]
        );
    }

    /// A form in steps is still one form: the step showing comes first, the step behind it
    /// follows with what it last held, placed nowhere in particular — and a field on screen
    /// is never repeated, nor loses its box to a stale copy of itself.
    #[test]
    fn a_step_behind_the_one_showing_is_still_part_of_the_form() {
        let all = structure(&form(), 1.0, describe);
        let (account, security) = (vec![all[0].clone()], vec![all[1].clone()]);
        let merged = with_gone(security, &account);
        assert_eq!(
            merged.iter().map(|f| f.widget).collect::<Vec<_>>(),
            vec![id(12), id(11)],
            "the step showing, then the one behind it"
        );
        assert_eq!(
            merged[1].value, "someone@example.com",
            "with what it last held"
        );
        assert_eq!(merged[1].hints, vec!["username", "emailAddress"]);
        assert_eq!(merged[1].bounds, [0, 0, 1, 1], "and not on screen anywhere");
        assert_eq!(
            merged[0].bounds, all[1].bounds,
            "the field showing keeps its box"
        );
        assert_eq!(
            with_gone(all.clone(), &all),
            all,
            "nothing on screen is repeated"
        );
    }

    /// Only what moved is reported, and a field the service was never shown counts as
    /// moved — it has never heard its value at all.
    #[test]
    fn only_a_value_the_service_has_not_heard_is_reported() {
        let reported = structure(&form(), 1.0, describe);
        let mut now = reported.clone();
        assert!(changed(&reported, &now).is_empty(), "nothing typed");
        now[1].value = "hunter22".into();
        assert_eq!(
            changed(&reported, &now),
            vec![(virtual_id(id(12)), "hunter22".to_string())]
        );
        let unseen = changed(&reported[..1], &reported);
        assert_eq!(unseen, vec![(virtual_id(id(12)), "hunter2".to_string())]);
    }
}
