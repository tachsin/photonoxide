//! Arrays: many shapes as one region, their union, each element with its index; built on a
//! [`Lattice`] (the same shape at every site, or one made for each site from its index and
//! place), or as a [`Reference`]'s placements of a named [`Cell`].
//!
//! **Fast for many elements.** A bounding-volume hierarchy ([`Index`]) over the elements' boxes
//! finds the few near a point or a cell, so a lattice of 10⁴ holes costs a point or a cell a
//! handful of tests, not 10⁴. It is built in a fixed order and queried in a fixed order, so
//! answers are the same bits on any number of threads.
//!
//! **What is exact.** When no two elements' boxes meet (the usual lattice of holes or
//! pillars, [`Array::is_separate`]), everything is the elements' own: the inside test is any
//! element's, the signed distance the nearest element's (exact, as the union's boundary near a
//! point is that element's), the area, perimeter and fill their sums, the polygons theirs. When
//! boxes meet (touching or overlapping elements, a grating's teeth on a spine), the region is
//! their union as a [`super::Boolean`] union is: the inside test combines the elements' (an
//! edge two of them share is inside), the distance is the elements' least where that is exact
//! (always outside), and from the union's polygons within [`super::FILL_TOLERANCE`] of its size
//! elsewhere, as are the area, perimeter and fill.

use std::fmt;
use std::sync::Arc;

use super::clip::{self, Operation, Part};
use super::compose::{exact, fine, hull, size_of, tightened};
use super::flat::Flat;
use super::lattice::{Lattice, Site};
use super::primitives::{FILL_TOLERANCE, um};
use super::region::{Distance, Location, Polygons, Region, Tolerance};
use super::transform::Transform;
use super::{Bounds, Index, Point, Shape};
use crate::units::Length;
use crate::{Error, Result};

#[cfg(test)]
thread_local! {
    /// How many elements this thread's queries have tested: what the tests read to show a
    /// query visits only the elements near it.
    pub(crate) static TESTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts an element (or a ring of a boolean's polygons) tested, in tests only.
#[inline]
pub(super) fn tested() {
    #[cfg(test)]
    TESTED.with(|c| c.set(c.get() + 1));
}

/// An element of an array: a shape and its index, (i, j) on a lattice, (placement, shape) in a
/// reference.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Element {
    /// Its index.
    pub index: [i64; 2],
    /// Its shape, where it is drawn.
    pub shape: Shape,
}

impl Element {
    /// The element `shape` of index `index`.
    pub fn new(index: [i64; 2], shape: Shape) -> Element {
        Element { index, shape }
    }
}

/// Many shapes as one region, their union, each element with an index. Cheap to clone. See
/// the module's docs.
#[derive(Clone)]
pub struct Array {
    data: Arc<ArrayData>,
}

struct ArrayData {
    elements: Vec<Element>,
    boxes: Vec<Bounds>,
    index: Index,
    bounds: Bounds,
    /// No two elements' boxes meet.
    separate: bool,
    /// The union's polygons, where elements' boxes meet.
    flat: Option<Flat>,
    area: f64,
    /// A step along a normal far below the polygons' tolerance, µm.
    step: f64,
}

impl PartialEq for Array {
    fn eq(&self, other: &Array) -> bool {
        self.data.elements == other.data.elements
    }
}

impl fmt::Debug for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Array")
            .field("elements", &self.data.elements.len())
            .field("separate", &self.data.separate)
            .field("bounds", &self.data.bounds)
            .finish()
    }
}

/// Where a site's element goes: scaled about the site's origin, then moved to it.
fn placing(site: &Site) -> Transform {
    let to = Transform::translate(site.at.x, site.at.y);
    if site.scale == 1.0 {
        return to;
    }
    Transform::scale(site.scale, Point::default())
        .expect("a site's scale is positive")
        .then(&to)
}

