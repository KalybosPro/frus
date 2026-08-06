//! [`DatePicker`] : un calendrier mensuel **contrôlé**, bâti sur [`crate::Grid`].
//! Calcul de date **maison** (aucune dépendance temporelle).

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::grid::Grid;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

const CELL: f32 = 34.0;
const SIZE: f32 = 15.0;
/// Écart entre les deux mois d'un calendrier double.
const DUAL_GAP: f32 = 24.0;

/// Vrai si `year` est bissextile.
fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Nombre de jours du mois `month` (1..=12) de `year`.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Jour de la semaine (`0` = dimanche … `6` = samedi) du 1er du mois (Sakamoto).
fn first_weekday(year: i32, month: u32) -> usize {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let m = month as usize;
    (((y + y / 4 - y / 100 + y / 400 + T[m - 1] + 1) % 7 + 7) % 7) as usize
}

/// État d'une case-jour vis-à-vis de la sélection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DayMark {
    /// Ni sélectionné ni dans une plage.
    Off,
    /// Jour sélectionné (mode simple).
    Selected,
    /// Borne **début** d'une plage.
    Start,
    /// Borne **fin** d'une plage.
    End,
    /// Jour **entre** les deux bornes (bande de liaison).
    Between,
}

/// Marqueur de plage d'une `date` `(année, mois, jour)` vis-à-vis de `[start, end]` :
/// borne début/fin, jour intermédiaire, ou hors plage. Les dates se comparent par tuple
/// (année, puis mois, puis jour), donc l'ordre chronologique est celui de `<`.
fn range_mark(
    date: (i32, u32, u32),
    start: Option<(i32, u32, u32)>,
    end: Option<(i32, u32, u32)>,
) -> DayMark {
    match (start, end) {
        (Some(s), _) if date == s => DayMark::Start,
        (_, Some(e)) if date == e => DayMark::End,
        (Some(s), Some(e)) if s < date && date < e => DayMark::Between,
        _ => DayMark::Off,
    }
}

/// Une case-jour cliquable (vide si `day == 0`).
struct Day<Msg> {
    day: u32,
    mark: DayMark,
    /// Jour **désactivé** (hors bornes) : atténué, non cliquable — jalon 231.
    disabled: bool,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Day<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(CELL),
            height: Dimension::Length(CELL),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        if self.day == 0 {
            return; // case de remplissage
        }
        let o = status.opacity;
        // Jour désactivé (hors bornes) : chiffre atténué, ni fond ni bande (jalon 231).
        if self.disabled {
            let label = self.day.to_string();
            let w = frus_text::measure(&label, SIZE).width;
            scene.text(
                Point::new(
                    bounds.x + (CELL - w) * 0.5,
                    bounds.y + (CELL - frus_text::line_height(SIZE)) * 0.5,
                ),
                label,
                SIZE,
                theme.muted.fade(o * 0.4),
            );
            return;
        }
        // Bande de plage (fond doux, coins carrés pour se toucher entre cases voisines).
        let band = theme.primary.fade(0.18 * o);
        let half = Rect::new(bounds.x, bounds.y, bounds.width * 0.5, bounds.height);
        let right = Rect::new(
            bounds.x + bounds.width * 0.5,
            bounds.y,
            bounds.width * 0.5,
            bounds.height,
        );
        match self.mark {
            // La borne peint sa demi-bande côté intérieur pour rejoindre les jours entre.
            DayMark::Start => scene.draw_rect(right, band, 0.0, 0.0, Color::TRANSPARENT),
            DayMark::End => scene.draw_rect(half, band, 0.0, 0.0, Color::TRANSPARENT),
            DayMark::Between => scene.draw_rect(bounds, band, 0.0, 0.0, Color::TRANSPARENT),
            _ => {}
        }
        // Fond de la case (pastille pleine pour une borne/sélection, survol sinon).
        let (bg, fg) = match self.mark {
            DayMark::Selected | DayMark::Start | DayMark::End => (theme.primary, theme.on_primary),
            _ => {
                let hovered = theme.state_layer(theme.surface, theme.on_surface, &status);
                (hovered, theme.on_surface)
            }
        };
        if self.mark != DayMark::Between {
            scene.draw_rect(bounds, bg.fade(o), CELL * 0.5, 0.0, Color::TRANSPARENT);
        }
        let label = self.day.to_string();
        let w = frus_text::measure(&label, SIZE).width;
        scene.text(
            Point::new(
                bounds.x + (CELL - w) * 0.5,
                bounds.y + (CELL - frus_text::line_height(SIZE)) * 0.5,
            ),
            label,
            SIZE,
            fg.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }
}

