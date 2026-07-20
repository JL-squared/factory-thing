use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct SplitterNode {
    base: Base<Node3D>,
    key: SplitterKey,
}

#[godot_api]
impl INode3D for SplitterNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: SplitterKey::null(),
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        self.key = bound.game.add_splitter();

        if let Ok(mut hatch_node) = self.base().find_child("Output Hatch 1").unwrap().try_cast::<HatchNode>() {
            let mut node = hatch_node.bind_mut();
            node.key = bound.game.splitters[self.key].output[0]
        }

        if let Ok(mut hatch_node) = self.base().find_child("Output Hatch 2").unwrap().try_cast::<HatchNode>() {
            let mut node = hatch_node.bind_mut();
            node.key = bound.game.splitters[self.key].output[1]
        }

        if let Ok(mut hatch_node) = self.base().find_child("Input Hatch").unwrap().try_cast::<HatchNode>() {
            let mut node = hatch_node.bind_mut();
            node.key = bound.game.splitters[self.key].input
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

        bound.game.remove_splitter(self.key);
    }
}

