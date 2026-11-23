//! UnrealScript `#exec` Directive Compiler & Class Template Scaffolding
//!
//! Provides parsing, validation, and generation for compile-time `#exec` macros
//! used by `UCC make` to embed vertex meshes, animations, textures, and sounds directly
//! into compiled `.u` packages.
//!
//! Based on forensic analysis of Epic Games `UScript Tutorial` and UT99 stock packages.

use std::collections::HashMap;

/// An individual `#exec` directive parsed from or generated for an UnrealScript file.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecDirective {
    /// `#exec MESH IMPORT MESH=MyMesh ANIVFILE=MODELS\MyMesh_a.3d DATAFILE=MODELS\MyMesh_d.3d X=0 Y=0 Z=0`
    MeshImport {
        mesh: String,
        aniv_file: String,
        data_file: String,
        x: f32,
        y: f32,
        z: f32,
    },
    /// `#exec MESH ORIGIN MESH=MyMesh X=0 Y=0 Z=0 YAW=0 PITCH=0 ROLL=0`
    MeshOrigin {
        mesh: String,
        x: f32,
        y: f32,
        z: f32,
        yaw: i32,
        pitch: i32,
        roll: i32,
    },
    /// `#exec MESH SEQUENCE MESH=MyMesh SEQ=Fire STARTFRAME=0 NUMFRAMES=10 RATE=24.0`
    MeshSequence {
        mesh: String,
        seq: String,
        start_frame: u32,
        num_frames: u32,
        rate: f32,
    },
    /// `#exec MESHMAP SCALE MESHMAP=MyMesh X=0.05 Y=0.05 Z=0.1`
    MeshMapScale {
        mesh_map: String,
        x: f32,
        y: f32,
        z: f32,
    },
    /// `#exec MESHMAP SETTEXTURE MESHMAP=MyMesh NUM=0 TEXTURE=MySkin`
    MeshMapSetTexture {
        mesh_map: String,
        num: u32,
        texture: String,
    },
    /// `#exec AUDIO IMPORT FILE=SOUNDS\Fire.wav NAME=WeaponFire GROUP=Weapons`
    AudioImport {
        file: String,
        name: String,
        group: String,
    },
    /// `#exec TEXTURE IMPORT NAME=Skin FILE=TEXTURES\Skin.pcx GROUP=Skins FLAGS=2`
    TextureImport {
        name: String,
        file: String,
        group: String,
        flags: u32,
        mips: bool,
    },
    /// `#exec OBJ LOAD FILE=..\Textures\Botpack.utx PACKAGE=Botpack`
    ObjLoad {
        file: String,
        package: String,
    },
    /// Raw unparsed or custom directive: `#exec <raw>`
    Raw(String),
}

impl ExecDirective {
    /// Formats the directive as a standard UnrealScript `#exec` statement line.
    pub fn to_directive_string(&self) -> String {
        match self {
            ExecDirective::MeshImport { mesh, aniv_file, data_file, x, y, z } => {
                format!("#exec MESH IMPORT MESH={mesh} ANIVFILE={aniv_file} DATAFILE={data_file} X={x} Y={y} Z={z}")
            }
            ExecDirective::MeshOrigin { mesh, x, y, z, yaw, pitch, roll } => {
                format!("#exec MESH ORIGIN MESH={mesh} X={x} Y={y} Z={z} YAW={yaw} PITCH={pitch} ROLL={roll}")
            }
            ExecDirective::MeshSequence { mesh, seq, start_frame, num_frames, rate } => {
                format!("#exec MESH SEQUENCE MESH={mesh} SEQ={seq} STARTFRAME={start_frame} NUMFRAMES={num_frames} RATE={rate:.1}")
            }
            ExecDirective::MeshMapScale { mesh_map, x, y, z } => {
                format!("#exec MESHMAP SCALE MESHMAP={mesh_map} X={x:.3} Y={y:.3} Z={z:.3}")
            }
            ExecDirective::MeshMapSetTexture { mesh_map, num, texture } => {
                format!("#exec MESHMAP SETTEXTURE MESHMAP={mesh_map} NUM={num} TEXTURE={texture}")
            }
            ExecDirective::AudioImport { file, name, group } => {
                if group.is_empty() || group == "None" {
                    format!("#exec AUDIO IMPORT FILE=\"{file}\" NAME=\"{name}\"")
                } else {
                    format!("#exec AUDIO IMPORT FILE=\"{file}\" NAME=\"{name}\" GROUP=\"{group}\"")
                }
            }
            ExecDirective::TextureImport { name, file, group, flags, mips } => {
                let mips_str = if *mips { "" } else { " MIPS=OFF" };
                if group.is_empty() || group == "None" {
                    format!("#exec TEXTURE IMPORT NAME=\"{name}\" FILE=\"{file}\" FLAGS={flags}{mips_str}")
                } else {
                    format!("#exec TEXTURE IMPORT NAME=\"{name}\" FILE=\"{file}\" GROUP=\"{group}\" FLAGS={flags}{mips_str}")
                }
            }
            ExecDirective::ObjLoad { file, package } => {
                format!("#exec OBJ LOAD FILE=\"{file}\" PACKAGE={package}")
            }
            ExecDirective::Raw(line) => {
                if line.starts_with("#exec ") {
                    line.clone()
                } else {
                    format!("#exec {line}")
                }
            }
        }
    }
}

