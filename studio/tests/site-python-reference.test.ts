// Where the site's Python API reference page reads a page (site/lib/projects/photonoxide/
// python-reference.js): the app's committed copy first, else the latest release's asset. On a
// temporary folder and a stand-in for the release: no network.
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import {
  PYTHON_REFERENCE_DIR,
  localReferenceVersion,
  readLocalReferencePage,
  readPythonReferencePage,
  referenceSegments,
  releaseAssetName,
} from "../../site/lib/projects/photonoxide/python-reference.js";

/** An app root with a copy of the reference: its files as { path: text }. */
function appWith(files: Record<string, string>): string {
  const root = mkdtempSync(path.join(tmpdir(), "pyref-"));
  for (const [file, text] of Object.entries(files)) {
    const full = path.join(root, PYTHON_REFERENCE_DIR, ...file.split("/"));
    mkdirSync(path.dirname(full), { recursive: true });
    writeFileSync(full, text);
  }
  return root;
}

/** The two sources over an app root, the release's answers given, and which were asked. */
function sources(root: string, release: Record<string, string>) {
  const asked: string[] = [];
  return {
    asked,
    readLocal: (segments: string[]) => readLocalReferencePage(segments, root),
    fetchRelease: async (segments: string[]) => {
      const asset = releaseAssetName(segments);
      asked.push(asset);
      return release[asset] ?? null;
    },
  };
}

const RELEASE = {
  "python-reference-photonoxide.html": "release: package",
  "python-reference-photonoxide.errors.html": "release: errors",
};

test("only the file names pdoc makes are pages", () => {
  assert.deepEqual(referenceSegments("photonoxide/errors.html"), ["photonoxide", "errors.html"]);
  assert.deepEqual(referenceSegments("index.html"), ["index.html"]);
  for (const bad of ["../photonoxide.html", "photonoxide/../../VERSION", ".hidden", "a b.html", "", "x//y"]) {
    assert.equal(referenceSegments(bad), null, bad);
  }
});

test("a release asset is the page's path with / as .", () => {
  assert.equal(releaseAssetName(["photonoxide", "errors.html"]), "python-reference-photonoxide.errors.html");
  assert.equal(releaseAssetName(["photonoxide.html"]), "python-reference-photonoxide.html");
});

test("the app's copy comes first, and the release isn't asked", async () => {
  const root = appWith({
    VERSION: "v0.6.3\n",
    "photonoxide.html": "local: package",
    "photonoxide/errors.html": "local: errors",
  });
  try {
    const s = sources(root, RELEASE);
    assert.equal(await localReferenceVersion(root), "v0.6.3");
    assert.equal(await readPythonReferencePage("photonoxide.html", s), "local: package");
    assert.equal(await readPythonReferencePage("photonoxide/errors.html", s), "local: errors");
    assert.deepEqual(s.asked, []);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("with no copy, the release's asset", async () => {
  const root = appWith({});
  try {
    const s = sources(root, RELEASE);
    assert.equal(await localReferenceVersion(root), null);
    assert.equal(await readPythonReferencePage("photonoxide/errors.html", s), "release: errors");
    assert.deepEqual(s.asked, ["python-reference-photonoxide.errors.html"]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("a copy without VERSION counts as none", async () => {
  const root = appWith({ "photonoxide.html": "local: half-written" });
  try {
    const s = sources(root, RELEASE);
    assert.equal(await readPythonReferencePage("photonoxide.html", s), "release: package");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("a page the copy doesn't have comes from the release, and neither gives null", async () => {
  const root = appWith({ VERSION: "v0.6.3", "photonoxide.html": "local: package" });
  try {
    const s = sources(root, RELEASE);
    assert.equal(await readPythonReferencePage("photonoxide/errors.html", s), "release: errors");
    assert.equal(await readPythonReferencePage("photonoxide/other.html", s), null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("a path outside the reference asks neither source", async () => {
  const root = appWith({ VERSION: "v0.6.3" });
  try {
    const s = sources(root, RELEASE);
    assert.equal(await readPythonReferencePage("../VERSION", s), null);
    assert.deepEqual(s.asked, []);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
