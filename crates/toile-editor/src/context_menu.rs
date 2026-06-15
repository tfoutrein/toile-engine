//! Intelligent, context-adaptive right-click menus (ADR-037).
//!
//! Two render paths, one vocabulary of items, one point of mutation:
//! - egui-widget surfaces (hierarchy, inspector, browser…) use `response.context_menu`
//!   directly (added in later phases).
//! - the central VIEWPORT is raw wgpu (no egui `Response` — there is no `CentralPanel`
//!   when a project is open), so its right-click is captured winit-side in `input.rs`
//!   and the menu is drawn here as an `egui::Area` at the cursor.
//!
//! Because the menu closure borrows the cloned `&egui::Context`, items may NOT mutate
//! `self`: each item only sets a flag in [`ContextMenuActions`]; everything is applied
//! afterwards in [`EditorApp::apply_context_actions`] (with `push_undo` before mutation).
//! This mirrors the existing `pending_add_to_scene` idiom.

use glam::Vec2;

use crate::editor_app::EditorApp;

/// What the cursor is over when the viewport right-click fires. `world` is the
/// world-space cursor position (used for "paste/add here" and as the menu anchor seed).
#[derive(Clone, Copy)]
pub(crate) enum ContextMenuKind {
    Entity { id: u64, world: Vec2 },
    Viewport { world: Vec2 },
}

/// Deferred actions posted by menu items, applied after the egui block.
#[derive(Default)]
pub(crate) struct ContextMenuActions {
    pub copy: Option<u64>,
    pub cut: Option<u64>,
    pub paste_at: Option<Vec2>,
    pub duplicate: Option<u64>,
    pub delete: Option<u64>,
    pub rename: Option<u64>,
    pub focus_camera: Option<u64>,
    pub toggle_visibility: Option<u64>,
    pub add_entity_at: Option<Vec2>,
    pub reset_camera: bool,
    pub toggle_grid: bool,
    pub copy_text: Option<String>,
    /// Set by any item when clicked → closes the menu.
    pub close: bool,
}

/// Platform-correct modifier label for shortcut hints ("Cmd" on macOS, else "Ctrl").
pub(crate) fn m_label() -> &'static str {
    if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" }
}

fn menu_btn(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    ui.add(egui::Button::new(label).shortcut_text(shortcut)).clicked()
}

impl EditorApp {
    /// Rotation-aware top-most entity under a world position (shared by selection + menu).
    pub(crate) fn hit_test_entity(&self, world: Vec2) -> Option<u64> {
        for entity in self.scene.entities.iter().rev() {
            let hw = entity.width * entity.scale_x * 0.5;
            let hh = entity.height * entity.scale_y * 0.5;
            // Transform the world point into the entity's local space (undo rotation).
            let d = world - Vec2::new(entity.x, entity.y);
            let (sin, cos) = (-entity.rotation).sin_cos();
            let local = Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
            if local.x >= -hw && local.x <= hw && local.y >= -hh && local.y <= hh {
                return Some(entity.id);
            }
        }
        None
    }

    /// Items for a right-click on an entity (read-only: only posts flags).
    pub(crate) fn entity_menu_items(
        &self,
        ui: &mut egui::Ui,
        id: u64,
        world: Vec2,
        a: &mut ContextMenuActions,
    ) {
        ui.set_min_width(190.0);
        let m = m_label();
        let entity = self.scene.entities.iter().find(|e| e.id == id);
        if let Some(e) = entity {
            ui.label(egui::RichText::new(&e.name).strong());
            ui.separator();
        }

        if menu_btn(ui, "Copy", &format!("{m}+C")) { a.copy = Some(id); a.close = true; }
        if menu_btn(ui, "Cut", &format!("{m}+X")) { a.cut = Some(id); a.close = true; }
        if menu_btn(ui, "Duplicate", &format!("{m}+D")) { a.duplicate = Some(id); a.close = true; }
        let can_paste = self.clipboard_entity.is_some();
        if ui.add_enabled(can_paste, egui::Button::new("Paste Here")).clicked() {
            a.paste_at = Some(world);
            a.close = true;
        }

        ui.separator();
        if ui.button("Rename…").clicked() { a.rename = Some(id); a.close = true; }
        let visible = entity.map(|e| e.visible).unwrap_or(true);
        if ui.button(if visible { "Hide" } else { "Show" }).clicked() {
            a.toggle_visibility = Some(id);
            a.close = true;
        }
        if ui.button("Focus Camera").clicked() { a.focus_camera = Some(id); a.close = true; }
        if ui.button("Copy Entity ID").clicked() { a.copy_text = Some(id.to_string()); a.close = true; }

        ui.separator();
        let del = egui::Button::new(
            egui::RichText::new("Delete").color(egui::Color32::from_rgb(220, 90, 90)),
        )
        .shortcut_text("Del");
        if ui.add(del).clicked() { a.delete = Some(id); a.close = true; }
    }

