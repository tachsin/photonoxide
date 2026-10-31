//! The validation cases of second-order processes in waveguides ([`super::guide`]): the modal
//! mismatch against the bulk crystal's where the mode is a plane wave, Zhao et al.'s poling
//! period, phase-matched wavelength and effective areas from their guide's modes; the overlap's
//! efficiency against Boyd's plane-wave form and a slab's exact fields; a waveguide's SHG through
//! the coupled equations with loss; and the group index the joint spectrum takes, against a
//! slab's exact one.

use num_complex::Complex64 as c64;

use crate::convergence::Study;
use crate::material::catalogue::{Conditions, entry};
use crate::material::{CrystalAxis, Material, Model, Orientation, Provenance, vacuum};
use crate::mode::Polarization;
use crate::mode::pockels::Along;
use crate::mode::slab::Slab;
use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity, VectorMode};
use crate::nonlinear::coupled::{SecondHarmonic, loss_from_db_per_cm, undepleted_lossy_shg};
use crate::nonlinear::guide::{
    Family, Grid, ModeField, ModeIndex, Ridge, SecondOrder, fundamental, percent_per_watt_cm2,
    plane_wave_efficiency, second_harmonic_coupling,
};
use crate::nonlinear::phase::{
    Device, Grating, Polarization as Pol, Process, State, Type, Uniaxial,
};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier, note_numerical};

const OK: &str = "a valid model";

fn um(lam: f64) -> Wavelength {
    Wavelength::um(lam).expect(OK)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The catalogue's default model of `id` as an isotropic material (silica's Malitson).
pub(crate) fn isotropic(id: &str) -> Material {
    entry(id)
        .expect(OK)
        .default_model()
        .expect(OK)
        .materials(Conditions::default())
        .expect(OK)
        .remove(0)
}

/// A material of constant index `n` from 0.2 to 5 µm.
fn constant(n: f64) -> Material {
    Material::new(
        "n",
        Model::Constant(c64::new(n, 0.0)),
        um(0.2),
        um(5.0),
        Provenance {
            reference: "a constant index, for a closed form".into(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    )
    .expect(OK)
}

/// The interpolants' tolerance in n_eff: 1e-10, far below the grids' own errors.
const INDEX_TOLERANCE: f64 = 1e-10;

/// A grid study's finest value and its estimated error ([`Study::check`]).
fn studied(quantity: &str, values: &[(f64, f64)]) -> (f64, f64) {
    let mut study = Study::new(quantity);
    for &(h, v) in values {
        study = study.on(h, v).expect(OK);
    }
    let c = study.check(1.0).expect(OK);
    (c.value, c.error.unwrap_or(f64::INFINITY))
}

// ---------------------------------------------------------------------------------------------
// A plane wave: the mode in a crystal filling the window
// ---------------------------------------------------------------------------------------------

/// The side of the window a crystal fills, µm: small enough that no mode polarized across c has
/// an index above the uniform one's along it (a field across c varies across the window's walls).
const FILLED: f64 = 0.2;

/// Congruent lithium niobate (Zelmon et al.'s indices) cut as `cut` (x: c across a guide along
/// the chip's y; z: c up), filling a [`FILLED`] µm square window of 8 × 8 cells between walls that
/// hold a uniform field polarized along c: electric walls normal to it, magnetic ones along it.
fn filled(cut: CrystalAxis, lam: f64) -> CrossSection {
    let orientation = match cut {
        CrystalAxis::X => Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)),
        _ => Orientation::cut(CrystalAxis::Z, None),
    }
    .expect(OK);
    let crystal = entry("linbo3")
        .expect(OK)
        .oriented(None, Conditions::default(), orientation)
        .expect(OK);
    let t =
        Permittivity::from_tensor(Along::Y.tensor(crystal.tensor(um(lam)).expect(OK))).expect(OK);
    let (along, across) = (Boundary::ElectricWall, Boundary::MagneticWall);
    let boundaries = if cut == CrystalAxis::X {
        // E along x: electric walls normal to x
        Boundaries {
            west: along,
            east: along,
            south: across,
            north: across,
        }
    } else {
        Boundaries {
            west: across,
            east: across,
            south: along,
            north: along,
        }
    };
    CrossSection::uniform((0.0, FILLED, 8), (0.0, FILLED, 8), |_, _| t)
        .and_then(|cs| cs.with_boundaries(boundaries))
        .expect(OK)
}

