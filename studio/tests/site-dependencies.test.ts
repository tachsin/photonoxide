// The Dependencies page's plain functions (site/lib/projects/photonoxide/dependencies), on
// recorded answers and the repository's own files: no network. The page itself runs on
// tachsin.gr; these are the parts of it that decide what a row says.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  allows,
  caretTakes,
  compare,
  highest,
  isMajor,
  isNewer,
  numbers,
  pastTag,
  preRelease,
} from "../../site/lib/projects/photonoxide/dependencies/versions.js";
import {
  allActions,
  cargoReqTakes,
  checkedLibraries,
  crates,
  manifestCrates,
  nodeVersion,
  npmPackages,
  packageManager,
  pnpmLocked,
  pythonRequirements,
  rustVersion,
  workflowActions,
  workspaceMembers,
} from "../../site/lib/projects/photonoxide/dependencies/manifests.js";
import {
  conda,
  crateRepositories,
  githubRelease,
  githubRepo,
  highestTag,
  indexNewest,
  indexPath,
  mumpsRelease,
  NATIVE_LIBRARIES,
  newestManifest,
  nodeLts,
  npmLatest,
  nvidiaRelease,
  pypi,
  runnerMacos,
  rustStable,
} from "../../site/lib/projects/photonoxide/dependencies/registries.js";
import {
  changelogReleases,
  changelogSections,
  feedReleases,
  parseReleasesFeed,
  sanitizeHtml,
  saysBreaking,
  tagVersion,
} from "../../site/lib/projects/photonoxide/dependencies/changelog.js";
import { createReader, HOUR, OUR_FILES_TTL, REGISTRY_TTL } from "../../site/lib/projects/photonoxide/dependencies/polite.js";

const repo = (path: string) => readFileSync(fileURLToPath(new URL(`../../${path}`, import.meta.url)), "utf8");

test("versions compare by their numbers, and pre-releases are never newer", () => {
  assert.deepEqual(numbers("v0.3.34"), [0, 3, 34]);
  assert.deepEqual(numbers("2020.4.304-2ubuntu3"), [2020, 4, 304]);
  assert.deepEqual(numbers("7.0.0rc1"), [7, 0]);
  assert.equal(compare("5.10.0", "5.9.1"), 1);
  assert.equal(compare("26", "26.0"), 0);
  assert.equal(compare("12.8.6.49", "12.8.6.72"), -1);
  for (const pre of ["1.0.0-rc.1", "2.0.0b1", "3.1.dev2", "7.0.0rc1", "1.0a1"]) assert.ok(preRelease(pre), pre);
  for (const release of ["1.0.0", "v2.3", "2.0.0+build.5", "1.4.post2", "2026.1.0"]) assert.ok(!preRelease(release), release);
  assert.equal(highest(["1.2", "1.10", null, "1.9"]), "1.10");
});

test("a Python requirement takes what its specifiers take", () => {
  assert.ok(allows(">=1.15,<2", "1.20.0") && !allows(">=1.15,<2", "2.0.0"));
  assert.ok(allows("==16.0.0", "16.0.0") && !allows("==16.0.0", "16.1.0"));
  assert.ok(allows("==5.*", "5.3.1") && !allows("==5.*", "6.0"));
  assert.ok(allows("~=4.2", "4.9") && !allows("~=4.2", "5.0"));
  assert.ok(allows("~=4.2.1", "4.2.7") && !allows("~=4.2.1", "4.3.0"));
  assert.ok(allows(">=2.2.6", "3.0.0") && allows("", "1.0"));
  assert.ok(!allows("!=1.0", "1.0") && allows(">1.0", "1.0.1"));
});

