use glam::Vec3;
use ut99_bsp::{
    build_cube, build_cylinder, build_sheet_advanced, build_staircase, Brush,
    CsgOperation, SheetAxis, SheetPreset, PF_FAKE_BACKDROP, PF_INVISIBLE, PF_NOT_SOLID, PF_PORTAL,
    PF_SEMISOLID, PF_SPECIAL_LIT,
};
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue};

use crate::editor_state::{EditorMover, MoverKeyframe};

/// High-level categories for the unified data-driven Smart Prefab system (`Deliverable E.4` & `F.1`),
/// consolidating the 65+ specialized Studio dialogs into a single parametric catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SmartPrefabCategory {
    Architecture,
    MoversAndLifts,
    HazardsAndTraps,
    LightingAndAtmosphere,
    GameplayAndObjectives,
}

impl SmartPrefabCategory {
    pub fn all() -> &'static [SmartPrefabCategory] {
        &[
            Self::Architecture,
            Self::MoversAndLifts,
            Self::HazardsAndTraps,
            Self::LightingAndAtmosphere,
            Self::GameplayAndObjectives,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Architecture => "🏛 Architecture & Rooms",
            Self::MoversAndLifts => "🛗 Movers, Lifts & Doors",
            Self::HazardsAndTraps => "☣ Hazards, ForceFields & Traps",
            Self::LightingAndAtmosphere => "💡 Lighting, Skybox & Zones",
            Self::GameplayAndObjectives => "🚩 CTF, Domination & Pickups",
        }
    }
}

/// Parametric configuration shared across data-driven Smart Prefabs.
#[derive(Debug, Clone)]
pub struct SmartPrefabParams {
    pub width: f32,
    pub length: f32,
    pub height: f32,
    pub wall_thickness: f32,
    pub travel_height: f32,
    pub move_time: f32,
    pub damage_per_sec: i32,
    pub light_brightness: u8,
    pub light_hue: u8,
    pub light_saturation: u8,
    pub team_number: u8,
    pub wall_texture: String,
    pub trim_texture: String,
    pub tag_prefix: String,
}

impl Default for SmartPrefabParams {
    fn default() -> Self {
        Self {
            width: 256.0,
            length: 256.0,
            height: 256.0,
            wall_thickness: 16.0,
            travel_height: 256.0,
            move_time: 1.5,
            damage_per_sec: 25,
            light_brightness: 160,
            light_hue: 150,
            light_saturation: 128,
            team_number: 0,
            wall_texture: "DefaultTexture".to_string(),
            trim_texture: "BaseTrim".to_string(),
            tag_prefix: "SmartPrefab".to_string(),
        }
    }
}

/// Result of instantiating a [`SmartPrefabTemplate`] in world space.
#[derive(Debug, Clone, Default)]
pub struct SmartPrefabOutput {
    pub brushes: Vec<Brush>,
    pub movers: Vec<EditorMover>,
    pub actors: Vec<ActorRecord>,
}

/// Metadata descriptor for a data-driven Smart Prefab template.
#[derive(Debug, Clone)]
pub struct SmartPrefabTemplate {
    pub id: &'static str,
    pub title: &'static str,
    pub category: SmartPrefabCategory,
    pub summary: &'static str,
}

/// Unified catalog of parametric UT99 Smart Prefabs (`Deliverable E.4` & `F.1`).
pub struct SmartPrefabLibrary;

