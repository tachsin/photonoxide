---
title: "The Pockels effect: an electrode field in a mode, and Vπ·L"
module: "mode::pockels, electro"
summary: "A crystal's electro-optic tensor r_ijk from the catalogue, turned by its orientation into the chip's frame and the waveguide's, the static solver's field in each cell, Δε = −ε Δη ε with Δη = r·E, the mode's Δn_eff by first-order perturbation (checked by re-solving the perturbed cross-section), and one Vπ·L: λ/(4|dn_eff/dV|) in push–pull, λ/(2|dn_eff/dV|) on one arm. The bulk crystal's −½n³rE for every cut to 4e-6, a film between plates against its slab to 1.5e-5, the re-solve within 0.06%. Wang et al. 2018's and Zhang et al. 2022's thin-film modulators: 1.84 V·cm against 1.8, 2.37 against 2.3, both grids refined to 10 nm and gated."
order: 25.5
papers:
  - cite: "M. Jazbinšek, M. Zgonik, Appl. Phys. B 74, 407 (2002) (lithium niobate's clamped and unclamped r and static ε, Tables 1, 4 and 5)"
    doi: 10.1007/s003400200818
  - cite: "S. G. Johnson et al., Phys. Rev. E 65, 066611 (2002) (first-order perturbation of Maxwell's equations)"
    doi: 10.1103/PhysRevE.65.066611
  - cite: "C. Wang, M. Zhang, B. Stern, M. Lipson, M. Lončar, Opt. Express 26, 1547 (2018) (an etched thin-film modulator, 1.8 V·cm)"
    doi: 10.1364/OE.26.001547
  - cite: "Y. Zhang et al., Photon. Res. 10, 2380 (2022) (a thin-film modulator's full stack, 2.3 V·cm at DC)"
    doi: 10.1364/PRJ.468518
  - cite: "M. He et al., Nat. Photonics 13, 359 (2019) (a hybrid silicon and thin-film lithium niobate modulator, 2.2 and 2.5 V·cm at 100 kHz, not reproduced)"
    doi: 10.1038/s41566-019-0378-6
  - cite: "D. Zhu et al., Adv. Opt. Photon. 13, 242 (2021) (thin-film lithium niobate, its modulators and their overlap)"
    doi: 10.1364/AOP.411024
  - cite: "J. Holzgrafe et al., Opt. Express 32, 3619 (2024) (the electro-optic response's relaxation below about 1 MHz)"
    doi: 10.1364/OE.507536
---

A voltage on a modulator's electrodes puts a static field across its crystal; the field changes
the crystal's index through its linear electro-optic (Pockels) tensor; the guided mode's
effective index follows, and with it the phase along the guide. `photonoxide::mode::pockels`
takes the chain from the catalogue's tensor to Vπ·L, with the field from the static solver
([static.md](static.md), `electro::Electrostatic::electric_field`).

## The tensor and its frames

A field E changes the impermeability η = ε⁻¹ by

$$\Delta\eta_{ij} = \sum_k r_{ijk} E_k ,$$

r symmetric in i and j, written in Voigt notation r_Ik (I = 1 … 6 for 11, 22, 33, 23, 13, 12) as
the catalogue gives it, in the crystal's axes, with its source, wavelength and clamping
([catalogue.md](catalogue.md)). `Pockels::from_tensor` reads one; `Pockels::completed` fills the
elements one source doesn't give from another (the thin film's r₃₃ with the bulk crystal's other
elements).

r is a true tensor of rank three. With M the orientation's matrix (crystal components to the
chip's, [catalogue.md](catalogue.md)'s "Orientation"), in the chip's axes

$$r'_{abc} = \sum_{ijk} M_{ai} M_{bj} M_{ck}\, r_{ijk}$$

(`Pockels::rotated`), and the cross-section's axes (x across the guide, y up, z along it) are the
chip's as a modes job takes them (`Along`: along y, the chip's x, z and y; along x, its y, z and
−x). The static solver's field is in the cross-section's plane, (E_x, E_y, 0).

