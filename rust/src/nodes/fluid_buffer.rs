use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct FluidBufferNode {
    base: Base<Node3D>,
    key: FluidBufferKey,
}

#[godot_api]
impl INode3D for FluidBufferNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: FluidBufferKey::null(),
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        self.key = bound.game.add_fluid_buffer();
                
        for child in self.base().get_children().iter_shared() {
            if let Ok(mut hatch_node) = child.try_cast::<FluidHatchNode>() {
                let mut node = hatch_node.bind_mut();

                // update the HatchNode key based on the generated hatch keys when inserting into the manager
                if node.input_hatch {
                    node.key = bound.game.fluid_buffers[self.key].input;
                } else {
                    node.key = bound.game.fluid_buffers[self.key].output;
                }
            }
        }
    }

    fn process(&mut self, _delta: f32) {
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_fluid_buffer(self.key);
    }
}

#[godot_api]
impl FluidBufferNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let buffer = &bound.game.fluid_buffers[self.key];

        let str = format!("v: {}", buffer.fluid);
        GString::from_str(&str).unwrap()
    }
}

