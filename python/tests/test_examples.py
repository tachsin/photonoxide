"""The Python examples (python/examples): each prints its setup first (its layers, materials and
grid, as the Rust examples' setup block does: examples/common/setup.rs), reproduces its paper's
numbers and exits 0, and prints exactly what is recorded in python/examples/output/<name>.txt,
as the Rust examples' output is recorded in examples/output. One with a Rust twin in examples/
prints what the twin prints, line for line, after the setup block (and slab_soi, which needs no
catalogue, the setup block too)."""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
EXAMPLES = ROOT / "python" / "examples"
# the examples whose Rust twin computes the same numbers the same way
TWINS = ("silicon_index", "slab_soi")
# the twins whose setup block is the Rust example's too
SAME_SETUP = ("slab_soi",)


def examples() -> list[str]:
    return sorted(p.stem for p in EXAMPLES.glob("*.py") if not p.name.startswith("_"))


def test_the_examples_are_the_readmes() -> None:
    readme = (EXAMPLES / "README.md").read_text(encoding="utf-8")
    listed = sorted(re.findall(r"^\| \[`([a-z_]+)`\]", readme, re.MULTILINE))
    assert listed == examples()
    assert set(TWINS) <= set(examples())
    assert set(SAME_SETUP) <= set(TWINS)


def test_every_example_has_its_output_recorded() -> None:
    recorded = sorted(p.stem for p in (EXAMPLES / "output").glob("*.txt"))
    assert recorded == examples()


@pytest.mark.parametrize("name", examples())
def test_an_example_reproduces_its_paper(name: str) -> None:
    run = subprocess.run(
        [sys.executable, str(EXAMPLES / f"{name}.py")],
        capture_output=True,
        encoding="utf-8",
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        timeout=1800,
        check=False,
    )
    assert run.returncode == 0, run.stdout + run.stderr
    out = run.stdout.replace("\r\n", "\n")
    assert re.search(r"^all \d+ within tolerance of the paper$", out, re.MULTILINE), out
    # the setup block first, up to and including its blank line, as the Rust examples print it
    setup, blank, computed = out.partition("\n\n")
    assert setup.startswith("Setup\n") and blank, out
    recorded = (EXAMPLES / "output" / f"{name}.txt").read_text(encoding="utf-8")
    assert out == recorded.replace("\r\n", "\n"), (
        f"python/examples/output/{name}.txt is not what the example prints now:\n{out}"
    )
    if name in TWINS:
        rust = (ROOT / "examples" / "output" / f"{name}.txt").read_text(encoding="utf-8")
        rust = rust.replace("\r\n", "\n")
        rust_setup, rust_blank, rest = rust.partition("\n\n")
        assert rust_setup.startswith("Setup\n") and rust_blank, rust
        assert computed == rest
        if name in SAME_SETUP:
            assert setup == rust_setup
