# Examples

Each example reproduces a published result: it computes something with photonoxide, prints it
next to the number the paper prints, with the grid and the tolerance, and fails (exit code 1)
when they disagree. CI runs every example.

```sh
cargo run --release --example <name>
```

What each prints is in [`output/`](output), and CI checks that it still prints exactly that.
After changing an example, regenerate its output:

```sh
cargo run --release --quiet --example <name> > examples/output/<name>.txt
```

| Example | What it computes | Checked against | Tolerance |
|---|---|---|---|
| [`silicon_index`](silicon_index.rs) | Silicon's refractive index, 1.3–10 µm | H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624), Table 1 (293 K) | 5e-5 (4 printed decimals) |
| [`silica_index`](silica_index.rs) | Fused silica's refractive index, 0.55–3.2 µm | I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965), [doi:10.1364/JOSA.55.001205](https://doi.org/10.1364/JOSA.55.001205), Table I | 1e-6 (6 printed decimals) |
| [`slab_soi`](slab_soi.rs) | TE and TM modes of a 220 nm silicon slab in oxide at 1550 nm, exact | L. Chrostowski, M. Hochberg, *Silicon Photonics Design* (2015), [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.2 | 5e-4 (3 printed decimals) |
| [`slab_yariv_yeh`](slab_yariv_yeh.rs) | Guided modes of an asymmetric and a symmetric slab, exact | A. Yariv, P. Yeh, *Photonics*, 6th ed., Oxford University Press (2007), Sections 3.1–3.2 (a textbook, no DOI) | 5e-5 (4 printed decimals); mode counts exact |
| [`bend_loss`](bend_loss.rs) | The radiation loss of bent slabs: exact, against Marcuse's formula as the radius grows, and from the full-vector solver with a conformal map and a PML | D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), [doi:10.1002/j.1538-7305.1971.tb02620.x](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x), Eqs. 32–33; M. Heiblum, J. H. Harris (1975) | 10 % at 120 µm: the formula is an approximation of order d/R |
| [`effective_index_method`](effective_index_method.rs) | The effective index method on 220 nm strips, 400–600 nm wide, and its error against the full-vector solver in n_eff and n_g | L. Chrostowski, M. Hochberg (2015), Section 3.2.5: 2.489 for 500 nm; G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), [doi:10.1364/AO.16.000113](https://doi.org/10.1364/AO.16.000113) | 1e-3: the book's rounded slab index and 10 nm mesh |
| [`group_index`](group_index.rs) | The group index of 220 nm silicon strips, 400–600 nm wide, at 1.55 µm, with the book's dispersive silicon; the TE-like mode tracked over wavelength, on a quarter domain | L. Chrostowski, M. Hochberg, *Silicon Photonics Design* (2015), [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Fig. 3.22b, read off the plot | 0.02: the plot's reading (±0.005), the book's 20 nm mesh and our corners |
| [`multilayer_chilwell`](multilayer_chilwell.rs) | A four-layer planar guide by transfer matrices: its 8 bound modes, the power each puts in every layer, and 5 leaky waves (complex) | J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), [doi:10.1364/JOSAA.1.000742](https://doi.org/10.1364/JOSAA.1.000742), Tables 2–3 | 5e-7 bound, 1.5e-5 leaky (one printed value is a unit off), 0.06 % power |
| [`leaky_waves`](leaky_waves.rs) | The same guide's five TE leaky waves, now from the full-vector solver with a PML absorbing the leakage, 2.5 nm grid | J. Chilwell, I. Hodgkinson (1984), Table 2 | 5e-5 (m = 8, at the cover's cutoff: 2e-4) |
| [`hadley_corners`](hadley_corners.rs) | Four waveguides with dielectric corners (boxes and impinged corners, ε = 2.25 and 8), full-vector, at 40, 80 and 160 grids, with mirror walls | G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371), Figs. 4–7 (series expansions, 1e-8) | 5e-5 at a 6.25 nm grid |
| [`strip_waveguide`](strip_waveguide.rs) | TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector, at 20, 10 and 5 nm grids | L. Chrostowski, M. Hochberg, *Silicon Photonics Design* (2015), Fig. 3.14 (Lumerical MODE, 20 nm mesh) | 3e-3 at 5 nm: the book's value is good to about 1e-3, and the corners slow convergence here |

**Adding an example:**

- One file per published result. Its doc comment names the paper (with its DOI), the table, figure or page, and how many digits it prints.
- Compare with `common::Checks`, against the paper's numbers as printed. Set the tolerance from the printed digits, or from the paper's stated accuracy.
- Print the grid for every computed number ("exact: no grid" for closed forms).
- Add a row to the table above, and its output to `output/`.
- Print nothing that changes between runs (timings, for instance): the output is compared.

The full validation report, including analytic and convergence checks, is [docs/validation.md](../docs/validation.md).
