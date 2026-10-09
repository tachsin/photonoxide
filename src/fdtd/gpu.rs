//! FDTD on the GPU, through wgpu's compute shaders, behind the `gpu` feature.
//!
//! A [`Simulation`](super::Simulation) set to a [`Gpu`] by
//! [`Simulation::set_device`](super::Simulation::set_device) steps there: the same problem, the
//! same API, its fields, probes and monitors read back on the CPU after each run. The kernels are
//! written by hand in WGSL (`gpu/yee.wgsl`):
//!
//! - **The curl updates** march along z a plane at a time with the plane in workgroup memory
//!   (P. Micikevicius, Proc. GPGPU-2, 79 (2009), doi:10.1145/1513895.1513905): a workgroup of
//!   32 × 8 invocations owns a tile of 32 × 8 values in each plane of a run of planes, holds the
//!   other field's plane with the row and column of neighbours the curl needs, and keeps its own
//!   column's value of the plane before (E) or after (H̃) in registers. Each field's three
//!   components are updated together, and the CPML's ψ in the same pass where a value is in a
//!   slab, in the order the CPU's slabs take them.
//! - **Sources and currents** are added after each field's update, one invocation per value, its
//!   sources and currents in the order the CPU adds them. Their waveforms are taken on the CPU in
//!   f64 at each step's times.
//! - **Probes** are copied out after each step, and **transforms** accumulated per value, one
//!   invocation a value taking every frequency in turn, step after step: in time order, with no
//!   sum across invocations. Each run's sums start from zero on the GPU and are added to the
//!   simulation's in f64 at the end.
//!
//! **Determinism.** No atomics, no reductions across invocations, fixed workgroup sizes: every
//! value is computed by one invocation in a fixed order, so a run repeats bit for bit on the same
//! device and driver. WGSL lets a compiler fuse and reorder arithmetic (§15.7), so the GPU's
//! fields aren't the CPU's bits: they agree to a tolerance stated per quantity (see the method's
//! write-up, docs/methods/fdtd.md).
//!
//! **Precision.** [`Precision::Single`], f32, on any wgpu backend (Vulkan, DirectX 12, Metal);
//! [`Precision::Double`], f64, on Vulkan where the adapter has `SHADER_F64`, for checking
//! against the CPU. Software adapters (a CPU emulating a GPU) aren't used.
//!
//! **What it steps:** E and H̃, CPMLs, walls and periodic sides, conductors and conductivities,
//! point sources, currents (mode sources, dipoles, the adjoint's), probes, and every transform
//! monitor (transforms, fluxes, modes, a design's). Not yet: Bloch phases, dispersive media,
//! smoothed tensors that couple E's components, and plane waves on total-field/scattered-field
//! boxes; [`Simulation::set_device`](super::Simulation::set_device) refuses a problem with them.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::{Error, Result};

mod resident;
pub(super) use resident::Resident;

/// The precision a GPU steps in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Precision {
    /// f32, on any wgpu backend.
    Single,
    /// f64, on Vulkan where the adapter supports it: for checking against the CPU, at a fraction
    /// of f32's rate on consumer GPUs.
    Double,
}

impl Precision {
    /// The bytes of one value.
    pub fn bytes(self) -> usize {
        match self {
            Precision::Single => 4,
            Precision::Double => 8,
        }
    }

    fn wgsl(self) -> &'static str {
        match self {
            Precision::Single => "f32",
            Precision::Double => "f64",
        }
    }
}

/// A GPU to step simulations on: a wgpu device and its compiled kernels, in one [`Precision`].
/// Cloning it shares the device.
#[derive(Clone)]
pub struct Gpu {
    context: Arc<Context>,
}

pub(super) struct Context {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    pub(super) precision: Precision,
    pub(super) limits: wgpu::Limits,
    pub(super) kernels: Kernels,
}

/// The compiled kernels and their bindings' layouts.
pub(super) struct Kernels {
    pub(super) fields: wgpu::BindGroupLayout,
    pub(super) steps: wgpu::BindGroupLayout,
    pub(super) update_h: wgpu::ComputePipeline,
    pub(super) update_e: wgpu::ComputePipeline,
    /// E's update in a medium with no conductivity, which doesn't read ca.
    pub(super) update_e_lossless: wgpu::ComputePipeline,
    /// A whole step in one pass, from one copy of the fields to the other.
    pub(super) update_fused: wgpu::ComputePipeline,
    pub(super) update_fused_lossless: wgpu::ComputePipeline,
    pub(super) inject_h: wgpu::ComputePipeline,
    pub(super) inject_e: wgpu::ComputePipeline,
    pub(super) probe_values: wgpu::ComputePipeline,
    pub(super) transform: wgpu::ComputePipeline,
}

/// The storage buffers of the kernels' first group, by binding (1 to 19), and whether each is
/// read only.
const STORAGE: [bool; 19] = [
    false, false, true, true, true, false, true, true, true, true, true, true, false, true, true,
    false, false, false, false,
];

pub(super) fn gpu_error(reason: impl Into<String>) -> Error {
    Error::Gpu {
        reason: reason.into(),
    }
}

