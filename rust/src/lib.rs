use std::collections::HashMap;
use std::collections::HashSet;
use godot::prelude::*;
use godot::classes::*;

pub mod nodes;
pub mod handle;
pub mod items;
pub mod simulation;
pub mod tests;
mod registry;
mod heat;
mod systems;
mod common;

pub use systems::*;
pub use handle::*;
pub use items::*;
pub use simulation::*;

use crate::nodes::FluidHatchNode;
use crate::nodes::HatchNode;
use crate::nodes::PoleNode;
use crate::registry::DefaultRegistry;
use crate::registry::Registry;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}


#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct FactoryManager {
    base: Base<Node3D>,
    real_time: f32,
    game: Simulation<DefaultRegistry>,
    godot_item_models_registry: HashMap<u8, Gd<PackedScene>>,
    godot_item_materials_registry: HashMap<u8, Gd<Material>>,
    item_prefab: Gd<PackedScene>,
}


#[godot_api]
impl INode3D for FactoryManager {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            real_time: 0f32,
            game: Simulation::<DefaultRegistry>::default(),
            godot_item_models_registry: HashMap::default(),
            godot_item_materials_registry: HashMap::default(),
            item_prefab: Gd::default()
        }
    }

    fn enter_tree(&mut self) {
        self.game.registry = DefaultRegistry::build();

        
        let mut models = HashMap::<String, Option<Gd<PackedScene>>>::default();
        let mut materials = HashMap::<String, Option<Gd<Material>>>::default();
        
        let items = self.game.registry.list_registry_items();
        for (index, item) in items.enumerate() {
            if index != 0 {
                if !item.data.item_model_resource.is_empty() {
                    models.insert(item.data.item_model_resource.to_string(), None);
                }

                if !item.data.item_material_resource.is_empty() {
                    materials.insert(item.data.item_material_resource.to_string(), None);
                }
            }
        }

        godot::prelude::godot_print!("# of model resources to load in registry: {}", models.len());
        godot::prelude::godot_print!("# of material resources to load in registry: {}", materials.len());
        
        // load all models
        for (path, dst) in models.iter_mut() {
            *dst = Some(load::<PackedScene>(path));
        }

        // load all materials
        for (path, dst) in materials.iter_mut() {
            *dst = Some(load::<Material>(path));
        }

        // assign references to model and materials to the items loaded in registry
        let items = self.game.registry.list_registry_items();
        for (index, item) in items.enumerate() {
            if index != 0 {
                self.godot_item_models_registry.insert(index as u8, models[item.data.item_model_resource].as_ref().unwrap().clone());
                self.godot_item_materials_registry.insert(index as u8, materials[item.data.item_material_resource].as_ref().unwrap().clone());
            }
        }

        self.item_prefab = load::<PackedScene>("res://actors/item.tscn");
    }

    fn process(&mut self, delta: f32) {
        self.real_time += delta;
    }

    fn physics_process(&mut self, _delta: f32) {
        // TODO: move to custom loop
        self.game.tick();
    }
}


#[godot_api]
impl FactoryManager {
    #[func]
    fn are_poles_connected(&mut self, fst: Gd<PoleNode>, snd: Gd<PoleNode>) -> bool {
        self.game.are_poles_connected(fst.bind().key, snd.bind().key)
    }

    #[func]
    fn are_hatches_connected(&mut self, fst: Gd<HatchNode>, snd: Gd<HatchNode>) -> bool {
        self.game.are_hatches_connected(fst.bind().key, snd.bind().key)
    }

    #[func]
    fn is_hatch_connected(&mut self, hatch: Gd<HatchNode>) -> bool {
        self.game.is_hatch_connected(hatch.bind().key)
    }

    #[func]
    fn are_fluid_hatches_connected(&mut self, fst: Gd<FluidHatchNode>, snd: Gd<FluidHatchNode>) -> bool {
        self.game.are_fluid_hatches_connected(fst.bind().key, snd.bind().key)
    }

    #[func]
    fn is_fluid_hatch_connected(&mut self, hatch: Gd<FluidHatchNode>) -> bool {
        self.game.is_fluid_hatch_connected(hatch.bind().key)
    }
}