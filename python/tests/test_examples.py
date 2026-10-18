"""The Python examples (python/examples): each reproduces its paper's numbers and exits 0, and
one with a Rust twin in examples/ prints what the twin prints (examples/output), line for line."""

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


def examples() -> list[str]:
    return sorted(p.stem for p in EXAMPLES.glob("*.py") if not p.name.startswith("_"))


def test_the_examples_are_the_readmes() -> None:
    readme = (EXAMPLES / "README.md").read_text(encoding="utf-8")
    listed = sorted(re.findall(r"^\| \[`([a-z_]+)`\]", readme, re.MULTILINE))
    assert listed == examples()
    assert set(TWINS) <= set(examples())


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
    if name in TWINS:
        rust = (ROOT / "examples" / "output" / f"{name}.txt").read_text(encoding="utf-8")
        assert out == rust.replace("\r\n", "\n")
