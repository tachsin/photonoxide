//! The multigrid's finest level without its matrix on the GPU (#337): Shin and Fan's operator
//! from its stencil, a compute shader of photonoxide's own (`stencil.wgsl`) on Vulkan through
//! wgpu, on vectors cuSPARSE's run shares.
//!
//! cuSPARSE has no kernel for a stencil, and photonoxide's own GPU kernels are wgpu's
//! (docs/plans/performance.md, decision 3), so the product runs there, and the rest of the cycle
//! and GMRES stay on cuSPARSE ([`crate::gpu_multigrid`]). The two share two vectors, the
//! product's input and output: Vulkan allocates them exportable (`VK_KHR_external_memory_win32`,
//! or `_fd` on Linux), dedicated, on the device CUDA uses (the same PCI bus id), and CUDA imports
//! them (`cudaImportExternalMemory`), so nothing goes through the host. A product is a copy into
//! the input on cuSPARSE, CUDA's work finished, the three passes submitted to Vulkan and waited
//! for, and a copy out (docs/plans/backends.md, "The multigrid without its finest matrix",
//! has what was measured).
//!
//! - **f64,** Vulkan's `shaderFloat64` (wgpu's `SHADER_F64`): cuSPARSE's vectors are f64. f32
//!   compiles too, for the measurements.
//! - **Determinism:** each value is written by one invocation and each sum taken in the CPU's
//!   order, no atomics, fixed workgroups: a product repeats bit for bit on one GPU and driver.
//!   WGSL lets a compiler fuse a multiplication and an addition, so it is the CPU's to rounding.

use std::ffi::c_void;
use std::sync::Arc;

use ash::vk;
use num_complex::{Complex32 as c32, Complex64 as c64};
use photonoxide::Result;
use photonoxide::backend::Stencil;
use wgpu::hal::api::Vulkan;

use crate::cuda::{Buffer, Opaque, Runtime, check, serial};
use crate::library::error;

/// The precision a kernel computes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Precision {
    Single,
    Double,
}

impl Precision {
    fn wgsl(self) -> &'static str {
        match self {
            Precision::Single => "f32",
            Precision::Double => "f64",
        }
    }

    /// A complex value's bytes.
    pub(crate) fn complex(self) -> u64 {
        match self {
            Precision::Single => 8,
            Precision::Double => 16,
        }
    }

    fn bytes(self, values: &[c64]) -> Vec<u8> {
        let mut out = Vec::with_capacity(values.len() * self.complex() as usize);
        for z in values {
            match self {
                Precision::Single => {
                    let z = c32::new(z.re as f32, z.im as f32);
                    out.extend_from_slice(&z.re.to_le_bytes());
                    out.extend_from_slice(&z.im.to_le_bytes());
                }
                Precision::Double => {
                    out.extend_from_slice(&z.re.to_le_bytes());
                    out.extend_from_slice(&z.im.to_le_bytes());
                }
            }
        }
        out
    }
}

/// Waits for a future of wgpu's: on native backends they are ready, or become ready as the
/// device is polled.
fn wait<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The Vulkan device CUDA's GPU is, and the stencil's kernels compiled on it.
pub(crate) struct Interop {
    runtime: Arc<Runtime>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    /// The kernels in f64, and in f32 (for the measurements).
    kernels: [Kernels; 2],
    name: String,
}

struct Kernels {
    curl_e: wgpu::ComputePipeline,
    potential: wgpu::ComputePipeline,
    product: wgpu::ComputePipeline,
}

