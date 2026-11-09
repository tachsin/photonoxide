"""E5: a conductivity that depends on temperature, photonoxide against Elmer (#613, #536).

The case is static/nonlinear-slab's: a slab of oxide 2 um thick (1 um wide, its sides
insulated) generating 560 W/m over its cross-section, both faces held at 0 K, its conductivity
k = k0 (1 + b T) with k0 = 1.4 W/(m K) and b = -2e-3 per K. Kirchhoff's transform gives the exact
temperature, T(y) = (sqrt(1 + 2 b U(y) / k0) - 1) / b with U = q y (L - y) / 2, a peak of
112.70 K at the middle.

photonoxide solves it by Picard's iteration (`Heat::conductivity_slope`, `Heat::solve_nonlinear`):
on 80 rows its largest error is 2.70e-4 of the peak (static/nonlinear-slab, docs/validation.md).
This script writes the same slab for Elmer on the same mesh lines (one column of 80 rows of
quadrilaterals, nodes on the rows' edges), HeatSolver with the conductivity a table
in temperature (linear between its two points, so exactly k0 (1 + b T)) and its own Picard
iteration to 1e-12, runs ElmerSolver, and compares Elmer's peak (the node at the middle) with
the closed form. It runs 40 and 80 rows, so Elmer's order is shown too.

Rule: Elmer's peak within 0.5 % of the closed form on 80 rows, the cross-code pairs' tolerance
for a rise (crosscode/README.md); photonoxide's own error on the same rows is the case's 2.70e-4,
so the two codes then agree within their sum.

Usage, from the repository's root (Python 3, the standard library only):

    python crosscode/e5/e5.py                 # finds ElmerSolver by $PHOTONOXIDE_ELMER or PATH
    python crosscode/e5/e5.py --write-only    # writes the inputs to target/tmp/elmer-e5/ only

$PHOTONOXIDE_ELMER names ElmerSolver or the folder holding it, and $PHOTONOXIDE_ELMER_PATH the
folders of its run-time libraries, as for the other Elmer pairs. Each run is kept in
target/tmp/elmer-e5/<rows>/ with Elmer's log beside it. The exit status is 0 when the rule holds
(or Elmer isn't found and the run is skipped), 1 when it doesn't.
"""

import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

K0 = 1.4  # W/(m K)
B = -2e-3  # per K
LENGTH_UM = 2.0
WIDTH_UM = 1.0
POWER = 560.0  # W per metre of length, over the cross-section
Q = POWER / (WIDTH_UM * LENGTH_UM * 1e-12)  # W/m^3
TOLERANCE = 5e-3  # relative to the peak: the cross-code pairs' 0.5 % for a rise
PHOTONOXIDE_80 = 2.70e-4  # static/nonlinear-slab on 80 rows, of the peak


def exact(y_m):
    """The closed form's temperature at y (m), K."""
    u = Q * y_m * (LENGTH_UM * 1e-6 - y_m) / 2.0
    return (math.sqrt(1.0 + 2.0 * B * u / K0) - 1.0) / B


def mesh(rows):
    """Elmer's mesh files for one column of `rows` quadrilaterals, coordinates in um."""
    xs = [0.0, WIDTH_UM]
    ys = [LENGTH_UM * j / rows for j in range(rows + 1)]

    def node(i, j):
        return j * 2 + i + 1

    nodes = "".join(
        f"{node(i, j)} -1 {x:e} {y:e} 0\n" for j, y in enumerate(ys) for i, x in enumerate(xs)
    )
    elements = "".join(
        f"{j + 1} 1 404 {node(0, j)} {node(1, j)} {node(1, j + 1)} {node(0, j + 1)}\n"
        for j in range(rows)
    )
    # the bottom face (boundary 1) and the top face (boundary 2), each with its one parent
    boundary = (
        f"1 1 1 0 202 {node(0, 0)} {node(1, 0)}\n"
        f"2 2 {rows} 0 202 {node(1, rows)} {node(0, rows)}\n"
    )
    header = f"{2 * (rows + 1)} {rows} 2\n2\n404 {rows}\n202 2\n"
    return {
        "mesh.header": header,
        "mesh.nodes": nodes,
        "mesh.elements": elements,
        "mesh.boundary": boundary,
    }


