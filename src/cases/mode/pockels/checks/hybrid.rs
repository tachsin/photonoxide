//! Valdez et al. 2023's hybrid modulator (Optica 10, 578): an unetched 200 nm x-cut film of
//! MgO-doped lithium niobate bonded, with 40 nm of oxide between them, onto a planarized silicon
//! nitride strip (0.9 µm wide, 0.18 µm thick) in silica on silicon, gold electrodes on the film
//! and air above, light at 784 nm. The strip carries the mode, the film is the active material:
//! no lithium niobate is etched.
//!
//! The case is the paper's push–pull V_π L of 0.8 V·cm, the electrodes' gap its 4 µm. The
//! paper's simulated numbers for its Fig. 1(e) mode (n_eff 1.90, the power in the film and the
//! strip 0.69 and 0.15, n_g about 2.24) are not cases: they hang on the nitride's index, which
//! the main text doesn't give.

use super::*;

/// The wavelength of the paper's measurements, µm.
const LAMBDA_NIR: f64 = 0.784;

/// The static relative permittivity of silicon nitride (LPCVD).
const NITRIDE_EPS: f64 = 7.5;

/// The cross-section's dimensions, µm.
#[derive(Clone, Copy, Debug)]
pub(super) struct Hybrid {
    /// The strip's width.
    pub width: f64,
    /// Its thickness.
    pub height: f64,
    /// The oxide between the strip's top and the film.
    pub bond: f64,
    /// The film's thickness.
    pub film: f64,
    /// The oxide below the strip.
    pub buried: f64,
    /// The electrodes' gap.
    pub gap: f64,
    /// The signal electrode's half width.
    pub signal: f64,
    /// The ground electrode's width.
    pub ground: f64,
    /// The electrodes' thickness.
    pub metal: f64,
}

/// Valdez et al.'s (Fig. 1 and 2): a 0.9 µm strip of 0.18 µm, a 40 nm bond, a 200 nm film, 4 µm
/// of oxide below the strip, a 4 µm gap between the electrodes. The paper's main text gives
/// neither the electrodes' thickness (1 µm here) nor their widths.
pub(super) fn valdez_2023() -> Hybrid {
    Hybrid {
        width: 0.9,
        height: 0.18,
        bond: 0.04,
        film: 0.2,
        buried: 4.0,
        gap: 4.0,
        signal: 25.0,
        ground: 25.0,
        metal: 1.0,
    }
}

/// The materials, at `wavelength`: the film's permittivity tensor in the section's axes, silica's
/// and the strip's.
struct Materials {
    crystal: [[c64; 3]; 3],
    oxide: c64,
    nitride: c64,
}

fn materials(wavelength: Wavelength) -> Materials {
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).expect(OK);
    let crystal = entry("linbo3-mgo")
        .expect(OK)
        .oriented(None, Conditions::default(), x_cut)
        .expect(OK);
    let crystal = Along::Y.tensor(crystal.tensor(wavelength).expect(OK));
    let eps = |id: &str| {
        entry(id)
            .expect(OK)
            .default_model()
            .expect(OK)
            .materials(Conditions::default())
            .expect(OK)
            .remove(0)
            .permittivity(wavelength)
            .expect(OK)
    };
    Materials {
        crystal,
        oxide: eps("sio2"),
        nitride: eps("si3n4"),
    }
}

/// The film's electro-optic tensor: Akiyama et al.'s r₁₃ and r₃₃ of 5 mol% MgO-doped
/// congruent crystals (unclamped, 633 nm), the other elements from the bulk's unclamped set.
fn mgo_pockels() -> Pockels {
    let mgo = entry("linbo3-mgo")
        .expect(OK)
        .tensors
        .into_iter()
        .find(|t| t.label == "r₁₃ and r₃₃, bulk, unclamped, 633 nm")
        .expect("a tensor of the catalogue");
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).expect(OK);
    Pockels::completed(&mgo, &linbo3_tensor("bulk, unclamped, 633 nm"))
        .expect(OK)
        .rotated(&x_cut)
        .in_section(Along::Y)
}

