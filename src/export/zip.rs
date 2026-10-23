//! A minimal ZIP writer: local file headers, central directory, end record (PKWARE's *APPNOTE*,
//! <https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT>, sections 4.3.6, 4.3.12 and
//! 4.3.16), stored or deflated. No Zip64, no data descriptors (every entry's size is known
//! before it is written), no encryption: exactly what an `.npz` needs, which NumPy's own
//! `numpy.lib.format` writes the same way. Writing only.

use std::io::{self, Write};

use flate2::Compression;
use flate2::write::DeflateEncoder;

const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_HEADER_SIGNATURE: u32 = 0x0201_4b50;
const END_SIGNATURE: u32 = 0x0605_4b50;
/// 1980-01-01, the MS-DOS epoch: this writer's files carry no real timestamp.
const DOS_DATE: u16 = 0x0021;

struct Entry {
    name: String,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    offset: u32,
    method: u16,
}

/// The *APPNOTE* version a reader needs for ZIP method `method` (4.4.3): 2.0 for stored and
/// deflated, 6.3 for Zstandard.
fn version_needed(method: u16) -> u16 {
    if method == 93 { 63 } else { 20 }
}

/// Builds a ZIP archive into `out`, one file at a time.
pub(super) struct ZipWriter<W: Write> {
    out: W,
    offset: u32,
    entries: Vec<Entry>,
}

impl<W: Write> ZipWriter<W> {
    pub(super) fn new(out: W) -> ZipWriter<W> {
        ZipWriter {
            out,
            offset: 0,
            entries: Vec::new(),
        }
    }

    /// Adds `data` under `name`, deflated at `level` (0 stores it instead, as `np.savez` does;
    /// 1 to 9 deflate, as `np.savez_compressed` does with zlib's default level).
    ///
    /// # Errors
    ///
    /// [`io::Error`] if `out` can't be written.
    pub(super) fn add(&mut self, name: &str, data: &[u8], level: u8) -> io::Result<()> {
        if level == 0 {
            return self.add_compressed(name, data, 0, data.to_vec());
        }
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::new(u32::from(level)));
        enc.write_all(data)?;
        let payload = enc.finish()?;
        self.add_compressed(name, data, 8, payload)
    }

    /// Adds `data` under `name` as `payload`, `data` compressed by ZIP method `method` (0
    /// stored, 8 deflate, 93 Zstandard: *APPNOTE* 4.4.5); the checksum and sizes are `data`'s.
    ///
    /// # Errors
    ///
    /// [`io::Error`] if `out` can't be written.
    pub(super) fn add_compressed(
        &mut self,
        name: &str,
        data: &[u8],
        method: u16,
        payload: Vec<u8>,
    ) -> io::Result<()> {
        let crc32 = crc32fast::hash(data);
        let name_bytes = name.as_bytes();
        let mut header = Vec::with_capacity(30 + name_bytes.len());
        header.extend_from_slice(&LOCAL_HEADER_SIGNATURE.to_le_bytes());
        header.extend_from_slice(&version_needed(method).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes()); // flags
        header.extend_from_slice(&method.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes()); // mod time
        header.extend_from_slice(&DOS_DATE.to_le_bytes());
        header.extend_from_slice(&crc32.to_le_bytes());
        header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes()); // extra field length
        header.extend_from_slice(name_bytes);
        self.out.write_all(&header)?;
        self.out.write_all(&payload)?;
        self.entries.push(Entry {
            name: name.to_owned(),
            crc32,
            compressed_size: payload.len() as u32,
            uncompressed_size: data.len() as u32,
            offset: self.offset,
            method,
        });
        self.offset += header.len() as u32 + payload.len() as u32;
        Ok(())
    }

    /// Writes the central directory and the end record, flushes `out` and gives it back.
    ///
    /// # Errors
    ///
    /// [`io::Error`] if `out` can't be written.
    pub(super) fn finish(mut self) -> io::Result<W> {
        let directory_start = self.offset;
        let mut directory = Vec::new();
        for e in &self.entries {
            let name_bytes = e.name.as_bytes();
            directory.extend_from_slice(&CENTRAL_HEADER_SIGNATURE.to_le_bytes());
            directory.extend_from_slice(&version_needed(e.method).to_le_bytes()); // made by
            directory.extend_from_slice(&version_needed(e.method).to_le_bytes()); // needed
            directory.extend_from_slice(&0u16.to_le_bytes()); // flags
            directory.extend_from_slice(&e.method.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes()); // mod time
            directory.extend_from_slice(&DOS_DATE.to_le_bytes());
            directory.extend_from_slice(&e.crc32.to_le_bytes());
            directory.extend_from_slice(&e.compressed_size.to_le_bytes());
            directory.extend_from_slice(&e.uncompressed_size.to_le_bytes());
            directory.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes()); // extra field length
            directory.extend_from_slice(&0u16.to_le_bytes()); // comment length
            directory.extend_from_slice(&0u16.to_le_bytes()); // disk number start
            directory.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
            directory.extend_from_slice(&0u32.to_le_bytes()); // external attributes
            directory.extend_from_slice(&e.offset.to_le_bytes());
            directory.extend_from_slice(name_bytes);
        }
        self.out.write_all(&directory)?;
        let mut end = Vec::with_capacity(22);
        end.extend_from_slice(&END_SIGNATURE.to_le_bytes());
        end.extend_from_slice(&0u16.to_le_bytes()); // this disk
        end.extend_from_slice(&0u16.to_le_bytes()); // disk with the central directory
        end.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        end.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        end.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        end.extend_from_slice(&directory_start.to_le_bytes());
        end.extend_from_slice(&0u16.to_le_bytes()); // comment length
        self.out.write_all(&end)?;
        self.out.flush()?;
        Ok(self.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every signature a reader looks for is where it belongs, and the offsets the end record
    /// and the central directory give land exactly on the local and central headers they name:
    /// a real parser's first checks (this writer's own round trip, against the `zip` crate, is
    /// in `crate::export::tests`).
    #[test]
    fn structure_is_well_formed() {
        let mut out = Vec::new();
        {
            let mut zip = ZipWriter::new(&mut out);
            zip.add("a.txt", b"hello", 0).unwrap();
            zip.add("b.txt", b"world, deflated", 6).unwrap();
            zip.finish().unwrap();
        }
        let end = u32::from_le_bytes(out[out.len() - 22..out.len() - 18].try_into().unwrap());
        assert_eq!(end, END_SIGNATURE);
        let directory_start =
            u32::from_le_bytes(out[out.len() - 6..out.len() - 2].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_le_bytes(
                out[directory_start..directory_start + 4]
                    .try_into()
                    .unwrap()
            ),
            CENTRAL_HEADER_SIGNATURE
        );
        assert_eq!(
            u32::from_le_bytes(out[0..4].try_into().unwrap()),
            LOCAL_HEADER_SIGNATURE
        );
    }
}
