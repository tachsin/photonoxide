"""The examples' checks, printed as the Rust examples' ``common::Checks`` prints them: each number
beside the paper's, with its tolerance, and the exit code 1 when one is outside it. An example
with a Rust twin prints what the twin prints, line for line (python/tests/test_examples.py)."""

from __future__ import annotations

import sys
from decimal import Decimal


def display(x: float) -> str:
    """A float as Rust's ``{}`` writes it: the shortest that reads back, ``10`` for 10.0."""
    s = repr(float(x))
    return s[:-2] if s.endswith(".0") else s


def exponent(x: float) -> str:
    """A float as Rust's ``{:e}`` writes it: ``5e-4``, ``1.5e-5``."""
    sign, digits, exp = Decimal(repr(float(x))).normalize().as_tuple()
    assert isinstance(exp, int)
    mantissa = str(digits[0]) + ("." + "".join(map(str, digits[1:])) if len(digits) > 1 else "")
    return ("-" if sign else "") + f"{mantissa}e{exp + len(digits) - 1}"


class Checks:
    def __init__(self) -> None:
        sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[union-attr]
        self.total = 0
        self.failed = 0

    def compare(self, what: str, got: float, printed: float, tolerance: float) -> None:
        """``got`` within ``tolerance`` of the paper's ``printed``."""
        self._row(
            abs(got - printed) <= tolerance,
            f"  {what:<34} {got:>12.6f}   paper {display(printed):<10} ± {exponent(tolerance)}",
        )

    def against(self, what: str, got: float, source: str, value: float, tolerance: float) -> None:
        """``got`` within ``tolerance`` of ``value``, a formula's (``source``) from the paper."""
        self._row(
            abs(got - value) <= tolerance,
            f"  {what:<34} {got:>12.6f}   {source} {value:<10.6f} ± {exponent(tolerance)}",
        )

    def _row(self, ok: bool, text: str) -> None:
        self.total += 1
        if not ok:
            self.failed += 1
        print(f"{text:<80} {'ok' if ok else 'FAILED'}")

    def finish(self) -> None:
        if self.failed == 0:
            print(f"all {self.total} within tolerance of the paper")
        else:
            print(f"{self.failed} of {self.total} outside tolerance")
            sys.exit(1)
