use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SceneData {
    pub name: String,
    pub entities: Vec<EntityData>,
    #[serde(skip)]
    pub next_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityData {
    pub id: u64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub layer: i32,
    #[serde(default)]
    pub sprite_path: String,
    pub width: f32,
    pub height: f32,
}

impl SceneData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_entity(&mut self, name: &str, x: f32, y: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(EntityData {
            id,
            name: name.to_string(),
            x,
            y,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            layer: 0,
            sprite_path: String::new(),
            width: 32.0,
            height: 32.0,
        });
        id
    }

    pub fn remove_entity(&mut self, id: u64) {
        self.entities.retain(|e| e.id != id);
    }

    pub fn find_entity_mut(&mut self, id: u64) -> Option<&mut EntityData> {
        self.entities.iter_mut().find(|e| e.id == id)
    }
}
