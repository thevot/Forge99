use ut99_bsp::unrealscript_exec::{
    generate_scaffold_class, parse_exec_directives_from_uc, parse_exec_line, ExecDirective,
    UcArchetype, UcScaffoldConfig,
};

#[test]
fn test_all_exec_directives_parsing_and_roundtrip() {
    let uc_script = r#"
//=============================================================================
// TestMinigun.uc
//=============================================================================
class TestMinigun extends TournamentWeapon;

#exec MESH IMPORT MESH=MinigunM ANIVFILE=MODELS\minigun_a.3d DATAFILE=MODELS\minigun_d.3d X=0 Y=0 Z=0
#exec MESH ORIGIN MESH=MinigunM X=0 Y=0 Z=0 YAW=64 PITCH=0 ROLL=0
#exec MESH SEQUENCE MESH=MinigunM SEQ=All STARTFRAME=0 NUMFRAMES=45
#exec MESH SEQUENCE MESH=MinigunM SEQ=Fire STARTFRAME=10 NUMFRAMES=12 RATE=30.0
#exec MESHMAP SCALE MESHMAP=MinigunM X=0.06 Y=0.06 Z=0.12
#exec MESHMAP SETTEXTURE MESHMAP=MinigunM NUM=1 TEXTURE=MiniSkin
#exec AUDIO IMPORT FILE="SOUNDS\MiniFire.wav" NAME="MiniFireSnd" GROUP="Weapons"
#exec TEXTURE IMPORT NAME="MiniSkin" FILE="TEXTURES\MiniSkin.pcx" GROUP="Skins" FLAGS=2
#exec OBJ LOAD FILE="..\Textures\Botpack.utx" PACKAGE=Botpack

function Fire(float Value)
{
    Super.Fire(Value);
}
"#;

    let dirs = parse_exec_directives_from_uc(uc_script);
    assert_eq!(dirs.len(), 9);

    // 1. Mesh Import
    match &dirs[0] {
        ExecDirective::MeshImport { mesh, aniv_file, data_file, .. } => {
            assert_eq!(mesh, "MinigunM");
            assert_eq!(aniv_file, r"MODELS\minigun_a.3d");
            assert_eq!(data_file, r"MODELS\minigun_d.3d");
        }
        _ => panic!("Expected MeshImport"),
    }

    // 2. Mesh Origin
    match &dirs[1] {
        ExecDirective::MeshOrigin { mesh, yaw, .. } => {
            assert_eq!(mesh, "MinigunM");
            assert_eq!(*yaw, 64);
        }
        _ => panic!("Expected MeshOrigin"),
    }

    // 3. Audio Import
    match &dirs[6] {
        ExecDirective::AudioImport { name, group, .. } => {
            assert_eq!(name, "MiniFireSnd");
            assert_eq!(group, "Weapons");
        }
        _ => panic!("Expected AudioImport"),
    }

    // 4. Texture Import
    match &dirs[7] {
        ExecDirective::TextureImport { name, group, flags, .. } => {
            assert_eq!(name, "MiniSkin");
            assert_eq!(group, "Skins");
            assert_eq!(*flags, 2);
        }
        _ => panic!("Expected TextureImport"),
    }

    // 5. Obj Load
    match &dirs[8] {
        ExecDirective::ObjLoad { package, .. } => {
            assert_eq!(package, "Botpack");
        }
        _ => panic!("Expected ObjLoad"),
    }
}

#[test]
fn test_archetypes_code_scaffolding() {
    // 1. Mutator
    let mut mut_cfg = UcScaffoldConfig::default();
    mut_cfg.class_name = "InstaGibArena".to_string();
    mut_cfg.archetype = UcArchetype::Mutator;
    mut_cfg.super_class = "Mutator".to_string();
    let mut_code = generate_scaffold_class(&mut_cfg);
    assert!(mut_code.contains("class InstaGibArena extends Mutator;"));
    assert!(mut_code.contains("function PostBeginPlay()"));
    assert!(mut_code.contains("function bool CheckReplacement"));
    assert!(mut_code.contains("function MutatorTakeDamage"));
    assert!(mut_code.contains("bAlwaysKeep=True"));

    // 2. Projectile
    let mut proj_cfg = UcScaffoldConfig::default();
    proj_cfg.class_name = "PlasmaOrb".to_string();
    proj_cfg.archetype = UcArchetype::Projectile;
    proj_cfg.super_class = "Projectile".to_string();
    proj_cfg.proj_speed = 1200.0;
    proj_cfg.proj_damage = 95.0;
    let proj_code = generate_scaffold_class(&proj_cfg);
    assert!(proj_code.contains("class PlasmaOrb extends Projectile;"));
    assert!(proj_code.contains("simulated function Explode"));
    assert!(proj_code.contains("Speed=1200.0"));
    assert!(proj_code.contains("Damage=95.0"));
    assert!(proj_code.contains("Physics=PHYS_Projectile"));

    // 3. Pickup
    let mut pickup_cfg = UcScaffoldConfig::default();
    pickup_cfg.class_name = "SuperDamageAmp".to_string();
    pickup_cfg.archetype = UcArchetype::Pickup;
    pickup_cfg.super_class = "TournamentPickup".to_string();
    let pickup_code = generate_scaffold_class(&pickup_cfg);
    assert!(pickup_code.contains("class SuperDamageAmp extends TournamentPickup;"));
    assert!(pickup_code.contains("auto state Pickup"));
    assert!(pickup_code.contains("RespawnTime=30.000000"));
}

#[test]
fn test_custom_exec_directive_parser() {
    let raw_line = "#exec MESHMAP SETTEXTURE MESHMAP=Gun NUM=0 TEXTURE=GunTex";
    let dir = parse_exec_line(raw_line).expect("Failed to parse");
    match dir {
        ExecDirective::MeshMapSetTexture { mesh_map, num, texture } => {
            assert_eq!(mesh_map, "Gun");
            assert_eq!(num, 0);
            assert_eq!(texture, "GunTex");
        }
        _ => panic!("Expected MeshMapSetTexture"),
    }
}
