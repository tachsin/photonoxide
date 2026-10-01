# Validation report

Written by `photonoxide validate`; don't edit it by hand. CI fails when a case fails or this file is out of date. Values have six significant digits; a value that should be zero shows as "≤ tolerance" when it is within it.

| Case | Tier | What | Against | Measured | Expected | Tolerance | Result |
|---|---|---|---|---|---|---|---|
| `units/amplitude-convention` | analytic | The amplitude of a real signal Re(A e^(-iwt)) is recovered with the kernel e^(+iwt) (magnitude shown) | the e^(-iwt) convention: (2/T) int Re(A e^(-iwt)) e^(iwt) dt = A over whole periods | 0.806226 | 0.806226 | 1e-9 | pass |
| `units/lossy-attenuation` | analytic | A wave in a medium with Im(eps) > 0 decays over one wavelength by exp(-2 pi kappa) (decay shown) | e^(i n k0 x) with n = n' + i kappa, kappa >= 0 | 0.635495 | 0.635495 | 1e-12 | pass |
| `material/spline-line` | analytic | A natural cubic spline through points on a line is that line (largest deviation shown) | a line has zero second derivative, which the natural end conditions impose | ≤ 1e-13 | 0 | 1e-13 | pass |
| `material/silicon-li-table` | published | Silicon's index passes through Li's table at all 35 points (largest deviation shown) | H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980), doi:10.1063/1.555624, 293 K | ≤ 1e-12 | 0 | 1e-12 | pass |
