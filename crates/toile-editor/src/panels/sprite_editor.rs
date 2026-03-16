use std::path::PathBuf;

use crate::editor_app::{EditorApp, EditorMode};
use crate::helpers::*;

impl EditorApp {
    /// Show the SpriteAnim full-screen mode panels (left import + right animation list).
    pub(crate) fn show_sprite_anim_panels(
        &mut self,
        ctx: &egui::Context,
        pdir: &Option<PathBuf>,
    ) {
        if self.editor_mode == EditorMode::SpriteAnim {
            if let Some(id) = self.selected_id {
                // Left panel — Import + Sprite Sheet config
                egui::SidePanel::left("sprite_anim_left").min_width(280.0).default_width(300.0).show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if ui.button("← Back to Editor").clicked() {
                            self.editor_mode = EditorMode::Entity;
                        }
                        ui.separator();

                        if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                            ui.label(egui::RichText::new(format!("Entity: {}", entity.name)).strong().size(14.0));
                        }
                        ui.add_space(8.0);

                        // Import buttons
                        ui.label(egui::RichText::new("Import").strong().size(13.0));
                        ui.separator();
                        if let Some(entity) = self.scene.entities.iter_mut().find(|e| e.id == id) {
                            if ui.button("🖼 Image (PNG/JPG) — static sprite").clicked() {
                                if let Some(file) = rfd::FileDialog::new().set_title("Select Sprite").add_filter("Images", &["png", "jpg", "jpeg", "bmp"]).pick_file() {
                                    entity.sprite_path = pdir.as_ref().and_then(|pd| file.strip_prefix(pd).ok().map(|p| p.to_string_lossy().to_string())).unwrap_or_else(|| file.to_string_lossy().to_string());
                                    entity.sprite_sheet = None;
                                    self.sprite_cache.clear();
                                }
                            }
                            if ui.button("✨ Aseprite (.ase) — auto animations").clicked() {
                                if let Some(file) = rfd::FileDialog::new().set_title("Import Aseprite").add_filter("Aseprite", &["aseprite", "ase"]).pick_file() {
                                    if let Ok(ase) = toile_assets::aseprite::load_ase_file(&file) {
                                        let (atlas_rgba, atlas_w, atlas_h, _) = toile_assets::aseprite::build_atlas(&ase);
                                        let fc = ase.frames.len() as u32;
                                        let stem = file.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                        let afn = format!("assets/{stem}_atlas.png");
                                        let afull = pdir.as_ref().map(|d| d.join(&afn)).unwrap_or_else(|| PathBuf::from(&afn));
                                        if let Some(p) = afull.parent() { let _ = std::fs::create_dir_all(p); }
                                        if let Some(img) = image::RgbaImage::from_raw(atlas_w, atlas_h, atlas_rgba) { let _ = img.save(&afull); }
                                        let guess_name = |s: &str| -> String {
                                            let n = s.to_lowercase();
                                            ["idle","run","walk","jump","die","dash","slide"].iter().find(|k| n.contains(**k)).map(|k| k.to_string()).unwrap_or_else(|| n.split('_').next().unwrap_or("anim").to_string())
                                        };
                                        if ase.tags.is_empty() {
                                            let name = guess_name(&stem);
                                            let avg = ase.frames.iter().map(|f| f.duration_ms as f32).sum::<f32>() / fc.max(1) as f32;
                                            let a = toile_scene::AnimationData { name: name.clone(), frames: (0..fc).collect(), fps: (1000.0/avg.max(1.0)).round(), looping: true, sprite_file: Some(afn.clone()), strip_frames: Some(fc) };
                                            if let Some(e) = entity.animations.iter_mut().find(|x| x.name == name) { *e = a; } else { entity.animations.push(a); }
                                        } else {
                                            for tag in &ase.tags {
                                                let from = tag.from as u32; let to = tag.to.min(fc as u16 - 1) as u32;
                                                let frames: Vec<u32> = (from..=to).collect();
                                                let avg = ase.frames[from as usize..=to as usize].iter().map(|f| f.duration_ms as f32).sum::<f32>() / frames.len().max(1) as f32;
                                                let name = tag.name.to_lowercase();
                                                let a = toile_scene::AnimationData { name: name.clone(), frames, fps: (1000.0/avg.max(1.0)).round(), looping: tag.direction != 1, sprite_file: Some(afn.clone()), strip_frames: Some(fc) };
                                                if let Some(e) = entity.animations.iter_mut().find(|x| x.name == name) { *e = a; } else { entity.animations.push(a); }
                                            }
                                        }
                                        if entity.sprite_path.is_empty() { entity.sprite_path = afn; }
                                        if entity.default_animation.is_none() { entity.default_animation = entity.animations.first().map(|a| a.name.clone()); }
                                        self.sprite_cache.clear();
                                        self.status_msg = format!("Imported '{stem}'");
                                    }
                                }
                            }
                            if ui.button("📜 Strip (PNG) — horizontal strip").clicked() {
                                if let Some(file) = rfd::FileDialog::new().set_title("Import Strip").add_filter("PNG", &["png"]).pick_file() {
                                    let rel = pdir.as_ref().and_then(|pd| file.strip_prefix(pd).ok().map(|p| p.to_string_lossy().to_string())).unwrap_or_else(|| file.to_string_lossy().to_string());
                                    if let Ok((w, h)) = image::image_dimensions(&file) {
                                        let fs = h; let nf = w / fs;
                                        let stem = file.file_stem().unwrap_or_default().to_string_lossy().to_string().to_lowercase().replace("-sheet","").replace("_sheet","");
                                        let name = ["idle","run","walk","jump","die","dash","slide"].iter().find(|k| stem.contains(**k)).map(|k| k.to_string()).unwrap_or(stem);
                                        if !entity.animations.iter().any(|a| a.name == name) {
                                            entity.animations.push(toile_scene::AnimationData { name, frames: (0..nf).collect(), fps: 10.0, looping: true, sprite_file: Some(rel), strip_frames: Some(nf) });
                                        }
                                    }
                                }
                            }

                            // Sprite Sheet config
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Sprite Sheet Grid").strong().size(13.0));
                            ui.separator();
                            let has_sheet = entity.sprite_sheet.is_some();
                            let mut en = has_sheet;
                            if ui.checkbox(&mut en, "Enable grid mode").changed() {
                                if en { entity.sprite_sheet = Some(auto_detect_sprite_sheet(&entity.sprite_path, pdir)); }
                                else { entity.sprite_sheet = None; }
                            }
                            if let Some(ref mut sheet) = entity.sprite_sheet {
                                ui.horizontal(|ui| {
                                    for (name, fw, fh, c, r) in &[("Mana Seed", 64u32, 64u32, 8u32, 8u32), ("RPG Maker", 48, 48, 3, 4), ("32×32", 32, 32, 0, 0), ("64×64", 64, 64, 0, 0)] {
                                        if ui.small_button(*name).clicked() {
                                            sheet.frame_width = *fw; sheet.frame_height = *fh;
                                            if *c > 0 { sheet.columns = *c; sheet.rows = *r; }
                                            else if let Some((iw, ih)) = get_image_dimensions(&entity.sprite_path, pdir) { sheet.columns = (iw / *fw).max(1); sheet.rows = (ih / *fh).max(1); }
                                        }
                                    }
                                });
                                egui::Grid::new("sa_sheet").num_columns(4).show(ui, |ui| {
                                    ui.label("Frame"); ui.add(egui::DragValue::new(&mut sheet.frame_width).prefix("W:"));
                                    ui.add(egui::DragValue::new(&mut sheet.frame_height).prefix("H:"));
                                    ui.label(format!("{}×{}", sheet.columns, sheet.rows));
                                    ui.end_row();
                                });
                            }
                        }
                    });
                });

                // Right panel — Animation list
                egui::SidePanel::right("sprite_anim_right").min_width(280.0).default_width(300.0).show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(egui::RichText::new("Animations").strong().size(14.0));
                        ui.separator();

                        if let Some(entity) = self.scene.entities.iter_mut().find(|e| e.id == id) {
                            if entity.animations.is_empty() {
                                ui.label(egui::RichText::new("No animations yet.\nUse Import on the left.").color(egui::Color32::from_gray(130)));
                            }
                            let mut remove_anim: Option<usize> = None;
                            for (i, anim) in entity.animations.iter_mut().enumerate() {
                                let is_default = entity.default_animation.as_deref() == Some(&anim.name);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(is_default, egui::RichText::new(&anim.name).strong().size(13.0)).on_hover_text("Click = set as default").clicked() {
                                        entity.default_animation = Some(anim.name.clone());
                                    }
                                    if is_default { ui.label(egui::RichText::new("★").color(egui::Color32::YELLOW)); }
                                    if ui.small_button("x").clicked() { remove_anim = Some(i); }
                                });
                                egui::Grid::new(format!("sa_anim_{i}")).num_columns(2).show(ui, |ui| {
                                    ui.label("FPS"); ui.add(egui::DragValue::new(&mut anim.fps).range(1.0..=60.0).speed(0.5)); ui.end_row();
                                    ui.label("Loop"); ui.checkbox(&mut anim.looping, ""); ui.end_row();
                                    ui.label("Frames"); ui.label(format!("{}", anim.frames.len())); ui.end_row();
                                });
                                // Frame badges
                                ui.horizontal_wrapped(|ui| {
                                    let mut remove_f: Option<usize> = None;
                                    for (fi, frame) in anim.frames.iter().enumerate() {
                                        if ui.small_button(format!("{frame}")).clicked() { remove_f = Some(fi); }
                                    }
                                    if let Some(fi) = remove_f { anim.frames.remove(fi); }
                                    if entity.sprite_sheet.is_some() {
                                        if ui.small_button("+ pick").clicked() {
                                            self.show_frame_picker = true;
                                            self.frame_picker_anim = anim.name.clone();
                                        }
                                    }
                                });
                                if let Some(ref f) = anim.sprite_file {
                                    let short = f.rsplit('/').next().unwrap_or(f);
                                    ui.label(egui::RichText::new(format!("file: {short}")).size(9.0).color(egui::Color32::from_gray(120)));
                                }
                                ui.separator();
                            }
                            if let Some(idx) = remove_anim { entity.animations.remove(idx); }

                            // Quick-add
                            ui.horizontal(|ui| {
                                ui.label("Add:");
                                for (name, fps, l) in &[("idle", 4.0f32, true), ("walk", 7.0, true), ("run", 10.0, true), ("jump", 5.0, false)] {
                                    if !entity.animations.iter().any(|a| a.name == *name) {
                                        if ui.small_button(*name).clicked() {
                                            entity.animations.push(toile_scene::AnimationData { name: name.to_string(), frames: vec![], fps: *fps, looping: *l, sprite_file: None, strip_frames: None });
                                        }
                                    }
                                }
                            });
                        }
                    });
                });

                // Central panel — sprite preview (zoomed entity)
                // The normal draw() already renders the entity in the viewport
            } else {
                // No entity selected — go back
                self.editor_mode = EditorMode::Entity;
            }
        }
    }

    /// Show the Sprite & Animation Editor floating window.
    pub(crate) fn show_sprite_editor_window(
        &mut self,
        ctx: &egui::Context,
        pdir: &Option<PathBuf>,
    ) {
        if self.show_sprite_editor {
            if let Some(id) = self.selected_id {
                let mut open = true;
                egui::Window::new("Sprite & Animation Editor")
                    .open(&mut open)
                    .default_width(550.0)
                    .default_height(500.0)
                    .show(ctx, |ui| {
                        if let Some(entity) = self.scene.entities.iter_mut().find(|e| e.id == id) {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                // ── 1. Import section ──
                                ui.label(egui::RichText::new("Import").strong().size(14.0));
                                ui.separator();
                                ui.horizontal(|ui| {
                                    // Import simple sprite
                                    if ui.button("🖼 Image (PNG/JPG)").on_hover_text("Single static sprite").clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .set_title("Select Sprite Image")
                                            .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                            .pick_file()
                                        {
                                            entity.sprite_path = pdir.as_ref()
                                                .and_then(|pd| file.strip_prefix(pd).ok().map(|p| p.to_string_lossy().to_string()))
                                                .unwrap_or_else(|| file.to_string_lossy().to_string());
                                            entity.sprite_sheet = None;
                                            self.sprite_cache.clear();
                                        }
                                    }
                                    // Import Aseprite
                                    if ui.button("✨ Aseprite (.ase)").on_hover_text("Parse .aseprite file → atlas + animations from tags").clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .set_title("Import Aseprite File")
                                            .add_filter("Aseprite", &["aseprite", "ase"])
                                            .pick_file()
                                        {
                                            // (reuse existing import logic via a flag)
                                            self.status_msg = "Use 'Import .aseprite' in the Animations section below".to_string();
                                            // Inline the import
                                            if let Ok(ase) = toile_assets::aseprite::load_ase_file(&file) {
                                                let (atlas_rgba, atlas_w, atlas_h, _) = toile_assets::aseprite::build_atlas(&ase);
                                                let frame_count = ase.frames.len() as u32;
                                                let _fw = ase.width as u32;
                                                let _fh = ase.height as u32;
                                                let stem = file.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                                let atlas_filename = format!("assets/{stem}_atlas.png");
                                                let atlas_full = pdir.as_ref().map(|d| d.join(&atlas_filename)).unwrap_or_else(|| PathBuf::from(&atlas_filename));
                                                if let Some(parent) = atlas_full.parent() { let _ = std::fs::create_dir_all(parent); }
                                                if let Some(img) = image::RgbaImage::from_raw(atlas_w, atlas_h, atlas_rgba) { let _ = img.save(&atlas_full); }

                                                let anim_name_from_stem = |s: &str| -> String {
                                                    let n = s.to_lowercase();
                                                    if n.contains("idle") { "idle".into() } else if n.contains("run") { "run".into() }
                                                    else if n.contains("walk") || n.contains("flow") { "walk".into() }
                                                    else if n.contains("jump") { "jump".into() } else if n.contains("die") { "die".into() }
                                                    else if n.contains("dash") { "dash".into() } else if n.contains("slide") { "slide".into() }
                                                    else { n.split('_').next().unwrap_or("anim").to_string() }
                                                };

                                                if ase.tags.is_empty() {
                                                    let name = anim_name_from_stem(&stem);
                                                    let avg_dur = ase.frames.iter().map(|f| f.duration_ms as f32).sum::<f32>() / frame_count.max(1) as f32;
                                                    let anim = toile_scene::AnimationData {
                                                        name: name.clone(), frames: (0..frame_count).collect(),
                                                        fps: (1000.0 / avg_dur.max(1.0)).round(), looping: true,
                                                        sprite_file: Some(atlas_filename.clone()), strip_frames: Some(frame_count),
                                                    };
                                                    if let Some(existing) = entity.animations.iter_mut().find(|a| a.name == name) { *existing = anim; }
                                                    else { entity.animations.push(anim); }
                                                } else {
                                                    for tag in &ase.tags {
                                                        let from = tag.from as u32;
                                                        let to = tag.to.min(frame_count as u16 - 1) as u32;
                                                        let frames: Vec<u32> = (from..=to).collect();
                                                        let avg_dur = ase.frames[from as usize..=to as usize].iter().map(|f| f.duration_ms as f32).sum::<f32>() / frames.len().max(1) as f32;
                                                        let name = tag.name.to_lowercase();
                                                        let anim = toile_scene::AnimationData {
                                                            name: name.clone(), frames, fps: (1000.0 / avg_dur.max(1.0)).round(),
                                                            looping: tag.direction != 1,
                                                            sprite_file: Some(atlas_filename.clone()), strip_frames: Some(frame_count),
                                                        };
                                                        if let Some(existing) = entity.animations.iter_mut().find(|a| a.name == name) { *existing = anim; }
                                                        else { entity.animations.push(anim); }
                                                    }
                                                }
                                                if entity.sprite_path.is_empty() { entity.sprite_path = atlas_filename; }
                                                if entity.default_animation.is_none() { entity.default_animation = entity.animations.first().map(|a| a.name.clone()); }
                                                self.sprite_cache.clear();
                                                self.status_msg = format!("Imported '{stem}'");
                                            }
                                        }
                                    }
                                    // Import strip
                                    if ui.button("📜 Strip (PNG)").on_hover_text("Horizontal strip — one row of frames").clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .set_title("Import Sprite Strip")
                                            .add_filter("PNG", &["png"])
                                            .pick_file()
                                        {
                                            let rel_path = pdir.as_ref()
                                                .and_then(|pd| file.strip_prefix(pd).ok().map(|p| p.to_string_lossy().to_string()))
                                                .unwrap_or_else(|| file.to_string_lossy().to_string());
                                            if let Ok((w, h)) = image::image_dimensions(&file) {
                                                let frame_size = h;
                                                let num_frames = w / frame_size;
                                                let stem = file.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                                let n = stem.to_lowercase().replace("-sheet", "").replace("_sheet", "");
                                                let anim_name = if n.contains("idle") { "idle" } else if n.contains("run") { "run" }
                                                    else if n.contains("walk") || n.contains("flow") { "walk" } else if n.contains("jump") { "jump" }
                                                    else { &n }.to_string();
                                                if !entity.animations.iter().any(|a| a.name == anim_name) {
                                                    entity.animations.push(toile_scene::AnimationData {
                                                        name: anim_name, frames: (0..num_frames).collect(), fps: 10.0, looping: true,
                                                        sprite_file: Some(rel_path), strip_frames: Some(num_frames),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                });
                                ui.label(egui::RichText::new("Import multiple files to build different animations for the same entity.").size(10.0).color(egui::Color32::from_gray(130)));

                                // ── 2. Sprite Sheet Config ──
                                if !entity.sprite_path.is_empty() {
                                    ui.add_space(12.0);
                                    ui.label(egui::RichText::new("Sprite Sheet").strong().size(14.0));
                                    ui.separator();
                                    let has_sheet = entity.sprite_sheet.is_some();
                                    let mut enabled = has_sheet;
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut enabled, "Enable grid (for single-sheet sprites)").changed() {
                                            if enabled { entity.sprite_sheet = Some(auto_detect_sprite_sheet(&entity.sprite_path, pdir)); }
                                            else { entity.sprite_sheet = None; }
                                        }
                                    });
                                    if let Some(ref mut sheet) = entity.sprite_sheet {
                                        ui.horizontal(|ui| {
                                            for (name, fw, fh, c, r) in &[("Mana Seed", 64u32, 64u32, 8u32, 8u32), ("RPG Maker", 48, 48, 3, 4), ("32×32", 32, 32, 0, 0), ("64×64", 64, 64, 0, 0)] {
                                                if ui.small_button(*name).clicked() {
                                                    sheet.frame_width = *fw; sheet.frame_height = *fh;
                                                    if *c > 0 { sheet.columns = *c; sheet.rows = *r; }
                                                    else if let Some((iw, ih)) = get_image_dimensions(&entity.sprite_path, pdir) {
                                                        sheet.columns = (iw / *fw).max(1); sheet.rows = (ih / *fh).max(1);
                                                    }
                                                }
                                            }
                                            if ui.small_button("Auto").clicked() { *sheet = auto_detect_sprite_sheet(&entity.sprite_path, pdir); }
                                        });
                                        egui::Grid::new("se_sheet_grid").num_columns(4).show(ui, |ui| {
                                            ui.label("Frame"); ui.add(egui::DragValue::new(&mut sheet.frame_width).prefix("W:").range(1..=1024));
                                            ui.add(egui::DragValue::new(&mut sheet.frame_height).prefix("H:").range(1..=1024));
                                            ui.label(format!("{}×{}", sheet.columns, sheet.rows));
                                            ui.end_row();
                                        });
                                    }
                                }

                                // ── 3. Animations ──
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new("Animations").strong().size(14.0));
                                ui.separator();
                                if entity.animations.is_empty() {
                                    ui.label(egui::RichText::new("No animations. Import files above or add manually.").color(egui::Color32::from_gray(130)));
                                }
                                let mut remove_anim: Option<usize> = None;
                                for (i, anim) in entity.animations.iter_mut().enumerate() {
                                    let is_default = entity.default_animation.as_deref() == Some(&anim.name);
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(is_default, egui::RichText::new(&anim.name).strong().size(12.0)).on_hover_text("Click to set as default").clicked() {
                                            entity.default_animation = Some(anim.name.clone());
                                        }
                                        if is_default { ui.label(egui::RichText::new("★").color(egui::Color32::YELLOW)); }
                                        ui.add(egui::DragValue::new(&mut anim.fps).prefix("fps:").range(1.0..=60.0).speed(0.5));
                                        ui.checkbox(&mut anim.looping, "loop");
                                        if ui.small_button("x").clicked() { remove_anim = Some(i); }
                                    });
                                    // Frames
                                    ui.horizontal_wrapped(|ui| {
                                        let mut remove_f: Option<usize> = None;
                                        for (fi, frame) in anim.frames.iter().enumerate() {
                                            if ui.small_button(format!("{frame}")).on_hover_text("Click to remove").clicked() { remove_f = Some(fi); }
                                        }
                                        if let Some(fi) = remove_f { anim.frames.remove(fi); }
                                        if entity.sprite_sheet.is_some() {
                                            if ui.small_button("+ pick").clicked() {
                                                self.show_frame_picker = true;
                                                self.frame_picker_anim = anim.name.clone();
                                            }
                                        }
                                    });
                                    if let Some(ref f) = anim.sprite_file {
                                        let short = f.rsplit('/').next().unwrap_or(f);
                                        ui.label(egui::RichText::new(format!("  file: {short}")).size(9.0).color(egui::Color32::from_gray(120)));
                                    }
                                    ui.separator();
                                }
                                if let Some(idx) = remove_anim { entity.animations.remove(idx); }

                                // Quick-add
                                ui.horizontal(|ui| {
                                    ui.label("Add:");
                                    for (name, fps, looping) in &[("idle", 4.0f32, true), ("walk", 7.0, true), ("run", 10.0, true), ("jump", 5.0, false)] {
                                        if !entity.animations.iter().any(|a| a.name == *name) {
                                            if ui.small_button(*name).clicked() {
                                                entity.animations.push(toile_scene::AnimationData {
                                                    name: name.to_string(), frames: vec![], fps: *fps, looping: *looping,
                                                    sprite_file: None, strip_frames: None,
                                                });
                                            }
                                        }
                                    }
                                });
                            }); // end ScrollArea
                        }
                    });
                if !open { self.show_sprite_editor = false; }
            } else {
                self.show_sprite_editor = false;
            }
        }
    }

    /// Show the Frame Picker window for selecting sprite sheet frames.
    pub(crate) fn show_frame_picker_window(
        &mut self,
        ctx: &egui::Context,
        pdir: &Option<PathBuf>,
    ) {
        if self.show_frame_picker {
            if let Some(id) = self.selected_id {
                let entity = self.scene.entities.iter().find(|e| e.id == id);
                let sheet_info = entity.and_then(|e| e.sprite_sheet.as_ref().map(|s| (s.columns, s.rows, s.frame_width, s.frame_height)));
                let sprite_path = entity.map(|e| e.sprite_path.clone()).unwrap_or_default();

                // Load image as egui texture if needed
                if !sprite_path.is_empty() && self.frame_picker_loaded_path != sprite_path {
                    let full = pdir.as_ref().map(|d| d.join(&sprite_path)).unwrap_or_else(|| PathBuf::from(&sprite_path));
                    if let Ok(img) = image::open(&full) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                        self.frame_picker_egui_tex = Some(ctx.load_texture("frame_picker_sheet", color_image, egui::TextureOptions::NEAREST));
                        self.frame_picker_loaded_path = sprite_path.clone();
                    }
                }

                let has_egui_tex = self.frame_picker_egui_tex.is_some();
                if let (Some((cols, rows, fw, fh)), true) = (sheet_info, has_egui_tex) {
                    let egui_tex = self.frame_picker_egui_tex.as_ref().unwrap();
                    let mut open = true;
                    let anim_name = self.frame_picker_anim.clone();
                    egui::Window::new(format!("Frame Picker — {anim_name}"))
                        .open(&mut open)
                        .default_width(cols as f32 * 68.0 + 20.0)
                        .show(ctx, |ui| {
                            ui.label(egui::RichText::new(format!("Click frames to add to '{anim_name}'. Grid: {cols}×{rows}, frame: {fw}×{fh}px")).size(11.0));
                            ui.separator();

                            // Render the sprite sheet as a grid of clickable frame buttons
                            let cell_size = 64.0_f32;
                            let total = cols * rows;
                            let mut clicked_frame: Option<u32> = None;

                            // Get current frames for highlighting
                            let current_frames: Vec<u32> = self.scene.entities.iter()
                                .find(|e| e.id == id)
                                .and_then(|e| e.animations.iter().find(|a| a.name == anim_name))
                                .map(|a| a.frames.clone())
                                .unwrap_or_default();

                            egui::ScrollArea::both().max_height(500.0).show(ui, |ui| {
                                for row in 0..rows {
                                    ui.horizontal(|ui| {
                                        for col in 0..cols {
                                            let frame_idx = row * cols + col;
                                            if frame_idx >= total { break; }

                                            let u_step = 1.0 / cols as f32;
                                            let v_step = 1.0 / rows as f32;
                                            let uv0 = egui::pos2(col as f32 * u_step, row as f32 * v_step);
                                            let uv1 = egui::pos2((col + 1) as f32 * u_step, (row + 1) as f32 * v_step);

                                            let is_selected = current_frames.contains(&frame_idx);
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(cell_size, cell_size),
                                                egui::Sense::click(),
                                            );

                                            // Background
                                            let bg = if is_selected {
                                                egui::Color32::from_rgba_unmultiplied(80, 200, 80, 60)
                                            } else if response.hovered() {
                                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30)
                                            } else {
                                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 40)
                                            };
                                            ui.painter().rect_filled(rect, 2.0, bg);

                                            // Sprite frame
                                            let tex_id = egui_tex.id();
                                            ui.painter().image(tex_id, rect.shrink(2.0), egui::Rect::from_min_max(uv0, uv1), egui::Color32::WHITE);

                                            // Frame number
                                            ui.painter().text(
                                                rect.left_top() + egui::vec2(2.0, 1.0),
                                                egui::Align2::LEFT_TOP,
                                                format!("{frame_idx}"),
                                                egui::FontId::proportional(9.0),
                                                if is_selected { egui::Color32::YELLOW } else { egui::Color32::from_gray(180) },
                                            );

                                            // Border for selected
                                            if is_selected {
                                                ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(2.0, egui::Color32::YELLOW), egui::StrokeKind::Outside);
                                            }

                                            if response.clicked() {
                                                clicked_frame = Some(frame_idx);
                                            }
                                        }
                                    });
                                }
                            });

                            // Apply clicked frame
                            if let Some(frame) = clicked_frame {
                                if let Some(entity) = self.scene.entities.iter_mut().find(|e| e.id == id) {
                                    if let Some(anim) = entity.animations.iter_mut().find(|a| a.name == anim_name) {
                                        anim.frames.push(frame);
                                    }
                                }
                            }

                            // Show current sequence
                            ui.separator();
                            if let Some(entity) = self.scene.entities.iter().find(|e| e.id == id) {
                                if let Some(anim) = entity.animations.iter().find(|a| a.name == anim_name) {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(format!("{anim_name}:")).strong());
                                        for f in &anim.frames {
                                            ui.label(format!("{f}"));
                                        }
                                        if anim.frames.is_empty() {
                                            ui.label(egui::RichText::new("(empty — click frames above)").color(egui::Color32::from_gray(130)));
                                        }
                                    });
                                }
                            }

                            ui.horizontal(|ui| {
                                if ui.button("Clear frames").clicked() {
                                    if let Some(entity) = self.scene.entities.iter_mut().find(|e| e.id == id) {
                                        if let Some(anim) = entity.animations.iter_mut().find(|a| a.name == anim_name) {
                                            anim.frames.clear();
                                        }
                                    }
                                }
                                if ui.button("Done").clicked() {
                                    self.show_frame_picker = false;
                                }
                            });
                        });
                    if !open { self.show_frame_picker = false; }
                } else {
                    self.show_frame_picker = false;
                }
            } else {
                self.show_frame_picker = false;
            }
        }
    }
}
