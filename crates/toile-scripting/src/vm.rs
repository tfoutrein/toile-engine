use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mlua::{Function, Lua, Result as LuaResult, Table};

/// Manages the Lua VM and loaded script modules.
pub struct ScriptVm {
    lua: Lua,
    loaded_scripts: HashMap<PathBuf, mlua::RegistryKey>,
}

impl ScriptVm {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // Sandbox: remove dangerous modules
        {
            let globals = lua.globals();
            let _ = globals.set("os", mlua::Value::Nil);
            let _ = globals.set("io", mlua::Value::Nil);
            let _ = globals.set("loadfile", mlua::Value::Nil);
            let _ = globals.set("dofile", mlua::Value::Nil);
        }

        // Register engine.log
        {
            let engine_table = lua.create_table()?;
            engine_table.set(
                "log",
                lua.create_function(|_, msg: String| {
                    log::info!("[Lua] {msg}");
                    Ok(())
                })?,
            )?;
            lua.globals().set("engine", engine_table)?;
        }

        Ok(Self {
            lua,
            loaded_scripts: HashMap::new(),
        })
    }

    /// Load a Lua script file. The script must return a table with
    /// optional on_create, on_update, on_destroy functions.
    pub fn load_script(&mut self, path: &Path) -> LuaResult<()> {
        let source = std::fs::read_to_string(path).map_err(mlua::Error::external)?;

        let chunk = self.lua.load(&source).set_name(path.to_string_lossy());
        let module: Table = chunk.call::<Table>(())?;

        let key = self.lua.create_registry_value(module)?;

        if let Some(old_key) = self.loaded_scripts.insert(path.to_path_buf(), key) {
            self.lua.remove_registry_value(old_key)?;
        }

        log::info!("Loaded script: {}", path.display());
        Ok(())
    }

    pub fn call_on_create(&self, path: &Path, entity_id: u64) -> LuaResult<()> {
        if let Some(key) = self.loaded_scripts.get(path) {
            let module: Table = self.lua.registry_value(key)?;
            if let Ok(func) = module.get::<Function>("on_create") {
                func.call::<()>(entity_id)?;
            }
        }
        Ok(())
    }

    pub fn call_on_update(&self, path: &Path, entity_id: u64, dt: f64) -> LuaResult<()> {
        if let Some(key) = self.loaded_scripts.get(path) {
            let module: Table = self.lua.registry_value(key)?;
            if let Ok(func) = module.get::<Function>("on_update") {
                func.call::<()>((entity_id, dt))?;
            }
        }
        Ok(())
    }

    pub fn call_on_destroy(&self, path: &Path, entity_id: u64) -> LuaResult<()> {
        if let Some(key) = self.loaded_scripts.get(path) {
            let module: Table = self.lua.registry_value(key)?;
            if let Ok(func) = module.get::<Function>("on_destroy") {
                func.call::<()>(entity_id)?;
            }
        }
        Ok(())
    }

    pub fn reload_scripts(&mut self, paths: &[PathBuf]) {
        for path in paths {
            if self.loaded_scripts.contains_key(path) {
                match self.load_script(path) {
                    Ok(()) => log::info!("Hot-reloaded: {}", path.display()),
                    Err(e) => log::error!("Reload failed {}: {e}", path.display()),
                }
            }
        }
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}
