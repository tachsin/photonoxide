# The testbed's mode converter designs

Eight of the waveguide mode converter designs of the photonics inverse-design testbed (M. Chen et
al., J. Opt. Soc. Am. B 41, A161 (2024), doi:10.1364/JOSAB.506412), from
[NanoComp/photonics-opt-testbed](https://github.com/NanoComp/photonics-opt-testbed) at commit
`a06518872de6ec82bb4511190e544211539f55f1`, `waveguide_mode_converter/designs/`, under the MIT
licence (LICENSE here, the testbed's). The case `topology/ruler-testbed` measures their minimum
length scale against the lengths the testbed's README prints.

Each file is one design, 160 × 160 pixels of 10 nm, whose CSV values are all 0.00 or 1.00: its
rows as in the CSV (the first array axis, x), each row's 160 values as 40 hex digits, the first
value the first digit's highest bit. Converted from the CSV files with

```sh
awk -F, '{ s = ""; for (k = 1; k <= NF; k += 4) { n = 0; for (q = 0; q < 4; q++) n = n * 2 + ($(k + q) + 0 > 0.5); s = s sprintf("%x", n) } print s }' design.csv > design.hex
```

| File | Printed length (nm) |
|---|---|
| `converter_generator_circle_10_x47530832_w11_s483.hex` | 90.2978515625 |
| `converter_generator_circle_10_x47530832_w27_s681.hex` | 101.9775390625 |
| `converter_generator_circle_12_x47530832_w12_s248.hex` | 119.4970703125 |
| `converter_generator_circle_14_x47530832_w13_s793.hex` | 142.8564453125 |
| `converter_generator_circle_16_x47530832_w14_s150.hex` | 166.2158203125 |
| `converter_generator_circle_18_x47530832_w15_s879.hex` | 169.1357421875 |
| `converter_generator_circle_20_x47530832_w16_s416.hex` | 204.1748046875 |
| `converter_generator_circle_20_x47530832_w40_s988.hex` | 207.0947265625 |

## The metagrating designs

Five of the testbed's 3D metagrating designs (Section 3.F and Fig. 8 of the paper), from the same
repository at commit `683c9249f21bf751f3d2d4a1a0400250098a03b1`, `Metagrating3D/designs/`, under
the same licence: the interpolated files, which the testbed's validation reads (its
`metagrating_meep.py`: "image resolution is ~340 pixels/μm"). `device1_interpolated.csv` to
`device4_interpolated.csv` are 472 × 180 pixels of silicon (1) and air (0), the rows along x,
converted to hex as above (each row's 180 values as 45 hex digits);
`metagrating_device5_interpolated.csv` is copied as it is, 473 values along x on the nodes of 472
intervals, stripes. The example `optimize_metagrating` solves them.

| File | Testbed's README (RCWA, FDTD) | In the paper (RCWA, FDTD) |
|---|---|---|
| `metagrating_device1_interpolated.hex` | 95.7 %, 95.5 % | not shown |
| `metagrating_device2_interpolated.hex` | 93.3 %, 93.8 % | Fig. 8(b): 93.2 %, 93.8 % |
| `metagrating_device3_interpolated.hex` | 96.6 %, 95.0 % | Fig. 8(c): 96.8 %, 95.0 % |
| `metagrating_device4_interpolated.hex` | 93.3 %, 92.5 % | Fig. 8(d): 93.7 %, 92.5 % |
| `metagrating_device5_interpolated.csv` | 84.1 %, 84.3 % | Fig. 8(e): 83.0 %, 84.3 % |
