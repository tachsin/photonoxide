//! A simulation's state on the GPU: its fields, coefficients and CPMLs in buffers there, and a
//! run's sources, probes and transforms, stepped in submissions of several steps each.

use std::collections::HashMap;
use std::sync::mpsc;

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::{Gpu, Precision, gpu_error, wait};
use crate::Result;
use crate::fdfd::Axis;
use crate::fdtd::{Field, Simulation};

/// Values converted, written and read back a chunk at a time.
const CHUNK: usize = 1 << 22;

/// The device's memory a simulation leaves free, bytes: for the copies to and from it, a chunk
/// at a time, and for a run's sources, probes and monitors.
const HEADROOM: usize = 256 << 20;

/// About the cell-updates one submission takes: some hundredths of a second, so the CPU has the
/// next one ready before the GPU is done and no submission comes near the system's watchdog.
const SUBMISSION: usize = 1 << 27;

/// The most steps one submission takes.
const MOST_STEPS: usize = 256;

/// The stride of a step's uniform: the alignment of dynamic offsets, at most 256 bytes.
const STEP_STRIDE: usize = 256;

/// The planes along z a workgroup marches through, in f32 and f64: what measured fastest on an
/// RTX 4060 (fewer planes make more workgroups, but each recomputes the plane before its first).
const PLANES: [usize; 2] = [4, 8];

/// The CPML slabs' table: per field, component, axis and side, the slab's first index, its
/// count, where its ψ starts and where its b and a start.
type Slabs = [[u32; 4]; 36];

/// A simulation's grid on the GPU: E, H̃, E's coefficients, the CPMLs' ψ and recursions, and the
/// constants of the curl.
pub(in crate::fdtd) struct Resident {
    gpu: Gpu,
    cells: usize,
    params: wgpu::Buffer,
    /// E, H̃ and ψ, and their second copies for the fused step (16 bytes each without it).
    e: [wgpu::Buffer; 2],
    h: [wgpu::Buffer; 2],
    psi: [wgpu::Buffer; 2],
    ca: wgpu::Buffer,
    cb: wgpu::Buffer,
    reals: wgpu::Buffer,
    coefs: wgpu::Buffer,
    /// Whether a step is one pass from one copy to the other, or two in place.
    pub(in crate::fdtd) fused: bool,
    /// The copy that holds the fields now.
    parity: usize,
    /// Where each of the simulation's slabs' ψ starts.
    psi_offsets: Vec<usize>,
    slabs: Slabs,
    /// The CPMLs' cells at each axis's low and high ends.
    low: [u32; 3],
    high: [u32; 3],
    /// The planes along z a workgroup marches through.
    pub(in crate::fdtd) planes: usize,
    /// Whether the medium has no conductivity (ca 1 wherever cb isn't 0, and 0 where it is):
    /// ca then isn't uploaded or read.
    lossless: bool,
}

/// A run's sources, probes and transforms on the GPU, and the buffers they and its steps use.
struct Plan {
    /// Values the sources and currents add to on H̃, then on E.
    groups: [usize; 2],
    emitters: usize,
    probes: usize,
    series: usize,
    /// The values transformed, the phases a step and the sums.
    values: usize,
    phases: usize,
    sums: usize,
    /// Each transform monitor's angular frequencies.
    dfts: Vec<Vec<f64>>,
    /// Steps a submission.
    batch: usize,
    waves: wgpu::Buffer,
    phase_buffer: wgpu::Buffer,
    steps: wgpu::Buffer,
    recorded: wgpu::Buffer,
    sum_buffer: wgpu::Buffer,
    /// The fields' bindings with each copy the current one, and the steps'.
    fields: [wgpu::BindGroup; 2],
    steps_group: wgpu::BindGroup,
}

/// `values` as `precision`'s little-endian bytes.
fn bytes(values: &[f64], precision: Precision) -> Vec<u8> {
    let size = precision.bytes();
    let mut out = vec![0u8; values.len() * size];
    out.par_chunks_mut(size * 4096)
        .zip(values.par_chunks(4096))
        .for_each(|(out, values)| {
            for (o, &v) in out.chunks_exact_mut(size).zip(values) {
                match precision {
                    Precision::Single => o.copy_from_slice(&(v as f32).to_le_bytes()),
                    Precision::Double => o.copy_from_slice(&v.to_le_bytes()),
                }
            }
        });
    out
}