impl Interop {
    /// The Vulkan adapter on CUDA's GPU (the same PCI bus id), with f64 in its shaders, opened.
    ///
    /// # Errors
    ///
    /// Why not: a CUDA runtime that can't import memory, no such adapter, a device that won't
    /// open, kernels that don't compile.
    pub(crate) fn new(runtime: &Arc<Runtime>) -> std::result::Result<Interop, String> {
        let bus = runtime.bus_id().map_err(|e| e.to_string())?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::empty().with_env(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds {
                for_resource_creation: Some(95),
                for_device_loss: None,
            },
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapters = wait(instance.enumerate_adapters(wgpu::Backends::VULKAN));
        let found: Vec<String> = adapters
            .iter()
            .map(|a| {
                let i = a.get_info();
                format!("{} ({})", i.name, i.device_pci_bus_id)
            })
            .collect();
        let adapter = adapters
            .into_iter()
            .find(|a| a.get_info().device_pci_bus_id.eq_ignore_ascii_case(&bus))
            .ok_or_else(|| {
                format!(
                    "no Vulkan adapter is CUDA's GPU (PCI bus {bus}): {}",
                    if found.is_empty() {
                        "none found".to_string()
                    } else {
                        found.join(", ")
                    }
                )
            })?;
        let info = adapter.get_info();
        if !adapter.features().contains(wgpu::Features::SHADER_F64) {
            return Err(format!("{} has no f64 in its shaders on Vulkan", info.name));
        }
        // the extension for the exported memory is turned on with the device where the driver
        // has it (wgpu-hal does); the feature only says so
        // (on Linux, VK_KHR_external_memory_fd, which wgpu-hal turns on where the driver has it)
        #[cfg(windows)]
        let export = wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32;
        #[cfg(not(windows))]
        let export = wgpu::Features::empty();
        if !adapter.features().contains(export) {
            return Err(format!(
                "{}'s Vulkan driver can't export memory (VK_KHR_external_memory_win32)",
                info.name
            ));
        }
        let required_features = wgpu::Features::SHADER_F64 | export;
        let (device, queue) = wait(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("photonoxide multigrid stencil"),
            required_features,
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| format!("{}: {e}", info.name))?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let storage = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for (binding, read_only) in [
            (1, true),
            (2, false),
            (3, false),
            (4, false),
            (5, true),
            (6, true),
            (7, true),
        ] {
            entries.push(storage(binding, read_only));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stencil"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("stencil"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let kernels = [Precision::Double, Precision::Single].map(|precision| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("stencil"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "alias real = {};\n{}",
                        precision.wgsl(),
                        include_str!("stencil.wgsl")
                    )
                    .into(),
                ),
            });
            let pipeline = |entry: &str| {
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
            Kernels {
                curl_e: pipeline("curl_e"),
                potential: pipeline("potential"),
                product: pipeline("product"),
            }
        });
        if let Some(e) = wait(scope.pop()) {
            return Err(format!(
                "{}: the stencil's kernels don't compile: {e}",
                info.name
            ));
        }
        Ok(Interop {
            runtime: runtime.clone(),
            device,
            queue,
            layout,
            kernels,
            name: format!(
                "{} (Vulkan, {} {})",
                info.name, info.driver, info.driver_info
            ),
        })
    }

    /// The adapter and its driver.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Waits until the device has done everything submitted.
    pub(crate) fn wait(&self) -> Result<()> {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map(drop)
            .map_err(|e| error(format!("waiting for Vulkan: {e}")))
    }

    /// A buffer of `bytes` on the GPU, Vulkan's and wgpu's only.
    pub(crate) fn buffer(&self, bytes: u64, contents: Option<&[u8]>) -> wgpu::Buffer {
        let usage = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes.max(16),
            usage,
            mapped_at_creation: false,
        });
        if let Some(data) = contents {
            self.queue.write_buffer(&buffer, 0, data);
        }
        buffer
    }

    /// A buffer of `bytes` both Vulkan and CUDA see.
    ///
    /// # Errors
    ///
    /// Vulkan's and CUDA's, as they come.
    pub(crate) fn shared(&self, bytes: u64) -> Result<Shared> {
        let bytes = bytes.max(16);
        let (buffer, memory, size, handle) = self.exported(bytes)?;
        // wgpu takes the buffer and its memory over, and frees them with it.
        // SAFETY: the buffer was made on this device with STORAGE, TRANSFER_SRC and TRANSFER_DST
        // and bound to `memory` (size bytes) at 0; nothing else owns them
        let hal_buffer =
            unsafe { wgpu::hal::vulkan::Buffer::from_raw_managed(buffer, memory, 0, size) };
        // SAFETY: made from this device's handle, as `desc` says, never mapped, not empty
        let wgpu_buffer = unsafe {
            self.device.create_buffer_from_hal::<Vulkan>(
                hal_buffer,
                &wgpu::BufferDescriptor {
                    label: Some("shared with CUDA"),
                    size: bytes,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                },
            )
        };
        let (external, ptr) = self.import(handle, size, bytes)?;
        Ok(Shared {
            runtime: self.runtime.clone(),
            // SAFETY: a mapped external buffer of `bytes`, released by cudaFree, owned here
            cuda: Some(unsafe { Buffer::adopt(&self.runtime, ptr, bytes as usize) }),
            external,
            buffer: wgpu_buffer,
        })
    }

    /// A Vulkan buffer of `bytes` in dedicated, exportable device memory, and the handle CUDA
    /// imports it by: the buffer, its memory and its size, and the handle.
    fn exported(&self, bytes: u64) -> Result<(vk::Buffer, vk::DeviceMemory, u64, Handle)> {
        let vulkan = |what: &str, e: vk::Result| error(format!("Vulkan's {what}: {e}"));
        // SAFETY: the guard is dropped at the end of this function, before wgpu uses the device
        // again; every handle made here is either returned or destroyed
        let hal = unsafe { self.device.as_hal::<Vulkan>() }
            .ok_or_else(|| error("the device isn't Vulkan's"))?;
        let raw = hal.raw_device();
        let instance = hal.shared_instance().raw_instance();
        let physical = hal.raw_physical_device();
        let mut external_info =
            vk::ExternalMemoryBufferCreateInfo::default().handle_types(HANDLE_TYPE);
        let info = vk::BufferCreateInfo::default()
            .size(bytes)
            .usage(
                vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::TRANSFER_SRC
                    | vk::BufferUsageFlags::TRANSFER_DST,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .push_next(&mut external_info);
        // SAFETY: a valid device and create info, as Vulkan's spec asks
        let buffer =
            unsafe { raw.create_buffer(&info, None) }.map_err(|e| vulkan("vkCreateBuffer", e))?;
        // SAFETY: the buffer was just made on this device
        let requirements = unsafe { raw.get_buffer_memory_requirements(buffer) };
        // SAFETY: the physical device is this device's
        let properties = unsafe { instance.get_physical_device_memory_properties(physical) };
        let destroy = |memory: Option<vk::DeviceMemory>| {
            // SAFETY: made here, used by nothing yet
            unsafe {
                raw.destroy_buffer(buffer, None);
                if let Some(m) = memory {
                    raw.free_memory(m, None);
                }
            }
        };
        let Some(type_index) = (0..properties.memory_type_count).find(|&t| {
            requirements.memory_type_bits & (1 << t) != 0
                && properties.memory_types[t as usize]
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
        }) else {
            destroy(None);
            return Err(error("no device-local memory for an exported buffer"));
        };
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(buffer);
        let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(HANDLE_TYPE);
        let allocate = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(type_index)
            .push_next(&mut export)
            .push_next(&mut dedicated);
        // SAFETY: valid allocate info for this device, its chain alive meanwhile
        let memory = match unsafe { raw.allocate_memory(&allocate, None) } {
            Ok(m) => m,
            Err(e) => {
                destroy(None);
                return Err(vulkan("vkAllocateMemory", e));
            }
        };
        // SAFETY: the memory was allocated for this buffer, dedicated, of its size
        if let Err(e) = unsafe { raw.bind_buffer_memory(buffer, memory, 0) } {
            destroy(Some(memory));
            return Err(vulkan("vkBindBufferMemory", e));
        }
        match export_handle(instance, raw, memory) {
            Ok(handle) => Ok((buffer, memory, requirements.size, handle)),
            Err(e) => {
                destroy(Some(memory));
                Err(vulkan("the memory's handle", e))
            }
        }
    }

    /// CUDA's view of exported memory: the import, and its pointer to `bytes` from the start.
    fn import(&self, handle: Handle, size: u64, bytes: u64) -> Result<(Opaque, Opaque)> {
        let external = self
            .runtime
            .external
            .as_ref()
            .ok_or_else(|| error("this CUDA runtime can't import another API's memory"))?;
        let _serial = serial();
        let mut description = HandleDescription {
            kind: HANDLE_KIND,
            pad: 0,
            handle: [std::ptr::null_mut(); 2],
            size,
            flags: DEDICATED,
            reserved: [0; 16],
        };
        handle.describe(&mut description);
        let mut imported: Opaque = std::ptr::null_mut();
        check(
            // SAFETY: the description is cudaExternalMemoryHandleDesc's layout (driver_types.h),
            // its handle valid; the out-pointer writable
            unsafe {
                (external.import)(
                    &mut imported,
                    (&description as *const HandleDescription).cast(),
                )
            },
            "cudaImportExternalMemory",
        )?;
        // the handle isn't CUDA's on Windows: closed now (on Linux CUDA owns the descriptor)
        handle.release();
        let range = BufferDescription {
            offset: 0,
            size: bytes,
            flags: 0,
            reserved: [0; 16],
        };
        let mut ptr: Opaque = std::ptr::null_mut();
        let mapped = check(
            // SAFETY: cudaExternalMemoryBufferDesc's layout; the range is within the import
            unsafe {
                (external.mapped)(
                    &mut ptr,
                    imported,
                    (&range as *const BufferDescription).cast(),
                )
            },
            "cudaExternalMemoryGetMappedBuffer",
        );
        if let Err(e) = mapped {
            // SAFETY: imported above, destroyed once
            unsafe { (external.destroy)(imported) };
            return Err(e);
        }
        Ok((imported, ptr))
    }

    /// The kernels of a precision.
    fn kernels(&self, precision: Precision) -> &Kernels {
        &self.kernels[usize::from(precision == Precision::Single)]
    }
}