/// Waits for a future of wgpu's: on native backends they are ready, or become ready as the
/// device is polled.
pub(super) fn wait<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

impl Gpu {
    /// The best GPU here for `precision`: a discrete one before an integrated one, Vulkan before
    /// Metal before DirectX 12; never a software adapter. f64 needs Vulkan and an adapter with
    /// `SHADER_F64`.
    ///
    /// # Errors
    ///
    /// [`Error::Gpu`] if there is no such GPU, if the device can't be opened, or if the kernels
    /// don't compile on it.
    pub fn new(precision: Precision) -> Result<Gpu> {
        let backends = match precision {
            Precision::Single => wgpu::Backends::PRIMARY,
            Precision::Double => wgpu::Backends::VULKAN,
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            flags: wgpu::InstanceFlags::empty().with_env(),
            // allocations past the device's memory fail, rather than spill to the system's
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds {
                for_resource_creation: Some(95),
                for_device_loss: None,
            },
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapters = wait(instance.enumerate_adapters(backends));
        let rank = |a: &wgpu::Adapter| {
            let info = a.get_info();
            let kind = match info.device_type {
                wgpu::DeviceType::DiscreteGpu => 0,
                wgpu::DeviceType::IntegratedGpu => 1,
                wgpu::DeviceType::VirtualGpu => 2,
                wgpu::DeviceType::Other => 3,
                wgpu::DeviceType::Cpu => 4,
            };
            let backend = match info.backend {
                wgpu::Backend::Vulkan => 0,
                wgpu::Backend::Metal => 1,
                wgpu::Backend::Dx12 => 2,
                _ => 3,
            };
            (kind, backend)
        };
        let found = adapters.len();
        let adapter = adapters
            .into_iter()
            .filter(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
            .filter(|a| {
                precision == Precision::Single || a.features().contains(wgpu::Features::SHADER_F64)
            })
            .min_by_key(rank)
            .ok_or_else(|| {
                gpu_error(match precision {
                    Precision::Single if found == 0 => "no GPU found".to_string(),
                    Precision::Single => {
                        format!("no GPU found: {found} adapter(s), all software")
                    }
                    Precision::Double => format!(
                        "no GPU with f64 (SHADER_F64 on Vulkan) found: {found} Vulkan adapter(s)"
                    ),
                })
            })?;
        let info = adapter.get_info();
        let limits = adapter.limits();
        if (limits.max_storage_buffers_per_shader_stage as usize) < STORAGE.len() {
            return Err(gpu_error(format!(
                "{} allows {} storage buffers a shader, the kernels need {}",
                info.name,
                limits.max_storage_buffers_per_shader_stage,
                STORAGE.len()
            )));
        }
        let required_features = match precision {
            Precision::Single => wgpu::Features::empty(),
            Precision::Double => wgpu::Features::SHADER_F64,
        };
        let (device, queue) = wait(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("photonoxide fdtd"),
            required_features,
            required_limits: limits.clone(),
            ..Default::default()
        }))
        .map_err(|e| gpu_error(format!("{}: {e}", info.name)))?;
        let kernels = Kernels::new(&device, precision)
            .map_err(|e| gpu_error(format!("{}: the kernels don't compile: {e}", info.name)))?;
        Ok(Gpu {
            context: Arc::new(Context {
                device,
                queue,
                info,
                precision,
                limits,
                kernels,
            }),
        })
    }

    /// Its precision.
    pub fn precision(&self) -> Precision {
        self.context.precision
    }

    /// The adapter, its backend and its driver, e.g. "NVIDIA GeForce RTX 4060 (Vulkan, NVIDIA
    /// 616.56)": a run repeats bit for bit on the same one.
    pub fn name(&self) -> String {
        let i = &self.context.info;
        let driver = [i.driver.as_str(), i.driver_info.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} ({:?}, {driver})", i.name, i.backend)
    }

    /// The largest buffer the device allows to be bound, in bytes: a field's three components
    /// are one buffer, so a grid has at most this over 3 × [`Precision::bytes`] cells.
    pub fn largest_binding(&self) -> u64 {
        self.context
            .limits
            .max_storage_buffer_binding_size
            .min(self.context.limits.max_buffer_size)
    }

    pub(super) fn context(&self) -> &Context {
        &self.context
    }
}

impl std::fmt::Debug for Gpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gpu")
            .field("adapter", &self.name())
            .field("precision", &self.context.precision)
            .finish()
    }
}

impl PartialEq for Gpu {
    /// The same device.
    fn eq(&self, other: &Gpu) -> bool {
        Arc::ptr_eq(&self.context, &other.context)
    }
}