fn filled_mode(cut: CrystalAxis, lam: f64) -> (CrossSection, VectorMode) {
    let cs = filled(cut, lam);
    let family = if cut == CrystalAxis::X {
        Family::Te
    } else {
        Family::Tm
    };
    let m = fundamental(&cs, um(lam), family, 2).unwrap_or_else(|e| panic!("{e}"));
    (cs, m)
}

/// The SHG mismatch of 1.064 µm in x-cut lithium niobate from the plane-wave mode's effective
/// indices, interpolated over 1.04–1.09 and 0.52–0.545 µm and fitted, against the bulk crystal's from
/// [`super::phase`] (both type 0, extraordinary): the relative difference.
pub(crate) fn bulk_limit() -> f64 {
    let sweep = |from: f64, to: f64| {
        ModeIndex::sample(
            |w| Ok(filled_mode(CrystalAxis::X, w.to_um()).1),
            from,
            to,
            INDEX_TOLERANCE,
        )
        .expect(OK)
    };
    let (f, h) = (sweep(1.04, 1.09), sweep(0.52, 0.545));
    let (fi, hi) = (f.index().expect(OK), h.index().expect(OK));
    let modal = Process::second_harmonic(um(1.064), fi.clone(), fi, hi)
        .mismatch(&State::default())
        .expect(OK);
    let crystal = Uniaxial::catalogue("linbo3", None, None).expect(OK);
    let bulk = crystal
        .second_harmonic(um(1.064), Type::Zero(Pol::Extraordinary), &State::default())
        .expect(OK)
        .mismatch(&State::default())
        .expect(OK);
    (modal / bulk - 1.0).abs()
}

/// The SHG efficiency of the z-cut plane wave (d₃₃ = 25.2 pm/V, Shoji et al.'s at 1.064 µm, up
/// along c) from its overlap, against Boyd's plane-wave form over the window's area: the
/// relative difference.
pub(crate) fn plane_wave() -> f64 {
    let lam = 1.064;
    let (cs, f) = filled_mode(CrystalAxis::Z, lam);
    let (_, h) = filled_mode(CrystalAxis::Z, lam / 2.0);
    let (ef, eh) = (
        ModeField::new(&f, &cs, um(lam)).expect(OK),
        ModeField::new(&h, &cs, um(lam / 2.0)).expect(OK),
    );
    let d = z_cut_d33(25.2);
    let share = vec![1.0; cs.cells().len()];
    let eta = percent_per_watt_cm2(second_harmonic_coupling(&eh, &ef, &d, &share).expect(OK));
    let ne = |l: f64| {
        entry("linbo3")
            .expect(OK)
            .principal(None, Conditions::default())
            .expect(OK)[2]
            .refractive_index(um(l))
            .expect(OK)
            .re
    };
    let exact = plane_wave_efficiency(um(lam), 25.2, ne(lam), ne(lam / 2.0), FILLED * FILLED);
    (eta / exact - 1.0).abs()
}

/// A d tensor with d₃₃ alone, of a z-cut crystal in a guide along the chip's y: d_yyy.
fn z_cut_d33(d33: f64) -> SecondOrder {
    let mut v = [[0.0; 6]; 3];
    v[2][2] = d33;
    SecondOrder::from_voigt(v)
        .expect(OK)
        .rotated(&Orientation::cut(CrystalAxis::Z, None).expect(OK))
        .in_section(Along::Y)
}

// ---------------------------------------------------------------------------------------------
// A slab's exact fields
// ---------------------------------------------------------------------------------------------

/// The slab cases' film, µm: z-cut lithium niobate, c along the cross-section's y, between
/// silica, its interfaces normal to x and the slab uniform in y between electric walls.
const SLAB: f64 = 0.4;

