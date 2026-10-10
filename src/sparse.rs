//! photonoxide's sparse direct solver: a multifrontal LU on the structure of A + Aᵀ with static
//! pivoting ([`Multifrontal`]), in an order of photonoxide's choosing: nested dissection on the
//! grids its matrices come from (A. George, "Nested dissection of a regular finite element mesh",
//! SIAM J. Numer. Anal. 10, 345 (1973), doi:10.1137/0710032).
//!
//! George numbers each independent part of the grid before the set that separates it from its
//! neighbours, recursively (his Section 4): in 2D the factorization then takes O(n³) operations
//! and O(n² log n) storage on an n × n grid, against O(n⁴) and O(n³) numbered row by row, and the
//! same holds for an unsymmetric matrix with a symmetric structure (his closing remarks).
//!
//! Here the parts are cut geometrically, at the median of the longest axis of the unknowns'
//! grid positions, and the separator is taken from the matrix's own graph: the smallest set of
//! unknowns, on either side, that touches every edge across the cut. That separates for any
//! stencil (the 2D solver's 5 points, the mode solvers' wider ones, Yee's edges, a Bloch side's
//! wrap-around), where George's sets are drawn for one.

mod matching;
mod multifrontal;

pub(crate) use multifrontal::{Analysis, Multifrontal, faer_lu};

/// Below this many unknowns a part is numbered as it is, not dissected further.
const LEAF: usize = 64;

/// The structure of A + Aᵀ without its diagonal, by rows: each unknown's neighbours.
pub(crate) fn adjacency(n: usize, entries: &[(usize, usize)]) -> (Vec<usize>, Vec<usize>) {
    let mut lists: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(r, c) in entries {
        if r != c {
            lists[r].push(c);
            lists[c].push(r);
        }
    }
    let mut starts = Vec::with_capacity(n + 1);
    let mut neighbours = Vec::new();
    starts.push(0);
    for mut list in lists {
        list.sort_unstable();
        list.dedup();
        neighbours.extend(list);
        starts.push(neighbours.len());
    }
    (starts, neighbours)
}

