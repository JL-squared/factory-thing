use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct GeneratorNode {
    base: Base<Node3D>,
    
    #[export]
    load: u32,

    pole_key: PoleKey,
}

#[godot_api]
impl INode3D for GeneratorNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            load: 0,
            pole_key: PoleKey::null()
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut _pole = self.base().get_node_as::<PoleNode>("Power Pole");
        let mut pole = _pole.bind_mut();
        pole.owned = true;
        self.pole_key = pole.key;

        bound.game.add_generator_with_pole(self.load as LoadUnit, pole.key);
    }
}

#[godot_api]
impl GeneratorNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let pole = &bound.game.poles[self.pole_key];

        let text = match pole {
            Pole::Generator { max_load, current_load } => format!("max load: {}, current load: {}", max_load, current_load),
            _ => unreachable!(),
        };

        GString::from_str(&text).unwrap()
    }
}