impl Kernels {
    fn new(device: &wgpu::Device, precision: Precision) -> std::result::Result<Kernels, String> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = format!(
            "alias real = {};\n{}",
            precision.wgsl(),
            include_str!("gpu/yee.wgsl")
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fdtd"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let uniform = |binding: u32, dynamic: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let mut entries = vec![uniform(0, false)];
        entries.extend(STORAGE.iter().zip(1u32..).map(|(&read_only, binding)| {
            wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        }));
        let fields = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fdtd fields"),
            entries: &entries,
        });
        let steps = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fdtd steps"),
            entries: &[uniform(0, true)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fdtd"),
            bind_group_layouts: &[Some(&fields), Some(&steps)],
            immediate_size: 0,
        });
        let specialized = |entry: &str, constants: &[(&str, f64)]| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                cache: None,
            })
        };
        let pipeline = |entry: &str| specialized(entry, &[]);
        let kernels = Kernels {
            update_h: pipeline("update_h"),
            update_e: pipeline("update_e"),
            update_e_lossless: specialized("update_e", &[("lossless", 1.0)]),
            update_fused: pipeline("update_fused"),
            update_fused_lossless: specialized("update_fused", &[("lossless", 1.0)]),
            inject_h: pipeline("inject_h"),
            inject_e: pipeline("inject_e"),
            probe_values: pipeline("probe_values"),
            transform: pipeline("transform"),
            fields,
            steps,
        };
        match wait(scope.pop()) {
            Some(e) => Err(e.to_string()),
            None => Ok(kernels),
        }
    }
}

/// The GPU every new simulation starts on while [`everything_on`] runs.
static DEFAULT: Mutex<Option<Gpu>> = Mutex::new(None);

/// The steps taken on the GPU and on the CPU, counted for [`everything_on`].
static STEPS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

/// Runs `f` with every simulation made meanwhile, on any thread, starting on `gpu`: those the
/// GPU can't step fall back to the CPU when they first step. Returns `f`'s result and the steps
/// taken on the GPU and on the CPU meanwhile. For running the validation cases on the GPU, one
/// at a time: the default is the process's.
pub(crate) fn everything_on<T>(gpu: &Gpu, f: impl FnOnce() -> T) -> (T, [usize; 2]) {
    *DEFAULT.lock().unwrap_or_else(|e| e.into_inner()) = Some(gpu.clone());
    for s in &STEPS {
        s.store(0, Ordering::Relaxed);
    }
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    *DEFAULT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let steps = [0, 1].map(|n| STEPS[n].load(Ordering::Relaxed));
    match out {
        Ok(v) => (v, steps),
        Err(p) => std::panic::resume_unwind(p),
    }
}

/// A new simulation's GPU: [`everything_on`]'s, if it is running.
pub(super) fn by_default() -> Option<super::OnGpu> {
    DEFAULT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .map(|gpu| super::OnGpu {
            gpu,
            resident: None,
            fallback: true,
        })
}

/// Counts `steps` taken on the GPU or the CPU.
pub(super) fn count(on_gpu: bool, steps: usize) {
    STEPS[usize::from(!on_gpu)].fetch_add(steps, Ordering::Relaxed);
}

impl super::Simulation {
    /// Whether it steps on a GPU: set to one, and (if set by [`everything_on`]) with nothing the
    /// GPU doesn't step, else back on the CPU for good.
    pub(super) fn stays_on_gpu(&mut self) -> bool {
        match &self.gpu {
            None => false,
            Some(on) if on.fallback && self.off_gpu().is_some() => {
                self.gpu = None;
                false
            }
            Some(_) => true,
        }
    }

    /// What in this problem the GPU doesn't step yet, if anything.
    pub(super) fn off_gpu(&self) -> Option<&'static str> {
        if self.reference {
            Some("the plain loops (the CPU's reference)")
        } else if self.bloch.is_some() {
            Some("a Bloch phase")
        } else if !self.media.is_empty() {
            Some("a dispersive medium")
        } else if self.anisotropic.is_some() {
            Some("a smoothed tensor that couples E's components")
        } else if !self.plane_waves.is_empty() {
            Some("a plane wave")
        } else {
            None
        }
    }

    /// `count` steps on the simulation's GPU.
    pub(super) fn run_on_gpu(&mut self, count: usize) {
        if let Some(what) = self.off_gpu() {
            panic!("the GPU doesn't step {what} yet: set the device to the CPU");
        }
        let mut on = self.gpu.take().expect("a GPU to run on");
        let mut resident = match on.resident.take() {
            Some(r) => r,
            None => Resident::new(&on.gpu, self).unwrap_or_else(|e| panic!("{e}")),
        };
        if let Err(e) = resident.run(self, count) {
            panic!("{e}");
        }
        self::count(true, count);
        on.resident = Some(resident);
        self.gpu = Some(on);
    }
}

/// A GPU for the tests, or why there is none: they skip, saying so, where there is no GPU (on
/// CI, whose runners have none: the GPU's tests run on tachsin's machine before each release,
/// and their results go in docs/validation-gpu.md).
#[cfg(test)]
pub(crate) fn for_tests(precision: Precision) -> Option<Gpu> {
    if std::env::var_os("CI").is_some() {
        println!("skipped: on CI, which has no GPU (the GPU's tests run on tachsin's machine)");
        return None;
    }
    match Gpu::new(precision) {
        Ok(g) => Some(g),
        Err(e) => {
            println!("skipped: {e}");
            None
        }
    }
}

pub(crate) mod checks;
pub(crate) mod rates;
#[cfg(test)]
mod tests;