/// The order to eliminate the unknowns in: each part before the separator that cuts it from the
/// rest, recursively. `positions[u]` is unknown u's place on its grid (any units; only their
/// order along each axis matters); `(starts, neighbours)` the graph of [`adjacency`]. Each
/// separator is one step wide in that graph, for a factorization whose structure is A + Aᵀ's
/// ([`Multifrontal`]); faer's LU, whose rows pivot anywhere and whose structure is AᵀA's, would
/// need two. Fixed by the grid and the structure, not by the values or the machine.
pub(crate) fn nested_dissection(
    starts: &[usize],
    neighbours: &[usize],
    positions: &[[f64; 3]],
) -> Vec<usize> {
    let n = positions.len();
    let mut order = Vec::with_capacity(n);
    // 0: elsewhere, 1: this cut's low side, 2: its high side, 3: the separator
    let mut side = vec![0u8; n];
    // a separator is numbered after both halves it cuts apart, so the work list holds it, to
    // emit once the halves pushed after it are done
    enum Work {
        Part(Vec<usize>),
        Emit(Vec<usize>),
    }
    let mut work = vec![Work::Part((0..n).collect())];
    while let Some(item) = work.pop() {
        let mut part = match item {
            Work::Emit(separator) => {
                order.extend(separator);
                continue;
            }
            Work::Part(part) => part,
        };
        if part.len() <= LEAF {
            part.sort_unstable();
            order.extend(part);
            continue;
        }
        // the longest axis of the part's box
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for &u in &part {
            for a in 0..3 {
                low[a] = low[a].min(positions[u][a]);
                high[a] = high[a].max(positions[u][a]);
            }
        }
        let axis = (0..3)
            .max_by(|&a, &b| (high[a] - low[a]).total_cmp(&(high[b] - low[b])))
            .unwrap_or(0);
        if high[axis] <= low[axis] {
            part.sort_unstable();
            order.extend(part);
            continue;
        }
        // cut at the median position along it; ties go to the high side, so neither is empty
        let middle = part.len() / 2;
        part.select_nth_unstable_by(middle, |&u, &v| {
            positions[u][axis]
                .total_cmp(&positions[v][axis])
                .then(u.cmp(&v))
        });
        let cut = positions[part[middle]][axis];
        let (mut lower, mut upper): (Vec<usize>, Vec<usize>) =
            part.iter().partition(|&&u| positions[u][axis] < cut);
        if lower.is_empty() {
            // the median is the lowest position: cut just above it instead
            (lower, upper) = part.iter().partition(|&&u| positions[u][axis] <= cut);
            if upper.is_empty() {
                part.sort_unstable();
                order.extend(part);
                continue;
            }
        }
        for &u in &lower {
            side[u] = 1;
        }
        for &u in &upper {
            side[u] = 2;
        }
        // the separator: the smallest set that touches every edge across the cut, a minimum
        // vertex cover of the bipartite graph between the two sides' boundaries (König's
        // theorem), from either side. On the strips of docs/baselines.md it took the factors
        // from 1.3 times PARDISO's entries (the low side's boundary alone) to 0.9 times.
        let touches = |u: usize, side: &[u8], of: u8| {
            neighbours[starts[u]..starts[u + 1]]
                .iter()
                .any(|&v| side[v] == of)
        };
        let below: Vec<usize> = lower
            .iter()
            .copied()
            .filter(|&u| touches(u, &side, 2))
            .collect();
        let above: Vec<usize> = upper
            .iter()
            .copied()
            .filter(|&u| touches(u, &side, 1))
            .collect();
        for &u in &minimum_cover(&below, &above, starts, neighbours) {
            side[u] = 3;
        }
        let (mut separator, mut rest): (Vec<usize>, Vec<usize>) =
            lower.iter().partition(|&&u| side[u] == 3);
        let (above, mut upper): (Vec<usize>, Vec<usize>) =
            upper.iter().partition(|&&u| side[u] == 3);
        separator.extend(above);
        for &u in lower.iter().chain(&upper).chain(&separator) {
            side[u] = 0;
        }
        separator.sort_unstable();
        rest.sort_unstable();
        upper.sort_unstable();
        work.push(Work::Emit(separator));
        work.push(Work::Part(upper));
        work.push(Work::Part(rest));
    }
    order
}