/// `cudaExternalMemoryHandleDesc` (CUDA 13.1's driver_types.h): the handle's kind, a union of
/// a file descriptor or two pointers (a Win32 handle and a name), the size, the flags, reserved.
#[repr(C)]
struct HandleDescription {
    kind: i32,
    pad: i32,
    handle: [*mut c_void; 2],
    size: u64,
    flags: u32,
    reserved: [u32; 16],
}

/// `cudaExternalMemoryBufferDesc`.
#[repr(C)]
struct BufferDescription {
    offset: u64,
    size: u64,
    flags: u32,
    reserved: [u32; 16],
}

/// `cudaExternalMemoryDedicated`.
const DEDICATED: u32 = 1;

#[cfg(windows)]
const HANDLE_TYPE: vk::ExternalMemoryHandleTypeFlags =
    vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32;
#[cfg(not(windows))]
const HANDLE_TYPE: vk::ExternalMemoryHandleTypeFlags = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;

/// `cudaExternalMemoryHandleTypeOpaqueWin32`, or `…OpaqueFd`.
#[cfg(windows)]
const HANDLE_KIND: i32 = 2;
#[cfg(not(windows))]
const HANDLE_KIND: i32 = 1;

/// An exported memory's handle: an NT handle on Windows, a file descriptor on Linux.
#[cfg(windows)]
struct Handle(std::os::windows::io::OwnedHandle);
#[cfg(not(windows))]
struct Handle(std::os::raw::c_int);

