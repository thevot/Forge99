use std::io::Cursor;
use ut99_format::{read_tagged_properties, Package};

#[test]
fn test_inspect_indus1_utx() {
    let mut current = std::env::current_dir().unwrap();
    let mut utx_path = None;
    for _ in 0..4 {
        let p = current.join("UnrealTournament/Textures/Indus1.utx");
        if p.exists() {
            utx_path = Some(p);
            break;
        }
        if !current.pop() {
            break;
        }
    }
    let utx_path = match utx_path {
        Some(p) => p,
        None => {
            println!("Indus1.utx not found, skipping.");
            return;
        }
    };
    let pkg = Package::open(&utx_path).expect("Failed to open Indus1.utx");
    println!(
        "Indus1.utx: {} names, {} exports, {} imports",
        pkg.names.entries.len(),
        pkg.exports.entries.len(),
        pkg.imports.entries.len()
    );

    for (i, exp) in pkg.exports.entries.iter().enumerate() {
        let name = pkg.get_name(exp.object_name.max(0) as usize).unwrap_or("?");
        let class_name = if exp.class_index < 0 {
            let imp = &pkg.imports.entries[(-exp.class_index - 1) as usize];
            pkg.get_name(imp.object_name.max(0) as usize).unwrap_or("?")
        } else {
            "Field"
        };
        if class_name == "Texture" {
            println!("Texture #{}: {} (offset: {}, size: {})", i, name, exp.serial_offset, exp.serial_size);
            let start = exp.serial_offset as usize;
            let end = start + exp.serial_size as usize;
            let data = &pkg.raw_data[start..end];
            let mut cursor = Cursor::new(data);
            if let Ok(props) = read_tagged_properties(&mut cursor, &pkg) {
                let pos = cursor.position() as usize;
                println!("  Tagged props (count: {}):", props.len());
                for (k, v) in &props {
                    println!("    {} = {:?}", k, v.value);
                }
                let remaining = data.len() - pos;
                let preview = &data[pos..(pos + remaining.min(64))];
                println!("  Remaining bytes after props: {} bytes, preview: {:02X?}", remaining, preview);

                let _num_mips = data[pos] as usize;
                let mip0_len = 256 * 256;
                let mip0_bytes = &data[(pos + 1)..(pos + 1 + mip0_len)];
                println!("  Mip 0 length: {} bytes (starts at pos + 1)", mip0_bytes.len());
                let mip1_preview = &data[(pos + 1 + mip0_len)..(pos + 1 + mip0_len + 16)];
                println!("  Mip 1 preview (at pos + 1 + 65536): {:02X?}", mip1_preview);

                // Inspect Palette export #12
                let pal_exp = &pkg.exports.entries[12];
                let pal_start = pal_exp.serial_offset as usize;
                let pal_data = &pkg.raw_data[pal_start..(pal_start + pal_exp.serial_size as usize)];
                // Palette has 3 prefix bytes [04, 40, 04] followed by 256 RGBA colors (1024 bytes)
                let colors_bytes = &pal_data[3..1027];
                println!("  Extracted 256 palette colors ({} bytes)!", colors_bytes.len());
                // Color 0:
                println!("  Palette[0] = RGBA({}, {}, {}, {})", colors_bytes[0], colors_bytes[1], colors_bytes[2], colors_bytes[3]);
                // Sample pixel 0:
                let px0_idx = data[pos + 1] as usize;
                let px0_color = &colors_bytes[(px0_idx * 4)..(px0_idx * 4 + 4)];
                println!("  Pixel[0] index = 0x{:02X} -> RGBA({}, {}, {}, {})", px0_idx, px0_color[0], px0_color[1], px0_color[2], px0_color[3]);
            }
            break;
        }
    }
}

#[test]
fn test_utx_package_loader() {
    let mut current = std::env::current_dir().unwrap();
    let mut utx_path = None;
    for _ in 0..4 {
        let p = current.join("UnrealTournament/Textures/Indus1.utx");
        if p.exists() {
            utx_path = Some(p);
            break;
        }
        if !current.pop() {
            break;
        }
    }
    let utx_path = match utx_path {
        Some(p) => p,
        None => return,
    };

    let utx = ut99_format::UtxPackage::open(&utx_path).expect("Failed to parse UtxPackage");
    println!("Parsed UtxPackage '{}': {} textures loaded!", utx.package_name, utx.textures.len());
    assert!(!utx.textures.is_empty());

    for tex in utx.textures.iter().take(5) {
        println!("  Texture: {} ({}x{}) - {} RGBA bytes", tex.name, tex.width, tex.height, tex.rgba_pixels.len());
        assert_eq!(tex.rgba_pixels.len(), (tex.width * tex.height * 4) as usize);
    }
}