    /// Items for a right-click on empty viewport space.
    pub(crate) fn viewport_menu_items(
        &self,
        ui: &mut egui::Ui,
        world: Vec2,
        a: &mut ContextMenuActions,
    ) {
        ui.set_min_width(190.0);
        if ui.button("Add Entity Here").clicked() { a.add_entity_at = Some(world); a.close = true; }
        let can_paste = self.clipboard_entity.is_some();
        if ui.add_enabled(can_paste, egui::Button::new("Paste Here")).clicked() {
            a.paste_at = Some(world);
            a.close = true;
        }

        ui.separator();
        let camera_moved = self.camera_pos != Vec2::ZERO || (self.camera_zoom - 1.0).abs() > 1e-3;
        if ui.add_enabled(camera_moved, egui::Button::new("Reset Camera")).clicked() {
            a.reset_camera = true;
            a.close = true;
        }
        let mut grid = self.show_grid;
        if ui.checkbox(&mut grid, "Show Grid").changed() { a.toggle_grid = true; a.close = true; }
    }

    /// Render the viewport context menu (if open) as an egui Area at the cursor, then
    /// apply whatever items were clicked. Called at the end of `render_overlay`.
    pub(crate) fn show_viewport_context_menu(&mut self, ctx: &egui::Context) {
        let kind = match self.pending_context_menu {
            Some(k) => k,
            None => return,
        };
        // Capture the anchor once (in egui points), so the menu stays put if the mouse moves.
        if self.context_menu_anchor.is_none() {
            let pos = ctx
                .pointer_latest_pos()
                .unwrap_or_else(|| ctx.screen_rect().center());
            self.context_menu_anchor = Some(pos);
        }
        let anchor = self.context_menu_anchor.unwrap();

        let mut actions = ContextMenuActions::default();
        let this = &*self;
        let area = egui::Area::new(egui::Id::new("viewport_context_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                let style = ui.style().clone();
                egui::Frame::menu(&style).show(ui, |ui| match kind {
                    ContextMenuKind::Entity { id, world } => {
                        this.entity_menu_items(ui, id, world, &mut actions)
                    }
                    ContextMenuKind::Viewport { world } => {
                        this.viewport_menu_items(ui, world, &mut actions)
                    }
                });
            });

        let menu_rect = area.response.rect;
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let clicked_outside = ctx.input(|i| i.pointer.any_click())
            && ctx
                .pointer_latest_pos()
                .map_or(true, |p| !menu_rect.contains(p));
        let close = actions.close || escape || clicked_outside;

        self.apply_context_actions(ctx, actions);
        if close {
            self.pending_context_menu = None;
            self.context_menu_anchor = None;
        }
    }