/// Un calendrier mensuel (ou une paire de mois en mode [`range_dual`](DatePicker::range_dual)).
pub struct DatePicker<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// Deux mois côte à côte (double sa largeur).
    dual: bool,
}

const WEEKDAYS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl<Msg: Clone + 'static> DatePicker<Msg> {
    /// Crée un calendrier pour `year`/`month` (1..=12), avec le jour `selected`
    /// éventuel. `on_select(jour)` au clic ; `on_nav(±1)` pour changer de mois.
    pub fn new(
        year: i32,
        month: u32,
        selected: Option<u32>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| {
                if selected == Some(day) {
                    DayMark::Selected
                } else {
                    DayMark::Off
                }
            },
            |_| true,
        )
    }

    /// Calendrier simple **filtré par prédicat** : un jour `(année, mois, jour)` est cliquable ssi
    /// `is_enabled(date)` est vrai — les autres sont **désactivés** (atténués, non cliquables). C'est
    /// l'escape hatch général (façon `selectableDayPredicate` de Flutter) : jours **blackout**
    /// isolés, week-ends interdits, bornes, ou toute combinaison. `selected`/`on_select`/`on_nav`
    /// comme [`new`](Self::new) — jalon 235.
    pub fn filtered(
        year: i32,
        month: u32,
        selected: Option<u32>,
        is_enabled: impl Fn((i32, u32, u32)) -> bool + 'static,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| {
                if selected == Some(day) {
                    DayMark::Selected
                } else {
                    DayMark::Off
                }
            },
            move |day| is_enabled((year, month, day)),
        )
    }

    /// Calendrier simple **borné** : les jours hors `[min, max]` (dates `(année, mois, jour)`,
    /// bornes optionnelles et **incluses**) sont **désactivés** — atténués et non cliquables.
    /// Identique à [`new`](Self::new) pour le reste (jour `selected`, `on_select`, `on_nav`).
    /// Utile pour interdire les dates passées, une fenêtre de réservation, etc. — jalon 231.
    pub fn bounded(
        year: i32,
        month: u32,
        selected: Option<u32>,
        min: Option<(i32, u32, u32)>,
        max: Option<(i32, u32, u32)>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| {
                if selected == Some(day) {
                    DayMark::Selected
                } else {
                    DayMark::Off
                }
            },
            move |day| {
                let date = (year, month, day);
                min.map_or(true, |m| date >= m) && max.map_or(true, |m| date <= m)
            },
        )
    }

    /// Calendrier en **mode plage** : met en avant l'intervalle `[start, end]` (dates
    /// `(année, mois, jour)`) — bornes en pastille pleine, jours entre en bande douce. Une seule
    /// borne (`end == None`) affiche juste le début (sélection en cours). `on_select(jour)`
    /// rapporte le jour cliqué du mois affiché — l'application décide s'il devient début ou fin.
    pub fn range(
        year: i32,
        month: u32,
        start: Option<(i32, u32, u32)>,
        end: Option<(i32, u32, u32)>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| range_mark((year, month, day), start, end),
            |_| true,
        )
    }

    /// Calendrier en **mode plage borné** : comme [`range`](Self::range), mais les jours hors
    /// `[min, max]` (bornes optionnelles et **incluses**) sont **désactivés** — atténués et non
    /// cliquables. Combine la mise en avant d'un intervalle sélectionné et une fenêtre de saisie
    /// autorisée (p.ex. réserver une plage dans une période ouverte) — jalon 234.
    #[allow(clippy::too_many_arguments)]
    pub fn range_bounded(
        year: i32,
        month: u32,
        start: Option<(i32, u32, u32)>,
        end: Option<(i32, u32, u32)>,
        min: Option<(i32, u32, u32)>,
        max: Option<(i32, u32, u32)>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| range_mark((year, month, day), start, end),
            move |day| {
                let date = (year, month, day);
                min.map_or(true, |m| date >= m) && max.map_or(true, |m| date <= m)
            },
        )
    }

    /// Calendrier **double** : le mois `year`/`month` et le **suivant**, côte à côte, partageant
    /// la même plage `[start, end]` — pour saisir de longues plages sans changer de mois.
    /// `on_select((année, mois, jour))` rapporte la date **complète** du jour cliqué (le mois est
    /// désambiguïsé), `on_nav(±1)` décale la **paire**. La bande de plage se poursuit d'un mois
    /// à l'autre (les dates se comparent en entier — cf. [`range_mark`]).
    pub fn range_dual(
        year: i32,
        month: u32,
        start: Option<(i32, u32, u32)>,
        end: Option<(i32, u32, u32)>,
        on_select: impl Fn((i32, u32, u32)) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        let (ny, nm) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        // Partagés entre les deux mois.
        let on_select = std::rc::Rc::new(on_select);
        let on_nav = std::rc::Rc::new(on_nav);
        let (os1, os2) = (on_select.clone(), on_select);
        let (nv1, nv2) = (on_nav.clone(), on_nav);
        let left = DatePicker::range(
            year,
            month,
            start,
            end,
            move |d| os1((year, month, d)),
            move |n| nv1(n),
        );
        let right = DatePicker::range(
            ny,
            nm,
            start,
            end,
            move |d| os2((ny, nm, d)),
            move |n| nv2(n),
        );
        let row = Flex::row().gap(DUAL_GAP).child(left).child(right);
        Self {
            children: vec![Box::new(row)],
            dual: true,
        }
    }

    /// Assemble l'en-tête, la ligne des jours de semaine et la grille ; `mark_of(jour)` décide de
    /// l'état de chaque case (sélection simple ou plage).
    fn assemble(
        year: i32,
        month: u32,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
        mark_of: impl Fn(u32) -> DayMark,
        enabled: impl Fn(u32) -> bool,
    ) -> Self {
        let month = month.clamp(1, 12);

        // En-tête : ‹ Mois Année ›.
        let header = Flex::row()
            .align(Align::Center)
            .gap(8.0)
            .child(
                Button::new("‹")
                    .variant(Variant::Secondary)
                    .size(15.0)
                    .on_press(on_nav(-1)),
            )
            .child(Flex::row().flex(1.0))
            .child(Text::new(format!("{} {}", MONTHS[(month - 1) as usize], year)).size(16.0))
            .child(Flex::row().flex(1.0))
            .child(
                Button::new("›")
                    .variant(Variant::Secondary)
                    .size(15.0)
                    .on_press(on_nav(1)),
            );

        // Ligne des jours de la semaine.
        let mut weekdays = Grid::new(7).gap(2.0);
        for wd in WEEKDAYS {
            weekdays = weekdays.cell(WeekdayCell {
                label: wd.to_string(),
            });
        }

        // Grille des jours (cases vides avant le 1er).
        let lead = first_weekday(year, month);
        let total = days_in_month(year, month);
        let mut grid = Grid::new(7).gap(2.0);
        for _ in 0..lead {
            grid = grid.cell(Day::<Msg> {
                day: 0,
                mark: DayMark::Off,
                disabled: false,
                message: None,
            });
        }
        for day in 1..=total {
            let on = enabled(day);
            grid = grid.cell(Day {
                day,
                mark: mark_of(day),
                disabled: !on,
                message: if on { Some(on_select(day)) } else { None },
            });
        }

        Self {
            children: vec![Box::new(header), Box::new(weekdays), Box::new(grid)],
            dual: false,
        }
    }
}