/// The slab's cross-section at `lam` on cells of `h` (the interfaces on nodes), 4.02 µm wide.
pub(crate) fn slab_cs(lam: f64, h: f64, core: &Material, clad: &Material) -> CrossSection {
    let n = (4.02 / h).round() as usize;
    let z_cut = Orientation::cut(CrystalAxis::Z, None).expect(OK);
    let crystal =
        Material::oriented("LN (z)", [core.clone(), core.clone(), core.clone()], z_cut).expect(OK);
    let tc =
        Permittivity::from_tensor(Along::Y.tensor(crystal.tensor(um(lam)).expect(OK))).expect(OK);
    let tx = Permittivity::from_tensor(Along::Y.tensor(clad.tensor(um(lam)).expect(OK))).expect(OK);
    CrossSection::uniform((-2.01, 2.01, n), (0.0, 4.0 * h, 4), |x, _| {
        if x.abs() < SLAB / 2.0 { tc } else { tx }
    })
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            south: Boundary::ElectricWall,
            north: Boundary::ElectricWall,
            ..Boundaries::default()
        })
    })
    .expect(OK)
}

/// The extraordinary index of congruent lithium niobate (Zelmon et al.), isotropic.
fn ln_e() -> Material {
    entry("linbo3")
        .expect(OK)
        .principal(None, Conditions::default())
        .expect(OK)[2]
        .clone()
}

fn index_of(m: &Material, lam: f64) -> f64 {
    m.refractive_index(um(lam)).expect(OK).re
}

/// The slab's SHG efficiency (%/(W·cm²)) at the fundamental 1.064 µm, its TE₀ modes (E along c)
/// on cells of `h`, d₃₃ = 25.2 pm/V in the film.
pub(crate) fn slab_efficiency(h: f64) -> f64 {
    let (core, clad) = (ln_e(), isotropic("sio2"));
    let lam = 1.064;
    let solve = |l: f64| {
        let cs = slab_cs(l, h, &core, &clad);
        let m = fundamental(&cs, um(l), Family::Tm, 2).expect(OK);
        let f = ModeField::new(&m, &cs, um(l)).expect(OK);
        (cs, f)
    };
    let (cs, ef) = solve(lam);
    let (_, eh) = solve(lam / 2.0);
    let xs = cs.x().to_vec();
    let ny = cs.y().len() - 1;
    let share: Vec<f64> = (0..cs.cells().len())
        .map(|n| {
            let i = n / ny;
            let x = 0.5 * (xs[i] + xs[i + 1]);
            f64::from(u8::from(x.abs() < SLAB / 2.0))
        })
        .collect();
    let d = z_cut_d33(25.2);
    percent_per_watt_cm2(second_harmonic_coupling(&eh, &ef, &d, &share).expect(OK))
}

/// The same from the slab's exact TE fields ([`Slab`]) over the same 4.02 µm window and 4h
/// height: ê = u (2Z₀/(n_eff H ∫u² dx))^½, O = d₃₃ H ∫_film ê_2ω ê_ω² dx, κ = (ωε₀/2)|O|,
/// the integrals by a midpoint rule of 402 000 points (the interfaces on their cells' edges).
pub(crate) fn slab_efficiency_exact(h: f64) -> f64 {
    let (core, clad) = (ln_e(), isotropic("sio2"));
    let lam = 1.064;
    let mode = |l: f64| {
        Slab::new(
            index_of(&clad, l),
            index_of(&core, l),
            index_of(&clad, l),
            Length::um(SLAB),
        )
        .expect(OK)
        .modes(Polarization::Te, um(l))
        .remove(0)
    };
    let (mf, mh) = (mode(lam), mode(lam / 2.0));
    let (uf, uh) = (
        |x: f64| mf.field(Length::um(x + SLAB / 2.0)).0,
        |x: f64| mh.field(Length::um(x + SLAB / 2.0)).0,
    );
    let steps = 402_000;
    let dx = 4.02 / f64::from(steps);
    let (mut iff, mut ihh, mut c) = (0.0, 0.0, 0.0);
    for s in 0..steps {
        let x = -2.01 + (f64::from(s) + 0.5) * dx;
        let (a, b) = (uf(x), uh(x));
        iff += a * a * dx;
        ihh += b * b * dx;
        if x.abs() < SLAB / 2.0 {
            c += b * a * a * dx;
        }
    }
    let height = 4.0 * h;
    let z0 = 376.730_313_668;
    // per √W, in V/m: the area H·∫u² in m²
    let sf = (2.0 * z0 / (mf.effective_index() * height * iff * 1e-12)).sqrt();
    let sh = (2.0 * z0 / (mh.effective_index() * height * ihh * 1e-12)).sqrt();
    let o = 25.2e-12 * height * c * 1e-12 * sh * sf * sf;
    let omega = std::f64::consts::TAU * 299_792_458.0 / (lam * 1e-6);
    let eps0 = 8.854_187_812_8e-12;
    let kappa = 0.5 * omega * eps0 * o.abs() * 1e-6;
    percent_per_watt_cm2(kappa)
}

