use crate::registry::MinerRecipe;

use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct MinerNode {
    base: Base<Node3D>,
    key: MinerKey,
    pole_key: PoleKey,
}

#[godot_api]
impl INode3D for MinerNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: MinerKey::null(),
            pole_key: PoleKey::null(),
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
        
        const RECIPE: MinerRecipe = MinerRecipe { id: "", name: "", output: Item::new(DefaultRegistry::RAW_IRON_1, 1), ticks: 128, load: 30 };
        self.key = bound.game.add_miner_with_pole(&RECIPE, pole.key);

        if let Ok(mut hatch_node) = self.base().find_child("Output Hatch").unwrap().try_cast::<HatchNode>() {
            let mut node = hatch_node.bind_mut();
            node.key = bound.game.miners[self.key].output
        }

        self.base().find_child("Clicky Clicky Thing").unwrap().connect("clicky_thing_attached", &self.base().callable("attach_clicky_thing"));
        self.base().find_child("Clicky Clicky Thing").unwrap().connect("clicky_thing_detached", &self.base().callable("detach_clicky_thing"));
    }

    fn process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let machine = &bound.game.miners[self.key];
        let progressing = machine.progress.is_some() && machine.status == MachineStatus::None;
        
        let smoke = self.base().find_child("Smoke Vfx").unwrap();
        let mut smoke = smoke.cast::<GpuParticles3D>();
        smoke.set_emitting(progressing);
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_miner(self.key);
    }
}

#[godot_api]
impl MinerNode {
    #[func]
    fn get_ui_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let machine = &bound.game.miners[self.key];
        let recipe = machine.recipe.unwrap();
        
        let string = format!("machine. recipe: '{}'. status: {:?}", recipe.id, machine.status);
        
        GString::from_str(&string).unwrap()
    }

    #[func]
    fn get_debug_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let machine = &bound.game.miners[self.key];
        
        let string = format!("{:#?}", machine);
        
        GString::from_str(&string).unwrap()
    }

    #[func]
    fn get_ui_progress_bar_percentage(&mut self) -> f32 {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let machine = &bound.game.miners[self.key];
        let recipe = machine.recipe.unwrap();

        let progress = match &machine.progress {
            Some(progress) => {
                let remaining = progress.ticks_remaining.get();
                let total = recipe.ticks;
                let current = (total - remaining) as f32;
                let factor = current / total as f32;

                100f32 * factor
            },
            None => 0f32,
        };

        progress
    }

    #[func]
    fn get_ui_recipe_info(&mut self) -> GString {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();
        let machine = &bound.game.miners[self.key];
        let recipe = machine.recipe.unwrap();

        let string = format!("{} EU/t => {} ({}t)", recipe.load, recipe.output.display::<DefaultRegistry>(), recipe.ticks);
        
        GString::from_str(&string).unwrap()
    }

    
    #[func]
    fn attach_clicky_thing(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();
        let machine = &mut bound.game.miners[self.key];
        machine.clicky_thing_attached = true;
    }

        
    #[func]
    fn detach_clicky_thing(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();
        let machine = &mut bound.game.miners[self.key];
        machine.clicky_thing_attached = false;
    }
}
