//! The Chip page's quantum view's sampling part: photons on the chip's ports sampled live by the
//! library's exact sampler (`Transfer::sample`, Clifford and Clifford's algorithm B), against the
//! exact distribution of the same photons (`Transfer::distribution`), each call held to a hard
//! time limit ([`chip_sample`]).
//!
//! The window calls it batch after batch, handing back the counts so far: the command adds a
//! batch, each with a seed of its own, and returns the counts, the total variation distance to
//! the exact distribution, and the distance a correct sampler is expected to reach at that many
//! samples.

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::path::Path;
use std::time::{Duration, Instant};

use photonoxide::quantum::{Distribution, Fock, Photons, Transfer};
use serde::{Deserialize, Serialize};

use super::quantum::{
    Input, MOST_PATTERNS, MOST_PHOTONS, MOST_PORTS, SHOWN_PATTERNS, in_sweep, patterns_of,
    port_names,
};
use super::{Chip, complete};
use crate::studio::Studio;

/// The most samples one call draws.
pub const MOST_BATCH: usize = 20_000;
/// The longest a call may sample before it stops, whatever its batch: the hard limit, ms.
pub const MOST_LIMIT_MS: u64 = 5_000;
/// The most samples counted in all.
pub const MOST_SAMPLES: usize = 10_000_000;

/// A pattern at the ports and how many samples fell on it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Count {
    pub occupations: Vec<usize>,
    pub count: usize,
}

/// A batch to add to the counts so far.
#[derive(Clone, Debug, Deserialize)]
pub struct SampleRequest {
    pub inputs: Vec<Input>,
    /// Samples to draw in this call, unless the limit stops it first.
    pub batch: usize,
    /// This batch's seed: each batch a seed of its own, so the batches are independent.
    pub seed: u64,
    /// The hard limit on this call's sampling, ms.
    pub limit_ms: u64,
    /// The counts so far, from earlier batches.
    pub counts: Vec<Count>,
}

/// A pattern with its exact probability and the share of the samples on it.
#[derive(Debug, Serialize)]
pub struct Compared {
    pub(crate) occupations: Vec<usize>,
    pub(crate) exact: f64,
    pub(crate) sampled: f64,
}

/// The counts after a batch, against the exact distribution.
#[derive(Debug, Serialize)]
pub struct SampleView {
    pub(crate) ports: Vec<String>,
    /// Every pattern sampled so far, with its count.
    pub(crate) counts: Vec<Count>,
    pub(crate) samples: usize,
    /// The samples this call drew, and whether the limit stopped it before its batch.
    pub(crate) drawn: usize,
    pub(crate) stopped: bool,
    /// ½ Σ |p̂ − p| over the patterns at the ports.
    pub(crate) distance: f64,
    /// What a correct sampler's distance is expected to be at this many samples:
    /// ½ Σ √(2p(1 − p)/(πN)), each count binomial.
    pub(crate) expected: f64,
    /// The patterns most likely exactly, at most [`SHOWN_PATTERNS`], with their sampled shares.
    pub(crate) patterns: Vec<Compared>,
    /// Patterns sampled that the exact distribution gives no weight: a correct sampler has none.
    pub(crate) impossible: usize,
    pub(crate) seconds: f64,
}

/// A batch of samples of photons on `chip`'s ports at `wavelength_um`, added to the counts so far.
#[tauri::command(async)]
pub fn chip_sample(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    wavelength_um: f64,
    request: SampleRequest,
) -> Result<SampleView, String> {
    sampled(&chip, &state.workspace(), wavelength_um, &request)
}

