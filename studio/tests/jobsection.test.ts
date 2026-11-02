// A section of a job's text (src/lib/jobsection.ts), as the shared job editor opens it (#668):
// the editor shows the section alone, with the same paths as in the whole job, and an edit of it
// changes that section only; the rest of the text stays as it is, to the byte. Run by `pnpm test`.
import assert from "node:assert/strict";
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
const { bodyOf, intoSection, messageIn, outOfSection, sectionOf, sectionSpan, withSection } = await import("../src/lib/jobsection.ts");
const { outline } = await import("../src/lib/jobschema.ts");

type Format = "yaml" | "toml" | "json";

// jobs/coupler-gap-sweep.yaml, and the same job in TOML and JSON
const YAML = `# A directional coupler's study: its gap swept.
#   photonoxide run jobs/coupler-gap-sweep.yaml
name: coupler-gap-sweep
timeout_minutes: 5

device:
  kind: directional coupler
  fidelity: analytic

  parameters:
    length: "15"

# how it is computed
task:
  kind: sweep
  from_um: 1.5
  to_um: 1.6
  points: 21

  axis:
    - parameter: gap
      from: 0.15
      to: 0.45
      points: 31
`;

const TOML = `# A directional coupler's study: its gap swept.
name = "coupler-gap-sweep"
timeout_minutes = 5

[device]
kind = "directional coupler"
fidelity = "analytic"

[device.parameters]
length = "15"

# how it is computed
[task]
kind = "sweep"
from_um = 1.5
to_um = 1.6
points = 21

[[task.axis]]
parameter = "gap"
from = 0.15
to = 0.45
points = 31
`;

const JSON_TEXT = `{
  "name": "coupler-gap-sweep",
  "timeout_minutes": 5,
  "device": {
    "kind": "directional coupler",
    "fidelity": "analytic",
    "parameters": {
      "length": "15"
    }
  },
  "task": {
    "kind": "sweep",
    "from_um": 1.5,
    "to_um": 1.6,
    "points": 21,
    "axis": [
      {
        "parameter": "gap",
        "from": 0.15,
        "to": 0.45,
        "points": 31
      }
    ]
  }
}
`;

const JOBS: [Format, string][] = [
  ["yaml", YAML],
  ["toml", TOML],
  ["json", JSON_TEXT],
];

const keys = (text: string, format: Format) => outline(text, format).map((e) => e.path.join("."));

for (const [format, text] of JOBS) {
  test(`${format}: the device section is shown alone, with the paths it has in the whole job`, () => {
    const s = sectionOf(text, format, "device");
    assert.ok(s, "a section");
    assert.match(s.shown, /directional coupler/);
    assert.doesNotMatch(s.shown, /sweep|coupler-gap-sweep|how it is computed/);
    const inWhole = keys(text, format).filter((k) => k.startsWith("device"));
    assert.deepEqual(keys(s.shown, format), inWhole);
    assert.deepEqual(inWhole, ["device", "device.kind", "device.fidelity", "device.parameters", "device.parameters.length"]);
    // and the span's text is the section's, unwrapped
    assert.equal(bodyOf(s, s.shown, format), text.slice(s.from, s.to));
  });

  test(`${format}: an edit of the device section changes it only; the rest of the text stays to the byte`, () => {
    const s = sectionOf(text, format, "device")!;
    const edited = s.shown.replace('"15"', '"16.5"').replace("analytic", "2D");
    const { text: out, section } = withSection(text, s, edited, format);
    assert.equal(out.slice(0, s.from), text.slice(0, s.from));
    assert.equal(out.slice(section.to), text.slice(s.to));
    assert.equal(out, text.replace('"15"', '"16.5"').replace('"analytic"', '"2D"').replace("fidelity: analytic", "fidelity: 2D"));
    // the section read again from the new text is the one written
    assert.equal(sectionOf(out, format, "device")!.shown, edited);
    // a second edit over the first, by the span the first left
    const again = withSection(out, section, section.shown.replace("directional coupler", "waveguide"), format);
    assert.ok(again.text.endsWith(out.slice(section.to)));
    assert.equal(again.text.slice(0, s.from), text.slice(0, s.from));
    // the comment above, which names the coupler too, isn't the section's: it stays
    assert.equal(again.text, out.replace(/(kind\W+)directional coupler/, "$1waveguide"));
    if (format !== "json") assert.match(again.text, /^# A directional coupler's study/);
  });

  test(`${format}: the task section runs to its end, the job's name is a section of one line`, () => {
    const t = sectionOf(text, format, "task")!;
    assert.match(t.shown, /kind/);
    assert.match(t.shown, /points/);
    assert.doesNotMatch(t.shown, /directional coupler/);
    assert.deepEqual(
      keys(t.shown, format).filter((k) => k.split(".").length <= 2),
      keys(text, format).filter((k) => k.startsWith("task") && k.split(".").length <= 2),
    );
    const n = sectionOf(text, format, "name")!;
    assert.match(text.slice(n.from, n.to), /^"?name"?\s*[:=]\s*"?coupler-gap-sweep"?,?$/);
    assert.equal(sectionOf(text, format, "optimize"), null);
  });

  test(`${format}: offsets go into the section and back, and the check's line is counted in it`, () => {
    const s = sectionOf(text, format, "device")!;
    const at = text.indexOf("fidelity");
    const inside = intoSection(s, at)!;
    assert.equal(s.shown.slice(inside, inside + 8), "fidelity");
    assert.equal(outOfSection(s, inside), at);
    assert.equal(intoSection(s, text.indexOf("points")), null);
    const line = text.slice(0, at).split("\n").length;
    const col = at - text.lastIndexOf("\n", at - 1);
    const shown = messageIn(text, s, `bad value at line ${line}, column ${col}`)!;
    const m = /line (\d+), column (\d+)/.exec(shown)!;
    const lines = s.shown.split("\n");
    assert.equal(lines[Number(m[1]) - 1].slice(Number(m[2]) - 1, Number(m[2]) + 7), "fidelity");
    assert.equal(messageIn(text, s, `bad value at line ${text.slice(0, text.indexOf("points")).split("\n").length}, column 1`), null);
    assert.equal(messageIn(text, s, "[device] width: out of range"), "[device] width: out of range");
  });
}

test("toml: a table written in two pieces around another isn't one section", () => {
  const split = `name = "x"\n[device]\nkind = "waveguide"\n[task]\nkind = "model"\n[device.parameters]\nwidth = "0.5"\n`;
  assert.equal(sectionSpan(split, "toml", "device"), null);
  assert.ok(sectionSpan(split, "toml", "task"));
});