/// Parses key=value tokens from an `#exec` line payload.
fn parse_tokens(payload: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in payload.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    for token in tokens {
        if let Some((k, v)) = token.split_once('=') {
            map.insert(k.to_uppercase(), v.trim_matches('"').to_string());
        }
    }
    map
}

/// Parses a single line of text, returning `Some(ExecDirective)` if it is an `#exec` command.
pub fn parse_exec_line(line: &str) -> Option<ExecDirective> {
    let trimmed = line.trim();
    if !trimmed.to_lowercase().starts_with("#exec ") {
        return None;
    }

    let payload = trimmed["#exec ".len()..].trim();
    let upper = payload.to_uppercase();

    if upper.starts_with("MESH IMPORT ") {
        let tokens = parse_tokens(&payload["MESH IMPORT ".len()..]);
        return Some(ExecDirective::MeshImport {
            mesh: tokens.get("MESH").cloned().unwrap_or_default(),
            aniv_file: tokens.get("ANIVFILE").cloned().unwrap_or_default(),
            data_file: tokens.get("DATAFILE").cloned().unwrap_or_default(),
            x: tokens.get("X").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            y: tokens.get("Y").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            z: tokens.get("Z").and_then(|s| s.parse().ok()).unwrap_or(0.0),
        });
    }

    if upper.starts_with("MESH ORIGIN ") {
        let tokens = parse_tokens(&payload["MESH ORIGIN ".len()..]);
        return Some(ExecDirective::MeshOrigin {
            mesh: tokens.get("MESH").cloned().unwrap_or_default(),
            x: tokens.get("X").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            y: tokens.get("Y").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            z: tokens.get("Z").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            yaw: tokens.get("YAW").and_then(|s| s.parse().ok()).unwrap_or(0),
            pitch: tokens.get("PITCH").and_then(|s| s.parse().ok()).unwrap_or(0),
            roll: tokens.get("ROLL").and_then(|s| s.parse().ok()).unwrap_or(0),
        });
    }

    if upper.starts_with("MESH SEQUENCE ") {
        let tokens = parse_tokens(&payload["MESH SEQUENCE ".len()..]);
        return Some(ExecDirective::MeshSequence {
            mesh: tokens.get("MESH").cloned().unwrap_or_default(),
            seq: tokens.get("SEQ").cloned().unwrap_or_default(),
            start_frame: tokens.get("STARTFRAME").and_then(|s| s.parse().ok()).unwrap_or(0),
            num_frames: tokens.get("NUMFRAMES").and_then(|s| s.parse().ok()).unwrap_or(1),
            rate: tokens.get("RATE").and_then(|s| s.parse().ok()).unwrap_or(30.0),
        });
    }

    if upper.starts_with("MESHMAP SCALE ") {
        let tokens = parse_tokens(&payload["MESHMAP SCALE ".len()..]);
        return Some(ExecDirective::MeshMapScale {
            mesh_map: tokens.get("MESHMAP").cloned().unwrap_or_default(),
            x: tokens.get("X").and_then(|s| s.parse().ok()).unwrap_or(0.1),
            y: tokens.get("Y").and_then(|s| s.parse().ok()).unwrap_or(0.1),
            z: tokens.get("Z").and_then(|s| s.parse().ok()).unwrap_or(0.1),
        });
    }

    if upper.starts_with("MESHMAP SETTEXTURE ") {
        let tokens = parse_tokens(&payload["MESHMAP SETTEXTURE ".len()..]);
        return Some(ExecDirective::MeshMapSetTexture {
            mesh_map: tokens.get("MESHMAP").cloned().unwrap_or_default(),
            num: tokens.get("NUM").and_then(|s| s.parse().ok()).unwrap_or(0),
            texture: tokens.get("TEXTURE").cloned().unwrap_or_default(),
        });
    }

    if upper.starts_with("AUDIO IMPORT ") {
        let tokens = parse_tokens(&payload["AUDIO IMPORT ".len()..]);
        return Some(ExecDirective::AudioImport {
            file: tokens.get("FILE").cloned().unwrap_or_default(),
            name: tokens.get("NAME").cloned().unwrap_or_default(),
            group: tokens.get("GROUP").cloned().unwrap_or_else(|| "None".to_string()),
        });
    }

    if upper.starts_with("TEXTURE IMPORT ") {
        let tokens = parse_tokens(&payload["TEXTURE IMPORT ".len()..]);
        let mips = tokens.get("MIPS").map(|s| !s.eq_ignore_ascii_case("OFF")).unwrap_or(true);
        return Some(ExecDirective::TextureImport {
            name: tokens.get("NAME").cloned().unwrap_or_default(),
            file: tokens.get("FILE").cloned().unwrap_or_default(),
            group: tokens.get("GROUP").cloned().unwrap_or_else(|| "None".to_string()),
            flags: tokens.get("FLAGS").and_then(|s| s.parse().ok()).unwrap_or(0),
            mips,
        });
    }

    if upper.starts_with("OBJ LOAD ") {
        let tokens = parse_tokens(&payload["OBJ LOAD ".len()..]);
        return Some(ExecDirective::ObjLoad {
            file: tokens.get("FILE").cloned().unwrap_or_default(),
            package: tokens.get("PACKAGE").cloned().unwrap_or_default(),
        });
    }

    Some(ExecDirective::Raw(payload.to_string()))
}

