//! The Academy's diagrams: a lesson's device drawn before any chart, as a labelled schematic in
//! the lesson's own symbols (an SVG the window draws, studio/src/components/diagrams/) and, where
//! a built-in job builds the same device, in 3D from that job's structure, as the job builder
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
            job: Some("ring-fdfd.toml"),
            caption_3d: "The ring of jobs/ring-fdfd.toml as photonoxide's geometry builds it: a \
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
            job: Some("bragg-grating.toml"),
            caption_3d: "The waveguide grating of jobs/bragg-grating.toml as photonoxide's \
                         geometry builds it: a silicon strip 220 nm thick on oxide whose width \
                         steps between 600 and 400 nm, ten periods of 320 nm, deeper steps than \
                         a filter's so that they show. Drag to turn it.",
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