test("newer and major follow each group's rule", () => {
  // a crate or npm package: newer by its numbers, major outside the caret (0.x's 0.y counts)
  assert.ok(isNewer("caret", "1.0.228", "1.0.229") && !isMajor("caret", "1.0.228", "1.0.229"));
  assert.ok(isMajor("caret", "1.0.228", "2.0.0"));
  assert.ok(isMajor("caret", "0.24.4", "0.25.0") && !isMajor("caret", "0.24.4", "0.24.9"));
  assert.ok(caretTakes("0.0.3", "0.0.3") && !caretTakes("0.0.3", "0.0.4"));
  assert.ok(!isNewer("caret", "1.0.0", "2.0.0-rc.1"));
  // a Python requirement: behind when it doesn't take the newest, and that is major
  assert.ok(!isNewer("spec", ">=2.2.6", "3.1.0"));
  assert.ok(isNewer("spec", "maturin>=1.15,<2".slice(7), "2.0.0") && isMajor("spec", ">=1.15,<2", "2.0.0"));
  // an action's tag takes every release that starts with its numbers
  assert.ok(!isNewer("tag", "v7", "v7.2.0") && isNewer("tag", "v7", "v8.0.0") && isMajor("tag", "v7", "v8.0.0"));
  assert.ok(isNewer("tag", "v1.51.0", "v1.52.0") && !isMajor("tag", "v1.51.0", "v1.52.0"));
  assert.ok(pastTag("24", "26.1.0") && !pastTag("24", "24.11.1"));
  // the native libraries and the toolchain: by their numbers
  assert.ok(isNewer("exact", "1.95", "1.99.1") && !isMajor("exact", "1.95", "1.99.1"));
  assert.ok(!isNewer("exact", "5.9.1", "5.9.1"));
});

const MANIFEST = `[package]
name = "x"
rust-version = "1.95"

[dependencies]
serde = { version = "1", features = ["derive"] }
flate2 = "1.1"
local = { path = "../local" }
renamed = { package = "real-name", version = "0.3" }
photonoxide = { path = "..", version = "0.6.1" }

[dependencies.tabled]
version = "0.20"
features = ["std"]

[target.'cfg(windows)'.dependencies]
windows-sys = "0.61"

[dev-dependencies]
genoxide = { version = "0.13.1", default-features = false }

[profile.dev]
opt-level = 3
`;

test("our Cargo manifests' crates, at Cargo.lock's versions", () => {
  assert.deepEqual(
    manifestCrates(MANIFEST).map((c) => `${c.name} ${c.req}`),
    ["serde 1", "flate2 1.1", "real-name 0.3", "photonoxide 0.6.1", "tabled 0.20", "windows-sys 0.61", "genoxide 0.13.1"],
  );
  assert.equal(rustVersion(MANIFEST), "1.95");
  const lock = `[[package]]\nname = "serde"\nversion = "1.0.228"\n\n[[package]]\nname = "windows-sys"\nversion = "0.52.0"\n\n[[package]]\nname = "windows-sys"\nversion = "0.61.2"\n`;
  const found = crates([MANIFEST], lock);
  assert.equal(found.find((c) => c.name === "serde")?.uses, "1.0.228");
  // the version the requirement takes, of two locked
  assert.equal(found.find((c) => c.name === "windows-sys")?.uses, "0.61.2");
  assert.ok(!found.some((c) => c.name.startsWith("photonoxide")));
  assert.ok(cargoReqTakes("0.24.4", "0.24.9") && !cargoReqTakes("0.24.4", "0.25.0") && cargoReqTakes(">=1, <2", "1.9"));

  // the repository's own: every member's, none of the workspace's own crates
  const root = repo("Cargo.toml");
  const members = workspaceMembers(root);
  assert.ok(members.includes("studio/src-tauri") && members.includes("native"));
  const all = crates([root, ...members.map((m) => repo(`${m}/Cargo.toml`))], repo("Cargo.lock"));
  const serde = all.find((c) => c.name === "serde");
  assert.ok(serde && serde.uses.startsWith("1.") && numbers(serde.uses).length === 3, JSON.stringify(serde));
  assert.ok(all.some((c) => c.name === "tauri"));
  assert.ok(all.every((c) => !c.name.startsWith("photonoxide")));
  assert.equal(all.filter((c) => c.name === "serde").length, 1);
  assert.ok(rustVersion(root));
});

