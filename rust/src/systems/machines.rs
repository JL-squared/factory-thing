use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::num::NonZeroU16;

use fastrand::Rng;
use slotmap::SlotMap;

use crate::*;
use crate::handle::*;
use crate::common::*;
use crate::registry::*;
use crate::items::*;



#[derive(Debug)]
pub struct Progress {
    pub ticks_remaining: NonZeroU16,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum MachineStatus {
    RecipeInputResourcesMismatchOrEmpty,
    RecipeOutputResourcesMismatchOrFull,
    
    NonPowered,

    ClickyThingNotAttached,

    #[default]
    None,
}

#[derive(Default, Debug)]
pub struct Machine {
    pub input: Vec<HatchKey>,
    pub output: Vec<HatchKey>,
    pub recipe: Option<String>,
    pub progress: Option<Progress>,
    pub status: MachineStatus,
    pub pole: Option<PoleKey>,
    pub clicky_thing_attached: bool,
    pub internal_power_buffer: LoadUnit,
}

#[derive(Default, Debug)]
pub struct Miner {
    pub output: HatchKey,
    pub recipe: Option<String>,
    pub progress: Option<Progress>,
    pub status: MachineStatus,
    pub pole: Option<PoleKey>,
    pub clicky_thing_attached: bool,
    pub internal_power_buffer: LoadUnit,
}


pub struct Commons<'a, R: GenericRecipe> {
    recipe: &'a R,
    consumer_pole_id: PoleKey,
    internal_power_buffer: &'a mut LoadUnit,
    status: &'a mut MachineStatus,
    clicky_thing_attached: bool,
    progress: &'a mut Option<Progress>,

    inputs: Vec<HatchKey>,
    outputs: Vec<HatchKey>,
    
}

pub trait GenericRecipe {
    fn load(&self) -> LoadUnit;
    fn ticks(&self) -> u16;
    fn inputs(&self) -> Vec<Item>;
    fn outputs(&self) -> Vec<Item>;
}

pub trait GenericMachine {
    type Key: slotmap::Key;
    type Recipe: GenericRecipe;

    fn get_commons<'a, R: Registry>(&'a mut self, registry: &'a R) -> Commons<'a, Self::Recipe>;
    fn do_thing_machine<R: Registry>(recipe: &Self::Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng, registry: &R) {}
}

impl GenericRecipe for MinerRecipe {
    fn load(&self) -> LoadUnit {
        self.load
    }

    fn ticks(&self) -> u16 {
        self.ticks
    }

    fn inputs(&self) -> Vec<Item> {
        Vec::default()
    }

    fn outputs(&self) -> Vec<Item> {
        vec![self.output]
    }
}

impl GenericMachine for Miner {
    type Key = MinerKey;
    type Recipe = MinerRecipe;


    fn get_commons<'a, R: Registry>(&'a mut self, registry: &'a R) -> Commons<'a, Self::Recipe> {
        Commons {
            recipe: registry.registry_miner_recipe(self.recipe.as_ref().unwrap()),
            consumer_pole_id: self.pole.unwrap(),
            internal_power_buffer: &mut self.internal_power_buffer,
            status: &mut self.status,
            clicky_thing_attached: self.clicky_thing_attached,
            progress: &mut self.progress,
            inputs: Vec::default(),
            outputs: vec![self.output]
        }
    }

    fn do_thing_machine<R: Registry>(recipe: &Self::Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng, registry: &R) {
        if rng.bool() {
            let buffer = &mut hatches[outputs[0]].buffer;
            buffer.accumulate::<R>(&recipe.output, registry);
        }
    }
}



impl GenericRecipe for Recipe {
    fn load(&self) -> LoadUnit {
        self.load
    }

    fn ticks(&self) -> u16 {
        self.ticks
    }

    fn inputs(&self) -> Vec<Item> {
        self.input.to_vec()
    }

    fn outputs(&self) -> Vec<Item> {
        self.output.to_vec()
    }
}

impl GenericMachine for Machine {
    type Key = MachineKey;
    type Recipe = Recipe;


    fn get_commons<'a, R: Registry>(&'a mut self, registry: &'a R) -> Commons<'a, Self::Recipe> {
        Commons {
            recipe: registry.registry_recipe(&self.recipe.as_ref().unwrap()),
            consumer_pole_id: self.pole.unwrap(),
            internal_power_buffer: &mut self.internal_power_buffer,
            status: &mut self.status,
            clicky_thing_attached: self.clicky_thing_attached,
            progress: &mut self.progress,
            inputs: self.input.clone(),
            outputs: self.output.clone()
        }
    }

