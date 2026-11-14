// The two maths functions behind numbers the studio shows as results that the front end computes
// itself: the decimal logarithm (an S-parameter or a spectrum in dB, an error's log₁₀) and the
// angle of a point (an S-parameter's phase). Every number shown as a result comes from Rust
// (#822; CONTRIBUTING.md, "The same numbers on every system"), and these give Rust's bits:
// `photonoxide::portable::log10` and `atan2`, the `libm` crate's ports of FreeBSD's e_log10.c,
// e_atan2.c and s_atan.c, written out here operation for operation. They use + − × ÷ alone, which
// every JavaScript engine rounds as IEEE 754 binary64 requires (no fused operations), and read
// and write a number's bits through a DataView. `Math.log10` and `Math.atan2` are each engine's
// own (V8's in WebView2 on Windows, JavaScriptCore's in WebKit on macOS and Linux), which the
// language leaves "implementation-approximated", so they may round differently.
//
// tests/portable.test.ts holds them to the bits that photonoxide::portable gives on a fixed set of
// arguments (tests/portable-maths.json), and the studio's Rust test `front_end_maths` holds
// photonoxide::portable to the same file, so a change on either side fails a test. Drawing (pixel
// positions, ticks, the diagrams) keeps Math's functions: those numbers are never results.
//
// The functions are adapted from FreeBSD's msun, which carries this notice:
//
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.

const bits = new DataView(new ArrayBuffer(8));

/** The high 32 bits of x's binary64 representation, as an unsigned integer. */
function high(x: number): number {
  bits.setFloat64(0, x);
  return bits.getUint32(0);
}

/** The low 32 bits of x's binary64 representation, as an unsigned integer. */
function low(x: number): number {
  bits.setFloat64(0, x);
  return bits.getUint32(4);
}

/** The number whose binary64 representation has these high and low 32 bits. */
function fromWords(hi: number, lo: number): number {
  bits.setUint32(0, hi >>> 0);
  bits.setUint32(4, lo >>> 0);
  return bits.getFloat64(0);
}

// e_log10.c's constants, by their bits
const TWO54 = fromWords(0x43500000, 0);
const IVLN10HI = fromWords(0x3fdbcb7b, 0x15200000);
const IVLN10LO = fromWords(0x3dbb9438, 0xca9aadd5);
const LOG10_2HI = fromWords(0x3fd34413, 0x509f6000);
const LOG10_2LO = fromWords(0x3d59fef3, 0x11f12b36);
const LG1 = fromWords(0x3fe55555, 0x55555593);
const LG2 = fromWords(0x3fd99999, 0x9997fa04);
const LG3 = fromWords(0x3fd24924, 0x94229359);
const LG4 = fromWords(0x3fcc71c5, 0x1d8e78af);
const LG5 = fromWords(0x3fc74664, 0x96cb03de);
const LG6 = fromWords(0x3fc39a09, 0xd078c69f);
const LG7 = fromWords(0x3fc2f112, 0xdf3e5244);

/**
 * The base-10 logarithm of x, the same bits as photonoxide::portable::log10 (within 0.814 ulp):
 * −∞ for ±0, NaN below 0 and for NaN, ∞ for ∞, exactly 0 for 1.
 */
export function log10(x: number): number {
  let hx = high(x);
  const lx = low(x);
  let k = 0;
  if (hx < 0x00100000 || hx >>> 31 > 0) {
    if (((hx & 0x7fffffff) | lx) === 0) return -1 / (x * x); // log(±0) = −∞
    if (hx >>> 31 > 0) return (x - x) / 0; // log(−x) = NaN
    // a subnormal x: scale it up
    k -= 54;
    x *= TWO54;
    hx = high(x);
  } else if (hx >= 0x7ff00000) {
    return x;
  } else if (hx === 0x3ff00000 && lx === 0) {
    return 0;
  }
  // reduce x into [√2/2, √2]
  hx = (hx + (0x3ff00000 - 0x3fe6a09e)) >>> 0;
  k += (hx >>> 20) - 0x3ff;
  hx = ((hx & 0x000fffff) + 0x3fe6a09e) >>> 0;
  x = fromWords(hx, low(x));

  const f = x - 1;
  const hfsq = 0.5 * f * f;
  const s = f / (2 + f);
  const z = s * s;
  let w = z * z;
  const t1 = w * (LG2 + w * (LG4 + w * LG6));
  const t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
  const r = t2 + t1;
  // hi + lo = f − hfsq + s (hfsq + r) ≈ log(1 + f), hi with its low word cleared
  const hi = fromWords(high(f - hfsq), 0);
  const lo = f - hi - hfsq + s * (hfsq + r);
  // valHi + valLo ≈ log10(1 + f) + k log10(2)
  let valHi = hi * IVLN10HI;
  const dk = k;
  const y = dk * LOG10_2HI;
  let valLo = dk * LOG10_2LO + (lo + hi) * IVLN10LO + lo * IVLN10HI;
  w = y + valHi;
  valLo += y - w + valHi;
  valHi = w;
  return valLo + valHi;
}

