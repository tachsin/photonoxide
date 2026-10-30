//! A barium titanate cross-section's Vπ·L: S. Abel et al.'s strip-loaded guide (Nat. Mater. 18,
//! 42 (2019), doi:10.1038/s41563-018-0208-0, Methods and Supplementary Notes 6 and 8).
//!
//! A 100 nm silicon strip 750 nm wide on 225 nm of barium titanate on silica, clad in silica,
//! the TE mode at 1550 nm. Abel et al. take the field in the film as uniform, E = V/g with the
//! electrodes' gap g = 4.75 µm (2 µm from each side of the strip, Supplementary Notes 1 and 6),
//! along the film and across the guide; their single domain has its c axis in the plane at
//! φ = 45° from the field, and their tensor is r₄₂ = 923, r₃₃ = 341 and r₁₃ = −56 pm/V (Note 8).
//! The film's index is the catalogue's (Abel et al.'s ellipsometry, n̄ = 2.286, Δn = 0.03).
//!
//! The mode sees the film's in-plane average ((ε_o + ε_e)/2 across and along, ε_o up), which is
//! also the single domain's tensor without its coupling of the guide's axis to the cross-section
//! (the full-vector solver takes a principal axis along the guide; the coupling, ±(ε_e − ε_o)/2,
//! enters the TE mode at second order). Δε is the single domain's, −εΔηε, by first-order
//! perturbation. Their Note 8 turns the single domain's response R = dn_eff/dV into a
//! modulator's Vπ·L through the film's net domain population ξ = 2(0.376 − 0.124) = 0.504:
//! Vπ·L = λ/(2Rξ).

use faer::c64;

use crate::convergence::Study;
use crate::material::Orientation;
use crate::material::catalogue::{Conditions, entry};
use crate::mode::pockels::{Along, Drive, Pockels, half_wave_voltage_length, section_change};
use crate::mode::thermo::index_change;
use crate::mode::vector::{self, CrossSection, Permittivity, VectorMode, graded_nodes};
use crate::units::Wavelength;
use crate::verdict::{Case, Outcome, Tier};

const OK: &str = "a valid model";

/// The wavelength, µm.
const LAMBDA: f64 = 1.55;
/// The film's thickness, µm.
const FILM: f64 = 0.225;
/// The silicon strip's thickness and width, µm.
const STRIP: (f64, f64) = (0.1, 0.75);
/// The electrodes' gap, µm.
pub(crate) const GAP: f64 = 4.75;
/// The angle between the field and the domain's c axis, degrees.
const PHI: f64 = 45.0;
/// Abel et al.'s net domain population (Supplementary Note 8).
pub(crate) const XI: f64 = 0.504;

fn wavelength() -> Wavelength {
    Wavelength::um(LAMBDA).expect(OK)
}

/// The relative permittivity of the catalogue's default model of `id` at 1550 nm, each axis.
fn permittivities(id: &str) -> Vec<c64> {
    entry(id)
        .expect(OK)
        .default_model()
        .expect(OK)
        .materials(Conditions::default())
        .expect(OK)
        .iter()
        .map(|m| m.permittivity(wavelength()).expect(OK))
        .collect()
}

/// Abel et al.'s single domain (pm/V): r₁₃ = −56, r₃₃ = 341, r₄₂ = r₅₁ = 923.
fn abel_tensor() -> Pockels {
    let (r13, r33, r42) = (-56.0, 341.0, 923.0);
    let mut v = [[0.0; 3]; 6];
    v[0][2] = r13;
    v[1][2] = r13;
    v[2][2] = r33;
    v[3][1] = r42;
    v[4][0] = r42;
    Pockels::from_voigt(v).expect(OK)
}

/// The domain's orientation: c in the plane at φ from the chip's x (Euler angles (φ, 90°, 0),
/// c along (sin φ, −cos φ, 0)), the guide along the chip's y, the field along its x.
fn orientation() -> Orientation {
    Orientation::euler_deg(PHI, 90.0, 0.0).expect(OK)
}

/// Where a point (µm; y = 0 at the film's bottom) is: 0 silica, 1 the film, 2 silicon.
fn medium(x: f64, y: f64) -> usize {
    if (0.0..FILM).contains(&y) {
        1
    } else if (FILM..FILM + STRIP.0).contains(&y) && x.abs() < 0.5 * STRIP.1 {
        2
    } else {
        0
    }
}

/// The cross-section on a grid `h` fine over the film and the strip, the film's cells marked.
pub(crate) fn cross_section(h: f64) -> (CrossSection, Vec<bool>) {
    let oxide = permittivities("sio2")[0];
    let silicon = permittivities("si")[0];
    let bto = permittivities("batio3");
    let (eo, ee) = (bto[0], bto[1]);
    let zero = c64::new(0.0, 0.0);
    let film = Permittivity {
        xx: 0.5 * (eo + ee),
        xy: zero,
        yx: zero,
        yy: eo,
        zz: 0.5 * (eo + ee),
    };
    let w = 0.5 * STRIP.1;
    let xs = graded_nodes(-3.0, 3.0, (-w - 0.4, w + 0.4), h, 0.08, 0.1, &[-w, w]);
    let top = FILM + STRIP.0;
    let ys = graded_nodes(
        -1.5,
        top + 1.5,
        (-0.2, top + 0.2),
        h,
        0.08,
        0.1,
        &[0.0, FILM, top],
    );
    let mut cells = Vec::with_capacity((xs.len() - 1) * (ys.len() - 1));
    let mut in_film = Vec::with_capacity(cells.capacity());
    for i in 0..xs.len() - 1 {
        for j in 0..ys.len() - 1 {
            let (x, y) = (0.5 * (xs[i] + xs[i + 1]), 0.5 * (ys[j] + ys[j + 1]));
            let m = medium(x, y);
            cells.push(match m {
                1 => film,
                2 => Permittivity::isotropic(silicon),
                _ => Permittivity::isotropic(oxide),
            });
            in_film.push(m == 1);
        }
    }
    (CrossSection::new(xs, ys, cells).expect(OK), in_film)
}

