use super::*;
use crate::nonlinear::pulse::Dispersion;

fn kerr(gamma: f64) -> Guide {
    Guide::new(Dispersion::new(1.55, &[0.0]).unwrap(), gamma, 0.0)
}

#[test]
fn the_slower_pulse_moves_later_by_the_walk_off() {
    let pair = TwoPulses::new(kerr(0.0), kerr(0.0), 2.0);
    let a = Pulse::gaussian(1.0, 1.0, 512, 64.0).unwrap();
    let (p, q) = pair.propagate(&a, &a, 3.0, 30).unwrap();
    let peak = |x: &Pulse| {
        let power = x.power();
        let j = (0..power.len())
            .max_by(|&i, &k| power[i].total_cmp(&power[k]))
            .unwrap();
        x.times()[j]
    };
    assert!(peak(&p).abs() < 1e-12);
    assert!((peak(&q) - 6.0).abs() < 0.2, "{}", peak(&q));
    // without γ the moduli are moved, not changed
    assert!((q.energy() - a.energy()).abs() < 1e-10);
}

#[test]
fn without_walk_off_the_cross_phase_is_twice_the_self_phase() {
    let pair = TwoPulses::new(kerr(1.0), kerr(1.0), 0.0);
    let a = Pulse::gaussian(1.0, 1.0, 256, 32.0).unwrap();
    let zero = Pulse::sampled(256, 32.0, |_| c64::new(1e-30, 0.0)).unwrap();
    let (_, probe) = pair.propagate(&a, &zero, 1.0, 10).unwrap();
    let j = 128;
    let phase = (probe.field[j] / zero.field[j]).arg();
    assert!((phase - 2.0).abs() < 1e-12, "{phase}");
    let closed = gaussian_phase(1.0, [0.0, 1.0], 1.0, 0.0, 0.0, 2.0, 1.0, 0.0);
    assert!((closed - 2.0).abs() < 1e-15);
}

#[test]
fn it_refuses_what_it_does_not_model() {
    let a = Pulse::gaussian(1.0, 1.0, 64, 16.0).unwrap();
    let b = Pulse::gaussian(1.0, 1.0, 128, 16.0).unwrap();
    let pair = TwoPulses::new(kerr(1.0), kerr(1.0), 0.0);
    assert!(pair.propagate(&a, &b, 1.0, 1).is_err());
    assert!(pair.propagate(&a, &a, -1.0, 1).is_err());
    let raman = TwoPulses::new(
        kerr(1.0).with_raman(crate::nonlinear::pulse::Raman::SILICA),
        kerr(1.0),
        0.0,
    );
    assert!(raman.propagate(&a, &a, 1.0, 1).is_err());
}
