//! Masks: a GDSII file's cells and layers ([`gds_library`]), and what a cell draws on a layer
//! as polygons with holes ([`gds_polygons`]), as a job's `"gds"` shape reads them
//! ([`crate::geometry::Gds`]).

use std::path::Path;

use crate::geometry::{Bounds, Gds, Point};
use crate::{Error, Result};

/// A GDSII file's library: its name, its database unit and its cells.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GdsLibrary {
    /// The library's name (LIBNAME).
    pub library: String,
    /// The database unit, µm: every coordinate in the file is a whole number of it.
    pub database_unit_um: f64,
    /// The cells, in the file's order.
    pub cells: Vec<GdsCell>,
    /// The cells no other cell places, in the file's order: the one a job reads by default
    /// when there is exactly one.
    pub top_cells: Vec<String>,
}

/// A cell of a GDSII file, and the layers it draws on.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GdsCell {
    /// Its name (STRNAME).
    pub name: String,
    /// The `[layer, datatype]` pairs it draws on, the cells it places included, in order.
    pub layers: Vec<[u16; 2]>,
}

/// A polygon with holes, seen from above, in a file's coordinates.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MaskPolygon {
    /// Its outer boundary, (x, y) in µm, counterclockwise, the first vertex not repeated.
    pub outer: Vec<(f64, f64)>,
    /// Its holes, each clockwise.
    pub holes: Vec<Vec<(f64, f64)>>,
}

/// The library, cells and layers of the GDSII file at `path`.
///
/// # Errors
///
/// [`Error::Io`] for a file that can't be read, [`Error::Parse`] for one that isn't a GDSII
/// stream (naming the byte), and [`Error::InvalidValue`] for what [`Gds::parse`] refuses.
pub fn gds_library(path: &Path) -> Result<GdsLibrary> {
    let gds = Gds::read(path)?;
    let cells = gds
        .cells()
        .into_iter()
        .map(|name| {
            Ok(GdsCell {
                name: name.to_owned(),
                layers: gds.layers(name)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(GdsLibrary {
        library: gds.library().to_owned(),
        database_unit_um: gds.database_unit().to_um(),
        cells,
        top_cells: gds.top_cells().into_iter().map(str::to_owned).collect(),
    })
}

/// What `cell` (the file's one top cell when `None`) of the GDSII file at `path` draws on
/// `gds_layer` (`[layer, datatype]`), every cell it places flattened into it, as polygons with
/// holes in µm: the union of everything drawn there, keyholes as holes, within the window
/// `x_um` × `y_um` when both are given. These are the polygons a job's `"gds"` shape draws
/// before its `origin` and `angle`, exactly as the booleans give them
/// ([`crate::geometry::Gds::contours`]): no vertex is moved.
///
/// # Errors
///
/// As [`gds_library`], and [`Error::InvalidValue`] for a cell the file doesn't have, no cell
/// named when the file has no top cell or several, a window with only one of its ranges or a
/// range that isn't finite and increasing, or nothing drawn on the layer (or inside the
/// window), naming the layers the cell does draw on.
pub fn gds_polygons(
    path: &Path,
    gds_layer: [u16; 2],
    cell: Option<&str>,
    x_um: Option<(f64, f64)>,
    y_um: Option<(f64, f64)>,
) -> Result<Vec<MaskPolygon>> {
    let gds = Gds::read(path)?;
    let cell = match cell {
        Some(c) => c,
        None => gds.top_cell()?,
    };
    let window = match (x_um, y_um) {
        (None, None) => None,
        (Some(x), Some(y)) => {
            let fine = |(a, b): (f64, f64)| a.is_finite() && b.is_finite() && a < b;
            if !(fine(x) && fine(y)) {
                return Err(Error::invalid(
                    "GDSII window",
                    format!("its ranges must be finite and increasing, not x {x:?}, y {y:?}"),
                ));
            }
            Some(Bounds::new(Point::um(x.0, y.0), Point::um(x.1, y.1)))
        }
        _ => {
            return Err(Error::invalid(
                "GDSII window",
                "takes both x_um and y_um, its ranges in the file's coordinates",
            ));
        }
    };
    let xy = |p: &Point| (p.x.to_um(), p.y.to_um());
    Ok(gds
        .contours(cell, gds_layer, window)?
        .iter()
        .map(|c| MaskPolygon {
            outer: c.outer.iter().map(xy).collect(),
            holes: c.holes.iter().map(|h| h.iter().map(xy).collect()).collect(),
        })
        .collect())
}