#[test]
fn test_p0_task01_tlazyarray_skip_pos_palette_none_and_heritage_header() {
    use byteorder::{LittleEndian, ReadBytesExt};
    use ut99_format::package::compact_index::{read_compact_index, write_compact_index};
    use ut99_format::utx::{UtxPackage, UtxTexture};

    // 1. CompactIndex i32::MIN must not panic or overflow
    let mut ci_buf = Vec::new();
    write_compact_index(&mut ci_buf, i32::MIN).expect("write i32::MIN");
    let decoded_min = read_compact_index(&mut Cursor::new(&ci_buf)).expect("read i32::MIN");
    assert_eq!(decoded_min, i32::MIN);

    // 2. NameTable case-insensitive deduplication
    let mut pkg = Package::default();
    let idx_first = pkg.names.add("CustomName", 0);
    let idx_none = pkg.names.add("none", 0); // intentionally at index 1 (non-zero!) and lowercase
    let idx_same = pkg.names.add("CUSTOMNAME", 0);
    assert_eq!(idx_first, idx_same, "NameTable::add must be case-insensitive");
    assert_eq!(idx_none, 1, "None is at index 1 in this test package");

    // 3. Write a 4x4 texture into package where "None" is at index 1, serialize to bytes, and verify SkipPos & round-trip
    let tex = UtxTexture {
        name: "WallTex".to_string(),
        group: "Base".to_string(),
        width: 4,
        height: 4,
        format: 0,
        masked: false,
        rgba_pixels: vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ],
    };
    UtxPackage::write_textures_into_package(&[tex], &mut pkg, 0).expect("write_textures_into_package");
    let raw_file_bytes = pkg.to_bytes().expect("to_bytes");
    let pkg2 = Package::from_bytes(raw_file_bytes.clone()).expect("from_bytes");

    // Inspect the serialized Texture export in pkg2 and verify TLazyArray::SkipPos equals absolute file offset after mip pixels
    let tex_exp_idx = pkg2
        .exports
        .entries
        .iter()
        .position(|e| pkg2.get_export_class_name(e).eq_ignore_ascii_case("Texture"))
        .expect("Texture export present");
    let tex_exp = &pkg2.exports.entries[tex_exp_idx];
    let tex_payload = pkg2.get_export_bytes(tex_exp_idx).unwrap();
    let mut cur = Cursor::new(tex_payload);
    let props = read_tagged_properties(&mut cur, &pkg2).expect("read Texture tagged properties");
    assert!(props.contains_key("UBits"));
    assert!(props.contains_key("VBits"));
    assert!(props.contains_key("Format"));
    let mip_count = read_compact_index(&mut cur).expect("mip count");
    assert!(mip_count >= 1);
    for _ in 0..mip_count {
        let skip_pos = cur.read_i32::<LittleEndian>().expect("SkipPos");
        let data_count = read_compact_index(&mut cur).expect("DataCount");
        let data_end_rel = cur.position() as i32 + data_count;
        assert_eq!(
            skip_pos,
            tex_exp.serial_offset + data_end_rel,
            "TLazyArray::SkipPos must equal absolute file offset immediately after mip pixel data"
        );
        cur.set_position((data_end_rel + 10) as u64);
    }

    // Verify UtxPackage::from_package decodes the palette & texture cleanly even when "None" is at NameTable index 1
    let decoded_utx = UtxPackage::from_package(pkg2, "TestPkg".to_string()).expect("decode UtxPackage");
    assert_eq!(decoded_utx.textures.len(), 1);
    assert_eq!(decoded_utx.textures[0].width, 4);
    assert_eq!(decoded_utx.textures[0].height, 4);

    // 4. Verify heritage header preservation on v64 Indus1.utx if present
    let mut current = std::env::current_dir().unwrap();
    for _ in 0..4 {
        let p = current.join("UnrealTournament/Textures/Indus1.utx");
        if p.exists() {
            let indus = Package::open(&p).expect("open Indus1.utx");
            assert_eq!(indus.summary.file_version, 64);
            let orig_heritage = &indus.summary.guid[0..8];
            assert_ne!(orig_heritage, &[0u8; 8]);
            let indus_bytes = indus.to_bytes().expect("Indus1 to_bytes");
            let indus2 = Package::from_bytes(indus_bytes).expect("Indus1 from_bytes");
            assert_eq!(indus2.summary.file_version, 64);
            assert_eq!(&indus2.summary.guid[0..8], orig_heritage);
            break;
        }
        if !current.pop() {
            break;
        }
    }
}





