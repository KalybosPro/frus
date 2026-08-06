//! Courbes d'accélération : des fonctions pures `[0,1] → [0,1]` qui **façonnent**
//! une progression linéaire (le « comment » d'une animation, séparé du « combien
//! de temps »).
//!
//! Une seule progression `t` linéaire peut piloter plusieurs valeurs, chacune via
//! sa propre courbe et — grâce à [`Curve::Interval`] — sur sa propre sous-fenêtre
//! temporelle (animations étagées, *staggered*, gratuites).

/// Une courbe d'accélération remappant `[0,1] → [0,1]`.
///
/// `f(0) = 0` et `f(1) = 1` pour toutes les variantes.
#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    /// Identité : `f(t) = t`.
    Linear,
    /// Bézier cubique défini par ses deux points de contrôle `(x1,y1)`, `(x2,y2)`
    /// — la même paramétrisation que `cubic-bezier()` en CSS.
    Cubic { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// Réponse indicielle d'un ressort en amortissement **critique** (départ au
    /// repos, arrivée douce sans dépassement), renormalisée pour que `f(1) = 1`.
    /// `omega` règle la vivacité.
    CriticalSpring { omega: f32 },
    /// Applique `inner` seulement sur la sous-fenêtre `[begin, end]` : `0` avant,
    /// `1` après. Déverrouille les animations étagées.
    Interval {
        begin: f32,
        end: f32,
        inner: Box<Curve>,
    },
    /// Miroir : `f(t) = 1 − inner(1 − t)` (transforme un *ease-in* en *ease-out*).
    Flipped(Box<Curve>),
}

impl Curve {
    /// `ease` du web (`cubic-bezier(0.25, 0.1, 0.25, 1.0)`).
    pub fn ease() -> Curve {
        Curve::Cubic {
            x1: 0.25,
            y1: 0.1,
            x2: 0.25,
            y2: 1.0,
        }
    }

    /// `ease-in` : démarre lentement.
    pub fn ease_in() -> Curve {
        Curve::Cubic {
            x1: 0.42,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        }
    }

    /// `ease-out` : finit lentement.
    pub fn ease_out() -> Curve {
        Curve::Cubic {
            x1: 0.0,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        }
    }

    /// `ease-in-out` : lent aux deux bouts.
    pub fn ease_in_out() -> Curve {
        Curve::Cubic {
            x1: 0.42,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        }
    }

    /// La courbe en ressort par défaut du framework (`omega = 8`) : la sensation
    /// des transitions d'écran et de feuilles, sous forme fermée.
    pub fn critical_spring() -> Curve {
        Curve::CriticalSpring { omega: 8.0 }
    }

    /// Évalue la courbe en `t` (borné à `[0,1]`).
    pub fn transform(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Curve::Linear => t,
            Curve::Cubic { x1, y1, x2, y2 } => cubic_bezier(*x1, *y1, *x2, *y2, t),
            Curve::CriticalSpring { omega } => {
                // y(τ) = 1 − e^{−ωτ}(1 + ωτ), renormalisée par y(1).
                let resp = |x: f32| 1.0 - (-omega * x).exp() * (1.0 + omega * x);
                let end = resp(1.0);
                if end == 0.0 {
                    t
                } else {
                    resp(t) / end
                }
            }
            Curve::Interval { begin, end, inner } => {
                if t <= *begin {
                    0.0
                } else if t >= *end {
                    1.0
                } else {
                    inner.transform((t - begin) / (end - begin))
                }
            }
            Curve::Flipped(inner) => 1.0 - inner.transform(1.0 - t),
        }
    }
}

/// Évalue la coordonnée `y` d'une courbe de Bézier cubique `cubic-bezier(x1, y1,
/// x2, y2)` pour une abscisse `t ∈ [0,1]`. Les points d'ancrage sont `(0,0)` et
/// `(1,1)` ; `t` est la coordonnée **x** visée, on cherche le paramètre `s` tel
/// que `Bx(s) = t`, puis on rend `By(s)`.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    // Bézier cubique à 1D : ancres 0 et 1, contrôles p1, p2.
    let bezier = |p1: f32, p2: f32, s: f32| {
        let inv = 1.0 - s;
        // 3(1−s)²s·p1 + 3(1−s)s²·p2 + s³
        3.0 * inv * inv * s * p1 + 3.0 * inv * s * s * p2 + s * s * s
    };
    // Recherche binaire du paramètre `s` donnant `Bx(s) = t` (Bx est monotone
    // pour des points de contrôle x dans [0,1]).
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    let mut s = t;
    for _ in 0..24 {
        s = 0.5 * (lo + hi);
        let x = bezier(x1, x2, s);
        if (x - t).abs() < 1e-5 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
    }
    bezier(y1, y2, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoints(curve: &Curve) {
        assert!(curve.transform(0.0).abs() < 1e-4, "f(0) ≠ 0 : {:?}", curve);
        assert!(
            (curve.transform(1.0) - 1.0).abs() < 1e-4,
            "f(1) ≠ 1 : {:?}",
            curve
        );
    }

    #[test]
    fn all_curves_hit_endpoints() {
        endpoints(&Curve::Linear);
        endpoints(&Curve::ease());
        endpoints(&Curve::ease_in());
        endpoints(&Curve::ease_out());
        endpoints(&Curve::ease_in_out());
        endpoints(&Curve::critical_spring());
    }

    #[test]
    fn linear_is_identity() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((Curve::Linear.transform(t) - t).abs() < 1e-6);
        }
    }

    #[test]
    fn ease_in_starts_slow() {
        // À mi-course, un ease-in est en dessous de la diagonale.
        assert!(Curve::ease_in().transform(0.5) < 0.5);
        // Un ease-out est au-dessus.
        assert!(Curve::ease_out().transform(0.5) > 0.5);
    }

    #[test]
    fn curves_are_monotonic() {
        for curve in [
            Curve::ease(),
            Curve::ease_in(),
            Curve::ease_out(),
            Curve::ease_in_out(),
            Curve::critical_spring(),
        ] {
            let mut prev = 0.0;
            for i in 0..=100 {
                let v = curve.transform(i as f32 / 100.0);
                assert!(v >= prev - 1e-4, "{:?} décroît en {i}", curve);
                assert!(v <= 1.0 + 1e-4, "{:?} dépasse 1 en {i}", curve);
                prev = v;
            }
        }
    }

    #[test]
    fn interval_gates_the_subwindow() {
        let staggered = Curve::Interval {
            begin: 0.5,
            end: 1.0,
            inner: Box::new(Curve::Linear),
        };
        assert_eq!(staggered.transform(0.25), 0.0, "immobile avant begin");
        assert!(
            (staggered.transform(0.75) - 0.5).abs() < 1e-4,
            "à moitié à mi-fenêtre"
        );
        assert_eq!(staggered.transform(1.0), 1.0);
    }

    #[test]
    fn flipped_mirrors() {
        let flipped = Curve::Flipped(Box::new(Curve::ease_in()));
        // Un ease-in retourné se comporte comme un ease-out (au-dessus de la diagonale).
        assert!(flipped.transform(0.5) > 0.5);
        endpoints(&flipped);
    }

    #[test]
    fn critical_spring_matches_reference_shape() {
        // Bien avancée à mi-parcours (arrivée douce), bornée, sans dépassement.
        let c = Curve::critical_spring();
        assert!(c.transform(0.5) > 0.7);
        assert!(c.transform(1.0) <= 1.0 + 1e-6);
    }
}
