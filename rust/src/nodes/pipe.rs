use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct PipeNode {
    base: Base<Node3D>,

    #[export]
    pipe_start_hatch_ref: Option<Gd<FluidHatchNode>>,
    #[export]
    pipe_end_hatch_ref: Option<Gd<FluidHatchNode>>,

    key: PipeKey,

    #[export]
    hologram: bool,
    
}

#[godot_api]
impl INode3D for PipeNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            pipe_start_hatch_ref: None,
            pipe_end_hatch_ref: None,
            key: PipeKey::null(),
            hologram: false,
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

        let start_hatch = self.pipe_start_hatch_ref.clone().unwrap();
        start_hatch.signals().tree_exiting().connect_other(self, Self::hatch_destroyed);
        let start_hatch = start_hatch.bind();


        let end_hatch = self.pipe_end_hatch_ref.clone().unwrap();
        end_hatch.signals().tree_exiting().connect_other(self, Self::hatch_destroyed);
        let end_hatch = end_hatch.bind();

        let input_hatch = end_hatch.key;
        let output_hatch = start_hatch.key;
        
        let a = start_hatch.base().get_global_position();
        let b = end_hatch.base().get_global_position();
        let length = crate::simulation::BufferSize::WorldLength(Vector3::distance_to(a,b));

        self.key = bound.game.add_pipe(output_hatch, input_hatch, length);
        godot::global::godot_print!("{:?}", self.key);
    }

    fn physics_process(&mut self, _delta: f32) {
        if self.hologram {
            return;
        }

        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let game = &bound.game;
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

        bound.game.remove_pipe(self.key);
    }
}

#[godot_api]
impl PipeNode {
    fn hatch_destroyed(&mut self) {
        self.base_mut().queue_free();
    }

    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let pipe = &bound.game.pipes[self.key];

        let total_volume = pipe.buffer.iter().sum::<f32>();
        let text = format!("volume: {total_volume}");
        GString::from_str(&text).unwrap()
    }
}