/// Scans a full `.uc` file string and extracts all contained `#exec` directives.
pub fn parse_exec_directives_from_uc(uc_text: &str) -> Vec<ExecDirective> {
    uc_text.lines().filter_map(parse_exec_line).collect()
}

// -------------------------------------------------------------------------------------------------
// UnrealScript Class Wizard Archetypes & Scaffolding
// -------------------------------------------------------------------------------------------------

/// Standard gameplay archetypes supported by the UnrealScript Class Wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UcArchetype {
    Mutator,
    TournamentWeapon,
    Projectile,
    Pickup,
    Custom,
}

impl UcArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            UcArchetype::Mutator => "Gameplay Mutator (Mutator)",
            UcArchetype::TournamentWeapon => "Tournament Weapon (TournamentWeapon)",
            UcArchetype::Projectile => "Custom Projectile (Projectile)",
            UcArchetype::Pickup => "Inventory / Powerup Pickup (TournamentPickup)",
            UcArchetype::Custom => "Custom Class (Object / Actor)",
        }
    }

    pub fn default_super_class(&self) -> &'static str {
        match self {
            UcArchetype::Mutator => "Mutator",
            UcArchetype::TournamentWeapon => "TournamentWeapon",
            UcArchetype::Projectile => "Projectile",
            UcArchetype::Pickup => "TournamentPickup",
            UcArchetype::Custom => "Actor",
        }
    }
}

/// Configuration settings for scaffolding an UnrealScript `.uc` class file.
#[derive(Debug, Clone, PartialEq)]
pub struct UcScaffoldConfig {
    pub package_name: String,
    pub class_name: String,
    pub super_class: String,
    pub archetype: UcArchetype,
    pub description: String,
    pub author: String,