/// The slab's efficiency on 10, 5 and 2.5 nm cells, each against the exact fields' over its own
/// height: the finest grid's relative error and the grid study's estimate of its own error,
/// relative; and the observed order.
pub(crate) fn slab_overlap() -> (f64, f64, f64) {
    let hs = [0.01, 0.005, 0.0025];
    // η ∝ 1/H: compare each at its height, as η·H
    let values: Vec<(f64, f64)> = hs
        .iter()
        .map(|&h| (h, slab_efficiency(h) * 4.0 * h))
        .collect();
    let exact = slab_efficiency_exact(0.0025) * 0.01;
    let (value, error) = studied("slab SHG efficiency", &values);
    let order = ((values[0].1 - values[1].1) / (values[1].1 - values[2].1)).log2();
    ((value / exact - 1.0).abs(), error / exact, order)
}

// ---------------------------------------------------------------------------------------------
// Zhao et al. 2020
// ---------------------------------------------------------------------------------------------

/// Zhao et al. 2020's ridge (Phys. Rev. Lett. 124, 163603, p. 1 and Fig. 1(c)): 300 nm of x-cut
/// 5 mol% MgO-doped lithium niobate on silica (1.8 µm, over silicon), etched 50 nm to a ridge
/// 1.24 µm wide with 75° sidewalls, air above; the guide along the crystal's y (their Fig. 1(b)
/// and p. 3: "ŷ (which is the waveguide axis)"), so c across. `model` is the crystal's index
/// model (the catalogue's default, Zelmon et al.'s, when `None`); the paper doesn't name one.
pub(crate) fn zhao_ridge(model: Option<&str>) -> Ridge {
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).expect(OK);
    Ridge {
        film: 0.3,
        etch: 0.05,
        width: 1.24,
        angle: 75.0,
        core: entry("linbo3-mgo")
            .expect(OK)
            .oriented(model, Conditions::default(), x_cut)
            .expect(OK),
        substrate: isotropic("sio2"),
        cladding: vacuum(),
        along: Along::Y,
    }
}

/// The TE₀ mode of `ridge` at `lam` µm on `grid`, and its cross-section.
pub(crate) fn te0_mode(ridge: &Ridge, grid: &Grid, lam: f64) -> (CrossSection, VectorMode) {
    let w = um(lam);
    let cs = ridge.cross_section(w, grid, Family::Te).expect(OK);
    let m = fundamental(&cs, w, Family::Te, 4).expect(OK);
    (cs, m)
}

/// The TE₀ effective index of `ridge` at `lam` µm on `grid`.
pub(crate) fn te0(ridge: &Ridge, grid: &Grid, lam: f64) -> f64 {
    te0_mode(ridge, grid, lam).1.effective_index().re
}

/// The first-order period of degenerate down-conversion 0.785 → 1.57 µm (and of SHG of 1.57 µm)
/// in Zhao et al.'s guide, from its TE₀ modes on cells of `h` (half the window): λ_p/(n_p − n_s).
pub(crate) fn zhao_period_on(h: f64) -> f64 {
    let (r, g) = (zhao_ridge(None), Grid::new(h).halved());
    0.785 / (te0(&r, &g, 0.785) - te0(&r, &g, 1.57))
}

/// The period on 20, 10 and 5 nm cells: the finest value and the grid study's estimate of its
/// error.
pub(crate) fn zhao_period() -> (f64, f64) {
    let values: Vec<(f64, f64)> = [0.02, 0.01, 0.005]
        .iter()
        .map(|&h| (h, zhao_period_on(h)))
        .collect();
    studied("Zhao et al.'s period", &values)
}

