//! The plane a 2D solver sees: the effective index of the slab under each point of a stack's
//! chip, from the layers' materials and the shapes drawn over them, plain or etched, or for
//! averaging from the fill ([`crate::averaging`]). A job and a device both take it from here.

use num_complex::Complex64 as c64;

use super::Grid;
use crate::geometry::Point;
use crate::mode::slab::Slab;
use crate::stack::Structure;
use crate::units::{Length, Wavelength, refractive_index};
use crate::{Error, Result};

/// A 2D task's error, as a job reports it.
fn task_error(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "task".into(),
        reason: reason.into(),
    }
}

/// The effective index of the slab at `p`: the layer's material there between what lies just
/// below and above it, or the material's own index where the slab guides nothing.
fn effective_index(
    s: &Structure,
    p: Point,
    layer: (Length, Length),
    (kind, axis): (crate::mode::Polarization, usize),
    wavelength: Wavelength,
) -> Result<f64> {
    slab_index(|z| s.material_at(p, z), layer, (kind, axis), wavelength)
}

/// A material's index for the field along the chip's `axis` (0, 1 or 2: x, y or z), from its
/// tensor's diagonal entry there ([`crate::material::Material::diagonal_permittivity`]; checked
/// diagonal before): an isotropic material's own index, to the bit.
pub(crate) fn index_along(
    m: &crate::material::Material,
    axis: usize,
    wavelength: Wavelength,
) -> Result<f64> {
    Ok(refractive_index(m.tensor(wavelength)?[axis][axis]).re)
}

/// [`effective_index`] of the column whose material at each height z is `material(z)`: the
/// layer's, between what lies just below and above it, each medium's index for E along the
/// chip's `axis` ([`index_along`]): for TE, the in-plane E's.
pub(crate) fn slab_index<'a>(
    material: impl Fn(Length) -> &'a crate::material::Material,
    (bottom, top): (Length, Length),
    (kind, axis): (crate::mode::Polarization, usize),
    wavelength: Wavelength,
) -> Result<f64> {
    let n = |z: Length| -> Result<f64> { index_along(material(z), axis, wavelength) };
    let tiny = Length::nm(1e-3);
    let (below, core, above) = (n(bottom - tiny)?, n((bottom + top) * 0.5)?, n(top + tiny)?);
    if core <= below.max(above) {
        return Ok(core);
    }
    let slab = Slab::new(below, core, above, top - bottom)?;
    Ok(slab
        .modes(kind, wavelength)
        .first()
        .map_or(core, crate::mode::slab::SlabMode::effective_index))
}

/// The most layers a 2D job's stack may have when its layer is etched: its columns' indices
/// are solved for every set of the other layers' shapes over a point.
const ETCHED_LAYERS: usize = 6;

