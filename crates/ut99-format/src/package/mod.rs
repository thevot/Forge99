pub mod compact_index;
pub mod decompressor;
pub mod dependency;
pub mod export_table;
pub mod header;
pub mod import_table;
pub mod name_table;
pub mod resource;

pub use compact_index::{read_compact_index, write_compact_index};
pub use decompressor::{compress_uz, decompress_uz, detect_and_unpack, is_uz_file, UZ_FILE_TAG};
pub use dependency::{
    scan_map_dependencies, MapManifest, PackageDependency, STOCK_PACKAGES,
};
pub use export_table::{ExportTable, ObjectExport};
pub use header::{GenerationInfo, PackageFileSummary, PACKAGE_FILE_TAG};
pub use import_table::{ImportTable, ObjectImport};
pub use name_table::{NameEntry, NameTable};
pub use resource::{
    build_class_tree, build_uax_package, build_umx_package, decode_umesh_geometry,
    encode_music_export, encode_sound_export, encode_umesh_geometry, extract_class_script_text,
    extract_u_package_meshes, pack_fmesh_vert, parse_music_export, parse_sound_export,
    scan_u_package_classes, scan_u_package_meshes, unpack_fmesh_vert, write_sounds_into_package,
    ClassResource, ClassTreeNode, MusicResource, PackageMeshData, SoundResource,
};

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Package {
    pub summary: PackageFileSummary,
    pub names: NameTable,
    pub imports: ImportTable,
    pub exports: ExportTable,
    pub raw_data: Vec<u8>,
}

impl Default for Package {
    fn default() -> Self {
        Self {
            summary: PackageFileSummary::default(),
            names: NameTable::default(),
            imports: ImportTable::default(),
            exports: ExportTable::default(),
            raw_data: Vec::new(),
        }
    }
}

