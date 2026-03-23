//! Tool definitions + executor — maps Claude tool calls to scene operations.

use std::collections::HashMap;
use toile_scene::SceneData;

/// All available tool definitions for Claude (JSON schema format).
pub fn tool_definitions() -> Vec<serde_json::Value> {
    vec![
        tool_def("list_entities", "List all entities in the current scene with their ID, name, position, and size", serde_json::json!({
            "type": "object", "properties": {}
        })),
        tool_def("create_entity", "Create a new entity in the scene", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Entity name"},
                "x": {"type": "number", "description": "X position"},
                "y": {"type": "number", "description": "Y position"},
                "width": {"type": "number", "description": "Width in pixels"},
                "height": {"type": "number", "description": "Height in pixels"},
                "role": {"type": "string", "enum": ["object", "player_platformer", "player_topdown", "solid", "collectible", "enemy"], "description": "Entity role (auto-configures behaviors and tags)"}
            },
            "required": ["name", "x", "y"]
        })),
        tool_def("update_entity", "Update an existing entity's properties", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer", "description": "Entity ID"},
                "name": {"type": "string"},
                "x": {"type": "number"},
                "y": {"type": "number"},
                "width": {"type": "number"},
                "height": {"type": "number"},
                "rotation": {"type": "number"},
                "layer": {"type": "integer"},
                "visible": {"type": "boolean"}
            },
            "required": ["entity_id"]
        })),
        tool_def("delete_entity", "Delete an entity by ID", serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "integer", "description": "Entity ID to delete"}
            },
            "required": ["entity_id"]
        })),
        tool_def("get_scene_info", "Get current scene name, entity count, settings, and viewport info", serde_json::json!({
            "type": "object", "properties": {}
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
                    "id": e.id,
                    "name": e.name,
                    "x": e.x,
                    "y": e.y,
                    "width": e.width,
                    "height": e.height,
                    "layer": e.layer,
                    "tags": e.tags,
                    "behaviors": e.behaviors.len(),
                    "visible": e.visible,
                })
            }).collect();
            serde_json::to_string_pretty(&serde_json::json!({
                "entity_count": entities.len(),
                "entities": entities
            })).unwrap_or_else(|_| "{}".into())
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

                // Apply role
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
                },
            }).to_string()
        }

        _ => serde_json::json!({"error": format!("Unknown tool: {}", tool_name)}).to_string(),
    }
}