/// The effective index squared of every column of a structure whose layer `name` is etched (a
/// partial etch, or sloped walls), as a function of the plane's point: the column holds the
/// layer's material up to the height [`Structure::kept`] gives there and its background
/// above, between what lies just below and above the layer (as each other layer's shapes have
/// it), a four-layer slab ([`crate::mode::multilayer`]). Its index is solved at the heights the
/// etches stand at (the layer's top and bottom and each partial etch's floor) and, under a
/// sloped wall, at 64 heights across the layer, and taken between them as n² linear in the
/// height: exact where the walls are vertical. The other layers must be plain.
///
/// # Errors
///
/// [`Error::Parse`] for an etched layer other than `name`, or a stack of more than
/// [`ETCHED_LAYERS`] layers; [`Error::OutsideValidity`] for a material without data at
/// `wavelength`, and the multilayer's errors.
pub(crate) fn etched_plane(
    s: &Structure,
    name: &str,
    (kind, axis): (crate::mode::Polarization, usize),
    wavelength: Wavelength,
) -> Result<Box<dyn Fn(f64, f64) -> f64 + Send + Sync>> {
    let stack = s.stack().clone();
    let layers = stack.layers();
    let k = layers
        .iter()
        .position(|l| l.name == name)
        .ok_or_else(|| task_error(format!("the stack has no layer {name}")))?;
    if let Some(other) = layers
        .iter()
        .find(|l| l.name != name && !s.is_plain(&l.name))
    {
        return Err(task_error(format!(
            "a 2D job's etches are on its own layer, {name}: {} has a partial etch or a sloped \
             wall",
            other.name
        )));
    }
    if layers.len() > ETCHED_LAYERS {
        return Err(task_error(format!(
            "a 2D job with an etched layer may have at most {ETCHED_LAYERS} layers, got {}",
            layers.len()
        )));
    }
    let (_, bottom, top) = stack.layer(name).expect("found above");
    let t = (top - bottom).to_um();
    // the heights the column's index is solved at
    let mut heights = vec![0.0, t];
    for e in s.etches(name) {
        if let Some(d) = e.depth {
            heights.push(t - d.to_um());
        }
        if e.run() > 0.0 {
            heights.extend((1..64).map(|i| t * f64::from(i) / 64.0));
        }
    }
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    let n = |m: &crate::material::Material| index_along(m, axis, wavelength);
    let (material, background) = (n(&layers[k].material)?, n(&layers[k].background)?);
    // the others' bits: layer i's at bit i (the etched layer's never set)
    let others: Vec<usize> = (0..layers.len()).filter(|&i| i != k).collect();
    let mut tables = vec![Vec::new(); 1 << layers.len()];
    for combo in 0..1usize << others.len() {
        let mask = others
            .iter()
            .enumerate()
            .filter(|(b, _)| combo & (1 << b) != 0)
            .fold(0, |m, (_, &i)| m | (1 << i));
        let medium = |i: usize| {
            if mask & (1 << i) != 0 {
                &layers[i].material
            } else {
                &layers[i].background
            }
        };
        let below = n(if k == 0 {
            stack.substrate()
        } else {
            medium(k - 1)
        })?;
        let above = n(if k + 1 == layers.len() {
            stack.cladding()
        } else {
            medium(k + 1)
        })?;
        let mut table = Vec::with_capacity(heights.len());
        for &h in &heights {
            let index = column_index(
                (below, above),
                (material, background),
                (t, h),
                kind,
                wavelength,
            )?;
            table.push(index * index);
        }
        tables[mask] = table;
    }
    let drawn: Vec<(usize, Vec<crate::geometry::Shape>)> = others
        .iter()
        .map(|&i| (i, s.shapes(&layers[i].name).to_vec()))
        .collect();
    let s = s.clone();
    let name = name.to_owned();
    Ok(Box::new(move |x: f64, y: f64| {
        let p = Point::um(x, y);
        let mask = drawn
            .iter()
            .filter(|(_, shapes)| shapes.iter().any(|shape| shape.contains(p)))
            .fold(0, |m, (i, _)| m | (1 << i));
        let table = &tables[mask];
        let h = s.kept(&name, p).map_or(0.0, Length::to_um);
        // n² linear in the height between the two heights about it; exact at each
        let j = heights.partition_point(|&v| v < h);
        if j < heights.len() && heights[j] == h {
            return table[j];
        }
        let j = j.clamp(1, heights.len() - 1);
        let (h0, h1) = (heights[j - 1], heights[j]);
        let w = ((h - h0) / (h1 - h0)).clamp(0.0, 1.0);
        table[j - 1] + w * (table[j] - table[j - 1])
    }))
}

