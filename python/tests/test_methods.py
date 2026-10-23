"""The Python calls in the method write-ups (docs/methods): each page's ``python`` blocks run, in
order, as one script, so a call the package doesn't have, or one whose arguments changed, fails
here. A page that uses another's names (compact.md the MZI of circuits.md) runs after it, in the
same namespace. MATLAB's blocks beside them aren't run: MATLAB isn't run in CI."""

from __future__ import annotations

import re
import shutil
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
METHODS = ROOT / "docs" / "methods"
# each page with Python calls, and the page whose names it uses, if any
PAGES = {
    "circuits": None,
    "compact": "circuits",
    "devices": None,
    "export": None,
    "fdfd-ports": None,
    "vector": None,
    "fields": "vector",
}
PYTHON = re.compile(r"^```python\n(.*?)^```", re.MULTILINE | re.DOTALL)


def blocks(page: str) -> list[str]:
    return PYTHON.findall((METHODS / f"{page}.md").read_text(encoding="utf-8"))


def test_every_page_with_python_calls_is_run() -> None:
    pages = sorted(p.stem for p in METHODS.glob("*.md") if blocks(p.stem))
    assert pages == sorted(PAGES)


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