def sif():
    """The solver input: HeatSolver with k(T) as a table, Picard's iteration to 1e-12."""
    k_hot = K0 * (1.0 + B * 200.0)
    return f"""Header
  Mesh DB "." "mesh"
End

Simulation
  Coordinate System = Cartesian 2D
  Coordinate Scaling = 1.0e-6
  Simulation Type = Steady State
  Steady State Max Iterations = 1
  Output Intervals = 0
End

Body 1
  Name = "oxide"
  Equation = 1
  Material = 1
  Body Force = 1
End

Equation 1
  Active Solvers(1) = 1
End

Solver 1
  Equation = Heat Equation
  Procedure = "HeatSolve" "HeatSolver"
  Variable = Temperature
  Linear System Solver = Direct
  Linear System Direct Method = Umfpack
  Nonlinear System Max Iterations = 200
  Nonlinear System Convergence Tolerance = 1.0e-12
  Nonlinear System Newton After Iterations = 1000
  Nonlinear System Newton After Tolerance = 0.0
  Nonlinear System Relaxation Factor = 1.0
End

Solver 2
  Exec Solver = After All
  Equation = SaveScalars
  Procedure = "SaveData" "SaveScalars"
  Filename = "scalars.dat"
  Variable 1 = Temperature
  Operator 1 = max
End

! k = k0 (1 + b T), k0 = {K0} W/(m K), b = {B} per K: linear between the table's two points
Material 1
  Heat Conductivity = Variable Temperature
    Real
      0.0 {K0:e}
      200.0 {k_hot:e}
    End
  Density = 1.0
End

! {POWER} W/m over the {WIDTH_UM} x {LENGTH_UM} um cross-section, W/m^3 (per mass, a density of 1)
Body Force 1
  Heat Source = {Q:e}
End

Boundary Condition 1
  Name = "bottom"
  Target Boundaries(1) = 1
  Temperature = 0.0
End

Boundary Condition 2
  Name = "top"
  Target Boundaries(1) = 2
  Temperature = 0.0
End
"""


def find_elmer():
    """ElmerSolver by $PHOTONOXIDE_ELMER (the program or its folder), then PATH."""
    program = "ElmerSolver.exe" if os.name == "nt" else "ElmerSolver"
    named = os.environ.get("PHOTONOXIDE_ELMER", "")
    if named:
        p = Path(named)
        return p / program if p.is_dir() else p
    found = shutil.which(program)
    return Path(found) if found else None


def peak_of(folder):
    """The max of the temperature SaveScalars wrote, from scalars.dat and its .names file."""
    names = (folder / "scalars.dat.names").read_text()
    column = None
    for line in names.splitlines():
        head, _, name = line.partition(":")
        if name.strip().lower() == "max: temperature" and head.strip().isdigit():
            column = int(head) - 1
    if column is None:
        raise RuntimeError(f"{folder}: no max: temperature in scalars.dat.names")
    rows = [r.split() for r in (folder / "scalars.dat").read_text().splitlines() if r.strip()]
    return float(rows[-1][column])


def main():
    write_only = "--write-only" in sys.argv[1:]
    root = Path("target/tmp/elmer-e5")
    elmer = None if write_only else find_elmer()
    if not write_only and (elmer is None or not elmer.exists()):
        print("skipped: Elmer isn't installed here (neither $PHOTONOXIDE_ELMER nor PATH has ElmerSolver)")
        return 0
    env = dict(os.environ)
    extra = os.environ.get("PHOTONOXIDE_ELMER_PATH", "")
    if extra:
        env["PATH"] = extra + os.pathsep + env.get("PATH", "")
    peak = exact(0.5 * LENGTH_UM * 1e-6)
    print(f"closed form's peak: {peak:.6f} K")
    errors = {}
    for rows in (40, 80):
        folder = root / str(rows)
        (folder / "mesh").mkdir(parents=True, exist_ok=True)
        for name, text in mesh(rows).items():
            (folder / "mesh" / name).write_text(text)
        (folder / "case.sif").write_text(sif())
        (folder / "ELMERSOLVER_STARTINFO").write_text("case.sif\n")
        if write_only:
            print(f"written: {folder}")
            continue
        with open(folder / "elmer.log", "w") as log:
            run = subprocess.run([str(elmer)], cwd=folder, stdout=log, stderr=subprocess.STDOUT, env=env)
        if run.returncode != 0:
            print(f"Elmer failed on {rows} rows: see {folder / 'elmer.log'}")
            return 1
        value = peak_of(folder)
        errors[rows] = abs(value - peak) / peak
        print(f"Elmer, {rows} rows: peak {value:.6f} K, {errors[rows]:.3e} of the peak")
    if write_only:
        return 0
    order = math.log2(errors[40] / errors[80]) if errors[80] > 0 else float("inf")
    print(f"Elmer's order from 40 to 80 rows: {order:.2f}")
    print(f"photonoxide, 80 rows (static/nonlinear-slab): {PHOTONOXIDE_80:.2e} of the peak")
    ok = errors[80] <= TOLERANCE
    print(f"{'pass' if ok else 'FAIL'}: Elmer's {errors[80]:.3e} against the {TOLERANCE:.1%} rule; "
          f"the two codes within {errors[80] + PHOTONOXIDE_80:.3e} of each other")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

