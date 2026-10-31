//! A polygon with holes cut into triangles, for the caps of the Fabrication page's solids: each
//! hole joined to the polygon by a bridge to a vertex it sees, then ears clipped. Every
//! triangle's corners are the rings' own vertices, so the caps meet the walls edge for edge.

/// A point of the plane, µm.
pub type P2 = [f64; 2];

fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// Twice the signed area of a ring: positive counterclockwise.
pub fn twice_area(ring: &[P2]) -> f64 {
    (0..ring.len())
        .map(|i| cross(ring[i], ring[(i + 1) % ring.len()]))
        .sum()
}

/// Whether `a` and `b` are the same point.
fn same(a: P2, b: P2) -> bool {
    a[0] == b[0] && a[1] == b[1]
}

/// Whether the segments `p`–`q` and `a`–`b` cross at a point inside both (touching at an end
/// doesn't count).
fn crosses(p: P2, q: P2, a: P2, b: P2) -> bool {
    let d1 = cross(sub(q, p), sub(a, p));
    let d2 = cross(sub(q, p), sub(b, p));
    let d3 = cross(sub(b, a), sub(p, a));
    let d4 = cross(sub(b, a), sub(q, a));
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// The angle from `b` to `v` counterclockwise, in [0, 2π).
fn turn(b: P2, v: P2) -> f64 {
    let a = cross(b, v).atan2(dot(b, v));
    if a < 0.0 {
        a + std::f64::consts::TAU
    } else {
        a
    }
}

/// Whether the direction `d` from the vertex `v` of a counterclockwise walk, between `prev` and
/// `next`, points into the polygon: it lies in the wedge counterclockwise from `next` to `prev`.
fn inward(prev: P2, v: P2, next: P2, d: P2) -> bool {
    let (a, b) = (sub(prev, v), sub(next, v));
    let (ta, td) = (turn(b, a), turn(b, d));
    td > 0.0 && td < ta
}

/// The triangles of the polygon `outer` (counterclockwise) with the holes `holes` (each
/// clockwise), as indices into their vertices taken in turn (`outer`'s, then each hole's), each
/// triangle counterclockwise.
pub fn triangulate(outer: &[P2], holes: &[Vec<P2>]) -> Vec<[u32; 3]> {
    let mut pts: Vec<P2> = outer.to_vec();
    let mut starts = Vec::with_capacity(holes.len());
    for h in holes {
        starts.push(pts.len());
        pts.extend_from_slice(h);
    }
    let mut walk: Vec<u32> = (0..outer.len() as u32).collect();
    // the holes from the leftmost: the first thing a hole sees to its left is then the walk
    let mut order: Vec<usize> = (0..holes.len()).filter(|&k| holes[k].len() >= 3).collect();
    let leftmost = |k: usize| {
        (0..holes[k].len())
            .min_by(|&i, &j| {
                let (a, b) = (holes[k][i], holes[k][j]);
                a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
            })
            .unwrap_or(0)
    };
    order.sort_by(|&a, &b| holes[a][leftmost(a)][0].total_cmp(&holes[b][leftmost(b)][0]));
    for (n, &k) in order.iter().enumerate() {
        let m_local = leftmost(k);
        let mp = pts[starts[k] + m_local];
        // the edges a bridge may not cross: the walk's, this hole's and the holes still to come
        let mut edges: Vec<(P2, P2)> = (0..walk.len())
            .map(|i| {
                (
                    pts[walk[i] as usize],
                    pts[walk[(i + 1) % walk.len()] as usize],
                )
            })
            .collect();
        for &j in &order[n..] {
            let h = &holes[j];
            edges.extend((0..h.len()).map(|i| (h[i], h[(i + 1) % h.len()])));
        }
        // the walk's vertices, nearest first; the first one seen from the hole's leftmost vertex
        let mut near: Vec<usize> = (0..walk.len()).collect();
        let dist = |i: usize| {
            let d = sub(pts[walk[i] as usize], mp);
            dot(d, d)
        };
        near.sort_by(|&a, &b| dist(a).total_cmp(&dist(b)));
        let seen = near.iter().copied().find(|&i| {
            let p = pts[walk[i] as usize];
            let n = walk.len();
            let (prev, next) = (
                pts[walk[(i + n - 1) % n] as usize],
                pts[walk[(i + 1) % n] as usize],
            );
            !same(p, mp)
                && inward(prev, p, next, sub(mp, p))
                && !edges.iter().any(|&(a, b)| {
                    !same(a, p)
                        && !same(b, p)
                        && !same(a, mp)
                        && !same(b, mp)
                        && crosses(p, mp, a, b)
                })
        });
        let Some(i) = seen.or(near.first().copied()) else {
            continue;
        };
        let h = holes[k].len();
        let mut spliced: Vec<u32> = Vec::with_capacity(walk.len() + h + 2);
        spliced.extend_from_slice(&walk[..=i]);
        spliced.extend((0..=h).map(|s| (starts[k] + (m_local + s) % h) as u32));
        spliced.push(walk[i]);
        spliced.extend_from_slice(&walk[i + 1..]);
        walk = spliced;
    }
    clip_ears(&pts, walk)
}

/// The ears of the counterclockwise walk `walk` over `pts`, clipped one by one.
fn clip_ears(pts: &[P2], mut walk: Vec<u32>) -> Vec<[u32; 3]> {
    let mut out = Vec::with_capacity(walk.len().saturating_sub(2));
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for &i in &walk {
        let p = pts[i as usize];
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    let size = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(f64::MIN_POSITIVE);
    // an ear thinner than this is no ear yet: it is taken only when nothing else is left
    let thin = size * size * 1e-15;
    let mut start = 0;
    while walk.len() > 3 {
        let n = walk.len();
        let at = |i: usize| pts[walk[i % n] as usize];
        let ear = |i: usize, strict: bool| -> bool {
            let (a, b, c) = (at(i + n - 1), at(i), at(i + 1));
            let area = cross(sub(b, a), sub(c, b));
            if strict && area <= thin || !strict && area < -thin {
                return false;
            }
            // nothing of the walk inside the triangle or on it, but its own corners
            (0..n).all(|j| {
                if j == (i + n - 1) % n || j == i % n || j == (i + 1) % n {
                    return true;
                }
                let p = at(j);
                if same(p, a) || same(p, b) || same(p, c) {
                    return true;
                }
                let (u, v, w) = (
                    cross(sub(b, a), sub(p, a)),
                    cross(sub(c, b), sub(p, b)),
                    cross(sub(a, c), sub(p, c)),
                );
                !(u >= 0.0 && v >= 0.0 && w >= 0.0)
            })
        };
        let found = (0..n)
            .map(|k| (start + k) % n)
            .find(|&i| ear(i, true))
            .or_else(|| (0..n).find(|&i| ear(i, false)))
            // a walk no ear clips cleanly (a ring that crosses itself): the flattest corner
            .unwrap_or_else(|| {
                (0..n)
                    .min_by(|&i, &j| {
                        let area = |i: usize| {
                            cross(sub(at(i), at(i + n - 1)), sub(at(i + 1), at(i))).abs()
                        };
                        area(i).total_cmp(&area(j))
                    })
                    .unwrap_or(0)
            });
        out.push([
            walk[(found + n - 1) % n],
            walk[found],
            walk[(found + 1) % n],
        ]);
        walk.remove(found);
        start = if found == 0 { 0 } else { found - 1 };
    }
    if walk.len() == 3 {
        out.push([walk[0], walk[1], walk[2]]);
    }
    out
}
