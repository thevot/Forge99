use super::compact_index::{read_compact_index, write_compact_index};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectExport {
    pub class_index: i32,
    pub super_index: i32,
    pub package_index: i32,
    pub object_name: i32,
    pub object_flags: u32,
    pub serial_size: i32,
    pub serial_offset: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportTable {
    pub entries: Vec<ObjectExport>,
}

impl ExportTable {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn read<R: Read>(reader: &mut R, count: u32) -> io::Result<Self> {
        let mut entries = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let class_index = read_compact_index(reader)?;
            let super_index = read_compact_index(reader)?;
            let package_index = reader.read_i32::<LittleEndian>()?;
            let object_name = read_compact_index(reader)?;
            let object_flags = reader.read_u32::<LittleEndian>()?;
            let serial_size = read_compact_index(reader)?;
            let serial_offset = if serial_size > 0 {
                read_compact_index(reader)?
            } else {
                0
            };

            entries.push(ObjectExport {
                class_index,
                super_index,
                package_index,
                object_name,
                object_flags,
                serial_size,
                serial_offset,
            });
        }

        Ok(Self { entries })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for entry in &self.entries {
            write_compact_index(writer, entry.class_index)?;
            write_compact_index(writer, entry.super_index)?;
            writer.write_i32::<LittleEndian>(entry.package_index)?;
            write_compact_index(writer, entry.object_name)?;
            writer.write_u32::<LittleEndian>(entry.object_flags)?;
            write_compact_index(writer, entry.serial_size)?;
            if entry.serial_size > 0 {
                write_compact_index(writer, entry.serial_offset)?;
            }
        }
        Ok(())
    }
}