## The permittivity and the mode

To first order in Δη, ε′ = (η + Δη)⁻¹ = ε − ε Δη ε. On a principal axis of index n with the field
along an axis k, this is the textbook Δn = −½ n³ r E, Δε = 2n Δn = −n⁴ r E.

The mode's index moves by first-order perturbation, as for a temperature
([fields.md](fields.md)):

$$\Delta n_\mathrm{eff} = \frac{\int E^*\cdot\Delta\varepsilon\cdot E\, dA}{2\,\mathrm{Re}\int (E\times H^*)\cdot\hat z\, dA},$$

E in units of Z₀H. The full-vector solver takes a tensor with a principal axis along the guide,
so `section_change` keeps Δε's transverse block and its zz element. The elements coupling z to x
and y (from r₅₁ and r₂₂ in an x-cut crystal, with the field up) would add
2 Re(E_x^* E_z) Δε_xz to the numerator; that vanishes for a lossless guided mode, whose E_z is a
quarter period out of phase with its transverse field. The re-solve of the perturbed
cross-section (`mode::thermo::index_change_resolved`) checks the rest.

A cell the crystal fills in part takes the mean of −ε Δη ε over its samples inside the crystal,
each at the field there, as its permittivity is the mean of its samples'. The static field at a
point is interpolated between the static cells' centres only within one medium: across an
interface, D·n is continuous and E jumps (in the crystal, ε₃₃ ≈ 27 against the oxide's 3.9, the
field is seven times smaller), so the field comes from each face's flux D, divided by the cell's
own ε, not from a central difference of the potential across the interface.

## Vπ·L

A straight section of length L driven at V shifts the phase by (2π/λ) (dn_eff/dV) V L. On one arm
(a phase modulator), π takes

$$V_\pi L = \frac{\lambda}{2\,|dn_\mathrm{eff}/dV|}$$

(`Drive::SingleArm`). A Mach–Zehnder driven in push and pull (a ground–signal–ground line, each
arm in one of its gaps, the fields in them opposite) moves the two arms' indices apart by twice
that, so

$$V_\pi L = \frac{\lambda}{4\,|dn_\mathrm{eff}/dV|}$$

(`Drive::PushPull`), dn_eff/dV that of one arm with V across its gap. Both are a cross-section's
numbers per unit length of a straight section, at DC or the quasi-static limit: 2D quantities, not
a device's performance (its electrodes' RF loss, the velocity mismatch and the line's impedance
are the travelling-wave electrodes', [travelling-wave.md](travelling-wave.md)).

**The papers' overlap.** The thin-film papers write it with an overlap
Γ = (g/V) ∫_LN E_s |E|² dA / ∫ |E|² dA, E_s the static field along c (`overlap_factor`), as
V_πL = λ g / (2 n_e³ r₃₃ Γ) in push–pull (C. Wang et al. 2018; D. Zhu et al. 2021). That form
normalizes the mode by ∫|E|², not by its power: with Re (E × H*)·ẑ ≈ n_eff |E|² (in units of Z₀),
the dominant term of the perturbation is Δn_eff ≈ −½ n_e³ r₃₃ (n_e/n_eff) Γ V/g, so

$$V_\pi L \approx \frac{\lambda g\, n_\mathrm{eff}}{2 n_e^4 r_{33} \Gamma},$$

about 10% below the papers' form for a thin-film rib (n_e/n_eff ≈ 2.14/1.93). On Wang et al.'s
cross-section Γ = 0.51; the scaled form is within 0.8% of the perturbation's Vπ·L, the rest E's
other components and r₁₃ (`pockels/overlap-gamma`). photonoxide reports the perturbation's.

## The coefficients

Lithium niobate's in the catalogue:

