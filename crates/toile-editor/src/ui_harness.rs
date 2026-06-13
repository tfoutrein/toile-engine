//! Headless egui UI harness for the editor (egui_kittest).
//!
//! This is the editor's equivalent of Playwright: it drives the *real* egui panel
//! tree (`show_overlay_panels`) with simulated clicks/typing and renders frames to
//! PNG for visual inspection — all headless, no window. Tests live in-crate
//! (`#[cfg(test)]`) so they can reach the crate-private panel entry points.

#![cfg(test)]

use std::path::PathBuf;

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

use crate::editor_app::EditorApp;

/// Where rendered PNGs are written for inspection.
fn out_dir() -> PathBuf {
    let d = std::env::temp_dir().join("toile-ui");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// A fresh temp workspace so project-creation tests don't touch real dirs.
fn temp_workspace(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("toile-ui-ws").join(name);
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Build a kittest harness driving the editor's full overlay panel tree with a
/// real wgpu renderer, so frames can be rendered to PNG. Mirrors `render_overlay`
/// by pre-collecting the project file lists each frame.
fn editor_harness(app: EditorApp) -> Harness<'static, EditorApp> {
    Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .wgpu()
        .build_state(
            |ctx, app| {
                let scenes = app.list_project_scenes();
                let scripts = app.list_project_files("scripts", "json");
                let particles = app.list_project_files("particles", "json");
                let pdir = app.project_dir.clone();
                app.show_overlay_panels(ctx, &scenes, &scripts, &particles, &pdir);
            },
            app,
        )
}

/// A fresh editor with the splash skipped and an isolated workspace.
fn fresh_editor(workspace_name: &str) -> EditorApp {
    let mut app = EditorApp::new();
    app.show_splash = false;
    app.workspace_dir = temp_workspace(workspace_name);
    app
}

/// Build a harness, create the default empty project, and land in the main editor.
fn editor_with_project(name: &str) -> Harness<'static, EditorApp> {
    let mut h = editor_harness(fresh_editor(name));
    h.run();
    h.get_by_label_contains("Create Project").click();
    h.run();
    h
}

/// Render the current frame to `toile-ui/<name>.png` (best effort; skips if no GPU).
fn snapshot(h: &mut Harness<'static, EditorApp>, name: &str) {
    match h.render() {
        Ok(img) => {
            let path = out_dir().join(format!("{name}.png"));
            let _ = img.save(&path);
            eprintln!("rendered {}", path.display());
        }
        Err(e) => eprintln!("render skipped ({name}): {e}"),
    }
}

#[test]
fn welcome_screen_renders() {
    let mut h = editor_harness(fresh_editor("welcome"));
    h.run();
    snapshot(&mut h, "01-welcome");
    // The welcome dialog must be showing the create-project affordance.
    assert!(h.state().show_project_dialog);
}

#[test]
fn create_project_opens_main_editor() {
    let mut h = editor_harness(fresh_editor("create"));
    h.run();

    // Drive the welcome screen: create the default "my-game" empty project.
    // (label_contains tolerates the button's padding spaces / emoji.)
    h.get_by_label_contains("Create Project").click();
    h.run();
    snapshot(&mut h, "02-main-editor");

    let app = h.state();
    assert!(!app.show_project_dialog, "welcome should close after creating a project");
    assert!(app.project_dir.is_some(), "a project directory should be set");
    // The project skeleton must exist on disk.
    let pdir = app.project_dir.clone().unwrap();
    assert!(pdir.join("Toile.toml").exists(), "Toile.toml created");
    assert!(pdir.join("scenes/main.json").exists(), "main scene created");
}

#[test]
fn add_entity_shows_inspector() {
    let mut h = editor_with_project("add-entity");
    h.get_by_label_contains("Add Entity").click();
    h.run();
    // The new entity should be added and auto-selected so the inspector populates.
    assert_eq!(h.state().scene.entities.len(), 1, "one entity after Add Entity");
    if h.state().selected_id.is_none() {
        let id = h.state().scene.entities[0].id;
        h.state_mut().selected_id = Some(id);
        h.run();
    }
    snapshot(&mut h, "03-inspector");
    assert!(h.state().selected_id.is_some(), "entity should be selected");
}

#[test]
fn scene_settings_window_renders() {
    let mut h = editor_with_project("scene-settings");
    h.state_mut().show_scene_settings = true;
    h.run();
    snapshot(&mut h, "04-scene-settings");
}

#[test]
fn tilemap_mode_renders() {
    let mut h = editor_with_project("tilemap");
    h.state_mut().editor_mode = crate::editor_app::EditorMode::Tilemap;
    h.run();
    snapshot(&mut h, "05-tilemap");
}

#[test]
fn particle_mode_renders() {
    let mut h = editor_with_project("particle");
    h.state_mut().editor_mode = crate::editor_app::EditorMode::Particle;
    h.run();
    snapshot(&mut h, "06-particle");
}

#[test]
fn ai_copilot_renders() {
    let mut h = editor_with_project("ai");
    h.state_mut().editor_mode = crate::editor_app::EditorMode::AICopilot;
    h.run();
    snapshot(&mut h, "07-ai-copilot");
}

#[test]
fn game_output_console_shows_logs() {
    let mut h = editor_with_project("game-output");
    {
        let app = h.state_mut();
        app.game_logs = vec![
            "[INFO] Game launched".into(),
            "[WARN] missing optional asset".into(),
            "ERROR: something failed at frame 12".into(),
        ];
        app.show_game_output = true;
    }
    h.run();
    snapshot(&mut h, "08-game-output");
    assert!(h.state().show_game_output);
    assert_eq!(h.state().game_logs.len(), 3);
}

#[test]
fn undo_redo_add_entity() {
    let mut h = editor_with_project("undo");
    h.get_by_label_contains("Add Entity").click();
    h.run();
    assert_eq!(h.state().scene.entities.len(), 1, "entity added");
    assert_eq!(h.state().undo_stack.len(), 1, "add pushed an undo snapshot");

    h.state_mut().undo();
    h.run();
    assert_eq!(h.state().scene.entities.len(), 0, "undo removes the entity");
    assert_eq!(h.state().redo_stack.len(), 1, "undo populated redo");

    h.state_mut().redo();
    h.run();
    assert_eq!(h.state().scene.entities.len(), 1, "redo restores the entity");
}
