use std::path::{Path, PathBuf};
use std::future::Future;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ListToolsResult, PaginatedRequestParams,
    ServerInfo, Tool, JsonObject, ServerCapabilities, ToolsCapability, Implementation,
};
use std::borrow::Cow;
use std::sync::Arc;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer, ServerHandler};
use serde::Serialize;
use toile_scene::SceneData;

#[derive(Clone)]
pub struct ToileMcpServer {
    project_dir: PathBuf,
}

fn make_tool(name: &'static str, desc: &'static str, schema: serde_json::Value) -> Tool {
    let map = match schema {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    Tool {
        name: Cow::Borrowed(name),
        title: None,
        description: Some(Cow::Borrowed(desc)),
        input_schema: Arc::new(map),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: None,
        meta: None,
    }
}

fn success_text<T: Serialize>(data: T) -> CallToolResult {
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "status": "success",
        "data": data,
    }))
    .unwrap();
    CallToolResult::success(vec![Content::text(json)])
}

fn error_text(code: &str, message: &str, suggestion: Option<&str>) -> CallToolResult {
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "status": "error",
        "error": {"code": code, "message": message, "suggestion": suggestion}
    }))
    .unwrap();
    CallToolResult::error(vec![Content::text(json)])
}

impl ToileMcpServer {
    pub fn new(project_dir: PathBuf) -> Self {
        Self { project_dir }
    }

    fn resolve(&self, relative: &str) -> PathBuf {
        self.project_dir.join(relative)
    }

    fn load(&self, path: &str) -> Result<(PathBuf, SceneData), String> {
        let p = self.resolve(path);
        toile_scene::load_scene(&p)
            .map(|s| (p, s))
            .map_err(|e| format!("{path}: {e}"))
    }

    fn save(&self, path: &Path, scene: &SceneData) -> Result<(), String> {
        toile_scene::save_scene(path, scene).map_err(|e| e.to_string())
    }