    /// Single point of mutation for all context-menu actions (push_undo before scene edits).
    pub(crate) fn apply_context_actions(&mut self, ctx: &egui::Context, a: ContextMenuActions) {
        if let Some(id) = a.copy {
            if let Some(e) = self.scene.entities.iter().find(|e| e.id == id) {
                self.clipboard_entity = Some(e.clone());
                self.status_msg = format!("Copied '{}'", e.name);
            }
        }
        if let Some(id) = a.cut {
            if let Some(e) = self.scene.entities.iter().find(|e| e.id == id).cloned() {
                self.push_undo();
                self.clipboard_entity = Some(e.clone());
                self.scene.remove_entity(id);
                if self.selected_id == Some(id) {
                    self.selected_id = None;
                }
                self.status_msg = format!("Cut '{}'", e.name);
            }
        }
        if let Some(world) = a.paste_at {
            if let Some(src) = self.clipboard_entity.clone() {
                self.push_undo();
                let id = self.scene.next_id;
                self.scene.next_id += 1;
                let mut ne = src.clone();
                ne.id = id;
                ne.name = format!("{}_copy", src.name);
                ne.x = world.x;
                ne.y = world.y;
                self.scene.entities.push(ne);
                self.selected_id = Some(id);
                self.status_msg = format!("Pasted '{}_copy'", src.name);
            }
        }
        if let Some(id) = a.duplicate {
            if let Some(src) = self.scene.entities.iter().find(|e| e.id == id).cloned() {
                self.push_undo();
                let nid = self.scene.next_id;
                self.scene.next_id += 1;
                let mut ne = src.clone();
                ne.id = nid;
                ne.name = format!("{}_dup", src.name);
                ne.x += 20.0;
                ne.y -= 20.0;
                self.scene.entities.push(ne);
                self.selected_id = Some(nid);
                self.status_msg = format!("Duplicated '{}'", src.name);
            }
        }
        if let Some(id) = a.delete {
            self.push_undo();
            self.scene.remove_entity(id);
            if self.selected_id == Some(id) {
                self.selected_id = None;
            }
            self.status_msg = format!("Deleted entity {id}");
        }
        if let Some(id) = a.rename {
            if let Some(e) = self.scene.entities.iter().find(|e| e.id == id) {
                self.hierarchy_rename = Some((id, e.name.clone()));
                self.hierarchy_rename_focus = true;
                self.selected_id = Some(id);
            }
        }
        if let Some(id) = a.focus_camera {
            if let Some(e) = self.scene.entities.iter().find(|e| e.id == id) {
                self.camera_pos = Vec2::new(e.x, e.y);
                self.status_msg = format!("Focused '{}'", e.name);
            }
        }
        if let Some(id) = a.toggle_visibility {
            self.push_undo();
            if let Some(e) = self.scene.find_entity_mut(id) {
                e.visible = !e.visible;
            }
        }
        if let Some(world) = a.add_entity_at {
            self.push_undo();
            let id = self.scene.add_entity("Entity", world.x, world.y);
            self.selected_id = Some(id);
            self.status_msg = "Added entity".to_string();
        }
        if a.reset_camera {
            self.camera_pos = Vec2::ZERO;
            self.camera_zoom = 1.0;
        }
        if a.toggle_grid {
            self.show_grid = !self.show_grid;
        }
        if let Some(text) = a.copy_text {
            ctx.copy_text(text);
            self.status_msg = "Copied to clipboard".to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toile_scene::SceneData;

    #[test]
    fn hit_test_picks_topmost_and_misses_empty_space() {
        let mut app = EditorApp::new();
        app.scene = SceneData::new("t");
        // Default entity size is 32 → half-extent 16.
        let a = app.scene.add_entity("a", 100.0, 0.0);
        let b = app.scene.add_entity("b", 100.0, 0.0); // overlaps a, added later → on top

        // Inside the overlapping box → top-most (last pushed) wins.
        assert_eq!(app.hit_test_entity(Vec2::new(100.0, 0.0)), Some(b));
        // Just inside the box edge.
        assert_eq!(app.hit_test_entity(Vec2::new(115.0, 10.0)), Some(b));
        // Far away → nothing.
        assert_eq!(app.hit_test_entity(Vec2::new(500.0, 500.0)), None);
        let _ = a;
    }

    #[test]
    fn hit_test_respects_rotation() {
        let mut app = EditorApp::new();
        app.scene = SceneData::new("t");
        let id = app.scene.add_entity("r", 0.0, 0.0);
        // Make it a thin wide box and rotate 90° so it becomes thin & tall.
        if let Some(e) = app.scene.find_entity_mut(id) {
            e.width = 80.0;
            e.height = 10.0;
            e.rotation = std::f32::consts::FRAC_PI_2;
        }
        // A point far along +y is inside only because of the 90° rotation.
        assert_eq!(app.hit_test_entity(Vec2::new(0.0, 35.0)), Some(id));
        // The same distance along +x is now outside (box is thin there).
        assert_eq!(app.hit_test_entity(Vec2::new(35.0, 0.0)), None);
    }
}