impl Package {

    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut file = File::open(path)?;
        let mut raw_data = Vec::new();
        file.read_to_end(&mut raw_data)?;
        Self::from_bytes(raw_data)
    }

    pub fn from_bytes(raw_data: Vec<u8>) -> anyhow::Result<Self> {
        let mut cursor = Cursor::new(&raw_data);
        let summary = PackageFileSummary::read(&mut cursor)?;

        // Read Name Table
        cursor.seek(SeekFrom::Start(summary.name_offset as u64))?;
        let names = NameTable::read(&mut cursor, summary.name_count, summary.file_version)?;

        // Read Import Table
        cursor.seek(SeekFrom::Start(summary.import_offset as u64))?;
        let imports = ImportTable::read(&mut cursor, summary.import_count)?;

        // Read Export Table
        cursor.seek(SeekFrom::Start(summary.export_offset as u64))?;
        let exports = ExportTable::read(&mut cursor, summary.export_count)?;

        Ok(Self {
            summary,
            names,
            imports,
            exports,
            raw_data,
        })
    }

    pub fn to_bytes(&self) -> std::io::Result<Vec<u8>> {
        let mut summary = self.summary.clone();
        // Ensure generations has at least 1 entry if file_version >= 68
        if summary.file_version >= 68 && summary.generations.is_empty() {
            summary.generations.push(GenerationInfo {
                export_count: self.exports.entries.len() as u32,
                name_count: self.names.entries.len() as u32,
            });
        }

        // 1. Calculate header size
        let mut header_buf = Vec::new();
        summary.write(&mut header_buf)?;
        let header_len = header_buf.len() as u32;

        // 2. Write Name Table immediately after header
        summary.name_offset = header_len;
        summary.name_count = self.names.entries.len() as u32;
        let mut names_buf = Vec::new();
        self.names.write(&mut names_buf, summary.file_version)?;

        // 3. Write Import Table immediately after name table
        summary.import_offset = summary.name_offset + names_buf.len() as u32;
        summary.import_count = self.imports.entries.len() as u32;
        let mut imports_buf = Vec::new();
        self.imports.write(&mut imports_buf)?;

        // 4. Collect and arrange export payloads
        let export_data_start = summary.import_offset + imports_buf.len() as u32;
        let mut export_data_buf = Vec::new();
        let mut updated_exports = self.exports.clone();

        for (i, entry) in updated_exports.entries.iter_mut().enumerate() {
            let payload = self.get_export_bytes(i).unwrap_or(&[]);
            if !payload.is_empty() {
                let new_serial_offset = (export_data_start + export_data_buf.len() as u32) as i32;
                let mut relocated = payload.to_vec();
                self.relocate_export_lazy_arrays(i, &mut relocated, new_serial_offset as usize);
                entry.serial_offset = new_serial_offset;
                entry.serial_size = relocated.len() as i32;
                export_data_buf.extend_from_slice(&relocated);
            } else {
                entry.serial_offset = 0;
                entry.serial_size = 0;
            }
        }

        // 5. Write Export Table after export payloads
        summary.export_offset = export_data_start + export_data_buf.len() as u32;
        summary.export_count = updated_exports.entries.len() as u32;
        let mut exports_buf = Vec::new();
        updated_exports.write(&mut exports_buf)?;

        // 6. Update generations in summary if needed
        if summary.file_version >= 68 {
            if summary.generations.is_empty() {
                summary.generations.push(GenerationInfo {
                    export_count: summary.export_count,
                    name_count: summary.name_count,
                });
            } else {
                summary.generations[0].export_count = summary.export_count;
                summary.generations[0].name_count = summary.name_count;
            }
        }

        // 7. Write final header with exact offsets
        let mut final_header = Vec::new();
        summary.write(&mut final_header)?;
        if final_header.len() != header_len as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Header length changed unexpectedly during package serialization",
            ));
        }

        let mut out = Vec::with_capacity(
            final_header.len()
                + names_buf.len()
                + imports_buf.len()
                + export_data_buf.len()
                + exports_buf.len(),
        );
        out.extend_from_slice(&final_header);
        out.extend_from_slice(&names_buf);
        out.extend_from_slice(&imports_buf);
        out.extend_from_slice(&export_data_buf);
        out.extend_from_slice(&exports_buf);

        Ok(out)
    }

    /// Relocates `TLazyArray::SkipPos` absolute file offsets inside `Texture`, `Sound`, and `Music`
    /// export payloads when serialized at `new_serial_offset`.
    fn relocate_export_lazy_arrays(
        &self,
        export_index: usize,
        payload: &mut [u8],
        new_serial_offset: usize,
    ) {
        if self.summary.file_version < 62 || payload.len() < 6 {
            return;
        }
        let Some(exp) = self.exports.entries.get(export_index) else {
            return;
        };
        let cls = self.get_export_class_name(exp);
        if cls.eq_ignore_ascii_case("Texture") {
            let mut cur = Cursor::new(&payload[..]);
            if crate::unr::read_tagged_properties(&mut cur, self).is_ok() {
                let mut pos = cur.position() as usize;
                if pos < payload.len() {
                    let mip_count = payload[pos] as usize;
                    pos += 1;
                    for _ in 0..mip_count {
                        if pos + 5 > payload.len() {
                            break;
                        }
                        let skip_pos_offset = pos;
                        pos += 4;
                        let mut ci_cur = Cursor::new(&payload[pos..]);
                        let Ok(arr_len) = read_compact_index(&mut ci_cur) else {
                            break;
                        };
                        if arr_len < 0 {
                            break;
                        }
                        let ci_len = ci_cur.position() as usize;
                        let data_end = pos + ci_len + (arr_len as usize);
                        if data_end + 10 > payload.len() {
                            break;
                        }
                        let abs_skip_pos = (new_serial_offset + data_end) as i32;
                        payload[skip_pos_offset..skip_pos_offset + 4]
                            .copy_from_slice(&abs_skip_pos.to_le_bytes());
                        pos = data_end + 10;
                    }
                }
            }
        } else if cls.eq_ignore_ascii_case("Sound") || cls.eq_ignore_ascii_case("Music") {
            let mut cur = Cursor::new(&payload[..]);
            if crate::unr::read_tagged_properties(&mut cur, self).is_ok() {
                if read_compact_index(&mut cur).is_ok() {
                    let pos = cur.position() as usize;
                    if pos + 5 <= payload.len() {
                        let skip_pos_offset = pos;
                        let mut ci_cur = Cursor::new(&payload[(pos + 4)..]);
                        if let Ok(arr_len) = read_compact_index(&mut ci_cur) {
                            if arr_len >= 0 {
                                let ci_len = ci_cur.position() as usize;
                                let data_end = pos + 4 + ci_len + (arr_len as usize);
                                if data_end <= payload.len() {
                                    let abs_skip_pos = (new_serial_offset + data_end) as i32;
                                    payload[skip_pos_offset..skip_pos_offset + 4]
                                        .copy_from_slice(&abs_skip_pos.to_le_bytes());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let dest = path.as_ref();
        let data = self.to_bytes()?;
        let mut tmp_os = dest.as_os_str().to_os_string();
        tmp_os.push(".tmp");
        let tmp_path = std::path::PathBuf::from(tmp_os);
        {
            use std::io::Write;
            let mut file = File::create(&tmp_path)?;
            file.write_all(&data)?;
            file.sync_all()?;
        }
        if dest.exists() {
            let mut bak_os = dest.as_os_str().to_os_string();
            bak_os.push(".bak");
            let bak_path = std::path::PathBuf::from(bak_os);
            let _ = std::fs::copy(dest, &bak_path);
        }
        std::fs::rename(&tmp_path, dest)
    }

    /// Adds an export with payload bytes, returning the 1-based export reference (positive integer).
    pub fn add_export(
        &mut self,
        object_name: i32,
        class_index: i32,
        super_index: i32,
        package_index: i32,
        object_flags: u32,
        payload: Vec<u8>,
    ) -> i32 {
        let serial_offset = self.raw_data.len() as i32;
        let serial_size = payload.len() as i32;
        self.raw_data.extend_from_slice(&payload);
        self.exports.entries.push(ObjectExport {
            class_index,
            super_index,
            package_index,
            object_name,
            object_flags,
            serial_size,
            serial_offset,
        });
        self.exports.entries.len() as i32
    }

    /// Updates the binary payload of an existing 1-based export reference.
    pub fn set_export_payload(&mut self, export_ref: i32, payload: Vec<u8>) {
        if export_ref > 0 {
            if let Some(entry) = self.exports.entries.get_mut((export_ref - 1) as usize) {
                entry.serial_offset = self.raw_data.len() as i32;
                entry.serial_size = payload.len() as i32;
                self.raw_data.extend_from_slice(&payload);
            }
        }
    }

    /// Adds an import (or returns the existing one if already registered), returning the negative 1-based import reference (< 0).
    pub fn add_import(
        &mut self,
        class_package: i32,
        class_name: i32,
        package_index: i32,
        object_name: i32,
    ) -> i32 {
        if let Some(idx) = self.imports.entries.iter().position(|imp| {
            imp.class_package == class_package
                && imp.class_name == class_name
                && imp.package_index == package_index
                && imp.object_name == object_name
        }) {
            return -((idx as i32) + 1);
        }
        self.imports.entries.push(ObjectImport {
            class_package,
            class_name,
            package_index,
            object_name,
        });
        -(self.imports.entries.len() as i32)
    }

    /// Resolves a name string from the name table.
    pub fn get_name(&self, index: usize) -> Option<&str> {
        self.names.get(index)
    }

    /// Returns the raw binary payload of an export by its 0-based export table index.
    pub fn get_export_bytes(&self, export_index: usize) -> Option<&[u8]> {
        let export = self.exports.entries.get(export_index)?;
        if export.serial_size <= 0 {
            return Some(&[]);
        }
        let start = export.serial_offset as usize;
        let end = start + export.serial_size as usize;
        if end <= self.raw_data.len() {
            Some(&self.raw_data[start..end])
        } else {
            None
        }
    }

    /// Resolves an object reference (where > 0 is Export, < 0 is Import, 0 is None).
    pub fn resolve_object_reference(&self, ref_index: i32) -> String {
        if ref_index == 0 {
            "None".to_string()
        } else if ref_index > 0 {
            let exp_idx = (ref_index - 1) as usize;
            if let Some(exp) = self.exports.entries.get(exp_idx) {
                self.names.get(exp.object_name as usize).unwrap_or("?").to_string()
            } else {
                format!("Export({exp_idx})")
            }
        } else {
            let imp_idx = (-ref_index - 1) as usize;
            if let Some(imp) = self.imports.entries.get(imp_idx) {
                let obj = self.names.get(imp.object_name as usize).unwrap_or("?");
                let cls = self.names.get(imp.class_name as usize).unwrap_or("?");
                format!("{cls}'{obj}'")
            } else {
                format!("Import({imp_idx})")
            }
        }
    }

    /// Resolves the dotted hierarchy path (`Package.Group.Object` or `MyLevel.Object`) for an import or export reference.
    pub fn resolve_object_hierarchy_name(&self, ref_index: i32) -> String {
        if ref_index == 0 {
            return "None".to_string();
        }
        let mut parts = Vec::new();
        let mut cur = ref_index;
        let mut depth = 0;
        while cur != 0 && depth < 8 {
            depth += 1;
            if cur > 0 {
                let exp_idx = (cur - 1) as usize;
                if let Some(exp) = self.exports.entries.get(exp_idx) {
                    let name = self.names.get(exp.object_name.max(0) as usize).unwrap_or("?");
                    parts.push(name.to_string());
                    cur = exp.package_index;
                } else {
                    break;
                }
            } else {
                let imp_idx = (-cur - 1) as usize;
                if let Some(imp) = self.imports.entries.get(imp_idx) {
                    let name = self.names.get(imp.object_name.max(0) as usize).unwrap_or("?");
                    parts.push(name.to_string());
                    cur = imp.package_index;
                } else {
                    break;
                }
            }
        }
        if parts.is_empty() {
            return "None".to_string();
        }
        parts.reverse();
        parts.join(".")
    }

    /// Resolves a full Unreal Engine 1 object path (`Class'Package.Group.Object'`) for an import or export reference.
    pub fn resolve_full_object_path(&self, ref_index: i32) -> String {
        if ref_index == 0 {
            return "None".to_string();
        }
        let hierarchy = self.resolve_object_hierarchy_name(ref_index);
        if ref_index > 0 {
            let exp_idx = (ref_index - 1) as usize;
            if let Some(exp) = self.exports.entries.get(exp_idx) {
                let cls = self.get_export_class_name(exp);
                if hierarchy.contains('.') {
                    return format!("{cls}'{hierarchy}'");
                } else {
                    return format!("{cls}'MyLevel.{hierarchy}'");
                }
            }
            hierarchy
        } else {
            let imp_idx = (-ref_index - 1) as usize;
            if let Some(imp) = self.imports.entries.get(imp_idx) {
                let cls = self.names.get(imp.class_name.max(0) as usize).unwrap_or("Object");
                format!("{cls}'{hierarchy}'")
            } else {
                hierarchy
            }
        }
    }

    pub fn get_export_object_name(&self, export: &ObjectExport) -> String {
        self.names.get(export.object_name as usize).unwrap_or("?").to_string()
    }

    pub fn get_export_class_name(&self, export: &ObjectExport) -> String {
        if export.class_index == 0 {
            "Class".to_string()
        } else {
            let s = self.resolve_object_reference(export.class_index);
            // If format is Class'Foo', extract Foo
            if let Some(start) = s.find('\'') {
                if let Some(end) = s.rfind('\'') {
                    if end > start {
                        return s[start + 1..end].to_string();
                    }
                }
            }
            s
        }
    }

    pub fn get_export_super_name(&self, export: &ObjectExport) -> String {
        let s = self.resolve_object_reference(export.super_index);
        if let Some(start) = s.find('\'') {
            if let Some(end) = s.rfind('\'') {
                if end > start {
                    return s[start + 1..end].to_string();
                }
            }
        }
        s
    }
}
