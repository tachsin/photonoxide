// A component as a section of a job file (src/lib/designs.ts, #668): the page holds a design's
// parameters in the one order a file gives back (devices.rs `in_order` is the same rule), a
// built-in component's name is its own, and each built-in component's file in jobs/components/
// opens in the Job tab as its `device` section. Run by `pnpm test`.
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { test } from "node:test";

// the studio's modules import each other without the `.ts` the bundler doesn't need: found here
registerHooks({
  resolve(specifier, context, next) {
    try {
      return next(specifier, context);
    } catch (e) {
      if (specifier.startsWith(".") && !/\.\w+$/.test(specifier)) return next(`${specifier}.ts`, context);
      throw e;
    }
  },
});
const { builtInWith, designNameOf, inOrder, nameProblem, sameParameters } = await import("../src/lib/designs.ts");
const { sectionOf } = await import("../src/lib/jobsection.ts");

// the directional coupler's own parameters, in its order
const coupler = { parameters: ["width", "thickness", "gap", "length", "loss", "wavelength", "span", "slab", "sidewall"].map((name) => ({ name, unit: "", default: 0, min: 0, max: 1 })) };

test("the device's own parameters come first, in its order, then the others by name", () => {
  const typed: [string, string][] = [
    ["p", "0.2"],
    ["gap", "p"],
    ["a", "1"],
    ["width", "0.5"],
  ];
  assert.deepEqual(inOrder(coupler, typed), [
    ["width", "0.5"],
    ["gap", "p"],
    ["a", "1"],
    ["p", "0.2"],
  ]);
  // the order is the same however the parameters were given: a file's sorted table too
  const sorted = typed.slice().sort((x, y) => (x[0] < y[0] ? -1 : 1));
  assert.ok(sameParameters(inOrder(coupler, sorted), inOrder(coupler, typed)));
  assert.ok(!sameParameters(inOrder(coupler, typed), typed));
});

test("a built-in component's name is its own, in either case; a name is a job's", () => {
  const builtIn = [{ name: "directional-coupler" }, { name: "waveguide" }];
  assert.equal(builtInWith(builtIn, "Directional-Coupler")?.name, "directional-coupler");
  assert.equal(builtInWith(builtIn, "my-coupler"), undefined);
  assert.equal(nameProblem("my-coupler_2.v1"), "");
  assert.notEqual(nameProblem("my coupler"), "");
});

test("a tab's job, opened from its file, gives its design's name back", () => {
  // the Simulate tab's run of "my-mmi" at each way it is computed, as SimulateTab names it
  for (const how of ["analytic", "2D", "2D FDFD", "3D", "FDTD 2D", "FDTD 3D"]) {
    const job = `my-mmi-${how.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`.replace(/-+$/, "");
    assert.equal(designNameOf(job, how), "my-mmi", how);
  }
  // the Study, Optimize and Model tabs' jobs
  assert.equal(designNameOf("my-ring-sweep-gap-width", "sweep-gap-width"), "my-ring");
  assert.equal(designNameOf("my-ring-corners", "corners"), "my-ring");
  assert.equal(designNameOf("my-mmi-optimize-imbalance", "optimize-imbalance"), "my-mmi");
  assert.equal(designNameOf("my-ring-compact-gap", "compact-gap"), "my-ring");
  // a job named otherwise keeps its name; a suffix alone is no design's name
  assert.equal(designNameOf("ring-check", "2D"), "ring-check");
  assert.equal(designNameOf("-2d", "2D"), "-2d");
});

test("each built-in component's file opens as its device section, every parameter in it", () => {
  const dir = new URL("../../jobs/components/", import.meta.url);
  const files = readdirSync(dir).filter((f) => f.endsWith(".yaml"));
  assert.equal(files.length, 18);
  for (const f of files) {
    const text = readFileSync(new URL(f, dir), "utf8").replace(/\r\n/g, "\n");
    const s = sectionOf(text, "yaml", "device");
    assert.ok(s, f);
    assert.ok(s.shown.startsWith("device:\n"), f);
    assert.ok(/\n {2}parameters:\n/.test(s.shown), f);
    // the rest of the file is its name and description, outside the section
    assert.ok(text.startsWith(`name: ${f.replace(/\.yaml$/, "")}\ndescription: `), f);
  }
});
