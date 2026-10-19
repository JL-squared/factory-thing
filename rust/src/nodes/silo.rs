use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct SiloNode {
    base: Base<Node3D>,
    key: SiloKey,
    
    #[export]
    starting_item_id: u32,

    #[export]
    starting_item_count: u32,

    #[export]
    starting_item_stacks: u32,
}

#[godot_api]
impl INode3D for SiloNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: SiloKey::null(),
            starting_item_id: 0,
            starting_item_count: 0,
            starting_item_stacks: 0,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        self.key = bound.game.add_silo();

        bound.game.silos[self.key].stack.extend(std::iter::repeat(Item::new(self.starting_item_id as u8, self.starting_item_count as u8)).take(self.starting_item_stacks as usize));

        for child in self.base().get_children().iter_shared() {
            if let Ok(mut hatch_node) = child.try_cast::<HatchNode>() {
                let mut node = hatch_node.bind_mut();

                // update the HatchNode key based on the generated hatch keys when inserting into the manager
                if node.input_hatch {
                    node.key = bound.game.silos[self.key].input;
                } else {
                    node.key = bound.game.silos[self.key].output;
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

        bound.game.remove_silo(self.key);
    }
}

#[godot_api]
impl SiloNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let registry = &bound.game.registry;

        let silo = &bound.game.silos[self.key];


        let mut total = HashMap::<u8, u32>::new();

        for stack in silo.stack.iter() {
            *total.entry(stack.id).or_default() += stack.count as u32;
        }

        let mut str = String::new();
        for (id, count) in total {
            let name = registry.name(id);
            str += &format!("{name}x{count},");
        }


        GString::from_str(&str).unwrap()
    }
}

