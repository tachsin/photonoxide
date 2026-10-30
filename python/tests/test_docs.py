"""The API reference stays complete: every public item of the package has a docstring (the
reference, made by pdoc in CI, shows exactly these), and every example in them runs and prints
what the docstring says it does."""

from __future__ import annotations

import ast
import doctest
import inspect
import types
from pathlib import Path

import numpy as np
import pytest

import photonoxide as phox
import photonoxide.errors as errors

MODULES = (phox, errors)


def _declared(module: types.ModuleType) -> list[str]:
    tree = ast.parse(inspect.getsource(module))
    exported = {
        elt.value
        for node in tree.body
        if isinstance(node, ast.Assign)
        and any(isinstance(t, ast.Name) and t.id == "__all__" for t in node.targets)
        for elt in getattr(node.value, "elts", [])
        if isinstance(elt, ast.Constant)
    }

    def public(name: str) -> bool:
        return not name.startswith("_") or name in exported

    def named(node: ast.stmt) -> str | None:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            return node.name
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            return node.target.id
        if isinstance(node, ast.Assign) and len(node.targets) == 1:
            target = node.targets[0]
            return target.id if isinstance(target, ast.Name) else None
        return None

    def has_docstring(node: ast.stmt, following: ast.stmt | None) -> bool:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            return bool((ast.get_docstring(node) or "").strip())
        # a variable's docstring is the string literal right after it, as pdoc reads it
        return (
            isinstance(following, ast.Expr)
            and isinstance(following.value, ast.Constant)
            and isinstance(following.value.value, str)
            and bool(following.value.value.strip())
        )

    missing: list[str] = []

    def check(body: list[ast.stmt], prefix: str, in_class: bool) -> None:
        for node, following in zip(body, [*body[1:], None]):
            name = named(node)
            if name is None or not public(name):
                continue
            if in_class and name.startswith("__"):
                continue
            qualified = f"{module.__name__}.{prefix}{name}"
            if not has_docstring(node, following):
                missing.append(qualified)
            if isinstance(node, ast.ClassDef):
                check(node.body, f"{prefix}{name}.", True)

    check(tree.body, "", False)
    # the names `__all__` promises must exist, and be defined here or be a module
    for name in sorted(exported):
        assert hasattr(module, name), f"{module.__name__}.__all__ lists {name}, which isn't there"
    return missing


@pytest.mark.parametrize("module", MODULES, ids=lambda m: m.__name__)
def test_every_public_item_has_a_docstring(module: types.ModuleType) -> None:
    missing = _declared(module)
    assert not missing, "no docstring (the API reference would show them bare): " + ", ".join(
        missing
    )


def test_the_modules_have_docstrings() -> None:
    for module in MODULES:
        assert (module.__doc__ or "").strip(), module.__name__


def test_every_function_the_package_exports_is_in_the_reference() -> None:
    # a public function or class defined in the package but left out of `__all__` would be in the
    # reference without being promised: keep the two lists the same
    exported = set(phox.__all__)
    defined = {
        name
        for name, obj in vars(phox).items()
        if not name.startswith("_")
        and (inspect.isfunction(obj) or inspect.isclass(obj))
        and getattr(obj, "__module__", None) == phox.__name__
    }
    assert defined <= exported, sorted(defined - exported)


def test_the_docstring_examples_run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # the examples run in file order and share their variables (`run`, the first job's run, is
    # `run_tables`'s and `export_run`'s), in a folder of their own: they write files
    monkeypatch.chdir(tmp_path)
    flags = doctest.ELLIPSIS | doctest.NORMALIZE_WHITESPACE
    finder = doctest.DocTestFinder()
    tests = [
        test
        for module in MODULES
        for test in finder.find(module, module.__name__, globs={})
        if test.examples
    ]
    assert len(tests) >= 10, [t.name for t in tests]
    tests.sort(key=lambda t: (t.filename or "", t.lineno if t.lineno is not None else -1))
    shared: dict[str, object] = {"phox": phox, "np": np}
    report: list[str] = []
    runner = doctest.DocTestRunner(optionflags=flags, verbose=False)
    for test in tests:
        test.globs = shared
        runner.run(test, out=report.append, clear_globs=False)
    assert runner.failures == 0, "".join(report)
