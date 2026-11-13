"""photonoxide::portable's reference values (src/portable/tests.rs; golden.py beside it has those of
the system's functions): exact results, correctly rounded to binary64, from mpmath at 4096 bits
(at 256, mpmath's complex arcsine loses an imaginary part of 1e-200 next to a real part of 0.3).

How it was run (once, on 2026-11-13; never in CI, which only reads portable.txt):

    uv run --no-project --with mpmath==1.3.0 python tests/data/canary/portable.py > tests/data/canary/portable.txt

The inputs are binary64 numbers chosen here (fixed lists, among them the inputs with the largest
known errors of OpenLibm and musl, Gladman et al. 2026, Table 7, and a fixed 64-bit LCG), written as
their bits, so the tests read exactly the arguments mpmath was given. Each result is the exact value
of the function at that argument, rounded to the nearest binary64 (ties to even): no result here
comes from photonoxide or from any system's maths library. Rerunning it writes the same file.

Each line: `name a b c d r i  # decimals`: a, b the first argument (b its imaginary part, for a
complex one), c, d the second; r, i the result. `-` where there is none.
"""

import math
import struct
import sys

import mpmath as mp

mp.mp.prec = 4096


def bits(x):
    return "%016x" % struct.unpack("<Q", struct.pack("<d", x))[0]


def h(s):
    """A binary64 from C's hexadecimal notation."""
    return float.fromhex(s)


def rnd(v):
    """v rounded to the nearest binary64 (ties to even); v must be in the normal range or 0."""
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
        # by its exponent, so no system's log or exp is involved: 2^e with e uniform, times a
        # mantissa drawn from [1, 2)
        e = math.floor(self.uniform(math.frexp(a)[1], math.frexp(b)[1]))
        return math.ldexp(1.0 + self.unit(), e - 1)


def sample(fixed, count, draw):
    xs = list(fixed)
    while len(xs) < count:
        xs.append(draw())
    return xs


lines = []


def line(name, a, b, c, d, r, i, note):
    f = lambda v: "-" if v is None else bits(v)
    lines.append(f"{name} {f(a)} {f(b)} {f(c)} {f(d)} {f(r)} {f(i)}  # {note}")


def real(name, xs, f):
    for x in xs:
        r = rnd(f(mp.mpf(x)))
        line(name, x, None, None, None, r, None, f"{x!r} -> {r!r}")


def real2(name, pairs, f):
    for x, y in pairs:
        r = rnd(f(mp.mpf(x), mp.mpf(y)))
        line(name, x, None, y, None, r, None, f"{x!r}, {y!r} -> {r!r}")


def cplx(name, zs, f, real_out=False):
    for x, y in zs:
        w = f(mp.mpc(x, y))
        if real_out:
            r = rnd(w)
            line(name, x, y, None, None, r, None, f"{x!r}{y:+}i -> {r!r}")
        else:
            r, i = rnd(mp.re(w)), rnd(mp.im(w))
            line(name, x, y, None, None, r, i, f"{x!r}{y:+}i -> {r!r}{i:+}i")


N = 48
g = Lcg(0x0822_9047_AB1E_2026)
PI = math.pi

# --- the real functions: each one's fixed arguments (edges, then Table 7's worst for OpenLibm and
# musl), then uniform and log-uniform ones over its range ---

real("exp", sample([0.5, 1.0, -1.0, 1e-10, -1e-10, 2.0**-29, 0.6931471805599453, 10.0, 100.0,
                    700.0, -700.0, 709.7, -708.0, 1e-300, h("0x1.2e8f20cf3cbe7p+8"),
                    h("-0x1.18209ecd19a8cp+6")], N, lambda: g.uniform(-708.0, 709.0)), mp.exp)
real("exp2", sample([0.5, 1.0, -1.0, 1e-10, 10.5, 1023.5, -1021.0,
                     h("-0x1.1a4ce073ea908p-5")], N, lambda: g.uniform(-1020.0, 1023.0)),
     lambda x: mp.power(2, x))
real("exp_m1", sample([1e-10, -1e-10, 1e-5, -1e-5, 0.5, -0.5, 1.0, 10.0, -30.0, 1e-300, 700.0,
                       -0.6858221764202648, h("0x1.62ff47a01658fp-2")], N,
                      lambda: g.uniform(-40.0, 40.0)), mp.expm1)
real("ln", sample([2.0, 10.0, 0.5, math.e, 1.0 + 2.0**-30, 1.0 - 2.0**-40, 1e-300, 1e300, 3.0, 0.1,
                   2.2250738585072014e-308, 1.7976931348623157e308, h("0x1.48ae5a67204f5p+0"),
                   h("0x1.dc0b586f2b26p-1")], N, lambda: g.log_uniform(1e-300, 1e300)), mp.log)
