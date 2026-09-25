use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct FluidHatchNode {
    base: Base<Node3D>,

    #[export]
    pub hatch_index: u32,
    
    #[export]
    pub input_hatch: bool,

    pub key: FluidHatchKey,
}

#[godot_api]
impl INode3D for FluidHatchNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            input_hatch: false,
            hatch_index: 0,
            key: FluidHatchKey::null()
        }
    }

    
    fn process(&mut self, _delta: f32) {
    }
}

#[godot_api]
impl FluidHatchNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();

        //let item = bound.game.fluid_hatches[self.key].buffer;
        //let item_text = item.display::<DefaultRegistry>();
        //let text = format!("id: {:?}. {}", self.key, item_text);
        let text = String::new();
        GString::from_str(&text).unwrap()
    }
}
