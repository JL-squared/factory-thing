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
        let length = Vector3::distance_to(a,b);

        self.key = bound.game.add_pipe(output_hatch, input_hatch, length);
        godot::global::godot_print!("{:?}", self.key);
    }

    
    // TODO: because some physics tick can be skipped (when running in sped-up mode, for example), this can cause a desync between world time and tick time
    // easiest fix is to get this to run on a tick basis instead. this *will* give chopped animations...
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

        /*
        // https://github.com/JL-squared/Factory-ECS-Prototype/blob/main/Assets/Actors/Systems/VisualSystems.cs#L625
        let Pipe = &game.Pipes[self.key];
        let last_transfer_tick = Pipe.last_transfer_tick;

        let Pipe_ticks_between_transfers = game.settings.Pipe_ticks_between_transfers;
        let diff = (game.tick - last_transfer_tick) as f32 / Pipe_ticks_between_transfers as f32;

        // let did_transfer_this_tick = last_transfer_tick == game.tick;

        let tick_interpolation_factor = diff;
        let tick_interpolation_factor = tick_interpolation_factor.clamp(0f32, 1f32);

        let a = self.Pipe_start_hatch_ref.as_ref().unwrap().bind().base().get_global_position();
        let b = self.Pipe_end_hatch_ref.as_ref().unwrap().bind().base().get_global_position();

        // TODO: don't spawn / despawn entities every frame bruh
        for x in self.items.drain(..) {
            x.unwrap().free();
        }

        for (i, elem) in Pipe.buffer.iter().enumerate() {
            let factor_1 = i as f32 / (Pipe.buffer.len() as f32);
            let factor_2 = (i+1) as f32 / (Pipe.buffer.len() as f32);
            let factor = f32::lerp(factor_1, factor_2, tick_interpolation_factor);
            
            
            if !elem.is_invalid() {
                let mut item_node = bound.item_prefab.instantiate().unwrap();


                let model_node = bound.godot_item_models_registry[&elem.id].instantiate().unwrap();
                let material = &bound.godot_item_materials_registry[&elem.id];
                
                
                for child in model_node.get_children().iter_shared() {
                    if let Ok(mut mesh_instance) = child.try_cast::<MeshInstance3D>() {
                        mesh_instance.set_material_override(material);
                    }
                }



                item_node.add_child(&model_node);

                self.base_mut().add_child(&item_node);
                
                let mut instance = item_node.cast::<Node3D>();
                let lerped = Vector3::lerp(a, b, factor);
                
                instance.set_global_position(lerped);
                self.items.push(Some(instance));
            }
        }
        */
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

        /*
        let Pipe = &bound.game.Pipes[self.key];
        let last_transfer_tick = Pipe.last_transfer_tick;

        let text = format!("id: {:?}, LTT: {}, buf-len: {}", self.key, last_transfer_tick, Pipe.buffer.len());
        */
        let text = String::new();
        GString::from_str(&text).unwrap()
    }
}