    // Mutator Specifics
    pub mutator_override_damage: bool,
    pub mutator_check_replacement: bool,

    // Weapon Specifics
    pub weapon_primary_damage: i32,
    pub weapon_alt_damage: i32,
    pub weapon_fire_rate: f32,
    pub weapon_alt_fire_rate: f32,
    pub weapon_ammo_name: String,
    pub weapon_pickup_ammo: i32,

    // Projectile Specifics
    pub proj_speed: f32,
    pub proj_max_speed: f32,
    pub proj_damage: f32,
    pub proj_momentum: f32,
    pub proj_damage_radius: f32,

    // Directives
    pub exec_directives: Vec<ExecDirective>,
}

impl Default for UcScaffoldConfig {
    fn default() -> Self {
        Self {
            package_name: "MyPackage".to_string(),
            class_name: "MyCustomWeapon".to_string(),
            super_class: "TournamentWeapon".to_string(),
            archetype: UcArchetype::TournamentWeapon,
            description: "Custom Unreal Tournament 99 gameplay weapon".to_string(),
            author: "Unreal Mapper".to_string(),

            mutator_override_damage: true,
            mutator_check_replacement: true,

            weapon_primary_damage: 40,
            weapon_alt_damage: 90,
            weapon_fire_rate: 1.0,
            weapon_alt_fire_rate: 0.5,
            weapon_ammo_name: "Miniammo".to_string(),
            weapon_pickup_ammo: 50,

            proj_speed: 1000.0,
            proj_max_speed: 2000.0,
            proj_damage: 75.0,
            proj_momentum: 40000.0,
            proj_damage_radius: 180.0,

            exec_directives: Vec::new(),
        }
    }
}