fn zhao_period_case() -> Outcome {
    let (period, error) = zhao_period();
    note_numerical(error);
    outcome(period, 2.8, 0.05)
}

/// The TE₀ mode's effective area at `lam` µm in Zhao et al.'s guide on cells of `h` (half the
/// window), µm²: Afshar and Monro's Eq. 41, from S_z
/// ([`crate::mode::fields::Fields::effective_area`]). The paper doesn't define its "modal
/// effective area"; Eq. 41's converges on these grids, where the standard Eq. 44, from |E_t|⁴,
/// takes the ridge's corners twice (1.110, 1.107 and 1.103 µm² at 1.57 µm; 0.400, 0.401 and 0.401
/// at 0.785 µm): both round to the paper's.
pub(crate) fn zhao_area_on(lam: f64, h: f64) -> f64 {
    let (r, g) = (zhao_ridge(None), Grid::new(h).halved());
    let (cs, m) = te0_mode(&r, &g, lam);
    m.fields_one_sided(&cs).expect(OK).effective_area()
}

/// The area on 20, 10 and 5 nm cells: the finest value and the grid study's estimate of its error.
pub(crate) fn zhao_area(lam: f64) -> (f64, f64) {
    let values: Vec<(f64, f64)> = [0.02, 0.01, 0.005]
        .iter()
        .map(|&h| (h, zhao_area_on(lam, h)))
        .collect();
    studied("Zhao et al.'s effective area", &values)
}

fn zhao_area_signal_case() -> Outcome {
    let (area, error) = zhao_area(1.57);
    note_numerical(error);
    outcome(area, 1.1, 0.05)
}

fn zhao_area_pump_case() -> Outcome {
    let (area, error) = zhao_area(0.785);
    note_numerical(error);
    outcome(area, 0.4, 0.05)
}

/// The TE₀ indices of Zhao et al.'s guide on cells of `h` over 1.50–1.65 µm and 0.75–0.825 µm.
pub(crate) fn zhao_indices(h: f64) -> (ModeIndex, ModeIndex) {
    let (r, g) = (zhao_ridge(None), Grid::new(h).halved());
    let sweep =
        |range| ModeIndex::of_ridge(&r, &g, Family::Te, 4, range, INDEX_TOLERANCE).expect(OK);
    (sweep((1.50, 1.65)), sweep((0.75, 0.825)))
}

/// The fundamental (µm) that a first-order grating of `period` µm phase-matches for SHG in a
/// guide of these indices, between 1.51 and 1.64 µm.
pub(crate) fn matched_wavelength(
    fundamental: &ModeIndex,
    harmonic: &ModeIndex,
    period: f64,
) -> f64 {
    let (fi, hi) = (fundamental.index().expect(OK), harmonic.index().expect(OK));
    let process = Process::second_harmonic(um(1.57), fi.clone(), fi, hi);
    Device::new(process, 5000.0, State::default())
        .expect(OK)
        .with_grating(Grating::new(period, 1, 0.5).expect(OK))
        .phase_matching_wavelength(1.51, 1.64)
        .expect(OK)
}

/// Zhao et al.'s guide at its printed period, 2.8 µm: the phase-matched fundamental (µm, on 10
/// nm cells), and its change between 20 and 10 nm cells.
pub(crate) fn zhao_wavelength() -> (f64, f64) {
    let at = |h: f64| {
        let (f, s) = zhao_indices(h);
        matched_wavelength(&f, &s, 2.8)
    };
    let (coarse, fine) = (at(0.02), at(0.01));
    (fine, (fine - coarse).abs())
}

fn zhao_wavelength_case() -> Outcome {
    let (lam, error) = zhao_wavelength();
    note_numerical(error);
    outcome(lam, 1.57, ZHAO_WAVELENGTH_TOLERANCE)
}

/// How far the phase-matched fundamental moves when the period moves by 0.05 µm, half the last
/// printed digit of 2.8 µm: from 2.75 to 2.85 µm on 10 nm cells, half the span, 0.0389 µm,
/// rounded down.
pub(crate) const ZHAO_WAVELENGTH_TOLERANCE: f64 = 0.038;