pub(crate) fn sampled(
    chip: &Chip,
    root: &Path,
    wavelength_um: f64,
    request: &SampleRequest,
) -> Result<SampleView, String> {
    let n: usize = request.inputs.iter().map(|i| i.photons).sum();
    if n == 0 {
        return Err("put a photon on a port first".into());
    }
    if n > MOST_PHOTONS {
        return Err(format!(
            "{n} photons: sampling against the exact distribution takes up to {MOST_PHOTONS}"
        ));
    }
    if request.batch == 0 || request.batch > MOST_BATCH {
        return Err(format!(
            "a batch draws 1 to {MOST_BATCH} samples, not {}",
            request.batch
        ));
    }
    if request.limit_ms == 0 || request.limit_ms > MOST_LIMIT_MS {
        return Err(format!(
            "a batch's hard limit is 1 to {MOST_LIMIT_MS} ms, not {} ms",
            request.limit_ms
        ));
    }
    let wavelength = in_sweep(chip, wavelength_um)?;
    let circuit = complete(chip, root)?.compile().map_err(|e| e.to_string())?;
    let ports = port_names(&circuit);
    if ports.len() > MOST_PORTS {
        return Err(format!(
            "{} external ports: the view's sampling takes up to {MOST_PORTS}",
            ports.len()
        ));
    }
    let started = Instant::now();
    let limit = Duration::from_millis(request.limit_ms);
    let transfer = Transfer::of(&circuit, wavelength).map_err(|e| e.to_string())?;
    let modes = ports.len() + transfer.loss_modes();
    let computed = patterns_of(modes, n);
    if computed > MOST_PATTERNS {
        return Err(format!(
            "{n} photons in {} ports and {} loss modes make {computed} patterns; the exact \
             distribution is computed up to {MOST_PATTERNS}: take fewer photons or ports",
            ports.len(),
            transfer.loss_modes()
        ));
    }
    let given: Vec<(&str, usize)> = request
        .inputs
        .iter()
        .filter(|i| i.photons > 0)
        .map(|i| (i.port.as_str(), i.photons))
        .collect();
    let input = transfer.input(&given).map_err(|e| e.to_string())?;
    let exact = transfer
        .distribution(&input, &Photons::Indistinguishable)
        .map_err(|e| e.to_string())?;

    // the counts so far, checked, then this batch in pieces until the batch or the limit
    let mut counts: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
    for c in &request.counts {
        if c.occupations.len() != ports.len() || c.occupations.iter().sum::<usize>() > n {
            return Err(format!(
                "a count's pattern {:?} isn't one of {n} photons at {} ports: start the \
                 sampling again",
                c.occupations,
                ports.len()
            ));
        }
        *counts.entry(c.occupations.clone()).or_default() += c.count;
    }
    let before: usize = counts.values().sum();
    if before + request.batch > MOST_SAMPLES {
        return Err(format!(
            "{} samples would pass the {MOST_SAMPLES} the view counts",
            before + request.batch
        ));
    }
    let piece = (request.batch / 20).max(1);
    let mut drawn = 0;
    let mut part = 0u64;
    while drawn < request.batch && started.elapsed() < limit {
        let take = piece.min(request.batch - drawn);
        // each piece its own stream: the batch's seed and the piece's number
        let seed = request.seed.wrapping_mul(1_000_003).wrapping_add(part);
        for f in transfer
            .sample(&input, take, seed)
            .map_err(|e| e.to_string())?
        {
            *counts.entry(f.occupations().to_vec()).or_default() += 1;
        }
        drawn += take;
        part += 1;
    }
    let samples = before + drawn;

    // against the exact distribution
    let counted: Vec<(Fock, usize)> = counts
        .iter()
        .map(|(o, &c)| (Fock::new(o.clone()), c))
        .collect();
    let distance = exact.total_variation(&Distribution::from_counts(&counted));
    let weight: BTreeMap<&[usize], f64> = exact
        .patterns()
        .iter()
        .zip(exact.probabilities())
        .map(|(f, &p)| (f.occupations(), p))
        .collect();
    let expected = 0.5
        * exact
            .probabilities()
            .iter()
            .map(|&p| (2.0 * p * (1.0 - p) / (PI * samples as f64)).sqrt())
            .sum::<f64>();
    let impossible = counts
        .keys()
        .filter(|o| weight.get(o.as_slice()).copied().unwrap_or(0.0) == 0.0)
        .count();
    let mut patterns: Vec<Compared> = exact
        .patterns()
        .iter()
        .zip(exact.probabilities())
        .filter(|(_, p)| **p > 0.0)
        .map(|(f, &p)| Compared {
            occupations: f.occupations().to_vec(),
            exact: p,
            sampled: counts.get(f.occupations()).copied().unwrap_or(0) as f64 / samples as f64,
        })
        .collect();
    patterns.sort_by(|a, b| b.exact.total_cmp(&a.exact));
    patterns.truncate(SHOWN_PATTERNS);
    Ok(SampleView {
        ports,
        counts: counts
            .into_iter()
            .map(|(occupations, count)| Count { occupations, count })
            .collect(),
        samples,
        drawn,
        stopped: drawn < request.batch,
        distance,
        expected,
        patterns,
        impossible,
        seconds: started.elapsed().as_secs_f64(),
    })
}
