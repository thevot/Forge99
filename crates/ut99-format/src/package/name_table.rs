use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    pub name: String,
    pub flags: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameTable {
    pub entries: Vec<NameEntry>,
}

impl NameTable {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn read<R: Read>(reader: &mut R, count: u32, version: u32) -> io::Result<Self> {
        let mut entries = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let name = if version >= 64 {
                // String with CompactIndex length prefix
                let len_val = super::compact_index::read_compact_index(reader)?;
                if len_val > 0 {
                    let len = len_val as usize;
                    let mut buf = vec![0u8; len];
                    reader.read_exact(&mut buf)?;
                    // Trim trailing null terminator if present
                    if let Some(&0) = buf.last() {
                        buf.pop();
                    }
                    String::from_utf8_lossy(&buf).to_string()
                } else if len_val < 0 {
                    // Wide UTF-16 string
                    let char_count = (-len_val) as usize;
                    let mut u16_buf = Vec::with_capacity(char_count);
                    for _ in 0..char_count {
                        u16_buf.push(reader.read_u16::<LittleEndian>()?);
                    }
                    if let Some(&0) = u16_buf.last() {
                        u16_buf.pop();
                    }
                    String::from_utf16_lossy(&u16_buf)
                } else {
                    String::new()
                }
            } else {
                // Null terminated ASCII
                let mut buf = Vec::new();
                loop {
                    let b = reader.read_u8()?;
                    if b == 0 {
                        break;
                    }
                    buf.push(b);
                }
                String::from_utf8_lossy(&buf).to_string()
            };

            let flags = reader.read_u32::<LittleEndian>()?;
            entries.push(NameEntry { name, flags });
        }

        Ok(Self { entries })
    }

    pub fn write<W: Write>(&self, writer: &mut W, version: u32) -> io::Result<()> {
        for entry in &self.entries {
            if version >= 64 {
                let bytes = entry.name.as_bytes();
                let len = (bytes.len() + 1) as i32;
                super::compact_index::write_compact_index(writer, len)?;
                writer.write_all(bytes)?;
                writer.write_u8(0)?; // null terminator
            } else {
                writer.write_all(entry.name.as_bytes())?;
                writer.write_u8(0)?;
            }
            writer.write_u32::<LittleEndian>(entry.flags)?;
        }
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|e| e.name.as_str())
    }

    pub fn add(&mut self, name: impl Into<String>, flags: u32) -> usize {
        let name = name.into();
        let effective_flags = if flags == 0 { 0x0007_0010 } else { flags };
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| e.name.eq_ignore_ascii_case(&name))
        {
            if self.entries[pos].flags == 0 {
                self.entries[pos].flags = effective_flags;
            }
            return pos;
        }
        let idx = self.entries.len();
        self.entries.push(NameEntry {
            name,
            flags: effective_flags,
        });
        idx
    }
}
