"""The platform canaries' golden values (tests/canary.rs, src/canary_tests.rs): exact results,
correctly rounded to binary64, from mpmath at 256 bits.

How it was run (once, on 2026-11-13; never in CI, which only reads golden.txt):

    uv run --no-project --with mpmath==1.3.0 python tests/data/canary/golden.py > tests/data/canary/golden.txt

The inputs are binary64 numbers chosen here (fixed lists and a fixed 64-bit LCG), written as their
bits, so the tests read exactly the arguments mpmath was given. Each result is the exact value of
the function at that argument, rounded to the nearest binary64 (ties to even): no result here
comes from photonoxide or from any system's maths library. Rerunning it writes the same file.

Each line: `name a b r i  # decimals`, with a, b the arguments' bits (b for a second argument or an
imaginary part, `-` when there is none) and r, i the result's (i for an imaginary part).
"""

import math
import struct
import sys

import mpmath as mp

mp.mp.prec = 256


def bits(x):
    return "%016x" % struct.unpack("<Q", struct.pack("<d", x))[0]


def rnd(v):
    """v rounded to the nearest binary64 (ties to even); v must be in the normal range."""
    v = mp.mpf(v)
    with mp.workprec(53):
        r = +v
    f = float(r)
    assert f == 0.0 or 2.0**-1022 <= abs(f) < math.inf, v
    return f


