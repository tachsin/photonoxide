<p align="center">
  <img src="https://raw.githubusercontent.com/tachsin/photonoxide/main/assets/brand/banner.svg" alt="photonoxide: validated photonics for Rust" width="100%">
</p>

# photonoxide for Python

[photonoxide](https://github.com/tachsin/photonoxide) is a photonics library in Rust whose every
method is checked against an analytic solution or a published result, in a
[validation report](https://github.com/tachsin/photonoxide/blob/main/docs/validation.md). This
package brings a part of it to Python, and through Python to MATLAB:

- the materials catalogue: indices with their source, validity range and conditions;
- waveguide modes: the exact slab, and full-vector modes of a cross-section with their fields;
- jobs (TOML, JSON or YAML, or a dict), run as the photonoxide program runs them, and an FDFD
  job's S-parameters;
- circuits from a netlist given as data, and Touchstone files.

```sh
pip install photonoxide
```

Wheels for CPython 3.10 and later on Linux, macOS and Windows; nothing else to install.

```python
import photonoxide as po

si = po.refractive_index("si", wavelength_um=1.55).real    # Li 1980
ox = po.refractive_index("sio2", wavelength_um=1.55).real  # Malitson 1965
for polarization in ("te", "tm"):
    for mode in po.slab_modes(below=ox, core=si, above=ox, thickness_um=0.22,
                              polarization=polarization, wavelength_um=1.55):
        print(f"{polarization.upper()}{mode.order}: n_eff = {mode.effective_index:.4f}")
```

It prints `TE0: n_eff = 2.8475` and `TM0: n_eff = 2.0531`.

Units are in the names (`wavelength_um`, `thickness_um`), as photonoxide's job files have them;
arrays are NumPy's, in C order, with their coordinates; errors are photonoxide's, with its
messages. The package's version is the Rust crate's it was built from. The guide, with MATLAB's
use: [docs/python.md](https://github.com/tachsin/photonoxide/blob/main/docs/python.md).

## License

MIT or Apache-2.0, at your option.
