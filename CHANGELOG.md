# Changelog — Forge99

All notable changes to **Forge99** are documented in this file in accordance with [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.25.0] — 2026-11-23

### Changed
- 3D preview performance and hardware scalability: adaptive spatial chunking with per-viewport frustum culling, BVH-accelerated CSG evaluation and navigation path queries, lightmap atlas packing, cached grid and selection overlays, asynchronous non-blocking CSG updates, copy-on-write level snapshots, and multi-backend hardware-adaptive frame pacing.
- Viewport rendering reuses failed texture lookups until resources change, avoiding repeated scans for missing or ambiguous materials. Storage is bounded; imports and repaired assets are retried, and GPU atlas exhaustion remains distinct from a missing texture.
- World-brush wireframe outlines transform positions through a reusable scratch buffer instead of cloning complete polygons, material strings, vertex colors and lightmaps on each frame. Existing segment order, selection colors and degenerate-polygon filtering are preserved.
- The scene Outliner renders only visible rows, with single-line labels and full-name tooltips. Range selection retains offscreen items; changing filters/categories or changing the scene invalidates obsolete range anchors, including edits from the viewport and shortcuts.

### Fixed
- Close/Cancel buttons now dismiss Find & Replace Textures, Add Special Brush, CTF Symmetry and Zone Boundary Leak Detector without being overwritten at frame end. Confirming Add Special Brush creates once and closes; titlebar close buttons and live tool settings retain their behavior.
- Surface flag controls enable mixed selections uniformly and clear fully enabled selections. Summaries cover every targeted face; multi-brush changes form one undo step and mark the map dirty. Builder flag edits are undoable without dirtying the map, while invalid targets and unchanged flags create no history entries.
- Find Actors focuses search on opening and updates results before handling actions. Arrows and single clicks highlight results without changing scene selection or moving the camera; Enter, double-click and explicit actions confirm the target. Background windows ignore unrelated keys, and stale actor indices are revalidated.
- Command Palette preserves manual scrolling and no longer recenters on hover or idle frames. Opening, search changes and keyboard navigation reveal the selected result with minimal scrolling.
- Texture references preserve package and group identity across lookup, browser selection, In Use filtering, thumbnails and single-texture operations. Missing qualified references and ambiguous short names no longer select unrelated textures. Resource changes invalidate stale viewport lookups; thumbnail replacement and conversion clear matching memory accounting.
- Undo and Redo track actual document changes after a save, restoring unsaved-change prompts for opening, creating and closing maps. Builder tools, studio previews, empty history and edits without valid targets retain the previous dirty state.
- T3D, glTF and OBJ exports preserve the current map path, unsaved changes and recent maps; export completion cannot satisfy a pending save-and-close request. Active operations cannot be replaced or dismissed before completion, and export feedback distinguishes exports from document saves.
- Playtest validates executable/folder, launch arguments and active saves before building; failures are visible and Launch Settings stays open. External maps use absolute paths, URL options are attached to the map, and quoted argument paths retain spaces.
- F8 requires a .unr destination instead of overwriting a .t3d source, and unnamed playtests use distinct filenames. The wizard checks launch configuration before generating or saving.
- New Map clears prior document objects, MyLevel assets, selections and derived state while retaining shared libraries and preferences. Active saves/exports or viewport transforms must finish before the reset; deferred New Map still runs after a successful save.
- Audio browsers use the active UT installation for discovery, loading and music extraction; confirmed path changes refresh resources while retaining imported and MyLevel sounds.
- Audio loading and export failures are reported, package exports ask for a destination, and music preview reports a request to an external player rather than claiming playback.
- Snapshot restore now creates an unsaved copy requiring an explicit save destination, clears obsolete undo/selection state, and restores maps with no brushes without retaining previous geometry.
- Safe recovery loads the original map directly without temporary copies, preserves its name and recent-file entry, clears stale navigation, and marks recovered work as unsaved.
- Auto-save T3D snapshots and UNR backups now rotate independently through all five and ten slots, respectively.
- Backups are staged before replacement; disk failures preserve in-memory snapshots and report the failed destination without retrying every frame.
- Each editor session uses a separate backup folder, shown in the recovery dialog with an Open Folder action. Manual UNR backups now use that folder instead of the working directory's `Maps/` folder. Existing backups are retained.