test("the studio's packages at pnpm-lock.yaml's versions, and its pnpm", () => {
  const lock =
    "---\nlockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    configDependencies: {}\n    packageManagerDependencies:\n      pnpm:\n        specifier: 12.4.1\n        version: 12.4.1\n\n" +
    "packages:\n\n  'x@1.0.0':\n    resolution: {}\n\n---\nlockfileVersion: '9.0'\n\nimporters:\n\n  .:\n" +
    "    dependencies:\n      '@codemirror/view':\n        specifier: ^6.43.15\n        version: 6.43.15\n      svelte:\n        specifier: ^5.57.2\n        version: 5.57.3\n" +
    "    devDependencies:\n      '@sveltejs/vite-plugin-svelte':\n        specifier: ^7.3.1\n        version: 7.3.1(svelte@5.57.3)(vite@8.3.5)\n\npackages:\n\n  svelte@5.57.3:\n    version: 9.9.9\n";
  const locked = pnpmLocked(lock);
  assert.deepEqual([...locked.entries()], [
    ["@codemirror/view", "6.43.15"],
    ["svelte", "5.57.3"],
    ["@sveltejs/vite-plugin-svelte", "7.3.1"],
  ]);
  const real = npmPackages(repo("studio/package.json"), repo("studio/pnpm-lock.yaml"));
  const svelte = real.find((p) => p.name === "svelte");
  assert.ok(svelte && numbers(svelte.uses).length === 3);
  assert.ok(real.some((p) => p.name === "vite") && real.some((p) => p.name === "@tauri-apps/cli"));
  assert.deepEqual(packageManager('{"packageManager":"pnpm@12.4.1+sha512.abc"}'), { name: "pnpm", version: "12.4.1" });
  assert.equal(packageManager(repo("studio/package.json"))?.name, "pnpm");
});

test("the Python requirements as written", () => {
  const reqs = pythonRequirements(repo("python/pyproject.toml"));
  assert.equal(reqs.find((r) => r.name === "maturin")?.uses, ">=1.15,<2");
  assert.ok(reqs.some((r) => r.name === "numpy"));
  assert.deepEqual(
    pythonRequirements('[build-system]\nrequires = [\n  "a>=1",  # why\n  "b[x]==2 ; python_version<\'3.9\'",\n]\n[project]\ndependencies = ["c"]\n[project.optional-dependencies]\ntest = ["d~=4.2"]\n[tool.x]\ndependencies = ["e"]\n'),
    [
      { name: "a", uses: ">=1" },
      { name: "b", uses: "==2" },
      { name: "c", uses: "" },
      { name: "d", uses: "~=4.2" },
    ],
  );
});

test("the workflows' actions and Node.js", () => {
  const workflow = `steps:\n  - uses: actions/checkout@v7\n  - uses: ./.github/actions/x\n  - uses: dtolnay/rust-toolchain@stable\n  - uses: dtolnay/rust-toolchain@1.95\n  - uses: 'astral-sh/setup-uv@v10.3.2' # pinned\n  - uses: actions/upload-artifact@v7.0.2\n  - uses: actions/upload-artifact@v6\n    with:\n      node-version: 24\n`;
  assert.deepEqual(
    workflowActions(workflow).map((a) => `${a.name}@${a.uses}`),
    ["actions/checkout@v7", "astral-sh/setup-uv@v10.3.2", "actions/upload-artifact@v7.0.2", "actions/upload-artifact@v6"],
  );
  // one row for each major an action is used at, its highest tag
  assert.deepEqual(
    allActions([workflow, "- uses: actions/upload-artifact@v7"]).map((a) => `${a.name}@${a.uses}`),
    ["actions/checkout@v7", "actions/upload-artifact@v6", "actions/upload-artifact@v7.0.2", "astral-sh/setup-uv@v10.3.2"],
  );
  assert.equal(nodeVersion([workflow, "node-version: '22'"]), "24");
  assert.equal(nodeVersion(["nothing"]), null);
  assert.ok(nodeVersion([repo(".github/workflows/ci.yml")]));
});

