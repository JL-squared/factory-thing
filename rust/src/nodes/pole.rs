use super::*;

use godot::classes::*;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct PoleNode {
    base: Base<Node3D>,
    pub key: PoleKey,

    #[var]
    pub owned: bool,
}

#[godot_api]
impl INode3D for PoleNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: PoleKey::null(),
            owned: false,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        // add generic pole, owning machines can change the type (to consumer / generator) afterwards... 
        self.key = bound.game.add_pole();
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();
        
        bound.game.remove_pole(self.key);
    }
}

#[godot_api]
impl PoleNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let text = format!("id: {:?}", self.key);

        GString::from_str(&text).unwrap()
    }
}
