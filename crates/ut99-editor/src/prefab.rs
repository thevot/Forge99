use glam::Vec3;
use ut99_bsp::{build_cube, build_cylinder, Brush, CsgOperation};
use ut99_format::unr::actor::ActorRecord;
use crate::editor_state::{EditorMover, MoverKeyframe};

#[derive(Debug, Clone)]
pub struct Prefab {
    pub name: String,
    pub category: String,
    pub description: String,
    pub brushes: Vec<Brush>,
    pub movers: Vec<EditorMover>,
    pub actors: Vec<ActorRecord>,
}

impl Prefab {
    pub fn new(name: impl Into<String>, category: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            description: description.into(),
            brushes: Vec::new(),
            movers: Vec::new(),
            actors: Vec::new(),
        }
    }

    /// Convenience method to retrieve all canonical presets
    pub fn canonical_presets() -> Vec<Self> {
        get_default_prefabs()
    }
}

/// Returns the built-in library of canonical Unreal Tournament 99 architectural and gameplay prefabs.
pub fn get_default_prefabs() -> Vec<Prefab> {
    let mut prefabs = Vec::new();

    // 1. Gothic Arched Pillar
    {
        let mut p = Prefab::new(
            "Gothic Arched Pillar",
            "Architecture",
            "Segmented octagonal column with decorative footing and capital crown.",
        );
        // Base footing block
        let mut footing = build_cube("Pillar_Footing", CsgOperation::Add, Vec3::new(96.0, 96.0, 32.0));
        footing.location = Vec3::new(0.0, 0.0, -112.0);
        p.brushes.push(footing);

        // Shaft cylinder
        let shaft = build_cylinder("Pillar_Shaft", CsgOperation::Add, 192.0, 36.0, 8);
        p.brushes.push(shaft);

        // Capital crown
        let mut capital = build_cube("Pillar_Capital", CsgOperation::Add, Vec3::new(104.0, 104.0, 32.0));
        capital.location = Vec3::new(0.0, 0.0, 112.0);
        p.brushes.push(capital);

        prefabs.push(p);
    }

    // 2. Tournament Jump Pad
    {
        let mut p = Prefab::new(
            "Tournament Jump Pad",
            "Gameplay",
            "Octagonal kicker pad with neon directional accent and pre-wired jump trigger.",
        );
        // Base plate
        let base = build_cylinder("JumpPad_Base", CsgOperation::Add, 16.0, 64.0, 8);
        p.brushes.push(base);

        // Directional lip
        let mut lip = build_cube("JumpPad_Lip", CsgOperation::Add, Vec3::new(48.0, 24.0, 8.0));
        lip.location = Vec3::new(0.0, 48.0, 10.0);
        p.brushes.push(lip);

        // Jump Light
        let mut light = ActorRecord::default();
        light.name = "JumpPad_Glow".to_string();
        light.class_name = "Light".to_string();
        light.location = Vec3::new(0.0, 0.0, 24.0);
        light.light_hue = 155; // Turquoise / cyan
        light.light_saturation = 255;
        light.light_brightness = 128;
        light.light_radius = 24;
        p.actors.push(light);

        // Kicker Trigger
        let mut kicker = ActorRecord::default();
        kicker.name = "JumpPad_Trigger".to_string();
        kicker.class_name = "SpecialEvent".to_string();
        kicker.location = Vec3::new(0.0, 0.0, 32.0);
        p.actors.push(kicker);

        prefabs.push(p);
    }

    // 3. Wall Torch Sconce
    {
        let mut p = Prefab::new(
            "Wall Torch Sconce",
            "Lighting",
            "Gothic iron wall mount holding a flickering torch flame light source.",
        );
        // Bracket mount
        let mount = build_cube("Torch_Bracket", CsgOperation::Add, Vec3::new(8.0, 16.0, 32.0));
        p.brushes.push(mount);

        // Torch bowl
        let mut bowl = build_cylinder("Torch_Bowl", CsgOperation::Add, 16.0, 12.0, 6);
        bowl.location = Vec3::new(16.0, 0.0, 8.0);
        p.brushes.push(bowl);

        // Torch Flame Actor
        let mut torch = ActorRecord::default();
        torch.name = "Torch_Light".to_string();
        torch.class_name = "TorchFlame".to_string();
        torch.location = Vec3::new(16.0, 0.0, 24.0);
        torch.light_type = 1; // LT_Flicker
        torch.light_hue = 28; // Warm Fire Amber
        torch.light_saturation = 230;
        torch.light_brightness = 96;
        torch.light_radius = 36;
        p.actors.push(torch);

        prefabs.push(p);
    }

    // 4. Health Vial Wall Niche
    {
        let mut p = Prefab::new(
            "Health Vial Wall Niche",
            "Pickups",
            "Subtractive alcove carved into walls with cyan spotlight and HealthVial pickup.",
        );
        // Subtractive alcove
        let alcove = build_cube("Niche_Alcove", CsgOperation::Subtract, Vec3::new(64.0, 32.0, 72.0));
        p.brushes.push(alcove);

        // Sill ledge
        let mut sill = build_cube("Niche_Sill", CsgOperation::Add, Vec3::new(72.0, 36.0, 8.0));
        sill.location = Vec3::new(0.0, 0.0, -36.0);
        p.brushes.push(sill);

        // Niche Light
        let mut light = ActorRecord::default();
        light.name = "Niche_Light".to_string();
        light.class_name = "Light".to_string();
        light.location = Vec3::new(0.0, 0.0, 16.0);
        light.light_hue = 135;
        light.light_saturation = 255;
        light.light_brightness = 48;
        light.light_radius = 16;
        p.actors.push(light);

        // Health Vial
        let mut pickup = ActorRecord::default();
        pickup.name = "Niche_HealthVial".to_string();
        pickup.class_name = "HealthVial".to_string();
        pickup.location = Vec3::new(0.0, 0.0, -20.0);
        p.actors.push(pickup);

        prefabs.push(p);
    }

    // 5. Sci-Fi Sliding Door
    {
        let mut p = Prefab::new(
            "Sci-Fi Sliding Door Assembly",
            "Doors",
            "Complete automated door: subtractive corridor portal, vertical sliding mover, and trigger.",
        );
        // Subtractive doorway opening
        let doorway = build_cube("Door_Frame", CsgOperation::Subtract, Vec3::new(128.0, 32.0, 160.0));
        p.brushes.push(doorway);

        // Door Mover
        let mut door_brush = build_cube("SciFiDoor_Mover", CsgOperation::Active, Vec3::new(120.0, 12.0, 156.0));
        door_brush.location = Vec3::ZERO;

        let mut keyframes: [Option<MoverKeyframe>; 8] = Default::default();
        keyframes[0] = Some(MoverKeyframe { location: Vec3::ZERO, rotation: Vec3::ZERO });
        keyframes[1] = Some(MoverKeyframe { location: Vec3::new(0.0, 0.0, 144.0), rotation: Vec3::ZERO });

        let mover = EditorMover {
            name: "DoorMover0".to_string(),
            mover_class: "Mover".to_string(),
            brush: door_brush,
            current_key: 0,
            keyframes,
            initial_state: "TriggerOpenTimed".to_string(),
            stay_open_time: 4.0,
            move_time: 1.0,
            delay_time: 0.0,
            glide_type: "MV_Glide".to_string(),
            encroach_type: "ME_ReturnWhenEncroach".to_string(),
            tag: "SciFiDoor0".to_string(),
            event: String::new(),
            opening_sound: String::new(),
            opened_sound: String::new(),
            closing_sound: String::new(),
            closed_sound: String::new(),
            move_ambient_sound: String::new(),
        };
        p.movers.push(mover);

        // Proximity Trigger
        let mut trigger = ActorRecord::default();
        trigger.name = "Door_Trigger".to_string();
        trigger.class_name = "Trigger".to_string();
        trigger.location = Vec3::new(0.0, 0.0, -48.0);
        trigger.event = "SciFiDoor0".to_string();
        p.actors.push(trigger);

        prefabs.push(p);
    }

    prefabs
}
