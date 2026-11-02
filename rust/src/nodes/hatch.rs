use super::*;

use godot::classes::*;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct HatchNode {
    base: Base<Node3D>,

    #[export]
    pub hatch_index: u32,
    
    #[export]
    pub input_hatch: bool,

    pub key: HatchKey,
}

#[godot_api]
impl INode3D for HatchNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            input_hatch: false,
            hatch_index: 0,
            key: HatchKey::null()
        }
    }

    
    fn process(&mut self, _delta: f32) {
    }
}

#[godot_api]
impl HatchNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let item = bound.game.hatches[self.key].buffer;
        let registry = &bound.game.registry;
        let text = item.iter().map(|x| x.display(registry)).fold(String::new(), |a, b| a+","+&b);

        GString::from_str(&text).unwrap()
    }


    #[func]
    fn get_debug_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let hatch = &bound.game.hatches[self.key];
        
        let string = format!("{:#?}", hatch);
        
        GString::from_str(&string).unwrap()
    }
}
