use std::f64::consts::PI;

use super::correction_checks::{gurses, neighbour_p_pi, rise_at_pi, row};
use super::{Correction, Network};

#[test]
fn gurses_et_al_s_pair_is_their_eq_36() {
    // ΔT at W(1) per W at H, R (4 + 12x + 9x²) / (4 (8 + 28x + 30x² + 9x³)), x = R_X/R
    let r = 1.0 / gurses::G;
    let x = (2.0 * r + 1.0 / gurses::G_SS) / r;
    let eq36 = r * (4.0 + 12.0 * x + 9.0 * x * x)
        / (4.0 * (8.0 + 28.0 * x + 30.0 * x * x + 9.0 * x * x * x));
    let t = row(2, gurses::G_SS).steady(&[1.0, 0.0, 0.0, 0.0]).unwrap();
    assert!((t[3] / eq36 - 1.0).abs() < 1e-12, "{} against {eq36}", t[3]);
    // Eq. 31: ΔT_π = P_π R/12
    assert!((rise_at_pi() / (gurses::P_PI * r / 12.0) - 1.0).abs() < 1e-12);
    // their Eq. 38's approximation is 23 % off at their numbers; Eq. 36 is what they solve
    let p = neighbour_p_pi(gurses::G_SS);
    assert!((p / gurses::P_PI_NEIGHBOUR - 1.0).abs() < 0.01, "{p}");
}

#[test]
fn a_network_s_model_has_its_steady_state_and_its_single_pole() {
    let n = row(1, gurses::G_SS);
    let m = n.model(&[0], &[1]).unwrap();
    // one massive node: one pole, τ = 5 R C_W / 12, and R/12 at DC
    let r = 1.0 / gurses::G;
    assert_eq!(m.time_constants_s().len(), 1);
    assert!((m.time_constants_s()[0] / (5.0 * r * gurses::C_W / 12.0) - 1.0).abs() < 1e-12);
    assert!((m.steady()[0][0] / (r / 12.0) - 1.0).abs() < 1e-12);
    // a row's model between its waveguides is passive, its DC the network's own
    let n = row(3, gurses::G_SS);
    let w = [1, 3, 5];
    let m = n.model(&w, &w).unwrap();
    assert!(m.is_passive());
    let t = n.steady(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]).unwrap();
    for (i, &k) in w.iter().enumerate() {
        assert!((m.steady()[i][1] - t[k]).abs() < 1e-9 * t[3]);
    }
    assert!(n.model(&[0], &[0]).is_err(), "an output with no mass");
}

#[test]
fn a_network_not_joined_to_the_ambient_is_refused() {
    let mut n = Network::new(2);
    n.conductance(0, 1, 1.0).unwrap();
    n.capacitance(0, 1.0).unwrap();
    n.capacitance(1, 1.0).unwrap();
    assert!(n.steady(&[1.0, 0.0]).is_err());
    assert!(n.model(&[0], &[1]).is_err());
    assert!(n.conductance(0, 0, 1.0).is_err());
    assert!(n.to_ambient(2, 1.0).is_err());
}

#[test]
fn the_inverse_zeroes_the_objective_and_the_uncorrected_doesn_t() {
    let c = Correction::row(5, 0.01, &[0.05, 0.2], vec![1.0, 2.0, 3.0, 2.5, 0.5]).unwrap();
    assert_eq!(c.response()[0][2], PI / 0.2);
    assert_eq!(c.response()[0][3], 0.0);
    let p = c.inverse().unwrap();
    let (f, g) = c.objective(&p).unwrap();
    assert!(f < 1e-24 && g.iter().all(|v| v.abs() < 1e-9), "{f} {g:?}");
    let u = c.uncorrected().unwrap();
    assert!(c.objective(&u).unwrap().0 > 0.1);
    // with no crosstalk the uncorrected setting is the correction
    let alone = Correction::row(3, 0.01, &[], vec![1.0, 7.0, 3.0]).unwrap();
    let e = alone.errors(&alone.uncorrected().unwrap()).unwrap();
    assert!(e.iter().all(|v| v.abs() < 1e-12), "{e:?}");
}

#[test]
fn a_correction_names_what_it_lacks() {
    assert!(Correction::new(vec![vec![1.0, 2.0]], vec![1.0, 2.0]).is_err());
    assert!(Correction::new(vec![vec![1.0], vec![1.0, 2.0]], vec![1.0, 2.0]).is_err());
    let c = Correction::new(vec![vec![1.0, 2.0]], vec![1.0]).unwrap();
    assert!(c.inverse().is_err(), "not square");
    assert!(c.phases(&[1.0]).is_err());
    assert!(Correction::row(3, -1.0, &[], vec![0.0; 3]).is_err());
}
