"""Python gives the Rust library's results to the bit.

photonoxide's façade lists calls with their results (``photonoxide::facade::conformance``),
computed here, on this machine, by the Rust code the package wraps; its Rust tests check that
those results are the library's own calls' to the bit. Each call is made again through the
package's Python functions, and every number compared by its bits: the package adds nothing
to them.
"""

from __future__ import annotations

import dataclasses
import json
import os
from pathlib import Path
from typing import Any

import numpy as np
import pytest

import photonoxide as po
from photonoxide import _photonoxide


@pytest.fixture(scope="module")
def scratch(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return tmp_path_factory.mktemp("conformance")


@pytest.fixture(scope="module")
def cases(scratch: Path) -> list[dict[str, Any]]:
    return _photonoxide._conformance(str(scratch))


def plain(x: Any) -> Any:
    """A result in the cases' plain terms: objects of fields, lists, [re, im] for a complex."""
    if dataclasses.is_dataclass(x) and not isinstance(x, type):
        return {f.name: plain(getattr(x, f.name)) for f in dataclasses.fields(x)}
    if isinstance(x, np.ndarray):
        return plain(x.tolist())
    if isinstance(x, np.generic):
        return plain(x.item())
    if isinstance(x, dict):
        return {k: plain(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [plain(v) for v in x]
    if isinstance(x, complex):
        return [x.real, x.imag]
    if isinstance(x, os.PathLike):
        return os.fspath(x)
    return x


def same(ours: Any, rusts: Any, ignore: set[str], where: str) -> None:
    """Asserts two plain results are the same, every float to the bit."""
    if isinstance(rusts, dict):
        assert isinstance(ours, dict), where
        assert ours.keys() - ignore == rusts.keys() - ignore, where
        for key in rusts.keys() - ignore:
            same(ours[key], rusts[key], ignore, f"{where}.{key}")
    elif isinstance(rusts, list):
        assert isinstance(ours, list) and len(ours) == len(rusts), where
        for k, (a, b) in enumerate(zip(ours, rusts)):
            same(a, b, ignore, f"{where}[{k}]")
    elif isinstance(rusts, float) or isinstance(ours, float):
        assert isinstance(ours, (int, float)) and isinstance(rusts, (int, float)), where
        assert float(ours).hex() == float(rusts).hex(), f"{where}: {ours!r} != {rusts!r}"
    else:
        assert ours == rusts, f"{where}: {ours!r} != {rusts!r}"


def complexes(nested: Any) -> np.ndarray:
    a = np.asarray(nested, dtype=float)
    return a[..., 0] + 1j * a[..., 1]


def call(case: dict[str, Any]) -> Any:
    """The case's call, made through the package."""
    args = json.loads(case["args"])
    function = case["function"]
    if function == "write_touchstone":
        # its result is the file's text
        s = args.pop("spectrum")
        spectrum = po.Spectrum(
            ports=s["ports"], wavelength_um=np.asarray(s["wavelength_um"]), s=complexes(s["s"])
        )
        path = args.pop("path")
        po.write_touchstone(path, spectrum, **args)
        return Path(path).read_text(encoding="utf-8")
    return getattr(po, function)(**args)


def test_every_case_gives_rusts_result_to_the_bit(cases: list[dict[str, Any]]) -> None:
    assert cases
    for case in cases:
        ours = plain(call(case))
        rusts = json.loads(case["result"])
        same(ours, rusts, set(case["ignore"]), case["name"])


def test_the_cases_call_every_function_of_the_package(cases: list[dict[str, Any]]) -> None:
    called = {case["function"] for case in cases}
    functions = {
        name for name in po.__all__ if callable(getattr(po, name)) and name[0].islower()
    }
    assert called == functions


def test_a_changed_number_is_found(cases: list[dict[str, Any]]) -> None:
    # the comparison itself: one bit off in one number is a difference
    case = next(c for c in cases if c["name"] == "slab_modes/book-te")
    rusts = json.loads(case["result"])
    ours = plain(call(case))
    ours[0]["effective_index"] = np.nextafter(ours[0]["effective_index"], 3.0)
    with pytest.raises(AssertionError):
        same(ours, rusts, set(), case["name"])