/// Un en-tête de jour de semaine (non cliquable).
struct WeekdayCell {
    label: String,
}

impl<Msg> Widget<Msg> for WeekdayCell {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(22.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let w = frus_text::measure(&self.label, 13.0).width;
        scene.text(
            Point::new(bounds.x + (CELL - w) * 0.5, bounds.y),
            self.label.clone(),
            13.0,
            theme.muted.fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

impl<Msg: Clone> Widget<Msg> for DatePicker<Msg> {
    fn style(&self) -> Style {
        let month_w = 7.0 * (CELL + 2.0);
        let width = if self.dual {
            2.0 * month_w + DUAL_GAP
        } else {
            month_w
        };
        Style {
            width: Dimension::Length(width),
            flex_direction: FlexDirection::Column,
            gap: 8.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pick(u32),
        Nav(i32),
    }

    #[test]
    fn date_math_is_correct() {
        assert_eq!(days_in_month(2024, 2), 29); // bissextile
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        // 1er janvier 2024 = lundi (1).
        assert_eq!(first_weekday(2024, 1), 1);
        // 1er juillet 2026 = mercredi (3).
        assert_eq!(first_weekday(2026, 7), 3);
    }

    #[test]
    fn builds_header_weekdays_and_grid() {
        let dp = DatePicker::new(2026, 7, Some(11), Msg::Pick, Msg::Nav);
        // [en-tête, jours de semaine, grille].
        assert_eq!(Widget::<Msg>::children(&dp).len(), 3);
        // La grille = cases de remplissage + 31 jours ; juillet 2026 commence mercredi
        // (3 cases vides) → 3 + 31 = 34 cellules.
        let grid = &Widget::<Msg>::children(&dp)[2];
        assert_eq!(grid.children().len(), 34);
    }

    #[test]
    fn bounded_disables_days_outside_the_range() {
        // Fenêtre [10, 20] juillet 2026 : hors bornes = non cliquable, dans les bornes = cliquable.
        let dp = DatePicker::bounded(
            2026,
            7,
            None,
            Some((2026, 7, 10)),
            Some((2026, 7, 20)),
            Msg::Pick,
            Msg::Nav,
        );
        let grid = &Widget::<Msg>::children(&dp)[2];
        // Juillet 2026 commence mercredi (3 cases vides) ; le jour `d` est à l'index 3 + (d - 1).
        let at = |d: u32| grid.children()[3 + (d - 1) as usize].on_click();
        assert_eq!(at(9), None, "9 juillet désactivé (avant min)");
        assert_eq!(
            at(10),
            Some(Msg::Pick(10)),
            "10 juillet cliquable (borne min incluse)"
        );
        assert_eq!(
            at(15),
            Some(Msg::Pick(15)),
            "15 juillet cliquable (intérieur)"
        );
        assert_eq!(
            at(20),
            Some(Msg::Pick(20)),
            "20 juillet cliquable (borne max incluse)"
        );
        assert_eq!(at(21), None, "21 juillet désactivé (après max)");
        // Sans borne max : tout ce qui est >= min est cliquable.
        let open = DatePicker::bounded(
            2026,
            7,
            None,
            Some((2026, 7, 10)),
            None,
            Msg::Pick,
            Msg::Nav,
        );
        let g2 = &Widget::<Msg>::children(&open)[2];
        assert_eq!(
            g2.children()[3 + 30].on_click(),
            Some(Msg::Pick(31)),
            "31 juillet cliquable (pas de max)"
        );
    }

    #[test]
    fn filtered_disables_days_by_predicate() {
        // Blackout : les 12 et 18 juillet indisponibles ; tout le reste cliquable.
        let blackout = [(2026, 7, 12), (2026, 7, 18)];
        let dp = DatePicker::filtered(
            2026,
            7,
            None,
            move |date| !blackout.contains(&date),
            Msg::Pick,
            Msg::Nav,
        );
        let grid = &Widget::<Msg>::children(&dp)[2];
        let at = |d: u32| grid.children()[3 + (d - 1) as usize].on_click();
        assert_eq!(at(12), None, "12 juillet en blackout");
        assert_eq!(at(18), None, "18 juillet en blackout");
        assert_eq!(at(13), Some(Msg::Pick(13)), "13 juillet cliquable");
        assert_eq!(at(1), Some(Msg::Pick(1)), "1er juillet cliquable");
    }

    #[test]
    fn range_bounded_disables_days_outside_the_window() {
        // Plage 10..15 sélectionnée, mais fenêtre de saisie autorisée [8, 20] juillet 2026.
        let dp = DatePicker::range_bounded(
            2026,
            7,
            Some((2026, 7, 10)),
            Some((2026, 7, 15)),
            Some((2026, 7, 8)),
            Some((2026, 7, 20)),
            Msg::Pick,
            Msg::Nav,
        );
        let grid = &Widget::<Msg>::children(&dp)[2];
        let at = |d: u32| grid.children()[3 + (d - 1) as usize].on_click();
        assert_eq!(at(7), None, "7 juillet hors fenêtre (avant min)");
        assert_eq!(at(8), Some(Msg::Pick(8)), "8 juillet cliquable (borne min)");
        assert_eq!(
            at(12),
            Some(Msg::Pick(12)),
            "12 juillet cliquable (dans la plage)"
        );
        assert_eq!(
            at(20),
            Some(Msg::Pick(20)),
            "20 juillet cliquable (borne max)"
        );
        assert_eq!(at(21), None, "21 juillet hors fenêtre (après max)");
    }

    #[test]
    fn range_marks_endpoints_and_interior() {
        let start = Some((2026, 7, 10));
        let end = Some((2026, 7, 15));
        assert_eq!(range_mark((2026, 7, 10), start, end), DayMark::Start);
        assert_eq!(range_mark((2026, 7, 15), start, end), DayMark::End);
        assert_eq!(range_mark((2026, 7, 12), start, end), DayMark::Between);
        assert_eq!(
            range_mark((2026, 7, 9), start, end),
            DayMark::Off,
            "avant le début"
        );
        assert_eq!(
            range_mark((2026, 7, 16), start, end),
            DayMark::Off,
            "après la fin"
        );
        // Traverse les frontières de mois : juin est « entre » juin-15 et août-01.
        let cross = (Some((2026, 6, 15)), Some((2026, 8, 1)));
        assert_eq!(
            range_mark((2026, 7, 20), cross.0, cross.1),
            DayMark::Between
        );
        assert_eq!(range_mark((2026, 5, 31), cross.0, cross.1), DayMark::Off);
        // Sélection en cours (une seule borne) : seul le début est marqué.
        assert_eq!(range_mark((2026, 7, 10), start, None), DayMark::Start);
        assert_eq!(
            range_mark((2026, 7, 12), start, None),
            DayMark::Off,
            "pas de bande sans fin"
        );
    }

    #[test]
    fn range_builds_grid_with_clickable_days() {
        let dp = DatePicker::range(
            2026,
            7,
            Some((2026, 7, 10)),
            Some((2026, 7, 15)),
            Msg::Pick,
            Msg::Nav,
        );
        assert_eq!(Widget::<Msg>::children(&dp).len(), 3);
        let grid = &Widget::<Msg>::children(&dp)[2];
        // Cases de remplissage (mercredi = 3) + 31 jours.
        assert_eq!(grid.children().len(), 34);
        // Le 10 juillet (3 vides + jours 1..10 → index 12) reste cliquable.
        assert_eq!(grid.children()[12].on_click(), Some(Msg::Pick(10)));
    }

    #[test]
    fn range_dual_shows_two_consecutive_months() {
        #[derive(Clone, Debug, PartialEq)]
        enum M {
            Pick(i32, u32, u32),
            Nav(i32),
        }
        // Décembre → janvier de l'année suivante (bascule d'année).
        let dp = DatePicker::range_dual(
            2026,
            12,
            Some((2026, 12, 28)),
            Some((2027, 1, 3)),
            |(y, m, d)| M::Pick(y, m, d),
            M::Nav,
        );
        // Un seul enfant : la rangée des deux mois.
        let row = &Widget::<M>::children(&dp);
        assert_eq!(row.len(), 1);
        let months = row[0].children();
        assert_eq!(months.len(), 2, "deux calendriers côte à côte");
        // Le mois de droite est janvier 2027 : cliquer son 3 rapporte la date complète.
        let jan_grid = &months[1].children()[2];
        // Janvier 2027 commence un vendredi (5 cases vides) → le 3 est à l'index 5 + 2 = 7.
        assert_eq!(first_weekday(2027, 1), 5);
        assert_eq!(jan_grid.children()[7].on_click(), Some(M::Pick(2027, 1, 3)));
    }
}
