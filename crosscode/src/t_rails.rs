//! Kharel et al.'s segmented (T-rail) electrodes on thin-film lithium niobate on quartz (P.
//! Kharel, C. Reimer, K. Luke, L. He, M. Zhang, Optica 8, 357 (2021),
//! doi:10.1364/OPTICA.416155, Table 1's row d and Fig. 1; its erratum, Optica 8, 1218 (2021),
//! doi:10.1364/OPTICA.440484, corrects only Fig. 4(a)), in a perfectly conducting box: openEMS's
//! 3D FDTD on the electrodes as drawn, rails, stems and all, against photonoxide's periodic line
//! ([`photonoxide::electro::line::Periodic`]), its Bloch wave from 2D sections, on the same mesh
//! lines across.
//!
//! Row d: a signal 100 µm wide, the rails' gap g = 5 µm, each rail s = 2 µm wide across, h = 5
//! µm from its channel electrode, r = 45 µm long, c = 5 µm between rails along the line, joined
//! to its channel by a stem t = 5 µm wide in its middle; the channels g + 2(h + s) = 19 µm
//! apart, the grounds 100 µm wide (not given). The electrodes are 800 nm thick (gold, perfect
//! conductors here), on a 600 nm x-cut film etched 350 nm to a rib 2 µm wide in the rails' gap
//! (its sidewalls upright, so that the mesh lines follow them), on 2 µm of oxide (ε 3.9) over
//! 500 µm of quartz (ε 4.5, their Section 2), under 1 µm of oxide over the slab (their Section
//! 3, around the electrodes) and air. The film's clamped static permittivity is
//! [`crate::zhang_line::LN`]'s: 26.2 across (c), 45.5 up and along. The box is as wide as the
//! line, the grounds running to its sides; its floor under the quartz, its lid 200 µm above the
//! film.
//!
//! photonoxide's line is the library's model of these electrodes (its validation cases
//! line/kharel-2021-segmented-*): a period of half the channel line's c, the line loaded by the
//! rails (J. Shin et al., IEEE Trans. Microw. Theory Tech. 53, 636 (2005), Eq. (7): the
//! capacitance of the section through the rails, the channel line's inductance) r long, and
//! the other half; each section its static solver's on half the box, the stems left out; the
//! Bloch wave of the period's ABCD matrix at the period's start, the middle of a gap between
//! rails. openEMS runs a section [`LENGTH`] long, its mesh along it the same in every period,
//! driven across both channels' gaps at the middle of a gap between rails; the voltage across
//! a channel's gap at three planes [`SPACING`] apart at the middles of gaps between rails (whole
//! periods apart, so that the Bloch wave is the same there, and an even number of them, so
//! that the point midway is such a middle too) and the current around the signal and its rails
//! midway (the mean of the dual mesh's planes either side of it) give k and the Bloch Z₀.

use num_complex::Complex64 as c64;
use photonoxide::electro::line::{Metal, Periodic, QuasiTem};
use photonoxide::electro::{
    Axis, Boundary, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor,
};
use photonoxide::geometry::{Point, Shape};
use photonoxide::units::Length;

use crate::cpw::{C, Measured, Mesh};
use crate::line::{impedance, wavenumber};
use crate::openems::{self, Cuboid, Material, Measure, Model, Probe, Run, Source};
use crate::zhang_line::LN;

/// Quartz's relative permittivity, Kharel et al.'s.
pub const QUARTZ: f64 = 4.5;
/// The oxide's, under the film and over it.
pub const OXIDE: f64 = 3.9;

/// The periods along the section.
pub const PERIODS: usize = 26;
/// The section's length, µm, [`PERIODS`] of row d's 50 µm.
pub const LENGTH: f64 = 1300.0;
/// The source's plane, µm: the middle of the second gap between rails.
pub const SOURCE: f64 = 100.0;
/// The first voltage plane, µm: eight periods past the source, for the fields the source
/// leaves beside the line's own wave to die away (the quartz under the electrodes, a guide
/// whose first mode is cut off near 140 GHz, keeps them over some 170 µm at 50 GHz); the last
/// is four periods before the absorbing layer.
pub const FIRST: f64 = 500.0;
/// The voltage planes' spacing, µm: six periods, so that the current midway is three periods
/// on, at the middle of a gap between rails too.
pub const SPACING: f64 = 300.0;
/// The current loop's extent in height, µm: from inside the buried oxide to the air.
const LOOP: [f64; 2] = [-1.0, 3.0];
/// The leads at both ends, beyond the periodic section: the channel line alone, without rails,
/// in uniform cells, ended by Mur's absorbing condition. The pulse leaves a static field on the
/// line (its spectrum's tail at 0 Hz), and openEMS's PMLs at the ends grow unstable under it,
/// the energy rising without end some 70 to 90 ps in: at once where they end the rails' own
/// graded cells, later (on the 0.5 µm mesh) in the leads. With perfectly conducting ends the
/// same model holds; Mur's condition, which the static field doesn't trouble, absorbs the line's
/// wave meeting it head on, part of it reflected (the line's speed isn't the light's), which
/// the three-plane method takes as it takes any reflection.
pub const LEAD_CELLS: usize = 12;
/// The leads' cells, µm.
pub const LEAD_STEP: f64 = 5.0;

