//! The rules (#544) of the units', materials', stacks', averaging's and the geometry
//! generator's cases: most of these cases are in [`super::cases`] itself, the rest beside their
//! modules, so their rules are kept together here, by the case's id.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

/// Rounding in a printed table: half a unit of its last digit, unless the case says more.
pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        // ---- units ----
        (
            "units/amplitude-convention",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "the kernel recovers a real signal's amplitude exactly over whole periods: round-off",
            ),
        ),
        (
            "units/lossy-attenuation",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "exp(-2πκ) in closed form: round-off",
            ),
        ),
        // ---- tables, splines and the silicon and silica models ----
        (
            "material/spline-line",
            Rule::new(
                Absolute,
                RoundOff,
                1e-13,
                Exact,
                "a natural spline through a line is the line: round-off",
            ),
        ),
        (
            "material/silicon-li-table",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the spline passes through its knots, Li's own table: round-off",
            ),
        ),
        (
            "material/silicon-frey-table",
            Rule::new(
                Absolute,
                Reference,
                1e-5,
                Exact,
                "Table 2 printed to five decimals: a unit of the last, for the print and Table 5's printed coefficients",
            ),
        ),
        (
            "material/silicon-frey-li",
            Rule::new(
                Absolute,
                Reference,
                2.0867e-3,
                Exact,
                "the two papers' stated uncertainties summed, Frey et al.'s and Li's",
            ),
        ),
        (
            "material/silicon-frey-smooth-d",
            Rule::new(
                Absolute,
                RoundOff,
                0.5,
                Exact,
                "a count: a smooth Sellmeier sum reverses its slope nowhere, so none, within half a reversal",
            ),
        ),
        (
            "material/silica-malitson-formula",
            Rule::new(
                Absolute,
                Reference,
                1e-6,
                Exact,
                "his computed index printed to six decimals: a unit of the last",
            ),
        ),
        (
            "material/silica-malitson-measured",
            Rule::new(
                Absolute,
                Reference,
                1e-4,
                Exact,
                "his measured indices to five decimals: the computed index and his residual, a unit of the fourth",
            ),
        ),
        (
            "material/silica-leviton-table",
            Rule::new(
                Absolute,
                Reference,
                6e-6,
                Exact,
                "Table 4 printed to five decimals: 6e-6 for the print and Table 3's printed coefficients",
            ),
        ),
        (
            "material/silica-leviton-dndt",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Exact,
                "measured dn/dT to three figures, in 1e-6 per K, which moves a strip's dn_eff/dT by about 1e-7 per K",
            ),
        ),
        (
            "material/silicon-li-table-1",
            Rule::new(
                Absolute,
                Reference,
                3e-4,
                Exact,
                "three units of the fourth decimal, which Li prints for smoothness",
            ),
        ),
        (
            "material/silicon-li-table-2",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "five units of the third decimal of Table 2, in 1e-4 per K, its printed rounding and Li's smoothing",
            ),
        ),
        (
            "material/silicon-li-dndt-order",
            Rule::new(
                Absolute,
                Order,
                0.05,
                Measured,
                "a central difference's second order, within 0.05",
            ),
        ),
        (
            "material/silicon-frey-dndt",
            Rule::new(
                Absolute,
                Reference,
                0.1,
                Exact,
                "the formula's residual in n (1e-4) over Table 4's 10 K steps, in 1e-4 per K",
            ),
        ),
        (
            "material/silicon-li-frey-dndt",
            Rule::new(
                Absolute,
                Reference,
                0.15,
                Exact,
                "Li's stated ±0.15e-4 per K, in 1e-4 per K",
            ),
        ),
        (
            "material/silicon-komma-reading",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "a figure read off its vector drawing, its marks to 1 % of their values",
            ),
        ),
        (
            "material/silicon-komma-frey-dndt",
            Rule::new(
                Absolute,
                Reference,
                2.57,
                Exact,
                "the half-width of Komma et al.'s last error bar at 35 K, in 1e-6 per K",
            ),
        ),
        (
            "material/silicon-komma-room",
            Rule::new(
                Absolute,
                Reference,
                0.15,
                Exact,
                "Li's stated ±0.15e-4 per K, in 1e-4 per K",
            ),
        ),
        (
            "material/silicon-dndt-positive",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "a count of temperatures where the default turns negative: none",
            ),
        ),
        (
            "material/silicon-nitride-elshaari-reading",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "a figure read off its vector drawing, its marks to 1 % of their values",
            ),
        ),
        (
            "material/silicon-nitride-elshaari-between",
            Rule::new(
                Relative,
                Reference,
                0.08 / 2.51,
                Exact,
                "Elshaari et al.'s ±0.08e-5 of 2.51e-5 at room temperature, as a fraction",
            ),
        ),
        (
            "material/silicon-nitride-room",
            Rule::new(
                Absolute,
                Reference,
                0.17,
                Exact,
                "the two papers' stated uncertainties summed, ±0.09e-5 and ±0.08e-5, in 1e-5 per K",
            ),
        ),
        (
            "material/silicon-nitride-arbabi-table-2",
            Rule::new(
                Absolute,
                Reference,
                1e-2,
                Exact,
                "Table 2's second decimal, in 1e-5 per K: a unit of it",
            ),
        ),
        // ---- conductivities ----
        (
            "material/conductivity-tables",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "a power law between a table's rows meets the rows between them to 1 %, under the tables' ±3 to ±10 %",
            ),
        ),
        (
            "material/silicon-kremer-ho",
            Rule::new(
                Absolute,
                Reference,
                1e-2,
                Exact,
                "Kremer et al.'s about 3 %, to ±1 %, as a fraction",
            ),
        ),
        (
            "material/silicon-glassbrenner-table",
            Rule::new(
                Relative,
                Reference,
                5e-2,
                Exact,
                "Glassbrenner and Slack's stated ±5 %",
            ),
        ),
        (
            "material/silicon-glassbrenner-ho",
            Rule::new(
                Relative,
                Reference,
                0.1,
                Exact,
                "the two accuracies summed, ±5 % each",
            ),
        ),
        (
            "material/silicon-film-ju-goodson",
            Rule::new(
                Relative,
                Reference,
                0.12,
                Exact,
                "their films read off the plot, under their ±15 % error bars",
            ),
        ),
        (
            "material/silicon-film-220",
            Rule::new(
                Relative,
                Reference,
                0.05,
                Exact,
                "5 %, Jacques et al.'s 90 W/(m·K) being two digits of Asheghi et al.'s measured layers",
            ),
        ),
        (
            "material/silicon-film-sondheimer-limits",
            Rule::new(
                Relative,
                Model,
                1e-5,
                Exact,
                "the limits' next terms at δ = 100 and 1e-3: below 1e-5 of the closed form",
            ),
        ),
        (
            "material/silicon-film-sondheimer-order",
            Rule::new(
                Absolute,
                Order,
                0.05,
                Measured,
                "the midpoint rule's second order, within 0.05",
            ),
        ),
        // ---- phase-change materials ----
        (
            "material/pcm-delaney-fits",
            Rule::new(
                Absolute,
                Reference,
                1e-4,
                Exact,
                "our fits to their ellipsometry, within 1e-4 of every point (8.2e-5 at most), at the four wavelengths",
            ),
        ),
        (
            "material/pcm-delaney-fits-gst",
            Rule::new(
                Absolute,
                Reference,
                3e-3,
                Exact,
                "our fits to their ellipsometry of GST, within 3e-3 of every point (2.5e-3 at most)",
            ),
        ),
        (
            "material/pcm-delaney-contrast",
            Rule::new(
                Absolute,
                Reference,
                1.5e-2,
                Exact,
                "the abstract's two digits, half a unit and the data set's own 0.0065 against Table 1",
            ),
        ),
        (
            "material/pcm-delaney-table-1",
            Rule::new(
                Absolute,
                Reference,
                5e-4,
                Exact,
                "Table 1's three decimals: half a unit of the third",
            ),
        ),
        (
            "material/pcm-delaney-losses",
            Rule::new(
                Relative,
                Reference,
                5e-2,
                Exact,
                "the paper's one and two digits, 5 %",
            ),
        ),
        (
            "material/pcm-delaney-fom",
            Rule::new(
                Relative,
                Reference,
                5e-2,
                Exact,
                "the paper's figures of merit to three digits and its Δn_eff to two or three, 5 %",
            ),
        ),
        (
            "material/pcm-delaney-lossless",
            Rule::new(
                Absolute,
                Reference,
                1e-5,
                Exact,
                "the abstract's lossless amorphous films: k under 1e-5 at 1550 nm",
            ),
        ),
        (
            "material/pcm-sb2se3-crystallization",
            Rule::new(
                Absolute,
                Reference,
                25.0,
                Exact,
                "the anneals' step, 25 K, in K",
            ),
        ),
        (
            "material/pcm-sb2s3-crystallization",
            Rule::new(
                Absolute,
                Reference,
                10.0,
                Exact,
                "the anneals' step, 10 K, in K",
            ),
        ),
        (
            "material/pcm-gst-kalb-fusion",
            Rule::new(
                Absolute,
                Reference,
                0.1,
                Exact,
                "Table II's printed digit, in J/(mol·K)",
            ),
        ),
        (
            "material/pcm-gst-zhao-melting",
            Rule::new(
                Absolute,
                Reference,
                15.0,
                Exact,
                "Zhao et al.'s stated ±15 °C, in K",
            ),
        ),
        (
            "material/pcm-gst-njoroge-change",
            Rule::new(
                Absolute,
                Reference,
                0.2,
                Exact,
                "the abstract's ±0.2 %, in percent",
            ),
        ),
        (
            "material/pcm-gst-njoroge-xrd",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "two decimals: half a unit of the second, in g/cm³",
            ),
        ),
        (
            "material/pcm-sb2s3-gurevich-low",
            Rule::new(
                Absolute,
                Reference,
                3e-3,
                Exact,
                "their equations against their tables, largest at 10 K: 1.516 against 1.52, the table's last digit",
            ),
        ),
        (
            "material/pcm-sb2s3-gurevich-high",
            Rule::new(
                Absolute,
                Reference,
                5e-2,
                Exact,
                "Table 3's one decimal: half a unit of it, in J/(mol·K)",
            ),
        ),
        (
            "material/pcm-sb2s3-gurevich-density",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "the cell's density against Aryana et al.'s Table 1 (from Popescu's book), to its three digits and the cell's own",
            ),
        ),
        (
            "material/pcm-slab-closed-form",
            Rule::new(
                Absolute,
                Discretisation,
                1e-4,
                Measured,
                "the slab's second-order error on the case's grid, its order material/pcm-slab-order's",
            ),
        ),
        (
            "material/pcm-slab-order",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "second order with the faces on nodes, within 0.2",
            ),
        ),
        // ---- lithium niobate and tantalate, barium titanate ----
        (
            "material/linbo3-zelmon-633",
            Rule::new(
                Absolute,
                Reference,
                2e-4,
                Exact,
                "Zelmon's fit is within 2e-4 of its data",
            ),
        ),
        (
            "material/linbo3-zelmon-opo",
            Rule::new(
                Absolute,
                Reference,
                3e-2,
                Exact,
                "the predicted point read off the plot to 0.03 µm",
            ),
        ),
        (
            "material/linbo3-jundt-opo",
            Rule::new(
                Absolute,
                Reference,
                2e-2,
                Exact,
                "the 250 °C fit read off the plot to 0.02 µm",
            ),
        ),
        (
            "material/linbo3-shoji-miller",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "Shoji et al.'s Miller deltas to three digits, and their own indices against Zelmon's",
            ),
        ),
        (
            "material/linbo3-mgo-gayer-zelmon",
            Rule::new(
                Absolute,
                Reference,
                5.1e-4,
                Exact,
                "Gayer et al.'s stated 3.1e-4, with Zelmon's own 2e-4 added",
            ),
        ),
        (
            "material/litao3-bond-fit",
            Rule::new(
                Absolute,
                Reference,
                7e-4,
                Exact,
                "our fit to Bond's Table IV, four decimals: within 7e-4 of every row",
            ),
        ),
        (
            "material/litao3-casson",
            Rule::new(
                Absolute,
                Reference,
                1e-3,
                Exact,
                "an independent print of the indices, to three decimals",
            ),
        ),
        (
            "material/litao3-abedin-bond",
            Rule::new(
                Absolute,
                RoundOff,
                1e-7,
                Exact,
                "their standard deviations recomputed from Bond's rows: the same arithmetic, to their printed digits",
            ),
        ),
        (
            "material/litao3-abedin-refit",
            Rule::new(
                Relative,
                Reference,
                2e-4,
                Exact,
                "Table I's printed digits, five for each constant",
            ),
        ),
        (
            "material/litao3-abedin-points",
            Rule::new(
                Absolute,
                Reference,
                5e-5,
                Exact,
                "the figure's points read off its bitmap, against their standard deviations, within half a pixel",
            ),
        ),
        (
            "material/litao3-abedin-curves",
            Rule::new(
                Absolute,
                Reference,
                2e-4,
                Exact,
                "the figure's dashes read off its bitmap, 1.2e-4 to 1.5e-4 per pixel",
            ),
        ),
        (
            "material/batio3-abel-films",
            Rule::new(
                Absolute,
                Reference,
                3e-2,
                Exact,
                "the films' spread across the three papers",
            ),
        ),
        // ---- the III-Vs and aluminium nitride ----
        (
            "material/gaas-gehrsitz-gap",
            Rule::new(
                Absolute,
                Reference,
                2e-6,
                Exact,
                "Table II printed to six decimals: a unit and a half of the last, in µm⁻²",
            ),
        ),
        (
            "material/gaas-gehrsitz-n-inf",
            Rule::new(
                Absolute,
                Reference,
                1e-4,
                Exact,
                "the temperature forms are fits to Table II's columns, within a unit of its fourth decimal",
            ),
        ),
        (
            "material/algaas-gehrsitz-samples",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Exact,
                "Tables III and IV's quality of fit, in the papers' own units",
            ),
        ),
        (
            "material/algaas-papatryfonos",
            Rule::new(
                Relative,
                Reference,
                1e-2,
                Exact,
                "the paper's differences from the reference models, of the order of 1 %",
            ),
        ),
        (
            "material/algaas-afromowitz-eta",
            Rule::new(
                Absolute,
                Reference,
                5e-5,
                Exact,
                "his printed 0.1032: half a unit of its fourth decimal",
            ),
        ),
        (
            "material/gaas-skauli-shg",
            Rule::new(
                Relative,
                Reference,
                3e-3,
                Exact,
                "a difference of indices to 0.2 % of itself, and the period's printed 0.08 %",
            ),
        ),
        (
            "material/gaas-skauli-dndt",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "their printed 2.33: half a unit of its second decimal, in 1e-4 per K",
            ),
        ),
        (
            "material/ingap-tanaka-ueno",
            Rule::new(
                Absolute,
                Reference,
                5e-3,
                Exact,
                "the printed 3.12: half a unit of its second decimal",
            ),
        ),
        (
            "material/inp-pettit-turner-suzuki",
            Rule::new(
                Absolute,
                Reference,
                7e-3,
                Exact,
                "Pettit and Turner's fit is within 0.007 of their data",
            ),
        ),
        (
            "material/inp-suzuki-tada-voltages",
            Rule::new(
                Relative,
                Reference,
                1.5e-2,
                Exact,
                "three digits of the indices and coefficients",
            ),
        ),
        (
            "material/inp-suzuki-tada-faust-henry",
            Rule::new(
                Relative,
                Reference,
                1.5e-2,
                Exact,
                "the printed values' two digits",
            ),
        ),
        (
            "material/aln-majkic-d33",
            Rule::new(
                Absolute,
                Reference,
                5e-2,
                Exact,
                "the printed 4.3 pm/V: half a unit of its decimal",
            ),
        ),
        (
            "material/aln-rigler-se",
            Rule::new(
                Absolute,
                Reference,
                1e-3,
                Exact,
                "Table I's three decimals with its A and B rounded",
            ),
        ),
        (
            "material/aln-rigler-maie",
            Rule::new(
                Absolute,
                Reference,
                1e-2,
                Exact,
                "Table I's MAIE column, each to 0.01",
            ),
        ),
        (
            "material/ingap-ferrini-table-consistency",
            Rule::new(
                Absolute,
                Reference,
                1e-2,
                Exact,
                "n and k to three decimals, which allows about 0.006 in their product",
            ),
        ),
        (
            "material/ingap-ferrini-table-knots",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a natural spline passes through its knots: round-off",
            ),
        ),
        // ---- crystals in the solvers ----
        (
            "material/crystal-turned-tensor",
            Rule::new(
                Relative,
                RoundOff,
                1e-13,
                Exact,
                "a turned tensor keeps its eigenvalues: round-off",
            ),
        ),
        (
            "material/crystal-x-cut-slab-te",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "the 2.5 nm mesh's second-order error against the exact slab, tested at 10 and 5 nm",
            ),
        ),
        (
            "material/crystal-x-cut-slab-tm",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "the 2.5 nm mesh's second-order error, as the TE case's",
            ),
        ),
        (
            "material/crystal-z-cut-slab-te",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "the 2.5 nm mesh's second-order error, as the x-cut's",
            ),
        ),
        (
            "material/crystal-z-cut-slab-tm",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "the 2.5 nm mesh's second-order error against the anisotropic slab's closed form",
            ),
        ),
        // ---- the geometry generator ----
        (
            "generator/ports",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the components' ports where their definitions put them: round-off (1.5e-15 measured)",
            ),
        ),
        (
            "generator/lengths-and-areas",
            Rule::new(
                Relative,
                RoundOff,
                1e-9,
                Exact,
                "lengths and areas in closed form, summed over up to a million vertices: round-off (1.6e-15 measured)",
            ),
        ),
        (
            "generator/gaps",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the gaps the definitions give: round-off (1.2e-15 measured)",
            ),
        ),
        (
            "generator/symmetry",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "a count of points inside one half and outside the other: none",
            ),
        ),
        (
            "generator/devices",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the devices' shapes and ports to the bit",
            ),
        ),
        (
            "generator/convergence",
            Rule::new(
                Absolute,
                Order,
                0.15,
                Measured,
                "chords stray by κh²/8: second order, within 0.15",
            ),
        ),
        (
            "generator/wang-bragg",
            Rule::new(
                Absolute,
                RoundOff,
                1e-9,
                Exact,
                "the published grating's geometry, in closed form: round-off (1.1e-13 measured)",
            ),
        ),
        (
            "generator/job-expressions",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the expressions' values to the bit",
            ),
        ),
        (
            "generator/job-ports",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the ports by name to the bit",
            ),
        ),
        // ---- stacks ----
        (
            "stack/vertical-sidewall-bits",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same cells by either path, to the bit",
            ),
        ),
        (
            "stack/sloped-wall-fill",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "each cell's share the clipped polygon's area, exactly: round-off",
            ),
        ),
        (
            "stack/wide-rib-slab",
            Rule::new(
                Absolute,
                Discretisation,
                2e-5,
                Measured,
                "the grid's and the 1/W² extrapolation's error against the slab's index",
            ),
        ),
        (
            "stack/rib-hadley-table",
            Rule::new(
                Absolute,
                Reference,
                3e-6,
                Stated(
                    1.7e-7,
                    "the grid study at 50, 25 and 12.5 nm, second order and more",
                ),
                "Hadley's Table I, his ±2–3e-6 (ours 1.4e-6 from it, converged)",
            ),
        ),
        (
            "stack/rib-order",
            Rule::new(
                Absolute,
                Order,
                1.0,
                Measured,
                "from second to third order with the corners' equations: within one of the second",
            ),
        ),
        (
            "stack/oblique-rod",
            Rule::new(
                Absolute,
                Discretisation,
                2.5e-3,
                Measured,
                "an interface off the grid's lines, near 2e-3 for the scheme (staircased 6.1e-3)",
            ),
        ),
        (
            "stack/sidewall-convergence",
            Rule::new(
                Absolute,
                Order,
                0.25,
                Measured,
                "first order halves the changes: their ratio 0.5, within 0.25",
            ),
        ),
        // ---- averaging ----
        (
            "averaging/film-integrals",
            Rule::new(
                Relative,
                RoundOff,
                1e-13,
                Exact,
                "the cells' integrals add up to the layer's: round-off",
            ),
        ),
        (
            "averaging/disk-area",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "a disk's exact fill adds up to its area: round-off",
            ),
        ),
        (
            "averaging/aligned-bits",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "one material in every cell: the averages to the bit",
            ),
        ),
    ]
}
