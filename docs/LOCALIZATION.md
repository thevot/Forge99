# Forge99 Internationalization & Localization (i18n) Guide

Forge99 is designed with a multi-tiered localization engine ensuring that **all application elements (UI menus, dialogs, preferences, in-app documentation, shortcuts, and wizards) are fully localizable and translatable**.

The universal fallback language is **English (`en`)**. If any term or section is missing in a target language, the engine automatically and seamlessly falls back to English without crashes, missing labels, or blank elements.

---

## 1. Supported Fundamental Languages

Forge99 provides native, zero-latency compiled dictionaries for 10 core languages:

| Language | ISO Code | Native Name | Status | Role |
|---|---|---|---|---|
| **English** | `en` | English | ✅ 100% | **Master Default & Universal Fallback** |
| **Italiano** | `it` | Italiano | ✅ 100% | Full Native Support |
| **Deutsch** | `de` | Deutsch | ✅ Core UI & Menus | Full Native Support |
| **Français** | `fr` | Français | ✅ Core UI & Menus | Full Native Support |
| **Português** | `pt` | Português | ✅ Core UI & Menus | Full Native Support |
| **Español** | `es` | Español | ✅ Core UI & Menus | Full Native Support |
| **Русский** | `ru` | Русский | ✅ Core UI & Menus | Full Native Support |
| **Українська** | `uk` | Українська | ✅ Core UI & Menus | Full Native Support |
| **Polski** | `pl` | Polski | ✅ Core UI & Menus | Full Native Support |
| **Nederlands** | `nl` | Nederlands | ✅ Core UI & Menus | Full Native Support |

---

## 2. Localization Architecture

The translation engine operates on a guaranteed resolution pipeline:

```text
Request: tr(lang, key)
  │
  ├──► 1. Check Runtime Overrides (Custom JSON file loaded by user/modder)
  │        └── Found? Return custom translation
  │
  ├──► 2. Check Target Language Native Dictionary (e.g. translate_it, translate_de, etc.)
  │        └── Found? Return native translation
  │
  ├──► 3. Fallback to English Master Dictionary (fallback_en)
  │        └── Found? Return English text
  │
  └──► 4. Fail-Safe Fallback
           └── Return raw key string (avoids blank labels or crashes)
```

---

## 3. Translation Key Namespaces

Translation keys follow a hierarchical, dot-separated naming convention:

| Namespace | Scope / Description | Example Keys |
|---|---|---|
| `menu.*` | Main menu bar and submenus | `menu.file`, `menu.file.new`, `menu.edit.undo`, `menu.build.all` |
| `action.*` | Common UI buttons, dialog actions | `action.save_preferences`, `action.browse`, `action.close`, `action.ok` |
| `settings.*` | Preferences window titles and tabs | `settings.title`, `settings.tab.general`, `settings.tab.appearance` |
| `general.*` | General preferences tab | `general.language`, `general.default_author`, `general.ui_scale` |
| `appearance.*` | Appearance & theme preferences | `appearance.theme`, `appearance.thumbnail_size` |
| `game.*` | UT99 integration settings | `game.exe_path`, `game.presets`, `game.target_engine` |
| `paths.*` | Export and map directories | `paths.export_dir`, `paths.use_default_maps` |
| `grid.*` | Grid & rotational snapping | `grid.snap_spatial`, `grid.step`, `grid.gizmo_space` |
| `viewport.*` | Viewport & 3D rendering | `viewport.speed`, `viewport.fov`, `viewport.shading` |
| `autosave.*` | Auto-save & backups | `autosave.enable`, `autosave.interval` |
| `keymap.*` | Keyboard shortcut profiles | `keymap.profile`, `keymap.modern`, `keymap.classic` |
| `save.*` | Save & export modal dialog | `save.unr_title`, `save.success_title`, `save.failed_title` |
| `help.*` | Help Center & mapping documentation | `help.title`, `help.search_hint`, `help.tab.quick_start` |
| `welcome.*` | Onboarding & welcome wizard | `welcome.window_title`, `welcome.tagline`, `welcome.step1` |
| `shortcuts.*` | Shortcut cheatsheet categories | `shortcuts.cat.viewport`, `shortcuts.cat.csg` |
| `sidebar.*` | Left toolbar, editor modes & CSG panels | `sidebar.editor_mode`, `sidebar.builder_brush`, `sidebar.csg_actions` |
| `builder.*` | 3D Builder Brush primitive shapes | `builder.cube`, `builder.cylinder`, `builder.stairs`, `builder.arch` |
| `inspector.*` | Right inspector & multi-selection panel | `inspector.split`, `inspector.selection_properties`, `inspector.snap_floor` |
| `outliner.*` | Scene elements & layers outliner | `outliner.title`, `outliner.show_all`, `outliner.cat.brushes` |
| `status.*` | Status bar and toast notifications | `status.ready`, `status.saved`, `status.opened` |

---

## 4. Contributing Translations

### Option A: Modifying Built-in Dictionaries
All core string tables reside in [`crates/ut99-editor/src/i18n.rs`](../crates/ut99-editor/src/i18n.rs).
To add or refine a translation:
1. Locate the language function (e.g., `translate_pt`, `translate_ru`, `translate_nl`).
2. Add the key and translated string to the `match key` block.
3. Run the automated test suite to verify:
   ```bash
   cargo test --test test_settings_and_preferences
   ```

### Option B: Runtime JSON Language Packs
Translators can also provide or test custom translations without recompiling by supplying a standard JSON dictionary:

```json
{
  "menu.file": "Bestand",
  "menu.file.new": "Nieuwe kaart",
  "action.save_preferences": "Voorkeuren opslaan",
  "help.title": "Handleiding & Helpcentrum"
}
```

These can be loaded at runtime using `ut99_editor::i18n::register_runtime_translations(lang, json_string)`.
