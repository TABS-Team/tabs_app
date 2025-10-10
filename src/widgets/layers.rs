use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiLayer {
    Overlay, // Game overlays, in a traditional game you would put a healthbar here
    Menus,   // Floating windows
    Debug,   // Debug windows
}

#[derive(Resource, Default)]
pub struct UiLayerStack {
    pub stacks: HashMap<UiLayer, VecDeque<Entity>>,
}

impl UiLayerStack {
    pub fn get_highest_z_index(&self, layer: UiLayer, windows: &Query<&ZIndex>) -> i32 {
        self.stacks
            .get(&layer)
            .and_then(|entities| {
                entities
                    .iter()
                    .filter_map(|entity| windows.get(*entity).ok())
                    .map(|z| z.0)
                    .max()
            })
            .unwrap_or(layer.base_z())
    }

    pub fn recalculate_z_order(&self, layer: UiLayer, commands: &mut Commands) {
        if let Some(queue) = self.stacks.get(&layer) {
            let base = layer.base_z();
            for (i, &entity) in queue.iter().enumerate() {
                commands
                    .entity(entity)
                    .insert(GlobalZIndex(base + (i as i32) + 1));
            }
        }
    }

    pub fn push(&mut self, layer: UiLayer, entity: Entity, commands: &mut Commands) {
        let queue = self.stacks.entry(layer).or_default();
        let z_index = layer.base_z() + (queue.len() as i32) + 1;
        commands.entity(entity).insert(GlobalZIndex(z_index));
        queue.push_back(entity);
    }

    pub fn remove(&mut self, layer: UiLayer, entity: Entity, commands: &mut Commands) {
        if let Some(queue) = self.stacks.get_mut(&layer) {
            queue.retain(|&e| e != entity);
            self.recalculate_z_order(layer, commands);
        }
    }

    pub fn bring_to_front(&mut self, layer: UiLayer, entity: Entity, commands: &mut Commands) {
        if let Some(queue) = self.stacks.get_mut(&layer) {
            if let Some(index) = queue.iter().position(|&e| e == entity) {
                if index == queue.len() - 1 {
                    return;
                }
                queue.remove(index);
                queue.push_back(entity);
                self.recalculate_z_order(layer, commands);
            }
        }
    }
}

// If someone spawns more than 10000 ui windows per layer... I will haunt them after I'm gone
impl UiLayer {
    pub fn base_z(self) -> i32 {
        match self {
            UiLayer::Overlay => 0,
            UiLayer::Menus => 10_000,
            UiLayer::Debug => 20_000,
        }
    }

    pub fn base_camera_order(self) -> isize {
        match self {
            UiLayer::Overlay => 0,
            UiLayer::Menus => 10_000,
            UiLayer::Debug => 20_000,
        }
    }
}