real("ln_1p", sample([1e-10, -1e-10, 1e-5, 0.5, -0.5, -0.9999, 1.0, 1e10, 2.0**-30, 1e300,
                      h("-0x1.2bf1de6b04a8ap-2"), h("-0x1.2bf32aaf122e2p-2")], N,
                     lambda: g.uniform(-0.99, 100.0)), mp.log1p)
real("log2", sample([2.0, 3.0, 0.5, 1.0 + 2.0**-30, 1e-300, 1e300, 1024.0, 0.1,
                     h("0x1.67eaf07ce24d1p+0"), h("0x1.0b53197bd66c8p+0")], N,
                    lambda: g.log_uniform(1e-300, 1e300)), lambda x: mp.log(x, 2))
real("log10", sample([2.0, 10.0, 1000.0, 0.001, 1e-300, 1e300, 3.0, 1.0 + 2.0**-30,
                      h("0x1.553e1cb579ee9p+0")], N, lambda: g.log_uniform(1e-300, 1e300)),
     mp.log10)
trig = [0.5, 1.0, PI, -PI, 2 * PI, 3 * PI, PI / 2, -PI / 2, PI / 4, 355.0, 104348.0, 1e6, 1e22,
        1e100, 1e300, 2.0**1000, 1e-8, 22.0, 710.0, 1e-300]
real("sin", sample(trig + [h("0x1.4d84db080b9fdp+21")], N, lambda: g.uniform(-1e6, 1e6)), mp.sin)
real("cos", sample(trig + [h("-0x1.34e729fd08086p+21")], N, lambda: g.uniform(-1e6, 1e6)), mp.cos)
real("tan", sample(trig + [h("0x1.3f9605aaeb51bp+21")], N, lambda: g.uniform(-1e6, 1e6)), mp.tan)
inverse = [0.5, -0.5, 1.0, -1.0, 1.0 - 2.0**-30, 1e-10, 0.9, -0.99, 1e-300]
real("asin", sample(inverse + [h("-0x1.004d1c5a9400bp-1")], N, lambda: g.uniform(-1.0, 1.0)),
     mp.asin)
real("acos", sample(inverse + [h("-0x1.0068b067c6feep-1")], N, lambda: g.uniform(-1.0, 1.0)),
     mp.acos)
real("atan", sample([0.5, -0.5, 1.0, -1.0, 1e-10, 1e10, -1e300, 2.0, 1e-300,
                     h("0x1.62ff6a1682c25p-1")], N, lambda: g.uniform(-50.0, 50.0)), mp.atan)
real("sinh", sample([1e-10, -1e-10, 0.5, 1.0, -1.0, 10.0, 700.0, -700.0, 0.01, 1e-300,
                     h("-0x1.63324af2fb5b7p-1")], N, lambda: g.uniform(-30.0, 30.0)), mp.sinh)
real("cosh", sample([1e-10, 0.5, 1.0, -1.0, 10.0, 700.0, -700.0, 0.01, h("-0x1.6310ab92794a8p+9"),
                     h("-0x1.502bf5ad80729p+0")], N, lambda: g.uniform(-30.0, 30.0)), mp.cosh)
real("tanh", sample([1e-10, -1e-10, 0.5, 1.0, -1.0, 10.0, 20.0, 0.01, 1e-300,
                     h("-0x1.e134557098e37p-3")], N, lambda: g.uniform(-20.0, 20.0)), mp.tanh)
real("asinh", sample([1e-10, -1e-10, 0.5, 1.0, -1.0, 1e10, -1e300, 0.125, 1e-300,
                      h("-0x1.02657ff36d5f3p-2"), h("-0x1.0240f2bdb3f25p-2")], N,
                     lambda: g.uniform(-50.0, 50.0)), mp.asinh)
real("acosh", sample([1.0, 1.0 + 2.0**-52, 1.0 + 1e-10, 1.125, 2.0, 10.0, 1e10, 1e300,
                      h("0x1.0001ff6afc4bap+0")], N, lambda: g.uniform(1.0, 50.0)), mp.acosh)
real("atanh", sample([1e-10, -1e-10, 0.5, -0.5, 0.999, 1.0 - 2.0**-30, 1e-300,
                      h("-0x1.f97fabc0650c4p-4"), h("-0x1.f8a404597baf4p-4")], N,
                     lambda: g.uniform(-1.0, 1.0)), mp.atanh)
real("cbrt", sample([27.0, -8.0, 2.0, 1e-300, 1e300, -1e300, 0.001, 3.0,
                     h("-0x1.13a5ccd87c9bbp+1008")], N, lambda: g.uniform(-1e3, 1e3)),
     lambda x: mp.sign(x) * mp.cbrt(abs(x)))