impl Array {
    /// The union of `elements`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when there are none.
    pub fn new(elements: Vec<Element>) -> Result<Array> {
        if elements.is_empty() {
            return Err(Error::invalid(
                "array",
                "has no elements: every site was removed, or none was made",
            ));
        }
        let boxes: Vec<Bounds> = elements.iter().map(|e| e.shape.bounds()).collect();
        let index = Index::new(&boxes);
        let bounds = boxes[1..].iter().fold(boxes[0], |b, x| hull(&b, x));
        let separate = (0..boxes.len()).all(|k| {
            index
                .meeting(boxes[k])
                .into_iter()
                .all(|m| m == k || !boxes[k].overlaps(&boxes[m]))
        });
        let t = FILL_TOLERANCE * size_of(&bounds);
        let (flat, area) = if separate {
            (None, elements.iter().map(|e| e.shape.area()).sum())
        } else {
            let (parts, dev) = union(&elements, t, 0.0, t);
            let flat = Flat::new(parts, dev)
                .ok_or_else(|| Error::invalid("array", "its elements' union is empty"))?;
            let area = flat.area();
            (Some(flat), area)
        };
        Ok(Array {
            data: Arc::new(ArrayData {
                elements,
                boxes,
                index,
                bounds,
                separate,
                flat,
                area,
                step: 1e-3 * t,
            }),
        })
    }

    /// `element`, drawn about the origin, at every site of `lattice`: moved to the site, and
    /// scaled about it first where a defect resizes it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the lattice has no sites left.
    pub fn of(lattice: &Lattice, element: &Shape) -> Result<Array> {
        Array::new(
            lattice
                .sites()
                .iter()
                .map(|s| Element::new(s.index, element.transformed(&placing(s))))
                .collect(),
        )
    }

    /// An element made for each site of `lattice` by `make`, from the site's index and place
    /// (its position in the lattice's frame, a shift included): drawn about the origin, moved
    /// to the site and scaled about it where a defect resizes it. `make` returning `None`
    /// leaves the site empty. Each element's size or shape can so follow its index (an
    /// apodized grating) or its place (a metalens's pillars from a phase map).
    ///
    /// # Errors
    ///
    /// The first error `make` returns, and [`Error::InvalidValue`] when no element is made.
    pub fn generate(
        lattice: &Lattice,
        mut make: impl FnMut(&Site) -> Result<Option<Shape>>,
    ) -> Result<Array> {
        let mut elements = Vec::with_capacity(lattice.len());
        for s in lattice.sites() {
            if let Some(shape) = make(s)? {
                elements.push(Element::new(s.index, shape.transformed(&placing(s))));
            }
        }
        Array::new(elements)
    }

    /// The elements, in order.
    pub fn elements(&self) -> &[Element] {
        &self.data.elements
    }

    /// How many elements it has.
    pub fn len(&self) -> usize {
        self.data.elements.len()
    }

    /// Whether it has none (never: an array is made with at least one).
    pub fn is_empty(&self) -> bool {
        self.data.elements.is_empty()
    }

    /// The element of index `index`, the first if several share it.
    pub fn element(&self, index: [i64; 2]) -> Option<&Element> {
        self.data.elements.iter().find(|e| e.index == index)
    }

    /// Whether no two elements' boxes meet: then every answer is the elements' own, exactly.
    pub fn is_separate(&self) -> bool {
        self.data.separate
    }

    /// The image under `t`: every element's.
    ///
    /// # Panics
    ///
    /// Never: an array has elements, and so has its image.
    pub fn transformed(&self, t: &Transform) -> Array {
        Array::new(
            self.data
                .elements
                .iter()
                .map(|e| Element::new(e.index, e.shape.transformed(t)))
                .collect(),
        )
        .expect("a transformed array has its elements")
    }

    /// The elements whose boxes meet `cell`, in their order.
    fn near(&self, cell: Bounds) -> Vec<usize> {
        let mut near = self.data.index.meeting(cell);
        near.sort_unstable();
        near
    }

    /// The nearest element's signed distance at `p`, outside every element: the index's nodes
    /// visited nearest first ([`Index::nearest`]), an element's distance being at least its
    /// box's; of elements equally near, the first.
    fn outside(&self, p: Point) -> Distance {
        let d = &self.data;
        let nearest = d.index.nearest(p, |k| {
            tested();
            d.elements[k].shape.distance(p).signed.to_um()
        });
        let (k, _) = nearest.expect("an array has elements");
        d.elements[k].shape.distance(p)
    }
}

