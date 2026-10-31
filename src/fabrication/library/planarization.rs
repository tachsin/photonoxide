//! Planarization: chemical-mechanical polishing of oxide, and of nitride in the photonic
//! Damascene process.

use super::{Platform, Process, Purpose, StepType, cap, fact, new_process, src, used};
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
    cmp.capabilities = vec![
        cap(
            Quantity::Uniformity,
            Stated::range(
                20.0,
                30.0,
                Unit::Nm,
                src("krishnan-2009", "PDF p. 6 (p. 183)"),
            ),
            "the nonplanarity left after polishing shallow-trench-isolation oxide, for most designs \
         (stated as 200–300 Å)",
        ),
        cap(
            Quantity::Uniformity,
            Stated::value(
                10.0,
                Unit::Percent,
                src("franssila-2010", "Table 16.1 (p. 182)"),
            ),
            "the polish's uniformity across the wafer, of the thickness it removes",
        ),
    ];
    cmp.facts = vec![fact(
        "A stop layer's high selectivity on a blanket wafer is not realized on a patterned one \
         where the stop layer covers only a small fraction of the area.",
        "krishnan-2009",
        "PDF p. 6 (p. 183)",
    )];
    let mut nitride = new_process(
        "cmp-nitride",
        StepType::Planarization,
        "Chemical-mechanical polishing (CMP) of nitride",
        "The nitride deposited over the oxide preform is polished away down to the oxide, \
         leaving nitride only in the preform's trenches: the guides, their top surface flat.",
        &["Si3N4", "SiN"],
    );
    nitride.capabilities = vec![cap(
        Quantity::Uniformity,
        Stated::not_specified(),
        "the nitride left over the oxide, and the guides' height, after polishing",
    )];
    nitride.facts = vec![
        fact(
            "The excess nitride is removed by chemical-mechanical polishing, leaving a polished \
             surface that exposes only the guides' tops.",
            "pfeiffer-2016",
            "PDF pp. 2–3",
        ),
        fact(
            "The dense pattern around the guides prevents cracks during the polish too; nitride \
             the polish leaves behind can still crack later.",
            "pfeiffer-2016",
            "PDF p. 3",
        ),
    ];
    nitride.used_for = vec![used(
        Platform::Nitride,
        Purpose::Guide,
        "pfeiffer-2016",
        "PDF pp. 2–3",
    )];
    vec![cmp, nitride]
}