impl SmartPrefabLibrary {
    pub fn catalog() -> Vec<SmartPrefabTemplate> {
        vec![
            SmartPrefabTemplate {
                id: "arch_hollow_room",
                title: "Hollow Arena Room + Light + PlayerStart",
                category: SmartPrefabCategory::Architecture,
                summary: "Subtractive room with outer shell, central light, and PlayerStart.",
            },
            SmartPrefabTemplate {
                id: "arch_sniper_parapet",
                title: "Sniper Bunker & Crenellated Parapet",
                category: SmartPrefabCategory::Architecture,
                summary: "Elevated semi-solid sniper platform with protective parapet and SniperRifle.",
            },
            SmartPrefabTemplate {
                id: "arch_stair_flight",
                title: "Linear Staircase + Bot PathNodes",
                category: SmartPrefabCategory::Architecture,
                summary: "Additive linear staircase with bottom/top PathNodes for bot traversal.",
            },
            SmartPrefabTemplate {
                id: "mover_lift_platform",
                title: "Vertical Elevator Lift + LiftCenter / LiftExits",
                category: SmartPrefabCategory::MoversAndLifts,
                summary: "2-keyframe StandOpenTimed Mover platform wired with LiftCenter and 2 LiftExits.",
            },
            SmartPrefabTemplate {
                id: "mover_secret_door",
                title: "Trigger-Activated Sliding Door",
                category: SmartPrefabCategory::MoversAndLifts,
                summary: "TriggerOpenTimed sliding door Mover paired with a proximity Trigger actor.",
            },
            SmartPrefabTemplate {
                id: "hazard_slime_basin",
                title: "Bio-Slime / Lava Fluid Basin + WaterZone",
                category: SmartPrefabCategory::HazardsAndTraps,
                summary: "Subtractive basin, non-solid translucent surface sheet, and damaging WaterZoneInfo.",
            },
            SmartPrefabTemplate {
                id: "hazard_force_field",
                title: "Energy Force Field Barrier + Trigger",
                category: SmartPrefabCategory::HazardsAndTraps,
                summary: "Translucent SpecialLit energy barrier sheet with SpecialLit light and Trigger.",
            },
            SmartPrefabTemplate {
                id: "light_skybox_chamber",
                title: "SkyBox Reference Chamber + SkyZoneInfo",
                category: SmartPrefabCategory::LightingAndAtmosphere,
                summary: "Isolated SkyBox cube with SkyZoneInfo and ambient sky lighting.",
            },
            SmartPrefabTemplate {
                id: "light_zone_portal",
                title: "Zone Portal Sheet + Dual ZoneInfo",
                category: SmartPrefabCategory::LightingAndAtmosphere,
                summary: "PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID partition sheet with two ZoneInfo actors.",
            },
            SmartPrefabTemplate {
                id: "gameplay_ctf_flag_base",
                title: "CTF Flag Base Pedestal + DefensePoints",
                category: SmartPrefabCategory::GameplayAndObjectives,
                summary: "Octagonal semi-solid flag pedestal with FlagBase, Team Light, and PathNode.",
            },
            SmartPrefabTemplate {
                id: "gameplay_teleporter_pair",
                title: "Bidirectional Teleporter Arch Pair",
                category: SmartPrefabCategory::GameplayAndObjectives,
                summary: "Two linked Teleporter actors with URL <-> Tag cross-wiring and arch pillars.",
            },
            SmartPrefabTemplate {
                id: "gameplay_supply_locker",
                title: "Tournament Weapon & Shield Supply Alcove",
                category: SmartPrefabCategory::GameplayAndObjectives,
                summary: "Semi-solid supply pad with ShockRifle, MedBox, and ShieldBelt.",
            },
        ]
    }

