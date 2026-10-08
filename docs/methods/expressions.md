---
title: "Expressions and parameters"
module: expr
summary: "A small, safe expression language of photonoxide's own for the numbers of a structure: numbers with units, arithmetic, comparisons and the functions shapes need, named parameters ordered by what they use, units checked with errors at the column, compiled once to a flat program evaluated without allocating, and derivatives with respect to any parameter by forward mode."
order: 1.6
papers:
  - cite: "R. E. Wengert, Commun. ACM 7, 463 (1964) (derivatives by forward mode)"
    doi: 10.1145/355586.364791
  - cite: "F. W. J. Olver et al., NIST Digital Library of Mathematical Functions, sections 7.6 and 7.9 (the error function)"
    doi: ""
validation: []
examples: []
---

Any number of a shape can be an expression of named parameters: a ring's radius, a gap that is
a fraction of a waveguide's width, a grating's period. A job stays readable text, and any
parameter can be swept, and later optimized. The language is photonoxide's own and evaluates
arithmetic only: it knows its functions, the coordinates and the parameters it is given, and
nothing else.

## The language

```text
500 nm      w/2 + gap      2*pi*x/period      if(i < 3, 0.2 um, r0)      90 deg
sqrt(w^2 + h^2)      r0*(1 + 0.1*cos(4*t))      piecewise(x < 0, 0, x < w, x/w, 1)
```

- **Numbers with units:** `nm`, `um` (or `µm`) and `mm` for lengths, kept in µm; `deg` and `rad`
  for angles, kept in radians. A unit follows its number (`500 nm`, `1e3nm`) and a power binds to
  the unit alone (`4 um^2` is 4 µm²). Alone, a unit is itself: `w/nm` is w in nanometres.
  Nanometres convert by dividing by 1000, as `Length::nm` does, so `"220 nm"` is the bits of
  `Length::nm(220.0)`.
- **Operators:** `+ - * / ^` with the usual precedence (`^` to the right and above a leading
  minus: $-2^2 = -4$, $2^{3^2} = 512$), comparisons `< <= > >= == !=` giving 1 or 0, parentheses.
- **Functions:** `sin cos tan asin acos atan atan2 exp ln log10 sqrt abs min max clamp floor ceil
  round erf tanh sinc smoothstep`, `if(c, a, b)` and `piecewise(c₁, a₁, c₂, a₂, …, otherwise)`;
  `pi`. `sinc(x)` is $\sin x / x$; `smoothstep(e0, e1, x)` is $3u^2 - 2u^3$ for
  $u = (x - e_0)/(e_1 - e_0)$ clamped to [0, 1]. The error function is the series of positive
  terms $\operatorname{erf} x = \frac{2}{\sqrt\pi} e^{-x^2} \sum_n 2^n x^{2n+1}/(2n+1)!!$ (DLMF
  7.6.2) for $\lvert x\rvert < 3$, which has no cancellation, and erfc's continued fraction (DLMF
  7.9.2) beyond: within 5e-16 of the tables.
- **Coordinates:** `x`, `y`, `z` and the arc length `s` (lengths), a curve's parameter `t` and an
  array's indices `i`, `j` (numbers), for the definitions that supply them; a shape's own fields
  in a job have none and refuse them.

**Units are checked when an expression is parsed.** Each has a dimension, a power of length and
of angle. Sums, comparisons, `min`, `max`, `clamp` and the two values of an `if` need the same
power of length; products add powers, quotients subtract them. Radians are numbers, so an angle
and a number add, but a length and an angle don't: `1 um + 30 deg` is refused at column 6,
"can't add an angle to a length". `sin`, `cos`, `tan` and `sinc` take an angle or a number;
`asin`, `acos`, `atan` and `atan2` give an angle; `exp`, `ln`, `log10`, `erf` and `tanh` take
numbers; `sqrt` halves powers (refusing an odd one); a dimensioned base needs a constant power
that keeps its powers whole. A bare `0` fits any dimension. Every error names its column, from
1: an unknown name, a misplaced operator, a function given the wrong number of arguments, a
division by a constant zero, a stack deeper than 64 values.

## Compiled once

An expression is parsed by recursive descent into a tree, its units checked, its constant parts
folded (`2*pi/3` becomes a number), and compiled to a flat list of instructions for a stack
machine: constants, loads of a slot, the operators and functions, and two jumps for `if`. The
deepest stack the program needs is known when it is compiled, so `Expr::eval` runs on a stack
of 64 values in place, without allocating. `Expr::try_eval` checks each instruction and says
where a value stopped being finite: a division by zero at a parameter's value, the root or
logarithm of a negative number. `Expr::eval_many` runs a second program, `if` as a select of
both sides, an instruction at a time over blocks of 32 points, which reads the program once a
block.

A deformed ring's radius at angle t, `r*(1 + 0.05*cos(4*t)) + w/2*sin(3*t)^2` (14
instructions), on one thread of an Intel Core Ultra 7 265K: 17.6 million evaluations a second one
at a time, 47.6 million in blocks, 14.5 million with a derivative
(`cargo test --release --lib expression_timing -- --ignored --nocapture`).

## Parameters

`Parameters` is a table of named parameters, each an expression of the others. They are
evaluated in an order where each comes after those it uses, ties in the order given; a
parameter that uses itself, or a cycle, is refused, naming it (`a -> b -> c -> a`). A name is a
letter or `_` then letters, digits and `_`, not one of the language's own. `Parameters::with`
sets one to a value, a sweep's point, and evaluates again those that use it.

In a job, `[task.parameters]` holds them, and every shape table's `center`, `size`, `radius` and
`width` takes expressions with units (its `_um` keys take numbers as before, or expressions read
in µm); a modes job's sweep can step through any parameter. See `photonoxide::job`.

## Derivatives

`Expr::eval_dual` carries each value's derivative along one direction of the parameters, dual
numbers $a + b\varepsilon$ with $\varepsilon^2 = 0$ (Wengert 1964): every instruction applies its
derivative by the chain rule, `if` follows the branch taken, and `floor`, `ceil`, `round` and the
comparisons have derivative zero. `Parameters::derivatives(name)` seeds `name` with 1 and carries
the derivatives through the table in its order, so an expression of the parameters gets its
derivative with respect to any one of them: the hook for shape optimization.

## Checks

Every function against its value (erf against tabulated values to 5e-16); numbers, units and
precedence; every error at its column; parameter tables in any order, set for a sweep, refused
for cycles, bad names and units; every function's derivative against central differences, and
through a table ($\partial y_0/\partial w = 1.4$ for $y_0 = r + w + w/2.5$); blocks of points
equal to the points one by one, to the bit. In jobs: a strip whose width and length are
parameters records exactly the events of the strip given in numbers, a sweep of that parameter
the effective indices of the width sweep it stands for, to the bit, and the check names the
table and field of each error.
