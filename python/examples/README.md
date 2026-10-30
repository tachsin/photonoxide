# Python examples

Each example reproduces a published result through the Python package, as the Rust examples in
[`examples/`](../../examples) do: it prints its numbers beside the paper's, with the grid and the
tolerance, and exits with 1 when they disagree. An example with a Rust twin prints what the twin
prints after its setup block, line for line, and CI checks both: the package's tests run every example
(`python/tests/test_examples.py`) and compare a twin's output with the Rust one's in
[`examples/output`](../../examples/output).

```sh
python python/examples/<name>.py
```

| Example | What it computes | Checked against | Tolerance |
|---|---|---|---|
| [`silicon_index`](silicon_index.py) | Silicon's refractive index, 1.3–10 µm; the twin of [`silicon_index.rs`](../../examples/silicon_index.rs) | H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624), Table 1 (293 K) | 5e-5 (4 printed decimals) |
| [`slab_soi`](slab_soi.py) | TE and TM modes of a 220 nm silicon slab in oxide at 1550 nm, exact; the twin of [`slab_soi.rs`](../../examples/slab_soi.rs) | L. Chrostowski, M. Hochberg, *Silicon Photonics Design* (2015), [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.2 | 5e-4 (3 printed decimals) |
| [`strip_waveguide`](strip_waveguide.py) | TE-like mode of a 500 × 220 nm silicon strip at 1550 nm, full-vector on the window's quarter, at 20, 10 and 5 nm grids (as [`strip_waveguide.rs`](../../examples/strip_waveguide.rs), whose cross-section is the strip device's) | L. Chrostowski, M. Hochberg (2015), Fig. 3.14 (Lumerical MODE, 20 nm mesh) | 3e-3 at 5 nm: the book's value is good to about 1e-3, and the corners slow convergence |
| [`ring_fsr`](ring_fsr.py) | An all-pass ring's free spectral range from its through port's minima, as a circuit given as data (the validation case `circuit/ring-fsr-bogaerts`) | W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), [doi:10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017), Eq. 9 | 1e-4 nm |
| [`touchstone_ring`](touchstone_ring.py) | `ring_fsr`'s ring through a Touchstone 2.0 file, every 10 pm from 1.53 to 1.57 µm: the S-parameters read back to the same bits, the file as a measured component gives the circuit's spectrum bit for bit, and its samples' minima the free spectral range | Bogaerts et al. (2012), Eq. 9; the format: Touchstone File Format Specification, Version 2.0, IBIS Open Forum (2009) | the bits exactly; 1e-4 nm (the samples' minima are within 3e-5 nm of the exact ones) |
