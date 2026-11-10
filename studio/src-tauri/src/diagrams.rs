//! The Academy's diagrams: a lesson's device drawn before any chart, as a labelled schematic in
//! the lesson's own symbols (an SVG the window draws, studio/src/components/diagrams/) and, where
//! a built-in job builds the same device, in 3D from that job's structure, as the Custom device
//! previews it. `::diagram <id>` shows one (academy/README.md).

use serde::Serialize;

/// A diagram: what it is called, what its drawing and its 3D view show.
#[derive(Serialize, Clone, Debug)]
pub struct DiagramSpec {
    pub id: &'static str,
    pub title: &'static str,
    /// What the schematic shows, as its caption; may hold TeX between dollars.
    pub caption: &'static str,
    /// The built-in job (jobs/<file>) whose structure the 3D view shows, if there is one.
    pub job: Option<&'static str>,
    /// What the 3D view shows, as its caption; empty without a job.
    pub caption_3d: &'static str,
}

/// Every diagram, by id.
pub fn specs() -> Vec<DiagramSpec> {
    vec![
        DiagramSpec {
            id: "dispersion",
            title: "A bound charge, and a glass's index against frequency",
            caption: "At the left, the model behind every dispersion formula: a charge $-e$ of \
                      mass $m$, held to its ion by a spring of resonance frequency $\\omega_0$ \
                      and slowed at the rate $\\gamma$, driven along $x$ by the light's field \
                      $E_0 e^{-i\\omega t}$. Its displacement $x(t)$ makes a dipole $p = -ex$, \
                      and many such dipoles make the medium's polarization. At the right, a \
                      sketch of the refractive index $n$ (solid) and the extinction $\\kappa$ \
                      (dashed) of a glass against the frequency $\\omega$: a resonance of the \
                      lattice in the infrared, $\\omega_\\text{IR}$, and one of the electrons in \
                      the ultraviolet, $\\omega_\\text{UV}$, each an absorption line across \
                      which $n$ falls; between them a transparent window where $\\kappa$ is \
                      small and $n$ rises with $\\omega$. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "leaky",
            title: "A leaky multilayer, and a bend from above",
            caption: "At the left, a planar guide in section: a cover $n_c$, films $n_1 \\ldots \
                      n_J$, and a substrate $n_s$ whose index is above the mode's \
                      $\\mathop{\\mathrm{Re}} n_\\text{eff}$. The mode travels along $z$ as \
                      $e^{ikn_\\text{eff}z}$ and sheds a plane wave into the substrate at an \
                      angle $\\theta$ from the faces' normal, weaker further along as the mode \
                      loses power, so $\\mathop{\\mathrm{Im}} n_\\text{eff} \\gt 0$. Its field \
                      $\\lvert\\psi(x)\\rvert$ decays into the cover and grows into the \
                      substrate as $e^{ik\\alpha_s x}$. At the right, a guide of index $n_1$ in a \
                      cladding $n_2$, bent at a radius $R$ about $O$, seen from above: the mode \
                      goes round as $e^{i\\nu\\varphi}$ with $\\nu = kn_\\text{eff}R$, so its \
                      phase fronts are radial and turn together, faster the further out. At \
                      $\\rho_r = R\\thinspace n_\\text{eff}/n_2$ (dashed) they would have to \
                      outrun light in the cladding, and from there the field radiates, forwards \
                      and outwards. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "process",
            title: "A silicon strip made, step by step",
            caption: "Five cross-sections of one wafer, left to right. 1: the SOI wafer as \
                      bought, a single-crystal silicon film $t$ thick on a buried oxide $h$ thick \
                      over a silicon handle. 2: resist spun on and exposed through a mask by light \
                      of wavelength $\\lambda$ through a lens of numerical aperture NA; the mask's \
                      bar shades the strip. 3: the developer washes away the exposed resist (a \
                      positive resist), leaving a line $w$ wide. 4: the silicon etched where the \
                      resist leaves it bare, the etch stopping on the oxide, which it attacks far \
                      more slowly. 5: the resist stripped and oxide deposited over everything: a \
                      strip $w$ by $t$ clad in oxide. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "ring",
            title: "A ring resonator, seen from above",
            caption: "A ring of radius $R$ and waveguide width $w$, a gap $g$ from the bus. At the \
                      coupler a share $r_1$ of the field goes straight on and a share $k_1$ \
                      crosses over; one trip round the ring, of length $L = 2\\pi R$, \
                      multiplies it by $a e^{i\\phi}$. The light enters as $a_1$ and leaves the \
                      through port as $b_1$; $b_2$ sets off round the ring and $a_2$ comes back. \
                      An all-pass ring has only the lower bus, its coupler's $r$ and $k$; the \
                      faint upper bus makes it an add-drop ring, $r_2$ and $k_2$ its coupler. \
                      The charts' input and drop couplings, $\\kappa_1^2$ and $\\kappa_2^2$, are \
                      $k_1^2$ and $k_2^2$. Not to scale: the gap and the width are drawn wide.",
            job: Some("ring-fdfd.yaml"),
            caption_3d: "The ring of jobs/ring-fdfd.yaml as photonoxide's geometry builds it: a \
                         500 nm silicon wire 220 nm thick on oxide, bent into a ring of 2 µm \
                         radius 100 nm from its bus. Drag to turn it.",
        },
        DiagramSpec {
            id: "bragg",
            title: "A Bragg mirror and a waveguide grating",
            caption: "Above, a quarter-wave stack in section: $N$ pairs of layers of index $n_H$ \
                      and $n_L$, thicknesses $d_H$ and $d_L$, a period $\\Lambda = d_H + d_L$, \
                      between a cover of index $n_0$ and a substrate of index $n_s$. Light comes \
                      in from the cover; $r$ is the field reflected and $t$ the field \
                      transmitted. Below, the same idea in a waveguide seen from above: its width \
                      steps between wide and narrow sections, whose effective indices take the \
                      place of $n_H$ and $n_L$, over a length $L = N\\Lambda$. Not to scale.",
            job: Some("bragg-grating.yaml"),
            caption_3d: "The waveguide grating of jobs/bragg-grating.yaml as photonoxide's \
                         geometry builds it: a silicon strip 220 nm thick on oxide whose width \
                         steps between 600 and 400 nm, ten periods of 320 nm, deeper steps than \
                         a filter's so that they show. Drag to turn it.",
        },
        DiagramSpec {
            id: "grating",
            title: "A grating coupler, in section along its guide",
            caption: "A silicon film $t$ thick, its grooves etched $e$ deep at a period \
                      $\\Lambda$, on a buried oxide $h$ thick over the silicon substrate, under a \
                      cladding of index $n_c$. The guided mode, of effective index \
                      $n_\\text{eff}$, comes in from the left and leaks out as it crosses the \
                      grating: upwards at $\\theta$ from the normal, where the first diffraction \
                      order satisfies $n_c \\sin\\theta = n_\\text{eff} - \\lambda/\\Lambda$, \
                      towards a fibre tilted at the same angle; downwards into the oxide, where \
                      the substrate reflects it back up with $r_b$; and onwards through the \
                      grating's end. Just above the grating, the power a uniform grating sends \
                      up, $e^{-2\\alpha z}$, fading as the guide empties, against the fibre's \
                      Gaussian $\\lvert G(z)\\rvert^2$, whose mode field diameter is $2w_0$. The \
                      rays fade with it. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "crystal",
            title: "A photonic-crystal slab, seen from above",
            caption: "A triangular lattice of air holes of radius $r$, a lattice constant $a$ \
                      apart, in a film of permittivity $\\varepsilon$. A row left out is the W1 \
                      waveguide: light at a frequency in the crystal's gap travels along it and \
                      nowhere else. Three holes left out in a line are the L3 cavity, its two end \
                      holes moved outwards by $s$ (dashed: where they were). Beside it, the \
                      lattice's Brillouin zone, a hexagon of wavevectors: $\\Gamma$ at its centre, \
                      $k = 0$, M at the middle of an edge and K at a corner. Bands are drawn along \
                      the edge of the shaded wedge, $\\Gamma$–M–K–$\\Gamma$, which by the lattice's \
                      symmetry holds every distinct wavevector. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "metalens",
            title: "A metalens in section, and a nanofin from above",
            caption: "Pillars of height $H$, one every period $p$, on a substrate, lit from below \
                      by a plane wave. Each pillar's width $w$ sets the phase it gives the light, \
                      and the lens needs $\\varphi(r) = -k(\\sqrt{r^2 + f^2} - f)$, wrapped into \
                      $[0, 2\\pi)$: the widths repeat zone by zone, the zones narrowing towards \
                      the rim (beside, $\\varphi$ against $r$). The light leaves as a wave \
                      converging on the focus, a focal length $f$ above the lens; the ray from \
                      the rim, a half-width $R$ from the axis, arrives at $\\theta$ from it, and \
                      $\\mathrm{NA} = \\sin\\theta$. Beside it, a nanofin from above, rotated by \
                      $\\alpha$: lit with one circular polarization, it sends part of the light \
                      into the other with the phase $2\\alpha$, whatever its width. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "fdfd",
            title: "A frequency-domain problem and its adjoint",
            caption: "A 2D grid closed by PMLs, where each derivative along $w$ is divided by \
                      the stretch $s_w$; a guide through it, a source $b$ that launches its mode, \
                      and a monitor that measures the mode's amplitude $a = c^\\mathsf{T}x$. \
                      Between them a design region of cells $\\varepsilon_k$. The forward field \
                      $x$ solves $A(\\varepsilon)\\,x = b$; the adjoint field $\\lambda$ solves \
                      $A^\\mathsf{T}\\lambda = \\partial F/\\partial x$, a source at the monitor, \
                      and travels back from it. Their product in each cell is the figure of \
                      merit's derivative with respect to that cell's permittivity, every cell \
                      from the same two solves. Beside it, Yee's grid for $E$ along $z$: $E_z$ at \
                      the cells' centres, $H_x$ and $H_y$ on their faces, and the five values of \
                      $E_z$ one row of $A$ couples. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "nonlinear",
            title: "A poled lithium niobate guide, its ridge in section, and a Kerr ring",
            caption: "Above, a periodically poled thin-film lithium niobate guide seen from \
                      above: the film's nonlinear coefficient alternates between $+d$ and $-d$ \
                      in domains one period $\\Lambda$ apart, over a length $L$. The fundamental \
                      $\\omega$ enters at the left; the fundamental and its second harmonic \
                      $2\\omega$ leave at the right. The harmonic grows along the whole length \
                      when the grating's wave vector $2\\pi/\\Lambda$ makes up the mismatch \
                      between the waves, $\\Delta k = 0$. Below at the left, the guide in \
                      section: an x-cut film $t$ thick etched $e$ deep to a ridge $w$ wide, on \
                      silica, air above; the crystal's $c$ axis lies across the guide, along the \
                      TE modes' main field. The fundamental's mode and the harmonic's smaller \
                      one share the ridge, and the coupling $\\kappa$ is their overlap through \
                      the $d$ tensor. Below at the right, a ring of a Kerr material seen from \
                      above, pumped at $\\omega_p$ through its bus: four-wave mixing \
                      $2\\omega_p = \\omega_s + \\omega_i$ builds a comb of lines one free \
                      spectral range $D_1/2\\pi$ apart, a signal $\\omega_s$ and an idler \
                      $\\omega_i$ either side of the pump. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "mzi",
            title: "A Mach–Zehnder interferometer, seen from above",
            caption: "Light enters the lower guide. The splitter, a coupler, keeps a share $r_1$ \
                      of the field in its guide and crosses a share $k_1$ over \
                      ($r_1^2 + k_1^2 = 1$; the charts' $\\kappa_1^2$ is $k_1^2$). The two arms, \
                      of lengths $L$ and $L + \\Delta L$, multiply their fields by \
                      $a_l e^{i\\phi_l}$ and $a_u e^{i\\phi_u}$, and a phase shifter adds \
                      $\\varphi$ to the upper one. The combiner, $r_2$ and $k_2$, mixes them \
                      again: what leaves each output depends on the phase difference \
                      $\\Delta\\phi$. The bar output is the guide the light came in by, the \
                      cross output the other. A Y-branch or a 1 × 2 MMI in place of a coupler \
                      splits evenly, and as a combiner has one output. Not to scale: the \
                      arms are drawn short.",
            job: Some("mzi-mmi.yaml"),
            caption_3d: "The interferometer of jobs/mzi-mmi.yaml as photonoxide's geometry \
                         builds it: 500 nm silicon wires 220 nm thick on oxide, between two \
                         1 × 2 MMIs 3 µm wide and 8.55 µm long, built as Dwivedi et al.'s were. \
                         The upper arm is 6 µm longer; its corners are square to keep the \
                         picture small, where a chip bends them. Drag to turn it.",
        },
        DiagramSpec {
            id: "coupler",
            title: "A directional coupler, seen from above",
            caption: "Two strips of width $w$ run a gap $g$ apart for a length $L$, brought \
                      together and apart by S-bends. Light enters the lower strip at $z = 0$. \
                      Across the pair the coupled section has two modes, drawn beneath: the even \
                      supermode, of index $n_\\text{even}$, and the odd one, of $n_\\text{odd}$. \
                      Light in one strip is the two in step; they slip out of step by $\\pi$ over \
                      $L_c = \\lambda / 2\\Delta n$, $\\Delta n = n_\\text{even} - n_\\text{odd}$, \
                      and the light is then in the other strip. A share $\\cos^2 \\kappa L$ \
                      leaves by the through port and $\\sin^2 \\kappa L$ by the cross port, \
                      $\\kappa = \\pi\\Delta n / \\lambda$. Not to scale: the width and the gap \
                      are drawn wide and the length short.",
            job: Some("coupler-supermodes.yaml"),
            caption_3d: "The coupled section of jobs/coupler-supermodes.yaml as photonoxide's \
                         geometry builds it: two silicon strips 500 nm wide and 220 nm thick on \
                         oxide, 200 nm apart, 10 µm of them. Run, the job solves their two \
                         supermodes as the gap goes from 100 to 400 nm. Drag to turn it.",
        },
        DiagramSpec {
            id: "mmi",
            title: "A 1 × 2 multimode interference splitter, seen from above",
            caption: "A section $W$ wide and $L$ long, fed in the middle of its left face by a \
                      guide $w_a$ wide. Its guided modes, the first four $\\psi_0 \\ldots \\psi_3$ \
                      drawn beneath across $W$, travel along $z$ at their own propagation \
                      constants $\\beta_\\nu$, which fall nearly as $\\nu(\\nu + 2)$; their \
                      interference, shaded, spreads the input's single spot across the section \
                      and gathers it again. At $L = 3L_\\pi/8$, $L_\\pi$ the beat length of the \
                      two lowest modes, it forms two images of the input at $\\pm W/4$, where two \
                      output guides take equal shares, in phase. The shading is computed from \
                      Soldano and Pennings's sine-like modes and quadratic phases. Not to scale: \
                      the section is drawn short for its width.",
            job: Some("mmi-fdfd.yaml"),
            caption_3d: "The 1 × 2 MMI of jobs/mmi-fdfd.yaml as photonoxide's geometry builds \
                         it: a silicon section 3 µm wide, 8.55 µm long and 220 nm thick on \
                         oxide, fed by a 500 nm guide and left by two at ±0.75 µm. Run, the job \
                         solves it by 2D FDFD from 1.5 to 1.6 µm. Drag to turn it.",
        },
        DiagramSpec {
            id: "crossing",
            title: "A crossing, an asymmetric coupler and a splitter-rotator, from above",
            caption: "Above at the left, a waveguide crossing from above: two guides $w$ wide, \
                      each widened to $W$ over a taper $L_t$ long on either side of the centre, \
                      so the beam that crosses the open centre is wider and spreads less; under \
                      them, dashed, two ellipses of a thinner slab that keep guiding it there. \
                      TE₀ enters at the left and leaves at the right; what the side arms take is \
                      crosstalk. Above at the right, an asymmetric directional coupler: a guide \
                      $w_a$ wide a gap $g$ from one $W$ wide over a length $L$. Where the narrow \
                      guide's TE₀ has the index of the wide guide's TE₁, its single lobe crosses \
                      over as TE₁'s two lobes of opposite sign. Below, a polarization \
                      splitter-rotator from above: its guide widened from $w_1$ to $w_2$ over $L$ \
                      under silicon nitride, where TM₀ turns into TE₁, then a coupler that passes \
                      TE₁ to the upper guide as TE₀; beside it, the guide in section, silicon on \
                      silica under nitride, which is not mirror-symmetric up and down. Not to \
                      scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "modulator",
            title: "A Mach–Zehnder modulator and a heater, from above and in section",
            caption: "Above, a push–pull Mach–Zehnder modulator seen from above: light of group \
                      index $n_o$ is split between two arms that run through the two gaps of a \
                      ground–signal–ground electrode $L$ long. A voltage $V(t)$ launched on the \
                      signal from a source of resistance $R_G$ travels along it as an RF wave of \
                      index $n_m$ and ends in a load $R_L$; at each instant its field across the \
                      upper gap, arrowed, points opposite to the one across the lower, so the \
                      arms' indices move apart. Below at the left, the same in section, x-cut \
                      thin-film lithium niobate: a film etched to two ribs, each in a gap $g$ \
                      wide between the electrodes on the film's slab, on buried oxide over \
                      silicon. The crystal's $c$ axis lies across the guides, along the field, so \
                      the TE mode's index $n_e$ moves by $-\\tfrac{1}{2}n_e^3r_{33}E$. Below at the \
                      right, a thermo-optic phase shifter in section: a heater a height $h$ above \
                      a silicon strip, dissipating $P'$ per unit length; the heat spreads down \
                      through the oxide to the substrate, held at $T_0$, and the strip's rise \
                      $\\Delta T$ moves its index by $(dn/dT)\\,\\Delta T$. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "plasmon",
            title: "A surface plasmon, and a metal sphere's localized plasmon",
            caption: "At the left, a flat interface between a dielectric of permittivity \
                      $\\varepsilon_d > 0$ above and a metal of $\\varepsilon_m < 0$ below. A \
                      wave of surface charge, $+$ and $-$, travels along $z$ as \
                      $e^{i(kz - \\omega t)}$, and its amplitude falls as $e^{-z/2L}$, $L$ the \
                      propagation length of its power. Its field's lines loop from each positive \
                      charge to the next negative one, far into the dielectric and a little way \
                      into the metal; at the right of the panel, the field's size $\\lvert E \
                      \\rvert$ across the interface falls by $e$ over $\\delta_d$ into the \
                      dielectric and over the shorter $\\delta_m$ into the metal. At the right, \
                      a metal sphere of radius $\\rho$ and permittivity $\\varepsilon$ in a \
                      dielectric $\\varepsilon_d$, much smaller than the wavelength, lit by a \
                      plane wave whose field $E_0$ pushes its electrons to one side: the \
                      charges left on its surface make a dipole $p$, which rings at the \
                      frequency where $\\varepsilon = -2\\varepsilon_d$ and sends light out in \
                      every direction. Not to scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "slab",
            title: "A slab waveguide, in section",
            caption: "A film of index $n_2$ and thickness $d$ lies between a cover of index $n_1$ \
                      and a substrate of index $n_3$, both lower. A ray meeting a face at an angle \
                      $\\theta$ from its normal larger than the critical angle \
                      $\\theta_c = \\arcsin(n_3/n_2)$ is totally reflected, and zig-zags along \
                      $z$ inside the film; a steeper ray is partly refracted out at every \
                      reflection and lost. At the right, a guided mode's field across the slab, \
                      $x$ up from the substrate's face: a cosine $\\cos(hx - \\phi_3)$ in the \
                      film, and tails $e^{px}$ in the substrate and $e^{-q(x - d)}$ in the \
                      cover, shorter where the index is lower. Not to scale.",
            job: Some("slab-soi.yaml"),
            caption_3d: "The film of jobs/slab-soi.yaml as photonoxide's geometry builds it: \
                         220 nm of silicon on the buried oxide of an SOI wafer, under an oxide \
                         cladding, unetched, so that across it the structure is a slab. Drag to \
                         turn it.",
        },
        DiagramSpec {
            id: "strip",
            title: "A strip waveguide, in section",
            caption: "A core of index $n_1$, $w$ wide and $t$ thick, in a cladding of index \
                      $n_2$ on a buried oxide $h$ thick over the silicon substrate; light \
                      travels along $z$, out of the page. The shaded contours are the TE-like \
                      mode's field, whose main component $E_x$ lies across the width. At the \
                      four corners, circled, the field's derivatives are singular. Marcatili's \
                      approximation solves the core and the four regions beside its faces, and \
                      leaves out the four corner regions, shaded. At the right, the effective \
                      index method: a slab across the thickness gives an index $N$, and a slab \
                      of index $N$ and width $w$ in the cladding gives the mode's $n_\\text{eff}$. \
                      Not to scale.",
            job: Some("strip-modes.yaml"),
            caption_3d: "The strip of jobs/strip-modes.yaml as photonoxide's geometry builds it: \
                         a silicon strip 500 nm wide and 220 nm thick on the buried oxide of an \
                         SOI wafer, under an oxide cladding. Run, the job solves its modes and \
                         their indices from 1.5 to 1.6 µm. Drag to turn it.",
        },
        DiagramSpec {
            id: "yee",
            title: "Yee's grid and the leapfrog",
            caption: "At the left, one cell of Yee's grid, $\\Delta x$ by $\\Delta y$ by \
                      $\\Delta z$, $x$ to the right, $y$ into the page and $z$ up. The electric \
                      field's components $E_x$, $E_y$ and $E_z$ sit at the middles of the cell's \
                      edges along them; the magnetic field's, $\\tilde H_x$, $\\tilde H_y$ and \
                      $\\tilde H_z$, at the centres of the faces normal to them. Each E is circled \
                      by four H and each H by four E, so every curl is a centred difference half \
                      a cell wide. At the right, the leapfrog in time: E at the whole steps \
                      $n - 1$, $n$, $n + 1$, H at the half steps between, each advanced from the \
                      other's curl, $\\Delta t$ apart; it is stable while \
                      $c\\Delta t \\le 1/\\sqrt{\\sum_i 1/\\Delta_i^2}$. One of each component is \
                      drawn: $E_x$ lies on all four edges along $x$, and $\\tilde H_x$ on both \
                      faces normal to $x$, each shared with the neighbouring cells. Not to \
                      scale.",
            job: None,
            caption_3d: "",
        },
        DiagramSpec {
            id: "quantum",
            title: "A quantum photonic chip",
            caption: "Above, a chip from the left to the right. A pump of frequency \
                      $\\omega_p$ is split onto two guides, and a ring on each turns pairs of \
                      pump photons into a signal and an idler, $2\\omega_p = \\omega_s + \\omega_i$. \
                      The photons enter a mesh of Mach–Zehnder cells, each a beam splitter set by \
                      $\\theta$ and a phase $\\phi$, which acts on the four modes' annihilation \
                      operators as a unitary, $U = D\\prod T_{m,m+1}(\\theta, \\phi)$; with its \
                      loss it is a matrix $S$, and the lost light lets the vacuum $\\hat c$ in. \
                      Four detectors of efficiency $\\eta_D$ count the photons; the probability of \
                      a pattern is the squared permanent of a submatrix, or for squeezed light a \
                      hafnian. Below, two photons, one delayed by $\\tau$, meet on a coupler of \
                      reflectance $R$ and transmittance $T$; alike and on time, they leave \
                      together by one output or the other, and their coincidences dip against \
                      $\\tau$ as $R^2 + T^2 - 2RT e^{-\\sigma^2\\tau^2}$. Not to scale.",
            job: None,
            caption_3d: "",
        },
    ]
}