/// The effective index of a column of a layer `t` µm thick whose material stands `h` µm high
/// under its background, between `below` and `above` (indices): a three-layer slab where the
/// layer is one material all through, a four-layer one ([`crate::mode::multilayer`]) between;
/// a medium's own index where nothing guides.
fn column_index(
    (below, above): (f64, f64),
    (material, background): (f64, f64),
    (t, h): (f64, f64),
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<f64> {
    Ok(if h <= 0.0 || h >= t {
        let core = if h >= t { material } else { background };
        if core <= below.max(above) {
            core
        } else {
            Slab::new(below, core, above, Length::um(t))?
                .modes(kind, wavelength)
                .first()
                .map_or(core, crate::mode::slab::SlabMode::effective_index)
        }
    } else {
        let ml = crate::mode::multilayer::Multilayer::new(
            c64::new(above, 0.0),
            &[
                (c64::new(background, 0.0), Length::um(t - h)),
                (c64::new(material, 0.0), Length::um(h)),
            ],
            c64::new(below, 0.0),
        )?;
        ml.bound_modes(kind, wavelength)?
            .first()
            .map_or(below.max(above).max(background), |m| m.effective_index().re)
    })
}

/// The plane of an `"fdfd"` job's layer `name` for averaging from the fill
/// ([`crate::averaging`]): the effective index squared of the column outside every shape as
/// the background, and of the column inside one on each shape. That takes two columns, so the
/// other layers must have nothing drawn on them and the layer's walls must be vertical (a
/// partial etch is fine: outside the shapes the layer stands at its highest floor).
///
/// # Errors
///
/// [`Error::Parse`] for shapes on another layer or a sloped wall; [`Error::OutsideValidity`]
/// for a material without data at `wavelength`, and the slabs' errors.
pub(crate) fn fill_plane(
    s: &Structure,
    name: &str,
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<crate::averaging::Plane> {
    let stack = s.stack();
    let layers = stack.layers();
    let k = layers
        .iter()
        .position(|l| l.name == name)
        .ok_or_else(|| task_error(format!("the stack has no layer {name}")))?;
    if let Some(other) = layers
        .iter()
        .find(|l| l.name != name && !s.shapes(&l.name).is_empty())
    {
        return Err(task_error(format!(
            "averaging from the fill takes a 2D job's shapes on its own layer, {name}: {} has \
             shapes too",
            other.name
        )));
    }
    if s.etches(name).iter().any(|e| e.run() > 0.0) {
        return Err(task_error(format!(
            "averaging from the fill needs {name}'s walls vertical: a sloped wall makes the \
             column's index change across it (use averaging = \"samples\")"
        )));
    }
    // one plane for both of E's components: each medium's ε_xx, which an fdfd job with a
    // crystal whose ε_yy differs refuses to average from the fill ([`in_plane`])
    let n = |m: &crate::material::Material| index_along(m, 0, wavelength);
    let below = n(if k == 0 {
        stack.substrate()
    } else {
        &layers[k - 1].background
    })?;
    let above = n(if k + 1 == layers.len() {
        stack.cladding()
    } else {
        &layers[k + 1].background
    })?;
    let (material, background) = (n(&layers[k].material)?, n(&layers[k].background)?);
    let t = layers[k].thickness.to_um();
    // outside the shapes the layer stands at its highest partial etch's floor, if any
    let outside = s
        .etches(name)
        .iter()
        .filter_map(|e| e.depth.map(|d| t - d.to_um()))
        .fold(0.0f64, f64::max);
    let column = |h: f64| -> Result<c64> {
        let index = column_index(
            (below, above),
            (material, background),
            (t, h),
            kind,
            wavelength,
        )?;
        Ok(c64::new(index * index, 0.0))
    };
    let inside = column(t)?;
    let mut plane = crate::averaging::Plane::new(column(outside)?);
    for shape in s.shapes(name) {
        plane.draw(shape.clone(), inside);
    }
    Ok(plane)
}

/// The in-plane permittivity at `wavelength` on `grid`. A point's column is fixed by which
/// layers have a shape over it; the few distinct columns are found at the cells' centres and each
/// one's effective index solved once. An etched layer's columns are [`etched_plane`]'s.
pub(crate) fn plane(
    s: &Structure,
    grid: &Grid,
    name: &str,
    layer: (Length, Length),
    kind: crate::mode::Polarization,
    wavelength: Wavelength,
) -> Result<Box<dyn Fn(f64, f64) -> c64 + Send + Sync>> {
    plane_along(s, grid, name, layer, (kind, 0), wavelength)
}

/// [`plane`] of each medium's index for E along the chip's `axis` ([`index_along`]).
pub(crate) fn plane_along(
    s: &Structure,
    grid: &Grid,
    name: &str,
    layer: (Length, Length),
    (kind, axis): (crate::mode::Polarization, usize),
    wavelength: Wavelength,
) -> Result<Box<dyn Fn(f64, f64) -> c64 + Send + Sync>> {
    if s.stack().layers().iter().any(|l| !s.is_plain(&l.name)) {
        let eps = etched_plane(s, name, (kind, axis), wavelength)?;
        return Ok(Box::new(move |x, y| c64::new(eps(x, y), 0.0)));
    }
    Ok(Box::new(plain_plane(
        s,
        grid,
        layer,
        (kind, axis),
        wavelength,
    )?))
}

/// [`plane`] of a structure whose layer is plain: as it always was.
fn plain_plane(
    s: &Structure,
    grid: &Grid,
    layer: (Length, Length),
    (kind, axis): (crate::mode::Polarization, usize),
    wavelength: Wavelength,
) -> Result<impl Fn(f64, f64) -> c64 + Send + Sync + use<>> {
    let s = s.clone();
    let names: Vec<String> = s.stack().layers().iter().map(|l| l.name.clone()).collect();
    let key = move |s: &Structure, p: Point| -> Vec<bool> {
        names
            .iter()
            .map(|n| s.shapes(n).iter().any(|shape| shape.contains(p)))
            .collect()
    };
    let mut table: Vec<(Vec<bool>, f64)> = Vec::new();
    for j in 0..grid.ny {
        for i in 0..grid.nx {
            let p = Point::um(grid.x(i), grid.y(j));
            let k = key(&s, p);
            if !table.iter().any(|(t, _)| *t == k) {
                let n = effective_index(&s, p, layer, (kind, axis), wavelength)?;
                table.push((k, n));
            }
        }
    }
    Ok(move |x: f64, y: f64| {
        let k = key(&s, Point::um(x, y));
        // a column only a sub-cell sample meets takes the first one's index
        let n = table.iter().find(|(t, _)| *t == k).unwrap_or(&table[0]).1;
        c64::new(n * n, 0.0)
    })
}