/// Generates valid, compiler-ready `.uc` source code matching `UCC make` compiler grammar.
pub fn generate_scaffold_class(config: &UcScaffoldConfig) -> String {
    let mut out = String::new();

    // 1. Header
    out.push_str("//=============================================================================\n");
    out.push_str(&format!("// {}.uc\n", config.class_name));
    out.push_str(&format!("// Package: {}\n", config.package_name));
    if !config.author.is_empty() {
        out.push_str(&format!("// Author:  {}\n", config.author));
    }
    if !config.description.is_empty() {
        out.push_str(&format!("// Purpose: {}\n", config.description));
    }
    out.push_str("// Generated by Forge99 UnrealScript Studio & UCC Make Pipeline\n");
    out.push_str("//=============================================================================\n");
    out.push_str(&format!("class {} extends {};\n\n", config.class_name, config.super_class));

    // 2. Embedded #exec directives
    if !config.exec_directives.is_empty() {
        out.push_str("// --- Embedded Assets & Animation Directives (#exec) ---\n");
        for dir in &config.exec_directives {
            out.push_str(&dir.to_directive_string());
            out.push('\n');
        }
        out.push('\n');
    }

    // 3. Archetype Code Body
    match config.archetype {
        UcArchetype::Mutator => {
            out.push_str("// Registered HUD Mutator or rules hook\n");
            out.push_str("function PostBeginPlay()\n");
            out.push_str("{\n");
            out.push_str("    Super.PostBeginPlay();\n");
            out.push_str("    Log(\"*** Loaded Mutator: \" $ Class $ \" ***\");\n");
            out.push_str("}\n\n");

            if config.mutator_check_replacement {
                out.push_str("function bool CheckReplacement(Actor Other, out byte bSuperRelevant)\n");
                out.push_str("{\n");
                out.push_str("    // Example: replace standard Enforcers or weapons\n");
                out.push_str("    return true;\n");
                out.push_str("}\n\n");
            }

            if config.mutator_override_damage {
                out.push_str("function MutatorTakeDamage(out int ActualDamage, Pawn Victim, Pawn InstigatedBy, out Vector HitLocation, out Vector Momentum, name DamageType)\n");
                out.push_str("{\n");
                out.push_str("    // Mutator damage modifications\n");
                out.push_str("    if ( NextMutator != None )\n");
                out.push_str("        NextMutator.MutatorTakeDamage(ActualDamage, Victim, InstigatedBy, HitLocation, Momentum, DamageType);\n");
                out.push_str("}\n\n");
            }
        }

        UcArchetype::TournamentWeapon => {
            out.push_str("function Fire(float Value)\n");
            out.push_str("{\n");
            out.push_str("    if ( AmmoType == None || AmmoType.AmmoAmount <= 0 )\n");
            out.push_str("        return;\n");
            out.push_str("    PlayFiring();\n");
            out.push_str("    TraceFire(0.0);\n");
            out.push_str("    AmmoType.UseAmmo(1);\n");
            out.push_str("}\n\n");

            out.push_str("function AltFire(float Value)\n");
            out.push_str("{\n");
            out.push_str("    if ( AmmoType == None || AmmoType.AmmoAmount < 2 )\n");
            out.push_str("        return;\n");
            out.push_str("    PlayAltFiring();\n");
            out.push_str("    TraceFire(1.5);\n");
            out.push_str("    AmmoType.UseAmmo(2);\n");
            out.push_str("}\n\n");

            out.push_str("function PlayFiring()\n");
            out.push_str("{\n");
            out.push_str("    PlayAnim('Fire', 1.0, 0.05);\n");
            out.push_str("    Owner.PlaySound(FireSound, SLOT_Misc, 2.0);\n");
            out.push_str("}\n\n");

            out.push_str("function PlayAltFiring()\n");
            out.push_str("{\n");
            out.push_str("    PlayAnim('AltFire', 1.0, 0.05);\n");
            out.push_str("    Owner.PlaySound(AltFireSound, SLOT_Misc, 2.0);\n");
            out.push_str("}\n\n");
        }

        UcArchetype::Projectile => {
            out.push_str("simulated function PostBeginPlay()\n");
            out.push_str("{\n");
            out.push_str("    Super.PostBeginPlay();\n");
            out.push_str("    Velocity = Vector(Rotation) * Speed;\n");
            out.push_str("}\n\n");

            out.push_str("simulated function ProcessTouch(Actor Other, Vector HitLocation)\n");
            out.push_str("{\n");
            out.push_str("    if ( Other != Instigator )\n");
            out.push_str("    {\n");
            out.push_str("        Explode(HitLocation, Normal(HitLocation - Other.Location));\n");
            out.push_str("    }\n");
            out.push_str("}\n\n");

            out.push_str("simulated function HitWall(Vector HitNormal, Actor Wall)\n");
            out.push_str("{\n");
            out.push_str("    Explode(Location, HitNormal);\n");
            out.push_str("}\n\n");

            out.push_str("simulated function Explode(Vector HitLocation, Vector HitNormal)\n");
            out.push_str("{\n");
            out.push_str(&format!("    HurtRadius(Damage, {:.1}, MyDamageType, MomentumTransfer, HitLocation);\n", config.proj_damage_radius));
            out.push_str("    Destroy();\n");
            out.push_str("}\n\n");
        }

        UcArchetype::Pickup => {
            out.push_str("auto state Pickup\n");
            out.push_str("{\n");
            out.push_str("    function Touch(Actor Other)\n");
            out.push_str("    {\n");
            out.push_str("        if ( ValidTouch(Other) )\n");
            out.push_str("        {\n");
            out.push_str("            Pawn(Other).ClientMessage(PickupMessage);\n");
            out.push_str("            PlaySound(PickupSound);\n");
            out.push_str("            SetRespawn();\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
            out.push_str("}\n\n");
        }

        UcArchetype::Custom => {
            out.push_str("// Custom logic implementation\n");
            out.push_str("simulated function PostBeginPlay()\n");
            out.push_str("{\n");
            out.push_str("    Super.PostBeginPlay();\n");
            out.push_str("}\n\n");
        }
    }

    // 4. DefaultProperties Block
    out.push_str("defaultproperties\n");
    out.push_str("{\n");
    match config.archetype {
        UcArchetype::Mutator => {
            out.push_str("    bAlwaysKeep=True\n");
        }
        UcArchetype::TournamentWeapon => {
            out.push_str(&format!("    ItemName=\"{}\"\n", config.class_name));
            out.push_str(&format!("    AmmoName=Class'Botpack.{}'\n", config.weapon_ammo_name));
            out.push_str(&format!("    PickupAmmoCount={}\n", config.weapon_pickup_ammo));
            out.push_str(&format!("    FiringSpeed={:.2}\n", config.weapon_fire_rate));
            out.push_str(&format!("    AltFiringSpeed={:.2}\n", config.weapon_alt_fire_rate));
            out.push_str("    bMeleeWeapon=False\n");
            out.push_str("    bInstantHit=True\n");
        }
        UcArchetype::Projectile => {
            out.push_str(&format!("    Speed={:.1}\n", config.proj_speed));
            out.push_str(&format!("    MaxSpeed={:.1}\n", config.proj_max_speed));
            out.push_str(&format!("    Damage={:.1}\n", config.proj_damage));
            out.push_str(&format!("    MomentumTransfer={:.1}\n", config.proj_momentum));
            out.push_str("    MyDamageType=exploded\n");
            out.push_str("    bNetTemporary=True\n");
            out.push_str("    Physics=PHYS_Projectile\n");
        }
        UcArchetype::Pickup => {
            out.push_str(&format!("    PickupMessage=\"You got the {}.\"\n", config.class_name));
            out.push_str("    RespawnTime=30.000000\n");
            out.push_str("    bAutoActivate=True\n");
        }
        UcArchetype::Custom => {
            out.push_str("    bStatic=False\n");
            out.push_str("    bNoDelete=False\n");
        }
    }
    out.push_str("}\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_directives_parsing_and_formatting() {
        let lines = r#"
#exec MESH IMPORT MESH=Soldier ANIVFILE=MODELS\Soldier_a.3d DATAFILE=MODELS\Soldier_d.3d X=0 Y=0 Z=0
#exec MESH SEQUENCE MESH=Soldier SEQ=Walk STARTFRAME=0 NUMFRAMES=15 RATE=30.0
#exec AUDIO IMPORT FILE="SOUNDS\Fire.wav" NAME="WeaponFire" GROUP="Weapons"
#exec OBJ LOAD FILE="..\Textures\Botpack.utx" PACKAGE=Botpack
"#;
        let dirs = parse_exec_directives_from_uc(lines);
        assert_eq!(dirs.len(), 4);

        match &dirs[0] {
            ExecDirective::MeshImport { mesh, aniv_file, .. } => {
                assert_eq!(mesh, "Soldier");
                assert_eq!(aniv_file, r"MODELS\Soldier_a.3d");
            }
            _ => panic!("Expected MeshImport"),
        }

        match &dirs[1] {
            ExecDirective::MeshSequence { mesh, seq, num_frames, .. } => {
                assert_eq!(mesh, "Soldier");
                assert_eq!(seq, "Walk");
                assert_eq!(*num_frames, 15);
            }
            _ => panic!("Expected MeshSequence"),
        }

        match &dirs[2] {
            ExecDirective::AudioImport { name, group, .. } => {
                assert_eq!(name, "WeaponFire");
                assert_eq!(group, "Weapons");
            }
            _ => panic!("Expected AudioImport"),
        }

        // Verify roundtrip formatting
        let stmt = dirs[1].to_directive_string();
        assert!(stmt.contains("MESH=Soldier"));
        assert!(stmt.contains("SEQ=Walk"));
    }

    #[test]
    fn test_generate_scaffold_class_weapon() {
        let mut cfg = UcScaffoldConfig::default();
        cfg.class_name = "SniperShockRifle".to_string();
        cfg.archetype = UcArchetype::TournamentWeapon;
        cfg.exec_directives.push(ExecDirective::MeshSequence {
            mesh: "SniperShock".to_string(),
            seq: "Fire".to_string(),
            start_frame: 0,
            num_frames: 8,
            rate: 24.0,
        });

        let code = generate_scaffold_class(&cfg);
        assert!(code.contains("class SniperShockRifle extends TournamentWeapon;"));
        assert!(code.contains("#exec MESH SEQUENCE MESH=SniperShock SEQ=Fire"));
        assert!(code.contains("function Fire(float Value)"));
        assert!(code.contains("defaultproperties"));
        assert!(code.contains("ItemName=\"SniperShockRifle\""));
    }
}