// ---------------------------------------------------------------------------------------------
// A waveguide's SHG through the coupled equations
// ---------------------------------------------------------------------------------------------

/// The slab's SHG (5 nm cells) over 4 mm, its κ from the overlap and its mismatch from its modes'
/// effective indices, against a grating 0.1 % off the period that cancels it, with 3 and 6 dB/cm
/// of loss, 1 aW in (undepleted: the slab's window is 20 nm tall, so η is large; the harmonic's depletion of the fundamental, ηP₀L², is 3e-15): the harmonic's power by RK4 on steps steps against the closed form, the
/// relative difference.
pub(crate) fn slab_coupled(steps: usize) -> f64 {
    let h = 0.005;
    let (core, clad) = (ln_e(), isotropic("sio2"));
    let n = |l: f64| {
        let cs = slab_cs(l, h, &core, &clad);
        fundamental(&cs, um(l), Family::Tm, 2)
            .expect(OK)
            .effective_index()
            .re
    };
    let lam = 1.064;
    let mismatch = std::f64::consts::TAU * (2.0 * n(lam / 2.0) / lam - 2.0 * n(lam) / lam);
    let period = std::f64::consts::TAU / mismatch.abs() * 1.001;
    let kappa = super::coupled::coupling_from_efficiency(slab_efficiency(h));
    let shg = SecondHarmonic {
        coupling: kappa,
        mismatch: mismatch - std::f64::consts::TAU / period,
        loss: [loss_from_db_per_cm(3.0), loss_from_db_per_cm(6.0)],
    };
    let (p0, length): (f64, f64) = (1e-18, 4000.0);
    let out = shg
        .propagate(
            [c64::new(p0.sqrt(), 0.0), c64::new(0.0, 0.0)],
            length,
            steps,
        )
        .expect(OK);
    let exact = undepleted_lossy_shg(&shg, p0, length);
    (out[1].norm_sqr() / exact - 1.0).abs()
}

// ---------------------------------------------------------------------------------------------
// The group index the joint spectrum takes
// ---------------------------------------------------------------------------------------------

/// A slab of constant indices (2.2 in 1.45, 0.4 µm, without material dispersion): its TE₀ group
/// index at 1.55 µm from the interpolated effective index on cells of `h`.
pub(crate) fn slab_group_index(h: f64) -> f64 {
    let (core, clad) = (constant(2.2), constant(1.45));
    let index = ModeIndex::sample(
        |w| {
            let cs = slab_cs(w.to_um(), h, &core, &clad);
            fundamental(&cs, w, Family::Tm, 2)
        },
        1.50,
        1.60,
        INDEX_TOLERANCE,
    )
    .expect(OK);
    index.group_index(1.55)
}

/// The same slab's exact group index: without material dispersion, n_g = ⟨ε⟩/n_eff with ⟨ε⟩ the
/// mean permittivity weighted by the exact field's square (Hellmann–Feynman on the TE equation,
/// as [`crate::mode::dispersion`]'s case), by a midpoint rule of 402 000 points over ±2.01 µm (the interfaces on\n/// their cells' edges).
pub(crate) fn slab_group_index_exact() -> f64 {
    let m = Slab::new(1.45, 2.2, 1.45, Length::um(SLAB))
        .expect(OK)
        .modes(Polarization::Te, um(1.55))
        .remove(0);
    let steps = 402_000;
    let (mut num, mut den) = (0.0, 0.0);
    for s in 0..steps {
        let x = -2.01 + 4.02 * (f64::from(s) + 0.5) / f64::from(steps);
        let u = m.field(Length::um(x + SLAB / 2.0)).0;
        let eps: f64 = if x.abs() < SLAB / 2.0 {
            2.2 * 2.2
        } else {
            1.45 * 1.45
        };
        num += eps * u * u;
        den += u * u;
    }
    num / den / m.effective_index()
}