- **bulk, clamped (r^S) and unclamped (r^T), 633 nm,** Jazbinšek & Zgonik's fit: r₃₃ = 31.2 and
  31.45 pm/V, r₁₃ = 9.10 and 10.12, r₂₂ = 3.40 and 6.64, r₅₁ = 18.1 and 33.96 (Tables 4 and 5);
  the static permittivities ε^S = 45.5 and 26.2 (perpendicular to c, along it; Table 5) and
  ε^T = 84.48 and 27.78 (Table 1's fit), for a field held long;
- **the thin film's r₃₃ at 1550 nm,** 26.9 ± 0.9 pm/V, clamped, flat from 100 MHz to 330 GHz
  (D. Chelladurai et al. 2025), which the published cases take with the bulk's other clamped
  elements and ε^S.

The 633 nm coefficients overstate the effect at 1550 nm: with r^T and ε^T the two published
cross-sections below give 1.56 and 2.02 V·cm. Below about 1 MHz thin-film lithium niobate's
response relaxes (J. Holzgrafe et al. 2024, who measure it from 1 mHz to 1 MHz: on timescales of
milliseconds in cladded films, and enhanced from 1 to 20 kHz in their devices, which can confound a
measured Vπ), which isn't modelled: these numbers are for fields above that, and a DC
measurement is compared with them within its own spread.

## Validation

| Case | What | Grid | Result |
|---|---|---|---|
| `pockels/bulk-x-cut`, `-z-cut`, `-y-cut` | a uniform field in bulk lithium niobate, the tensor turned by the cut, against −½ n³ r E (r₃₃, r₁₃, ±r₂₂) | — | 3.6e-6, 3.6e-6, 4.2e-7 (second order in E) |
| `pockels/plates-x-cut` | a 600 nm x-cut film in silica, a uniform field across: TE₀'s Δn_eff against the slab solved again | 5 nm | 1.5e-5 |
| `pockels/plates-z-cut` | the z-cut film between plates: the film's field against D/ε₃₃, and Δn_eff | 50 nm static, 5 nm optical | 1.1e-7 |
| `pockels/resolve-tfln` | Wang et al.'s cross-section at 10 V: the perturbation against the re-solve | 20 nm | −5.6e-4 |
| `pockels/resolve-order` | the same at 10, 20 and 40 V: the difference's part growing with the field | 20 nm | log₂ 1.00 |
| `pockels/static-order` | Vπ·L on the static grid's 20, 10, 5 nm | 20 nm optical | order 1.30 |
| `pockels/optical-convergence` | Vπ·L on 40, 20, 10 nm optical cells: the grid study's estimate | 10 nm static | 1.7e-6 |
| `pockels/overlap-gamma` | the papers' Γ form scaled by n_eff/n_e against the perturbation | 10 nm | Γ = 0.51, 0.8% |
| `pockels/wang-2018` | published: Wang et al. 2018, 1.8 V·cm (DC) | both 40, 20, 10 nm, gated at 0.5% | 1.84 V·cm |
| `pockels/zhang-2022` | published: Zhang et al. 2022, 2.3 V·cm (DC) | as above | 2.37 V·cm |
| `pockels/valdez-2023` | published: Valdez et al. 2023, 0.8 V·cm at 784 nm (DC) | both 20, 10, 5 nm, gated at 0.5% | 0.844 V·cm |
| `pockels/bto-angle` | barium titanate's 4mm tensor turned in the plane, 0 to 90°: r_eff against r₃₃ sin³φ + (r₁₃ + 2r₄₂) cos²φ sin φ | — | ≤ 1e-12 |
| `pockels/bto-deng` | published: Deng et al. 2026's r₄₂ = 358 pm/V at 45°, against their measured 253 pm/V | — | 253.1 pm/V |
| `pockels/bto-eltes` | published: Eltes et al. 2019's one-arm Vπ·L from their r_eff, n, Γ and gap, against 0.23 V·cm | — | 0.2324 V·cm |
| `pockels/litao3-x-cut` | a uniform field in x-cut lithium tantalate along c (Casson et al. 2004's r₁₃, r₃₃ and indices at 1558 nm) against −½ n³ r E | — | ≤ 1e-4 |
| `pockels/bto-abel-response` | published: Abel et al. 2018's barium titanate guide, a single domain at 45°: $\lvert dn_\mathrm{eff}/dV\rvert$ against their 3.4e-4 per volt | 40, 20, 10 nm, gated at 0.5% | 3.395e-4 |
| `pockels/bto-abel` | published: its Vπ·L through their net domain population, against 0.45 V·cm | as above | 0.453 V·cm |

Each case's numbers are in the validation report (docs/validation.md).

### The thin-film cross-sections

Both are x-cut lithium niobate, its c axis across the guide, so the TE mode (E across, n_e) and the
field across the gap meet through r₃₃. The electrostatic problem holds one gap: the ground
electrode to the window's left edge, the signal electrode to its centre line, a mirror there (the
line's symmetry), 20 µm of silicon below the buried oxide and of air above, every far side
insulated; its grid is graded from 10 nm at the rib's and the electrodes' corners by 15% a cell to
1 µm. The optical window is 6 µm wide, 1.5 µm into the buried oxide, cells 10 nm over the film
growing by 8% to 100 nm, each cell's permittivity the mean of 16 samples; the metal is left out of
the optics.

- **Wang et al. 2018** (Opt. Express, its Section 2): a 700 nm film on 2 µm of silica etched
  400 nm to a rib 900 nm wide on top, a 3.5 µm gap between electrodes of 15 nm of Ti and 300 nm of
  Au, 1.5 µm of silica over them; a 2 mm push–pull Mach–Zehnder's DC Vπ of 9 V, 1.8 V·cm. The
  paper doesn't give the sidewall (70° taken; 60 to 90° spans ±0.04 V·cm). 1.839 V·cm.
- **Zhang et al. 2022:** a 600 nm film etched 300 nm to a rib 1.2 µm wide on top, 2 µm of thermal
  oxide on silicon, a 5 µm gap beside a 20 µm signal electrode 0.83 µm thick, 0.6 µm of PECVD
  oxide (ε 5.5, as their model has it) and air; 2.3 V·cm at DC, 2.67 V·cm at 1 GHz through the
  line's impedance. The sidewall isn't given (70°; 60 to 90° spans ±0.03). 2.372 V·cm.

- **Valdez et al. 2023** (Optica 10, 578; `pockels/valdez-2023`): not a rib but an unetched 200 nm
  film of 5 mol% MgO-doped lithium niobate, bonded over 40 nm of silica onto a planarized silicon
  nitride strip 0.9 µm wide and 0.18 µm thick, at 784 nm; the hybrid mode's push–pull Vπ·L from
  electrodes 4 µm apart (the narrowest gap of their segmented electrode, taken as a plain gap), 0.8 V·cm
  measured (2.0 V on 0.4 cm, 1.0 V on 0.8 cm). The coefficients are Akiyama et al.'s r₁₃ and r₃₃ of the
  doped crystal with the bulk's other unclamped elements and ε^T. The tolerance is the paper's one digit.
  0.844 V·cm. What the main text doesn't give, and what we took: the nitride's index (the catalogue's
  LPCVD, 2.03), the electrodes' thickness (1 µm) and widths. The hybrid mode hangs on that index: the
  paper's Fig. 1(e) gives n_eff 1.90 and the power in the film and the strip 0.69 and 0.15 where we get
  1.908, 0.64 and 0.20, and a nitride index lower by 0.05 gives 1.898, 0.67 and 0.16; so the mode's
  numbers are not cases, only the Vπ·L.

Each tolerance is r₃₃'s ±0.9 pm/V carried through (±3.3%), the spans of what the paper doesn't
give, and its two digits. Wang et al. 2018's Nature modulator (doi:10.1038/s41586-018-0551-y:
2.2 and 2.3 V·cm) gives its electrodes only as "spacing < 2.5 µm between metal and waveguide
edge", so it isn't a case.

Kharel et al. 2021 (Optica 8, 357, doi:10.1364/OPTICA.416155; its erratum, Optica 8, 1218,
corrects only Fig. 4(a)'s axis) simulate regular electrodes on silicon in Table 1: 2.1, 2.6 and
3.7 V·cm at gaps of 5, 7 and 10 µm, from HFSS's field overlapped with Lumerical's mode. Their
numbers don't follow from one cross-section, and so aren't a case. Their fabricated rib, for the
measurements, is 2 µm wide, with 350 nm of a 600 nm film etched, 800 nm of gold in 1 µm of oxide
and a 30 µm signal; the sidewall here is taken at 70°. With the film's $r_{33}$, photonoxide gives
2.43, 3.70 and 5.79 V·cm on that rib (20 nm grids), 2.10 at 5 µm with the bulk's 31.2 pm/V.
Per µm of gap that is 0.486, 0.528 and 0.579 V·cm, rising with the gap. Theirs is 0.42, 0.37 and
0.37, falling, and their 4.3 µm segmented design's 1.7 V·cm is 0.40 per µm, below their 5 µm
rows'.

Vπ·L/g rising with g is what a fixed cross-section's electrostatics gives. At a fixed voltage the
field at the guide falls faster than 1/g: the film, the slab between the electrodes and the
electrodes' own thickness don't grow with the gap, and each holds the field near V/g in a narrow
gap, but less so in a wide one. The field at the rib's centre, times g/V, falls from 0.63 at 5 µm
to 0.53 at 10 µm. The rise doesn't depend on any one assumption here. Over 5 to 10 µm,
Vπ·L/g grows by 19% as computed, and stays within 11% to 19% when any one of these is changed:

- the silicon substrate replaced by oxide: 11%;
- a buried oxide 4.7 µm thick instead of 2: 15%;
- the electrodes 3 µm thick, or 50 nm: 14% and 16%;
- lithium niobate's static permittivity isotropic at 28 or 44, its two axes swapped, or 1: 18%,
  17%, 16% and 16%.

A Vπ·L/g that falls from 5 to 7 µm and is then flat is the rib's mode overlap Γ rising with the
gap. That needs a cross-section that changes between rows (a rib or a mode that widens with the
gap, say), and Table 1 doesn't give one; nor does it give the rib it simulated. Their Table's
Vπ·L column also stays at 2.1 V·cm at 5 µm across substrates and electrode designs (silicon,
quartz, segmented), so the 5 µm number may be a common reference rather than three solves. We
can't reproduce the trend without those details. Their measured 1.35 V at 20 mm (2.7 V·cm,
segmented, on quartz) is a device's.

### He et al. 2019: which coefficient the measurement probes

He et al. 2019 (Nat. Photonics 13, 359, doi:10.1038/s41566-019-0378-6) bond an x-cut film onto a
silicon circuit with BCB and etch the modulator's ribs into it: a 600 nm film (a 420 nm slab, a
180 nm rib 1 µm wide on top, 60° sidewalls), on about 300 nm of BCB over 3 µm of silica on
silicon, no cladding, gold electrodes 0.6 µm thick on the slab 2.75 µm from the rib's top edge
(w + 2g = 6.5 µm in the arXiv copy's supplement, Section II), a 19.5 µm signal and 30 µm grounds.
Vπ is 7.4 V on the 3 mm device and 5.1 V on the 5 mm one, push–pull, measured with a 100 kHz
triangular sweep (p. 3): 2.2 and 2.5 V·cm. Fig. 1b draws a silicon strip under each rib, but the
couplers move the light fully into the film over a taper that ends in an 80 nm tip (Fig. 1e, f);
a 500 × 220 nm strip 80 nm under the rib would carry the mode itself (n_eff 2.48 against 1.92),
so the arms are taken without it, and the tip's row below bounds what is left. The supplement's
simulation (COMSOL's static field, Δn = n_e³ r₃₃ E_z/2 with the in-plane field only, Lumerical's
mode) gives 2.46 V·cm without saying which r₃₃.

photonoxide's cross-section gives 2.851 V·cm with the film's r₃₃ and the bulk's other clamped
elements and ε^S, on 10 nm static and optical cells (2.852 on 20 nm). BCB isn't in the catalogue;
its static permittivity is taken as 2.65 and its index as 1.54, and the rows below bracket both.
Each candidate changes one thing:

| Candidate | Vπ·L (V·cm) | Against 2.851 |
|---|---|---|
| **The coefficient and its clamping** (Jazbinšek & Zgonik, Tables 4 and 5, 633 nm) | | |
| bulk clamped r^S (r₃₃ = 31.2 pm/V) and ε^S | 2.464 | −13.6% |
| bulk unclamped r^T (31.45) with ε^S | 2.429 | −14.8% |
| bulk unclamped r^T and ε^T | 2.400 | −15.8% |
| the film's tensor with r₃₃ = 28.8, Table 4's measured clamped input (line 74) | 2.666 | −6.5% |
| **The static permittivities** | | |
| the film's r with ε^T (84.48, 27.78) for ε^S (45.5, 26.2) | 2.811 | −1.4% |
| BCB's static permittivity 2.5 | 2.851 | 0.0% |
| BCB as silica (3.9, index 1.444) | 2.811 | −1.4% |
| the silicon substrate a conductor (doped, at 100 kHz) | 2.972 | +4.2% |
| **The cross-section** | | |
| the gap from the rib's foot, 6.71 µm | 2.955 | +3.7% |
| electrodes 50 nm or 1.1 µm thick | 2.888, 2.829 | +1.3%, −0.8% |
| sidewalls at 70° or 90° | 2.845, 2.830 | −0.2%, −0.7% |
| a 620 nm film, its slab 440 nm (20 nm cells) | 2.803 | −1.7% |
| the taper's 80 nm silicon tip under the rib, a conductor | 2.882 | +1.1% |
| **The definition** | | |
| the paper's Eq. (1): r₃₃ and the in-plane field only, the film's r₃₃ | 2.905 | +1.9% |
| the same with the bulk's r^S and ε^S, or r^T and ε^T | 2.505, 2.444 | −12.1%, −14.3% |
| one arm instead of push–pull | 5.701 | +100% |

Clamping moves Vπ·L by 2.6% (r^S and ε^S to r^T and ε^T: 2.464 to 2.400), since r₃₃^T and
r₃₃^S differ by 0.8% in the fit (Jazbinšek & Zgonik's Fig. 1: "unresolvable"). Every
structural or electrostatic candidate moves it by 4.2% or less. The paper's Eq. (1) with a bulk
coefficient gives 2.44 to 2.51, around their 2.46, so their simulation agrees with this one and
took a bulk r₃₃ of about 31 pm/V; it doesn't say which coefficient their film has.

Only r₃₃ itself closes the gap: 2.5 V·cm needs 30.7 pm/V and 2.2 V·cm needs 34.9 pm/V, against the
film's 26.9 ± 0.9 at 1550 nm (Chelladurai et al. 2025, flat from 100 MHz to 330 GHz), which
gives Wang et al.'s and Zhang et al.'s Vπ·L to 2 and 3% (both measured at DC). The 3 mm device's
34.9 pm/V is above even the bulk's unclamped 31.45 pm/V at 633 nm, so the bulk coefficients
don't explain it either: with r^T and ε^T, 2.400 V·cm is 9% above the 3 mm device's 2.2 and 4%
below the 5 mm device's 2.5, and they put Wang et al.'s and Zhang et al.'s at 1.56 and 2.02 against
1.8 and 2.3.

The two devices also disagree with each other. Vπ ∝ 1/L would take the 3 mm device's 7.4 V to
4.44 V at 5 mm, not 5.1 V: their products, 2.22 and 2.55 V·cm before rounding, differ by 15%,
more than any candidate above but r₃₃. One Vπ·L fits both only if each active length is 1.43 mm
longer than its arm, and that gives 3.28 V·cm.

What is left is the measurement's frequency. Below 1 MHz a thin film's response needn't be its
high-frequency one: Holzgrafe et al. 2024 measure modulators whose response rises up to 2.5 dB
(×1.33) above its 100 kHz to 1 MHz value in a band between 1 and 20 kHz, from conduction at the
film's surface. ×1.33 takes 2.851 to 2.14 V·cm, enough to span both devices. Where the band falls depends on the layers'
conductivities, and He et al.'s film is bonded on BCB and has no cladding, unlike Holzgrafe et
al.'s, so it can't be computed from what the papers give.

**What would settle it:** the same devices' Vπ over frequency, from 1 kHz to a few MHz, joined to
the electro-optic S21 at its low end (a Vπ that rises towards 2.85 V·cm above the band would be
the relaxation); more lengths than two, or the electrodes' length against the arms'; and the
published Supplementary Information, which may name the r₃₃ behind the 2.46 V·cm (that settles
their simulation, not the device). Until then He et al.'s modulator isn't a case (#420, #595).

### Barium titanate

The catalogue's films ([catalogue.md](catalogue.md)): Abel et al. 2018's tensor on silicon
(r₄₂ = 923 ± 215, r₃₃ = 342 ± 93 pm/V at 1550 nm, r₁₃ unresolved), Deng et al. 2026's r₄₂ = 358
pm/V on LSAT, and Chelladurai et al. 2025's against frequency (r₃₃ = 125 pm/V and
½(r₁₃ + 2r₄₂) = 481 pm/V at 100 MHz, 191 at 330 GHz), with the index at 1550 nm as constants
(2.26, Chelladurai et al.; 2.29, Eltes et al.): the spread is the films', from their growth, their
domains and the field's frequency. Its index model is Abel et al.'s film at 1550 nm (Supplementary
Note 6: $\bar n$ = 2.286, $\Delta n$ = 0.03), constant from 1.50 to 1.60 µm. Its cases check the
tensor's turn in the plane against the projection the papers use, the one-arm Vπ·L against Eltes
et al.'s own numbers, and Abel et al.'s guide:

- **Abel et al. 2018** (Nat. Mater. 18, 42, doi:10.1038/s41563-018-0208-0, Methods and
  Supplementary Notes 6 and 8): a 100 nm silicon strip 750 nm wide on 225 nm of barium titanate
  on silica, clad in silica (the Methods say air; the supplement's simulation and the devices
  have silica). The TE mode at 1550 nm carries 41% of its power in the film (their 39%). Abel et
  al. take the field in the film as uniform, $E = V/g$ across the guide with the electrodes'
  gap $g$ = 4.75 µm, and their single domain has its c axis in the plane at 45° from it, with
  $r_{42}$ = 923, $r_{33}$ = 341 and $r_{13}$ = −56 pm/V. The mode sees the film's in-plane
  average, $(\varepsilon_o + \varepsilon_e)/2$ across and along and $\varepsilon_o$ up: the single
  domain's tensor without its coupling of the guide's axis to the cross-section, which the
  full-vector solver doesn't take and which enters the TE mode at second order. The domain's
  $\Delta\varepsilon = -\varepsilon\Delta\eta\varepsilon$ is applied by first-order perturbation.
  $\lvert dn_\mathrm{eff}/dV\rvert$ = 3.446e-4, 3.404e-4 and 3.395e-4 per volt on 40, 20 and
  10 nm cells, against their 3.4e-4 (FDTD, 33 nm cells). Through their net domain population
  $\xi = 2(0.376 - 0.124) = 0.504$ (Note 8), $V_\pi L = \lambda/(2R\xi)$ = 0.453 V·cm against
  their 0.45. A cross-section's, per unit length, for a uniform field as theirs; the electrodes'
  own field isn't solved, since Abel et al. don't give their thickness or cladding.

### Lithium tantalate

Casson et al. 2004 (J. Opt. Soc. Am. B 21, 1948, doi:10.1364/JOSAB.21.001948, Table 2) measure the
unclamped r₁₃ and r₃₃ from 632.8 to 1558 nm by the deflection of a beam in a domain-patterned
crystal: 6.92 ± 0.07 and 27.4 ± 0.3 pm/V at 1558 nm, with the indices there (n_e = 2.1224, n_o =
2.1186: positive and a tenth of lithium niobate's birefringence). They are the catalogue's
`litao3`; r₂₂ and r₅₁ aren't measured. Its index model is our fit to Bond's table (1965, 0.45 to
4.0 µm at room temperature, within 6.7e-4 of it; [catalogue.md](catalogue.md)), so a lithium
tantalate film can be a stack's material.

## As a job

An `"electrostatic"` static job ([static.md](static.md)) with an `electro_optic` table runs the
chain: its stack holds the oriented crystal, its `conductor` rectangles are the electrodes at
their potentials, and the light travels along x, the mode solved on the cut x = `cut_x_um` over
the table's own window and step. The crystal's static permittivity is the catalogue's ε^S
(`static_permittivity: unclamped` for ε^T) turned by its orientation, unless
`[task.permittivity]` names it. The run records the mode's n_eff, dn_eff/dV by perturbation and
by re-solve, and Vπ·L per volt of the conductors' span, with the mode's grid;
`convergence: { tolerance: 0.01 }` (V·cm) checks Vπ·L on both grids twice and four times as
coarse, as a modes job's `[task.convergence]` checks its indices.

```yaml
task:
  kind: static
  physics: electrostatic
  y_um: [-16.75, 6.75]      # the ground electrode's far edge to the signal's centre line
  z_um: [-20.0, 23.8]
  step_nm: 1000.0
  fine_nm: 20.0
  growth: 1.15
  stack:
    substrate: si
    cladding: sio2
    layer:
      - { name: BOX, thickness_um: 2.0, material: sio2, background: sio2 }
      - { name: LN, thickness_um: 0.7, material: { id: linbo3, orientation: x-cut }, background: sio2 }
  rect:
    - { layer: LN, center_um: [0.0, 0.0], size_um: [10.0, 0.9], etch: { depth_um: 0.4, sidewall_deg: 70.0 } }
  permittivity: { Si: 11.7, SiO2: 3.9 }
  conductor:
    - { y_um: [-17.75, -1.75], z_um: [2.3, 2.6], potential_v: 0.0 }
    - { y_um: [1.75, 7.75], z_um: [2.3, 2.6], potential_v: 1.0 }
  electro_optic:
    layer: LN
    entry: linbo3
    tensor: "thin film (x-cut LNOI), clamped, 1550 nm"
    complete_from: "bulk, clamped, 633 nm"
    wavelength_um: 1.55
    y_um: [-3.0, 3.0]
    z_um: [0.5, 4.0]
    step_nm: 20.0
    convergence: { tolerance: 0.02 }
```

This is Wang et al. 2018's cross-section under oxide throughout: 1.874 V·cm push–pull at 20 nm,
the re-solve within 0.05% (the job's tests; the library's case with 3 µm of oxide over the slab
gives 1.863 on the same grids, its cells sampled its own way).

## Not yet

- A barium titanate cross-section's Vπ·L for Eltes et al. 2019's and Deng et al. 2026's devices:
  their coefficients are checked (`pockels/bto-eltes`, `pockels/bto-deng`), and the films' indices
  are in the catalogue ([catalogue.md](catalogue.md)).
- Lithium tantalate's r₂₂ and r₅₁, its static permittivity, and a thin-film modulator's Vπ·L
  (C. Wang et al., Nature 629, 784 (2024), doi:10.1038/s41586-024-07369-1). Its index against
  wavelength is in the catalogue (above).
- He et al. 2019's modulator as a case: its cross-section gives 2.85 V·cm push–pull with the thin
  film's r₃₃ against the 2.2 and 2.5 V·cm measured at 100 kHz; no structural, permittivity or
  clamping candidate closes it (the section above; #595).

The response in frequency is the travelling-wave electrodes' ([travelling-wave.md](travelling-wave.md))
and the TFLN modulator's ([devices](devices.md)).
