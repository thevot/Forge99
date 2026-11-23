use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

/// Decodes an Unreal Engine compact index (`FCompactIndex`) from a byte stream.
///
/// Unreal Engine 1 uses variable-length integers where:
/// - Byte 0: Bit 7 is Sign (1 = negative, 0 = positive)
/// - Byte 0: Bit 6 is MoreBytes flag (1 = more bytes follow)
/// - Byte 0: Bits 5..0 are the lowest 6 bits of the value
/// - Subsequent bytes: Bit 7 is MoreBytes, Bits 6..0 are 7 data bits
pub fn read_compact_index<R: Read>(reader: &mut R) -> io::Result<i32> {
    let b0 = reader.read_u8()?;
    let is_neg = (b0 & 0x80) != 0;
    let mut val = (b0 & 0x3F) as u32;

    if (b0 & 0x40) != 0 {
        let mut shift = 6;
        loop {
            let b = reader.read_u8()?;
            val |= ((b & 0x7F) as u32) << shift;
            shift += 7;
            if (b & 0x80) == 0 || shift >= 32 {
                break;
            }
        }
    }

    if is_neg {
        Ok((val as i32).wrapping_neg())
    } else {
        Ok(val as i32)
    }
}

/// Encodes an integer as an Unreal Engine compact index (`FCompactIndex`).
pub fn write_compact_index<W: Write>(writer: &mut W, value: i32) -> io::Result<usize> {
    let sign_bit = if value < 0 { 0x80u8 } else { 0x00u8 };
    let mut abs_val = value.unsigned_abs();

    let mut b0 = sign_bit | ((abs_val & 0x3F) as u8);
    abs_val >>= 6;

    if abs_val > 0 {
        b0 |= 0x40; // More bytes follow
        writer.write_u8(b0)?;
        let mut bytes_written = 1;

        while abs_val > 0 {
            let mut b = (abs_val & 0x7F) as u8;
            abs_val >>= 7;
            if abs_val > 0 {
                b |= 0x80;
            }
            writer.write_u8(b)?;
            bytes_written += 1;
        }
        Ok(bytes_written)
    } else {
        writer.write_u8(b0)?;
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_compact_index_roundtrip() {
        let test_cases = [0, 1, 42, 63, 64, 127, 128, 255, 1000, 65535, 1_000_000, -1, -42, -1000];
        for &val in &test_cases {
            let mut buf = Vec::new();
            write_compact_index(&mut buf, val).unwrap();
            let mut cursor = Cursor::new(&buf);
            let decoded = read_compact_index(&mut cursor).unwrap();
            assert_eq!(val, decoded, "Failed roundtrip for value {val}");
        }
    }
}
