use std::fs;
use std::path::PathBuf;
use ut99_bsp::asset_extractor::{
    batch_export_directory, extract_assets_from_package, AssetCategory, BatchExportOptions,
};
use ut99_format::package::{compress_uz, decompress_uz, is_uz_file};
use ut99_format::utx::UtxTexture;
use ut99_format::{Package, UtxPackage};

#[test]
fn test_uz_decompressor_integration() {
    let original_name = "CTF-Face.unr";
    let original_bytes = b"\xC1\x83\x2A\x9E\x44\x00\x00\x00Facing Worlds UT99 Level Map Data Binary Chunk";

    // 1. Compress into .uz
    let uz_data = compress_uz(original_name, original_bytes).expect("compress_uz failed");
    assert!(is_uz_file(&uz_data));
    assert_eq!(&uz_data[0..4], &1234u32.to_le_bytes());

    // 2. Decompress from .uz
    let mut cursor = std::io::Cursor::new(&uz_data);
    let (decomp_name, decomp_bytes) = decompress_uz(&mut cursor).expect("decompress_uz failed");
    assert_eq!(decomp_name, original_name);
    assert_eq!(decomp_bytes, original_bytes);
}

#[test]
fn test_package_texture_and_mesh_extraction() {
    let mut pkg = Package::default();
    pkg.summary.file_version = 68;

    let test_tex = UtxTexture {
        name: "MetalPlate".to_string(),
        group: "Industrial".to_string(),
        width: 32,
        height: 32,
        format: 0, // P8
        masked: false,
        rgba_pixels: vec![128u8; 32 * 32 * 4],
    };

    UtxPackage::write_textures_into_package(&[test_tex], &mut pkg, 0).unwrap();
    let pkg_bytes = pkg.to_bytes().unwrap();
    let loaded = Package::from_bytes(pkg_bytes).unwrap();

    let assets = extract_assets_from_package(
        &loaded,
        "TestBase",
        &[AssetCategory::Texture, AssetCategory::Mesh],
        true,
    );

    assert!(!assets.is_empty());
    let tex_asset = assets.iter().find(|a| a.name == "MetalPlate").expect("MetalPlate not found");
    assert_eq!(tex_asset.category, AssetCategory::Texture);
    assert_eq!(tex_asset.file_extension, "png");
    assert_eq!(&tex_asset.data[0..4], b"\x89PNG");

    let rel_path = tex_asset.make_relative_path(true, true);
    assert_eq!(
        rel_path,
        PathBuf::from("TestBase/Textures/Industrial/MetalPlate.png")
    );
}

#[test]
fn test_batch_export_directory_simulation() {
    let temp_dir = std::env::temp_dir().join("ut99_batch_export_test");
    let input_dir = temp_dir.join("Input");
    let output_dir = temp_dir.join("Output");

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // 1. Create a dummy package with texture
    let mut pkg = Package::default();
    let tex = UtxTexture {
        name: "FloorTile".to_string(),
        group: "Decor".to_string(),
        width: 16,
        height: 16,
        format: 0,
        masked: false,
        rgba_pixels: vec![255u8; 16 * 16 * 4],
    };
    UtxPackage::write_textures_into_package(&[tex], &mut pkg, 0).unwrap();
    let pkg_bytes = pkg.to_bytes().unwrap();

    let raw_package_path = input_dir.join("MyTextures.utx");
    fs::write(&raw_package_path, &pkg_bytes).unwrap();

    // 2. Create a .uz compressed package
    let uz_package_path = input_dir.join("MyRedirect.utx.uz");
    let uz_bytes = compress_uz("MyRedirect.utx", &pkg_bytes).unwrap();
    fs::write(&uz_package_path, &uz_bytes).unwrap();

    // 3. Run batch export
    let options = BatchExportOptions {
        output_dir: output_dir.clone(),
        categories: vec![AssetCategory::Texture],
        subfolder_by_package: true,
        subfolder_by_category: true,
        export_psk_for_meshes: false,
        overwrite: true,
    };

    let report = batch_export_directory(&input_dir, &options).expect("Batch export failed");
    assert_eq!(report.total_packages, 2);
    assert!(report.textures_extracted >= 2);
    assert!(report.total_bytes_written > 0);

    // Verify written files exist on disk
    let extracted_tex1 = output_dir.join("MyTextures/Textures/Decor/FloorTile.png");
    let extracted_tex2 = output_dir.join("MyRedirect/Textures/Decor/FloorTile.png");
    assert!(extracted_tex1.exists(), "FloorTile from raw package must exist");
    assert!(extracted_tex2.exists(), "FloorTile from .uz package must exist");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
