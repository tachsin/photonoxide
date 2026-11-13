// Shin and Fan's operator on Yee's grid without its matrix, the product photonoxide's
// `backend::Stencil::apply` takes, for the multigrid's finest level on the GPU (#337): A v =
// k0²(1 + iβ) ε v − ∇ × ∇ × v + ∇(ε⁻¹ ∇ · (ε v)), in three passes: H = ∇ × E at the faces, the
// potential φ = ε⁻¹ ∇ · (ε E) at the nodes, and the product. `real` is f64 (or f32, measured):
// the host puts its alias first (`alias real = f64;`) before compiling.
//
// One invocation per cell, all three components; every value is written by one invocation and
// every sum is taken by it in the order the CPU takes it: no atomics, no reductions across
// invocations, so a product repeats bit for bit on the same device and driver. Nothing divides
// (in f64 a division gave one of two results from run to run on an RTX 4060, in FDTD's kernels):
// the steps come as 1/(s h), and the phases and their inverses from the host.

struct Params {
    // nx, ny, nz, and the cells
    n: vec4<u32>,
    // 1 where an axis ends in walls (x, y, z), 0 where it is Bloch-periodic
    walls: vec4<u32>,
    // where 1/(s h) at the nodes starts in `consts`, for x, y and z
    nodes: vec4<u32>,
    // and halfway after them
    halves: vec4<u32>,
}

@group(0) @binding(0) var<uniform> params: Params;
// the input and the product, E's three components one after the other, k, j, i with i fastest
@group(0) @binding(1) var<storage, read> v: array<vec2<real>>;
@group(0) @binding(2) var<storage, read_write> out: array<vec2<real>>;
// H between the passes, and φ at the nodes (indexed as E's x component)
@group(0) @binding(3) var<storage, read_write> h: array<vec2<real>>;
@group(0) @binding(4) var<storage, read_write> phi: array<vec2<real>>;
// ε at each value of E, ε⁻¹ at each node (zero on a wall)
@group(0) @binding(5) var<storage, read> eps: array<vec2<real>>;
@group(0) @binding(6) var<storage, read> inverse: array<vec2<real>>;
// k0²(1 + iβ); the phases across each axis' high end (x, y, z), their inverses; then 1/(s h)
// at the nodes and halfway after them, along x, y and z, where `params` says
@group(0) @binding(7) var<storage, read> consts: array<vec2<real>>;

fn cmul(a: vec2<real>, b: vec2<real>) -> vec2<real> {
    return vec2<real>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

fn index(c: u32, p: vec3<u32>) -> u32 {
    return c * params.n.w + (p.z * params.n.y + p.y) * params.n.x + p.x;
}

// whether E's component c at p is held at zero: tangential to a wall at an axis' low end
fn fixed(c: u32, p: vec3<u32>) -> bool {
    return (c != 0u && params.walls.x == 1u && p.x == 0u)
        || (c != 1u && params.walls.y == 1u && p.y == 0u)
        || (c != 2u && params.walls.z == 1u && p.z == 0u);
}

// E masked: zero where a wall holds it
fn e(c: u32, p: vec3<u32>) -> vec2<real> {
    if (fixed(c, p)) {
        return vec2<real>(0.0, 0.0);
    }
    return v[index(c, p)];
}

// one step along axis a from p, forward or back: the point, its phase, and whether there is one
// (none beyond a wall)
struct Step {
    at: vec3<u32>,
    phase: vec2<real>,
    some: bool,
}

fn step(p: vec3<u32>, a: u32, forward: bool) -> Step {
    var q = p;
    let n = params.n[a];
    if (forward) {
        if (p[a] + 1u < n) {
            q[a] = p[a] + 1u;
            return Step(q, vec2<real>(1.0, 0.0), true);
        }
        if (params.walls[a] == 1u) {
            return Step(q, vec2<real>(0.0, 0.0), false);
        }
        q[a] = 0u;
        return Step(q, consts[1u + a], true);
    }
    if (p[a] > 0u) {
        q[a] = p[a] - 1u;
        return Step(q, vec2<real>(1.0, 0.0), true);
    }
    if (params.walls[a] == 1u) {
        return Step(q, vec2<real>(0.0, 0.0), false);
    }
    q[a] = n - 1u;
    return Step(q, consts[4u + a], true);
}

// E's component c at a step, times its phase
fn e_at(c: u32, s: Step) -> vec2<real> {
    if (!s.some) {
        return vec2<real>(0.0, 0.0);
    }
    return cmul(e(c, s.at), s.phase);
}

// ε E's component c at a step, times its phase
fn flux_at(c: u32, s: Step) -> vec2<real> {
    if (!s.some) {
        return vec2<real>(0.0, 0.0);
    }
    let r = index(c, s.at);
    return cmul(cmul(e(c, s.at), eps[r]), s.phase);
}

fn h_at(c: u32, s: Step) -> vec2<real> {
    if (!s.some) {
        return vec2<real>(0.0, 0.0);
    }
    return cmul(h[index(c, s.at)], s.phase);
}

fn phi_at(s: Step) -> vec2<real> {
    if (!s.some) {
        return vec2<real>(0.0, 0.0);
    }
    return cmul(phi[index(0u, s.at)], s.phase);
}

// 1/(s h) at the nodes, and halfway after them, along axis a at m
fn at_node(a: u32, m: u32) -> vec2<real> {
    return consts[params.nodes[a] + m];
}

fn at_half(a: u32, m: u32) -> vec2<real> {
    return consts[params.halves[a] + m];
}

// H = ∇ × E at the faces, each difference forward
@compute @workgroup_size(64, 1, 1)
fn curl_e(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= params.n.x) {
        return;
    }
    let p = g;
    let wx = at_half(0u, p.x);
    let wy = at_half(1u, p.y);
    let wz = at_half(2u, p.z);
    let x = step(p, 0u, true);
    let y = step(p, 1u, true);
    let z = step(p, 2u, true);
    h[index(0u, p)] = cmul(e_at(2u, y) - e(2u, p), wy) - cmul(e_at(1u, z) - e(1u, p), wz);
    h[index(1u, p)] = cmul(e_at(0u, z) - e(0u, p), wz) - cmul(e_at(2u, x) - e(2u, p), wx);
    h[index(2u, p)] = cmul(e_at(1u, x) - e(1u, p), wx) - cmul(e_at(0u, y) - e(0u, p), wy);
}

