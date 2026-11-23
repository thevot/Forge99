use super::compact_index::{read_compact_index, write_compact_index};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectImport {
    pub class_package: i32,
    pub class_name: i32,
    pub package_index: i32,
    pub object_name: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportTable {
    pub entries: Vec<ObjectImport>,
}

impl ImportTable {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn read<R: Read>(reader: &mut R, count: u32) -> io::Result<Self> {
        let mut entries = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let class_package = read_compact_index(reader)?;
            let class_name = read_compact_index(reader)?;
            let package_index = reader.read_i32::<LittleEndian>()?;
            let object_name = read_compact_index(reader)?;

            entries.push(ObjectImport {
                class_package,
                class_name,
                package_index,
                object_name,
            });
        }

        Ok(Self { entries })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for entry in &self.entries {
            write_compact_index(writer, entry.class_package)?;
            write_compact_index(writer, entry.class_name)?;
            writer.write_i32::<LittleEndian>(entry.package_index)?;
            write_compact_index(writer, entry.object_name)?;
        }
        Ok(())
    }
}