/// The pulse: openEMS's Gaussian of f₀ = 60 GHz, f_c = 50 GHz (10 to 110 GHz), short, for the
/// thin layers' small time step; the line's figures are taken at 10 to 50 GHz.
const PULSE: [f64; 2] = [60e9, 50e9];

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect("a rectangle of positive size")
}

/// The T-rail line in its box, µm: across from the signal's centre, up from the film's bottom,
/// along from a gap's middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TRails {
    /// The signal's half width.
    pub signal: f64,
    /// The rails' gap g.
    pub g: f64,
    /// The rails' distance from their channels, h: the stems' length.
    pub h: f64,
    /// The rails' width across, s.
    pub s: f64,
    /// The stems' width along the line, t.
    pub t: f64,
    /// The rails' length along the line, r.
    pub r: f64,
    /// The gap between rails along the line, c.
    pub c: f64,
    /// The grounds' widths.
    pub ground: f64,
    /// The electrodes' thickness.
    pub metal: f64,
    /// The film's thickness.
    pub film: f64,
    /// The slab left beside the rib.
    pub slab: f64,
    /// The rib's width.
    pub rib: f64,
    /// The buried oxide's thickness.
    pub buried: f64,
    /// The quartz's thickness.
    pub quartz: f64,
    /// The oxide's thickness over the slab.
    pub clad: f64,
    /// The box's lid's height over the film's bottom.
    pub lid: f64,
}

/// Where the conductors are across, µm from the signal's centre (the right half).
struct Across {
    /// The signal's edge.
    signal: f64,
    /// The signal's rail.
    signal_rail: [f64; 2],
    /// The rib.
    rib: [f64; 2],
    /// The ground's rail.
    ground_rail: [f64; 2],
    /// The ground's inner edge.
    ground: f64,
    /// The box's side, the ground's outer edge.
    reach: f64,
}

impl TRails {
    /// Kharel et al.'s row d.
    pub fn kharel_d() -> TRails {
        TRails {
            signal: 50.0,
            g: 5.0,
            h: 5.0,
            s: 2.0,
            t: 5.0,
            r: 45.0,
            c: 5.0,
            ground: 100.0,
            metal: 0.8,
            film: 0.6,
            slab: 0.25,
            rib: 2.0,
            buried: 2.0,
            quartz: 500.0,
            clad: 1.0,
            lid: 200.0,
        }
    }

    /// The period, µm: a rail and the gap after it.
    pub fn period(&self) -> f64 {
        self.r + self.c
    }

    /// The channels' gap, g + 2(h + s).
    pub fn channel_gap(&self) -> f64 {
        self.g + 2.0 * (self.h + self.s)
    }

    /// The box's half width.
    pub fn reach(&self) -> f64 {
        self.signal + self.channel_gap() + self.ground
    }

    fn across(&self) -> Across {
        let a = self.signal;
        let b = a + self.channel_gap();
        let centre = 0.5 * (a + b);
        Across {
            signal: a,
            signal_rail: [a + self.h, a + self.h + self.s],
            rib: [centre - 0.5 * self.rib, centre + 0.5 * self.rib],
            ground_rail: [b - self.h - self.s, b - self.h],
            ground: b,
            reach: self.reach(),
        }
    }

    fn floor(&self) -> f64 {
        -self.buried - self.quartz
    }

    fn top(&self) -> f64 {
        self.slab + self.metal
    }