// φ = ε⁻¹ ∇ · (ε E) at the nodes, each difference backward
@compute @workgroup_size(64, 1, 1)
fn potential(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= params.n.x) {
        return;
    }
    let p = g;
    let here = vec3<u32>(index(0u, p), index(1u, p), index(2u, p));
    let dx = cmul(cmul(e(0u, p), eps[here.x]) - flux_at(0u, step(p, 0u, false)), at_node(0u, p.x));
    let dy = cmul(cmul(e(1u, p), eps[here.y]) - flux_at(1u, step(p, 1u, false)), at_node(1u, p.y));
    let dz = cmul(cmul(e(2u, p), eps[here.z]) - flux_at(2u, step(p, 2u, false)), at_node(2u, p.z));
    phi[here.x] = cmul(inverse[here.x], dx + dy + dz);
}

// k0² ε E − ∇ × H + ∇φ, and the input itself where a wall holds E
@compute @workgroup_size(64, 1, 1)
fn product(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= params.n.x) {
        return;
    }
    let p = g;
    let wx = at_node(0u, p.x);
    let wy = at_node(1u, p.y);
    let wz = at_node(2u, p.z);
    let bx = step(p, 0u, false);
    let by = step(p, 1u, false);
    let bz = step(p, 2u, false);
    let hx = h[index(0u, p)];
    let hy = h[index(1u, p)];
    let hz = h[index(2u, p)];
    var curl: array<vec2<real>, 3>;
    curl[0] = cmul(hz - h_at(2u, by), wy) - cmul(hy - h_at(1u, bz), wz);
    curl[1] = cmul(hx - h_at(0u, bz), wz) - cmul(hz - h_at(2u, bx), wx);
    curl[2] = cmul(hy - h_at(1u, bx), wx) - cmul(hx - h_at(0u, by), wy);
    let node = phi[index(0u, p)];
    var gradient: array<vec2<real>, 3>;
    gradient[0] = cmul(phi_at(step(p, 0u, true)) - node, at_half(0u, p.x));
    gradient[1] = cmul(phi_at(step(p, 1u, true)) - node, at_half(1u, p.y));
    gradient[2] = cmul(phi_at(step(p, 2u, true)) - node, at_half(2u, p.z));
    let k2 = consts[0];
    for (var c = 0u; c < 3u; c++) {
        let r = index(c, p);
        if (fixed(c, p)) {
            out[r] = v[r];
        } else {
            out[r] = cmul(cmul(k2, eps[r]), v[r]) - curl[c] + gradient[c];
        }
    }
}
