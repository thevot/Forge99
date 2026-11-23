//! Integration & Regression Test Suite: Help & Documentation Center + Tooltips (DOC-TOOLTIP-20261026)
//!
//! Validates:
//! 1. HelpTab enumeration, titles and default state.
//! 2. State management for opening and switching help tabs.
//! 3. Escape key dismissal priority for the help center.
//! 4. Completeness and validity of the SHORTCUTS registry.
//! 5. Documentation search filtering for keywords across topics and shortcuts.

use ut99_editor::editor_state::{EditorState, HelpTab};
use ut99_editor::gui::shortcut_help_dialog::SHORTCUTS;

#[test]
fn test_help_tab_enum_and_defaults() {
    let mut state = EditorState::default();

    // Default state: dialog closed, default tab is QuickStart, search empty
    assert!(!state.show_help_dialog);
    assert!(!state.show_shortcut_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::QuickStart);
    assert!(state.help_search_query.is_empty());

    // Test title property of each tab
    assert!(HelpTab::QuickStart.title().contains("Quick Start"));
    assert!(HelpTab::Shortcuts.title().contains("Shortcuts"));
    assert!(HelpTab::CsgGeometry.title().contains("CSG"));
    assert!(HelpTab::Lighting.title().contains("Lighting"));
    assert!(HelpTab::MoversAndTriggers.title().contains("Movers"));
    assert!(HelpTab::ZonesAndAtmosphere.title().contains("Zones"));
    assert!(HelpTab::SurfacesAndMaterials.title().contains("Surfaces"));
    assert!(HelpTab::EventSystem.title().contains("Event"));
    assert!(HelpTab::BotNavigation.title().contains("Bot"));
    assert!(HelpTab::GameModes.title().contains("Game Modes"));
    assert!(HelpTab::StudiosAndTools.title().contains("Studios"));
    assert!(HelpTab::UnrealEdGotchas.title().contains("Gotchas"));
    assert!(HelpTab::About.title().contains("About"));

    // Open specific tabs via open_help_tab helper
    state.open_help_tab(HelpTab::CsgGeometry);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::CsgGeometry);

    state.open_help_tab(HelpTab::MoversAndTriggers);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::MoversAndTriggers);

    state.open_help_tab(HelpTab::ZonesAndAtmosphere);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::ZonesAndAtmosphere);

    state.open_help_tab(HelpTab::SurfacesAndMaterials);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::SurfacesAndMaterials);

    state.open_help_tab(HelpTab::EventSystem);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::EventSystem);

    state.open_help_tab(HelpTab::StudiosAndTools);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::StudiosAndTools);

    state.open_help_tab(HelpTab::UnrealEdGotchas);
    assert!(state.show_help_dialog);
    assert_eq!(state.help_active_tab, HelpTab::UnrealEdGotchas);
}

#[test]
fn test_escape_key_closes_help_dialog() {
    let mut state = EditorState::default();

    state.open_help_tab(HelpTab::QuickStart);
    assert!(state.show_help_dialog);

    // Escape should close help dialog and return true (handled)
    let handled = state.handle_escape();
    assert!(handled);
    assert!(!state.show_help_dialog);
}

#[test]
fn test_shortcuts_registry_validity() {
    assert!(!SHORTCUTS.is_empty(), "SHORTCUTS registry must not be empty");

    let expected_categories = [
        "Viewport & Camera",
        "Shading & Diagnostics",
        "Selection & Transform",
        "Editing & History",
        "Surfaces & Texturing",
        "CSG & Brush Geometry",
        "System & Windows",
    ];

    for cat in expected_categories {
        let count = SHORTCUTS.iter().filter(|s| s.category == cat).count();
        assert!(count > 0, "Category '{}' must have registered shortcuts", cat);
    }

    for entry in SHORTCUTS {
        assert!(!entry.title.is_empty(), "Shortcut title must not be empty");
        assert!(!entry.keys.is_empty(), "Shortcut keys must not be empty for '{}'", entry.title);
        assert!(!entry.description.is_empty(), "Shortcut description must not be empty for '{}'", entry.title);
    }
}

#[test]
fn test_documentation_search_filtering() {
    // Test fuzzy keyword filtering on shortcuts
    let query_csg = "csg";
    let matches_csg: Vec<_> = SHORTCUTS
        .iter()
        .filter(|e| {
            e.title.to_lowercase().contains(query_csg)
                || e.category.to_lowercase().contains(query_csg)
                || e.description.to_lowercase().contains(query_csg)
        })
        .collect();

    assert!(!matches_csg.is_empty(), "Search for 'csg' should find multiple shortcuts");

    let query_fly = "fly";
    let matches_fly: Vec<_> = SHORTCUTS
        .iter()
        .filter(|e| {
            e.title.to_lowercase().contains(query_fly)
                || e.description.to_lowercase().contains(query_fly)
        })
        .collect();

    assert!(!matches_fly.is_empty(), "Search for 'fly' should find flight navigation");

    let query_wasd = "wasd";
    let query_clean = query_wasd.replace(' ', "").replace('/', "");
    let matches_wasd: Vec<_> = SHORTCUTS
        .iter()
        .filter(|e| {
            e.keys.iter().any(|k| {
                let kl = k.to_lowercase();
                kl.contains(query_wasd) || kl.replace(' ', "").replace('/', "").contains(&query_clean)
            })
        })
        .collect();

    assert!(!matches_wasd.is_empty(), "Search for 'wasd' key should find flight navigation");
}

#[test]
fn test_documentation_topics_coverage_and_tab_switching() {
    let mut state = EditorState::default();

    let all_tabs = [
        HelpTab::QuickStart,
        HelpTab::Shortcuts,
        HelpTab::CsgGeometry,
        HelpTab::Lighting,
        HelpTab::MoversAndTriggers,
        HelpTab::ZonesAndAtmosphere,
        HelpTab::SurfacesAndMaterials,
        HelpTab::EventSystem,
        HelpTab::BotNavigation,
        HelpTab::GameModes,
        HelpTab::StudiosAndTools,
        HelpTab::UnrealEdGotchas,
        HelpTab::About,
    ];

    for tab in all_tabs {
        state.open_help_tab(tab);
        assert!(state.show_help_dialog);
        assert_eq!(state.help_active_tab, tab);
        assert!(!tab.title().is_empty());
    }
}