/// Which material a cell holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Region {
    Film,
    Strip,
    Oxide,
    Air,
}

impl Hybrid {
    /// The region of (x, y); y = 0 at the film's bottom.
    fn region(&self, x: f64, y: f64) -> Region {
        if (0.0..self.film).contains(&y) {
            Region::Film
        } else if y >= self.film {
            Region::Air
        } else if x.abs() < 0.5 * self.width && (-self.bond - self.height..-self.bond).contains(&y)
        {
            Region::Strip
        } else {
            Region::Oxide
        }
    }

    /// The optical cross-section on cells `h` fine at `wavelength`, and each cell's region.
    pub(super) fn optics(&self, h: f64, wavelength: Wavelength) -> (CrossSection, Vec<Region>) {
        let m = materials(wavelength);
        let top = -self.bond - self.height;
        let xs = graded_nodes(
            -4.0,
            4.0,
            (-1.5, 1.5),
            h,
            0.08,
            0.1,
            &[-0.5 * self.width, 0.5 * self.width],
        );
        let ys = graded_nodes(
            top - 1.5,
            self.film + 1.5,
            (top - 0.3, self.film + 0.3),
            h,
            0.08,
            0.1,
            &[top, -self.bond, 0.0, self.film],
        );
        let zero = c64::new(0.0, 0.0);
        let iso = |e: c64| [[e, zero, zero], [zero, e, zero], [zero, zero, e]];
        let mut cells = Vec::new();
        let mut regions = Vec::new();
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let (x, y) = (0.5 * (xs[i] + xs[i + 1]), 0.5 * (ys[j] + ys[j + 1]));
                let region = self.region(x, y);
                let t = match region {
                    Region::Film => m.crystal,
                    Region::Strip => iso(m.nitride),
                    Region::Oxide => iso(m.oxide),
                    Region::Air => iso(c64::new(1.0, 0.0)),
                };
                cells.push(Permittivity::from_tensor(t).expect(OK));
                regions.push(region);
            }
        }
        (CrossSection::new(xs, ys, cells).expect(OK), regions)
    }

    /// The electrostatic problem at `volts` on the signal, the ground at 0, one gap as in
    /// [`Tfln::electrostatic`], its grid graded by `grading` from the strip's and the
    /// electrodes' corners.
    fn electrostatic(&self, grading: &Grading, volts: f64) -> Electrostatic {
        let (perpendicular, along_c) = Coefficients::Unclamped.static_permittivity();
        let left = -0.5 * self.gap - self.ground;
        let right = 0.5 * self.gap + self.signal;
        let box_bottom = -self.bond - self.height - self.buried;
        let (bottom, top) = (box_bottom - SUBSTRATE, self.film + self.metal + AIR);
        let mut section = Section::new("air");
        section
            .layer(bottom, box_bottom, "Si")
            .and_then(|s| s.layer(box_bottom, 0.0, "box"))
            .and_then(|s| s.layer(0.0, self.film, "LN"))
            .expect(OK);
        let strip = Tfln::rect(
            [-0.5 * self.width, 0.5 * self.width],
            [-self.bond - self.height, -self.bond],
        );
        section.draw(strip, "SiN");
        let ground = Tfln::rect(
            [left - 1.0, -0.5 * self.gap],
            [self.film, self.film + self.metal],
        );
        let signal = Tfln::rect(
            [0.5 * self.gap, right + 1.0],
            [self.film, self.film + self.metal],
        );
        let mut features: Features = section.features().clone();
        features
            .shape(&ground)
            .shape(&signal)
            .line_x(left)
            .line_x(right);
        for x in [-1.5 * self.width, 1.5 * self.width] {
            features.corner(x, 0.0).corner(x, self.film);
        }
        let grid = Grid::fitted([left, right], [bottom, top], grading, &features).expect(OK);
        let mut problem = Electrostatic::new(section, grid);
        problem
            .permittivity("air", Static::isotropic(1.0))
            .permittivity("Si", Static::isotropic(SILICON_EPS))
            .permittivity("box", Static::isotropic(OXIDE_EPS))
            .permittivity("SiN", Static::isotropic(NITRIDE_EPS))
            // x-cut, c across: across ε₃₃, up ε₁₁
            .permittivity("LN", Static::diagonal(along_c, perpendicular))
            .conductor(Conductor::shape(ground, 0.0))
            .conductor(Conductor::shape(signal, volts))
            .boundary(Side::Left, Boundary::INSULATED)
            .boundary(Side::Right, Boundary::INSULATED);
        problem
    }

    /// The film's Δε per volt on `cs`, from the static field.
    fn change(
        &self,
        cs: &CrossSection,
        regions: &[Region],
        grading: &Grading,
        wavelength: Wavelength,
    ) -> Vec<Permittivity> {
        let p = self.electrostatic(grading, 1.0);
        let phi = p.solve().expect(OK);
        let field = p.electric_field(&phi).expect(OK);
        let r = mgo_pockels();
        let crystal = materials(wavelength).crystal;
        let (xs, ys) = (cs.x(), cs.y());
        let mut out = Vec::with_capacity(cs.cells().len());
        let mut k = 0;
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let mut sum = [[c64::new(0.0, 0.0); 3]; 3];
                if regions[k] == Region::Film {
                    for (x, y) in Optics::samples(xs[i], xs[i + 1], ys[j], ys[j + 1]) {
                        let e = field.at(x, y);
                        let d = r.permittivity_change(crystal, [e[0], e[1], 0.0]);
                        for a in 0..3 {
                            for b in 0..3 {
                                sum[a][b] += d[a][b];
                            }
                        }
                    }
                }
                let n = (SAMPLES * SAMPLES) as f64;
                out.push(section_change(sum.map(|row| row.map(|v| v / n))));
                k += 1;
            }
        }
        out
    }
}