// s_atan.c's constants, by their bits: atan(0.5), atan(1), atan(1.5) and atan(∞) in two parts,
// and the polynomial's coefficients
const ATANHI = [fromWords(0x3fddac67, 0x0561bb4f), fromWords(0x3fe921fb, 0x54442d18), fromWords(0x3fef730b, 0xd281f69b), fromWords(0x3ff921fb, 0x54442d18)];
const ATANLO = [fromWords(0x3c7a2b7f, 0x222f65e2), fromWords(0x3c81a626, 0x33145c07), fromWords(0x3c700788, 0x7af0cbbd), fromWords(0x3c91a626, 0x33145c07)];
const AT = [
  fromWords(0x3fd55555, 0x5555550d),
  fromWords(0xbfc99999, 0x9998ebc4),
  fromWords(0x3fc24924, 0x920083ff),
  fromWords(0xbfbc71c6, 0xfe231671),
  fromWords(0x3fb745cd, 0xc54c206e),
  fromWords(0xbfb3b0f2, 0xaf749a6d),
  fromWords(0x3fb10d66, 0xa0d03d51),
  fromWords(0xbfadde2d, 0x52defd9a),
  fromWords(0x3fa97b4b, 0x24760deb),
  fromWords(0xbfa2b444, 0x2c6a6c2f),
  fromWords(0x3f90ad3a, 0xe322da11),
];
// the libm crate's tiny addend to atan(∞)'s high part (its bits 0x0380_0000), which leaves it as it is
const TINY = fromWords(0, 0x03800000);

/** The arctangent of x, in [−π/2, π/2]: the libm crate's s_atan.c, as photonoxide::portable::atan. */
function atan(x: number): number {
  let ix = high(x);
  const sign = ix >>> 31;
  ix &= 0x7fffffff;
  if (ix >= 0x44100000) {
    if (Number.isNaN(x)) return x;
    const z = ATANHI[3] + TINY;
    return sign !== 0 ? -z : z;
  }
  let id: number;
  if (ix < 0x3fdc0000) {
    // |x| < 0.4375
    if (ix < 0x3e400000) return x; // |x| < 2⁻²⁷
    id = -1;
  } else {
    x = Math.abs(x);
    if (ix < 0x3ff30000) {
      // |x| < 1.1875
      if (ix < 0x3fe60000) {
        // 7/16 <= |x| < 11/16
        x = (2 * x - 1) / (2 + x);
        id = 0;
      } else {
        // 11/16 <= |x| < 19/16
        x = (x - 1) / (x + 1);
        id = 1;
      }
    } else if (ix < 0x40038000) {
      // |x| < 2.4375
      x = (x - 1.5) / (1 + 1.5 * x);
      id = 2;
    } else {
      // 2.4375 <= |x| < 2⁶⁶
      x = -1 / x;
      id = 3;
    }
  }
  const z = x * x;
  const w = z * z;
  // the sum of AT[i] z^(i+1), as its odd and even parts
  const s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
  const s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
  if (id < 0) return x - x * (s1 + s2);
  const r = ATANHI[id] - (x * (s1 + s2) - ATANLO[id] - x);
  return sign !== 0 ? -r : r;
}

const PI = fromWords(0x400921fb, 0x54442d18);
const PI_LO = fromWords(0x3ca1a626, 0x33145c07);

/**
 * The angle of the point (x, y), in [−π, π], as `Math.atan2(y, x)`, with C's special cases for
 * zeros and infinities: the same bits as photonoxide::portable::atan2 (within 1.55 ulps).
 */
export function atan2(y: number, x: number): number {
  if (Number.isNaN(x) || Number.isNaN(y)) return x + y;
  let ix = high(x);
  const lx = low(x);
  let iy = high(y);
  const ly = low(y);
  if ((((ix - 0x3ff00000) >>> 0) | lx) === 0) return atan(y); // x = 1
  const m = ((iy >>> 31) & 1) | ((ix >>> 30) & 2); // 2 sign(x) + sign(y)
  ix &= 0x7fffffff;
  iy &= 0x7fffffff;
  // y = 0
  if ((iy | ly) === 0) {
    if (m === 0 || m === 1) return y; // atan(±0, +anything) = ±0
    return m === 2 ? PI : -PI; // atan(±0, −anything) = ±π
  }
  // x = 0
  if ((ix | lx) === 0) return m & 1 ? -PI / 2 : PI / 2;
  // x infinite
  if (ix === 0x7ff00000) {
    if (iy === 0x7ff00000) return [PI / 4, -PI / 4, (3 * PI) / 4, (-3 * PI) / 4][m];
    return [0, -0, PI, -PI][m];
  }
  // |y/x| > 2⁶⁴
  if (((ix + (64 << 20)) >>> 0) < iy || iy === 0x7ff00000) return m & 1 ? -PI / 2 : PI / 2;
  // z = atan(|y/x|) without a spurious underflow
  const z = m & 2 && ((iy + (64 << 20)) >>> 0) < ix ? 0 : atan(Math.abs(y / x));
  switch (m) {
    case 0:
      return z; // atan(+, +)
    case 1:
      return -z; // atan(−, +)
    case 2:
      return PI - (z - PI_LO); // atan(+, −)
    default:
      return z - PI_LO - PI; // atan(−, −)
  }
}
