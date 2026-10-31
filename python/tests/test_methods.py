"""The Python calls in the method write-ups (docs/methods): each page's ``python`` blocks run, in
order, as one script, so a call the package doesn't have, or one whose arguments changed, fails
here. A page that uses another's names (compact.md the MZI of circuits.md) runs after it, in the
same namespace."""

from __future__ import annotations

import re
import shutil
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
METHODS = ROOT / "docs" / "methods"
# each page with Python calls, and the page whose names it uses, if any
PAGES = {
    "catalogue": None,
    "circuits": None,
    "compact": "circuits",
    "devices": None,
    "export": None,
    "fabrication": None,
    "fdfd-ports": None,
    "nonlinear": None,
    "slab": None,
    "stacks": None,
    "vector": None,
    "fields": "vector",
}
PYTHON = re.compile(r"^```python\n(.*?)^```", re.MULTILINE | re.DOTALL)


def blocks(page: str) -> list[str]:
    return PYTHON.findall((METHODS / f"{page}.md").read_text(encoding="utf-8"))


def test_every_page_with_python_calls_is_run() -> None:
    pages = sorted(p.stem for p in METHODS.glob("*.md") if blocks(p.stem))
    assert pages == sorted(PAGES)


def test_a_change_to_a_page_with_python_calls_runs_them() -> None:
    # the wheels' workflow starts on its paths: each such page among them, so a pull request that
    # changes only the page still runs its calls here
    workflow = (ROOT / ".github" / "workflows" / "python-wheels.yml").read_text(encoding="utf-8")
    missing = [p for p in sorted(PAGES) if f'- "docs/methods/{p}.md"' not in workflow]
    assert missing == [], f"python-wheels.yml's paths don't name {missing}"


@pytest.mark.parametrize("page", sorted(PAGES))
def test_a_pages_python_calls_run(page: str, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # the calls name the repository's job files by their paths from its root, and write their
    # files (runs, Touchstone) where they are run: a copy of the jobs in a folder of their own
    shutil.copytree(ROOT / "jobs", tmp_path / "jobs")
    monkeypatch.chdir(tmp_path)
    chain = [page]
    while PAGES[chain[0]] is not None:
        chain.insert(0, PAGES[chain[0]])
    namespace: dict[str, object] = {"__name__": f"methods_{page.replace('-', '_')}"}
    for name in chain:
        for k, code in enumerate(blocks(name)):
            exec(compile(code, f"docs/methods/{name}.md, python block {k + 1}", "exec"), namespace)

def test_the_design_pages_python_calls_run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # docs/design.md shows each design as a job file and as Python calls: its blocks run in
    # order, as one script, on a copy of the jobs, as a write-up's do
    page = ROOT / "docs" / "design.md"
    code = PYTHON.findall(page.read_text(encoding="utf-8"))
    assert len(code) >= 4, len(code)
    workflow = (ROOT / ".github" / "workflows" / "python-wheels.yml").read_text(encoding="utf-8")
    assert '- "docs/design.md"' in workflow, "python-wheels.yml's paths don't name docs/design.md"
    shutil.copytree(ROOT / "jobs", tmp_path / "jobs")
    monkeypatch.chdir(tmp_path)
    namespace: dict[str, object] = {"__name__": "design"}
    for k, block in enumerate(code):
        exec(compile(block, f"docs/design.md, python block {k + 1}", "exec"), namespace)