    /// Instantiates a template by `id` at `origin` using `params`.
    pub fn instantiate(
        id: &str,
        origin: Vec3,
        params: &SmartPrefabParams,
    ) -> Result<SmartPrefabOutput, String> {
        let mut out = SmartPrefabOutput::default();
        let w = params.width.max(64.0);
        let l = params.length.max(64.0);
        let h = params.height.max(64.0);
        let tex = if params.wall_texture.is_empty() {
            "DefaultTexture"
        } else {
            &params.wall_texture
        };

        let make_light = |name: String, loc: Vec3, b: u8, hue: u8, sat: u8, special: bool| -> ActorRecord {
            let mut a = ActorRecord::default();
            a.class_name = "Light".to_string();
            a.name = name;
            a.location = loc;
            a.light_brightness = b;
            a.light_hue = hue;
            a.light_saturation = sat;
            a.light_radius = 64;
            if special {
                a.properties.insert(
                    "bSpecialLit".to_string(),
                    TaggedProperty {
                        name: "bSpecialLit".to_string(),
                        array_index: 0,
                        value: TaggedValue::Bool(true),
                    },
                );
            }
            a
        };

        match id {
            "arch_hollow_room" => {
                let mut room = build_cube("PrefabRoom", CsgOperation::Subtract, Vec3::new(w, l, h));
                room.location = origin;
                for p in &mut room.polys {
                    p.texture = tex.to_string();
                }
                out.brushes.push(room);

                out.actors.push(make_light(
                    format!("{}_Light0", params.tag_prefix),
                    origin + Vec3::new(0.0, 0.0, h * 0.3),
                    params.light_brightness,
                    params.light_hue,
                    params.light_saturation,
                    false,
                ));

                let mut ps = ActorRecord::default();
                ps.class_name = "PlayerStart".to_string();
                ps.name = format!("{}_PlayerStart0", params.tag_prefix);
                ps.location = origin + Vec3::new(0.0, 0.0, -h * 0.5 + 48.0);
                out.actors.push(ps);
            }
            "arch_sniper_parapet" => {
                let mut deck = build_cube(
                    "SniperDeck",
                    CsgOperation::Add,
                    Vec3::new(w, l * 0.5, 24.0),
                );
                deck.location = origin;
                for p in &mut deck.polys {
                    p.texture = tex.to_string();
                    p.flags |= PF_SEMISOLID;
                }
                out.brushes.push(deck);

                let mut parapet = build_cube(
                    "SniperParapet",
                    CsgOperation::Add,
                    Vec3::new(w, 16.0, 48.0),
                );
                parapet.location = origin + Vec3::new(0.0, l * 0.25 - 8.0, 36.0);
                for p in &mut parapet.polys {
                    p.texture = params.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                out.brushes.push(parapet);

                let mut rifle = ActorRecord::default();
                rifle.class_name = "SniperRifle".to_string();
                rifle.name = format!("{}_SniperRifle0", params.tag_prefix);
                rifle.location = origin + Vec3::new(0.0, 0.0, 36.0);
                out.actors.push(rifle);
            }
            "arch_stair_flight" => {
                let mut stairs = build_staircase(
                    "PrefabStairs",
                    CsgOperation::Add,
                    8,
                    16.0,
                    w.min(256.0),
                    32.0,
                );
                stairs.location = origin;
                for p in &mut stairs.polys {
                    p.texture = tex.to_string();
                }
                out.brushes.push(stairs);

                for (idx, offset) in [Vec3::new(0.0, -128.0, 24.0), Vec3::new(0.0, 128.0, 152.0)]
                    .into_iter()
                    .enumerate()
                {
                    let mut pn = ActorRecord::default();
                    pn.class_name = "PathNode".to_string();
                    pn.name = format!("{}_StairNode{}", params.tag_prefix, idx);
                    pn.location = origin + offset;
                    out.actors.push(pn);
                }
            }
            "mover_lift_platform" => {
                let mut brush = build_cube(
                    "LiftMoverBrush",
                    CsgOperation::Active,
                    Vec3::new(128.0, 128.0, 16.0),
                );
                brush.location = origin;
                for p in &mut brush.polys {
                    p.texture = params.trim_texture.clone();
                }
                let lift_tag = format!("{}_Lift", params.tag_prefix);
                let mut keyframes: [Option<MoverKeyframe>; 8] = Default::default();
                keyframes[0] = Some(MoverKeyframe {
                    location: origin,
                    rotation: Vec3::ZERO,
                });
                keyframes[1] = Some(MoverKeyframe {
                    location: origin + Vec3::new(0.0, 0.0, params.travel_height),
                    rotation: Vec3::ZERO,
                });
                out.movers.push(EditorMover {
                    name: format!("{}_Mover0", params.tag_prefix),
                    mover_class: "Mover".to_string(),
                    brush,
                    current_key: 0,
                    keyframes,
                    initial_state: "StandOpenTimed".to_string(),
                    stay_open_time: 3.0,
                    move_time: params.move_time.max(0.2),
                    delay_time: 0.0,
                    glide_type: "MV_GlideByTime".to_string(),
                    encroach_type: "ME_ReturnWhenEncroach".to_string(),
                    tag: lift_tag.clone(),
                    event: String::new(),
                    opening_sound: "DoorsMod.ls1".to_string(),
                    opened_sound: String::new(),
                    closing_sound: String::new(),
                    closed_sound: "DoorsMod.le1".to_string(),
                    move_ambient_sound: String::new(),
                });

                let mut lc = ActorRecord::default();
                lc.class_name = "LiftCenter".to_string();
                lc.name = format!("{}_LiftCenter0", params.tag_prefix);
                lc.location = origin + Vec3::new(0.0, 0.0, 32.0);
                lc.properties.insert(
                    "LiftTag".to_string(),
                    TaggedProperty {
                        name: "LiftTag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(lift_tag.clone()),
                    },
                );
                out.actors.push(lc);

                for (i, z_off) in [0.0f32, params.travel_height].into_iter().enumerate() {
                    let mut le = ActorRecord::default();
                    le.class_name = "LiftExit".to_string();
                    le.name = format!("{}_LiftExit{}", params.tag_prefix, i);
                    le.location = origin + Vec3::new(96.0 * if i == 0 { -1.0 } else { 1.0 }, 0.0, z_off + 32.0);
                    le.properties.insert(
                        "LiftTag".to_string(),
                        TaggedProperty {
                            name: "LiftTag".to_string(),
                            array_index: 0,
                            value: TaggedValue::Name(lift_tag.clone()),
                        },
                    );
                    out.actors.push(le);
                }
            }
            "mover_secret_door" => {
                let mut brush = build_cube(
                    "SecretDoorBrush",
                    CsgOperation::Active,
                    Vec3::new(16.0, 128.0, 192.0),
                );
                brush.location = origin;
                for p in &mut brush.polys {
                    p.texture = tex.to_string();
                }
                let door_tag = format!("{}_SecretDoor", params.tag_prefix);
                let mut keyframes: [Option<MoverKeyframe>; 8] = Default::default();
                keyframes[0] = Some(MoverKeyframe {
                    location: origin,
                    rotation: Vec3::ZERO,
                });
                keyframes[1] = Some(MoverKeyframe {
                    location: origin + Vec3::new(0.0, 120.0, 0.0),
                    rotation: Vec3::ZERO,
                });
                out.movers.push(EditorMover {
                    name: format!("{}_DoorMover0", params.tag_prefix),
                    mover_class: "Mover".to_string(),
                    brush,
                    current_key: 0,
                    keyframes,
                    initial_state: "TriggerOpenTimed".to_string(),
                    stay_open_time: 4.0,
                    move_time: params.move_time.max(0.2),
                    delay_time: 0.0,
                    glide_type: "MV_GlideByTime".to_string(),
                    encroach_type: "ME_ReturnWhenEncroach".to_string(),
                    tag: door_tag.clone(),
                    event: String::new(),
                    opening_sound: "DoorsMod.ds1".to_string(),
                    opened_sound: String::new(),
                    closing_sound: String::new(),
                    closed_sound: "DoorsMod.de1".to_string(),
                    move_ambient_sound: String::new(),
                });

                let mut trig = ActorRecord::default();
                trig.class_name = "Trigger".to_string();
                trig.name = format!("{}_DoorTrigger0", params.tag_prefix);
                trig.location = origin + Vec3::new(-64.0, 0.0, -32.0);
                trig.event = door_tag;
                out.actors.push(trig);
            }
            "hazard_slime_basin" => {
                let mut basin = build_cube(
                    "SlimeBasinPit",
                    CsgOperation::Subtract,
                    Vec3::new(w, l, 96.0),
                );
                basin.location = origin;
                for p in &mut basin.polys {
                    p.texture = tex.to_string();
                }
                out.brushes.push(basin);

                let mut sheet = build_sheet_advanced(
                    "SlimeSurfaceSheet",
                    CsgOperation::Add,
                    w,
                    l,
                    SheetAxis::XY,
                    SheetPreset::AcidSlime,
                );
                sheet.location = origin + Vec3::new(0.0, 0.0, 32.0);
                for p in &mut sheet.polys {
                    p.texture = "Liquids.Slime".to_string();
                    p.flags |= PF_PORTAL | PF_NOT_SOLID;
                }
                out.brushes.push(sheet);

                let mut wz = ActorRecord::default();
                wz.class_name = "WaterZone".to_string();
                wz.name = format!("{}_WaterZone0", params.tag_prefix);
                wz.location = origin;
                wz.properties.insert(
                    "DamagePerSec".to_string(),
                    TaggedProperty {
                        name: "DamagePerSec".to_string(),
                        array_index: 0,
                        value: TaggedValue::Int(params.damage_per_sec),
                    },
                );
                out.actors.push(wz);
            }
            "hazard_force_field" => {
                let mut sheet = build_sheet_advanced(
                    "ForceFieldSheet",
                    CsgOperation::Add,
                    w,
                    h,
                    SheetAxis::YZ,
                    SheetPreset::Water,
                );
                sheet.location = origin;
                for p in &mut sheet.polys {
                    p.texture = params.trim_texture.clone();
                    p.flags |= PF_SPECIAL_LIT;
                }
                out.brushes.push(sheet);

                out.actors.push(make_light(
                    format!("{}_ForceFieldLight0", params.tag_prefix),
                    origin + Vec3::new(32.0, 0.0, 0.0),
                    params.light_brightness,
                    params.light_hue,
                    params.light_saturation,
                    true,
                ));
            }
            "light_skybox_chamber" => {
                let mut sky = build_cube(
                    "SkyBoxChamber",
                    CsgOperation::Subtract,
                    Vec3::new(512.0, 512.0, 512.0),
                );
                sky.location = origin;
                for p in &mut sky.polys {
                    p.texture = tex.to_string();
                    p.flags |= PF_FAKE_BACKDROP;
                }
                out.brushes.push(sky);

                let mut sz = ActorRecord::default();
                sz.class_name = "SkyZoneInfo".to_string();
                sz.name = format!("{}_SkyZoneInfo0", params.tag_prefix);
                sz.location = origin;
                out.actors.push(sz);
            }
            "light_zone_portal" => {
                let mut portal = build_sheet_advanced(
                    "ZonePortalSheet",
                    CsgOperation::Add,
                    w,
                    h,
                    SheetAxis::YZ,
                    SheetPreset::ZonePortal,
                );
                portal.location = origin;
                for p in &mut portal.polys {
                    p.flags |= PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID;
                }
                out.brushes.push(portal);

                for (i, dx) in [-96.0f32, 96.0f32].into_iter().enumerate() {
                    let mut zi = ActorRecord::default();
                    zi.class_name = "ZoneInfo".to_string();
                    zi.name = format!("{}_ZoneInfo{}", params.tag_prefix, i);
                    zi.location = origin + Vec3::new(dx, 0.0, 0.0);
                    out.actors.push(zi);
                }
            }
            "gameplay_ctf_flag_base" => {
                let mut ped = build_cylinder(
                    "FlagPedestal",
                    CsgOperation::Add,
                    24.0,
                    64.0,
                    8,
                );
                ped.location = origin;
                for p in &mut ped.polys {
                    p.texture = params.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                out.brushes.push(ped);

                let mut fb = ActorRecord::default();
                fb.class_name = "FlagBase".to_string();
                fb.name = format!("{}_FlagBase0", params.tag_prefix);
                fb.location = origin + Vec3::new(0.0, 0.0, 48.0);
                fb.properties.insert(
                    "Team".to_string(),
                    TaggedProperty {
                        name: "Team".to_string(),
                        array_index: 0,
                        value: TaggedValue::Byte(params.team_number),
                    },
                );
                out.actors.push(fb);

                let hue = if params.team_number == 0 { 0 } else { 160 };
                out.actors.push(make_light(
                    format!("{}_TeamLight0", params.tag_prefix),
                    origin + Vec3::new(0.0, 0.0, 112.0),
                    200,
                    hue,
                    64,
                    false,
                ));
            }
            "gameplay_teleporter_pair" => {
                let tag_a = format!("{}_TeleA", params.tag_prefix);
                let tag_b = format!("{}_TeleB", params.tag_prefix);
                for (i, (pos, my_tag, dst_url)) in [
                    (origin + Vec3::new(-192.0, 0.0, 0.0), tag_a.clone(), tag_b.clone()),
                    (origin + Vec3::new(192.0, 0.0, 0.0), tag_b, tag_a),
                ]
                .into_iter()
                .enumerate()
                {
                    let mut arch = build_cube(
                        format!("TeleArch{}", i),
                        CsgOperation::Add,
                        Vec3::new(32.0, 96.0, 128.0),
                    );
                    arch.location = pos;
                    for p in &mut arch.polys {
                        p.texture = params.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    out.brushes.push(arch);

                    let mut tp = ActorRecord::default();
                    tp.class_name = "Teleporter".to_string();
                    tp.name = format!("{}_Teleporter{}", params.tag_prefix, i);
                    tp.location = pos;
                    tp.tag = my_tag;
                    tp.properties.insert(
                        "URL".to_string(),
                        TaggedProperty {
                            name: "URL".to_string(),
                            array_index: 0,
                            value: TaggedValue::Str(dst_url),
                        },
                    );
                    out.actors.push(tp);
                }
            }
            "gameplay_supply_locker" => {
                let mut pad = build_cube(
                    "SupplyPad",
                    CsgOperation::Add,
                    Vec3::new(192.0, 96.0, 16.0),
                );
                pad.location = origin;
                for p in &mut pad.polys {
                    p.texture = params.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                out.brushes.push(pad);

                for (i, (cls, dx)) in [
                    ("ShockRifle", -56.0f32),
                    ("MedBox", 0.0f32),
                    ("UT_ShieldBelt", 56.0f32),
                ]
                .into_iter()
                .enumerate()
                {
                    let mut item = ActorRecord::default();
                    item.class_name = cls.to_string();
                    item.name = format!("{}_{}_{}", params.tag_prefix, cls, i);
                    item.location = origin + Vec3::new(dx, 0.0, 32.0);
                    out.actors.push(item);
                }
            }
            other => {
                return Err(format!("Unknown SmartPrefabTemplate id '{}'", other));
            }
        }

        Ok(out)
    }
}