impl Handle {
    #[cfg(windows)]
    fn describe(&self, d: &mut HandleDescription) {
        use std::os::windows::io::AsRawHandle;
        d.handle[0] = self.0.as_raw_handle().cast();
    }

    #[cfg(not(windows))]
    fn describe(&self, d: &mut HandleDescription) {
        // the union's int, in its first bytes
        let mut first = [0u8; size_of::<*mut c_void>()];
        first[..4].copy_from_slice(&self.0.to_ne_bytes());
        d.handle[0] = usize::from_ne_bytes(first) as *mut c_void;
    }

    /// After CUDA's import: closed on Windows (the import doesn't take it).
    #[cfg(windows)]
    fn release(self) {
        drop(self.0);
    }

    /// After CUDA's import: on Linux CUDA owns the descriptor.
    #[cfg(not(windows))]
    fn release(self) {}
}

#[cfg(windows)]
fn export_handle(
    instance: &ash::Instance,
    device: &ash::Device,
    memory: vk::DeviceMemory,
) -> std::result::Result<Handle, vk::Result> {
    use std::os::windows::io::FromRawHandle;
    let win32 = ash::khr::external_memory_win32::Device::new(instance, device);
    let info = vk::MemoryGetWin32HandleInfoKHR::default()
        .memory(memory)
        .handle_type(HANDLE_TYPE);
    // SAFETY: the memory was allocated exportable as OPAQUE_WIN32 on this device
    let raw = unsafe { win32.get_memory_win32_handle(&info) }?;
    // SAFETY: a new NT handle, ours to close
    Ok(Handle(unsafe {
        std::os::windows::io::OwnedHandle::from_raw_handle(raw as *mut c_void)
    }))
}

