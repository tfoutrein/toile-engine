//! Tool definitions + executor — maps Claude tool calls to scene operations.

use std::collections::HashMap;
use toile_scene::SceneData;

/// All available tool definitions for Claude (JSON schema format).
pub fn tool_definitions() -> Vec<serde_json::Value> {
    vec![
        // ── Scene ──
        tool_def("get_scene_info", "Get current scene name, entity count, settings, and viewport info", serde_json::json!({
            "type": "object", "properties": {}
        })),
        tool_def("set_scene_settings", "Update scene settings (gravity, camera mode, viewport)", serde_json::json!({
            "type": "object",
            "properties": {
                "gravity": {"type": "number"},
                "camera_zoom": {"type": "number"},
                "camera_mode": {"type": "string", "enum": ["Fixed", "FollowPlayer", "PlatformerFollow"]},
                "viewport_width": {"type": "integer"},
                "viewport_height": {"type": "integer"}
            }
        })),

        // ── Entities ──
        tool_def("list_entities", "List all entities in the current scene with their properties", serde_json::json!({
            "type": "object", "properties": {}
        })),
        tool_def("create_entity", "Create a new entity in the scene", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Entity name"},
                "x": {"type": "number", "description": "X position (0=center, Y-up)"},
                "y": {"type": "number", "description": "Y position (positive=up)"},
                "width": {"type": "number", "description": "Width in pixels (default 32)"},
                "height": {"type": "number", "description": "Height in pixels (default 32)"},
                "role": {"type": "string", "enum": ["object", "player_platformer", "player_topdown", "solid", "collectible", "enemy"], "description": "Auto-configures behaviors and tags"}
            },
            "required": ["name", "x", "y"]
        })),
        tool_def("update_entity", "Update an existing entity's properties", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer"},
                "name": {"type": "string"},
                "x": {"type": "number"}, "y": {"type": "number"},
                "width": {"type": "number"}, "height": {"type": "number"},
                "rotation": {"type": "number"},
                "layer": {"type": "integer"},
                "visible": {"type": "boolean"}
            },
            "required": ["entity_id"]
        })),
        tool_def("delete_entity", "Delete an entity by ID", serde_json::json!({
            "type": "object",
            "properties": {"entity_id": {"type": "integer"}},
            "required": ["entity_id"]
        })),

        // ── Behaviors ──
        tool_def("add_behavior", "Add a behavior to an entity (Platform, TopDown, Bullet, Sine, Fade, Wrap, Solid)", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer"},
                "behavior_type": {"type": "string", "enum": ["Platform", "TopDown", "Bullet", "Sine", "Fade", "Wrap", "Solid"]},
                "config": {"type": "object", "description": "Behavior-specific config. Platform: {gravity, jump_force, max_speed, max_jumps}. Bullet: {speed, angle_degrees, gravity}. Sine: {property, magnitude, period}. Fade: {fade_in_time, fade_out_time, destroy_on_fade_out}. Wrap: {margin}."}
            },
            "required": ["entity_id", "behavior_type"]
        })),
        tool_def("remove_behavior", "Remove a behavior from an entity by index", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer"},
                "behavior_index": {"type": "integer", "description": "0-based index in the behaviors list"}
            },
            "required": ["entity_id", "behavior_index"]
        })),

        // ── Tags & Variables ──
        tool_def("set_tags", "Set the tags on an entity (replaces existing tags)", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer"},
                "tags": {"type": "array", "items": {"type": "string"}, "description": "Tags like Player, Solid, Coin, Enemy, Projectile"}
            },
            "required": ["entity_id", "tags"]
        })),
        tool_def("set_variables", "Set initial variables on an entity", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer"},
                "variables": {"type": "object", "description": "Key-value pairs, e.g. {\"health\": 3, \"score\": 0}"}
            },
            "required": ["entity_id", "variables"]
        })),

        // ── Event Sheets ──
        tool_def("create_event_sheet", "Create an event sheet file and assign it to an entity. Events are condition→action rules.", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer", "description": "Entity to assign the event sheet to"},
                "name": {"type": "string", "description": "Event sheet name (used for filename)"},
                "events": {"type": "array", "items": {"type": "object", "properties": {
                    "conditions": {"type": "array", "items": {"type": "object"}, "description": "Conditions: {\"type\":\"OnKeyPressed\",\"key\":\"Space\"}, {\"type\":\"OnCollisionWith\",\"tag\":\"Enemy\"}, {\"type\":\"EveryTick\"}, {\"type\":\"EveryNSeconds\",\"interval\":1.0}"},
                    "actions": {"type": "array", "items": {"type": "object"}, "description": "Actions: {\"type\":\"Destroy\"}, {\"type\":\"SpawnObject\",\"prefab\":\"Bullet\",\"x\":0,\"y\":0}, {\"type\":\"PlaySound\",\"sound\":\"shoot.wav\"}, {\"type\":\"GoToScene\",\"scene\":\"level2.json\"}, {\"type\":\"SetVariable\",\"name\":\"score\",\"value\":10}, {\"type\":\"AddToVariable\",\"name\":\"score\",\"amount\":1}, {\"type\":\"Log\",\"message\":\"hit!\"}"}
                }}}
            },
            "required": ["entity_id", "name", "events"]
        })),

        // ── Prefabs ──
        tool_def("save_as_prefab", "Save an entity as a reusable prefab template", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer"},
                "prefab_name": {"type": "string", "description": "Prefab name (saved to prefabs/ folder)"}
            },
            "required": ["entity_id", "prefab_name"]
        })),
    ]
}