#[test]
fn test_evo09_pure_rust_dxt1_dxt5_compression_and_mipmaps() {
    use ut99_format::{
        decode_dxt1_to_rgba8, decode_dxt5_to_rgba8, encode_rgba8_to_dxt1, encode_rgba8_to_dxt5,
        generate_dxt1_mip_chain, generate_dxt5_mip_chain, generate_rgba8_mip_chain,
    };

    // 1. Create a 16x16 RGBA test image with a color gradient
    let w = 16u32;
    let h = 16u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            rgba[idx] = (x * 16) as u8;       // Red gradient
            rgba[idx + 1] = (y * 16) as u8;   // Green gradient
            rgba[idx + 2] = 128;              // Blue constant
            rgba[idx + 3] = 255;              // Opaque
        }
    }

    // 2. Compress to DXT1 and verify byte length: (16/4) * (16/4) * 8 = 4 * 4 * 8 = 128 bytes
    let dxt1 = encode_rgba8_to_dxt1(&rgba, w, h, false);
    assert_eq!(dxt1.len(), 128);

    // Decode back and check fidelity
    let decoded_dxt1 = decode_dxt1_to_rgba8(&dxt1, w, h).expect("Decode DXT1");
    assert_eq!(decoded_dxt1.len(), rgba.len());

    // Check maximum error per channel is bounded
    let mut max_err_r = 0i32;
    let mut max_err_g = 0i32;
    for i in 0..(w * h) as usize {
        let orig_r = rgba[i * 4] as i32;
        let orig_g = rgba[i * 4 + 1] as i32;
        let dec_r = decoded_dxt1[i * 4] as i32;
        let dec_g = decoded_dxt1[i * 4 + 1] as i32;
        max_err_r = max_err_r.max((orig_r - dec_r).abs());
        max_err_g = max_err_g.max((orig_g - dec_g).abs());
    }
    assert!(max_err_r <= 40, "DXT1 Red max error {} exceeds tolerance", max_err_r);
    assert!(max_err_g <= 40, "DXT1 Green max error {} exceeds tolerance", max_err_g);

    // 3. Test DXT1 Punch-Through 1-bit Alpha
    let mut rgba_masked = rgba.clone();
    rgba_masked[0..4].copy_from_slice(&[0, 0, 0, 0]); // transparent pixel at (0, 0)
    let dxt1_masked = encode_rgba8_to_dxt1(&rgba_masked, w, h, true);
    let decoded_masked = decode_dxt1_to_rgba8(&dxt1_masked, w, h).expect("Decode masked DXT1");
    assert_eq!(decoded_masked[3], 0, "Pixel (0,0) alpha must be transparent");
    assert_eq!(decoded_masked[7], 255, "Pixel (1,0) alpha must remain opaque");

    // 4. Test DXT5 Smooth Alpha Compression: 16x16 with vertical alpha ramp 0..255
    let mut rgba_dxt5 = rgba.clone();
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            rgba_dxt5[idx + 3] = ((y * 255) / (h - 1)) as u8;
        }
    }
    let dxt5 = encode_rgba8_to_dxt5(&rgba_dxt5, w, h);
    // (16/4) * (16/4) * 16 = 256 bytes
    assert_eq!(dxt5.len(), 256);

    let decoded_dxt5 = decode_dxt5_to_rgba8(&dxt5, w, h).expect("Decode DXT5");
    assert_eq!(decoded_dxt5.len(), rgba_dxt5.len());
    let mut max_err_a = 0i32;
    for i in 0..(w * h) as usize {
        let orig_a = rgba_dxt5[i * 4 + 3] as i32;
        let dec_a = decoded_dxt5[i * 4 + 3] as i32;
        max_err_a = max_err_a.max((orig_a - dec_a).abs());
    }
    assert!(max_err_a <= 8, "DXT5 Alpha max error {} exceeds tolerance", max_err_a);

    // 5. Test Live Mipmap Pyramid Generation: 32x32 -> 6 levels (32, 16, 8, 4, 2, 1)
    let rgba_32 = vec![180u8; 32 * 32 * 4];
    let rgba_chain = generate_rgba8_mip_chain(&rgba_32, 32, 32);
    assert_eq!(rgba_chain.len(), 6);
    assert_eq!(rgba_chain[0].0, 32);
    assert_eq!(rgba_chain[1].0, 16);
    assert_eq!(rgba_chain[2].0, 8);
    assert_eq!(rgba_chain[3].0, 4);
    assert_eq!(rgba_chain[4].0, 2);
    assert_eq!(rgba_chain[5].0, 1);

    // DXT1 Mip chain
    let dxt1_chain = generate_dxt1_mip_chain(&rgba_32, 32, 32, false);
    assert_eq!(dxt1_chain.len(), 6);
    assert_eq!(dxt1_chain[0].2.len(), 512);
    assert_eq!(dxt1_chain[1].2.len(), 128);
    assert_eq!(dxt1_chain[2].2.len(), 32);
    assert_eq!(dxt1_chain[3].2.len(), 8);
    assert_eq!(dxt1_chain[4].2.len(), 8);
    assert_eq!(dxt1_chain[5].2.len(), 8);

    // DXT5 Mip chain
    let dxt5_chain = generate_dxt5_mip_chain(&rgba_32, 32, 32);
    assert_eq!(dxt5_chain.len(), 6);
    assert_eq!(dxt5_chain[0].2.len(), 1024);
    assert_eq!(dxt5_chain[5].2.len(), 16);
}