/// The TE-like fundamental mode of `cs` at `wavelength`.
fn te_mode_at(cs: &CrossSection, wavelength: Wavelength) -> VectorMode {
    let modes = vector::modes(cs, wavelength, 3, None).expect(OK);
    modes
        .into_iter()
        .find(|m| m.te_fraction() > 0.5)
        .expect("a TE-like mode")
}

/// The push–pull V_π·L (V·cm) of `hybrid` at 784 nm, the static grid's smallest cells `s` and
/// the optical cells `h` fine.
pub(super) fn modulation_nir(hybrid: &Hybrid, s: f64, h: f64) -> f64 {
    let w = Wavelength::um(LAMBDA_NIR).expect(OK);
    let (cs, regions) = hybrid.optics(h, w);
    let mode = te_mode_at(&cs, w);
    let delta = hybrid.change(&cs, &regions, &Grading::new(1.0, s, 1.15).expect(OK), w);
    let per_volt = index_change(&mode, &cs, &delta).expect(OK).re;
    100.0 * half_wave_voltage_length(per_volt, w, Drive::PushPull).expect(OK)
}

/// Valdez et al. 2023: their push–pull V_π L of 0.8 V·cm at 784 nm (2.0 V on 0.4 cm, 1.0 V on
/// 0.8 cm), both grids refined together (cells 20, 10 and 5 nm) and gated at 0.5%.
pub(super) fn valdez_2023_vpi_l() -> Outcome {
    let mut study = crate::convergence::Study::new("Vpi L");
    for h in [0.02, 0.01, 0.005] {
        study = study.on(h, modulation_nir(&valdez_2023(), h, h)).expect(OK);
    }
    let c = study.check(5e-3 * study.values[2]).expect(OK);
    outcome(c.gated().expect("a converged grid"), 0.8, 0.05)
}