test("docs/libraries.md's table gives each native library's version we use", () => {
  const table = checkedLibraries(repo("docs/libraries.md"));
  for (const lib of NATIVE_LIBRARIES) {
    const row = table.get(lib.library);
    assert.ok(row?.uses && numbers(row.uses).length > 0, `${lib.library}: ${JSON.stringify(row)}`);
    assert.ok(row.accepts, lib.library);
  }
  assert.equal(table.get("MUMPS")?.uses, "5.9.1");
  assert.equal(table.size, NATIVE_LIBRARIES.length);
});

const CONDA = {
  files: [
    { version: "5.8.1", upload_time: "2026-01-02 10:00:00.000000+00:00", labels: ["main"] },
    { version: "5.8.2", upload_time: "2026-09-21 19:27:39.958000+00:00", labels: ["main"] },
    { version: "5.8.2", upload_time: "2026-09-20 08:00:00.000000+00:00", labels: ["main"] },
    { version: "5.9.0", upload_time: "2026-10-01 08:00:00.000000+00:00", labels: ["broken"] },
    { version: "5.9.1", upload_time: "2026-10-02 08:00:00.000000+00:00", labels: ["main", "broken"] },
  ],
};

test("each registry's answer reads", () => {
  // conda-forge: the main label alone, never a build marked broken, and its first upload
  assert.deepEqual(conda(CONDA), { version: "5.8.2", date: "2026-09-20" });
  assert.deepEqual(
    pypi({
      info: { version: "2026.1.0", project_urls: { Changelog: "https://x.org/changes", Source: "https://github.com/a/b" } },
      releases: { "2026.1.0": [{ upload_time_iso_8601: "2026-07-02T15:11:10Z" }, { upload_time_iso_8601: "2026-07-01T15:11:10Z" }] },
    }),
    { version: "2026.1.0", date: "2026-07-01", notes: "https://x.org/changes", repo: "a/b" },
  );
  assert.deepEqual(
    githubRelease({ tag_name: "v0.3.34", published_at: "2026-07-16T19:58:36Z", html_url: "https://github.com/OpenMathLib/OpenBLAS/releases/tag/v0.3.34" }),
    { version: "0.3.34", tag: "v0.3.34", date: "2026-07-16", notes: "https://github.com/OpenMathLib/OpenBLAS/releases/tag/v0.3.34" },
  );
  assert.equal(githubRelease({ tag_name: "hdf5_1.14.6" })?.version, "1.14.6");
  assert.equal(githubRelease({ tag_name: "v7" })?.version, "7");
  assert.equal(highestTag([{ name: "v1.2.0" }, { name: "v1.10.0" }, { name: "v2.0.0-rc.1" }, { name: "latest" }]), "v1.10.0");
  assert.equal(
    newestManifest("<a href='redistrib_13.4.1.json'>redistrib_13.4.1.json</a>\n<a href='redistrib_13.10.0.json'>x</a><a href='redistrib_13.4.2.json'>x</a>"),
    "13.10.0",
  );
  assert.equal(newestManifest("nothing here"), null);
  assert.deepEqual(
    nvidiaRelease({ release_date: "2026-09-16", release_label: "13.4.2", cuda_cudart: { version: "13.4.92" } }, "cuda_cudart", "CUDA"),
    { version: "13.4.92", date: "2026-09-16", label: "CUDA 13.4.2" },
  );
  assert.deepEqual(
    mumpsRelease(
      "<p>Latest release (September 2026) :</p>\n<table summary=\"\">\n<tr><td>Release 5.10.0</td><td>: September 2026\n</td></tr>\n<tr><td>Release 5.9.1</td><td>: July 2026</td></tr>\n</table>",
    ),
    { version: "5.10.0", date: "2026-09" },
  );
  assert.deepEqual(
    runnerMacos("# macOS 27\n- OS Version: macOS 27.0.1 (26A434)\n- Kernel Version: Darwin 27.0.0\n- Image Version: 20261006.0244.1\n"),
    { version: "27.0.1", date: "2026-10-06" },
  );
  assert.deepEqual(
    indexNewest(
      '{"name":"serde","vers":"1.0.227","yanked":false,"pubtime":"2026-01-01T00:00:00Z"}\n{"name":"serde","vers":"1.0.229","yanked":true}\n{"name":"serde","vers":"2.0.0-rc.1","yanked":false}\n{"name":"serde","vers":"1.0.228","yanked":false,"pubtime":"2026-07-18T23:05:14Z"}\n',
    ),
    { version: "1.0.228", date: "2026-07-18" },
  );
  assert.deepEqual(["a", "ab", "abc", "serde", "Tauri"].map(indexPath), ["1/a", "2/ab", "3/a/abc", "se/rd/serde", "ta/ur/tauri"]);
  assert.deepEqual(npmLatest({ version: "2.12.4", repository: { url: "git+https://github.com/tauri-apps/tauri.git", directory: "packages/api" } }), {
    version: "2.12.4",
    repo: "tauri-apps/tauri",
    directory: "packages/api",
  });
  assert.equal(githubRepo("github:sveltejs/svelte"), "sveltejs/svelte");
  assert.equal(githubRepo("https://github.com/tauri-apps/tauri/tree/dev/crates/tauri"), "tauri-apps/tauri");
  assert.equal(githubRepo("https://gitlab.com/x/y"), null);
  assert.deepEqual(
    [...crateRepositories({ crates: [{ name: "rayon", repository: "https://github.com/rayon-rs/rayon" }, { name: "x", repository: null }] })],
    [
      ["rayon", "rayon-rs/rayon"],
      ["x", null],
    ],
  );
  assert.deepEqual(
    rustStable('manifest-version = "2"\ndate = "2026-10-15"\n\n[pkg.cargo]\nversion = "0.100.0 (5f94 2026-10-01)"\n[pkg.rust]\nversion = "1.99.1 (58cf8f9c3 2026-10-13)"\n'),
    { version: "1.99.1", date: "2026-10-15" },
  );
  assert.deepEqual(
    nodeLts([
      { version: "v27.0.0-alpha.0", date: "2026-10-29", lts: false },
      { version: "v26.1.0", date: "2026-10-28", lts: "Peach" },
      { version: "v24.11.0", date: "2026-10-20", lts: "Krypton" },
    ]),
    { version: "26.1.0", date: "2026-10-28", lts: "Peach" },
  );
});