    /// The mesh lines across (from the mirror at 0 to the box's side) and in height (from the
    /// box's floor to its lid) on `mesh`, and along one period (from a gap's middle) on
    /// `along`, fine at every edge.
    ///
    /// # Errors
    ///
    /// When a mesh isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh, along: &Mesh) -> photonoxide::Result<(Axis, Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let w = self.across();
        // the signal's middle too, where the half model's magnetic wall is: its current
        // loops start at the dual lines beside it
        let across = [
            0.0,
            w.signal,
            w.signal_rail[0],
            w.signal_rail[1],
            w.rib[0],
            w.rib[1],
            w.ground_rail[0],
            w.ground_rail[1],
            w.ground,
        ];
        let up = [0.0, self.slab, self.film, self.top(), self.slab + self.clad];
        let mut heights = up.to_vec();
        heights.push(-self.buried);
        let along = Grading::new(along.largest, along.smallest, along.growth)?;
        // half a period, from a gap's middle to a stem's, mirrored about the stem's middle:
        // the same either side of every middle
        let half = 0.5 * self.period();
        let (r0, h0) = (0.5 * self.c, half - 0.5 * self.t);
        let first = Axis::graded(0.0, half, &along, &[r0, h0], &[r0, h0])?;
        let mut period = first.nodes().to_vec();
        period.extend(first.nodes().iter().rev().skip(1).map(|v| 2.0 * half - v));
        Ok((
            Axis::graded(0.0, w.reach, &g, &across, &across)?,
            Axis::graded(self.floor(), self.lid, &g, &heights, &up)?,
            Axis::new(period)?,
        ))
    }

    /// The electrostatic problem of a section on the mesh lines `across` and `height`: the
    /// signal (conductor 0) and the ground (1), and with `rails` the signal's rail (2) and the
    /// ground's (3); the box's walls at the ground's potential, the mirror insulated.
    fn problem(
        &self,
        across: &Axis,
        height: &Axis,
        rails: bool,
        vacuum: bool,
    ) -> photonoxide::Result<Electrostatic> {
        let w = self.across();
        let mut section = Section::new("air");
        section
            .layer(self.floor(), -self.buried, "quartz")?
            .layer(-self.buried, 0.0, "box")?
            .layer(self.slab, self.slab + self.clad, "clad")?
            .layer(0.0, self.slab, "LN")?;
        section.draw(rect(w.rib, [self.slab, self.film]), "LN");
        let mut problem = Electrostatic::new(section, Grid::new(across.clone(), height.clone()));
        let z = [self.slab, self.top()];
        let eps = |t: Tensor| if vacuum { Tensor::isotropic(1.0) } else { t };
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("quartz", eps(Tensor::isotropic(QUARTZ)))
            .permittivity("box", eps(Tensor::isotropic(OXIDE)))
            .permittivity("clad", eps(Tensor::isotropic(OXIDE)))
            .permittivity("LN", eps(Tensor::diagonal(LN[0], LN[1])))
            .boundary(Side::Right, Boundary::Fixed(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .conductor(Conductor::shape(rect([-1.0, w.signal], z), 1.0))
            .conductor(Conductor::shape(rect([w.ground, w.reach + 1.0], z), 0.0));
        if rails {
            problem
                .conductor(Conductor::shape(rect(w.signal_rail, z), 1.0))
                .conductor(Conductor::shape(rect(w.ground_rail, z), 0.0));
        }
        Ok(problem)
    }

    /// photonoxide's sections on the mesh lines `across` and `height`, lossless (the
    /// electrodes perfect conductors, as openEMS's): the channel line and the line loaded by
    /// the rails, as [`QuasiTem::loaded`] has it (its capacitance the section's through the
    /// rails, the signal and its rail at 1 V; its inductance the channel line's, 1/(c²C₀)), from
    /// the static solver's capacitances on half the box, the whole line's twice the half's.
    ///
    /// # Errors
    ///
    /// When a solve fails.
    pub fn sections(
        &self,
        across: &Axis,
        height: &Axis,
    ) -> photonoxide::Result<(QuasiTem, QuasiTem)> {
        let held = |rails: bool, vacuum: bool| -> photonoxide::Result<f64> {
            let c = self.problem(across, height, rails, vacuum)?.capacitance()?;
            let signal: &[usize] = if rails { &[0, 2] } else { &[0] };
            Ok(2.0
                * signal
                    .iter()
                    .map(|&i| signal.iter().map(|&j| c[i][j]).sum::<f64>())
                    .sum::<f64>())
        };
        let vacuum = held(false, true)?;
        let inductance = 1.0 / (C * C * vacuum);
        Ok((
            QuasiTem::ideal(inductance, held(false, false)?)?,
            QuasiTem::ideal(inductance, held(true, false)?)?,
        ))
    }

    /// photonoxide's Bloch index and impedance (Ω) on the mesh lines `across` and `height`:
    /// the period of half the channel line's c, the loaded line's r and the other half, at 1
    /// GHz, where the Bloch wave is the static line's (its Bragg frequency is above 1 THz).
    ///
    /// # Errors
    ///
    /// When a solve fails.
    pub fn bloch(&self, across: &Axis, height: &Axis) -> photonoxide::Result<(f64, f64)> {
        let (ch, ld) = self.sections(across, height)?;
        let p = Periodic::new(vec![
            (ch.clone(), 0.5 * self.c),
            (ld, self.r),
            (ch, 0.5 * self.c),
        ])?;
        // an ideal line carries no current: the metal is never used
        let metal = Metal::new(1.0, self.metal)?;
        let w = p.bloch(1e9, &metal, &[])?;
        Ok((w.index(), w.impedance.re))
    }

    /// The 2D sections' upper bound on the rails' fringing: the index and impedance (Ω) of the
    /// line loaded as the rails are all along, every gap between rails too (the bound the
    /// library's validation cases give for these electrodes).
    ///
    /// # Errors
    ///
    /// When a solve fails.
    pub fn loaded_bound(&self, across: &Axis, height: &Axis) -> photonoxide::Result<(f64, f64)> {
        let (_, ld) = self.sections(across, height)?;
        Ok((ld.index(), ld.impedance()))
    }

    /// The openEMS model on the mesh lines `across` (half the line from its middle; the whole
    /// mirrors them), `height` and `along` (one period, repeated), ended at an energy of
    /// `end_energy`: the whole line, or its half beside a magnetic wall through the signal's
    /// middle, which the even mode's symmetry makes.
    pub fn model(
        &self,
        across: &Axis,
        height: &Axis,
        along: &Axis,
        end_energy: f64,
        extent: Extent,
    ) -> Model {
        let w = self.across();
        let rr = w.reach;
        let half = across.nodes();
        let whole = extent == Extent::Whole;
        let y: Vec<f64> = if whole {
            let mut y: Vec<f64> = half.iter().skip(1).rev().map(|v| -v).collect();
            y.extend_from_slice(half);
            y
        } else {
            half.to_vec()
        };
        let p = self.period();
        let mut x: Vec<f64> = (0..LEAD_CELLS)
            .map(|i| -LEAD_STEP * (LEAD_CELLS - i) as f64)
            .collect();
        x.push(0.0);
        for k in 0..PERIODS {
            x.extend(along.nodes().iter().skip(1).map(|v| v + k as f64 * p));
        }
        x.extend((1..=LEAD_CELLS).map(|i| LENGTH + LEAD_STEP * i as f64));
        // a box along the whole section; in the half, its part beside the wall (none if it
        // lies beyond it)
        let mut boxes = Vec::new();
        let full = |list: &mut Vec<Cuboid>, y0: f64, y1: f64, z0: f64, z1: f64, x: [f64; 2]| {
            let y0 = if whole { y0 } else { y0.max(0.0) };
            if y1 > y0 {
                list.push(Cuboid {
                    from: [x[0], y0, z0],
                    to: [x[1], y1, z1],
                });
            }
        };
        let lead = LEAD_STEP * LEAD_CELLS as f64;
        let all = [-lead, LENGTH + lead];
        let at = |x: f64, y0: f64, y1: f64, z0: f64, z1: f64| Cuboid {
            from: [x, y0, z0],
            to: [x, y1, z1],
        };
        let (slab, film, clad, top) = (self.slab, self.film, self.slab + self.clad, self.top());
        let [b0, b1] = w.rib;
        let mut material = |name: &str, epsilon: [f64; 3], spans: &[[f64; 4]]| {
            boxes.clear();
            for &[y0, y1, z0, z1] in spans {
                full(&mut boxes, y0, y1, z0, z1, all);
            }
            Material {
                name: name.into(),
                epsilon,
                kappa: 0.0,
                boxes: boxes.clone(),
            }
        };
        let materials = vec![
            material(
                "quartz",
                [QUARTZ; 3],
                &[[-rr, rr, self.floor(), -self.buried]],
            ),
            material("oxide", [OXIDE; 3], &[[-rr, rr, -self.buried, 0.0]]),
            // along the line, across (c), up
            material(
                "film",
                [LN[1], LN[0], LN[1]],
                &[
                    [-rr, rr, 0.0, slab],
                    [b0, b1, slab, film],
                    [-b1, -b0, slab, film],
                ],
            ),
            // around the ribs, which are the film's; the electrodes are metal over it
            material(
                "cladding",
                [OXIDE; 3],
                &[
                    [-rr, -b1, slab, film],
                    [-b0, b0, slab, film],
                    [b1, rr, slab, film],
                    [-rr, rr, film, clad],
                ],
            ),
        ];
        let (s, gnd) = (w.signal, w.ground);
        let mut metal = Vec::new();
        for [y0, y1] in [[-s, s], [gnd, rr], [-rr, -gnd]] {
            full(&mut metal, y0, y1, slab, top, all);
        }
        let (h0, h1) = (0.5 * (p - self.t), 0.5 * (p + self.t));
        let sides: &[f64] = if whole { &[1.0, -1.0] } else { &[1.0] };
        for k in 0..PERIODS {
            let x0 = k as f64 * p;
            let rail = [x0 + 0.5 * self.c, x0 + p - 0.5 * self.c];
            let stem = [x0 + h0, x0 + h1];
            for &sign in sides {
                let span = |[a, b]: [f64; 2]| if sign > 0.0 { [a, b] } else { [-b, -a] };
                for r in [w.signal_rail, w.ground_rail] {
                    let [lo, hi] = span(r);
                    full(&mut metal, lo, hi, slab, top, rail);
                }
                // the stems, from each channel to its rail
                for r in [[w.signal, w.signal_rail[0]], [w.ground_rail[1], w.ground]] {
                    let [lo, hi] = span(r);
                    full(&mut metal, lo, hi, slab, top, stem);
                }
            }
        }
        let mut probes = Vec::new();
        for k in 0..3 {
            // across the right channels' gap, signal to ground, at the rib's top
            probes.push(Probe {
                name: format!("v{}", k + 1),
                measure: Measure::Voltage,
                at: at(FIRST + k as f64 * SPACING, s, gnd, film, film),
            });
        }
        let middle = 0.5 * (w.signal_rail[1] + w.ground_rail[0]);
        let dual = dual_offsets(along);
        // the loops' first sides: through the gap on the left in the whole; in the half, the
        // first two dual lines beside the wall
        let starts: Vec<(&str, f64)> = if whole {
            vec![("", -middle)]
        } else {
            vec![("1", 0.5 * half[1]), ("2", 0.5 * (half[1] + half[2]))]
        };
        for k in 0..2 {
            let x = FIRST + (k as f64 + 0.5) * SPACING;
            for (side, d) in [("a", dual[0]), ("b", dual[1])] {
                for &(start, y0) in &starts {
                    probes.push(Probe {
                        name: format!("i{}{side}{start}", k + 1),
                        measure: Measure::Current { normal: 0 },
                        at: at(x + d, y0, middle, LOOP[0], LOOP[1]),
                    });
                }
            }
        }
        let mut sources = vec![Source {
            name: "gap_right".into(),
            direction: [0.0, 1.0, 0.0],
            at: at(SOURCE, s, gnd, slab, top),
        }];
        if whole {
            sources.push(Source {
                name: "gap_left".into(),
                direction: [0.0, -1.0, 0.0],
                at: at(SOURCE, -gnd, -s, slab, top),
            });
        }
        Model {
            unit_m: 1e-6,
            lines: [x, y, height.nodes().to_vec()],
            boundaries: [
                openems::Boundary::Mur,
                openems::Boundary::Mur,
                if whole {
                    openems::Boundary::Pec
                } else {
                    openems::Boundary::Pmc
                },
                openems::Boundary::Pec,
                openems::Boundary::Pec,
                openems::Boundary::Pec,
            ],
            pulse: PULSE,
            max_steps: 20_000_000,
            end_energy,
            materials,
            metal,
            sources,
            probes,
        }
    }
}

/// What openEMS runs: the whole line, or its half beside a magnetic wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// The whole line, its sides across perfect conductors.
    Whole,
    /// The half beside the signal's middle, a perfect magnetic conductor there.
    Half,
}

