#!/usr/bin/env python3
"""The public API of the `photonoxide` crate as its users see it, one item a line, sorted.

    python3 .github/scripts/public-api.py WORKSPACE > listing.txt

CI's Public API job lists the pull request's base and its head with it and compares the two
(issue #508, decision D3). It needs cargo-public-api and the nightly that writes the rustdoc
JSON it reads, named by PUBLIC_API_NIGHTLY (the job pins both).

cargo-public-api lists one crate. An item that `photonoxide` re-exports from another crate of
the workspace (`crates/<name>/`) is not in photonoxide's rustdoc JSON, only the `pub use`
naming it, so the tool would show a module moved into a crate as one `pub use` line and all its
items gone. This script lists every crate of `crates/` too, and rewrites their items' paths to
the paths photonoxide re-exports them at, as rustdoc's JSON records the re-exports
(`photonoxide_core::units::Length` becomes `photonoxide::units::Length` when photonoxide has
`pub use photonoxide_core::units`). A path reachable at several places in photonoxide gives
the line once for each; an item of a crate that photonoxide doesn't re-export keeps its own
crate's path. Every line of the base is then expected at the head, and a moved item, still
re-exported at its old path, shows no change.
"""

import itertools
import json
import os
import re
import subprocess
import sys
from pathlib import Path

FACADE = "photonoxide"
# a line naming paths with many places each is listed this many times at most
MAX_COPIES = 256


def run(args, cwd):
    return subprocess.run(args, cwd=cwd, check=True, stdout=subprocess.PIPE, text=True).stdout


def rustdoc_json(workspace, nightly, package, lib_name, target_dir):
    """Builds the package's rustdoc JSON, all features on, and returns it with its file."""
    subprocess.run(
        ["cargo", f"+{nightly}", "rustdoc", "--manifest-path", str(workspace / "Cargo.toml"),
         "--package", package, "--lib", "--all-features", "--",
         "-Z", "unstable-options", "--output-format", "json"],
        cwd=workspace, check=True, stdout=sys.stderr)
    path = Path(target_dir) / "doc" / f"{lib_name}.json"
    with open(path, encoding="utf-8") as f:
        return json.load(f), path


def listing(workspace, path):
    """cargo-public-api's lines for one rustdoc JSON, the blanket impls left out (-s)."""
    out = run(["cargo", "public-api", "--simplified", "--rustdoc-json", str(path)], workspace)
    return [line for line in out.splitlines() if line.strip()]


def reexports(doc, internal):
    """(path in the workspace crate, path here) for every `pub use` of a workspace crate's item."""
    index, paths, crates = doc["index"], doc["paths"], doc["external_crates"]
    found = []
    seen = set()

    def walk(module_id, here, depth):
        if depth > 32 or (module_id, here) in seen:
            return
        seen.add((module_id, here))
        module = index.get(str(module_id))
        if module is None or "module" not in module["inner"]:
            return
        for child in module["inner"]["module"]["items"]:
            item = index.get(str(child))
            if item is None:
                continue
            inner = item["inner"]
            if "module" in inner:
                walk(child, f"{here}::{item['name']}", depth + 1)
                continue
            use = inner.get("use")
            if use is None or use.get("id") is None:
                continue
            target = use["id"]
            at = here if use["is_glob"] else f"{here}::{use['name']}"
            if str(target) in index:
                # a re-export of this crate's own module: its items are reachable there too
                walk(target, at, depth + 1)
                continue
            source = paths.get(str(target))
            if source is None or source["crate_id"] == 0:
                continue
            if crates.get(str(source["crate_id"]), {}).get("name") in internal:
                found.append(("::".join(source["path"]), at))

    walk(doc["root"], index[str(doc["root"])]["name"], 0)
    return found


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    workspace = Path(sys.argv[1]).resolve()
    nightly = os.environ["PUBLIC_API_NIGHTLY"]
    metadata = json.loads(run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1",
         "--manifest-path", str(workspace / "Cargo.toml")], workspace))
    target_dir = metadata["target_directory"]
    crates_dir = (workspace / "crates").resolve()

    def lib_of(package):
        return next((t["name"] for t in package["targets"] if "lib" in t["kind"]), None)

    packages = []
    for package in metadata["packages"]:
        lib = lib_of(package)
        if lib is None:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        if package["name"] == FACADE or crates_dir in manifest.parents:
            packages.append((package["name"], lib))
    if FACADE not in [name for name, _ in packages]:
        sys.exit(f"no package {FACADE} in {workspace}")
    internal = {lib for name, lib in packages if name != FACADE}

    lines, rules = [], []
    for name, lib in packages:
        doc, path = rustdoc_json(workspace, nightly, name, lib, target_dir)
        lines += listing(workspace, path)
        rules += reexports(doc, internal)
    print(f"{len(packages)} crates, {len(rules)} re-exports from workspace crates", file=sys.stderr)

    def places(path, depth=0):
        """Every path the item at `path` is reachable at, through the re-exports."""
        out = set()
        if depth < 16:
            for source, at in rules:
                if path == source or path.startswith(source + "::"):
                    out |= places(at + path[len(source):], depth + 1)
        return out or {path}

    result = set()
    if internal:
        token = re.compile(
            r"(?<![A-Za-z0-9_])(?:" + "|".join(sorted(map(re.escape, internal), key=len, reverse=True))
            + r")(?![A-Za-z0-9_])(?:::[A-Za-z_][A-Za-z0-9_]*)*")
        for line in lines:
            parts = token.split(line)
            found = token.findall(line)
            if not found:
                result.add(line)
                continue
            choices = [sorted(places(p)) for p in found]
            for n, combo in enumerate(itertools.product(*choices)):
                if n == MAX_COPIES:
                    print(f"more than {MAX_COPIES} places for: {line}", file=sys.stderr)
                    break
                result.add("".join(itertools.chain.from_iterable(
                    itertools.zip_longest(parts, combo, fillvalue=""))))
    else:
        result.update(lines)
    sys.stdout.write("".join(line + "\n" for line in sorted(result)))


if __name__ == "__main__":
    main()