/// `bytes` of `precision`'s values into `out`.
fn values(bytes: &[u8], precision: Precision, out: &mut [f64]) {
    let size = precision.bytes();
    out.par_chunks_mut(4096)
        .zip(bytes.par_chunks(size * 4096))
        .for_each(|(out, bytes)| {
            for (o, b) in out.iter_mut().zip(bytes.chunks_exact(size)) {
                *o = match precision {
                    Precision::Single => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
                    Precision::Double => {
                        f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                    }
                };
            }
        });
}

fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn u32_of(n: usize, what: &str) -> Result<u32> {
    u32::try_from(n).map_err(|_| gpu_error(format!("{what}: {n} is too many for the GPU")))
}

/// The workgroups of a one-dimensional kernel of 64-wide workgroups for `n` invocations, laid
/// out in x and y.
fn spread(n: usize) -> (u32, u32) {
    let groups = n.div_ceil(64).max(1);
    let x = groups.min(65_535);
    (x as u32, groups.div_ceil(x) as u32)
}

impl Resident {
    /// `s`'s grid on `gpu`: its buffers allocated, its constants written.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Gpu`] if a field's three components are more than the device can bind,
    /// or the device's memory runs out.
    pub(in crate::fdtd) fn new(gpu: &Gpu, s: &Simulation) -> Result<Resident> {
        let c = gpu.context();
        let real = c.precision.bytes();
        let cells = s.grid.cells();
        let largest = gpu.largest_binding();
        if (3 * cells * real) as u64 > largest {
            return Err(gpu_error(format!(
                "a field's three components take {} MB, more than the {} MB {} binds at once",
                3 * cells * real / 1_000_000,
                largest / 1_000_000,
                gpu.name()
            )));
        }
        let g = s.grid;
        let mut reals = vec![s.dt, g.dx, g.dy, g.dz];
        for kappa in [&s.kappa_nodes, &s.kappa_halves] {
            for a in Axis::ALL {
                let h = g.step(a);
                reals.extend(kappa[a.index()].iter().map(|k| k / h));
            }
        }
        let mut slabs: Slabs = [[0; 4]; 36];
        let mut psi_offsets = Vec::new();
        let mut coefs = Vec::new();
        let mut psi_len = 0;
        for slab in &s.slabs {
            let field = usize::from(slab.field == Field::H);
            let side = usize::from(slab.from != 0);
            let entry = ((field * 3 + slab.component.index()) * 3 + slab.axis.index()) * 2 + side;
            slabs[entry] = [
                u32_of(slab.from, "a slab's start")?,
                u32_of(slab.count, "a slab's cells")?,
                u32_of(psi_len, "the CPMLs' values")?,
                u32_of(coefs.len(), "the CPMLs' coefficients")?,
            ];
            psi_offsets.push(psi_len);
            psi_len += slab.psi.len();
            coefs.extend_from_slice(&slab.b);
            coefs.extend_from_slice(&slab.a);
        }
        let mut low = [0; 3];
        let mut high = [0; 3];
        for a in Axis::ALL {
            let (l, h) = s.boundaries.edges(a).pml();
            low[a.index()] = u32_of(l, "CPML cells")?;
            high[a.index()] = u32_of(h, "CPML cells")?;
        }
        u32_of(3 * cells, "a field's values")?;
        let validation = c.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let memory = c.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let storage = |label: &str, values: usize| {
            c.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (values * real).max(16) as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        let params = c.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params"),
            size: (4 * (24 + 4 * 36)) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (e, h, psi) = (
            storage("e", 3 * cells),
            storage("h", 3 * cells),
            storage("psi", psi_len),
        );
        let (ca, cb) = (storage("ca", 3 * cells), storage("cb", 3 * cells));
        let reals_buffer = storage("reals", reals.len());
        let coefs_buffer = storage("coefs", coefs.len());
        let oom = wait(memory.pop());
        let invalid = wait(validation.pop());
        if let Some(e) = oom.or(invalid) {
            return Err(gpu_error(format!(
                "{} MB of fields don't fit on {}: {e}",
                12 * cells * real / 1_000_000,
                gpu.name()
            )));
        }
        // room left for the copies to and from the device and a run's monitors
        let room = || {
            let memory = c.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
            drop(storage("room", HEADROOM / real));
            wait(memory.pop()).is_none()
        };
        // the fused step's second copy, where it fits with that room
        let memory = c.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let second = (
            storage("e next", 3 * cells),
            storage("h next", 3 * cells),
            storage("psi next", psi_len),
        );
        let fused = wait(memory.pop()).is_none() && room();
        let second = if fused {
            second
        } else {
            drop(second);
            (
                storage("e next", 0),
                storage("h next", 0),
                storage("psi next", 0),
            )
        };
        if !fused && !room() {
            return Err(gpu_error(format!(
                "{} MB of fields fit on {}, but not the room to copy them there and back",
                12 * cells * real / 1_000_000,
                gpu.name()
            )));
        }
        let resident = Resident {
            gpu: gpu.clone(),
            cells,
            params,
            e: [e, second.0],
            h: [h, second.1],
            psi: [psi, second.2],
            ca,
            cb,
            reals: reals_buffer,
            coefs: coefs_buffer,
            fused,
            parity: 0,
            psi_offsets,
            slabs,
            low,
            high,
            planes: PLANES[usize::from(c.precision == Precision::Double)],
            lossless: false,
        };
        resident.write(&resident.reals, 0, &reals)?;
        resident.write(&resident.coefs, 0, &coefs)?;
        Ok(resident)
    }

    /// Waits until the device has done everything submitted.
    fn finish(&self) -> Result<()> {
        self.gpu
            .context()
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map(|_| ())
            .map_err(|e| gpu_error(format!("{}: {e}", self.gpu.name())))
    }

    /// `values` into `buffer` from value `at` on, a chunk at a time.
    fn write(&self, buffer: &wgpu::Buffer, at: usize, values: &[f64]) -> Result<()> {
        let c = self.gpu.context();
        let real = c.precision.bytes();
        for (n, chunk) in values.chunks(CHUNK).enumerate() {
            let data = bytes(chunk, c.precision);
            let memory = c.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
            c.queue
                .write_buffer(buffer, ((at + n * CHUNK) * real) as u64, &data);
            if let Some(e) = wait(memory.pop()) {
                return Err(gpu_error(format!("copying to {}: {e}", self.gpu.name())));
            }
            // flushed a chunk at a time, so their copies don't pile up in memory
            c.queue.submit(std::iter::empty());
            self.finish()?;
        }
        Ok(())
    }

    /// The values of `buffer` from value `at` on into `out`, a chunk at a time.
    fn read(&self, buffer: &wgpu::Buffer, at: usize, out: &mut [f64]) -> Result<()> {
        if out.is_empty() {
            return Ok(());
        }
        let c = self.gpu.context();
        let real = c.precision.bytes();
        let memory = c.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let staging = c.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read back"),
            size: (out.len().min(CHUNK) * real) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(e) = wait(memory.pop()) {
            return Err(gpu_error(format!("copying from {}: {e}", self.gpu.name())));
        }
        for (n, chunk) in out.chunks_mut(CHUNK).enumerate() {
            let len = (chunk.len() * real) as u64;
            let mut encoder = c
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            encoder.copy_buffer_to_buffer(
                buffer,
                ((at + n * CHUNK) * real) as u64,
                &staging,
                0,
                len,
            );
            c.queue.submit([encoder.finish()]);
            let (sender, receiver) = mpsc::channel();
            staging
                .slice(..len)
                .map_async(wgpu::MapMode::Read, move |r| {
                    let _ = sender.send(r);
                });
            self.finish()?;
            receiver
                .recv()
                .map_err(|e| gpu_error(e.to_string()))?
                .map_err(|e| gpu_error(format!("reading back: {e}")))?;
            {
                let view = staging
                    .slice(..len)
                    .get_mapped_range()
                    .map_err(|e| gpu_error(format!("reading back: {e}")))?;
                values(&view, c.precision, chunk);
            }
            staging.unmap();
        }
        Ok(())
    }

    /// `s`'s fields, E's coefficients and the CPMLs' ψ, onto the GPU.
    pub(in crate::fdtd) fn upload(&mut self, s: &Simulation) -> Result<()> {
        let n = self.cells;
        self.lossless = (0..3).all(|c| {
            s.ca[c]
                .par_iter()
                .zip(&s.cb[c])
                .all(|(&a, &b)| if b == 0.0 { a == 0.0 } else { a == 1.0 })
        });
        self.parity = 0;
        for c in 0..3 {
            self.write(&self.e[0], c * n, &s.e[c])?;
            self.write(&self.h[0], c * n, &s.h[c])?;
            if !self.lossless {
                self.write(&self.ca, c * n, &s.ca[c])?;
            }
            self.write(&self.cb, c * n, &s.cb[c])?;
        }
        for (slab, &at) in s.slabs.iter().zip(&self.psi_offsets) {
            self.write(&self.psi[0], at, &slab.psi)?;
        }
        Ok(())
    }

    /// The fields and the CPMLs' ψ back into `s`.
    pub(in crate::fdtd) fn download(&self, s: &mut Simulation) -> Result<()> {
        let n = self.cells;
        let p = self.parity;
        for c in 0..3 {
            self.read(&self.e[p], c * n, &mut s.e[c])?;
            self.read(&self.h[p], c * n, &mut s.h[c])?;
        }
        for (slab, &at) in s.slabs.iter_mut().zip(&self.psi_offsets) {
            self.read(&self.psi[p], at, &mut slab.psi)?;
        }
        Ok(())
    }

    /// `count` steps of `s` on the GPU, as [`Simulation::run`] takes them on the CPU: its
    /// fields there and back, its sources and currents added, its probes recorded and its
    /// transforms accumulated.
    pub(in crate::fdtd) fn run(&mut self, s: &mut Simulation, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        self.upload(s)?;
        self.step(s, count, true)?;
        self.download(s)
    }

    /// `count` steps of the fields on the GPU, with `s`'s sources, probes and transforms if
    /// `everything` (recorded into `s`, whose step count goes up), or the kernel alone; the
    /// fields stay there.
    pub(in crate::fdtd) fn step(
        &mut self,
        s: &mut Simulation,
        count: usize,
        everything: bool,
    ) -> Result<()> {
        let plan = self.plan(s, count, everything)?;
        let gpu = self.gpu.clone();
        let c = gpu.context();
        let k = &c.kernels;
        let g = s.grid;
        let tiles = (
            u32_of(g.nx.div_ceil(32), "workgroups along x")?,
            u32_of(g.ny.div_ceil(8), "workgroups along y")?,
            u32_of(g.nz.div_ceil(self.planes), "workgroups along z")?,
        );
        let mut before: Option<wgpu::SubmissionIndex> = None;
        let mut done = 0;
        while done < count {
            let batch = plan.batch.min(count - done);
            if everything {
                self.prepare(s, &plan, s.steps + done, done, batch);
            } else {
                let steps: Vec<u32> = (0..batch)
                    .flat_map(|m| {
                        let mut words = [0u32; STEP_STRIDE / 4];
                        words[0] = m as u32;
                        words
                    })
                    .collect();
                c.queue.write_buffer(&plan.steps, 0, &words(&steps));
            }
            let mut encoder = c
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            if done == 0 && plan.sums > 0 {
                encoder.clear_buffer(&plan.sum_buffer, 0, None);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                for m in 0..batch {
                    pass.set_bind_group(1, &plan.steps_group, &[(m * STEP_STRIDE) as u32]);
                    pass.set_bind_group(0, &plan.fields[self.parity], &[]);
                    if self.fused {
                        // M added to H̃ before its update rather than after: the same sum in
                        // another order
                        if plan.groups[0] > 0 {
                            let (x, y) = spread(plan.groups[0]);
                            pass.set_pipeline(&k.inject_h);
                            pass.dispatch_workgroups(x, y, 1);
                        }
                        pass.set_pipeline(if self.lossless {
                            &k.update_fused_lossless
                        } else {
                            &k.update_fused
                        });
                        pass.dispatch_workgroups(tiles.0, tiles.1, tiles.2);
                        self.parity = 1 - self.parity;
                        pass.set_bind_group(0, &plan.fields[self.parity], &[]);
                    } else {
                        pass.set_pipeline(&k.update_h);
                        pass.dispatch_workgroups(tiles.0, tiles.1, tiles.2);
                        if plan.groups[0] > 0 {
                            let (x, y) = spread(plan.groups[0]);
                            pass.set_pipeline(&k.inject_h);
                            pass.dispatch_workgroups(x, y, 1);
                        }
                        pass.set_pipeline(if self.lossless {
                            &k.update_e_lossless
                        } else {
                            &k.update_e
                        });
                        pass.dispatch_workgroups(tiles.0, tiles.1, tiles.2);
                    }
                    if plan.groups[1] > 0 {
                        let (x, y) = spread(plan.groups[1]);
                        pass.set_pipeline(&k.inject_e);
                        pass.dispatch_workgroups(x, y, 1);
                    }
                    if plan.probes > 0 {
                        let (x, y) = spread(plan.probes);
                        pass.set_pipeline(&k.probe_values);
                        pass.dispatch_workgroups(x, y, 1);
                    }
                    if plan.values > 0 {
                        let (x, y) = spread(plan.values);
                        pass.set_pipeline(&k.transform);
                        pass.dispatch_workgroups(x, y, 1);
                    }
                }
            }
            let index = c.queue.submit([encoder.finish()]);
            // one submission in flight while the next is prepared
            if let Some(b) = before.take() {
                c.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(b),
                        timeout: None,
                    })
                    .map_err(|e| gpu_error(format!("{}: {e}", self.gpu.name())))?;
            }
            before = Some(index);
            done += batch;
        }
        self.finish()?;
        if everything {
            self.record(s, &plan, count)?;
        }
        Ok(())
    }

    /// The buffers and tables of a run of `count` steps of `s`: with its sources, probes and
    /// transforms if `everything`, else none.
    fn plan(&self, s: &Simulation, count: usize, everything: bool) -> Result<Plan> {
        let c = self.gpu.context();
        let cells = self.cells;
        // the values the sources and currents add to on H̃, then on E, each with its sources and
        // currents in the order a step on the CPU adds them
        let mut groups: Vec<u32> = Vec::new();
        let mut emitters: Vec<u32> = Vec::new();
        let mut amplitudes: Vec<f64> = Vec::new();
        let mut counts = [0; 2];
        let sources = s.sources.len();
        if everything {
            for (f, field) in [Field::H, Field::E].into_iter().enumerate() {
                let mut order: Vec<usize> = Vec::new();
                let mut lists: HashMap<usize, Vec<(usize, c64)>> = HashMap::new();
                let mut add = |place: usize, emitter: usize, a: c64| {
                    lists
                        .entry(place)
                        .or_insert_with(|| {
                            order.push(place);
                            Vec::new()
                        })
                        .push((emitter, a));
                };
                for (n, x) in s.sources.iter().enumerate() {
                    if x.field == field {
                        let place = x.component.index() * cells + s.index(x.at);
                        add(place, n, c64::new(1.0, 0.0));
                    }
                }
                for (n, x) in s.currents.iter().enumerate() {
                    if x.field == field {
                        for &(component, r, a) in &x.values {
                            add(component * cells + r, sources + n, a);
                        }
                    }
                }
                for place in &order {
                    let list = &lists[place];
                    groups.extend([
                        u32_of(*place, "a source's value")?,
                        u32_of(emitters.len(), "the sources' values")?,
                        u32_of(list.len(), "the sources at one value")?,
                    ]);
                    for &(emitter, a) in list {
                        emitters.push(u32_of(emitter, "the sources")?);
                        amplitudes.extend([a.re, a.im]);
                    }
                }
                counts[f] = order.len();
            }
        }
        let emitter_count = if everything {
            sources + s.currents.len()
        } else {
            0
        };
        let probes: Vec<u32> = if everything {
            s.probes
                .iter()
                .map(|p| {
                    Ok([
                        u32::from(p.field == Field::H),
                        u32_of(p.component.index() * cells + p.index, "a probe's value")?,
                    ])
                })
                .collect::<Result<Vec<_>>>()?
                .concat()
        } else {
            Vec::new()
        };
        // the transforms: each monitor's phases at E's times, then at H̃'s, a step's after another
        let mut series: Vec<u32> = Vec::new();
        let mut dfts = Vec::new();
        let (mut values, mut phases, mut sums) = (0usize, 0usize, 0usize);
        if everything {
            for (frequencies, list) in s.monitors.layout() {
                let count = frequencies.len();
                for (field, component, ranges) in list {
                    let volume: usize = ranges.iter().map(|r| r.len()).product();
                    let phase = if field == Field::E {
                        phases
                    } else {
                        phases + count
                    };
                    let mut row = vec![u32::from(field == Field::H), component.index() as u32];
                    for r in &ranges {
                        row.push(u32_of(r.start, "a monitor's box")?);
                        row.push(u32_of(r.len(), "a monitor's box")?);
                    }
                    row.extend([
                        u32_of(sums, "the transforms")?,
                        u32_of(count, "a monitor's frequencies")?,
                        u32_of(phase, "the monitors' frequencies")?,
                        u32_of(values, "the transformed values")?,
                    ]);
                    series.extend(row);
                    values += volume;
                    sums += volume * count;
                }
                phases += 2 * count;
                dfts.push(frequencies.iter().map(|f| f.angular()).collect());
            }
        }
        let series_count = series.len() / 12;
        let real = c.precision.bytes();
        let batch = (SUBMISSION / cells.max(1)).clamp(1, MOST_STEPS).min(count);
        let params = {
            let mut p: Vec<u32> = Vec::with_capacity(24 + 4 * 36);
            let n = [s.grid.nx, s.grid.ny, s.grid.nz, cells];
            for v in n {
                p.push(u32_of(v, "the grid's cells")?);
            }
            for a in Axis::ALL {
                p.push(u32::from(s.boundaries.periodic(a)));
            }
            p.push(0);
            p.extend(self.low);
            p.push(0);
            p.extend(self.high);
            p.push(0);
            p.extend([
                u32_of(self.planes, "planes")?,
                u32_of(counts[0], "sources")?,
                u32_of(counts[1], "sources")?,
                u32_of(probes.len() / 2, "probes")?,
            ]);
            p.extend([
                u32_of(emitter_count, "sources")?,
                u32_of(series_count, "transforms")?,
                u32_of(values, "transformed values")?,
                u32_of(phases, "frequencies")?,
            ]);
            for entry in &self.slabs {
                p.extend(entry);
            }
            p
        };
        c.queue.write_buffer(&self.params, 0, &words(&params));
        let validation = c.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let memory = c.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let buffer = |label: &str, size: usize, usage: wgpu::BufferUsages| {
            c.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(16).next_multiple_of(4) as u64,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let storage = wgpu::BufferUsages::STORAGE;
        let read = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        let groups_buffer = buffer("groups", 4 * groups.len(), storage);
        let emitters_buffer = buffer("emitters", 4 * emitters.len(), storage);
        let amplitudes_buffer = buffer("amplitudes", real * amplitudes.len(), storage);
        let waves = buffer("waves", real * 2 * emitter_count * batch, storage);
        let probes_buffer = buffer("probes", 4 * probes.len(), storage);
        let recorded = buffer("recorded", real * (probes.len() / 2) * count, read);
        let series_buffer = buffer("series", 4 * series.len(), storage);
        let phase_buffer = buffer("phases", real * 2 * phases * batch, storage);
        let sum_buffer = buffer("sums", real * 2 * sums, read);
        let steps = buffer("steps", STEP_STRIDE * batch, wgpu::BufferUsages::UNIFORM);
        let fields = |p: usize| {
            c.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fdtd fields"),
                layout: &c.kernels.fields,
                entries: &[
                    &self.params,
                    &self.e[p],
                    &self.h[p],
                    &self.ca,
                    &self.cb,
                    &self.reals,
                    &self.psi[p],
                    &self.coefs,
                    &groups_buffer,
                    &emitters_buffer,
                    &amplitudes_buffer,
                    &waves,
                    &probes_buffer,
                    &recorded,
                    &series_buffer,
                    &phase_buffer,
                    &sum_buffer,
                    &self.e[1 - p],
                    &self.h[1 - p],
                    &self.psi[1 - p],
                ]
                .into_iter()
                .zip(0u32..)
                .map(|(b, binding)| wgpu::BindGroupEntry {
                    binding,
                    resource: b.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
            })
        };
        let fields = [fields(0), fields(1)];
        let step_group = c.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fdtd steps"),
            layout: &c.kernels.steps,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &steps,
                    offset: 0,
                    size: wgpu::BufferSize::new(16),
                }),
            }],
        });
        let oom = wait(memory.pop());
        let invalid = wait(validation.pop());
        if let Some(e) = oom.or(invalid) {
            return Err(gpu_error(format!(
                "the run's sources, probes and monitors don't fit on {}: {e}",
                self.gpu.name()
            )));
        }
        let precision = c.precision;
        let write_reals = |b: &wgpu::Buffer, v: &[f64]| {
            if !v.is_empty() {
                c.queue.write_buffer(b, 0, &bytes(v, precision));
            }
        };
        let write_words = |b: &wgpu::Buffer, v: &[u32]| {
            if !v.is_empty() {
                c.queue.write_buffer(b, 0, &words(v));
            }
        };
        write_words(&groups_buffer, &groups);
        write_words(&emitters_buffer, &emitters);
        write_reals(&amplitudes_buffer, &amplitudes);
        write_words(&probes_buffer, &probes);
        write_words(&series_buffer, &series);
        Ok(Plan {
            groups: counts,
            emitters: emitter_count,
            probes: probes.len() / 2,
            series: series_count,
            values,
            phases,
            sums,
            dfts,
            batch,
            waves,
            phase_buffer,
            steps,
            recorded,
            sum_buffer,
            fields,
            steps_group: step_group,
        })
    }

    /// The waveforms, the transforms' phases and the steps' numbers of `batch` steps from step
    /// `first` of the simulation (step `run_step` of the run), into the plan's buffers.
    fn prepare(&self, s: &Simulation, plan: &Plan, first: usize, run_step: usize, batch: usize) {
        let c = self.gpu.context();
        let dt = s.dt;
        let mut waves = Vec::with_capacity(2 * plan.emitters * batch);
        let mut phases = Vec::with_capacity(2 * plan.phases * batch);
        let mut steps = Vec::with_capacity(STEP_STRIDE / 4 * batch);
        for m in 0..batch {
            // as a step on the CPU: M at t, J at t + Δt/2, E transformed at t + Δt and H̃ at
            // t + Δt/2
            let t = (first + m) as f64 * dt;
            let half = t + 0.5 * dt;
            let when = |f: Field| if f == Field::H { t } else { half };
            if plan.emitters > 0 {
                for x in &s.sources {
                    waves.extend([x.waveform.at(when(x.field)), 0.0]);
                }
                for x in &s.currents {
                    let w = x.waveform.complex_at(when(x.field));
                    waves.extend([w.re, w.im]);
                }
            }
            let te = (first + m + 1) as f64 * dt;
            let th = te - 0.5 * dt;
            for omegas in &plan.dfts {
                for time in [te, th] {
                    for &w in omegas {
                        let p = c64::new(0.0, w * time).exp() * dt;
                        phases.extend([p.re, p.im]);
                    }
                }
            }
            let mut words = [0u32; STEP_STRIDE / 4];
            words[0] = m as u32;
            words[1] = (run_step + m) as u32;
            steps.extend(words);
        }
        let precision = c.precision;
        if !waves.is_empty() {
            c.queue
                .write_buffer(&plan.waves, 0, &bytes(&waves, precision));
        }
        if !phases.is_empty() {
            c.queue
                .write_buffer(&plan.phase_buffer, 0, &bytes(&phases, precision));
        }
        c.queue.write_buffer(&plan.steps, 0, &words(&steps));
    }

    /// The run's probes and transforms into `s`, and its steps counted.
    fn record(&self, s: &mut Simulation, plan: &Plan, count: usize) -> Result<()> {
        let mut recorded = vec![0.0; plan.probes * count];
        self.read(&plan.recorded, 0, &mut recorded)?;
        let mut sums = vec![0.0; 2 * plan.sums];
        self.read(&plan.sum_buffer, 0, &mut sums)?;
        if plan.probes > 0 {
            for step in recorded.chunks_exact(plan.probes) {
                for (p, &v) in s.probes.iter_mut().zip(step) {
                    p.values.push(v);
                }
            }
        }
        if plan.series > 0 {
            let sums: Vec<c64> = sums
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[re, im]| c64::new(re, im))
                .collect();
            s.monitors.add(&sums);
        }
        s.steps += count;
        Ok(())
    }
}
