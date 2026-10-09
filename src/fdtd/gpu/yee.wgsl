// photonoxide's FDTD on the GPU: the Yee updates with the CPML in its slabs, the sources and
// currents, the probes and the transforms, one step at a time in time order. `real` is f32 or
// f64: the host puts its alias first (`alias real = f32;`) before compiling.
//
// Every value is written by one invocation, and every sum is taken by one invocation in a fixed
// order: no atomics, no reductions across invocations, so a run repeats bit for bit on the same
// device and driver.
//
// The curl updates march along z, a plane at a time (P. Micikevicius, GPGPU-2, 79 (2009),
// doi:10.1145/1513895.1513905): a workgroup of 32 x 8 invocations owns a tile of 32 x 8 values in
// every plane of a run of planes; it puts each plane of the other field into workgroup memory
// with the one row and column of neighbours the curl needs, and keeps its own column's value of
// the plane before (or after) in registers, so each value of the other field is read from memory
// about once.

const TX: u32 = 32u;
const TY: u32 = 8u;
// a tile's row with its neighbours: TX + 1 values, TY + 1 rows
const SX: u32 = 33u;

struct Params {
    // nx, ny, nz, the cells
    n: vec4<u32>,
    // 1 where an axis wraps (periodic), x, y, z
    periodic: vec4<u32>,
    // the CPML's cells at each axis's low and high ends
    low: vec4<u32>,
    high: vec4<u32>,
    // planes a workgroup marches through, the injections' groups on H and on E, the probes
    run: vec4<u32>,
    // emitters, transformed series, transformed values, phases a step
    more: vec4<u32>,
    // each CPML slab, at ((field * 3 + component) * 3 + axis) * 2 + side (field 0 for E, 1 for
    // H; side 0 low, 1 high): its first index along the axis, its count (0: none), where its ψ
    // starts, where its b and a start (b then a, count each)
    slabs: array<vec4<u32>, 36>,
}

