//! ROADMAP's pitfall: the system allocator limited 18 threads to 13 solves a second (30 with
//! mimalloc). The rule: nothing is allocated in the time-stepping and solve loops. Here a
//! counting allocator counts what the loops allocate, on every thread of a pool of four, after a
//! first stretch that makes what is made once.
//!
//! A global allocator needs `unsafe`, which the library forbids; this test binary is its own
//! crate, and the allocator only counts and passes every call on to the system's. Each test
//! counts on the threads it marks (its own and its pool's), so tests running side by side, and
//! the harness, don't count in each other's numbers. The test modules are named for their areas
//! (`fdtd::`, `fdfd::`).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The counters, one per test; slot 0 counts nothing.
static COUNTS: [AtomicUsize; 3] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

thread_local! {
    /// The counter this thread counts in: 0 for none.
    static SLOT: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

impl Counting {
    fn count() {
        // a thread being torn down has no slot left: count nothing
        let slot = SLOT.try_with(Cell::get).unwrap_or(0);
        if slot != 0 {
            COUNTS[slot].fetch_add(1, Ordering::Relaxed);
        }
    }
}

// SAFETY: every call is passed on to the system allocator unchanged; counting touches only an
// atomic and a thread-local `Cell` with a constant initializer, neither of which allocates.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Counting::count();
        // SAFETY: the caller's contract for `alloc`, passed on
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        Counting::count();
        // SAFETY: the caller's contract for `alloc_zeroed`, passed on
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        Counting::count();
        // SAFETY: the caller's contract for `realloc`, passed on
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract for `dealloc`, passed on
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// A pool of four threads counting in `slot`, and the calling thread marked too.
fn counted_pool(slot: usize) -> rayon::ThreadPool {
    SLOT.with(|s| s.set(slot));
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .start_handler(move |_| SLOT.with(|s| s.set(slot)))
        .build()
        .expect("a pool of four threads")
}

/// The allocations `f` makes, counted in `slot`.
fn allocations(slot: usize, f: impl FnOnce()) -> usize {
    let before = COUNTS[slot].load(Ordering::SeqCst);
    f();
    COUNTS[slot].load(Ordering::SeqCst) - before
}

mod fdtd {
    use photonoxide::fdfd::{Axis, Grid3d};
    use photonoxide::fdtd::{Boundaries, Field, FluxPlane, Simulation, Source, Waveform};
    use photonoxide::units::Frequency;

    use super::{allocations, counted_pool};

    /// A silicon cube in oxide inside CPMLs, a pulse at its centre, a transform box and a flux
    /// plane recording: after the first few steps, 100 more allocate nothing, on any thread. (It
    /// was 34 a step: rayon's `skip` over a CPML slab's planes, 24, and each transform's phases,
    /// 10.)
    #[test]
    fn a_time_step_allocates_nothing() {
        let pool = counted_pool(1);
        let n = 28;
        let grid = Grid3d {
            nx: n,
            ny: n,
            nz: n,
            dx: 0.04,
            dy: 0.04,
            dz: 0.04,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        };
        let centre = 0.5 * n as f64 * 0.04;
        let cube = move |x: f64, y: f64, z: f64| {
            let inside = [x, y, z].iter().all(|c| (c - centre).abs() < 0.2);
            if inside { 12.1 } else { 2.085 }
        };
        let frequency = Frequency::natural(1.0 / 1.55).expect("a frequency");
        let mut s = Simulation::new(grid, cube, Boundaries::cpml(8), 0.5).expect("the problem");
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (n / 2, n / 2, n / 2),
            waveform: Waveform::pulse(frequency, 0.2).expect("a pulse"),
        })
        .expect("the source");
        s.add_dft((10, 10, 10), (17, 17, 17), &[frequency])
            .expect("the transform");
        s.add_flux(FluxPlane::new(Axis::X, 20), &[frequency])
            .expect("the flux");
        pool.install(|| {
            s.run(5);
            let steps = 100;
            let made = allocations(1, || s.run(steps));
            assert_eq!(made, 0, "{made} allocations in {steps} time steps");
        });
    }
}

mod fdfd {
    use num_complex::Complex64 as c64;
    use photonoxide::fdfd::{Axis, Boundaries3d, Formulation, Grid3d, IterativeSolver3d, Stopping};
    use photonoxide::units::Wavelength;

    use super::{allocations, counted_pool};

    /// The allocations of a solve held to `iterations`, its tolerance out of reach.
    fn solve(solver: &IterativeSolver3d, current: &[c64], iterations: usize) -> usize {
        let stopping = Stopping {
            tolerance: 1e-300,
            max_iterations: iterations,
        };
        // the tolerance isn't reached, which is an error: what is counted is the loop's work
        allocations(2, || drop(solver.solve(current, stopping)))
    }

    /// A silicon strip in oxide inside PMLs, solved by QMR on the curl-curl operator: twice the
    /// iterations allocate no more than the convergence history's reallocations (it doubles its
    /// room) and the stop's message; an allocation in an iteration would add one an iteration.
    /// (QMR preconditioned by ILU(0) and GMRES with the multigrid cycle still allocate in each
    /// iteration, about 8 and 47 on this strip, and aren't held to it yet.)
    #[test]
    fn a_solves_iteration_allocates_nothing() {
        let pool = counted_pool(2);
        let grid = Grid3d {
            nx: 16,
            ny: 14,
            nz: 12,
            dx: 0.05,
            dy: 0.05,
            dz: 0.05,
            x0: -0.4,
            y0: -0.35,
            z0: -0.3,
        };
        let wavelength = Wavelength::um(1.55).expect("a wavelength");
        let strip = |_: f64, y: f64, z: f64| {
            let n = if y.abs() < 0.25 && z.abs() < 0.11 {
                3.476
            } else {
                1.444
            };
            c64::new(n * n, 0.0)
        };
        let solver = IterativeSolver3d::new(
            grid,
            wavelength,
            strip,
            Boundaries3d::pml(4),
            Formulation::CurlCurl,
        )
        .expect("the problem");
        let mut current = vec![c64::new(0.0, 0.0); grid.unknowns()];
        current[grid.index(Axis::Y, (8, 7, 6))] = c64::new(1.0, 0.0);
        let iterations = 40;
        pool.install(|| {
            solve(&solver, &current, 4);
            let some = solve(&solver, &current, iterations);
            let twice = solve(&solver, &current, 2 * iterations);
            assert!(
                twice <= some + 4,
                "{some} allocations in {iterations} iterations, {twice} in {}",
                2 * iterations
            );
        });
    }
}
