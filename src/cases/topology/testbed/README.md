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
