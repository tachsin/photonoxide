# Components and circuits: the design

The model behind 0.4's components and circuits (`photonoxide::circuit`): what a component is,
the conventions every S-matrix follows, what a netlist is, and how the rest of photonoxide plugs
in: the solvers, compact models and measurements that make components, the circuit adjoint and
optimization, and the studio's library and chip view. The method itself, the circuit solve and
its validation, is in [docs/methods/circuits.md](../methods/circuits.md).

## The model

A chip is a **netlist**: instances of **components**, **connections** between their ports, and
**external ports** left open to the outside. A component is anything with ports and an S-matrix
at a wavelength. A netlist, solved, has an S-matrix between its external ports, so a circuit is
a component too, and circuits nest.

Nothing in a circuit is physics of its own: the components carry it, at whatever fidelity they
were made, and the circuit only connects them. A connection has no length and no loss: what
leaves one port enters the other, at the same reference plane. A waveguide between two
components is a component.

## Conventions

They are the FDFD solver's ([fdfd-ports.md](../methods/fdfd-ports.md)), so its S-matrices are
components as they are.

- **Time:** e^(−iωt) ([conventions.md](../methods/conventions.md)). A waveguide of effective
  index n and length L transmits e^(+i 2π n L / λ); loss makes the magnitude smaller, never the
  phase run backwards.
- **Power-normalized amplitudes:** a port's incoming amplitude a and outgoing amplitude b are
  scaled so that |a|² and |b|² are the powers the waves carry. Then b = S a, and |S_qp|² is the
  share of the power in at port p that comes out at port q.
- **Indexing:** `s[(q, p)]` is from port p into port q: the row is the output, the column the
  input. The ports are in the order the component lists them.
- **Reference planes:** each port's phase is measured at its reference plane, where the
  component ends. Connecting two components joins their reference planes.
- **Reciprocity:** a reciprocal component has S = Sᵀ, exactly in the FDFD solver, whose
  S-matrices are normalized by the unconjugated Lorentz form for that reason. Everything up to
  magneto-optics is reciprocal; `Component::reciprocal` says so, and defaults to true.
- **Passivity:** a passive component's S has every singular value at most 1; a lossless one has
  SᴴS = I. `SMatrix` checks all three: `reciprocity_error`, `largest_singular_value` (and
  `is_passive`), `unitarity_error`.

## The types

| Type | What it is |
|---|---|
| `SMatrix` | A square complex matrix, `s[(q, p)]`, with the checks above; `from_rows` takes the FDFD solver's `Vec<Vec<Complex64>>`. |
| `Port` | A port's `name` (unique in its component, no dots) and optionally its `PortMode`: polarization (TE/TM), mode order, effective index, group index, at a reference wavelength. A multimode waveguide has a port per mode. |
| `Parameter` | A continuous value S depends on: `name`, `unit`, `default`, `min`, `max`. Anything that changes the ports (a splitter's number of outputs) is fixed when the component is built, not a parameter. |
| `Fidelity` | `Analytic`, `Compact`, `TwoD`, `ThreeD`, `Measured`. |
| `Provenance` | A component's `fidelity`, its `source` (a paper's DOI and equation, a solver and its grid, a file), its `error` against that source (largest \|ΔS_qp\|, if measured) and the wavelengths it is `validity`-limited to. |
| `Component` | The trait: `kind`, `ports`, `parameters`, `s_matrix(wavelength, values)`, `provenance`, `reciprocal`, and `derivatives` for the adjoint. |
| `Fixed` | A component whose S is the same at every wavelength, with no parameters. |
| `Spectrum` | S-matrices at several wavelengths, with the ports' names. |
| `Netlist` | Instances, connections, external ports, built a step at a time and checked as it goes. |
| `Instance` | A component in a netlist under a name, with its parameters' values. |
| `PortRef` | `instance.port`. |
| `NetlistError` | Why a netlist is invalid, one variant per reason, wrapped as `Error::Netlist`. |

### Components are models, values live in the netlist

`Component::s_matrix` takes the parameters' values as an argument instead of holding them. A
component is then immutable and shared, `Arc<dyn Component>`: one directional coupler model
serves every coupler on a chip, each instance with its own gap. And an optimizer moves values
in the netlist, a flat vector, without rebuilding any component.

A trait, not an enum, so every module adds its own components without touching this one: the
solvers' wrappers, compact models, measured data and circuits themselves.

### Netlists check every step

`Netlist::add`, `connect`, `expose` and `set` each refuse what can't be built, with a
`NetlistError` that names it: a name with a dot (`InvalidName`), a taken name
(`DuplicateInstance`, `DuplicateExternal`), an instance or port or parameter that doesn't exist
(`UnknownInstance`, `UnknownPort`, `UnknownParameter`), a value out of its range
(`ValueOutOfRange`), a port connected to itself (`SelfConnection`) or used twice
(`PortUsedTwice`), and ports whose stated modes differ in polarization or order
(`ModeMismatch`). A component with duplicate port or parameter names, or a default out of its
range, is refused when added (`InvalidComponent`). What remains is a port neither connected nor
exposed (`Dangling`), which only the finished netlist can tell: `Netlist::problems` lists them
all, and `validate` fails on the first. A component whose S-matrix isn't one row and column per
port is caught when the circuit is solved (`SizeMismatch`).