/// The TE-like fundamental mode.
fn te_mode(cs: &CrossSection) -> VectorMode {
    vector::modes(cs, wavelength(), 2, None)
        .expect(OK)
        .into_iter()
        .max_by(|a, b| a.te_fraction().total_cmp(&b.te_fraction()))
        .expect("a mode")
}

/// The single domain's response R = |dn_eff/dV| (1/V) on a grid `h` fine, and the share of the
/// mode's power in the film.
pub(crate) fn response(h: f64) -> (f64, f64) {
    let (cs, in_film) = cross_section(h);
    let mode = te_mode(&cs);
    let bto = permittivities("batio3");
    let principal = [bto[0], bto[0], bto[1]];
    let domain = Along::Y.tensor(orientation().rotate(principal));
    let r = abel_tensor().rotated(&orientation()).in_section(Along::Y);
    // a volt across the gap, along the cross-section's x
    let change = section_change(r.permittivity_change(domain, [1.0 / (GAP * 1e-6), 0.0, 0.0]));
    let zero = Permittivity::isotropic(c64::new(0.0, 0.0));
    let delta: Vec<Permittivity> = in_film
        .iter()
        .map(|&f| if f { change } else { zero })
        .collect();
    let per_volt = index_change(&mode, &cs, &delta).expect(OK).re.abs();
    (per_volt, film_share(&mode, &cs))
}

/// The share of `mode`'s power flow (its z Poynting component) in the film.
fn film_share(mode: &VectorMode, cs: &CrossSection) -> f64 {
    let f = mode.fields(cs).expect(OK);
    let (x, y) = (f.x(), f.y());
    let (mut film, mut all) = (0.0, 0.0);
    for i in 0..x.len() {
        for j in 0..y.len() {
            let dx = 0.5 * (x[(i + 1).min(x.len() - 1)] - x[i.saturating_sub(1)]);
            let dy = 0.5 * (y[(j + 1).min(y.len() - 1)] - y[j.saturating_sub(1)]);
            let (e, hf) = (f.e(i, j), f.h(i, j));
            let s = (e[0] * hf[1].conj() - e[1] * hf[0].conj()).re * dx * dy;
            all += s;
            if (0.0..FILM).contains(&y[j]) {
                film += s;
            }
        }
    }
    film / all
}

/// Vπ·L (V·cm) from a response R through Abel et al.'s ξ: λ/(2Rξ).
pub(crate) fn vpi_l(per_volt: f64) -> f64 {
    100.0 * half_wave_voltage_length(per_volt * XI, wavelength(), Drive::SingleArm).expect(OK)
}

/// The single domain's response on optical cells of 40, 20 and 10 nm, gated by the grid study
/// at 0.5%: the finest's.
pub(crate) fn gated_response() -> f64 {
    let mut study = Study::new("dn_eff/dV");
    for h in [0.04, 0.02, 0.01] {
        study = study.on(h, response(h).0).expect(OK);
    }
    let c = study.check(5e-3 * study.values[2]).expect(OK);
    c.gated().expect("a converged grid")
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// Abel et al.'s single-domain response, 3.4e-4 per volt (Supplementary Note 8).
fn abel_response() -> Outcome {
    outcome(gated_response() * 1e4, 3.4, 0.1)
}

/// Their Vπ·L, 0.45 V·cm, from it.
fn abel_vpi_l() -> Outcome {
    outcome(vpi_l(gated_response()), 0.45, 0.015)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "pockels/bto-abel-response",
            title: r"Abel et al.'s barium titanate guide, a 100 nm silicon strip 750 nm wide on 225 nm of the film on silica, clad in silica: the TE mode's $\lvert dn_\mathrm{eff}/dV\rvert$ at 1.55 µm (1e-4 per volt, shown) for a single domain, its c axis in the plane at 45° from a uniform field $V/g$ across ($g$ = 4.75 µm), $r_{42}$ = 923, $r_{33}$ = 341, $r_{13}$ = −56 pm/V, the film's index Abel et al.'s; first-order perturbation, optical cells of 40, 20 and 10 nm gated at 0.5%. A cross-section's, per unit length",
            tier: Tier::Published,
            source: r"S. Abel et al., Nat. Mater. 18, 42 (2019), doi:10.1038/s41563-018-0208-0, Supplementary Note 8: $R$ = 3.4e-4 per volt from their FDTD (Note 6: 33 nm cells, silica cladding; the Methods say air, the devices are clad in silica); the tolerance their two digits and their grid, 3%. The mode's power in the film, 41%, against their 39% (main text, from PHOENIX and COMSOL)",
            run: abel_response,
        },
        Case {
            id: "pockels/bto-abel",
            title: r"The same guide's $V_\pi L$ (V·cm, a cross-section's, per unit length; shown) from that response through Abel et al.'s net domain population $\xi$ = 0.504: $V_\pi L = \lambda/(2R\xi)$",
            tier: Tier::Published,
            source: r"S. Abel et al., Nat. Mater. 18, 42 (2019), doi:10.1038/s41563-018-0208-0, main text and Supplementary Note 8: 0.45 V·cm for their TE photonic waveguides, $\lambda/(2R\xi)$ = 0.452 from their own $R$; the tolerance as above",
            run: abel_vpi_l,
        },
    ]
}
