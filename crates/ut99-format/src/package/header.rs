use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

pub const PACKAGE_FILE_TAG: u32 = 0x9E2A83C1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationInfo {
    pub export_count: u32,
    pub name_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFileSummary {
    pub tag: u32,
    pub file_version: u32,
    pub pkg_flags: u32,
    pub name_count: u32,
    pub name_offset: u32,
    pub export_count: u32,
    pub export_offset: u32,
    pub import_count: u32,
    pub import_offset: u32,
    pub guid: [u8; 16],
    pub generations: Vec<GenerationInfo>,
}

impl Default for PackageFileSummary {
    fn default() -> Self {
        Self {
            tag: PACKAGE_FILE_TAG,
            file_version: 68,
            pkg_flags: 0,
            name_count: 0,
            name_offset: 0,
            export_count: 0,
            export_offset: 0,
            import_count: 0,
            import_offset: 0,
            guid: [0; 16],
            generations: Vec::new(),
        }
    }
}

impl PackageFileSummary {

    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let tag = reader.read_u32::<LittleEndian>()?;
        if tag != PACKAGE_FILE_TAG {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid package tag: 0x{tag:08X}, expected 0x{PACKAGE_FILE_TAG:08X}"),
            ));
        }

        let file_version = reader.read_u32::<LittleEndian>()?;
        let pkg_flags = reader.read_u32::<LittleEndian>()?;
        let name_count = reader.read_u32::<LittleEndian>()?;
        let name_offset = reader.read_u32::<LittleEndian>()?;
        let export_count = reader.read_u32::<LittleEndian>()?;
        let export_offset = reader.read_u32::<LittleEndian>()?;
        let import_count = reader.read_u32::<LittleEndian>()?;
        let import_offset = reader.read_u32::<LittleEndian>()?;

        let mut guid = [0u8; 16];
        let mut generations = Vec::new();

        if file_version >= 68 {
            reader.read_exact(&mut guid)?;
            let gen_count = reader.read_u32::<LittleEndian>()?;
            for _ in 0..gen_count {
                let exp_cnt = reader.read_u32::<LittleEndian>()?;
                let nm_cnt = reader.read_u32::<LittleEndian>()?;
                generations.push(GenerationInfo {
                    export_count: exp_cnt,
                    name_count: nm_cnt,
                });
            }
        } else {
            // UE1 file_version < 68 has heritage_count (u32) + heritage_offset (u32)
            let _ = reader.read_exact(&mut guid[0..8]);
        }

        Ok(Self {
            tag,
            file_version,
            pkg_flags,
            name_count,
            name_offset,
            export_count,
            export_offset,
            import_count,
            import_offset,
            guid,
            generations,
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u32::<LittleEndian>(self.tag)?;
        writer.write_u32::<LittleEndian>(self.file_version)?;
        writer.write_u32::<LittleEndian>(self.pkg_flags)?;
        writer.write_u32::<LittleEndian>(self.name_count)?;
        writer.write_u32::<LittleEndian>(self.name_offset)?;
        writer.write_u32::<LittleEndian>(self.export_count)?;
        writer.write_u32::<LittleEndian>(self.export_offset)?;
        writer.write_u32::<LittleEndian>(self.import_count)?;
        writer.write_u32::<LittleEndian>(self.import_offset)?;

        if self.file_version >= 68 {
            writer.write_all(&self.guid)?;
            writer.write_u32::<LittleEndian>(self.generations.len() as u32)?;
            for gen in &self.generations {
                writer.write_u32::<LittleEndian>(gen.export_count)?;
                writer.write_u32::<LittleEndian>(gen.name_count)?;
            }
        } else {
            writer.write_all(&self.guid[0..8])?;
        }

        Ok(())
    }
}