struct Step {
    // the step within the submission, the step within the run
    m: u32,
    run_step: u32,
    unused0: u32,
    unused1: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
// E and H̃, the three components one after the other
@group(0) @binding(1) var<storage, read_write> e: array<real>;
@group(0) @binding(2) var<storage, read_write> h: array<real>;
// E's coefficients: E ← ca E + cb (∇ × H̃)
@group(0) @binding(3) var<storage, read> ca: array<real>;
@group(0) @binding(4) var<storage, read> cb: array<real>;
// Δt, Δx, Δy, Δz, then κ/Δ along x, y and z for E's differences, then for H̃'s
@group(0) @binding(5) var<storage, read> reals: array<real>;
// the CPML's ψ, slab after slab, and their recursions' b and a
@group(0) @binding(6) var<storage, read_write> psi: array<real>;
@group(0) @binding(7) var<storage, read> coefs: array<real>;
// the injections: per group, its value's index (component * cells + index), its first entry and
// its entries' count; per entry, its emitter and its amplitude (re, im); per step and emitter, the
// waveform (re, im)
@group(0) @binding(8) var<storage, read> groups: array<u32>;
@group(0) @binding(9) var<storage, read> emitters: array<u32>;
@group(0) @binding(10) var<storage, read> amplitudes: array<real>;
@group(0) @binding(11) var<storage, read> waves: array<real>;
// the probes: per probe, its field and its value's index; their values, step by step
@group(0) @binding(12) var<storage, read> probes: array<u32>;
@group(0) @binding(13) var<storage, read_write> recorded: array<real>;
// the transforms: per series, 12 numbers (below); per step, the phases e^(iωt) Δt; the sums
@group(0) @binding(14) var<storage, read> series: array<u32>;
@group(0) @binding(15) var<storage, read> phases: array<real>;
@group(0) @binding(16) var<storage, read_write> sums: array<real>;

// The fused step's other copy of E, H̃ and ψ: it reads one copy and writes the other.
@group(0) @binding(17) var<storage, read_write> e_next: array<real>;
@group(0) @binding(18) var<storage, read_write> h_next: array<real>;
@group(0) @binding(19) var<storage, read_write> psi_next: array<real>;

@group(1) @binding(0) var<uniform> now: Step;

var<workgroup> tile_x: array<real, 297>;
var<workgroup> tile_y: array<real, 297>;
var<workgroup> tile_z: array<real, 297>;

// The index along an axis of n values for the position m, one beyond either end included:
// across a periodic side it wraps; beyond a wall, -1 (the field is zero there).
fn resolve(m: i32, n: i32, periodic: bool) -> i32 {
    if (m >= 0 && m < n) {
        return m;
    }
    if (periodic) {
        if (m == n) {
            return 0;
        }
        if (m == -1) {
            return n - 1;
        }
    }
    return -1;
}

// A component's index of the value at (i, j, k), or -1 beyond a wall.
fn index_of(i: i32, j: i32, k: i32) -> i32 {
    let n = vec3<i32>(params.n.xyz);
    let ri = resolve(i, n.x, params.periodic.x != 0u);
    let rj = resolve(j, n.y, params.periodic.y != 0u);
    let rk = resolve(k, n.z, params.periodic.z != 0u);
    if (ri < 0 || rj < 0 || rk < 0) {
        return -1;
    }
    return (rk * n.y + rj) * n.x + ri;
}

fn load_e(c: u32, i: i32, j: i32, k: i32) -> real {
    let r = index_of(i, j, k);
    if (r < 0) {
        return real(0.0);
    }
    return e[c * params.n.w + u32(r)];
}

fn load_h(c: u32, i: i32, j: i32, k: i32) -> real {
    let r = index_of(i, j, k);
    if (r < 0) {
        return real(0.0);
    }
    return h[c * params.n.w + u32(r)];
}

// κ/Δ along `axis` at index m, for E's differences (field 0) or H̃'s (field 1).
fn factor(field: u32, axis: u32, m: u32) -> real {
    let n = params.n;
    var start = 4u + field * (n.x + n.y + n.z);
    if (axis > 0u) {
        start += n.x;
    }
    if (axis > 1u) {
        start += n.y;
    }
    return reals[start + m];
}

// One CPML correction of the value of `field`'s component c at `at`, for its slab along axis w:
// ψ ← b ψ + a d, d the derivative along w of the other field's component (without κ), and the
// new ψ returned. Called only where the value is in a slab along w.
fn convolve(field: u32, c: u32, w: u32, at: vec3<u32>, d: real) -> real {
    let n = params.n;
    let m = at[w];
    var side = 0u;
    if (m >= params.low[w]) {
        side = 1u;
    }
    let s = params.slabs[((field * 3u + c) * 3u + w) * 2u + side];
    var local = at;
    var dims = n.xyz;
    local[w] = m - s.x;
    dims[w] = s.y;
    let p = s.z + (local.z * dims.y + local.y) * dims.x + local.x;
    let q = m - s.x;
    let v = coefs[s.w + q] * psi[p] + coefs[s.w + s.y + q] * d;
    psi[p] = v;
    return v;
}

// Whether the medium has no conductivity: ca is then 1 wherever cb isn't 0, and 0 where it is
// (walls and conductors), and isn't read.
override lossless: bool = false;

// ca at value r, whose cb is `cbv`.
fn keep(r: u32, cbv: real) -> real {
    if (lossless) {
        return select(real(1.0), real(0.0), cbv == real(0.0));
    }
    return ca[r];
}

// Whether `at` is in a CPML slab along axis w.
fn in_slab(at: vec3<u32>, w: u32) -> bool {
    let m = at[w];
    return m < params.low[w] || m + params.high[w] >= params.n[w];
}

// H̃ ← H̃ − Δt (∇ × E), the differences forward, then the CPML's ψ in its slabs.
@compute @workgroup_size(32, 8, 1)
fn update_h(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let n = params.n;
    let cells = n.w;
    let lx = lid.x;
    let ly = lid.y;
    let i = wid.x * TX + lx;
    let j = wid.y * TY + ly;
    let ii = i32(i);
    let jj = i32(j);
    let k0 = wid.z * params.run.x;
    let k1 = min(k0 + params.run.x, n.z);
    let inside = i < n.x && j < n.y;
    let dt = reals[0];
    let hx = reals[1];
    let hy = reals[2];
    let hz = reals[3];
    var fx = real(0.0);
    var fy = real(0.0);
    if (inside) {
        fx = factor(1u, 0u, i);
        fy = factor(1u, 1u, j);
    }
    let own = ly * SX + lx;
    // E_x and E_y of this column at plane k
    var ex = load_e(0u, ii, jj, i32(k0));
    var ey = load_e(1u, ii, jj, i32(k0));
    for (var k = k0; k < k1; k++) {
        let kk = i32(k);
        workgroupBarrier();
        let ez = load_e(2u, ii, jj, kk);
        tile_x[own] = ex;
        tile_y[own] = ey;
        tile_z[own] = ez;
        if (ly == TY - 1u) {
            tile_x[own + SX] = load_e(0u, ii, jj + 1, kk);
            tile_z[own + SX] = load_e(2u, ii, jj + 1, kk);
        }
        if (lx == TX - 1u) {
            tile_y[own + 1u] = load_e(1u, ii + 1, jj, kk);
            tile_z[own + 1u] = load_e(2u, ii + 1, jj, kk);
        }
        let ex_next = load_e(0u, ii, jj, kk + 1);
        let ey_next = load_e(1u, ii, jj, kk + 1);
        workgroupBarrier();
        if (inside) {
            let r = (k * n.y + j) * n.x + i;
            let fz = factor(1u, 2u, k);
            let ex_y = tile_x[own + SX];
            let ey_x = tile_y[own + 1u];
            let ez_x = tile_z[own + 1u];
            let ez_y = tile_z[own + SX];
            // (∇ × E)_c = ∂E_b/∂a κ_a/Δ_a − ∂E_a/∂b κ_b/Δ_b, (a, b) the axes after c
            let curl_x = (ez_y - ez) * fy - (ey_next - ey) * fz;
            let curl_y = (ex_next - ex) * fz - (ez_x - ez) * fx;
            let curl_z = (ey_x - ey) * fx - (ex_y - ex) * fy;
            var vx = h[r] - dt * curl_x;
            var vy = h[cells + r] - dt * curl_y;
            var vz = h[2u * cells + r] - dt * curl_z;
            let at = vec3<u32>(i, j, k);
            let sx = in_slab(at, 0u);
            let sy = in_slab(at, 1u);
            let sz = in_slab(at, 2u);
            if (sx || sy || sz) {
                // each component's slabs in the order of the axes; the sign that of the
                // derivative in the curl
                if (sy) {
                    vx = vx - dt * convolve(1u, 0u, 1u, at, (ez_y - ez) / hy);
                }
                if (sz) {
                    vx = vx - dt * -convolve(1u, 0u, 2u, at, (ey_next - ey) / hz);
                }
                if (sx) {
                    vy = vy - dt * -convolve(1u, 1u, 0u, at, (ez_x - ez) / hx);
                }
                if (sz) {
                    vy = vy - dt * convolve(1u, 1u, 2u, at, (ex_next - ex) / hz);
                }
                if (sx) {
                    vz = vz - dt * convolve(1u, 2u, 0u, at, (ey_x - ey) / hx);
                }
                if (sy) {
                    vz = vz - dt * -convolve(1u, 2u, 1u, at, (ex_y - ex) / hy);
                }
            }
            h[r] = vx;
            h[cells + r] = vy;
            h[2u * cells + r] = vz;
        }
        ex = ex_next;
        ey = ey_next;
    }
}

// E ← ca E + cb (∇ × H̃), the differences back, then the CPML's ψ in its slabs.
@compute @workgroup_size(32, 8, 1)
fn update_e(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let n = params.n;
    let cells = n.w;
    let lx = lid.x;
    let ly = lid.y;
    let i = wid.x * TX + lx;
    let j = wid.y * TY + ly;
    let ii = i32(i);
    let jj = i32(j);
    let k0 = wid.z * params.run.x;
    let k1 = min(k0 + params.run.x, n.z);
    let inside = i < n.x && j < n.y;
    let hx = reals[1];
    let hy = reals[2];
    let hz = reals[3];
    var fx = real(0.0);
    var fy = real(0.0);
    if (inside) {
        fx = factor(0u, 0u, i);
        fy = factor(0u, 1u, j);
    }
    // the tile is shifted by one: its first row and column are the neighbours before it
    let own = (ly + 1u) * SX + lx + 1u;
    // H̃_x and H̃_y of this column at plane k − 1
    var hx_before = load_h(0u, ii, jj, i32(k0) - 1);
    var hy_before = load_h(1u, ii, jj, i32(k0) - 1);
    for (var k = k0; k < k1; k++) {
        let kk = i32(k);
        workgroupBarrier();
        let vhx = load_h(0u, ii, jj, kk);
        let vhy = load_h(1u, ii, jj, kk);
        let vhz = load_h(2u, ii, jj, kk);
        tile_x[own] = vhx;
        tile_y[own] = vhy;
        tile_z[own] = vhz;
        if (ly == 0u) {
            tile_x[own - SX] = load_h(0u, ii, jj - 1, kk);
            tile_z[own - SX] = load_h(2u, ii, jj - 1, kk);
        }
        if (lx == 0u) {
            tile_y[own - 1u] = load_h(1u, ii - 1, jj, kk);
            tile_z[own - 1u] = load_h(2u, ii - 1, jj, kk);
        }
        workgroupBarrier();
        if (inside) {
            let r = (k * n.y + j) * n.x + i;
            let fz = factor(0u, 2u, k);
            let hx_y = tile_x[own - SX];
            let hy_x = tile_y[own - 1u];
            let hz_x = tile_z[own - 1u];
            let hz_y = tile_z[own - SX];
            let curl_x = (vhz - hz_y) * fy - (vhy - hy_before) * fz;
            let curl_y = (vhx - hx_before) * fz - (vhz - hz_x) * fx;
            let curl_z = (vhy - hy_x) * fx - (vhx - hx_y) * fy;
            let cbx = cb[r];
            let cby = cb[cells + r];
            let cbz = cb[2u * cells + r];
            var vx = keep(r, cbx) * e[r] + cbx * curl_x;
            var vy = keep(cells + r, cby) * e[cells + r] + cby * curl_y;
            var vz = keep(2u * cells + r, cbz) * e[2u * cells + r] + cbz * curl_z;
            let at = vec3<u32>(i, j, k);
            let sx = in_slab(at, 0u);
            let sy = in_slab(at, 1u);
            let sz = in_slab(at, 2u);
            if (sx || sy || sz) {
                if (sy) {
                    vx = vx + cbx * convolve(0u, 0u, 1u, at, (vhz - hz_y) / hy);
                }
                if (sz) {
                    vx = vx + cbx * -convolve(0u, 0u, 2u, at, (vhy - hy_before) / hz);
                }
                if (sx) {
                    vy = vy + cby * -convolve(0u, 1u, 0u, at, (vhz - hz_x) / hx);
                }
                if (sz) {
                    vy = vy + cby * convolve(0u, 1u, 2u, at, (vhx - hx_before) / hz);
                }
                if (sx) {
                    vz = vz + cbz * convolve(0u, 2u, 0u, at, (vhy - hy_x) / hx);
                }
                if (sy) {
                    vz = vz + cbz * -convolve(0u, 2u, 1u, at, (vhx - hx_y) / hy);
                }
            }
            e[r] = vx;
            e[cells + r] = vy;
            e[2u * cells + r] = vz;
        }
        hx_before = vhx;
        hy_before = vhy;
    }
}

// ---- The fused step: H̃ and E in one pass ----
//
// Each workgroup marches along z as above, and at each plane takes H̃ over its tile and the row and
// column before it, then E over its tile from those: one pass a step reads E, H̃ and E's
// coefficients once and writes E and H̃ once. A neighbouring workgroup still reads the old E and
// H̃ around its tile while this one writes, so the new values go to the other copy (e_next,
// h_next, psi_next), and the copies swap each step. The values of H̃ on the row and column before
// the tile, and on the plane before the run of planes, are computed again by each workgroup that
// needs them, by the same function as by the one that owns them.

// E's region of a plane: the tile with a row and column before it and after it
const RX: u32 = 34u;
const RE: u32 = 340u;
// H̃'s: the tile with the row and column before it
const HX: u32 = 33u;
const RH: u32 = 297u;

var<workgroup> plane_e: array<real, 2040>;
var<workgroup> plane_h: array<real, 891>;

// As `convolve`, ψ read from this step's copy and, if `write`, written to the next.
fn convolve_into(field: u32, c: u32, w: u32, at: vec3<u32>, d: real, write: bool) -> real {
    let n = params.n;
    let m = at[w];
    var side = 0u;
    if (m >= params.low[w]) {
        side = 1u;
    }
    let s = params.slabs[((field * 3u + c) * 3u + w) * 2u + side];
    var local = at;
    var dims = n.xyz;
    local[w] = m - s.x;
    dims[w] = s.y;
    let p = s.z + (local.z * dims.y + local.y) * dims.x + local.x;
    let q = m - s.x;
    let v = coefs[s.w + q] * psi[p] + coefs[s.w + s.y + q] * d;
    if (write) {
        psi_next[p] = v;
    }
    return v;
}

// H̃ at t + Δt/2 at `at` (on the grid), from H̃ there at t − Δt/2 and E at t: its own values
// `own` and its neighbours forward along x (E_y, E_z), y (E_x, E_z) and z (E_x, E_y); ψ written
// to the next copy if `write`.
fn h_at(at: vec3<u32>, own: vec3<real>, x: vec2<real>, y: vec2<real>, z: vec2<real>, write: bool) -> vec3<real> {
    let n = params.n;
    let cells = n.w;
    let r = (at.z * n.y + at.y) * n.x + at.x;
    let dt = reals[0];
    let fx = factor(1u, 0u, at.x);
    let fy = factor(1u, 1u, at.y);
    let fz = factor(1u, 2u, at.z);
    let ex = own.x;
    let ey = own.y;
    let ez = own.z;
    let curl_x = (y.y - ez) * fy - (z.y - ey) * fz;
    let curl_y = (z.x - ex) * fz - (x.y - ez) * fx;
    let curl_z = (x.x - ey) * fx - (y.x - ex) * fy;
    var vx = h[r] - dt * curl_x;
    var vy = h[cells + r] - dt * curl_y;
    var vz = h[2u * cells + r] - dt * curl_z;
    let sx = in_slab(at, 0u);
    let sy = in_slab(at, 1u);
    let sz = in_slab(at, 2u);
    if (sx || sy || sz) {
        let hx = reals[1];
        let hy = reals[2];
        let hz = reals[3];
        if (sy) {
            vx = vx - dt * convolve_into(1u, 0u, 1u, at, (y.y - ez) / hy, write);
        }
        if (sz) {
            vx = vx - dt * -convolve_into(1u, 0u, 2u, at, (z.y - ey) / hz, write);
        }
        if (sx) {
            vy = vy - dt * -convolve_into(1u, 1u, 0u, at, (x.y - ez) / hx, write);
        }
        if (sz) {
            vy = vy - dt * convolve_into(1u, 1u, 2u, at, (z.x - ex) / hz, write);
        }
        if (sx) {
            vz = vz - dt * convolve_into(1u, 2u, 0u, at, (x.x - ey) / hx, write);
        }
        if (sy) {
            vz = vz - dt * -convolve_into(1u, 2u, 1u, at, (y.x - ex) / hy, write);
        }
    }
    return vec3<real>(vx, vy, vz);
}

// One step, H̃ then E, from this step's copy into the next.
@compute @workgroup_size(32, 8, 1)
fn update_fused(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let n = params.n;
    let cells = n.w;
    let nn = vec3<i32>(n.xyz);
    let lane = lid.y * TX + lid.x;
    let i = wid.x * TX + lid.x;
    let j = wid.y * TY + lid.y;
    // the regions' first values: a row and a column before the tile
    let i0 = i32(wid.x * TX) - 1;
    let j0 = i32(wid.y * TY) - 1;
    let k0 = wid.z * params.run.x;
    let k1 = min(k0 + params.run.x, n.z);
    let inside = i < n.x && j < n.y;
    let one_plane = n.z == 1u && params.periodic.z != 0u;
    // H̃_x and H̃_y of this column at plane k − 1, at t + Δt/2
    var before = vec2<real>(real(0.0), real(0.0));
    if (inside && !one_plane) {
        let kb = resolve(i32(k0) - 1, nn.z, params.periodic.z != 0u);
        if (kb >= 0) {
            let ii = i32(i);
            let jj = i32(j);
            let own = vec3<real>(load_e(0u, ii, jj, kb), load_e(1u, ii, jj, kb), load_e(2u, ii, jj, kb));
            let x = vec2<real>(load_e(1u, ii + 1, jj, kb), load_e(2u, ii + 1, jj, kb));
            let y = vec2<real>(load_e(0u, ii, jj + 1, kb), load_e(2u, ii, jj + 1, kb));
            let z = vec2<real>(load_e(0u, ii, jj, kb + 1), load_e(1u, ii, jj, kb + 1));
            before = h_at(vec3<u32>(i, j, u32(kb)), own, x, y, z, false).xy;
        }
    }
    // E at plane k0
    for (var q = lane; q < RE; q += TX * TY) {
        let gi = i0 + i32(q % RX);
        let gj = j0 + i32(q / RX);
        for (var c = 0u; c < 3u; c++) {
            plane_e[c * RE + q] = load_e(c, gi, gj, i32(k0));
        }
    }
    for (var k = k0; k < k1; k++) {
        let slot = 3u * RE * ((k - k0) % 2u);
        let next = 3u * RE - slot;
        workgroupBarrier();
        // E at plane k + 1
        for (var q = lane; q < RE; q += TX * TY) {
            let gi = i0 + i32(q % RX);
            let gj = j0 + i32(q / RX);
            for (var c = 0u; c < 3u; c++) {
                plane_e[next + c * RE + q] = load_e(c, gi, gj, i32(k) + 1);
            }
        }
        workgroupBarrier();
        // H̃ at plane k over the tile and the row and column before it
        for (var q = lane; q < RH; q += TX * TY) {
            let hu = q % HX;
            let hv = q / HX;
            let gi = i0 + i32(hu);
            let gj = j0 + i32(hv);
            let ri = resolve(gi, nn.x, params.periodic.x != 0u);
            let rj = resolve(gj, nn.y, params.periodic.y != 0u);
            var value = vec3<real>(real(0.0), real(0.0), real(0.0));
            if (ri >= 0 && rj >= 0) {
                let p = hv * RX + hu;
                let own = vec3<real>(plane_e[slot + p], plane_e[slot + RE + p], plane_e[slot + 2u * RE + p]);
                let x = vec2<real>(plane_e[slot + RE + p + 1u], plane_e[slot + 2u * RE + p + 1u]);
                let y = vec2<real>(plane_e[slot + p + RX], plane_e[slot + 2u * RE + p + RX]);
                let z = vec2<real>(plane_e[next + p], plane_e[next + RE + p]);
                let owner = hu >= 1u && hv >= 1u && gi < nn.x && gj < nn.y;
                let at = vec3<u32>(u32(ri), u32(rj), k);
                value = h_at(at, own, x, y, z, owner);
                if (owner) {
                    let r = (k * n.y + at.y) * n.x + at.x;
                    h_next[r] = value.x;
                    h_next[cells + r] = value.y;
                    h_next[2u * cells + r] = value.z;
                }
            }
            plane_h[q] = value.x;
            plane_h[RH + q] = value.y;
            plane_h[2u * RH + q] = value.z;
        }
        workgroupBarrier();
        if (inside) {
            let q = (lid.y + 1u) * HX + lid.x + 1u;
            let p = (lid.y + 1u) * RX + lid.x + 1u;
            let vhx = plane_h[q];
            let vhy = plane_h[RH + q];
            let vhz = plane_h[2u * RH + q];
            if (one_plane) {
                before = vec2<real>(vhx, vhy);
            }
            let hx_y = plane_h[q - HX];
            let hz_y = plane_h[2u * RH + q - HX];
            let hy_x = plane_h[RH + q - 1u];
            let hz_x = plane_h[2u * RH + q - 1u];
            let fx = factor(0u, 0u, i);
            let fy = factor(0u, 1u, j);
            let fz = factor(0u, 2u, k);
            let curl_x = (vhz - hz_y) * fy - (vhy - before.y) * fz;
            let curl_y = (vhx - before.x) * fz - (vhz - hz_x) * fx;
            let curl_z = (vhy - hy_x) * fx - (vhx - hx_y) * fy;
            let r = (k * n.y + j) * n.x + i;
            let cbx = cb[r];
            let cby = cb[cells + r];
            let cbz = cb[2u * cells + r];
            var vx = keep(r, cbx) * plane_e[slot + p] + cbx * curl_x;
            var vy = keep(cells + r, cby) * plane_e[slot + RE + p] + cby * curl_y;
            var vz = keep(2u * cells + r, cbz) * plane_e[slot + 2u * RE + p] + cbz * curl_z;
            let at = vec3<u32>(i, j, k);
            let sx = in_slab(at, 0u);
            let sy = in_slab(at, 1u);
            let sz = in_slab(at, 2u);
            if (sx || sy || sz) {
                let hx = reals[1];
                let hy = reals[2];
                let hz = reals[3];
                if (sy) {
                    vx = vx + cbx * convolve_into(0u, 0u, 1u, at, (vhz - hz_y) / hy, true);
                }
                if (sz) {
                    vx = vx + cbx * -convolve_into(0u, 0u, 2u, at, (vhy - before.y) / hz, true);
                }
                if (sx) {
                    vy = vy + cby * -convolve_into(0u, 1u, 0u, at, (vhz - hz_x) / hx, true);
                }
                if (sz) {
                    vy = vy + cby * convolve_into(0u, 1u, 2u, at, (vhx - before.x) / hz, true);
                }
                if (sx) {
                    vz = vz + cbz * convolve_into(0u, 2u, 0u, at, (vhy - hy_x) / hx, true);
                }
                if (sy) {
                    vz = vz + cbz * -convolve_into(0u, 2u, 1u, at, (vhx - hx_y) / hy, true);
                }
            }
            e_next[r] = vx;
            e_next[cells + r] = vy;
            e_next[2u * cells + r] = vz;
            before = vec2<real>(vhx, vhy);
        }
    }
}

// The invocation's number in a dispatch of 64-wide workgroups laid out in x and y.
fn invocation(lid: vec3<u32>, wid: vec3<u32>, count: vec3<u32>) -> u32 {
    return (wid.y * count.x + wid.x) * 64u + lid.x;
}

// What the sources and currents add to one value, entry by entry in the order a step on the CPU
// adds them: Re(a w(t)) for each, H̃ −= Δt Re(a w), E −= cb Re(a w).
fn inject(g: u32, field: u32) {
    let place = groups[3u * g];
    let begin = groups[3u * g + 1u];
    let count = groups[3u * g + 2u];
    let dt = reals[0];
    let emitters_count = params.more.x;
    for (var q = begin; q < begin + count; q++) {
        let em = emitters[q];
        let w = 2u * (now.m * emitters_count + em);
        let amount = amplitudes[2u * q] * waves[w] - amplitudes[2u * q + 1u] * waves[w + 1u];
        if (field == 1u) {
            h[place] = h[place] - dt * amount;
        } else {
            e[place] = e[place] - cb[place] * amount;
        }
    }
}

@compute @workgroup_size(64, 1, 1)
fn inject_h(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
    @builtin(num_workgroups) count: vec3<u32>,
) {
    let g = invocation(lid, wid, count);
    if (g < params.run.y) {
        inject(g, 1u);
    }
}

@compute @workgroup_size(64, 1, 1)
fn inject_e(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
    @builtin(num_workgroups) count: vec3<u32>,
) {
    let g = invocation(lid, wid, count);
    if (g < params.run.z) {
        inject(params.run.y + g, 0u);
    }
}

// Each probe's value after the step.
@compute @workgroup_size(64, 1, 1)
fn probe_values(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
    @builtin(num_workgroups) count: vec3<u32>,
) {
    let p = invocation(lid, wid, count);
    let total = params.run.w;
    if (p < total) {
        let field = probes[2u * p];
        let at = probes[2u * p + 1u];
        var v = e[at];
        if (field == 1u) {
            v = h[at];
        }
        recorded[now.run_step * total + p] = v;
    }
}

// Each transformed value's sums, Σ F(tₙ) e^(iωtₙ) Δt, one invocation a value adding this step's
// term at each frequency. A series is 12 numbers: its field, its component, its box's start and
// length along x, y and z, where its sums start (complex values), its frequencies, where its
// phases start in a step's, and its first value's number among all the series' values.
@compute @workgroup_size(64, 1, 1)
fn transform(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
    @builtin(num_workgroups) count: vec3<u32>,
) {
    let v = invocation(lid, wid, count);
    if (v >= params.more.z) {
        return;
    }
    // the series holding value v: the last whose first value is at or before it
    var lo = 0u;
    var hi = params.more.y;
    while (hi - lo > 1u) {
        let mid = (lo + hi) / 2u;
        if (series[12u * mid + 11u] <= v) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let s = 12u * lo;
    let o = v - series[s + 11u];
    let nx = series[s + 3u];
    let ny = series[s + 5u];
    let nz = series[s + 7u];
    let i = series[s + 2u] + o % nx;
    let j = series[s + 4u] + (o / nx) % ny;
    let k = series[s + 6u] + o / (nx * ny);
    let n = params.n;
    let at = series[s + 1u] * n.w + (k * n.y + j) * n.x + i;
    var f = e[at];
    if (series[s] == 1u) {
        f = h[at];
    }
    let volume = nx * ny * nz;
    let frequencies = series[s + 9u];
    let p0 = 2u * (now.m * params.more.w + series[s + 10u]);
    for (var q = 0u; q < frequencies; q++) {
        let a = 2u * (series[s + 8u] + q * volume + o);
        sums[a] = sums[a] + f * phases[p0 + 2u * q];
        sums[a + 1u] = sums[a + 1u] + f * phases[p0 + 2u * q + 1u];
    }
}