/// A minimum vertex cover of the bipartite graph between `left` and `right` (the edges of the
/// graph `(starts, neighbours)` that join them): a maximum matching by Hopcroft and Karp's
/// phases of shortest augmenting paths, then König's construction, the left vertices not
/// reachable from an unmatched left vertex by alternating paths and the right ones that are.
/// Deterministic: everything is visited in the lists' order.
fn minimum_cover(
    left: &[usize],
    right: &[usize],
    starts: &[usize],
    neighbours: &[usize],
) -> Vec<usize> {
    const NONE: usize = usize::MAX;
    let index: std::collections::HashMap<usize, usize> =
        right.iter().enumerate().map(|(j, &v)| (v, j)).collect();
    let edges: Vec<Vec<usize>> = left
        .iter()
        .map(|&u| {
            neighbours[starts[u]..starts[u + 1]]
                .iter()
                .filter_map(|v| index.get(v).copied())
                .collect()
        })
        .collect();
    let mut match_left = vec![NONE; left.len()];
    let mut match_right = vec![NONE; right.len()];
    let mut level = vec![NONE; left.len()];
    loop {
        // breadth first from the free left vertices, by alternating levels
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for (i, l) in level.iter_mut().enumerate() {
            *l = if match_left[i] == NONE {
                queue.push_back(i);
                0
            } else {
                NONE
            };
        }
        let mut found = false;
        while let Some(i) = queue.pop_front() {
            for &j in &edges[i] {
                let k = match_right[j];
                if k == NONE {
                    found = true;
                } else if level[k] == NONE {
                    level[k] = level[i] + 1;
                    queue.push_back(k);
                }
            }
        }
        if !found {
            break;
        }
        // depth first along the levels from each free left vertex, without recursion
        let mut next = vec![0usize; left.len()];
        for root in 0..left.len() {
            if match_left[root] != NONE {
                continue;
            }
            let mut path = vec![root];
            while let Some(&i) = path.last() {
                if next[i] == edges[i].len() {
                    level[i] = NONE;
                    path.pop();
                    continue;
                }
                let j = edges[i][next[i]];
                next[i] += 1;
                let k = match_right[j];
                if k == NONE {
                    // augment along the path back to the root
                    let mut j = j;
                    for &i in path.iter().rev() {
                        let previous = match_left[i];
                        match_left[i] = j;
                        match_right[j] = i;
                        j = previous;
                    }
                    break;
                } else if level[k] == level[i] + 1 {
                    path.push(k);
                }
            }
        }
    }
    // König: alternating reachability from the free left vertices
    let mut reached_left = vec![false; left.len()];
    let mut reached_right = vec![false; right.len()];
    let mut stack: Vec<usize> = (0..left.len()).filter(|&i| match_left[i] == NONE).collect();
    for &i in &stack {
        reached_left[i] = true;
    }
    while let Some(i) = stack.pop() {
        for &j in &edges[i] {
            if !reached_right[j] && match_left[i] != j {
                reached_right[j] = true;
                let k = match_right[j];
                if k != NONE && !reached_left[k] {
                    reached_left[k] = true;
                    stack.push(k);
                }
            }
        }
    }
    let mut cover: Vec<usize> = (0..left.len())
        .filter(|&i| !reached_left[i])
        .map(|i| left[i])
        .collect();
    cover.extend(
        (0..right.len())
            .filter(|&j| reached_right[j])
            .map(|j| right[j]),
    );
    cover
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 5-point stencil's graph on an nx × ny grid, and the cells' positions.
    fn grid(nx: usize, ny: usize) -> (Vec<usize>, Vec<usize>, Vec<[f64; 3]>) {
        let mut entries = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                if i + 1 < nx {
                    entries.push((k, k + 1));
                }
                if j + 1 < ny {
                    entries.push((k, k + nx));
                }
            }
        }
        let (starts, neighbours) = adjacency(nx * ny, &entries);
        let positions = (0..nx * ny)
            .map(|k| [(k % nx) as f64, (k / nx) as f64, 0.0])
            .collect();
        (starts, neighbours, positions)
    }

    #[test]
    fn the_order_is_a_permutation_with_each_separator_after_its_parts() {
        let (nx, ny) = (37, 23);
        let (starts, neighbours, positions) = grid(nx, ny);
        let order = nested_dissection(&starts, &neighbours, &positions);
        let mut seen = order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..nx * ny).collect::<Vec<_>>());
        // the last unknowns numbered are the first cut, across the longest axis (x): one whole
        // column of the grid, the smallest set that separates the 5-point stencil's halves
        let last: Vec<usize> = order[order.len() - ny..].to_vec();
        let columns: std::collections::BTreeSet<usize> = last.iter().map(|k| k % nx).collect();
        assert_eq!(columns.len(), 1, "{columns:?}");
        // and no edge of the graph joins the two halves it cuts apart
        let column = *columns.iter().next().unwrap();
        for u in 0..nx * ny {
            for &v in &neighbours[starts[u]..starts[u + 1]] {
                let (a, b) = (u % nx, v % nx);
                assert!(!(a < column && b > column || a > column && b < column));
            }
        }
    }

    #[test]
    fn the_cover_is_minimum_on_both_sides() {
        // a path a-b-c-d split as {a, b} | {c, d}: the edge b-c is covered by one vertex; a star
        // whose centre is on the high side and leaves on the low side is covered by the centre
        let (starts, neighbours) = adjacency(4, &[(0, 1), (1, 2), (2, 3)]);
        assert_eq!(minimum_cover(&[1], &[2], &starts, &neighbours).len(), 1);
        let (starts, neighbours) = adjacency(5, &[(0, 4), (1, 4), (2, 4), (3, 4)]);
        assert_eq!(
            minimum_cover(&[0, 1, 2, 3], &[4], &starts, &neighbours),
            vec![4]
        );
    }
}
