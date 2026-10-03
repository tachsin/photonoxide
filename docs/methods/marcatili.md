---
title: "Marcatili's approximation"
module: mode::marcatili
summary: "A rectangular guide's modes from two slab-like equations, ignoring the corners: exact far from cutoff, measured against the full-vector solver."
order: 13
papers:
  - cite: "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969)"
    doi: 10.1002/j.1538-7305.1969.tb01166.x
validation:
  - mode/marcatili-closed-form
  - mode/marcatili-vector
examples:
  - marcatili
---

A rectangular core of index $n_1$, width a and height b, with claddings $n_2$ above, $n_3$ left,
$n_4$ below and $n_5$ right. Marcatili ignores the four corner regions, so the field separates and

$$
k_z^2 = k_1^2 - k_x^2 - k_y^2 ,
$$

with $k_x$ and $k_y$ from two slab-like equations. For the $E^y_{pq}$ modes (E mostly along y),

$$
k_x a = p\pi - \tan^{-1}(k_x \xi_3) - \tan^{-1}(k_x \xi_5), \qquad
k_y b = q\pi - \tan^{-1}\negthinspace\left(\frac{n_2^2}{n_1^2} k_y \eta_2\right) - \tan^{-1}\negthinspace\left(\frac{n_4^2}{n_1^2} k_y \eta_4\right)
$$

(his Eqs. 6–7), with $\xi_j = [(k_1^2 - k_j^2) - k_x^2]^{-1/2}$ and $\eta_j$ likewise. For the $E^x_{pq}$ modes
the $n^2$ ratios move to the x equation (Eqs. 20–21). `Rectangle::mode` solves these exactly (his
solid curves); `Rectangle::closed_form` uses his closed-form approximations, Eqs. 12–13 and 22–23
(his dashed curves).

## Validation

His Fig. 6b guide, a = 2b, $n_1 = 1.5$ in $n_1/1.05$, at 1 µm, in his normalized constant
$(k_z^2 - k_4^2)/(k_1^2 - k_4^2)$ against $B = 2b/\lambda\thinspace(n_1^2 - n_4^2)^{1/2}$:

| B | full-vector $E^x_{11}$ | Marcatili | closed form |
|---|---|---|---|
| 1 | 0.5071 | 0.4983 | 0.4761 |
| 1.5 | 0.7095 | 0.7083 | 0.7028 |
| 2 | 0.8104 | 0.8103 | 0.8084 |
| 3 | 0.9015 | 0.9016 | 0.9013 |
| 4 | 0.9399 | 0.9400 | 0.9399 |

- **Far from cutoff** Marcatili and the full-vector solver agree to 1e-4, as he found against
  Goell's computer solutions ("the three solutions coincide even for moderately large values of
  b"); near cutoff (B = 1) they differ by 9e-3, the field reaching into the corners he ignores.
- **The closed form** is within 4.1 % of his exact solution wherever the constant is at least 0.5,
  as he states ("within a few percent", p. 2083).
- A very wide rectangle is the slab: $E^y$ its TM mode and $E^x$ its TE mode, to 1e-6.