    fn do_thing_machine<R: registry::Registry>(recipe: &Self::Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng, registry: &R) {
        // machine finished the recipe (remaining ticks is zero, but no need to update it, as we invalidate `progress` anyways)
        // take items from input hatches
        for (recipe_input, hatch_input) in
            recipe.input.iter().zip(inputs.iter())
        {
            let buffer = &mut hatches[*hatch_input].buffer;
            buffer.take(recipe_input);
        }

        // put items in output hatches
        for (recipe_output, hatch_output) in
            recipe.output.iter().zip(outputs.iter())
        {
            let buffer = &mut hatches[*hatch_output].buffer;
            buffer.accumulate::<R>(recipe_output, registry);
        }
    }
}


pub fn handle_generic_machine<R: registry::Registry, M: GenericMachine>(machines: &mut SlotMap<M::Key, M>, poles: &mut SlotMap<PoleKey, Pole>, hatches: &mut SlotMap<HatchKey, Hatch>, settings: &mut Settings, rng: &mut Rng, registry: &R) {
    for machine in machines.values_mut() {
        let Commons {
            recipe,
            consumer_pole_id,
            internal_power_buffer,
            status,
            clicky_thing_attached,
            progress,
            inputs,
            outputs
        } = machine.get_commons(registry);

        // set the machine's consumer pole to enabled state
        let given_load = match poles[consumer_pole_id] {
            Pole::Consumer { current_load, .. } => current_load,
            _ => unreachable!()
        };

        *internal_power_buffer += given_load;

        // try to buffer 2 recipes worth of power in internal power buffer
        if *internal_power_buffer < recipe.load() * 2 {
            poles[consumer_pole_id] = Pole::Consumer {
                target_load: recipe.load(),
                current_load: 0,
            };
        } else {
            poles[consumer_pole_id] = Pole::Consumer {
                target_load: 0,
                current_load: 0,
            };
        }

        if settings.machine_require_clicky_thing_attached && !clicky_thing_attached {
            *status = MachineStatus::ClickyThingNotAttached;
            continue;
        }

        if settings.machine_require_power && *internal_power_buffer < recipe.load() {
            *status = MachineStatus::NonPowered;
            continue;
        }

        if let Some(valid_progress) = progress.as_mut() {
            // check internal power buffer
            *status = MachineStatus::None;
            *internal_power_buffer -= recipe.load();

            // machine is currently progressing through the recipe, take one tick off
            // TODO: add pause / stop / resume functionality here
            // TODO: the thing that the machine does *during* progress
            let non_zero = NonZeroU16::new(valid_progress.ticks_remaining.get() - 1);

            if let Some(non_zero) = non_zero {
                // number of ticks is non-zero, update, and continue
                valid_progress.ticks_remaining = non_zero;
            } else {
                // the thing that the machine does at the end of progress
                M::do_thing_machine::<R>(recipe, &inputs, &outputs, hatches, rng, registry);

                // reset machine progress
                progress.take().unwrap();
            }
        }

        if progress.is_none() {
            // assert_eq!(recipe.input.len(), machine.input.len(), "machine recipe input items count and input hatches count do not match");
            // assert_eq!(recipe.output.len(), machine.output.len(), "machine recipe output items count and output hatches count do not match");
            let inputs_match_recipe_input =
                recipe.inputs().iter().zip(inputs.iter()).all(
                    |(recipe_input_item, input_hatch)| {
                        let buffer = hatches[*input_hatch].buffer;
                        buffer.id == recipe_input_item.id
                            && buffer.count >= recipe_input_item.count
                    },
                );
            let outputs_match_recipe_output =
                recipe.outputs().iter().zip(outputs.iter()).all(
                    |(recipe_output_item, output_hatch)| {
                        let buffer = hatches[*output_hatch].buffer;
                        if buffer.is_invalid() {
                            return true;
                        }

                        // if the item is the same, must make sure that we have enough space in the hatch to place it 
                        let same_id = buffer.id == recipe_output_item.id;
                        let stack_size = registry.stack_size(buffer.id);

                        // this CAN overflow if stack size is at MAX
                        // if we know it will overflow, then we cannot process the recipe
                        let opt_non_overflowing_enough_space_considering_stack_size = buffer.count.checked_add(recipe_output_item.count).map(|x| x <= stack_size);
                        same_id && opt_non_overflowing_enough_space_considering_stack_size.unwrap_or_default()
                    },
                );

            // if requirements are met, then we can begin machine progress
            if inputs_match_recipe_input && outputs_match_recipe_output {
                if progress.is_none() {
                    // godot::global::godot_print!("new progress");
                    progress.replace(Progress {
                        ticks_remaining: NonZeroU16::new(recipe.ticks())
                            .expect("recipe ticks must not be zero"),
                    });
                }
            } else {
                if !inputs_match_recipe_input {
                    *status = MachineStatus::RecipeInputResourcesMismatchOrEmpty;
                }

                if !outputs_match_recipe_output {
                    *status = MachineStatus::RecipeOutputResourcesMismatchOrFull;
                }
            }
        }
    }
}