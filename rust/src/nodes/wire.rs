use super::*;

use godot::classes::*;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct WireNode {
    base: Base<Node3D>,
    key: WireKey,

    #[export]
    pole_1_ref: Option<Gd<PoleNode>>,
    #[export]
    pole_2_ref: Option<Gd<PoleNode>>,
    #[export]
    hologram: bool,
}

#[godot_api]
impl INode3D for WireNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: WireKey::null(),
            pole_1_ref: None,
            pole_2_ref: None,
            hologram: false
        }
    }

    fn ready(&mut self) {
        if self.hologram {
            return;
        }

        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut pole_1 = self.pole_1_ref.clone().unwrap();
        let mut pole_1 = pole_1.bind_mut();
        let pole_1_key = pole_1.key;
        pole_1.signals().tree_exiting().connect_other(self, Self::pole_destroyed);

        
        let mut pole_2 = self.pole_2_ref.clone().unwrap();
        let mut pole_2 = pole_2.bind_mut();
        let pole_2_key = pole_2.key;
        pole_2.signals().tree_exiting().connect_other(self, Self::pole_destroyed);
        
        self.key = bound.game.add_wire(pole_1_key, pole_2_key);
    }

    // TODO: not optimized, should use a callback with user data (godot nodes) instead
    fn physics_process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let sim = &bound.game;

        if !sim.wires.contains_key(self.key) {
            self.base_mut().queue_free();
        }
    }

    fn exit_tree(&mut self) {
        if self.hologram {
            return;
        }

        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_wire(self.key);
    }
}

#[godot_api]
impl WireNode {
    fn pole_destroyed(&mut self) {
        self.base_mut().queue_free();
    }

    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let wire = &bound.game.wires[self.key];
        let text = format!("a: {:?}, b: {:?}, flow: {}", wire.a, wire.b, wire.flow);
        
        GString::from_str(&text).unwrap()
    }
}