const FEED = `<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <updated>2026-10-02T00:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/o/r/releases/tag/v2.0.0"/>
    <title>v2.0.0</title>
    <content type="html">&lt;h2&gt;BREAKING&lt;/h2&gt;&lt;ul&gt;&lt;li&gt;removed &lt;code&gt;x&lt;/code&gt; &amp;amp; y&lt;/li&gt;&lt;/ul&gt;&lt;script&gt;alert(1)&lt;/script&gt;</content>
  </entry>
  <entry>
    <updated>2026-09-01T00:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/o/r/releases/tag/v2.0.0-rc.1"/>
    <title>v2.0.0-rc.1</title>
    <content type="html">&lt;p&gt;a pre-release&lt;/p&gt;</content>
  </entry>
  <entry>
    <updated>2026-08-01T00:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/o/r/releases/tag/v1.3.0"/>
    <title>One point three</title>
    <content type="html">&lt;p onclick="x()"&gt;See &lt;a href="javascript:alert(1)"&gt;this&lt;/a&gt; and &lt;a href="/o/r/pull/7" class="x"&gt;#7&lt;/a&gt;&lt;img src=x onerror=alert(1)&gt;&lt;/p&gt;</content>
  </entry>
  <entry>
    <updated>2026-07-01T00:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/o/r/releases/tag/other-v1.4.0"/>
    <title>other v1.4.0</title>
    <content type="html">&lt;p&gt;another package's&lt;/p&gt;</content>
  </entry>
  <entry>
    <updated>2026-06-01T00:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/o/r/releases/tag/v1.2.0"/>
    <title>v1.2.0</title>
    <content type="html">&lt;p&gt;ours&lt;/p&gt;</content>
  </entry>
</feed>`;