/// The group index on 10, 5 and 2.5 nm cells against the exact one: the finest grid's relative
/// error and the grid study's estimate of its own, relative.
pub(crate) fn group_index() -> (f64, f64) {
    let values: Vec<(f64, f64)> = [0.01, 0.005, 0.0025]
        .iter()
        .map(|&h| (h, slab_group_index(h)))
        .collect();
    let exact = slab_group_index_exact();
    let (value, error) = studied("slab group index", &values);
    ((value / exact - 1.0).abs(), error / exact)
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

fn bulk_limit_case() -> Outcome {
    outcome(bulk_limit(), 0.0, 3e-9)
}

fn plane_wave_case() -> Outcome {
    outcome(plane_wave(), 0.0, 1e-12)
}

fn slab_overlap_case() -> Outcome {
    let (error, ours, _) = slab_overlap();
    note_numerical(ours);
    outcome(error, 0.0, 1e-5)
}

fn slab_order_case() -> Outcome {
    let (_, _, order) = slab_overlap();
    outcome(order, 2.0, 0.1)
}

fn slab_coupled_case() -> Outcome {
    outcome(slab_coupled(1000), 0.0, 1e-10)
}

fn group_index_case() -> Outcome {
    let (error, ours) = group_index();
    note_numerical(ours);
    outcome(error, 0.0, 2e-5)
}

/// The cases: modal phase matching, the overlaps, and the pairs' group index.
pub(crate) fn cases() -> Vec<Case> {
    let mut cases = modal_cases();
    cases.extend(overlap_cases());
    cases.extend(pair_cases());
    cases
}

/// Modal phase matching: the plane-wave limit and Zhao et al.'s guide.
fn modal_cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/guide-bulk-limit",
            title: r"Congruent lithium niobate (Zelmon et al.'s indices) x-cut, filling a 0.2 µm square window between walls that hold a uniform field along c: SHG of 1.064 µm, type 0, from the plane-wave mode's effective indices interpolated at Chebyshev points over 1.04–1.09 and 0.52–0.545 µm, against the bulk crystal's $\Delta k$ from the same indices (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the crystal's own $\Delta k = k_{2\omega} - 2k_\omega$ (nonlinear::phase): a mode that is a plane wave has the bulk index",
            run: bulk_limit_case,
        },
        Case {
            id: "nonlinear/guide-zhao-period",
            title: r"Zhao et al. 2020's guide: 300 nm of x-cut 5 mol% MgO:LN (Zelmon et al.'s indices) on silica, etched 50 nm to a ridge 1.24 µm wide with 75° sidewalls, air above: the first-order period of SHG of 1.57 µm (and of its degenerate down-conversion) from the TE₀ modes at 1.57 and 0.785 µm on 20, 10 and 5 nm cells (µm, shown)",
            tier: Tier::Published,
            source: r"J. Zhao et al., Phys. Rev. Lett. 124, 163603 (2020), doi:10.1103/PhysRevLett.124.163603, p. 1: the calculated period 2.8 µm matching the TE modes at 1570 and 785 nm; the tolerance half its last digit",
            run: zhao_period_case,
        },
        Case {
            id: "nonlinear/guide-zhao-wavelength",
            title: r"The same guide poled at their 2.8 µm: the fundamental it phase-matches (µm, shown), from the TE₀ indices interpolated at Chebyshev points over 1.50–1.65 and 0.75–0.825 µm on 10 nm cells (and on 20 nm: the change is our error)",
            tier: Tier::Published,
            source: r"J. Zhao et al. 2020, p. 1: 1570 and 785 nm at $\Lambda$ = 2.8 µm; the tolerance the shift of the matched wavelength across half the period's last digit, 2.75 to 2.85 µm",
            run: zhao_wavelength_case,
        },
        Case {
            id: "nonlinear/guide-zhao-area-signal",
            title: r"Zhao et al. 2020's guide (above): the effective area of the TE₀ mode at 1.57 µm, $\lvert\int S_z\rvert^2/\int\lvert S_z\rvert^2$, on 20, 10 and 5 nm cells (µm², shown)",
            tier: Tier::Published,
            source: r"J. Zhao et al. 2020, p. 1: $A_\mathrm{eff}$ = 1.1 µm² at 1570 nm from their vectorial mode solver; the definition not given: this is S. Afshar V. and T. M. Monro's, Opt. Express 17, 2298 (2009), Eq. 41, which converges on these grids (their Eq. 44 from $\lvert E_t\rvert^4$ gives 1.103 on 5 nm cells, as close); the tolerance half its last digit",
            run: zhao_area_signal_case,
        },
        Case {
            id: "nonlinear/guide-zhao-area-pump",
            title: r"The same guide's TE₀ effective area at 0.785 µm, the pump's (µm², shown)",
            tier: Tier::Published,
            source: r"J. Zhao et al. 2020, p. 1: $A_\mathrm{eff}$ = 0.4 µm² at 785 nm; the tolerance half its last digit",
            run: zhao_area_pump_case,
        },
    ]
}

