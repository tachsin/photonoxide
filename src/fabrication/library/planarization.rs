//! Planarization: chemical-mechanical polishing of oxide.

use super::{Process, StepType, cap, fact, new_process, src};
use crate::fabrication::stated::{Quantity, Stated, Unit};

pub(super) fn processes() -> Vec<Process> {
    let mut cmp = new_process(
        "cmp-oxide",
        StepType::Planarization,
        "Chemical-mechanical polishing (CMP) of oxide",
        "The wafer is pressed face down onto a turning pad with a slurry of fine abrasive: the \
         high points are polished away first, and the surface is left flat.",
        &["SiO2"],
    );
    cmp.capabilities = vec![cap(
        Quantity::Uniformity,
        Stated::range(
            20.0,
            30.0,
            Unit::Nm,
            src("krishnan-2009", "PDF p. 6 (p. 183)"),
        ),
        "the nonplanarity left after polishing shallow-trench-isolation oxide, for most designs \
         (stated as 200–300 Å)",
    )];
    cmp.facts = vec![fact(
        "A stop layer's high selectivity on a blanket wafer is not realized on a patterned one \
         where the stop layer covers only a small fraction of the area.",
        "krishnan-2009",
        "PDF p. 6 (p. 183)",
    )];
    vec![cmp]
}