test("a releases feed's notes between our version and the newest, made safe", () => {
  const entries = parseReleasesFeed(FEED);
  assert.equal(entries.length, 5);
  assert.deepEqual(
    entries.map((e) => [e.tag, e.date]),
    [
      ["v2.0.0", "2026-10-02"],
      ["v2.0.0-rc.1", "2026-09-01"],
      ["v1.3.0", "2026-08-01"],
      ["other-v1.4.0", "2026-07-01"],
      ["v1.2.0", "2026-06-01"],
    ],
  );
  const row = {
    name: "r",
    newest: "2.0.0",
    isNewer: (v: string) => isNewer("caret", "1.2.0", v),
    isMajor: (v: string) => isMajor("caret", "1.2.0", v),
  };
  const releases = feedReleases(entries, row);
  // ours and the pre-release left out, another package's tag too, newest first
  assert.deepEqual(
    releases.map((r) => [r.version, r.breaking]),
    [
      ["2.0.0", true],
      ["1.3.0", false],
    ],
  );
  assert.equal(releases[0].html, "<h4>BREAKING</h4><ul><li>removed <code>x</code> &amp; y</li></ul>");
  // no script, no event handler, no javascript: address, no image; GitHub's own links absolute
  assert.equal(
    releases[1].html,
    'See <a>this</a> and <a href="https://github.com/o/r/pull/7" target="_blank" rel="noopener noreferrer nofollow">#7</a>'.replace(/^/, "<p>") + "</p>",
  );
  assert.equal(tagVersion("tauri-v2.9.0", "tauri"), "2.9.0");
  assert.equal(tagVersion("tauri-build-v2.9.0", "tauri"), null);
  assert.equal(tagVersion("@tauri-apps/api-v2.9.0", "@tauri-apps/api"), "2.9.0");
  assert.equal(tagVersion("hdf5_1.14.6", "HDF5"), "1.14.6");
  assert.equal(tagVersion("1.99.1", "rust"), "1.99.1");
});

test("sanitised notes keep no markup of their own and are cut at an element's end", () => {
  const evil = '<div><p>a</p><style>p{}</style><iframe src="x">y</iframe><p>b <b>c</b> &copy; 1 < 2 & 3</p><a href=\'https://x.org/"onmouseover="z\'>l</a></div>';
  const { html } = sanitizeHtml(evil);
  assert.equal(html, '<p>a</p><p>b <strong>c</strong> &copy; 1 &lt; 2 &amp; 3</p><a href="https://x.org/&quot;onmouseover=&quot;z" target="_blank" rel="noopener noreferrer nofollow">l</a>');
  const long = `<ul>${"<li>a line of notes</li>".repeat(100)}</ul><p>after</p>`;
  const cut = sanitizeHtml(long, 200);
  assert.ok(cut.cut && cut.html.endsWith("</li></ul>") && !cut.html.includes("after") && cut.html.length < 400, cut.html);
  assert.ok(saysBreaking("**Breaking change:** x") && saysBreaking("BREAKING: y") && !saysBreaking("fixes a break"));
});

test("a CHANGELOG's sections between our version and the newest", () => {
  const md = `# Changelog\n\n## [Unreleased]\n\n- soon\n\n## [1.4.0] - 2026-10-01\n\n### Added\n\n- a thing\n\n## v1.3.1 (2026-09-02)\n\n- a fix\n\n## 1.3.0\n\n- ours\n\nVersion 1.2.0 (2026-01-01)\n==========\n`;
  const sections = changelogSections(md);
  assert.deepEqual(
    sections.map((s) => [s.version, s.date]),
    [
      ["1.4.0", "2026-10-01"],
      ["1.3.1", "2026-09-02"],
      ["1.3.0", null],
    ],
  );
  assert.equal(sections[0].markdown, "### Added\n\n- a thing");
  const releases = changelogReleases(sections, {
    name: "x",
    newest: "1.4.0",
    isNewer: (v: string) => isNewer("caret", "1.3.0", v),
    isMajor: (v: string) => isMajor("caret", "1.3.0", v),
  });
  assert.deepEqual(
    releases.map((r) => r.version),
    ["1.4.0", "1.3.1"],
  );
  // Rust's RELEASES.md names its releases "Version 1.91.0 (2025-10-30)"
  assert.deepEqual(
    changelogSections("Version 1.99.1 (2026-10-13)\n===\n\n- a\n\nVersion 1.99.0 (2026-09-30)\n===\n").map((s) => s.version),
    [],
  );
  assert.deepEqual(
    changelogSections("# Version 1.99.1 (2026-10-13)\n\n- a\n\n# Version 1.99.0 (2026-09-30)\n\n- b\n").map((s) => [s.version, s.date]),
    [
      ["1.99.1", "2026-10-13"],
      ["1.99.0", "2026-09-30"],
    ],
  );
});

