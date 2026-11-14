//! A uniform line's complex wavenumber and impedance from the voltages and currents a 3D
//! simulation measures at planes along it, whatever its reflections.
//!
//! In photonoxide's e^{−iωt} convention a wave along +x goes as e^{ikx}, k = β + iα, and on a
//! uniform line V(x) = A e^{ikx} + B e^{−ikx}, I(x) = (A e^{ikx} − B e^{−ikx})/Z₀ for any
//! forward and backward waves A and B. Then, exactly:
//!
//! - V(x − Δ) + V(x + Δ) = 2 cos(kΔ) V(x): k from the voltages at three planes Δ apart;
//! - V(x + δ) − V(x − δ) = 2i sin(kδ) Z₀ I(x): Z₀ from the voltages at two planes 2δ apart and
//!   the current midway.
//!
//! This is the transmission-line form of the three-plane (and two-plane) methods by which 3D
//! solvers give a line's γ and Z₀ (the telegrapher's solution: D. M. Pozar, Microwave
//! Engineering, 4th ed., Wiley (2012), ISBN 978-0-470-63155-3, Section 2.1), with nothing
//! assumed of the ends.

use num_complex::Complex64 as c64;
use photonoxide::portable;

/// The wavenumber k = β + iα (1/m) from the voltages `v` at three planes `spacing` (m) apart,
/// in order along +x: acos((V₁ + V₃)/(2V₂))/Δ, its principal value (βΔ in (0, π), α ≥ 0 for a
/// wave that decays along +x).
pub fn wavenumber(v: [c64; 3], spacing: f64) -> c64 {
    acos((v[0] + v[2]) / (2.0 * v[1])) / spacing
}

/// The principal arccosine, by num-complex's formula −i ln(z + i √(1 − z²)) with the portable
/// functions, so that the same voltages give the same k on every system (#822).
fn acos(z: c64) -> c64 {
    let i = c64::i();
    -i * portable::complex::ln(i * portable::complex::sqrt(c64::new(1.0, 0.0) - z * z) + z)
}

/// The impedance Z₀ (Ω) from the voltages `v` at two planes `spacing` (m) apart, in order
/// along +x, the current `current` midway (along +x) and the wavenumber `k`:
/// (V₂ − V₁)/(2i sin(kΔ/2) I).
pub fn impedance(v: [c64; 2], current: c64, k: c64, spacing: f64) -> c64 {
    (v[1] - v[0]) / (2.0 * c64::i() * portable::complex::sin(0.5 * k * spacing) * current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::portable;

    /// A lossy line with a forward and a strong backward wave: k and Z₀ recovered exactly.
    #[test]
    fn a_line_with_reflections_gives_its_wavenumber_and_impedance() {
        let k = c64::new(440.0, 3.0);
        let z = c64::new(48.0, -0.7);
        let (a, b) = (c64::new(1.0, 0.2), c64::new(-0.4, 0.3));
        let v = |x: f64| {
            a * portable::complex::exp(c64::i() * k * x)
                + b * portable::complex::exp(-c64::i() * k * x)
        };
        let i = |x: f64| {
            (a * portable::complex::exp(c64::i() * k * x)
                - b * portable::complex::exp(-c64::i() * k * x))
                / z
        };
        let (x0, d) = (1.5e-3, 6.25e-4);
        let found = wavenumber([v(x0), v(x0 + d), v(x0 + 2.0 * d)], d);
        assert!(
            portable::complex::abs(found - k) < 1e-9 * portable::complex::abs(k),
            "{found}"
        );
        let zz = impedance([v(x0), v(x0 + d)], i(x0 + 0.5 * d), found, d);
        assert!(
            portable::complex::abs(zz - z) < 1e-9 * portable::complex::abs(z),
            "{zz}"
        );
    }
}