/// The overlap's κ and the efficiency along a guide.
fn overlap_cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/guide-plane-wave",
            title: r"The same window z-cut, the plane wave polarized along c at 1.064 and 0.532 µm, $d_{33}$ = 25.2 pm/V: the SHG efficiency $\eta = \kappa^2$ from the overlap of the fields normalized to 1 W, against $2\omega^2 d^2/(\varepsilon_0 c^3 n_\omega^2 n_{2\omega} A)$ over the window's area (relative difference shown)",
            tier: Tier::Analytic,
            source: r"R. W. Boyd, Nonlinear Optics, 4th ed. (2020), Eq. (2.7.11) with $I = 2n\varepsilon_0 c\lvert A\rvert^2$ (Eq. (2.2.16)) over the area; C. Wang et al., Optica 5, 1438 (2018), Eq. (1) with uniform modes",
            run: plane_wave_case,
        },
        Case {
            id: "nonlinear/guide-slab-overlap",
            title: r"A 400 nm z-cut lithium niobate slab in silica, c along its plane: the SHG efficiency of its TE₀ modes at 1.064 and 0.532 µm ($d_{33}$ = 25.2 pm/V in the film) from the full-vector fields on 10, 5 and 2.5 nm cells, against the overlap of the slab's exact cos/exp fields (the finest grid's relative error shown)",
            tier: Tier::Analytic,
            source: r"the slab's exact TE fields (mode::slab), $\hat e = u\thinspace(2Z_0/(n_\mathrm{eff}\int u^2))^{1/2}$, $O = d_{33}\int_\mathrm{film} \hat e_{2\omega}\hat e_\omega^2$, $\kappa = \omega\varepsilon_0\lvert O\rvert/2$",
            run: slab_overlap_case,
        },
        Case {
            id: "nonlinear/guide-slab-order",
            title: r"The same efficiency's order of convergence on 10, 5 and 2.5 nm cells (shown)",
            tier: Tier::Analytic,
            source: r"the full-vector scheme's second order where the interfaces are straight and on nodes",
            run: slab_order_case,
        },
        Case {
            id: "nonlinear/guide-slab-coupled",
            title: r"The slab's SHG over 4 mm from its own $\kappa$ (the overlap, 5 nm cells) and mismatch (its modes' indices) against a grating 0.1 % off, with 3 and 6 dB/cm of loss and 1 aW in (undepleted): the harmonic's power by the coupled equations (RK4, 1000 steps) against the undepleted lossy closed form (relative difference shown)",
            tier: Tier::Analytic,
            source: r"$P_{2\omega} = \eta P_0^2 \lvert e^{-\alpha_2 L/2}(e^{\gamma L} - 1)/\gamma\rvert^2$, $\gamma = -\alpha_1 + \alpha_2/2 - i\Delta k$ (nonlinear::coupled)",
            run: slab_coupled_case,
        },
    ]
}

/// The group index the joint spectrum takes.
fn pair_cases() -> Vec<Case> {
    vec![Case {
        id: "nonlinear/guide-group-index",
        title: r"A 400 nm slab of index 2.2 in 1.45 without material dispersion: its TE₀ group index at 1.55 µm from the effective index interpolated at Chebyshev points over 1.50–1.60 µm on 10, 5 and 2.5 nm cells, as the joint spectrum takes it, against the exact one (the finest grid's relative error shown)",
        tier: Tier::Analytic,
        source: r"$n_g = \langle\varepsilon\rangle/n_\mathrm{eff}$ from the slab's exact field (Hellmann–Feynman on the TE equation)",
        run: group_index_case,
    }]
}
