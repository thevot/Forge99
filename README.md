# Forge99 — Modern 64-Bit Level Editor for Unreal Tournament 99

[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-v0.25.0-blue.svg)](Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](LICENSE-MIT)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-informational.svg)](#installation--quick-start)
[![Sponsor](https://img.shields.io/badge/Sponsor-%40thevot-ea4aaa?logo=githubsponsors)](https://github.com/sponsors/thevot)

> **Release v0.25.0:** First open-source release of Forge99, featuring adaptive spatial chunking and frustum culling, BVH-accelerated CSG and navigation queries, hardware-adaptive 3D viewport frame pacing, opt-in Map Wizard construction rules, and comprehensive editor stability fixes. See [CHANGELOG.md](CHANGELOG.md) for full details.

**Forge99** is a native 64-bit 3D level editor and procedural arena generator written in Rust for **Unreal Tournament 99** (`v436` / `v469`) and **Unreal Engine 1** (`v227`).

Designed as a modern, cross-platform successor to the legacy 32-bit *UnrealEd 2.0*, Forge99 combines a hardware-accelerated **WGPU** viewport (DirectX 12, Vulkan, Metal), a chronological polyhedral **CSG** and **BSP** compiler, native binary `.unr` package serialization, and an interactive **Map Generator Wizard** for creating levels to inspect and test.

---

## Why Forge99?

| Capability | Classic UnrealEd 2.0 (1999) | **Forge99 preview** |
| :--- | :--- | :--- |
| **Architecture & Stability** | 32-bit Windows MFC/VB wrapper; frequent GPF crashes | **64-bit Memory-Safe Rust** with automatic crash-recovery snapshots |
| **Operating Systems** | Windows only (requires Wine on Linux/macOS) | **Native Windows, Linux (X11/Wayland), and macOS (Apple Silicon & Intel)** |
| **3D Viewport Renderer** | Legacy D3D7 / Software rendering | **60 FPS WGPU 24** (DirectX 12 / Vulkan / Metal) with 7 diagnostic shading modes |
| **3D Manipulation** | Prone to floating-point vertex drift over repeated transforms | **Transactional `TransformSession`** with snapshot math (`Esc`/RMB rollback, zero cumulative drift) |
| **Map File Pipeline** | Coupled to local 32-bit `Engine.dll` | **Dual Pipeline:** Native 64-bit `.unr` binary read/write + optional `UCC.exe` headless build bridge |
| **Controls & Ergonomics** | Fixed legacy shortcuts | **Switchable Keymap Profiles:** *Modern (UE5 / Blender)* or *Classic UnrealEd 2.0* |
| **Rapid Prototyping** | Manual room carving from scratch | **Procedural Map Wizard & 1-Click Mystery Map** (`DM`, `CTF`, `DOM`, `AS`) |

---

## Key Features

### 1. First-Run Onboarding & Zero-Friction Setup
- **Automatic UT99 Discovery:** Scans Windows Registry (Retail, Unreal Anthology, GOG Galaxy), Steam libraries, standard drive paths, and Unix/Wine directories to locate your `UnrealTournament` installation automatically.
- **Standalone / Offline Fallback:** Works out of the box even without UT99 installed, using built-in procedural fallback textures and the native 64-bit `.unr` / `.t3d` serializer.
- **Interactive Welcome Wizard:** Configures your UT99 path, preferred control scheme (*Modern* vs *Classic UnrealEd 2.0*), and starter workspace in under 30 seconds.

### 2. Chronological CSG, BSP & Parallel Raytraced Lighting
- **Polyhedral CSG Boolean Engine:** Authentic `CSG_Add`, `CSG_Subtract`, `CSG_Intersect`, and `CSG_Deintersect` operations with support for `PF_SEMISOLID` detail brushes and non-solid zone portals.
- **2D Shape Editor & Brush Builders:** Concave polygon triangulation (Ear-Clipping), cubic Bézier curves, `.2ds` / `.obj` / `.dxf` / `.gltf` import/export, and parametric builders (Cube, Cylinder, Spiral Staircase, Tarquin Torus, Heightmap Terrain).
- **Multi-Threaded Compilation (`Rayon`):** BSP tree construction, Potentially Visible Set (PVS) zone connectivity, multi-bounce static lightmap raytracing (`LE_Spotlight`, `LE_TorchWaver`, `LE_WateryShimmer`), and `ReachSpecs` bot navigation graph generation.

### 3. Hardware-Accelerated 3D & 2D Viewports (`WGPU 24`)
- **Synchronized Quad & Single Layouts:** Instant `Space` toggle between single viewport and 2x2 quad view (3D Perspective + Top XY, Front XZ, Side YZ orthographic views) with High-DPI accuracy.
- **7 Diagnostic Shading Modes (`Alt+1..Alt+7`):** Wireframe, Zone/Portal coloring, Unlit, Real-Time Lit, Depth Complexity Heatmap, Texture Usage Color-Coding, and BSP Cuts Visualizer.
- **Precision Transform Gizmos:** Translate, Rotate, and Scale gizmos in World or Local space with live axis locking (`X`/`Y`/`Z`), grid/angle snap inversion (`Ctrl`), fine step precision (`Shift`), and Unreal Engine world-bound warnings ($\pm 65,536$ UU).

### 4. Procedural Map Generator Wizard & 1-Click Mystery Map
- **Game-mode generation (preview):**
  - **DeathMatch (`DM-`):** Tactical weapon placement based on room volume and sightlines, curved Bézier health vial trails, and multi-tier vertical arenas.
  - **Capture The Flag (`CTF-`):** Polar base solver ensuring Euclidean separation ($d \ge 2400$ UU), symmetrical multi-lane `AlternatePath` routing, 3-tier `DefensePoint` bot hierarchy, sniper balconies, and contested central powerups.
  - **Domination (`DOM-`):** Topologically balanced 3-point control circuits with themed point naming, monumental semi-solid shrines, and neutral spawn standoff guarantees ($d \ge 480$ UU).
  - **Assault (`AS-`):** Sequential `FortStandard` objective chains (Destroyable targets, Touch switches, Shootable triggers, Infiltration zones) with automated min-cut blast door gating (`Mover` + `SpecialEvent` broadcasts) and `AssaultInfo` mission briefings.
- **15 Canonical UT99 Themes:** Coordinated texture palettes, 3-point atmospheric lighting (`Key`/`Fill`/`Rim` + `TorchFlame` sconces), and level music across *Industrial*, *Gothic*, *Egyptian*, *Castle*, *Starship*, *Ancient*, *Lava*, *Crypt*, *Water*, and more.

### 5. Native Package Browsers, Diagnostics & Packaging
- **Integrated Asset Browsers:** Native parsers for `.utx` textures (Indexed8 mipmaps + S3TC), `.uax` sounds (with built-in WAV preview), `.umx` tracker music, and `.u` UnrealScript classes and 3D meshes (`UMesh` / `ULodMesh`).
- **Level Doctor & Performance Profiler (`F7`):** Automated integrity checks for BSP holes, unsealed zones, missing `PlayerStart` actors, isolated `PathNodes`, and competitive item balance (`F12`).
- **Map Packaging Studio:** Scans custom vs. stock package dependencies, generates installation README manifests, and exports distribution-ready `.unr` / `.t3d` bundles.

---

## Installation & Quick Start

### Option A: Download Prebuilt Binaries (Recommended)
Download the latest release for your platform from the **[GitHub Releases](https://github.com/thevot/Forge99/releases)** page:
- **Windows (x64):** `forge99-v0.25.0-windows-x64-setup.exe` (Installer with `.unr`/`.t3d` file associations) or `forge99-v0.25.0-windows-x64.zip` (Portable).
- **Linux (x64):** `forge99-v0.25.0-x86_64-unknown-linux-gnu.tar.gz`
- **macOS (Apple Silicon & Intel):** `forge99-v0.25.0-aarch64-apple-darwin.tar.gz` / `forge99-v0.25.0-x86_64-apple-darwin.tar.gz`

### Option B: Build from Source
Requires **Rust 1.80+** (2021 edition) and a GPU supporting Vulkan 1.2+, DirectX 12, or Metal.

On Ubuntu/Debian Linux, install the standard windowing headers first:
```bash
sudo apt-get update && sudo apt-get install -y libasound2-dev libudev-dev libx11-xcb-dev libxkbcommon-dev libwayland-dev libgtk-3-dev pkg-config
```

Clone and run in release mode:
```bash
git clone https://github.com/thevot/Forge99.git
cd Forge99
cargo run --release --bin forge99
```

---

## Workspace Architecture

Forge99 is structured as a modular 3-crate Cargo workspace:

```text
Forge99/
├── crates/
│   ├── ut99-format/   # Zero-copy binary parser & serializer for UE1 packages (.unr, .utx, .uax, .umx, .u) and .t3d
│   ├── ut99-bsp/      # Polyhedral CSG boolean engine, BSP compiler, raytraced lightmaps, bot pathing & procedural builders
│   └── ut99-editor/   # Native 64-bit desktop application (wgpu 24 renderer, egui 0.31 UI, viewports, gizmos & studios)
├── docs/              # Localization and technical documentation
├── installer/         # Inno Setup installer configuration for Windows
├── scripts/           # Release packaging scripts
└── Cargo.toml         # Workspace manifest
```

### Running the Test Suite
```bash
cargo test --workspace
```

---

## Privacy & Security

- **100% Offline & Zero Telemetry:** Forge99 runs entirely locally on your machine. It collects no analytics, sends no background network requests, and never modifies your game installation without explicit user action.
- **Non-Destructive Saving:** Saving over an existing `.unr` map automatically creates a `.unr.bak` backup alongside rolling session snapshots.

---

## Support & Sponsorship

**Forge99** is conceived, architected, and maintained by **Nicola "thevot" Selenu** ([@thevot](https://github.com/thevot)) and released as free and open-source software.

If Forge99 saves you time, brings back great UT99 memories, or helps you build new arenas for the community, please consider supporting its ongoing development through **GitHub Sponsors**:

👉 **[Sponsor @thevot on GitHub Sponsors](https://github.com/sponsors/thevot)**

Every contribution directly supports cross-platform maintenance, new procedural layout archetypes, and long-term Unreal Engine 1 preservation.

---

## License & Legal Notice

Licensed under either of:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

> *Unreal® and Unreal Tournament® are registered trademarks of Epic Games, Inc. Forge99 is an independent, community-driven open-source project and is not affiliated with, endorsed, or sponsored by Epic Games, Inc. No proprietary game assets are distributed with this software.*