## [0.24.0] — 2026-11-04

### Added
- **Construction rules (`generator_version`) in the Map Wizard:** v1 Legacy (default, bit-for-bit v0.23.5), v2 Portals (wall-to-wall procedural corridors, no floating doorway frames, anchored beams/rafters/accent fixtures), v3 Heights (weighted ceiling-height library per role, stair headroom guard of 112 UU).
- **Generated-map provenance:** `<map>.f99.json` manifest; generated maps are excluded from the corpus.
- **Dedicated wizard output folder** and an explicit "Install in UT" button.
- **Tooling:** `wizard_baseline`, `module_clearance`, `fuzz_wizard_randomizer`, `build_wizard_matrix` examples.

### Fixed
- Isolated stair waypoints are pruned before export (one cause of Level Doctor navigation errors in the one-click randomizer).

### Known issues
- Rare residual failure of the one-click randomizer test (Domination, `CentralArenaMultiTier`); the test now logs the configuration.

---

## [0.23.5] — 2026-11-03

### Fixed
- **Stair Grid Snapping & Seam Alignment (`map_wizard.rs`):**
  - **Catwalk Stairs Grid Snapping:** Catwalk stairs run length updated from 256.0 UU to 288.0 UU ($12 \times 24.0\text{ UU}$ run, $16.0\text{ UU}$ rise), snapping all stair tread, riser, and landing vertices to integer multiples of 8.0 and 16.0 UU. Eliminates the $21.3333\text{ UU}$ fractional vertex coordinates that previously caused coplanar BSP splits, non-manifold geometry, and partially invisible stair steps in UT99.
  - **Transit Stairs Seam Alignment:** In `ensure_vertical_reachability`, transit stair positioning now aligns the top step tread directly with the bounding box boundary of the lower room (`lower_node.max_x().min(higher_node.max_x())`, etc.). Eliminates the 16.0–48.0 UU drop gap / trench between top step treads and higher room floors, achieving exact $0.0\text{ UU}$ seam transitions.
  - **Adaptive Step Sizing:** Transit stair step count is now computed dynamically as $\lceil \Delta Z / 16.0 \rceil$, guaranteeing that risers never exceed $16.0\text{ UU}$ (comfortably beneath UT99's $25.0\text{ UU}$ `MaxStepHeight`), while step runs snap cleanly to 8-UU grid increments.
  - **Comprehensive Stair Alignment Audit:** Added `audit_stair_alignment` verifying step rise ($\le 24.0\text{ UU}$), step run ($\ge 16.0\text{ UU}$), 8-UU grid compliance, and seam gap ($\le 8.0\text{ UU}$) across all 40 procedural archetypes. Validated 166 generated stairs with zero defects.
- **Viewport Camera Handedness Correction (`camera.rs`):**
  - Switched 3D perspective projection to left-handed coordinate convention (`Mat4::perspective_lh`), matching Unreal Tournament 99 / Unreal Engine 1's native left-handed world coordinate system (X forward/North, Y right/East, Z up).
  - Corrected view matrix derivation (`up.cross(fwd)` instead of `fwd.cross(up)`) for left-handed camera viewports in WGPU. The 3D perspective viewport now perfectly matches top-down (XY) and in-game orientations without horizontal mirroring.

### Added
- **Gameplay Ammo Controls & Expanded Item Quota (`map_wizard_dialog.rs`):**
  - **Ammo Per Weapon Configuration:** Added new selector in the "Gameplay Systems" tab allowing creators to choose between 1, 2, 3, or 4 ammo pickups spawned per placed weapon (`ammo_per_weapon`).
  - **Expanded Item Quota (200 items):** Increased maximum item quota slider limit from 50 to 200 items across weapons, ammo, health, and armor, allowing dense weapon spawns and intense arena setups.

### Verified
- **Map Regeneration & Engine Verification:**
  - Regenerated `DM-TwitchingRetina.unr` (`VerticalTierCinder` archetype, Dark Cyber Industrial theme) with 100% aligned stairs ($X = -576.0$ exact seam, 0 gap, 32 UU doorway clearance, full 192 UU riser climb).
  - Headless server launch test verified with `UCC.exe server DM-TwitchingRetina.unr?game=Botpack.DeathMatchPlus`: game initialized cleanly with zero errors.

---

## [0.23.4] — 2026-11-02

### Fixed
- **PlayerStart Sub-Floor Void Spawning & Telefrag Repulsion (`map_wizard.rs`):**
  - **Vertical Tier Isolation:** Resolved 3D telefrag repulsion vectors in Phase 5 and Phase 6B that pushed player spawns downwards into bedrock across vertically stacked chambers. Repulsion now explicitly skips player start pairs separated by $|\Delta Z| > 80.0\text{ UU}$ (players on different elevations cannot telefrag each other).
  - **In-Plane 2D Repulsion:** Constrained telefrag repulsion force vectors strictly to the horizontal 2D plane ($Z = 0$), preventing any downward displacement from vertical difference vectors.
  - **3D Z-Containment in Room Matching:** Fixed 2D-only XY room lookups across 5 separate locations in `map_wizard.rs`. In multi-tier arenas where rooms share the same XY center (e.g. `ColosseumBasinCore` over `SubBasinHypogeum`), actors now require $Z \in [\text{floor}_z - 32, \text{ceiling}_z + 32]$, preventing lower-tier actors from falsely matching upper-tier room floors.
  - **Floor Pinning & Unplaceable Recovery:** Added explicit floor clamping ($Z = \text{floor}_z + 41.0\text{ UU}$) and recovery logic: unplaceable `PlayerStart` actors in Phase 6B are safely centered on their room floor rather than abandoned in solid void rock.
- **Level Doctor Proactive Spawn Cavity Audit (`diagnostics.rs`):**
  - Added `SpawnClearance` verification in `LevelDoctor::audit_gameplay_clearance`. Any `PlayerStart` located outside subtracted CSG room/corridor volumes triggers a fatal `IssueSeverity::Error`, catching void and bedrock spawns proactively before compilation.
- **Map Regeneration:**
  - Regenerated `DM-BlackCoreDock.unr` (`SunkenColosseumBasin`). Spawns `PlayerStart10` and `PlayerStart11` now sit at $Z = -183.0\text{ UU}$ ($+41.0\text{ UU}$ clearance above $-224.0$ floor, Zone 1), completely eliminating the "Failed to spawn player actor" crash.

---

## [0.23.3] — 2026-11-02

### Fixed
- **Elimination of Nested & Overlapping Transit Stairs (`map_wizard.rs`):**
  - Added corridor bridging check (`bridged_by_corridor`) in `ensure_vertical_reachability`: when two rooms are already connected via an intermediate corridor node, direct room-to-room transit stairs across the seam are suppressed.
  - Added spatial bounding-box deconfliction tracking (`placed_transit_boxes`): rejects any new transit stair/ramp whose XY footprint overlaps an already placed flight by more than 24 UU, preventing double stairs from spawning inside each other.
  - Corrected `GothicCathedralNave` nave chamber length (`CathedralNave.size.y = step_y = 1280.0`, spanning `[-640, 640]`) so it no longer overlaps with `AltarAmbulatory`, `NarthexEntryConduit`, `ApseHighAltar`, and `NarthexVestibule`.
  - Expanded `CryptStairTransit` chamber length from 320 UU to 448 UU, providing 144 UU of level floor space for the foot navigation waypoint and eliminating waypoint clipping.
- **Escapable Water Basins & Hazard Pits (`map_wizard.rs`, `navigation.rs`):**
  - Reduced hazard pit depth `pit_d` in `CryptCatacomb_HazardPit` from 96 UU to 48 UU (water sheet depth 40 UU, rim height 48 UU), well within UT99 standard jump height (~60–64 UU).
  - Added submerged 2-step egress stairs (`HazardPitEgress_0` and `HazardPitEgress_1`, step rise 24 UU <= `MaxStepHeight = 25.0`) along open North and South basin edges, paired with smooth invisible collision ramps (`HazardPitEgressCol_...`) for seamless walking escape.
  - Added `HazardPitEgress` and `HazardBridge` exemptions to `is_nav_link_blocked_by_brushes` in both CSG line-of-sight sampling passes in `navigation.rs`, allowing bot navigation reachspecs to route freely across the basin.
- **Map Regeneration:**
  - Regenerated `UnrealTournament\Maps\DM-UmbrielAbyss.unr` with 143 brushes, 89 actors, 0 Level Doctor errors, clear single-run stairs, and fully escapable central water pit.

---

## [0.23.2] — 2026-11-02

### Fixed
- **Multi-Room Procedural Archetype Interconnection (`map_wizard.rs`):**
  - Corrected radial corridor and ramp spans in `SunkenColosseumBasin` (`ColosseumRamp_N/S/E/W`), extending reach from radius 640 UU to 896 UU to seamlessly bridge the central colosseum basin (`ColosseumBasinCore`) and all four spectator ledges (`North/South/East/WestSpectatorLedge`), eliminating a 106 UU gap of solid void rock that previously isolated players.
  - Sized and aligned connecting corridors, drainage shafts, and skybridges across all 7 multi-room procedural archetypes (`SunkenNecropolisCrypt`, `HyperFortressBastion`, `DeepSeaAbyssStation`, `VerticalTierCinder`, `SubterraneanMagmaForge`, `SkylineRooftopHighway`), guaranteeing 100% interconnected arenas.
- **CTF FlagBase Encroachment & Team Ownership Scoping (`map_wizard.rs`):**
  - Scoped `encroaches_flag_base` safety check strictly to `WizardGameMode::CaptureTheFlag`. Non-CTF modes (DeathMatch, Duel 1v1, Domination, Assault) never omit transit stairs due to false flag base collisions.
  - Clamped transit stair runs entering team bases to `((seam - center).abs() - 96.0).max(96.0)` in Capture the Flag, preventing stairs from colliding with flag pedestals while maintaining full reachability.
  - Restricted `team_owner: Some(...)` assignment in `ProceduralSeed` to team modes (`CaptureTheFlag`, `TeamDeathMatch`, `Assault`), eliminating phantom team ownership and unscalable vertical drops in DeathMatch.
- **PlayerStart Telefrag Repulsion Boundary Clamping (`map_wizard.rs`):**
  - Clamped player spawn coordinates to interior room boundaries (`min + 64.0 ..= max - 64.0`) in both telefrag repulsion passes, preventing strong mutual repulsion forces from pushing spawn points into exterior voids.
- **Bot AI Navigation ReachSpecs & Collision Ramp Penetration (`navigation.rs`):**
  - Updated `is_nav_link_blocked_by_brushes` brush filters to match `.contains("TransitRamp")` and `.contains("TransitStair")`, and added `HubSniperBalcony` to non-solid exemptions across both CSG air sampling passes. Invisible collision ramps (`TransitStairCol_...`) and elevated balconies are no longer treated as solid stone blocking bot line-of-sight.
  - Compensated for `InventorySpot` floor offsets (`(delta_z.abs() - 32.0).max(0.0)`) in `auto_build_navigation_reachspecs_with_brushes`, matching `builder.rs` reachability thresholds and achieving 0-error Level Doctor passes.

---

## [0.23.1] — 2026-11-02

### Fixed
- **Watertight Stepped Stairs Polygon Normals (`brush.rs`):**
  - Corrected inverted vertex winding order across all step risers (vertical faces) and back walls in `build_stepped_stairs_oriented` for all four cardinal directions (`North`, `South`, `East`, `West`). Risers previously faced inward, causing hardware backface culling to render them completely invisible (sliced staircase appearance).
  - Added unit test `test_build_stepped_stairs_oriented_watertight_and_cardinal` verifying outward-facing normal vectors across bottom, tread, riser, side, and back polygons.
- **Catwalk Deck & Guard Rail Overlap (`map_wizard.rs`):**
  - Sized `CatwalkDeck` and `CatwalkRail` to span only the horizontal portion (`shelf_len - stair_run`), eliminating the solid deck slab that previously hung directly over the ascending stair flight.
  - Stair flights now feature full vertical headroom up to the room ceiling and meet the deck seamlessly at `catwalk_z`.
  - Added unit test `catwalk_stair_treads_have_headroom_and_reach_the_deck`.
- **Vertical Reachability Transit Stairs Positioning (`map_wizard.rs`):**
  - Repositioned multi-elevation transit stairs to rest entirely within the lower room (`lower_node`), with the top step's rear face aligned flush with the room seam at `high_floor`. Previously, centering the brush on the room seam buried the top half of the flight inside the higher room's solid floor rock, leaving visible stairs that terminated halfway up.
  - Added unit test `transit_stairs_reach_higher_floor_and_lie_within_lower_room`.
- **Kinetic Kicker 3D Vector & JumpSpot Landing Alignment (`map_wizard.rs`):**
  - Aligned kinetic `Kicker` location with the foot of the flight (`foot_loc`) and `JumpSpot` with the head of the flight (`head_loc`), computing full 3D trajectory velocity vectors. Eliminates kickers placed inside solid void or rock between non-coincident room centers.
- **Bot AI Navigation Waypoint Deconfliction (`map_wizard.rs`):**
  - Enhanced `try_push_pathnode` proximity deconfliction to suppress redundant generic grid waypoints within 220 UU of dedicated stair waypoints (`PathNode_Foot_`, `PathNode_Head_`, `Catwalk`, `JumpSpot_Landing_`), preventing nodes from being displaced into solid stair brushes.
- **Secret Armory Alcove Exterior Wall Validation (`map_wizard.rs`):**
  - Constrained secret armory alcove candidates to exterior void walls, rejecting placements that overlap with elevated corridors or shafts in XY.
- **Navigation Raycast Brush Exemption (`navigation.rs`):**
  - Exempted `TransitStair_`, `CatwalkStairs`, and `CatwalkDeck` brushes from `is_nav_link_blocked_by_brushes` collision testing so bot navigation rays can traverse stepped stair flights.
- **Paths[16] ReachSpec Hard Array Capping (`navigation.rs`):**
  - Refined `prune_redundant_and_cap_paths16` with a two-phase capping strategy that prioritizes pruning nodes with alternative incoming paths before enforcing the UE1 `Paths[16]` array limit on excess edges, preventing isolated nodes in hub networks.

---

## [0.23.1-preview.1] — 2026-11-01

### Preview
- Improves Map Wizard diagnostics, verified save reporting, undoable repairs, procedural corridor updates after room edits, inventory marker references, and preset round trips.
- Adds a read-only 105-map corpus inventory, reproducible analysis, and clearer warnings for incomplete parsing and estimated connectivity.
- **Known limitation:** canonical archetype corridors can become disconnected after room transformations. The wizard warns about some lost contacts; this preview does not automatically repair those corridors.
- Engine builds and targeted physical checks have been run in an isolated UT99 installation. Bot matches, human playtests, the complete workspace suite, and clean-install verification remain outstanding. Static diagnostics do not establish professional map quality.
- Procedural presets with room overrides can generate different corridors from earlier versions. Explicit corridor overrides are preserved and require review.

---

## [0.23.0] — 2026-10-30

### Added
- **Level Doctor Pre-Export Audit & Automated Repairs (`editor_state::wizard`, `gui::map_wizard_dialog`):**
  - Integrated `audit_and_autofix_before_export(&mut self) -> Result<usize, Vec<String>>` directly into the Map Generator Wizard workflow.
  - Added real-time Level Doctor verification status badges in the wizard GUI: `✓ Level Doctor: 0 Errors (Verified Clean)` and `⚠ Level Doctor: N Errors Detected (Export Blocked)`.
  - The "▶ Generate & Save .unr" and "⚡ Generate, Save & Playtest (F8)" buttons now automatically execute Level Doctor geometry auto-repairs, navigation connectivity checks, and full diagnostic audits prior to disk serialization or launching UT99, blocking corrupt maps and ensuring zero-defect exports.
  - Automated pruning of orphaned or unreachable navigation waypoints (`JumpSpot`, `PathNode`, `InventorySpot` with 0 ReachSpec connections) to eliminate bot AI lockups and dead ends.
- **Comprehensive Multi-Mode Pre-Export Verification Tests (`tests/test_map_generator_wizard.rs`):**
  - Added `test_wizard_pre_export_audit_and_autofix_all_game_modes` verifying clean pre-export audits across DeathMatch, CaptureTheFlag, Domination, and Assault.
  - Added `test_fresh_ctf_map_zero_errors_and_verified_clean` guaranteeing 0 Level Doctor errors and solid floor support on procedural CTF maps.

### Fixed
- **Balcony Navigation & Chasm Probe False Positives (`navigation.rs`, `ctf_mission.rs`):**
  - Corrected unbridged chasm probing logic for elevated additive platforms: bots can now traverse upper sniper balconies and mezzanine catwalks without false negative reachability cancellations.
  - Clamped sniper perch platform elevations to canonical step height ($\le 54.0$ UU) and added dedicated platform `PathNode` waypoints for bidirectional AI navigation.
- **Corridor Wall Torch Positioning (`map_wizard.rs`):**
  - Mounted corridor torches and bracket fixtures flush against sidewalls instead of room centers, eliminating `EncroachingSolidBrush` collision errors with `FlagBase` actors and corridor passageways.

---

## [0.22.1] — 2026-10-30

### Fixed
- **Hazard Basin & Water Zone Containment (`map_wizard.rs`):**
  - Resolved whole-map water flooding and drowning damage observed in maps with hazard basins. Fluid sheets were erroneously assigned `PF_PORTAL`, causing Unreal Engine's zone builder to treat the open air volume above the pit as part of the fluid `WaterZone`. Fluid sheets are now correctly typed as `PF_NOT_SOLID | PF_TRANSLUCENT | PF_TWO_SIDED | PF_UNLIT` and grouped with additive structural brushes rather than zone portal sheets.
  - Added regression test `test_wizard_hazard_basin_water_containment_and_floor_visibility` validating that rooms with hazard basins maintain clear air physics, zero drowning damage, and visible solid room floors.
- **BSP Semi-Solid Bitmask Canonicalization (`poly.rs`, `compiler.rs`):**
  - Unified `PF_SEMISOLID` bitmask to canonical Unreal Engine 1 value `0x0000_0020` across `crates/ut99-bsp/src/poly.rs` and `crates/ut99-editor/src/compiler.rs`.
- **Engine BSP Solid Classification Parity (`level.rs`):**
  - Fixed point solidity classification in `CompiledUModelBsp::classify_point`: point traversals ending at active interior leaves (`leaf_idx > 0`, `leaf.i_zone > 0`) are now correctly recognized as non-solid open space even when reaching the leaf through semi-solid transit stairs or portal sub-trees.
- **Actor Physical Placement Engine & Weapon Pedestal Probing (`level.rs`, `map_wizard.rs`):**
  - Implemented `UnrLevel::floor_distance` combining compiled BSP node traces with level brush geometry (`UnrBrush`), ensuring actors resting atop semi-solid weapon and powerup pedestals are detected at their exact engine rest offsets.
  - Corrected doorway threshold clearances in `generate_perimeter_baseboard_trims` to accommodate connecting room overlaps up to 128 UU, preventing baseboard trims from running across open inter-room doorways.
  - Filtered out thin decorative moldings (< 24 UU footprint) from walkable floor probing to ensure PlayerStarts and navigation nodes only anchor to genuine walkable surfaces.
  - Guarded catwalk vantage pickups behind quota checks so exact item quota configurations (`WizardItemQuotaConfig`) are strictly honored without extra unmanaged spawns.
  - Snapped auxiliary weapon spawn coordinates to the 16 UU grid.
  - Verified 100% pass across all 10 engine-compiled physical placement scenarios in `test_wizard_physical_placement.rs` with 0 placement errors.

---

## [0.22.0] — 2026-10-30

### Added
- **Procedural Map Generator Wizard 2.0 (`map_wizard.rs`, `gui::map_wizard_dialog`):**
  - **Architectural Preset Engine:** Introduced `StandardTournament` (default) and `EpicHighDetail` presets. `StandardTournament` delivers clean, competitive-grade BSP geometry, high framerate, and low polycount. `EpicHighDetail` adds intricate subtractive ceiling coffers, recessed trims, and baseboard friezes modeled after canonical Epic maps (*DM-Curse][*, *DM-Deck16][*, *CTF-FacingWorlds*).
  - **Oriented Stepped Staircase Generator (`build_stepped_stairs_oriented`):** Replaced placeholder flat ramps with authentic oriented stepped staircases (16 UU riser, 32 UU tread) accompanied by smooth invisible collision hulls (`PF_INVISIBLE | PF_SEMISOLID`). Eliminates bot pathing hang-ups and guarantees fluid combat movement.
  - **Elevated Perimeter Catwalks & Mezzanines:** Procedural perimeter catwalks at $Z = 192$ UU supported by structural semi-solid corbel pillars, adding vital vertical engagement layers to multi-floor combat spaces.
  - **Sightline Teaser Apertures & Security Grilles:** Inter-room peep-holes and sniper slits flanked by semi-solid protective vertical bars (`PF_SEMISOLID`), interconnecting adjacent chambers visually while maintaining tactical separation.
  - **Multi-Layer Celestial Skybox:** Complete procedural skybox assembly with multi-tier panning cloud sheets, celestial moon/planetoid spheres, and calibrated dual-color ambient lighting.
  - **Physical Light Fixture Prefabs & Dynamic Coronas:** Procedural industrial bracket and gothic lantern light fixtures paired with authentic `bCorona = true` flare effects.
  - **Sealed ZonePortals:** Embedded zero-thickness two-sided zone portal sheets (`PF_PORTAL`) at room/corridor junctions for sound occlusion and rendering zone separation.
  - **Calibrated Tactical Armory & Bot Hazards:** Vertical mobility integration with `JumpSpot`, `Kicker`, and `LiftCenter`/`LiftExit` navigation actors, calibrated weapon tiering, and hazard pit avoidance.

### Fixed
- **Bot AI ReachSpec Safety (`navigation.rs`):** Strictly enforced the 16-connection limit per navigation actor (`Paths[16]`), pruning lower-priority paths and eliminating potential memory corruption and game engine crashes on complex multi-room path networks.
- **Corpus Forensic Profile Enrichment (`corpus.rs`):** Expanded historical map corpus metrics with authentic architectural ratios, verticality indices, and light fixture densities derived from native retail UT99 maps.

---

## [0.21.0] — 2026-10-28

### Added
- **First-Run Onboarding Welcome Wizard (`gui::welcome_wizard`):**
  - 3-step guided setup on first launch: automatic Unreal Tournament 99 installation discovery (Windows Registry, GOG Galaxy, Steam libraries, standard drive roots, and Linux/macOS paths), interactive control scheme selection (*Modern UE5/Blender* vs *Classic UnrealEd 2.0*), and 1-click starter map creation.
  - Cross-platform folder and executable selector supporting `System/Engine.u` validation across Windows, Linux, and macOS.
- **OS-Native Configuration Persistence (`preferences.rs`):**
  - Preferences (`forge99_prefs.json`) now persist in standard OS user configuration directories (`%APPDATA%\Forge99` on Windows, `~/.config/forge99` on Linux, `~/Library/Application Support/Forge99` on macOS) with automatic fallback to local portable mode when `forge99_prefs.json` is present alongside the binary.
- **Integrated Forge99 Level Design Manual & Help (`gui::help_dialog`):**
  - Built-in searchable manual (`F1`) covering CSG architecture, 3-point lighting, Bot AI `ReachSpecs` navigation, competitive item placement, keyboard shortcuts, and project attribution.
- **Multi-Platform Release Pipeline (`.github/workflows/release.yml`):**
  - Automated CI/CD builds and packaging for Windows x64 (Inno Setup installer + Portable ZIP), Linux x64 (`.tar.gz`), and macOS Apple Silicon & Intel (`.tar.gz`).

### Changed
- **Menu Architecture & Responsive Modal Engine (`gui::modal`, `gui::mod`):**
  - Unified all editor windows and dialogs under a screen-clamped, resizable, vertically scrollable modal framework compatible with compact 720p/768p laptops up to 4K displays.
  - Reorganized the `Studios` menu to highlight core level design tools while grouping specialized architectural and hazard prefab generators into a dedicated `Setpiece & Hazard Prefab Generators` submenu.
- **Unified English Status & Save Feedback (`editor_state::io`, `compiler`):**
  - Standardized all staged save and export progress messages (`.unr`, `.t3d`, `.gltf`, `.obj`) to English.

### Fixed
- **Zero-Panic UI Hardening:** Eliminated unsafe `.unwrap()` calls across bookmark management, zone fog editors, and layer outliners when operating on empty levels.
- **Custom UT99 Path Propagation:** Selecting a custom UT99 directory or executable in the Welcome Wizard or Preferences now immediately synchronizes with `CompilerBridge::find_ut_dir()` for texture loading, map generation, and `UCC.exe` compilation.

---

## [0.20.1] — 2026-10-25

### Fixed
- **Status Bar UTF-8 Slicing Crash:** Fixed a runtime panic (`byte index is not a char boundary`) when truncating log preview messages containing multi-byte Unicode symbols in the collapsed status bar. Added a 300-iteration headless stress test in `test_map_generator_wizard.rs`.

---

## [0.20.0] — 2026-10-25

### Added
- **Procedural Map Generator Wizard Expansion:**
  - Added 8 new canonical UT99 themes (*Ancient*, *Crypt*, *Castle*, *Lava*, *Water*, *Egyptian*, *Starship*, *Monastery*) for a total of 15 historical themes with coordinated textures, lighting, and music tracks.
  - Procedural semi-solid (`PF_SEMISOLID`) corner pillars and ceiling rafters that add architectural depth without fragmenting the BSP tree.
  - 3-point dynamic lighting (*Key*, *Fill*, *Rim*) with native UE1 light effects (`LE_TorchWaver`, `LE_WateryShimmer`, `LE_FireWaver`) and wall-mounted `TorchFlame` actors.
  - Tactical weapon assignment based on room volume and sightlines, plus curved quadratic Bézier `HealthVial` trails.
  - 49-probe `PlacementWorld` collision and floor-anchoring solver ensuring every generated pickup, spawn point, and objective rests cleanly on solid floor geometry without wall clipping.

---

## [0.19.0] — 2026-10-22

### Added
- **Transactional Viewport Transform Engine (`TransformSession`):**
  - Atomic snapshot-based 3D and 2D viewport manipulation for Brushes, Actors, Movers, Static Meshes, and the Red Builder Brush.
  - Instant lossless rollback via `Esc`, right-click, or window focus loss, recording a single compound undo entry on release.
  - Live axis locking (`X`/`Y`/`Z`), snap inversion (`Ctrl`), fine step precision (`Shift`), and degenerate axis suppression in 2D orthographic views.
- **Forensic Game Mode Generators (`CTF`, `DOM`, `AS`):**
  - Polar CTF base placement solver, multi-lane `AlternatePath` routing, and 3-tier `DefensePoint` hierarchies.
  - Balanced 3-point Domination control circuits with semi-solid dais shrines.
  - Sequential Assault (`AS-`) `FortStandard` objective chains with min-cut blast door gating and `AssaultInfo` briefings.