Every port is used exactly once. A port meant to absorb what reaches it is exposed, or
connected to a terminator (a 1-port with S = 0); a dangling port is a mistake, not a default.

## The circuit solve

With all ports of all instances numbered together, S_b is the block-diagonal matrix of the
components' S-matrices, b = S_b a. A connection between ports i and j says a_i = b_j and
a_j = b_i: a = Γ b + E x, with Γ the symmetric permutation of the connections and E putting the
external inputs x on their ports. So

$$
(I - S_b \Gamma)\thinspace b = S_b E\thinspace x, \qquad S = E^\mathsf{T} (I - S_b \Gamma)^{-1} S_b E,
$$

one sparse system, as many unknowns as ports, with a right-hand side per external port. Its
sparsity depends only on the netlist's topology, so a sweep analyses it once and reuses the
symbolic factorization, as the FDFD solver does. Filipsson's sub-network growth (Filipsson
1981, [10.1109/EUMA.1981.332972](https://doi.org/10.1109/EUMA.1981.332972)), one connection
at a time, is the reference it is checked against to round-off.

`Netlist::compile` checks the netlist, numbers its ports and analyses the sparsity, giving a
`Circuit`: `s_matrix(wavelength)` at its current values, `s_matrix_with(wavelength, values)` at
others (an optimizer's call), `spectrum(&wavelengths)`, `set(instance, parameter, value)`, and
`s_matrix_by_growth(wavelength)`, the reference. A `Circuit` is a `Component`: its ports are the
external ports, its parameters its instances', named `instance.parameter`, in the order of
`values()`.

## How the rest plugs in

**Solvers (2D and 3D FDFD).** `Solver2d::s_matrix(&ports)` returns `Vec<Vec<Complex64>>` in
these conventions; a component wraps it, `SMatrix::from_rows`, with `Fidelity::TwoD` or
`ThreeD`, ports carrying the port modes' effective indices, and the provenance naming the grid
and the convergence error. A solver run per wavelength is expensive, so such a component
usually samples a `Spectrum` once and interpolates, or is fitted into a compact model.

**Compact models and Touchstone (measured).** A compact model is a component whose `s_matrix`
evaluates the fit, with `Fidelity::Compact` and the fit's residual as its error. A Touchstone
file reads into a `Spectrum` (ports named from the file), and a measured component interpolates
it, with `Fidelity::Measured`, the file as the source and its validity the file's wavelength
range. Writing Touchstone takes a `Spectrum`, from any component (`Spectrum::of`) or circuit.

**Components (waveguide, bend, coupler, MMI, Y-branch, ring, MZI).** Each is a type implementing
`Component`, one per model: an analytic waveguide and a 2D FDFD waveguide are two components of
the same kind. Rings and MZIs can be components of their own (closed forms) or netlists of
couplers and waveguides, as circuits. They are `circuit::components`, each model in
[First components](../methods/components.md).

**The circuit adjoint.** `Component::derivatives` returns ∂S/∂θ_k for each parameter, or `None`
to have the circuit take finite differences. With M = I − S_b Γ and the incoming waves
A = Γ B + E (B the solved outgoing waves, a column per external input),

$$
\frac{\partial S}{\partial \theta} = E^\mathsf{T} M^{-1}\thinspace \frac{\partial S_b}{\partial \theta}\thinspace A ,
$$

and a scalar response's gradient with respect to every parameter of every instance takes one
solve with Mᵀ, reusing M's factorization, as the FDFD adjoint does. The netlist's values are the
optimizer's vector: instances in order, each instance's parameters in order, each with its
range for genoxide's bounds. `Circuit::gradient` and `Circuit::jacobian` implement it; the
method, its conjugation and its validation are in [circuit-adjoint.md](../methods/circuit-adjoint.md).

**The studio's library and chip view.** A component lists everything the library shows: `kind`,
its ports (and their modes), its parameters with units and ranges, its provenance. The chip
view edits a `Netlist` step by step, and each step's `NetlistError` says which instance, port or
parameter to point at. Saving a chip needs components rebuilt from their names and parameters:
a registry from an id to a constructor, which the studio keeps
(studio/src-tauri/src/circuits/library.rs: the library's components, built with the guide and
widths the studio chooses), and a chip file that names each instance's id, values and place
(studio/README.md, "Chip files"); a measured instance names its Touchstone file and the file's
time convention. Positions on the chip are the view's, not the netlist's, until layout (0.9)
gives ports positions of their own.