    fn get_str<'a>(args: &'a serde_json::Map<String, serde_json::Value>, key: &str) -> Option<&'a str> {
        args.get(key).and_then(|v| v.as_str())
    }

    fn get_f32(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f32> {
        args.get(key).and_then(|v| v.as_f64()).map(|v| v as f32)
    }

    fn get_u64(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
        args.get(key).and_then(|v| v.as_u64())
    }

    fn get_i32(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<i32> {
        args.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
    }

    async fn handle_tool(&self, name: &str, args: &serde_json::Map<String, serde_json::Value>) -> Result<CallToolResult, McpError> {
        match name {
            "get_project_info" => {
                let scenes = toile_scene::list_scene_files(&self.project_dir)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .collect::<Vec<_>>();
                Ok(success_text(serde_json::json!({
                    "project_dir": self.project_dir.display().to_string(),
                    "engine_version": env!("CARGO_PKG_VERSION"),
                    "scenes": scenes,
                })))
            }

            "list_scenes" => {
                let scenes = toile_scene::list_scene_files(&self.project_dir)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .collect::<Vec<_>>();
                Ok(success_text(scenes))
            }

            "create_scene" => {
                let name = Self::get_str(args, "name").unwrap_or("untitled");
                let filename = if name.ends_with(".json") { name.to_string() } else { format!("{name}.json") };
                let path = self.resolve(&filename);
                if path.exists() {
                    return Ok(error_text("SCENE_EXISTS", &format!("'{filename}' exists"), Some("Choose another name")));
                }
                let scene = SceneData::new(name);
                self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                Ok(success_text(serde_json::json!({"created": filename})))
            }

            "load_scene" => {
                let path = Self::get_str(args, "path").unwrap_or("scene.json");
                match self.load(path) {
                    Ok((_, scene)) => Ok(success_text(&scene)),
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, Some("Use list_scenes"))),
                }
            }

            "list_entities" => {
                let path = Self::get_str(args, "path").unwrap_or("scene.json");
                match self.load(path) {
                    Ok((_, scene)) => {
                        let entities: Vec<serde_json::Value> = scene.entities.iter().map(|e| {
                            serde_json::json!({"id": e.id, "name": e.name, "x": e.x, "y": e.y, "size": format!("{}x{}", e.width, e.height)})
                        }).collect();
                        Ok(success_text(entities))
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "create_entity" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let name = Self::get_str(args, "name").unwrap_or("Entity");
                let x = Self::get_f32(args, "x").unwrap_or(0.0);
                let y = Self::get_f32(args, "y").unwrap_or(0.0);
                match self.load(sp) {
                    Ok((path, mut scene)) => {
                        let id = scene.add_entity(name, x, y);
                        self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                        Ok(success_text(serde_json::json!({"id": id, "name": name, "x": x, "y": y, "total": scene.entities.len()})))
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "delete_entity" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let eid = Self::get_u64(args, "entity_id").unwrap_or(0);
                match self.load(sp) {
                    Ok((path, mut scene)) => {
                        if !scene.entities.iter().any(|e| e.id == eid) {
                            let ids: Vec<u64> = scene.entities.iter().map(|e| e.id).collect();
                            return Ok(error_text("ENTITY_NOT_FOUND", &format!("Entity {eid} not found"), Some(&format!("Available: {ids:?}"))));
                        }
                        scene.remove_entity(eid);
                        self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                        Ok(success_text(serde_json::json!({"deleted": eid, "remaining": scene.entities.len()})))
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "update_entity" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let eid = Self::get_u64(args, "entity_id").unwrap_or(0);
                match self.load(sp) {
                    Ok((path, mut scene)) => {
                        let entity = match scene.find_entity_mut(eid) {
                            Some(e) => e,
                            None => {
                                let ids: Vec<u64> = scene.entities.iter().map(|e| e.id).collect();
                                return Ok(error_text("ENTITY_NOT_FOUND", &format!("Entity {eid} not found"), Some(&format!("Available: {ids:?}"))));
                            }
                        };
                        if let Some(n) = Self::get_str(args, "name") { entity.name = n.to_string(); }
                        if let Some(v) = Self::get_f32(args, "x") { entity.x = v; }
                        if let Some(v) = Self::get_f32(args, "y") { entity.y = v; }
                        if let Some(v) = Self::get_f32(args, "width") { entity.width = v; }
                        if let Some(v) = Self::get_f32(args, "height") { entity.height = v; }
                        if let Some(v) = Self::get_i32(args, "layer") { entity.layer = v; }
                        let updated = entity.clone();
                        self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                        Ok(success_text(serde_json::json!({"updated": updated})))
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "create_tilemap" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let w = Self::get_u64(args, "width").unwrap_or(40) as u32;
                let h = Self::get_u64(args, "height").unwrap_or(23) as u32;
                let ts = Self::get_u64(args, "tile_size").unwrap_or(32) as u32;
                let tileset = Self::get_str(args, "tileset_path").unwrap_or("assets/platformer/tileset.png");
                let cols = Self::get_u64(args, "columns").unwrap_or(4) as u32;
                match self.load(sp) {
                    Ok((path, mut scene)) => {
                        let total = (w * h) as usize;
                        scene.tilemap = Some(toile_scene::TilemapData {
                            tileset_path: tileset.to_string(),
                            tile_size: ts,
                            columns: cols,
                            width: w,
                            height: h,
                            layers: vec![toile_scene::TilemapLayerData {
                                name: "Ground".to_string(),
                                tiles: vec![0; total],
                                visible: true,
                            }],
                        });
                        self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                        Ok(success_text(serde_json::json!({"created_tilemap": {"width": w, "height": h, "tile_size": ts}})))
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "set_tile" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let layer = Self::get_u64(args, "layer").unwrap_or(0) as usize;
                let col = Self::get_u64(args, "col").unwrap_or(0) as u32;
                let row = Self::get_u64(args, "row").unwrap_or(0) as u32;
                let gid = Self::get_u64(args, "gid").unwrap_or(0) as u32;
                match self.load(sp) {
                    Ok((path, mut scene)) => {
                        if let Some(tilemap) = &mut scene.tilemap {
                            if let Some(layer_data) = tilemap.layers.get_mut(layer) {
                                let idx = (row * tilemap.width + col) as usize;
                                if idx < layer_data.tiles.len() {
                                    layer_data.tiles[idx] = gid;
                                    self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                                    Ok(success_text(serde_json::json!({"set": {"col": col, "row": row, "gid": gid}})))
                                } else {
                                    Ok(error_text("OUT_OF_BOUNDS", &format!("({col},{row}) out of {}x{}", tilemap.width, tilemap.height), None))
                                }
                            } else {
                                Ok(error_text("LAYER_NOT_FOUND", &format!("Layer {layer} not found"), None))
                            }
                        } else {
                            Ok(error_text("NO_TILEMAP", "No tilemap in scene", Some("Use create_tilemap first")))
                        }
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "fill_rect" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let layer = Self::get_u64(args, "layer").unwrap_or(0) as usize;
                let col = Self::get_u64(args, "col").unwrap_or(0) as u32;
                let row = Self::get_u64(args, "row").unwrap_or(0) as u32;
                let w = Self::get_u64(args, "width").unwrap_or(1) as u32;
                let h = Self::get_u64(args, "height").unwrap_or(1) as u32;
                let gid = Self::get_u64(args, "gid").unwrap_or(1) as u32;
                match self.load(sp) {
                    Ok((path, mut scene)) => {
                        if let Some(tilemap) = &mut scene.tilemap {
                            if let Some(layer_data) = tilemap.layers.get_mut(layer) {
                                let mut count = 0u32;
                                for r in row..(row + h).min(tilemap.height) {
                                    for c in col..(col + w).min(tilemap.width) {
                                        let idx = (r * tilemap.width + c) as usize;
                                        if idx < layer_data.tiles.len() {
                                            layer_data.tiles[idx] = gid;
                                            count += 1;
                                        }
                                    }
                                }
                                self.save(&path, &scene).map_err(|e| McpError::internal_error(e, None))?;
                                Ok(success_text(serde_json::json!({"filled": count, "gid": gid})))
                            } else {
                                Ok(error_text("LAYER_NOT_FOUND", &format!("Layer {layer} not found"), None))
                            }
                        } else {
                            Ok(error_text("NO_TILEMAP", "No tilemap in scene", Some("Use create_tilemap first")))
                        }
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            "get_tile" => {
                let sp = Self::get_str(args, "scene_path").unwrap_or("scene.json");
                let layer = Self::get_u64(args, "layer").unwrap_or(0) as usize;
                let col = Self::get_u64(args, "col").unwrap_or(0) as u32;
                let row = Self::get_u64(args, "row").unwrap_or(0) as u32;
                match self.load(sp) {
                    Ok((_, scene)) => {
                        if let Some(tilemap) = &scene.tilemap {
                            if let Some(layer_data) = tilemap.layers.get(layer) {
                                let idx = (row * tilemap.width + col) as usize;
                                if idx < layer_data.tiles.len() {
                                    Ok(success_text(serde_json::json!({"col": col, "row": row, "gid": layer_data.tiles[idx]})))
                                } else {
                                    Ok(error_text("OUT_OF_BOUNDS", &format!("({col},{row}) out of bounds"), None))
                                }
                            } else {
                                Ok(error_text("LAYER_NOT_FOUND", &format!("Layer {layer} not found"), None))
                            }
                        } else {
                            Ok(error_text("NO_TILEMAP", "No tilemap in scene", None))
                        }
                    }
                    Err(e) => Ok(error_text("LOAD_FAILED", &e, None)),
                }
            }

            _ => Ok(error_text("UNKNOWN_TOOL", &format!("Unknown: {name}"), Some("Use tools/list"))),
        }
    }
}

impl ServerHandler for ToileMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "toile-mcp-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: Some("Toile Engine MCP Server — create and manipulate 2D game scenes.\n\
                Use list_scenes to discover scenes, then create/list/update entities.".into()),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let tools = vec![
            make_tool("get_project_info", "Get project info, scenes list", serde_json::json!({"type": "object", "properties": {}})),
            make_tool("list_scenes", "List scene files", serde_json::json!({"type": "object", "properties": {}})),
            make_tool("create_scene", "Create empty scene", serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]})),
            make_tool("load_scene", "Load scene data", serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]})),
            make_tool("list_entities", "List entities in scene", serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]})),
            make_tool("create_entity", "Add entity to scene", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "name": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"}}, "required": ["scene_path", "name", "x", "y"]})),
            make_tool("delete_entity", "Delete entity by ID", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "entity_id": {"type": "integer"}}, "required": ["scene_path", "entity_id"]})),
            make_tool("update_entity", "Update entity properties", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "entity_id": {"type": "integer"}, "name": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"}, "width": {"type": "number"}, "height": {"type": "number"}, "layer": {"type": "integer"}}, "required": ["scene_path", "entity_id"]})),
            // v0.2 tilemap tools
            make_tool("create_tilemap", "Create a tilemap in a scene", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "width": {"type": "integer", "description": "Map width in tiles"}, "height": {"type": "integer", "description": "Map height in tiles"}, "tile_size": {"type": "integer", "description": "Tile size in pixels"}, "tileset_path": {"type": "string"}, "columns": {"type": "integer", "description": "Tileset columns"}}, "required": ["scene_path", "width", "height"]})),
            make_tool("set_tile", "Set a tile in the tilemap", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "layer": {"type": "integer", "description": "Layer index (0-based)"}, "col": {"type": "integer"}, "row": {"type": "integer"}, "gid": {"type": "integer", "description": "Tile GID (0=empty)"}}, "required": ["scene_path", "col", "row", "gid"]})),
            make_tool("fill_rect", "Fill a rectangle of tiles", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "layer": {"type": "integer"}, "col": {"type": "integer"}, "row": {"type": "integer"}, "width": {"type": "integer"}, "height": {"type": "integer"}, "gid": {"type": "integer"}}, "required": ["scene_path", "col", "row", "width", "height", "gid"]})),
            make_tool("get_tile", "Get the tile GID at a position", serde_json::json!({"type": "object", "properties": {"scene_path": {"type": "string"}, "layer": {"type": "integer"}, "col": {"type": "integer"}, "row": {"type": "integer"}}, "required": ["scene_path", "col", "row"]})),
        ];
        std::future::ready(Ok(ListToolsResult { tools, ..Default::default() }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let name = request.name.to_string();
        let args = match &request.arguments {
            Some(arc_map) => {
                let map: &serde_json::Map<String, serde_json::Value> = arc_map;
                map.clone()
            }
            None => serde_json::Map::new(),
        };
        async move {
            self.handle_tool(&name, &args).await
        }
    }
}