real("erf", sample([0.1, 0.5, 1.0, 2.0, 3.0, 5.0, 6.0, -0.5, -3.5, 1e-8, 1e-300, 0.84375, 1.25,
                    h("-0x1.c57541b55c8ebp-16")], N, lambda: g.uniform(-6.0, 6.0)), mp.erf)
real("erfc", sample([0.1, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0, 26.0, -0.5, -3.5, 1e-8, 0.84375, 1.25,
                     h("0x1.5182d8799b84bp+0"), h("0x1.527f4fb0d9331p+0")], N,
                    lambda: g.uniform(-6.0, 26.5)), mp.erfc)
real("erfcx", sample([-26.0, -10.0, -3.0, -1.0, -0.5, -0.001, 0.0, 1e-10, 0.25, 0.5, 1.0, 2.0,
                      3.5, 5.0, 10.0, 20.0, 25.9, 26.0, 26.1, 30.0, 50.0, 100.0, 1000.0, 1e8, 1e20,
                      1e200], N, lambda: g.uniform(-26.0, 60.0)),
     lambda x: mp.erfc(x) * mp.exp(x * x) if x < 1e4 else
     # mpmath's erfc fails above about 1e10: there, A&S 7.1.23's series, whose 12 terms leave
     # (2x^2)^-12 of the value, below 1e-90 here
     mp.fsum((-1) ** n * mp.fac2(2 * n - 1) / (2 * x * x) ** n for n in range(12)) / (x * mp.sqrt(mp.pi)))
real2("atan2", sample([(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0), (1e-300, 1.0),
                       (1.0, 1e-300), (3.0, -4.0), (1e300, 1e-300), (1e-300, -1.0),
                       (h("-0x1.358bb5eb25bdcp+813"), h("0x1.2f86b82481a0ap+815"))], N,
                      lambda: (g.uniform(-10.0, 10.0), g.uniform(-10.0, 10.0))), mp.atan2)
real2("hypot", sample([(3.0, 4.0), (1e300, 1e300), (1e-300, 1e-300), (1.0, 1e-10), (1e10, 1.0),
                       (-5.0, 12.0), (h("0x1.00014d4b1c6b9p-1015"), h("-0x1.000105ba9bf4p-1015"))],
                      N, lambda: (g.uniform(-100.0, 100.0), g.uniform(-100.0, 100.0))), mp.hypot)
real2("powf", sample([(2.0, 0.5), (10.0, -3.0), (1.5, 100.0), (0.5, 0.5), (1e-5, 2.5), (-2.0, 3.0),
                      (-1.5, -2.0), (2.0, -1000.0), (1.0 + 2.0**-30, 1e9), (0.3, 0.25), (2.0, 0.8),
                      (h("0x1.000002c5e2e99p+0"), h("0x1.c9eee35374af6p+31")),
                      (h("0x1.010e2e7ec0c83p+0"), h("0x1.44bf00479249dp+17"))], N,
                     lambda: (g.uniform(0.01, 100.0), g.uniform(-50.0, 50.0))), mp.power)
real2("powi", sample([(1.1, 2.0), (0.7, 3.0), (1.3, 12.0), (0.9, -5.0), (-1.7, 7.0), (2.5, -12.0),
                      (7.6268145870115305, -3.0), (3.3, 20.0), (0.9, -31.0), (1.01, 1000.0),
                      (0.3, -1.0), (0.3, 1.0), (0.3, 2.0)], N,
                     lambda: (g.uniform(0.5, 2.0), float(int(g.uniform(-40.0, 41.0))))), mp.power)

# --- the complex functions: the edges of their formulas and branch cuts, then a box ---

box = lambda: (g.uniform(-20.0, 20.0), g.uniform(-20.0, 20.0))
cplx("c_abs", sample([(3.0, 4.0), (1e300, 1e300), (1e-300, 1e-300), (-1.0, 1e-10)], N, box),
     mp.fabs, real_out=True)
cplx("c_arg", sample([(3.0, 4.0), (-1.0, 1e-10), (-1.0, -1e-10), (0.0, 1.0), (1e-300, 1.0)], N, box),
     mp.arg, real_out=True)
cplx("c_exp", sample([(0.0, PI), (1.0, PI / 2), (-1.0, 1e6), (10.0, -3.0), (700.0, 1.0),
                      (-700.0, 2.0), (1e-10, 1e-10)], N, box), mp.exp)
cplx("c_ln", sample([(-1.0, 1e-10), (-1.0, -1e-10), (0.0, 1.0), (1.0, 1e-8), (1e300, 1e300),
                     (3.0, 4.0), (0.6, 0.8), (1.0 + 1e-10, 1e-5), (1e-300, 1e-300), (0.7, 0.7),
                     (1.0 - 2.0**-30, 2.0**-15), (-2.0, 0.5), (1e-5, 1.0)], N, box), mp.log)
