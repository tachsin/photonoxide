"""photonoxide's errors, one class per kind, each with photonoxide's message.

Every error photonoxide raises is an :class:`Error`; each kind also derives from the built-in
exception it resembles, so ``except ValueError`` catches an invalid value as it would anywhere
else. The error's fields are attributes: an :class:`OutsideValidity` has the ``material``, the
``wavelength_um`` asked for and the range, ``shortest_um`` to ``longest_um``::

    import photonoxide as po

    try:
        po.refractive_index("si", wavelength_um=0.5)
    except po.errors.OutsideValidity as e:
        print(e.material, e.shortest_um, e.longest_um)

A long call stopped by its ``timeout_s`` raises the built-in :class:`TimeoutError`, and one
interrupted by Ctrl+C :class:`KeyboardInterrupt`, after the work has stopped.
"""

from __future__ import annotations

__all__ = [
    "Error",
    "InvalidValue",
    "OutsideValidity",
    "IoError",
    "ParseError",
    "NetlistError",
    "GpuError",
]


class Error(Exception):
    """An error of photonoxide: the base class of the others."""

    def __init__(self, message: str, **fields: object) -> None:
        super().__init__(message)
        for name, value in fields.items():
            setattr(self, name, value)


class InvalidValue(Error, ValueError):
    """A value is invalid: not finite, out of its range, or inconsistent with another.

    Attributes: ``what`` (e.g. ``"wavelength"``) and ``reason``.
    """

    what: str
    reason: str


class OutsideValidity(Error, ValueError):
    """A material was asked for a wavelength outside the range its data is valid for.

    Attributes: ``material``, ``wavelength_um``, ``shortest_um`` and ``longest_um``.
    """

    material: str
    wavelength_um: float
    shortest_um: float
    longest_um: float


class IoError(Error, OSError):
    """A file or folder can't be read or written. Attributes: ``path`` and ``reason``."""

    path: str
    reason: str


class ParseError(Error, ValueError):
    """Text can't be read as what it should be: a job, a Touchstone file, a netlist.

    Attributes: ``what`` (e.g. a path and a line) and ``reason``.
    """

    what: str
    reason: str


class NetlistError(Error, ValueError):
    """A circuit's netlist is invalid: an unknown port or parameter, a port used twice or left
    unconnected, a value out of its range."""


class GpuError(Error, RuntimeError):
    """A GPU can't be found or opened, or can't do what was asked. Attribute: ``reason``."""

    reason: str