#[cfg(not(windows))]
fn export_handle(
    instance: &ash::Instance,
    device: &ash::Device,
    memory: vk::DeviceMemory,
) -> std::result::Result<Handle, vk::Result> {
    let fd = ash::khr::external_memory_fd::Device::new(instance, device);
    let info = vk::MemoryGetFdInfoKHR::default()
        .memory(memory)
        .handle_type(HANDLE_TYPE);
    // SAFETY: the memory was allocated exportable as OPAQUE_FD on this device
    Ok(Handle(unsafe { fd.get_memory_fd(&info) }?))
}

/// A buffer Vulkan and CUDA both see: CUDA's pointer, its import, and wgpu's buffer, which owns
/// the memory.
pub(crate) struct Shared {
    runtime: Arc<Runtime>,
    cuda: Option<Buffer>,
    external: Opaque,
    pub(crate) buffer: wgpu::Buffer,
}

// SAFETY: the import is a handle, not tied to a thread; every CUDA call on it is behind serial()
unsafe impl Send for Shared {}
// SAFETY: as Send's
unsafe impl Sync for Shared {}

impl Shared {
    /// CUDA's view, taken once (for a vector of cuSPARSE's, which frees it): the shared memory
    /// then lives as long as `self`.
    pub(crate) fn take_cuda(&mut self) -> Option<Buffer> {
        self.cuda.take()
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        // CUDA's pointer first (cudaFree), then the import; wgpu's buffer and its memory after
        drop(self.cuda.take());
        if let Some(external) = &self.runtime.external {
            let _serial = serial();
            // SAFETY: imported once, its mapped pointer freed above (or by its vector, which
            // the owner drops first), destroyed once
            unsafe { (external.destroy)(self.external) };
        }
    }
}

/// The stencil's operators on the GPU: the cycle's (shifted) and GMRES's, their data, and the
/// buffers between the passes.
pub(crate) struct GpuStencil {
    interop: Arc<Interop>,
    precision: Precision,
    /// The cycle's operator and GMRES's: the bindings, with their own k₀².
    bindings: Vec<wgpu::BindGroup>,
    groups: (u32, u32, u32),
    _buffers: Vec<wgpu::Buffer>,
}

