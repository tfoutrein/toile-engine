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