/** A fetch that answers from a list and records what it was asked. */
function recorded(answers: Record<string, () => { status: number; body?: string; headers?: Record<string, string> }>) {
  const asked: { url: string; headers: Record<string, string> }[] = [];
  const fetch = async (url: string, init: { headers: Record<string, string> }) => {
    asked.push({ url, headers: init.headers });
    const answer = answers[url];
    if (!answer) throw new Error("unreachable");
    const a = answer();
    return new Response(a.status === 304 ? null : (a.body ?? ""), { status: a.status, headers: a.headers ?? {} });
  };
  return { fetch, asked };
}

test("the reader asks each registry once in fifteen minutes, conditionally, and keeps the last answer through a failure", async () => {
  let clock = 1_000_000;
  let status = 200;
  const { fetch, asked } = recorded({
    "https://r.example/a": () => (status === 200 ? { status, body: '{"v":1}', headers: { etag: '"e1"' } } : { status }),
    "https://r.example/missing": () => ({ status: 404 }),
  });
  const reader = createReader({ fetch: fetch as unknown as typeof globalThis.fetch, now: () => clock, userAgent: "test (+https://example.org)", background: false });
  const parse = (t: string) => JSON.parse(t).v;

  const first = await reader.read("https://r.example/a", { parse });
  assert.deepEqual(first, { value: 1, readAt: 1_000_000, failedAt: null, error: null });
  assert.equal(asked[0].headers["User-Agent"], "test (+https://example.org)");
  assert.ok(!/@/.test(asked[0].headers["User-Agent"]), "no email in the User-Agent");

  // within its time: no request
  clock += REGISTRY_TTL - 1;
  await reader.read("https://r.example/a", { parse });
  assert.equal(asked.length, 1);

  // after it: a conditional request, and a 304 keeps the value
  clock += 2;
  status = 304;
  const again = await reader.read("https://r.example/a", { parse });
  assert.equal(asked.length, 2);
  assert.equal(asked[1].headers["If-None-Match"], '"e1"');
  assert.deepEqual([again.value, again.readAt, again.error], [1, clock, null]);

  // a failure keeps the last answer and says so; it is tried again after ten minutes, not before
  clock += REGISTRY_TTL;
  status = 503;
  const failed = await reader.read("https://r.example/a", { parse });
  assert.deepEqual([failed.value, failed.readAt, failed.failedAt, failed.error], [1, clock - REGISTRY_TTL, clock, "HTTP 503"]);
  clock += 60_000;
  await reader.read("https://r.example/a", { parse });
  assert.equal(asked.length, 3);
  clock += 600_000;
  status = 200;
  const back = await reader.read("https://r.example/a", { parse });
  assert.equal(asked.length, 4);
  assert.deepEqual([back.value, back.error], [1, null]);

  // a file that isn't there, when that is allowed, is null and not a failure
  const missing = await reader.read("https://r.example/missing", { missing: true });
  assert.deepEqual([missing.value, missing.error], [null, null]);
  // an unreachable host, never read: no value, and why
  const never = await reader.read("https://r.example/nowhere");
  assert.deepEqual([never.value, never.error], [undefined, "unreachable"]);
});