impl GpuStencil {
    /// The stencils `operators` (the same grid, permittivity and stretches; each its own k₀²)
    /// on the GPU, in `precision`, from `input` to `output` (each three values a cell).
    ///
    /// # Errors
    ///
    /// For stencils that differ in more than k₀², and for a grid too large for one dispatch.
    pub(crate) fn new(
        interop: &Arc<Interop>,
        operators: &[Stencil<'_>],
        precision: Precision,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
    ) -> Result<GpuStencil> {
        let first = operators
            .first()
            .ok_or_else(|| error("a stencil to put on the GPU"))?;
        for o in &operators[1..] {
            let same = o.cells() == first.cells()
                && o.epsilon() == first.epsilon()
                && o.node_inverse() == first.node_inverse()
                && (0..3).all(|a| {
                    o.at_nodes(a) == first.at_nodes(a)
                        && o.at_halves(a) == first.at_halves(a)
                        && o.phase(a) == first.phase(a)
                });
            if !same {
                return Err(error(
                    "GMRES's operator and the cycle's finest level differ in more than the shift",
                ));
            }
        }
        let [nx, ny, nz] = first.cells();
        let cells = nx * ny * nz;
        if ny > 65_535 || nz > 65_535 || nx.div_ceil(64) > 65_535 {
            return Err(error(format!(
                "a grid of {nx} × {ny} × {nz} cells is too large"
            )));
        }
        let complex = precision.complex();
        let device = &interop.device;
        let eps = interop.buffer(
            3 * cells as u64 * complex,
            Some(&precision.bytes(first.epsilon())),
        );
        let inverse = interop.buffer(
            cells as u64 * complex,
            Some(&precision.bytes(first.node_inverse())),
        );
        let h = interop.buffer(3 * cells as u64 * complex, None);
        let phi = interop.buffer(cells as u64 * complex, None);
        // the constants: k₀², the phases and their inverses, then the steps
        let one = c64::new(1.0, 0.0);
        let mut offsets = (Vec::new(), Vec::new());
        let mut steps = Vec::new();
        let at = 7;
        for a in 0..3 {
            offsets.0.push((at + steps.len()) as u32);
            steps.extend_from_slice(first.at_nodes(a));
        }
        for a in 0..3 {
            offsets.1.push((at + steps.len()) as u32);
            steps.extend_from_slice(first.at_halves(a));
        }
        let mut params = Vec::with_capacity(64);
        for v in [nx, ny, nz, cells] {
            params.extend_from_slice(&(v as u32).to_le_bytes());
        }
        for a in 0..3 {
            params.extend_from_slice(&u32::from(first.phase(a).is_none()).to_le_bytes());
        }
        params.extend_from_slice(&0u32.to_le_bytes());
        for list in [&offsets.0, &offsets.1] {
            for &o in list.iter() {
                params.extend_from_slice(&o.to_le_bytes());
            }
            params.extend_from_slice(&0u32.to_le_bytes());
        }
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stencil params"),
            size: params.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        interop.queue.write_buffer(&uniform, 0, &params);
        let mut buffers = vec![eps, inverse, h, phi, uniform];
        let mut bindings = Vec::new();
        for o in operators {
            let mut consts = vec![o.k0_squared()];
            consts.extend((0..3).map(|a| o.phase(a).unwrap_or(one)));
            consts.extend((0..3).map(|a| o.phase(a).map_or(one, |p| 1.0 / p)));
            consts.extend_from_slice(&steps);
            let buffer = interop.buffer(
                consts.len() as u64 * complex,
                Some(&precision.bytes(&consts)),
            );
            let entries: Vec<wgpu::BindGroupEntry<'_>> = [
                &buffers[4],
                input,
                output,
                &buffers[2],
                &buffers[3],
                &buffers[0],
                &buffers[1],
                &buffer,
            ]
            .into_iter()
            .enumerate()
            .map(|(binding, b)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
            bindings.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("stencil"),
                layout: &interop.layout,
                entries: &entries,
            }));
            buffers.push(buffer);
        }
        Ok(GpuStencil {
            interop: interop.clone(),
            precision,
            bindings,
            groups: (nx.div_ceil(64) as u32, ny as u32, nz as u32),
            _buffers: buffers,
        })
    }

    /// Records `count` products by operator `which` (its place in `new`'s list).
    pub(crate) fn encode(&self, which: usize, count: usize) -> wgpu::CommandBuffer {
        let kernels = self.interop.kernels(self.precision);
        let mut encoder = self
            .interop
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_bind_group(0, &self.bindings[which], &[]);
            let (x, y, z) = self.groups;
            for _ in 0..count {
                for pipeline in [&kernels.curl_e, &kernels.potential, &kernels.product] {
                    pass.set_pipeline(pipeline);
                    pass.dispatch_workgroups(x, y, z);
                }
            }
        }
        encoder.finish()
    }

    /// The product by operator `which`, from the input to the output, done when this returns.
    pub(crate) fn apply(&self, which: usize) -> Result<()> {
        self.interop.queue.submit([self.encode(which, 1)]);
        self.interop.wait()
    }
}