class Lcg:
    """Knuth's MMIX LCG: the same sequence on any machine."""

    def __init__(self, seed):
        self.s = seed

    def unit(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % 2**64
        return (self.s >> 11) / 2.0**53

    def uniform(self, a, b):
        return a + (b - a) * self.unit()

    def log_uniform(self, a, b):
        return math.exp(math.log(a) + (math.log(b) - math.log(a)) * self.unit())


def sample(fixed, count, draw):
    xs = list(fixed)
    while len(xs) < count:
        xs.append(draw())
    return xs


lines = []


def real(name, xs, f):
    for x in xs:
        r = rnd(f(mp.mpf(x)))
        lines.append(f"{name} {bits(x)} - {bits(r)} -  # {x!r} -> {r!r}")


def real2(name, pairs, f):
    for x, y in pairs:
        r = rnd(f(mp.mpf(x), mp.mpf(y)))
        lines.append(f"{name} {bits(x)} {bits(y)} {bits(r)} -  # {x!r}, {y!r} -> {r!r}")


def complex_(name, zs, f, real_out=False):
    for x, y in zs:
        w = f(mp.mpc(x, y))
        if real_out:
            r = rnd(w)
            lines.append(f"{name} {bits(x)} {bits(y)} {bits(r)} -  # {x!r}{y:+}i -> {r!r}")
        else:
            r, i = rnd(mp.re(w)), rnd(mp.im(w))
            lines.append(
                f"{name} {bits(x)} {bits(y)} {bits(r)} {bits(i)}  # {x!r}{y:+}i -> {r!r}{i:+}i"
            )


N = 50
g = Lcg(0x5EED_CA9A_2026_1113)
PI = math.pi

# --- the system's maths library, through std's f64 and num-complex ---

real("exp", sample([0.5, 1.0, -1.0, 1e-10, -1e-10, 0.6931471805599453, 10.0, 100.0, 700.0, -700.0,
                    709.0, -708.0, 1e-300], N, lambda: g.uniform(-700.0, 700.0)), mp.exp)
real("exp_m1", sample([1e-10, -1e-10, 1e-5, -1e-5, 0.5, -0.5, 1.0, 10.0, -30.0, 1e-300], N,
                      lambda: g.uniform(-40.0, 40.0)), mp.expm1)
real("ln", sample([2.0, 10.0, 0.5, math.e, 1.0 + 2.0**-30, 1.0 - 2.0**-40, 1e-300, 1e300, 3.0,
                   0.1], N, lambda: g.log_uniform(1e-300, 1e300)), mp.log)
real("ln_1p", sample([1e-10, -1e-10, 1e-5, 0.5, -0.5, -0.9999, 1.0, 1e10, 2.0**-30], N,
                     lambda: g.uniform(-0.99, 100.0)), mp.log1p)
real("log10", sample([2.0, 10.0, 1000.0, 0.001, 1e-300, 1e300, 3.0, 1.0 + 2.0**-30], N,
                     lambda: g.log_uniform(1e-300, 1e300)), mp.log10)
trig_fixed = [0.5, 1.0, PI, -PI, 2 * PI, 3 * PI, PI / 2, -PI / 2, PI / 4, 355.0, 104348.0, 1e6,
              1e22, 1e100, 1e300, 2.0**1000, 1e-8, 22.0, 710.0]
real("sin", sample(trig_fixed, N, lambda: g.uniform(-1e6, 1e6)), mp.sin)
real("cos", sample(trig_fixed, N, lambda: g.uniform(-1e6, 1e6)), mp.cos)
real("tan", sample(trig_fixed, N, lambda: g.uniform(-1e6, 1e6)), mp.tan)
real("asin", sample([0.5, -0.5, 1.0, -1.0, 1.0 - 2.0**-30, 1e-10, 0.9, -0.99], N,
                    lambda: g.uniform(-1.0, 1.0)), mp.asin)
real("acos", sample([0.5, -0.5, 1.0, -1.0, 1.0 - 2.0**-30, 1e-10, 0.9, -0.99], N,
                    lambda: g.uniform(-1.0, 1.0)), mp.acos)
real("atan", sample([0.5, -0.5, 1.0, -1.0, 1e-10, 1e10, -1e300, 2.0], N,
                    lambda: g.uniform(-50.0, 50.0)), mp.atan)
real2("atan2", sample([(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0), (0.0, -1.0),
                       (1e-300, 1.0), (1.0, 1e-300), (3.0, -4.0), (1e300, 1e-300)], N,
                      lambda: (g.uniform(-10.0, 10.0), g.uniform(-10.0, 10.0))), mp.atan2)
real2("hypot", sample([(3.0, 4.0), (1e300, 1e300), (1e-300, 1e-300), (1.0, 1e-10), (1e10, 1.0),
                       (-5.0, 12.0)], N,
                      lambda: (g.uniform(-100.0, 100.0), g.uniform(-100.0, 100.0))), mp.hypot)
real("sinh", sample([1e-10, -1e-10, 0.5, 1.0, -1.0, 10.0, 700.0, -700.0, 0.01], N,
                    lambda: g.uniform(-30.0, 30.0)), mp.sinh)
real("cosh", sample([1e-10, 0.5, 1.0, -1.0, 10.0, 700.0, -700.0, 0.01], N,
                    lambda: g.uniform(-30.0, 30.0)), mp.cosh)
real("tanh", sample([1e-10, -1e-10, 0.5, 1.0, -1.0, 10.0, 20.0, 0.01], N,
                    lambda: g.uniform(-20.0, 20.0)), mp.tanh)
real2("powf", sample([(2.0, 0.5), (10.0, -3.0), (1.5, 100.0), (0.5, 0.5), (1e-5, 2.5),
                      (-2.0, 3.0), (-1.5, -2.0), (2.0, -1000.0), (1.0 + 2.0**-30, 1e9)], N,
                     lambda: (g.uniform(0.01, 100.0), g.uniform(-50.0, 50.0))), mp.power)
real2("powi", sample([(1.1, 2.0), (0.7, 3.0), (1.3, 12.0), (0.9, -5.0), (-1.7, 7.0), (2.5, -12.0)],
                     N, lambda: (g.uniform(0.5, 2.0), float(int(g.uniform(-12.0, 13.0))))),
      mp.power)
real("cbrt", sample([27.0, -8.0, 2.0, 1e-300, 1e300, -1e300, 0.001, 3.0], N,
                    lambda: g.uniform(-1e3, 1e3)),
     lambda x: mp.sign(x) * mp.cbrt(abs(x)))
complex_("c_norm", sample([(3.0, 4.0), (1e300, 1e300), (1e-300, 1e-300), (-1.0, 1e-10)], N,
                          lambda: (g.uniform(-50.0, 50.0), g.uniform(-50.0, 50.0))),
         mp.fabs, real_out=True)
complex_("c_sqrt", sample([(-4.0, 1e-10), (-4.0, -1e-10), (3.0, 4.0), (0.0, 2.0), (1e-10, 1.0),
                           (-1.0, 1.0)], N,
                          lambda: (g.uniform(-50.0, 50.0), g.uniform(-50.0, 50.0))), mp.sqrt)
complex_("c_exp", sample([(0.0, PI), (1.0, PI / 2), (-1.0, 1e6), (10.0, -3.0)], N,
                         lambda: (g.uniform(-50.0, 50.0), g.uniform(-100.0, 100.0))), mp.exp)
complex_("c_ln", sample([(-1.0, 1e-10), (0.0, 1.0), (1.0, 1e-8), (1e300, 1e300), (3.0, 4.0)], N,
                        lambda: (g.uniform(-50.0, 50.0), g.uniform(-50.0, 50.0))), mp.log)

# --- photonoxide's own special functions, each in the range its doc comment gives ---

real("erf", sample([0.1, 0.5, 1.0, 2.0, 2.9, 2.99, 3.0, 3.01, 4.0, 5.0, 6.4, 6.6, -0.5, -3.5, 1e-8],
                   40, lambda: g.uniform(-6.5, 6.5)), mp.erf)
bessel_x = sample([0.05, 0.5, 1.0, 2.404825557695773, 3.8317059702075125, 5.0, 10.0, 15.0], 25,
                  lambda: g.uniform(0.05, 15.0))
for m in (0, 1):
    real(f"bessel_j{m}", bessel_x, lambda x, m=m: mp.besselj(m, x))
    real(f"bessel_y{m}", bessel_x, lambda x, m=m: mp.bessely(m, x))
bessel_z = sample([(2.0, 1.0), (0.5, -0.5), (10.0, 3.0), (1.0, 5.0)], 8,
                  lambda: (g.uniform(0.1, 10.0), g.uniform(-5.0, 5.0)))
for m in (0, 1):
    complex_(f"bessel_j{m}c", bessel_z, lambda z, m=m: mp.besselj(m, z))
    complex_(f"bessel_y{m}c", bessel_z, lambda z, m=m: mp.bessely(m, z))
    complex_(f"hankel1_{m}", bessel_z, lambda z, m=m: mp.hankel1(m, z))
real("e1", sample([0.01, 0.5, 0.999, 1.0, 1.001, 2.0, 10.0, 50.0, 1e-5], 40,
                  lambda: g.log_uniform(1e-3, 50.0)), mp.e1)
real("gamma", sample([0.5, 1.0, 1.25, 1.5, 2.0, 3.5, 10.0], 40, lambda: g.uniform(0.5, 10.0)),
     mp.gamma)

# --- the FFT: the exact DFT of a fixed signal of 64 exactly representable values ---

n = 64
x = [mp.mpc(((j * j) % 13) / 8 - 0.75, (j % 9) / 4 - 1.0) for j in range(n)]
for k in range(n):
    s = mp.fsum(x[j] * mp.expjpi(mp.mpf(-2 * j * k) / n) for j in range(n))
    r, i = rnd(mp.re(s)), rnd(mp.im(s))
    lines.append(f"fft64 {bits(float(k))} - {bits(r)} {bits(i)}  # X_{k} -> {r!r}{i:+}i")

# --- the slab: 220 nm of silicon (3.473) in oxide (1.444) at 1.55 um, the doubles
# photonoxide takes (t = 220/1000, k = TAU/1.55), its TE0 and TM0 indices exact ---

n1 = n3 = 1.444
n2 = 3.473
t = 220.0 / 1000.0
k0 = math.tau / 1.55


def condition(neff, tm):
    neff = mp.mpf(neff)
    h = k0 * mp.sqrt(n2**2 - neff**2)
    q = k0 * mp.sqrt(neff**2 - mp.mpf(n1) ** 2)
    p = k0 * mp.sqrt(neff**2 - mp.mpf(n3) ** 2)
    if tm:
        q, p = (mp.mpf(n2) / n1) ** 2 * q, (mp.mpf(n2) / n3) ** 2 * p
    return h * t - mp.atan2(q, h) - mp.atan2(p, h)


for name, tm in (("slab_te0", False), ("slab_tm0", True)):
    root = mp.findroot(lambda v: condition(v, tm), (mp.mpf(n1) + 1e-9, mp.mpf(n2) - 1e-9),
                       solver="anderson", tol=mp.mpf(2) ** -240)
    r = rnd(root)
    lines.append(f"{name} {bits(t)} {bits(k0)} {bits(r)} -  # t, k -> {r!r}")

# --- dense: the eigenvalues (its singular values too) of the 8 x 8 Hilbert matrix whose entries
# are the binary64 numbers 1/(i + j + 1), ascending ---

hilbert = mp.matrix(8, 8)
for i in range(8):
    for j in range(8):
        hilbert[i, j] = mp.mpf(1.0 / (i + j + 1))
for k, e in enumerate(sorted(mp.eigsy(hilbert, eigvals_only=True))):
    r = rnd(e)
    lines.append(f"hilbert8 {bits(float(k))} - {bits(r)} -  # lambda_{k} -> {r!r}")

sys.stdout.reconfigure(newline="\n")  # LF on every system
print(__doc__.strip().replace("\n", "\n# ").join(["# ", ""]))
print(f"# mpmath {mp.__version__}, {mp.mp.prec} bits")
print("\n".join(lines))
