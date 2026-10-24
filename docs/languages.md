# photonoxide from any language

Python has a package ([docs/python.md](python.md)). Every other language uses photonoxide the
same way, through the `photonoxide` program: it writes a job file, runs
`photonoxide run <job> --headless`, and reads the run's folder. Whatever a job can do (modes, 2D
and 3D FDFD, FDTD, the static solver, devices, sweeps, optimization), any language that can start
a process and read JSON or CSV can do it, with the numbers photonoxide computed, to the bit.

- [What the program gives a script](#what-the-program-gives-a-script)
- [The shell](#the-shell)
- [R](#r)
- [JavaScript](#javascript)
- [Go](#go)

## What the program gives a script

The program is in each release, for Linux, Windows and macOS (the [README](../README.md) has
the downloads); a script calls it as `photonoxide`, on the `PATH` or by its path.

- **The job.** A job file in YAML, TOML or JSON. `photonoxide new <kind> <file>` writes one of
  each kind to start from (`photonoxide new --list` lists them), and
  [docs/job.schema.json](job.schema.json) is its JSON Schema, which `photonoxide schema` prints.
  A language that builds the job as data writes it as JSON, which every language can.
- **The run.** `photonoxide run <job> --headless [--timeout <minutes>] [--out <dir>]` prints the
  run's folder on its first line of output (`runs/20261024-120000-<name>`) and exits when the run
  ends: 0 finished, 1 failed (the error on stderr and in `results.json`), 2 a wrong command line,
  3 stopped by its timeout, 4 stopped on request ([docs/runs.md](runs.md#exit-codes)).
- **The numbers.** The folder's `results.json` (`"schema": 1`) holds the run's numbers: its grids,
  modes, sweep points, S-parameters, spectra and resonances, each array with its shape and unit
  ([docs/runs.md](runs.md#resultsjson)). Every number is written so that it reads back to the same
  `f64`, and every JSON reader that parses into doubles gets it exactly.
- **The tables.** The same numbers as CSV files (`modes.csv`, `s_parameters.csv`, …), one value
  a row with its coordinates beside it, a text with a comma quoted ([export](methods/export.md)),
  for a language with a CSV reader and no JSON one.
- **The fields,** as NumPy's `.npz`, when the job's `[task.export]` asks for them; an `.npz` is a
  ZIP of `.npy` files, for which most languages have readers.

Never quote a number without its grid: the grid is in `results.json`'s `grids`, and the job says
it too. The examples below were run against the program built from main (after 0.5.4) on Linux,
with jq 1.7, R 4.3, Node.js 22 and Go 1.24; they aren't run by CI.

## The shell

With [jq](https://jqlang.github.io/jq/) for the JSON, the modes of the strip `photonoxide new modes` writes:

```sh
#!/usr/bin/env bash
set -euo pipefail
[ -f strip.yaml ] || photonoxide new modes strip.yaml > /dev/null   # a 500 x 220 nm strip
out=$(photonoxide run strip.yaml --headless --timeout 5)   # the script stops here if it fails
run=${out%%$'\n'*}                                       # the run's folder, on the first line
jq -e '.schema == 1 and .outcome == "finished"' "$run/results.json" > /dev/null
jq -r '.grids[0].step_um as $h | .modes[] |
  "\(.label): n_eff \(.effective_index[0]), TE fraction \(.te_fraction) (grid step \($h) um)"' \
  "$run/results.json"
```

```text
mode 1 of 2: n_eff 2.4497971939172745, TE fraction 0.9736023559179123 (grid step 0.02 um)
mode 2 of 2: n_eff 1.7853122219332618, TE fraction 0.006731361163811839 (grid step 0.02 um)
```

The output is captured whole and its first line taken, rather than piped through `head`, which
would end the program early if it printed more.

## R

Base R has no JSON reader, so this reads the CSV table (jsonlite reads `results.json`):

```r
# base R only: system2 runs the program, read.csv reads its table
out <- system2("photonoxide", c("run", "strip.yaml", "--headless", "--timeout", "5"), stdout = TRUE)
if (!is.null(attr(out, "status"))) stop("photonoxide run failed: exit ", attr(out, "status"))
run <- out[1]                                     # the run's folder, on the first line
modes <- read.csv(file.path(run, "modes.csv"))    # one row a mode
# the grid is in results.json (a JSON reader such as jsonlite reads it), and in the job
cat(trimws(sub("#.*", "", grep("^ *step:", readLines("strip.yaml"), value = TRUE))), "\n")
print(modes[, c("label", "wavelength_um", "n_eff_re", "te_fraction")], digits = 17)
```

```text
step: 20 nm
        label wavelength_um           n_eff_re          te_fraction
1 mode 1 of 2          1.55 2.4497971939172745 0.973602355917912332
2 mode 2 of 2          1.55 1.7853122219332618 0.006731361163811839
```

## JavaScript

Node.js, its standard library only: a 2D FDFD job built as an object and written as JSON, a
straight strip between two ports, and its S-matrix read from `results.json`. A lossless straight
guide transmits everything and is reciprocal, which the output shows:

```js
// Node.js 18 or later, its standard library only
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// the job as data: a straight 500 nm strip between two ports, by 2D FDFD
const job = {
  name: "straight-guide",
  timeout_minutes: 5,
  task: {
    kind: "fdfd", stack: "soi_220", wavelength_um: 1.55, layer: "Si", polarization: "te",
    x_um: [-2, 2], y_um: [-1.5, 1.5], step_nm: 20,
    rect: [{ layer: "Si", center_um: [0, 0], size_um: [6, 0.5] }],
    port: [{ x_um: -1, side: "left" }, { x_um: 1, side: "right" }],
  },
};
writeFileSync("guide.json", JSON.stringify(job, null, 2));
// throws if the run fails; its first line is the run's folder
const run = execFileSync("photonoxide", ["run", "guide.json", "--headless"], { encoding: "utf8" })
  .split("\n")[0];
const results = JSON.parse(readFileSync(join(run, "results.json"), "utf8"));
if (results.schema !== 1 || results.outcome !== "finished") throw new Error(results.error);

// an array: its data inline, or above 4 096 values a file of little-endian doubles
function values(a) {
  if (a.data) return Float64Array.from(a.data);
  const bytes = readFileSync(join(run, a.file));
  return new Float64Array(bytes.buffer, bytes.byteOffset, bytes.length / 8);
}
const { ports, wavelengths_um, s } = results.s_parameters;
const v = values(s);                     // complex128: re, im; shape [wavelengths, ports, ports]
const n = s.shape[1];
const at = (k, q, p) => [v[2 * ((k * n + q) * n + p)], v[2 * ((k * n + q) * n + p) + 1]];
const power = ([re, im]) => re * re + im * im;
const { step_um } = results.grids[0];
wavelengths_um.forEach((w, k) => {
  const [s11, s21, s12] = [at(k, 0, 0), at(k, 1, 0), at(k, 0, 1)];
  console.log(`${w} um, grid step ${step_um} um: |S21|^2 ${power(s21)},`
    + ` |S11|^2 + |S21|^2 ${power(s11) + power(s21)}`);
  console.log(`  reciprocity: S21 - S12 = ${s21[0] - s12[0]} + ${s21[1] - s12[1]}i`);
});
console.log(`ports: ${ports.join("; ")}`);
```

```text
1.55 um, grid step 0.02 um: |S21|^2 1.0000000021926054, |S11|^2 + |S21|^2 1.0000000021926054
  reciprocity: S21 - S12 = -2.7755575615628914e-17 + -1.7763568394002505e-15i
ports: 1 (left, x = -1 um); 2 (right, x = 1 um)
```

Run as `node guide.mjs`. The power is conserved to the solver's 2e-9 on this grid, and S21 = S12
to the last bits.

## Go

Go's standard library only: the same job (`guide.json`, as the JavaScript writes it), its
`results.json` read with a struct, an array taken inline or from its file, and checked against
`s_parameters.csv`, to the bit:

```go
// Go's standard library only: go run . (in a folder with guide.json)
package main

import (
	"bytes"
	"encoding/binary"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
)

// Array is results.json's array: inline data, or above 4 096 values a file of doubles.
type Array struct {
	Dtype string    `json:"dtype"`
	Shape []int     `json:"shape"`
	Data  []float64 `json:"data"`
	File  *string   `json:"file"`
}

func (a Array) values(run string) []float64 {
	if a.File == nil {
		return a.Data
	}
	raw, err := os.ReadFile(filepath.Join(run, *a.File))
	if err != nil {
		log.Fatal(err)
	}
	v := make([]float64, len(raw)/8)
	if err := binary.Read(bytes.NewReader(raw), binary.LittleEndian, v); err != nil {
		log.Fatal(err)
	}
	return v
}

func main() {
	out, err := exec.Command("photonoxide", "run", "guide.json", "--headless").Output()
	if err != nil {
		log.Fatalf("photonoxide run: %v", err) // an *exec.ExitError carries the exit code
	}
	run := string(bytes.SplitN(out, []byte("\n"), 2)[0])
	text, err := os.ReadFile(filepath.Join(run, "results.json"))
	if err != nil {
		log.Fatal(err)
	}
	var results struct {
		Schema  int    `json:"schema"`
		Outcome string `json:"outcome"`
		Grids   []struct {
			StepUm float64 `json:"step_um"`
		} `json:"grids"`
		SParameters struct {
			Ports         []string  `json:"ports"`
			WavelengthsUm []float64 `json:"wavelengths_um"`
			S             Array     `json:"s"`
		} `json:"s_parameters"`
	}
	if err := json.Unmarshal(text, &results); err != nil {
		log.Fatal(err)
	}
	if results.Schema != 1 || results.Outcome != "finished" {
		log.Fatalf("schema %d, outcome %s", results.Schema, results.Outcome)
	}
	s := results.SParameters.S.values(run) // complex128: re, im; shape [wavelengths, ports, ports]
	n := results.SParameters.S.Shape[1]
	index := map[string]int{} // a port's or a wavelength's place in the array
	for i, p := range results.SParameters.Ports {
		index[p] = i
	}
	for k, w := range results.SParameters.WavelengthsUm {
		index[strconv.FormatFloat(w, 'g', -1, 64)] = k
	}

	// the CSV table holds the same numbers, to the bit
	f, err := os.Open(filepath.Join(run, "s_parameters.csv"))
	if err != nil {
		log.Fatal(err)
	}
	rows, err := csv.NewReader(f).ReadAll() // wavelength_um, to, from, s_re, s_im
	if err != nil {
		log.Fatal(err)
	}
	for k, row := range rows[1:] {
		at := 2 * ((index[row[0]]*n+index[row[1]])*n + index[row[2]])
		re, _ := strconv.ParseFloat(row[3], 64)
		im, _ := strconv.ParseFloat(row[4], 64)
		if re != s[at] || im != s[at+1] {
			log.Fatalf("row %d: %v differs from results.json", k+1, row)
		}
	}
	fmt.Printf("%d S-parameters, the same bits in results.json and s_parameters.csv"+
		" (grid step %v um)\n", len(rows)-1, results.Grids[0].StepUm)
}
```

```text
4 S-parameters, the same bits in results.json and s_parameters.csv (grid step 0.02 um)
```

With a wavelength sweep in the job (`"sweep": {"parameter": "wavelength", "from": 1.5, "to": 1.6,
"points": 1025}`), the S-matrix has more than 4 096 values and goes to `s_parameters.s.c128`
beside `results.json`; the same program reads it from there: `4100 S-parameters, the same bits`.