test("a stale answer is served at once and refreshed behind it", async () => {
  let clock = 0;
  let v = 1;
  const { fetch, asked } = recorded({ "https://r.example/b": () => ({ status: 200, body: String(v) }) });
  const reader = createReader({ fetch: fetch as unknown as typeof globalThis.fetch, now: () => clock, userAgent: "test" });
  assert.equal((await reader.read("https://r.example/b", { parse: Number })).value, 1);
  clock += REGISTRY_TTL + 1;
  v = 2;
  assert.equal((await reader.read("https://r.example/b", { parse: Number })).value, 1);
  await reader.idle();
  assert.equal(asked.length, 2);
  assert.equal((await reader.read("https://r.example/b", { parse: Number })).value, 2);
  // a computation is kept by its key
  let runs = 0;
  const memo = () => reader.memo("k", HOUR, async () => ++runs);
  assert.equal(await memo(), 1);
  assert.equal(await memo(), 1);
});

test("our own files are kept five minutes, the registries fifteen, and a release's notes longer", async () => {
  assert.equal(OUR_FILES_TTL, 5 * 60_000);
  assert.equal(REGISTRY_TTL, 15 * 60_000);
  let clock = 0;
  let version = 1;
  const { fetch, asked } = recorded({
    "https://raw.example/Cargo.lock": () => ({ status: 200, body: String(version) }),
    "https://registry.example/latest": () => ({ status: 200, body: String(version) }),
  });
  const reader = createReader({ fetch: fetch as unknown as typeof globalThis.fetch, now: () => clock, userAgent: "test", background: false });
  const ours = () => reader.read("https://raw.example/Cargo.lock", { ttl: OUR_FILES_TTL, parse: Number });
  const registry = () => reader.read("https://registry.example/latest", { parse: Number });
  const requests = (host: string) => asked.filter((a) => a.url.startsWith(host)).length;

  assert.deepEqual([(await ours()).value, (await registry()).value], [1, 1]);
  // a bump of ours is seen after five minutes; the registry's answer is still the kept one
  version = 2;
  clock += OUR_FILES_TTL - 1;
  assert.equal((await ours()).value, 1);
  clock += 1;
  assert.deepEqual([(await ours()).value, (await registry()).value], [2, 1]);
  assert.deepEqual([requests("https://raw.example"), requests("https://registry.example")], [2, 1]);
  // another five minutes on, our files are asked again and the registry, read ten minutes ago, is not
  clock += OUR_FILES_TTL;
  assert.deepEqual([(await ours()).value, (await registry()).value], [2, 1]);
  assert.deepEqual([requests("https://raw.example"), requests("https://registry.example")], [3, 1]);
  // and the registry is asked once fifteen minutes have passed since its read
  clock = REGISTRY_TTL;
  assert.equal((await registry()).value, 2);
  assert.equal(requests("https://registry.example"), 2);

  // a failed read of one of our files is tried again after its own five minutes (a registry's after ten)
  let up = false;
  const flaky = recorded({ "https://raw.example/f": () => (up ? { status: 200, body: "7" } : { status: 503 }) });
  const second = createReader({ fetch: flaky.fetch as unknown as typeof globalThis.fetch, now: () => clock, userAgent: "test", background: false });
  const file = () => second.read("https://raw.example/f", { ttl: OUR_FILES_TTL, parse: Number });
  const failed = await file();
  assert.deepEqual([failed.value, failed.failedAt, failed.error], [undefined, clock, "HTTP 503"]);
  clock += OUR_FILES_TTL - 1;
  await file();
  assert.equal(flaky.asked.length, 1);
  clock += 1;
  up = true;
  assert.deepEqual([(await file()).value, (await file()).error], [7, null]);
  assert.equal(flaky.asked.length, 2);

  // a computed value (a release's notes) is kept by its own, longer time
  let runs = 0;
  const notes = () => reader.memo("changelog", HOUR * 24, async () => ++runs);
  assert.equal(await notes(), 1);
  clock += HOUR * 23;
  assert.equal(await notes(), 1);
  clock += HOUR * 2;
  assert.equal(await notes(), 2);
});
