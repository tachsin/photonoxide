//! A bounding-volume hierarchy over shapes' boxes, so that a point or a cell is tested only
//! against the shapes near it.
//!
//! It is built in a fixed order (the boxes split at the median of their centres along the
//! longer side, ties broken by the shapes' order), and queries visit the nodes in a fixed
//! order, so its answers are the same bits on any number of threads.

use super::{Bounds, Point, Shape};

/// The most boxes a leaf holds.
const LEAF: usize = 4;

/// A node: its box (min x, min y, max x, max y, µm) and either its two children, the nodes
/// `first` and `first + 1`, or (a leaf, `count` > 0) the items `first..first + count`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Node {
    lo: [f64; 2],
    hi: [f64; 2],
    first: u32,
    count: u32,
}

/// A bounding-volume hierarchy over a list of boxes, answering which boxes hold a point or meet
/// a cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Index {
    nodes: Vec<Node>,
    items: Vec<u32>,
}

/// A box widened by a hair, so that a point a shape counts as inside after rounding is never
/// outside its box: 10⁻⁹ of its size and distance from the origin.
fn widened(b: &Bounds) -> ([f64; 2], [f64; 2]) {
    let (lo, hi) = (
        [b.min.x.to_um(), b.min.y.to_um()],
        [b.max.x.to_um(), b.max.y.to_um()],
    );
    let scale = lo.iter().chain(&hi).fold(1.0f64, |m, v| m.max(v.abs()));
    let m = 1e-9 * scale;
    ([lo[0] - m, lo[1] - m], [hi[0] + m, hi[1] + m])
}

impl Index {
    /// The hierarchy over `shapes`' bounds, in their order.
    pub fn of(shapes: &[Shape]) -> Index {
        let bounds: Vec<Bounds> = shapes.iter().map(Shape::bounds).collect();
        Index::new(&bounds)
    }

    /// The hierarchy over `boxes`, item k being `boxes[k]`.
    ///
    /// # Panics
    ///
    /// With more than 2³² − 1 boxes.
    pub fn new(boxes: &[Bounds]) -> Index {
        let mut entries: Vec<(u32, [f64; 2], [f64; 2])> = boxes
            .iter()
            .enumerate()
            .map(|(k, b)| {
                let (lo, hi) = widened(b);
                (u32::try_from(k).expect("fewer than 2^32 shapes"), lo, hi)
            })
            .collect();
        let mut index = Index {
            nodes: Vec::with_capacity(2 * boxes.len() / LEAF + 1),
            items: Vec::with_capacity(boxes.len()),
        };
        index.nodes.push(Node {
            lo: [0.0; 2],
            hi: [0.0; 2],
            first: 0,
            count: 0,
        });
        index.build(0, &mut entries);
        index
    }

    fn build(&mut self, node: usize, entries: &mut [(u32, [f64; 2], [f64; 2])]) {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        let (mut clo, mut chi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for (_, l, h) in entries.iter() {
            for k in 0..2 {
                lo[k] = lo[k].min(l[k]);
                hi[k] = hi[k].max(h[k]);
                let c = 0.5 * (l[k] + h[k]);
                clo[k] = clo[k].min(c);
                chi[k] = chi[k].max(c);
            }
        }
        if entries.len() <= LEAF {
            let first = self.items.len() as u32;
            self.items.extend(entries.iter().map(|e| e.0));
            self.nodes[node] = Node {
                lo,
                hi,
                first,
                count: entries.len() as u32,
            };
            return;
        }
        let axis = usize::from(chi[1] - clo[1] > chi[0] - clo[0]);
        let mid = entries.len() / 2;
        let centre = |e: &(u32, [f64; 2], [f64; 2])| 0.5 * (e.1[axis] + e.2[axis]);
        entries.select_nth_unstable_by(mid, |a, b| {
            centre(a).total_cmp(&centre(b)).then(a.0.cmp(&b.0))
        });
        let first = self.nodes.len();
        let blank = Node {
            lo: [0.0; 2],
            hi: [0.0; 2],
            first: 0,
            count: 0,
        };
        self.nodes.push(blank);
        self.nodes.push(blank);
        self.nodes[node] = Node {
            lo,
            hi,
            first: first as u32,
            count: 0,
        };
        let (left, right) = entries.split_at_mut(mid);
        self.build(first, left);
        self.build(first + 1, right);
    }

    /// How many boxes it holds.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Visits the items whose boxes meet the box from `lo` to `hi` (µm), in a fixed order,
    /// until `visit` returns true; whether one did.
    fn visit(&self, lo: [f64; 2], hi: [f64; 2], mut visit: impl FnMut(usize) -> bool) -> bool {
        if self.items.is_empty() {
            return false;
        }
        // a balanced tree over fewer than 2³² items is at most 32 levels deep
        let mut stack = [0u32; 64];
        let mut top = 1;
        while top > 0 {
            top -= 1;
            let node = self.nodes[stack[top] as usize];
            if node.lo[0] > hi[0] || node.hi[0] < lo[0] || node.lo[1] > hi[1] || node.hi[1] < lo[1]
            {
                continue;
            }
            if node.count > 0 {
                let items = &self.items[node.first as usize..(node.first + node.count) as usize];
                for &k in items {
                    if visit(k as usize) {
                        return true;
                    }
                }
            } else {
                // the right child below the left, so the left is visited first
                stack[top] = node.first + 1;
                stack[top + 1] = node.first;
                top += 2;
            }
        }
        false
    }

    /// Whether `test` holds for an item whose box holds `p`, testing them in a fixed order and
    /// stopping at the first that passes.
    pub fn any_at(&self, p: Point, test: impl FnMut(usize) -> bool) -> bool {
        let q = [p.x.to_um(), p.y.to_um()];
        self.visit(q, q, test)
    }

    /// The items whose boxes meet `cell`, in a fixed order.
    pub fn meeting(&self, cell: Bounds) -> Vec<usize> {
        let mut out = Vec::new();
        self.visit(
            [cell.min.x.to_um(), cell.min.y.to_um()],
            [cell.max.x.to_um(), cell.max.y.to_um()],
            |k| {
                out.push(k);
                false
            },
        );
        out
    }
}
