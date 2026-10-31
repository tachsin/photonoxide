"""What an example prints first: its setup, every dimension and material of what it computes, as
the Rust examples' ``common::Setup`` prints it (examples/common/setup.rs).

An example builds a :class:`Setup` from the same values its computation uses (its layers, its
shapes, its materials, its grid) and prints it before it computes anything, so the block can't
drift from the computation. The layers are listed from the top down, the bottom one (the
substrate) the first laid down in fabrication. ``PHOTONOXIDE_SETUP_ONLY=1`` makes an example
print its setup and stop (exit code 0), before the computation.
"""

from __future__ import annotations

import os
import sys
from collections.abc import Mapping

import photonoxide as phox
from _checks import display, exponent

SETUP_ONLY = "PHOTONOXIDE_SETUP_ONLY"


def number(x: float) -> str:
    """A number to 7 significant digits, as short as it reads: ``0.6961663``, ``214.2857``, ``2.5``."""
    r = float(f"{x:.6e}")
    if r == 0.0:
        return "0"
    if abs(r) >= 1e7 or abs(r) < 1e-4:
        return exponent(r)
    return display(r)


def length(um: float) -> str:
    """A length in µm, in nm below a micrometre: ``220 nm``, ``2 µm``."""
    if um != 0.0 and abs(um) < 1.0:
        return f"{number(um * 1000.0)} nm"
    return f"{number(um)} µm"


def point(x_um: float, y_um: float) -> str:
    """A point in µm: ``(0, 0.35) µm``."""
    return f"({number(x_um)}, {number(y_um)}) µm"


def n(index: float) -> str:
    """A refractive index, for :meth:`Setup.layer`: ``n 3.476`` (six decimals)."""
    return f"n {index:.6f}"


def rectangle(center: tuple[float, float], width_um: float, height_um: float) -> str:
    """A rectangle, for :meth:`Setup.shape`, as the Rust setup describes one."""
    return (
        f"rectangle centred at {point(*center)}, {length(width_um)} along x "
        f"by {length(height_um)} along y"
    )


class Setup:
    """An example's setup, printed by :meth:`print`: the layers from the top down, the shapes on
    them and the settings (the wavelength, the window, the grid, the solver, the tolerance, in the
    order given); then the materials, each with its model, its source and its range."""

    def __init__(self) -> None:
        self.layers: list[str] = []
        self.shapes: list[str] = []
        self.items: list[tuple[str, str]] = []
        self.materials: list[str] = []
        self.named: set[str] = set()

    def layer(self, name: str, thickness_um: float | None, material: str, optical: str) -> None:
        """A layer, called from the top down: its thickness (``None`` for a half-space: a
        cladding above, a substrate below), what fills it, and its index at the wavelength."""
        thickness = "half-space" if thickness_um is None else length(thickness_um)
        self.layers.append(f"{name:<14} {thickness:<12} {material}, {optical}")

    def layer_sized(self, name: str, thickness: str, filled: str) -> None:
        """A layer whose filling is described in full (a shape's material and the background)."""
        self.layers.append(f"{name:<14} {thickness:<12} {filled}")

    def shape(self, layer: str, what: str) -> None:
        """A shape on ``layer``, with its position and size and their units."""
        self.shapes.append(f"on {layer}: {what}")

    def item(self, what: str, value: str) -> None:
        """A setting: the wavelength, the window, the grid, the solver, the tolerance, ..."""
        self.items.append((what, value))

    def wavelength(self, wavelength_um: float) -> None:
        """The wavelength the example computes at."""
        self.item("wavelength", f"{number(wavelength_um)} µm")

    def medium(self, name: str, what: str) -> None:
        """A material given by its index alone: what it is and where the number comes from."""
        if name not in self.named:
            self.named.add(name)
            self.materials.append(f"{name}: {what}")

    def catalogue(self, material: str, wavelength_um: float, model: str | None = None) -> str:
        """A material of the catalogue: its model, its range, its source and its index at
        ``wavelength_um``, all from :func:`photonoxide.materials`. Returns the material's name."""
        entry = next(m for m in phox.materials() if m.id == material)
        chosen = next(
            m for m in entry.models if (m.id == model if model is not None else m.default)
        )
        if entry.name not in self.named:
            self.named.add(entry.name)
            lo, hi = chosen.range_um
            at = phox.refractive_index(material, wavelength_um=wavelength_um, model=chosen.id)
            text = f"{entry.name}: {chosen.name}; valid {number(lo)} to {number(hi)} µm"
            if chosen.temperature_k is not None:
                text += f", at {number(chosen.temperature_k)} K"
            text += f"\n      source: {chosen.sources[0]}"
            text += f"\n      accuracy: {chosen.accuracy}"
            text += f"\n      n {at.real:.6f} at {number(wavelength_um)} µm"
            self.materials.append(text)
        return entry.name

    def instances(
        self,
        instances: Mapping[str, Mapping[str, object]],
        units: Mapping[str, str],
        ports: Mapping[str, str],
    ) -> None:
        """A circuit as data: each instance with its kind and every parameter's value (``units``
        names a parameter's unit, if it has one), and the ports it exposes."""
        for name, instance in instances.items():
            parameters = [
                f"{key} {number(value)}{(' ' + units[key]) if key in units else ''}"  # type: ignore[arg-type]
                for key, value in instance.items()
                if key != "kind"
            ]
            self.item(
                f"instance {name}",
                f"a {instance['kind']}{': ' if parameters else ''}{', '.join(parameters)}",
            )
        self.item("ports", ", ".join(ports))

    def print(self) -> None:
        """Prints the setup, then a blank line. With ``PHOTONOXIDE_SETUP_ONLY`` set (to anything
        but ``0``), stops the example there, successfully."""
        sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[union-attr]
        print("Setup")
        if self.layers:
            print("  (layers from the top down: the bottom one, the substrate, is the first laid down)")
            print("  layers:")
            for layer in self.layers:
                print(f"    {layer}")
        if self.shapes:
            print("  shapes:")
            for shape in self.shapes:
                print(f"    {shape}")
        for what, value in self.items:
            print(f"  {what}: {value}")
        if self.materials:
            print("  materials:")
            for material in self.materials:
                print(f"    {material}")
        print()
        if os.environ.get(SETUP_ONLY, "") not in ("", "0"):
            sys.exit(0)
