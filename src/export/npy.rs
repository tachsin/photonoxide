//! `.npy` array framing (NumPy's own format, *NEP 1*, <https://numpy.org/neps/nep-0001-npy-format.html>):
//! a magic string, a version, a little-endian header dict, then the raw array bytes in C order.
//! Writing only: photonoxide never reads `.npy` or `.npz` files back (NumPy and this module's
//! own tests do).

use num_complex::Complex64 as c64;

use super::Precision;

/// An array's element type: real or complex, at [`Precision::F32`] or [`Precision::F64`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dtype {
    Real(Precision),
    Complex(Precision),
}

impl Dtype {
    /// NumPy's `descr` string: `<f4`, `<f8`, `<c8` or `<c16`, little-endian.
    fn descr(self) -> &'static str {
        match self {
            Dtype::Real(Precision::F32) => "<f4",
            Dtype::Real(Precision::F64) => "<f8",
            Dtype::Complex(Precision::F32) => "<c8",
            Dtype::Complex(Precision::F64) => "<c16",
        }
    }

    /// Bytes an element takes.
    pub(super) fn item_size(self) -> usize {
        match self {
            Dtype::Real(Precision::F32) => 4,
            Dtype::Real(Precision::F64) => 8,
            Dtype::Complex(Precision::F32) => 8,
            Dtype::Complex(Precision::F64) => 16,
        }
    }
}

/// The header dict NumPy expects, padded with spaces to a multiple of 64 bytes (magic, version
/// and header-length bytes included) and ending with `\n`.
fn header(dtype: Dtype, shape: &[usize]) -> Vec<u8> {
    let shape_str = match shape {
        [] => "()".to_owned(),
        [n] => format!("({n},)"),
        many => format!(
            "({})",
            many.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let dict = format!(
        "{{'descr': '{}', 'fortran_order': False, 'shape': {shape_str}, }}",
        dtype.descr()
    );
    // magic (6) + version (2) + header length (2) + dict + '\n', a multiple of 64
    let prefix = 6 + 2 + 2;
    let pad = (64 - (prefix + dict.len() + 1) % 64) % 64;
    let mut bytes = dict.into_bytes();
    bytes.extend(std::iter::repeat_n(b' ', pad));
    bytes.push(b'\n');
    let mut out = Vec::with_capacity(prefix + bytes.len());
    out.extend_from_slice(b"\x93NUMPY");
    out.push(1); // major version
    out.push(0); // minor version
    out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    out.extend_from_slice(&bytes);
    out
}

/// A real array's `.npy` bytes, `values` in C order (the last axis fastest) for `shape`.
///
/// # Panics
///
/// If `values.len()` isn't `shape`'s product.
pub(super) fn real(shape: &[usize], precision: Precision, values: &[f64]) -> Vec<u8> {
    assert_eq!(values.len(), shape.iter().product::<usize>());
    let dtype = Dtype::Real(precision);
    let mut out = header(dtype, shape);
    out.reserve(values.len() * dtype.item_size());
    for &v in values {
        match precision {
            Precision::F32 => out.extend_from_slice(&(v as f32).to_le_bytes()),
            Precision::F64 => out.extend_from_slice(&v.to_le_bytes()),
        }
    }
    out
}

/// A complex array's `.npy` bytes, real and imaginary parts interleaved, `values` in C order.
///
/// # Panics
///
/// If `values.len()` isn't `shape`'s product.
pub(super) fn complex(shape: &[usize], precision: Precision, values: &[c64]) -> Vec<u8> {
    assert_eq!(values.len(), shape.iter().product::<usize>());
    let dtype = Dtype::Complex(precision);
    let mut out = header(dtype, shape);
    out.reserve(values.len() * dtype.item_size());
    for v in values {
        match precision {
            Precision::F32 => {
                out.extend_from_slice(&(v.re as f32).to_le_bytes());
                out.extend_from_slice(&(v.im as f32).to_le_bytes());
            }
            Precision::F64 => {
                out.extend_from_slice(&v.re.to_le_bytes());
                out.extend_from_slice(&v.im.to_le_bytes());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_64_aligned_and_terminated() {
        for shape in [vec![], vec![3], vec![3, 4], vec![128, 256]] {
            let h = header(Dtype::Real(Precision::F64), &shape);
            assert_eq!(h.len() % 64, 0, "{shape:?}");
            assert_eq!(&h[..6], b"\x93NUMPY");
            assert_eq!(h[h.len() - 1], b'\n');
        }
    }

    #[test]
    fn real_round_trips_as_little_endian_f64() {
        let bytes = real(&[2], Precision::F64, &[1.5, -2.25]);
        let header = header(Dtype::Real(Precision::F64), &[2]);
        let data = &bytes[header.len()..];
        assert_eq!(data.len(), 16);
        assert_eq!(f64::from_le_bytes(data[0..8].try_into().unwrap()), 1.5);
        assert_eq!(f64::from_le_bytes(data[8..16].try_into().unwrap()), -2.25);
    }

    #[test]
    fn complex_interleaves_re_im_as_f32() {
        let bytes = complex(&[1], Precision::F32, &[c64::new(1.0, -1.0)]);
        let header = header(Dtype::Complex(Precision::F32), &[1]);
        let data = &bytes[header.len()..];
        assert_eq!(data.len(), 8);
        assert_eq!(f32::from_le_bytes(data[0..4].try_into().unwrap()), 1.0);
        assert_eq!(f32::from_le_bytes(data[4..8].try_into().unwrap()), -1.0);
    }
}