fn tool_def(name: &str, description: &str, input_schema: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "input_schema": input_schema
    })
}

/// Execute a tool call on the scene. Returns the result as a string.
pub fn execute_tool(scene: &mut SceneData, tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "list_entities" => {
            let entities: Vec<serde_json::Value> = scene.entities.iter().map(|e| {
                serde_json::json!({
                    "id": e.id, "name": e.name,
                    "x": e.x, "y": e.y,
                    "width": e.width, "height": e.height,
                    "layer": e.layer,
                    "tags": e.tags,
                    "behaviors": e.behaviors.iter().map(|b| format!("{:?}", b).split('(').next().unwrap_or("?").to_string()).collect::<Vec<_>>(),
                    "visible": e.visible,
                    "event_sheet": e.event_sheet,
                })
            }).collect();
            serde_json::to_string_pretty(&serde_json::json!({"entities": entities, "count": entities.len()})).unwrap_or_else(|_| "{}".into())
        }

        "get_scene_info" => {
            serde_json::json!({
                "name": scene.name,
                "entity_count": scene.entities.len(),
                "settings": {
                    "gravity": scene.settings.gravity,
                    "viewport_width": scene.settings.viewport_width,
                    "viewport_height": scene.settings.viewport_height,
                    "camera_zoom": scene.settings.camera_zoom,
                    "camera_position": scene.settings.camera_position,
                    "camera_mode": format!("{:?}", scene.settings.camera_mode),
                },
            }).to_string()
        }

        "set_scene_settings" => {
            if let Some(v) = args.get("gravity").and_then(|v| v.as_f64()) { scene.settings.gravity = v as f32; }
            if let Some(v) = args.get("camera_zoom").and_then(|v| v.as_f64()) { scene.settings.camera_zoom = v as f32; }
            if let Some(v) = args.get("viewport_width").and_then(|v| v.as_u64()) { scene.settings.viewport_width = v as u32; }
            if let Some(v) = args.get("viewport_height").and_then(|v| v.as_u64()) { scene.settings.viewport_height = v as u32; }
            if let Some(mode) = args.get("camera_mode").and_then(|v| v.as_str()) {
                scene.settings.camera_mode = match mode {
                    "FollowPlayer" => toile_scene::CameraMode::FollowPlayer,
                    "PlatformerFollow" => toile_scene::CameraMode::PlatformerFollow {
                        deadzone_x: 0.3, deadzone_y: 0.4, bounds: [0.0; 4],
                    },
                    _ => toile_scene::CameraMode::Fixed,
                };
            }
            serde_json::json!({"updated": "scene_settings"}).to_string()
        }

        "create_entity" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("Entity");
            let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let width = args.get("width").and_then(|v| v.as_f64()).unwrap_or(32.0) as f32;
            let height = args.get("height").and_then(|v| v.as_f64()).unwrap_or(32.0) as f32;
            let role = args.get("role").and_then(|v| v.as_str()).unwrap_or("object");

            let id = scene.add_entity(name, x, y);
            if let Some(entity) = scene.find_entity_mut(id) {
                entity.width = width;
                entity.height = height;
                apply_role(entity, role);
            }
            serde_json::json!({"created": {"id": id, "name": name, "x": x, "y": y, "role": role}}).to_string()
        }

        "update_entity" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            if let Some(entity) = scene.find_entity_mut(eid) {
                if let Some(n) = args.get("name").and_then(|v| v.as_str()) { entity.name = n.to_string(); }
                if let Some(v) = args.get("x").and_then(|v| v.as_f64()) { entity.x = v as f32; }
                if let Some(v) = args.get("y").and_then(|v| v.as_f64()) { entity.y = v as f32; }
                if let Some(v) = args.get("width").and_then(|v| v.as_f64()) { entity.width = v as f32; }
                if let Some(v) = args.get("height").and_then(|v| v.as_f64()) { entity.height = v as f32; }
                if let Some(v) = args.get("rotation").and_then(|v| v.as_f64()) { entity.rotation = v as f32; }
                if let Some(v) = args.get("layer").and_then(|v| v.as_i64()) { entity.layer = v as i32; }
                if let Some(v) = args.get("visible").and_then(|v| v.as_bool()) { entity.visible = v; }
                serde_json::json!({"updated": eid}).to_string()
            } else {
                serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
            }
        }

        "delete_entity" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            scene.remove_entity(eid);
            serde_json::json!({"deleted": eid}).to_string()
        }

        "add_behavior" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let btype = args.get("behavior_type").and_then(|v| v.as_str()).unwrap_or("");
            let config = args.get("config").cloned().unwrap_or(serde_json::json!({}));

            if let Some(entity) = scene.find_entity_mut(eid) {
                let behavior = match btype {
                    "Platform" => {
                        let mut c = toile_behaviors::platform::PlatformConfig::default();
                        if let Some(v) = config.get("gravity").and_then(|v| v.as_f64()) { c.gravity = v as f32; }
                        if let Some(v) = config.get("jump_force").and_then(|v| v.as_f64()) { c.jump_force = v as f32; }
                        if let Some(v) = config.get("max_speed").and_then(|v| v.as_f64()) { c.max_speed = v as f32; }
                        if let Some(v) = config.get("max_jumps").and_then(|v| v.as_u64()) { c.max_jumps = v as u32; }
                        toile_behaviors::BehaviorConfig::Platform(c)
                    }
                    "TopDown" => {
                        let mut c = toile_behaviors::topdown::TopDownConfig::default();
                        if let Some(v) = config.get("max_speed").and_then(|v| v.as_f64()) { c.max_speed = v as f32; }
                        toile_behaviors::BehaviorConfig::TopDown(c)
                    }
                    "Bullet" => {
                        let mut c = toile_behaviors::bullet::BulletConfig::default();
                        if let Some(v) = config.get("speed").and_then(|v| v.as_f64()) { c.speed = v as f32; }
                        if let Some(v) = config.get("angle_degrees").and_then(|v| v.as_f64()) { c.angle_degrees = v as f32; }
                        if let Some(v) = config.get("gravity").and_then(|v| v.as_f64()) { c.gravity = v as f32; }
                        toile_behaviors::BehaviorConfig::Bullet(c)
                    }
                    "Sine" => {
                        let mut c = toile_behaviors::sine::SineConfig::default();
                        if let Some(p) = config.get("property").and_then(|v| v.as_str()) {
                            c.property = match p {
                                "X" => toile_behaviors::sine::SineProperty::X,
                                "Y" => toile_behaviors::sine::SineProperty::Y,
                                "Angle" => toile_behaviors::sine::SineProperty::Angle,
                                "Opacity" => toile_behaviors::sine::SineProperty::Opacity,
                                "Size" => toile_behaviors::sine::SineProperty::Size,
                                _ => toile_behaviors::sine::SineProperty::Y,
                            };
                        }
                        if let Some(v) = config.get("magnitude").and_then(|v| v.as_f64()) { c.magnitude = v as f32; }
                        if let Some(v) = config.get("period").and_then(|v| v.as_f64()) { c.period = v as f32; }
                        toile_behaviors::BehaviorConfig::Sine(c)
                    }
                    "Fade" => {
                        let mut c = toile_behaviors::fade::FadeConfig::default();
                        if let Some(v) = config.get("fade_in_time").and_then(|v| v.as_f64()) { c.fade_in_time = v as f32; }
                        if let Some(v) = config.get("fade_out_time").and_then(|v| v.as_f64()) { c.fade_out_time = v as f32; }
                        if let Some(v) = config.get("destroy_on_fade_out").and_then(|v| v.as_bool()) { c.destroy_on_fade_out = v; }
                        toile_behaviors::BehaviorConfig::Fade(c)
                    }
                    "Wrap" => {
                        let mut c = toile_behaviors::wrap::WrapConfig::default();
                        if let Some(v) = config.get("margin").and_then(|v| v.as_f64()) { c.margin = v as f32; }
                        toile_behaviors::BehaviorConfig::Wrap(c)
                    }
                    "Solid" => toile_behaviors::BehaviorConfig::Solid,
                    _ => return serde_json::json!({"error": format!("Unknown behavior: {}", btype)}).to_string(),
                };
                entity.behaviors.push(behavior);
                serde_json::json!({"added": btype, "entity_id": eid, "behavior_count": entity.behaviors.len()}).to_string()
            } else {
                serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
            }
        }

        "remove_behavior" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let idx = args.get("behavior_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if let Some(entity) = scene.find_entity_mut(eid) {
                if idx < entity.behaviors.len() {
                    entity.behaviors.remove(idx);
                    serde_json::json!({"removed_index": idx, "entity_id": eid}).to_string()
                } else {
                    serde_json::json!({"error": "Index out of range"}).to_string()
                }
            } else {
                serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
            }
        }

        "set_tags" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            if let Some(entity) = scene.find_entity_mut(eid) {
                entity.tags = args.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                serde_json::json!({"set_tags": entity.tags, "entity_id": eid}).to_string()
            } else {
                serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
            }
        }

        "set_variables" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            if let Some(entity) = scene.find_entity_mut(eid) {
                if let Some(vars) = args.get("variables").and_then(|v| v.as_object()) {
                    for (k, v) in vars {
                        if let Some(num) = v.as_f64() {
                            entity.variables.insert(k.clone(), num);
                        }
                    }
                }
                serde_json::json!({"set_variables": entity.variables, "entity_id": eid}).to_string()
            } else {
                serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
            }
        }

        "create_event_sheet" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("events");
            let events_json = args.get("events").cloned().unwrap_or(serde_json::json!([]));

            // Build the event sheet JSON
            let sheet = serde_json::json!({
                "name": name,
                "events": events_json,
            });

            // Save to scripts/ directory (relative path)
            let filename = format!("scripts/{}.event.json", name.to_lowercase().replace(' ', "_"));

            if let Some(entity) = scene.find_entity_mut(eid) {
                entity.event_sheet = Some(filename.clone());
            }

            // Return the sheet content + filename for the editor to save
            serde_json::json!({
                "created_event_sheet": filename,
                "entity_id": eid,
                "content": sheet,
                "note": "Event sheet assigned to entity. Save the scene to persist."
            }).to_string()
        }

        "save_as_prefab" => {
            let eid = args.get("entity_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let prefab_name = args.get("prefab_name").and_then(|v| v.as_str()).unwrap_or("prefab");

            if let Some(entity) = scene.entities.iter().find(|e| e.id == eid) {
                let prefab = toile_scene::prefab::Prefab::from_entity(prefab_name, entity);
                let filename = format!("prefabs/{}.prefab.json", prefab_name.to_lowercase().replace(' ', "_"));

                serde_json::json!({
                    "prefab_name": prefab_name,
                    "filename": filename,
                    "entity_id": eid,
                    "prefab": serde_json::to_value(&prefab).unwrap_or(serde_json::json!(null)),
                    "note": "Prefab created. Save to persist."
                }).to_string()
            } else {
                serde_json::json!({"error": format!("Entity {} not found", eid)}).to_string()
            }
        }

        _ => serde_json::json!({"error": format!("Unknown tool: {}", tool_name)}).to_string(),
    }
}

/// Apply a role to an entity (configures behaviors + tags).
fn apply_role(entity: &mut toile_scene::EntityData, role: &str) {
    match role {
        "player_platformer" => {
            entity.tags.push("Player".into());
            entity.behaviors.push(toile_behaviors::BehaviorConfig::Platform(Default::default()));
        }
        "player_topdown" => {
            entity.tags.push("Player".into());
            entity.behaviors.push(toile_behaviors::BehaviorConfig::TopDown(Default::default()));
        }
        "solid" => {
            entity.tags.push("Solid".into());
            entity.behaviors.push(toile_behaviors::BehaviorConfig::Solid);
        }
        "collectible" => {
            entity.tags.push("Coin".into());
            entity.behaviors.push(toile_behaviors::BehaviorConfig::Sine(toile_behaviors::sine::SineConfig {
                property: toile_behaviors::sine::SineProperty::Y,
                magnitude: 5.0,
                period: 1.5,
            }));
        }
        "enemy" => {
            entity.tags.push("Enemy".into());
        }
        _ => {}
    }
}