/// The union of `elements`' polygons within `e` µm, on a grid of `grid`, without slivers
/// thinner than `sliver`; and their deviation.
fn union(elements: &[Element], e: f64, grid: f64, sliver: f64) -> (Vec<Part>, f64) {
    let mut parts = Vec::new();
    let mut dev = 0.0f64;
    for el in elements {
        let (p, d) = fine(&el.shape, e);
        parts.extend(p);
        dev = dev.max(d);
    }
    (
        clip::boolean(&parts, &[], Operation::Union, grid, sliver),
        dev,
    )
}

impl Region for Array {
    fn locate(&self, p: Point) -> Location {
        let d = &self.data;
        let mut inside = false;
        let mut on: Vec<usize> = Vec::new();
        d.index.any_at(p, |k| {
            tested();
            match d.elements[k].shape.locate(p) {
                Location::Inside => {
                    inside = true;
                    true
                }
                Location::Boundary => {
                    on.push(k);
                    false
                }
                Location::Outside => false,
            }
        });
        if inside {
            return Location::Inside;
        }
        match on.len() {
            0 => Location::Outside,
            1 => Location::Boundary,
            _ => {
                // on two boundaries back to back (an edge two elements share): inside
                let normals: Vec<[f64; 2]> = on
                    .iter()
                    .map(|&k| d.elements[k].shape.distance(p).normal)
                    .collect();
                let back = (0..normals.len()).any(|a| {
                    (a + 1..normals.len()).any(|b| {
                        let (na, nb) = (normals[a], normals[b]);
                        na[0] * nb[0] + na[1] * nb[1] < -1.0 + 1e-9
                    })
                });
                if back {
                    Location::Inside
                } else {
                    Location::Boundary
                }
            }
        }
    }

    fn distance(&self, p: Point) -> Distance {
        let data = &self.data;
        // the elements holding p: the deepest one's distance
        let mut deepest: Option<Distance> = None;
        data.index.any_at(p, |k| {
            tested();
            let shape = &data.elements[k].shape;
            if shape.locate(p) != Location::Outside {
                let x = shape.distance(p);
                if deepest.is_none_or(|b| x.signed < b.signed) {
                    deepest = Some(x);
                }
            }
            false
        });
        let Some(inner) = deepest else {
            return self.outside(p);
        };
        let Some(flat) = &data.flat else {
            // the element's own boundary is the union's near p
            return inner;
        };
        let (dd, n) = (inner.signed.to_um(), inner.normal);
        let near = || self.locate(p) == Location::Boundary;
        if exact(self, um(p), dd, n, data.step, near) {
            return inner;
        }
        let (dd, n) = flat.distance(p, dd);
        Distance {
            signed: Length::um(dd),
            normal: n,
        }
    }

    fn bounds(&self) -> Bounds {
        self.data.bounds
    }

    /// Each element's polygons when no boxes meet; their union, measured against the distance
    /// and tightened as a boolean's, when they do.
    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let data = &self.data;
        if data.separate {
            let mut contours = Vec::new();
            let mut deviation = Length::ZERO;
            for e in &data.elements {
                let p = e.shape.polygons(tolerance);
                contours.extend(p.contours);
                if p.deviation > deviation {
                    deviation = p.deviation;
                }
            }
            return Polygons {
                contours,
                tolerance,
                deviation,
            };
        }
        let (grid, sliver) = (tolerance.grid().to_um(), tolerance.geometric().to_um());
        let floor = data.flat.as_ref().map_or(0.0, |f| f.deviation) + data.step;
        tightened(self, tolerance, floor, |e| {
            union(&data.elements, e, grid, sliver)
        })
    }

    /// The elements' areas summed when no boxes meet; their union's polygons' within
    /// [`super::FILL_TOLERANCE`] of its size when they do.
    fn area(&self) -> f64 {
        self.data.area
    }

    /// As [`Array::area`]: the elements' perimeters summed, or the union's polygons'.
    fn perimeter(&self) -> Option<Length> {
        let data = &self.data;
        match &data.flat {
            None => data
                .elements
                .iter()
                .map(|e| e.shape.perimeter())
                .try_fold(Length::ZERO, |s, p| p.map(|p| s + p)),
            Some(flat) => Some(Length::um(flat.perimeter())),
        }
    }

    /// The fills of the elements whose boxes meet the cell, summed in the elements' order, when
    /// no boxes meet; the union's polygons clipped to the cell when they do.
    fn fill(&self, cell: Bounds) -> f64 {
        let data = &self.data;
        if !data.bounds.overlaps(&cell) {
            return 0.0;
        }
        if cell.encloses(&data.bounds) {
            return data.area;
        }
        if let Some(flat) = &data.flat {
            return flat.fill(cell);
        }
        let mut sum = 0.0;
        for k in self.near(cell) {
            if data.boxes[k].overlaps(&cell) {
                tested();
                sum += data.elements[k].shape.fill(cell);
            }
        }
        sum
    }
}