/// The diagram `id`, if there is one.
pub fn spec(id: &str) -> Option<DiagramSpec> {
    specs().into_iter().find(|d| d.id == id)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Each diagram has its drawing in the window, named in the window's list of drawings, and
    /// its job, if it has one, is built in.
    #[test]
    fn every_diagram_is_drawn_and_its_job_exists() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/components/diagrams");
        let index = std::fs::read_to_string(dir.join("index.ts")).unwrap();
        let ids: Vec<&str> = specs().iter().map(|d| d.id).collect();
        for (k, d) in specs().iter().enumerate() {
            assert!(!ids[..k].contains(&d.id), "two diagrams called {}", d.id);
            assert!(!d.title.is_empty() && !d.caption.is_empty(), "{}", d.id);
            let name = component(d.id);
            assert!(
                dir.join(format!("{name}.svelte")).is_file(),
                "{}: no drawing {name}.svelte in studio/src/components/diagrams",
                d.id
            );
            assert!(
                index.contains(&format!("{:?}: {name},", d.id)),
                "{}: {name} isn't in studio/src/components/diagrams/index.ts",
                d.id
            );
            match d.job {
                Some(job) => {
                    assert!(
                        crate::examples::jobs().iter().any(|j| j.file == job),
                        "{}: no built-in job {job}",
                        d.id
                    );
                    assert!(
                        !d.caption_3d.is_empty(),
                        "{}: a 3D view without a caption",
                        d.id
                    );
                }
                None => assert!(
                    d.caption_3d.is_empty(),
                    "{}: a caption for no 3D view",
                    d.id
                ),
            }
        }
        assert!(spec("ring").is_some() && spec("no-such").is_none());
    }

    /// A diagram's drawing's component: its id in upper camel case, `ring` as `Ring`.
    fn component(id: &str) -> String {
        id.split('-')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect::<String>())
                    .unwrap_or_default()
            })
            .collect()
    }
}
