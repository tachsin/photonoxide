//! The rules (#544) of the geometry kernel's cases, by the case's id. Most are exact: a closed
//! form against arithmetic that is exact but for its round-off, arcs and areas integrated in
//! closed form rather than flattened. The rest measure our own discretisation: the polygons'
//! distance from the boundary at the default tolerance (1 nm, by construction), the distances
//! against a polygon within 1e-3 nm, the regions built from polygons within 1e-7 of their size,
//! and the bodies' fills with each crossed cell halved to 8 levels; and the orders those errors
//! fall at. The published cases are the papers' printed numbers, half a unit of the last digit,
//! or drawings that must match the paper's parameters exactly.

use super::{Basis::*, Measure::*, Numerical, Numerical::*, Rule};

/// The Fresnel integrals' error (geometry/paths/fresnel), the Euler bends' only numerical one.
const FRESNEL: Numerical = Stated(
    1.1e-16,
    "the Fresnel integrals, geometry/paths/fresnel; the root found to round-off",
);

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- the primitives ----
        (
            "geometry/areas",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "closed forms against Green's theorem on the exact boundary: round-off",
            ),
        ),
        (
            "geometry/polygons",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the default tolerance's bound: arcs within 1 − 1/√2 nm, snapping within 1/√2 nm",
            ),
        ),
        (
            "geometry/fill",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "each cell's area in closed form by clipping, summed: round-off",
            ),
        ),
        (
            "geometry/transforms",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a similarity maps each primitive to one of its kind exactly: round-off, µm",
            ),
        ),
        // ---- paths ----
        (
            "geometry/paths/fresnel",
            Rule::new(
                Absolute,
                RoundOff,
                1e-15,
                Exact,
                "series and continued fraction to double precision against the tables' 17 digits",
            ),
        ),
        (
            "geometry/paths/euler-ends",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the clothoids placed by the Fresnel integrals end on the circular bend's: round-off",
            ),
        ),
        (
            "geometry/paths/euler-curvature",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the curvature in closed form along the construction: round-off, 1/µm",
            ),
        ),
        (
            "geometry/paths/perimeter",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the outline's lengths by Gauss–Legendre between knots against closed forms: round-off",
            ),
        ),
        (
            "geometry/paths/distance",
            Rule::new(
                Absolute,
                Discretisation,
                2e-3,
                Measured,
                "against a polygon within 1e-3 nm of the outline, the reference's own error: nm",
            ),
        ),
        (
            "geometry/paths/polygons",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the default tolerance's bound, as every shape's: nm",
            ),
        ),
        (
            "geometry/paths/fill",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "exact along straights and arcs, the curves' polygons keeping the area: round-off",
            ),
        ),
        (
            "geometry/paths/convergence",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "a chord's sagitta κh²/8: second order, shown to two decimals over whole halvings",
            ),
        ),
        (
            "geometry/paths/cherchi-l-bend",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                FRESNEL,
                "Cherchi et al.'s 1.87: half a unit of its last digit",
            ),
        ),
        (
            "geometry/paths/cherchi-u-bend",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                FRESNEL,
                "Cherchi et al.'s 1.38: half a unit of its last digit",
            ),
        ),
        (
            "geometry/paths/vogelbacher-singular",
            Rule::new(
                Absolute,
                Reference,
                5e-2,
                FRESNEL,
                "Vogelbacher et al.'s 263.3°: half a unit of its last digit",
            ),
        ),
        (
            "geometry/paths/vogelbacher-270",
            Rule::new(
                Absolute,
                Reference,
                5e-4,
                FRESNEL,
                "Vogelbacher et al.'s p = 0.585: half a unit of its last digit",
            ),
        ),
        (
            "geometry/paths/vogelbacher-fujisawa",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                FRESNEL,
                "Vogelbacher et al.'s p = 0.52: half a unit of its last digit",
            ),
        ),
        (
            "geometry/paths/fujisawa-ratios",
            Rule::new(
                Absolute,
                Reference,
                0.5,
                FRESNEL,
                "Fujisawa et al.'s whole percentages: half a unit of their last digit",
            ),
        ),
        // ---- curves ----
        (
            "geometry/curves/circle",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the primitive's closed forms against the curves' quadrature and root finding: round-off",
            ),
        ),
        (
            "geometry/curves/closed-forms",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the areas' integrals in closed form: round-off",
            ),
        ),
        (
            "geometry/curves/taper-as-path",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the region between two lines is the path's straight of linear width: round-off",
            ),
        ),
        (
            "geometry/curves/distance",
            Rule::new(
                Absolute,
                Discretisation,
                2e-3,
                Measured,
                "against a polygon within 1e-3 nm of the boundary, the reference's own error: nm",
            ),
        ),
        (
            "geometry/curves/polygons",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the default tolerance's bound, as every shape's: nm",
            ),
        ),
        (
            "geometry/curves/fill",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the summed fill keeps each loop's area exactly: round-off (each cell's, by its own bound)",
            ),
        ),
        (
            "geometry/curves/convergence",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "a chord's sagitta κh²/8: second order, shown to two decimals",
            ),
        ),
        (
            "geometry/curves/implicit-estimate",
            Rule::new(
                Absolute,
                Order,
                5e-2,
                Measured,
                "the estimate's error d²/(2(R + d)) in closed form: second order",
            ),
        ),
        (
            "geometry/curves/mask",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "each pixel's value times the area it shares with the cell, clipped exactly: round-off",
            ),
        ),
        (
            "geometry/curves/zhang-y-junction",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the spline passes through Zhang et al.'s widths exactly: round-off",
            ),
        ),
        // ---- booleans ----
        (
            "geometry/booleans/areas",
            Rule::new(
                Relative,
                Discretisation,
                1e-9,
                Measured,
                "the regions' polygons within 1e-7 of their size, each arc's area kept: in error where the circles cross, measured 1.2e-10",
            ),
        ),
        (
            "geometry/booleans/polygons",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the operands within half the budget, overlaid on the 1 nm grid, tightened until within 1 nm",
            ),
        ),
        (
            "geometry/booleans/degenerate",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the exact results of degenerate pairs on an integer grid: round-off, µm²",
            ),
        ),
        (
            "geometry/booleans/distance",
            Rule::new(
                Absolute,
                Discretisation,
                1e-3,
                Measured,
                "off the boundary, the distance to polygons within 1e-7 of the size (2.9e-4 nm): nm",
            ),
        ),
        (
            "geometry/booleans/offsets",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "Steiner's formula against regions from polygons within 1e-7 of their size, measured 4.0e-8",
            ),
        ),
        (
            "geometry/booleans/fillets",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "the corner cuts against regions from polygons within 1e-7 of their size, measured 2.3e-9",
            ),
        ),
        (
            "geometry/booleans/convergence",
            Rule::new(
                Absolute,
                Order,
                0.15,
                Measured,
                "a chord's sagitta κh²/8, which the operations move no farther: second order, the least of three",
            ),
        ),
        (
            "geometry/booleans/zhang-y-junction",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the union's shared edges meet exactly: round-off, µm²",
            ),
        ),
        // ---- arrays ----
        (
            "geometry/arrays/lattices",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "each site in closed form: round-off, over the pitch",
            ),
        ),
        (
            "geometry/arrays/fill",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "each hole's exact area and fill, found through the index: round-off",
            ),
        ),
        (
            "geometry/arrays/index-expressions",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the expressions' values at each site: round-off, µm",
            ),
        ),
        (
            "geometry/arrays/defects",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "a count of elements wrong: none",
            ),
        ),
        (
            "geometry/arrays/references",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "a count of differences in any bit: none",
            ),
        ),
        (
            "geometry/arrays/slab-less-holes",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "inclusion–exclusion on areas each polygon keeps exactly: round-off, measured 9.8e-15",
            ),
        ),
        (
            "geometry/arrays/convergence",
            Rule::new(
                Absolute,
                Order,
                0.15,
                Measured,
                "a chord's sagitta κh²/8: second order, shown to two decimals",
            ),
        ),
        (
            "geometry/arrays/akahane-l3",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the cavity drawn to Akahane et al.'s parameters exactly: round-off",
            ),
        ),
        // ---- bodies ----
        (
            "geometry/bodies/volumes",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "each slice's area exact by clipping, integrated between closed-form roots: round-off",
            ),
        ),
        (
            "geometry/bodies/box",
            Rule::new(
                Absolute,
                RoundOff,
                1e-18,
                Exact,
                "the product of the overlaps along each axis: round-off, µm³",
            ),
        ),
        (
            "geometry/bodies/extrusions",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "each cell's area exact by clipping, times the height: round-off",
            ),
        ),
        (
            "geometry/bodies/sloped-walls",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "the corners' cones from each crossed cell halved to 8 levels, measured 3.7e-8",
            ),
        ),
        (
            "geometry/bodies/distances",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "closed forms, the ellipsoid's nearest point found to round-off: µm",
            ),
        ),
        (
            "geometry/bodies/booleans",
            Rule::new(
                Relative,
                Discretisation,
                1e-6,
                Measured,
                "each cell both surfaces cross halved to 8 levels, its last parts from the tangent planes, measured 1.3e-7",
            ),
        ),
        (
            "geometry/bodies/oblique",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "the cells a cap crosses halved to 8 levels, measured 1.5e-8",
            ),
        ),
        (
            "geometry/bodies/tangent-order",
            Rule::new(
                Absolute,
                Order,
                0.25,
                Measured,
                "a surface strays from its tangent plane by κρ²/2: second order, from two grids alone",
            ),
        ),
        (
            "geometry/bodies/fdtd-bits",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same arithmetic on the same bodies: no bit differs",
            ),
        ),
        (
            "geometry/bodies/akahane-l3",
            Rule::new(
                Relative,
                Discretisation,
                1e-7,
                Measured,
                "Akahane et al.'s cavity as a body: its fills over a/4 cells, the crossed ones halved, measured 7.8e-9",
            ),
        ),
    ]
}