cplx("c_sqrt", sample([(-4.0, 1e-10), (-4.0, -1e-10), (3.0, 4.0), (0.0, 2.0), (1e-10, 1.0),
                       (-1.0, 1.0), (1e300, 1e300), (-1e300, 1e300), (1e-300, 1e-300),
                       (-1e-300, 3e-300), (1.7e308, 1.7e308), (4.0, 1e-10), (-4.0, 1e-300)], N, box),
     mp.sqrt)
trig_z = [(0.5, 1e-10), (1.0, 2.0), (PI / 2, 1.0), (1e-10, 5.0), (3.0, -20.0), (-2.0, 0.3),
          (1e6, 0.5), (0.25, 300.0), (1e-300, 1e-300)]
cplx("c_sin", sample(trig_z, N, box), mp.sin)
cplx("c_cos", sample(trig_z, N, box), mp.cos)
cplx("c_sinh", sample([(b, a) for a, b in trig_z], N, box), mp.sinh)
cplx("c_cosh", sample([(b, a) for a, b in trig_z], N, box), mp.cosh)
tanh_z = [(0.5, 1e-10), (1.0, 2.0), (1e-10, PI / 2 - 1e-6), (21.0, 1.0), (23.0, -0.5),
          (-30.0, 2.0), (0.3, 1e6), (5.0, 1.5), (-1e-3, 1.57), (1e-300, 1e-300)]
cplx("c_tanh", sample(tanh_z, N, box), mp.tanh)
cplx("c_tan", sample([(b, a) for a, b in tanh_z], N, box), mp.tan)
cplx("c_asin", sample([(0.5, 1e-10), (2.0, 1e-10), (2.0, -1e-10), (-2.0, 1e-10), (1.0, 1e-10),
                       (1.0, 1e-5), (0.999, 1e-3), (1e-10, 1.0), (1e10, 1e10), (1e200, 1.0),
                       (1e-200, 1e-200), (0.3, 1e-200), (1.5, 1e-200), (1.0, 1e-200), (0.6, 0.6),
                       (-3.0, 4.0), (1.2, 0.3), (1e-12, 1e-12), (0.4, 0.1)], N, box), mp.asin)
for theta in sample([0.0, PI / 2, PI, 1e6, 1e-300], 24, lambda: g.uniform(-100.0, 100.0)):
    w = mp.expj(mp.mpf(theta))
    r, i = rnd(mp.re(w)), rnd(mp.im(w))
    line("c_cis", theta, None, None, None, r, i, f"{theta!r} -> {r!r}{i:+}i")
for rr, theta in sample([(2.0, 1.0), (1e300, 0.5), (1e-300, 2.0), (3.0, 1e6)], 24,
                        lambda: (g.log_uniform(1e-10, 1e10), g.uniform(-10.0, 10.0))):
    w = mp.mpf(rr) * mp.expj(mp.mpf(theta))
    r, i = rnd(mp.re(w)), rnd(mp.im(w))
    line("c_from_polar", rr, None, theta, None, r, i, f"{rr!r}, {theta!r} -> {r!r}{i:+}i")
for (x, y), p in sample([((3.0, 4.0), 0.5), ((-4.0, 1e-10), 0.5), ((-1.0, 1.0), 2.5),
                         ((0.5, -0.5), -3.0), ((10.0, 1.0), 1.0 / 3.0)], 32,
                        lambda: (box(), g.uniform(-4.0, 4.0))):
    w = mp.power(mp.mpc(x, y), mp.mpf(p))
    r, i = rnd(mp.re(w)), rnd(mp.im(w))
    line("c_powf", x, y, p, None, r, i, f"({x!r}{y:+}i)^{p!r} -> {r!r}{i:+}i")
for (x, y), (p, q) in sample([((3.0, 4.0), (0.5, 0.5)), ((-1.0, 1e-10), (0.0, 1.0)),
                              ((2.0, 0.0), (1.0, 1.0)), ((0.5, -0.5), (-2.0, 0.25))], 32,
                             lambda: ((g.uniform(-5.0, 5.0), g.uniform(-5.0, 5.0)),
                                      (g.uniform(-2.0, 2.0), g.uniform(-2.0, 2.0)))):
    w = mp.power(mp.mpc(x, y), mp.mpc(p, q))
    r, i = rnd(mp.re(w)), rnd(mp.im(w))
    line("c_powc", x, y, p, q, r, i, f"({x!r}{y:+}i)^({p!r}{q:+}i) -> {r!r}{i:+}i")

sys.stdout.reconfigure(newline="\n")  # LF on every system
print(__doc__.strip().replace("\n", "\n# ").join(["# ", ""]))
print(f"# mpmath {mp.__version__}, {mp.mp.prec} bits")
print("\n".join(lines))
