use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{self, Cursor, Read, Write};

use super::compact_index::{read_compact_index, write_compact_index};
use super::header::PACKAGE_FILE_TAG;

/// Magic tag identifying Unreal Tournament 99 redirect (.uz) files (`1234` / `0x000004D2`).
pub const UZ_FILE_TAG: u32 = 1234;

/// Checks whether raw byte data begins with the UT99 redirect `.uz` header tag.
pub fn is_uz_file(data: &[u8]) -> bool {
    if data.len() < 5 {
        return false;
    }
    let tag = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    tag == UZ_FILE_TAG
}

/// Decompresses an Unreal Tournament 99 `.uz` redirect file stream.
///
/// Returns `(original_filename, decompressed_package_bytes)`.
pub fn decompress_uz<R: Read>(reader: &mut R) -> io::Result<(String, Vec<u8>)> {
    let tag = reader.read_u32::<LittleEndian>()?;
    if tag != UZ_FILE_TAG {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid UZ tag: 0x{tag:08X} (expected {UZ_FILE_TAG})"),
        ));
    }

    let name_len = read_compact_index(reader)?;
    if name_len <= 0 || name_len > 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid embedded filename length in .uz: {name_len}"),
        ));
    }

    let mut name_bytes = vec![0u8; name_len as usize];
    reader.read_exact(&mut name_bytes)?;
    let original_filename = String::from_utf8_lossy(&name_bytes).trim_end_matches('\0').to_string();

    // Read remaining compressed payload
    let mut compressed_payload = Vec::new();
    reader.read_to_end(&mut compressed_payload)?;

    // Decompress using ZlibDecoder with fallback to raw DeflateDecoder
    let mut decompressed = Vec::new();
    let mut zlib_cursor = Cursor::new(&compressed_payload);
    let mut decoder = ZlibDecoder::new(&mut zlib_cursor);

    if decoder.read_to_end(&mut decompressed).is_ok() && !decompressed.is_empty() {
        return Ok((original_filename, decompressed));
    }

    // Fallback: raw DEFLATE stream without zlib wrapper
    decompressed.clear();
    let mut deflate_cursor = Cursor::new(&compressed_payload);
    let mut def_decoder = DeflateDecoder::new(&mut deflate_cursor);
    def_decoder.read_to_end(&mut decompressed)?;

    Ok((original_filename, decompressed))
}

/// Compresses an Unreal Tournament package into a `.uz` redirect file.
///
/// Matches the output produced by `ucc.exe compress <package>`.
pub fn compress_uz(original_filename: &str, data: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();

    // 1. Write UZ tag (1234)
    out.write_u32::<LittleEndian>(UZ_FILE_TAG)?;

    // 2. Write filename length and string
    let name_bytes = original_filename.as_bytes();
    write_compact_index(&mut out, name_bytes.len() as i32)?;
    out.extend_from_slice(name_bytes);

    // 3. Compress package payload via Zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    let compressed_payload = encoder.finish()?;

    out.extend_from_slice(&compressed_payload);
    Ok(out)
}

/// Automatically detects whether `data` is a `.uz` compressed file or an uncompressed
/// package file (`0x9E2A83C1`), decompressing if necessary.
pub fn detect_and_unpack(data: &[u8], filename_hint: &str) -> io::Result<(String, Vec<u8>)> {
    if is_uz_file(data) {
        let mut cursor = Cursor::new(data);
        decompress_uz(&mut cursor)
    } else if data.len() >= 4 {
        let tag = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if tag == PACKAGE_FILE_TAG {
            // Raw uncompressed Unreal package
            let base_name = filename_hint
                .strip_suffix(".uz")
                .or_else(|| filename_hint.strip_suffix(".UZ"))
                .unwrap_or(filename_hint);
            Ok((base_name.to_string(), data.to_vec()))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unrecognized package format (magic tag 0x{tag:08X})"),
            ))
        }
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Package data too small",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uz_compression_decompression_roundtrip() {
        let fake_package_payload = b"\xC1\x83\x2A\x9E\x44\x00\x00\x00Hello UT99 Redirect World Test Data 1234567890!";
        let filename = "DM-Deck16][.unr";

        let compressed = compress_uz(filename, fake_package_payload).expect("Failed to compress");
        assert!(is_uz_file(&compressed));

        let mut cursor = Cursor::new(&compressed);
        let (decomp_name, decomp_data) = decompress_uz(&mut cursor).expect("Failed to decompress");

        assert_eq!(decomp_name, filename);
        assert_eq!(decomp_data, fake_package_payload);
    }

    #[test]
    fn test_detect_and_unpack_both_modes() {
        let fake_package = vec![0xC1, 0x83, 0x2A, 0x9E, 0x44, 0x00, 0x00, 0x00, 0xFF, 0xAA];
        // 1. Raw package test
        let (name1, data1) = detect_and_unpack(&fake_package, "Botpack.u").unwrap();
        assert_eq!(name1, "Botpack.u");
        assert_eq!(data1, fake_package);

        // 2. Compressed .uz test
        let compressed = compress_uz("Botpack.u", &fake_package).unwrap();
        let (name2, data2) = detect_and_unpack(&compressed, "Botpack.u.uz").unwrap();
        assert_eq!(name2, "Botpack.u");
        assert_eq!(data2, fake_package);
    }
}