// ---------------------------------------------------------------------------------------------
// References

/// A named structure: shapes drawn together, placed as a whole by a [`Reference`]. 0.9's
/// layout's cells are these. Cheap to clone.
#[derive(Clone, PartialEq)]
pub struct Cell {
    data: Arc<(String, Vec<Shape>)>,
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell")
            .field("name", &self.data.0)
            .field("shapes", &self.data.1)
            .finish()
    }
}

impl Cell {
    /// The cell `name` of `shapes`, in their own frame.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty name or no shapes.
    pub fn new(name: &str, shapes: Vec<Shape>) -> Result<Cell> {
        if name.trim().is_empty() {
            return Err(Error::invalid("cell", "needs a name"));
        }
        if shapes.is_empty() {
            return Err(Error::invalid("cell", format!("\"{name}\" has no shapes")));
        }
        Ok(Cell {
            data: Arc::new((name.to_owned(), shapes)),
        })
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.data.0
    }

    /// Its shapes, in its own frame.
    pub fn shapes(&self) -> &[Shape] {
        &self.data.1
    }
}

/// A [`Cell`] placed many times, each placement a [`Transform`] (translations, rotations,
/// mirrors, scalings): the union of every shape of the cell under every placement, an
/// [`Array`] whose element (k, m) is the cell's shape m under placement k, exactly that
/// shape's [`Shape::transformed`]. Cheap to clone.
#[derive(Clone, Debug)]
pub struct Reference {
    cell: Cell,
    placements: Arc<Vec<Transform>>,
    array: Array,
}

impl PartialEq for Reference {
    fn eq(&self, other: &Reference) -> bool {
        self.cell == other.cell && self.placements == other.placements
    }
}

impl Reference {
    /// `cell` at each of `placements`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with no placements.
    pub fn new(cell: Cell, placements: Vec<Transform>) -> Result<Reference> {
        if placements.is_empty() {
            return Err(Error::invalid(
                "reference",
                format!("places the cell \"{}\" nowhere", cell.name()),
            ));
        }
        let mut elements = Vec::with_capacity(placements.len() * cell.shapes().len());
        for (k, t) in placements.iter().enumerate() {
            for (m, s) in cell.shapes().iter().enumerate() {
                elements.push(Element::new([k as i64, m as i64], s.transformed(t)));
            }
        }
        Ok(Reference {
            array: Array::new(elements)?,
            cell,
            placements: Arc::new(placements),
        })
    }

    /// The cell placed.
    pub fn cell(&self) -> &Cell {
        &self.cell
    }

    /// Its placements.
    pub fn placements(&self) -> &[Transform] {
        &self.placements
    }

    /// The placed shapes, as an array.
    pub fn array(&self) -> &Array {
        &self.array
    }

    /// The image under `t`: every placement followed by `t`.
    ///
    /// # Panics
    ///
    /// Never: a reference has placements, and so has its image.
    pub fn transformed(&self, t: &Transform) -> Reference {
        Reference::new(
            self.cell.clone(),
            self.placements.iter().map(|p| p.then(t)).collect(),
        )
        .expect("a transformed reference has its placements")
    }
}

impl Region for Reference {
    fn locate(&self, p: Point) -> Location {
        self.array.locate(p)
    }

    fn distance(&self, p: Point) -> Distance {
        self.array.distance(p)
    }

    fn bounds(&self) -> Bounds {
        self.array.bounds()
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        self.array.polygons(tolerance)
    }

    fn area(&self) -> f64 {
        self.array.area()
    }

    fn perimeter(&self) -> Option<Length> {
        self.array.perimeter()
    }

    fn fill(&self, cell: Bounds) -> f64 {
        self.array.fill(cell)
    }
}
