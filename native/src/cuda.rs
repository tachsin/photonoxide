//! The CUDA runtime as NVIDIA's backends share it: device memory, copies, and one call at a
//! time in the process.

use std::cell::Cell;
use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::{Arc, Mutex, MutexGuard};

use photonoxide::Result;

use crate::library::{Library, error, load};
use crate::nvidia::CUDA_RUNTIME;

pub(crate) type Opaque = *mut c_void;

// cudaMemcpyKind and cudaDataType, from CUDA 13.1's headers
const HOST_TO_DEVICE: c_int = 1;
const DEVICE_TO_HOST: c_int = 2;
pub(crate) const C_64F: c_int = 5;
pub(crate) const R_32I: c_int = 10;

/// One CUDA or library call at a time in the process: cuDSS's handles used from several
/// threads at once fail in its analysis (status 5), and there is one GPU to share anyway.
static SERIAL: Mutex<()> = Mutex::new(());

thread_local! {
    static HOLDING: Cell<bool> = const { Cell::new(false) };
}

/// [`SERIAL`] held by this thread, taken again freely by it (an object dropped on an early
/// return of a call that holds it).
pub(crate) struct Serial(Option<MutexGuard<'static, ()>>);

pub(crate) fn serial() -> Serial {
    if HOLDING.get() {
        return Serial(None);
    }
    let guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    HOLDING.set(true);
    Serial(Some(guard))
}

impl Drop for Serial {
    fn drop(&mut self) {
        if self.0.is_some() {
            HOLDING.set(false);
        }
    }
}

pub(crate) fn check(status: c_int, call: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(error(format!("{call} failed with status {status}")))
    }
}

/// The CUDA runtime's functions, and the library that holds them.
pub(crate) struct Runtime {
    malloc: unsafe extern "C" fn(*mut Opaque, usize) -> c_int,
    free: unsafe extern "C" fn(Opaque) -> c_int,
    memcpy: unsafe extern "C" fn(Opaque, *const c_void, usize, c_int) -> c_int,
    synchronize: unsafe extern "C" fn() -> c_int,
    mem_get_info: unsafe extern "C" fn(*mut usize, *mut usize) -> c_int,
    // last: the functions above must not outlive it
    _library: Library,
}

impl Runtime {
    /// The CUDA runtime, found and loaded: before a library that needs it, so that its own
    /// dependency on it is the same library.
    pub(crate) fn load() -> std::result::Result<Arc<Runtime>, String> {
        let (found, library) = load(&CUDA_RUNTIME, None, |_| Ok(()));
        let library = library.ok_or_else(|| found.reason())?;
        let runtime = || -> Result<Runtime> {
            // SAFETY: each type is the function's declaration in cuda_runtime_api.h
            unsafe {
                Ok(Runtime {
                    malloc: library.function("cudaMalloc")?,
                    free: library.function("cudaFree")?,
                    memcpy: library.function("cudaMemcpy")?,
                    synchronize: library.function("cudaDeviceSynchronize")?,
                    mem_get_info: library.function("cudaMemGetInfo")?,
                    _library: library,
                })
            }
        };
        runtime().map(Arc::new).map_err(|e| e.to_string())
    }

    /// Waits for the GPU's work, and reports its error, after `what`.
    pub(crate) fn synchronize(&self, what: &str) -> Result<()> {
        let _serial = serial();
        // SAFETY: no arguments
        check(
            unsafe { (self.synchronize)() },
            &format!("cudaDeviceSynchronize after {what}"),
        )
    }

    /// The GPU's free memory, in bytes.
    pub(crate) fn free_memory(&self) -> Result<usize> {
        let _serial = serial();
        let (mut free, mut total) = (0, 0);
        check(
            // SAFETY: two writable size_t
            unsafe { (self.mem_get_info)(&mut free, &mut total) },
            "cudaMemGetInfo",
        )?;
        Ok(free)
    }
}

/// Memory on the GPU, freed when dropped.
pub(crate) struct Buffer {
    runtime: Arc<Runtime>,
    pub(crate) ptr: Opaque,
    bytes: usize,
}

// SAFETY: device memory isn't tied to a thread; every use is behind serial()
unsafe impl Send for Buffer {}
// SAFETY: as Send's
unsafe impl Sync for Buffer {}

impl Buffer {
    /// Device memory holding `data`.
    pub(crate) fn new<T: Copy>(runtime: &Arc<Runtime>, data: &[T]) -> Result<Buffer> {
        let buffer = Buffer::with_bytes(runtime, size_of_val(data))?;
        buffer.write(data)?;
        Ok(buffer)
    }

    /// `bytes` of device memory, its contents undefined.
    pub(crate) fn with_bytes(runtime: &Arc<Runtime>, bytes: usize) -> Result<Buffer> {
        let _serial = serial();
        let bytes = bytes.max(1);
        let mut ptr = std::ptr::null_mut();
        // SAFETY: ptr is writable; bytes > 0
        check(unsafe { (runtime.malloc)(&mut ptr, bytes) }, "cudaMalloc")?;
        Ok(Buffer {
            runtime: runtime.clone(),
            ptr,
            bytes,
        })
    }

    pub(crate) fn write<T: Copy>(&self, data: &[T]) -> Result<()> {
        let _serial = serial();
        let bytes = size_of_val(data);
        assert!(bytes <= self.bytes);
        check(
            // SAFETY: the device buffer holds self.bytes ≥ bytes; data is bytes long
            unsafe { (self.runtime.memcpy)(self.ptr, data.as_ptr().cast(), bytes, HOST_TO_DEVICE) },
            "cudaMemcpy to the GPU",
        )
    }

    pub(crate) fn read<T: Copy>(&self, out: &mut [T]) -> Result<()> {
        let _serial = serial();
        let bytes = size_of_val(out);
        assert!(bytes <= self.bytes);
        check(
            // SAFETY: out is bytes long and writable; the device buffer holds at least that
            unsafe {
                (self.runtime.memcpy)(out.as_mut_ptr().cast(), self.ptr, bytes, DEVICE_TO_HOST)
            },
            "cudaMemcpy from the GPU",
        )
    }

    /// Its size in bytes.
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let _serial = serial();
        // SAFETY: ptr came from cudaMalloc and is freed once
        unsafe { (self.runtime.free)(self.ptr) };
    }
}