/// The dual mesh's planes either side of a gap's middle (the period's start), as offsets from
/// it: half the cells before and after it.
fn dual_offsets(along: &Axis) -> [f64; 2] {
    let n = along.nodes();
    let p = n[n.len() - 1];
    [-0.5 * (p - n[n.len() - 2]), 0.5 * n[1]]
}

/// openEMS's k and Bloch Z₀ at each of `frequencies_ghz` from `run`'s probes on a model of
/// `extent` on the mesh lines `across`. The current midway is the mean of the dual planes either
/// side of it, which a uniform stretch of line gives to second order in the half cell (the gap
/// between rails is one). In the half, the loops start at the first two dual lines beside the
/// wall, and the current between the wall and the first, I₁ − I₂ over the cell between them in
/// proportion (the signal's current flat at its middle), is added before the half is doubled.
///
/// # Errors
///
/// When a probe's file is missing or malformed.
pub fn measured(
    run: &Run,
    frequencies_ghz: &[f64],
    extent: Extent,
    across: &Axis,
) -> Result<Vec<Measured>, String> {
    let v = [run.probe("v1")?, run.probe("v2")?, run.probe("v3")?];
    let y = across.nodes();
    // the first dual line's distance from the wall over the second's from the first
    let sliver = y[1] / y[2];
    let mut i = Vec::new();
    for k in 1..=2 {
        let mut planes = Vec::new();
        for side in ["a", "b"] {
            planes.push(match extent {
                Extent::Whole => vec![run.probe(&format!("i{k}{side}"))?],
                Extent::Half => vec![
                    run.probe(&format!("i{k}{side}1"))?,
                    run.probe(&format!("i{k}{side}2"))?,
                ],
            });
        }
        i.push(planes);
    }
    let current = |planes: &Vec<Vec<openems::Signal>>, hz: f64| {
        let one = |loops: &Vec<openems::Signal>| match extent {
            Extent::Whole => loops[0].spectrum(hz),
            Extent::Half => {
                let (a, b) = (loops[0].spectrum(hz), loops[1].spectrum(hz));
                2.0 * (a + sliver * (a - b))
            }
        };
        0.5 * (one(&planes[0]) + one(&planes[1]))
    };
    let d = SPACING * 1e-6;
    Ok(frequencies_ghz
        .iter()
        .map(|&f| {
            let hz = f * 1e9;
            let vf = v.each_ref().map(|s| s.spectrum(hz));
            let jf = [current(&i[0], hz), current(&i[1], hz)];
            let k = wavenumber(vf, d);
            let z: [c64; 2] = [
                impedance([vf[0], vf[1]], jf[0], k, d),
                impedance([vf[1], vf[2]], jf[1], k, d),
            ];
            Measured {
                frequency_ghz: f,
                wavenumber: k,
                impedance: z,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a coarse mesh: the Bloch index near Kharel et al.'s 2.26 and Z₀ near their 41 Ω; the
    /// model's planes at gaps' middles, on the mesh lines, the currents on the dual mesh either
    /// side; the half model beside its magnetic wall.
    #[test]
    fn the_bloch_line_and_the_model_share_the_lines() {
        let line = TRails::kharel_d();
        let (y, z, x) = line
            .axes(
                &Mesh {
                    smallest: 1.0,
                    growth: 1.2,
                    largest: 25.0,
                },
                &Mesh {
                    smallest: 2.0,
                    growth: 1.3,
                    largest: 5.0,
                },
            )
            .unwrap();
        let (n, ohm) = line.bloch(&y, &z).unwrap();
        assert!(
            (n - 2.1).abs() < 0.2 && (ohm - 41.0).abs() < 8.0,
            "{n} {ohm}"
        );
        let m = line.model(&y, &z, &x, 1e-7, Extent::Whole);
        let on = |v: f64| m.lines[0].iter().any(|&l| (l - v).abs() < 1e-9);
        assert!(on(FIRST) && on(FIRST + SPACING) && on(FIRST + 2.0 * SPACING) && on(SOURCE));
        let lead = LEAD_STEP * LEAD_CELLS as f64;
        assert!((m.lines[0][m.lines[0].len() - 1] - LENGTH - lead).abs() < 1e-9);
        assert!((m.lines[0][0] + lead).abs() < 1e-9);
        let d = dual_offsets(&x);
        assert!(d[0] < 0.0 && d[1] > 0.0 && (d[0] + d[1]).abs() < 1e-9);
        assert!(m.xml().is_ok());
        let h = line.model(&y, &z, &x, 1e-7, Extent::Half);
        assert_eq!(h.lines[1].len(), y.nodes().len());
        assert_eq!(h.boundaries[2], openems::Boundary::Pmc);
        assert!(h.metal.iter().all(|b| b.from[1] >= 0.0));
        assert!(h.xml().unwrap().contains("i2b2"));
    }
}
