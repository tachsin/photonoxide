# Examples

Each example reproduces a published result: it computes something with photonoxide, prints it
next to the number the paper prints, with the grid and the tolerance, and fails (exit code 1)
when they disagree. CI runs every example.

```sh
cargo run --release --example <name>
```

| Example | What it computes | Checked against | Tolerance |
|---|---|---|---|
| [`silicon_index`](silicon_index.rs) | Silicon's refractive index, 1.3–10 µm | H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624), Table 1 (293 K) | 5e-5 (4 printed decimals) |
| [`silica_index`](silica_index.rs) | Fused silica's refractive index, 0.55–3.2 µm | I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965), [doi:10.1364/JOSA.55.001205](https://doi.org/10.1364/JOSA.55.001205), Table I | 1e-6 (6 printed decimals) |
| [`slab_soi`](slab_soi.rs) | TE and TM modes of a 220 nm silicon slab in oxide at 1550 nm, exact | L. Chrostowski, M. Hochberg, *Silicon Photonics Design* (2015), [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.2 | 5e-4 (3 printed decimals) |
| [`slab_yariv_yeh`](slab_yariv_yeh.rs) | Guided modes of an asymmetric and a symmetric slab, exact | A. Yariv, P. Yeh, *Photonics*, 6th ed., Oxford University Press (2007), Sections 3.1–3.2 (a textbook, no DOI) | 5e-5 (4 printed decimals); mode counts exact |
| [`strip_waveguide`](strip_waveguide.rs) | TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector, at 20, 10 and 5 nm grids | L. Chrostowski, M. Hochberg, *Silicon Photonics Design* (2015), Fig. 3.14 (Lumerical MODE, 20 nm mesh) | 3e-3 at 5 nm: the book's value is good to about 1e-3, and the corners slow convergence here |

**Adding an example:**

- One file per published result. Its doc comment names the paper (with its DOI), the table, figure or page, and how many digits it prints.
- Compare with `common::Checks`, against the paper's numbers as printed. Set the tolerance from the printed digits, or from the paper's stated accuracy.
- Print the grid for every computed number ("exact: no grid" for closed forms).
- Add a row to the table above.

The full validation report, including analytic and convergence checks, is [docs/validation.md](../docs/validation.md).
