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
    pub recipe_index: usize,
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


pub struct Commons<'a, R: GenericMultiRecipe> {
    multi_recipe: &'a R,
    consumer_pole_id: PoleKey,
    internal_power_buffer: &'a mut LoadUnit,
    status: &'a mut MachineStatus,
    clicky_thing_attached: bool,
    progress: &'a mut Option<Progress>,

    inputs: Vec<HatchKey>,
    outputs: Vec<HatchKey>,
    
}

type PossiblyMultipleItems = Vec<Item>;

pub trait GenericMultiRecipe {
    type T: GenericRecipe;
    fn inner(&self) -> Vec<&Self::T>;
}

pub trait GenericRecipe {
    fn load(&self) -> LoadUnit;
    fn ticks(&self) -> u16;
    fn inputs(&self) -> Vec<Item>;
    fn outputs(&self) -> Vec<PossiblyMultipleItems>;
}

pub trait GenericMachine {
    type Key: slotmap::Key;
    type MultiRecipe: GenericMultiRecipe;

    fn check_machine_input_requirements_valid(recipe: &<Self::MultiRecipe as GenericMultiRecipe>::T, inputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>) -> bool {
        recipe.inputs().iter().zip(inputs.iter()).all(
            |(recipe_input_item, input_hatch)| {
                hatches[*input_hatch].input_item_requirement(recipe_input_item)
            },
        )
    }

    fn check_machine_output_requirements_valid<R: Registry>(recipe: &<Self::MultiRecipe as GenericMultiRecipe>::T, outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, registry: &R) -> bool {
        recipe.outputs().iter().zip(outputs.iter()).all(
            |(multiple_possible_output_items, output_hatch)| {
                let hatch = &hatches[*output_hatch];
                if hatch.is_invalid() {
                    return true;
                }

                // if we have multiple different output items (weighted probability), then make sure that ALL of those items can be outputted from the machine
                multiple_possible_output_items.iter().all(|single_item| {
                    hatch.can_accumulate_from_without_partial_takes_overflow_conservative(single_item, registry)
                })
            },
        )
    }

    fn check_subrecipe_configuration_requirement_valid(recipe: &<Self::MultiRecipe as GenericMultiRecipe>::T, configuration: ()) -> bool {
        true
    }

    fn get_commons<'a, R: Registry>(&'a mut self, registry: &'a R) -> Commons<'a, Self::MultiRecipe>;
    fn do_thing_machine<R: Registry>(recipe: &<Self::MultiRecipe as GenericMultiRecipe>::T, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng, registry: &R) {}
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

    fn outputs(&self) -> Vec<PossiblyMultipleItems> {
        // return the WORST CASE size of the item
        // this function is only used to check if there is space in the output hatch,
        // so we must be rather conservative and make sure this holds for the maximally generated stack size
        let collection = self.outputs.iter().map(|x| Item::new(x.item_id, *x.min_max_stack_size.end())).collect();
        vec![collection]
    }
}

impl GenericMultiRecipe for MinerRecipe {
    type T = Self;

    fn inner(&self) -> Vec<&Self::T> {
        vec![self]
    }
}

impl GenericMachine for Miner {
    type Key = MinerKey;
    type MultiRecipe = MinerRecipe;


    fn get_commons<'a, R: Registry>(&'a mut self, registry: &'a R) -> Commons<'a, Self::MultiRecipe> {
        Commons {
            multi_recipe: registry.registry_miner_recipe(self.recipe.as_ref().unwrap()),
            consumer_pole_id: self.pole.unwrap(),
            internal_power_buffer: &mut self.internal_power_buffer,
            status: &mut self.status,
            clicky_thing_attached: self.clicky_thing_attached,
            progress: &mut self.progress,
            inputs: Vec::default(),
            outputs: vec![self.output]
        }
    }

    fn do_thing_machine<R: Registry>(recipe: &Self::MultiRecipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng, registry: &R) {
        let hatch = &mut hatches[outputs[0]];
        if let Some(x) = rng.choice(recipe.outputs.iter()) {
            let mut item = x.generate(rng);
            hatch.transfer_from_item_limited::<R>(&mut item, 255, registry);

            // item MUST have been taken completely, no remaining item count
            assert_eq!(item.count, 0);
            assert!(item.is_invalid());
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

    fn outputs(&self) -> Vec<PossiblyMultipleItems> {
        let collection: Vec<Vec<Item>> = self.output.iter().map(|x| vec![Item::new(x.item_id, *x.min_max_stack_size.end())]).collect();
        collection
    }
}

impl GenericMultiRecipe for Recipe {
    type T = Self;

    fn inner(&self) -> Vec<&Self::T> {
        vec![self]
    }
}

impl GenericMultiRecipe for MultiRecipe {
    type T = Recipe;

    fn inner(&self) -> Vec<&Self::T> {
        self.recipes.iter().collect::<Vec<_>>()
    }
} 

impl GenericMachine for Machine {
    type Key = MachineKey;
    type MultiRecipe = MultiRecipe;


    fn get_commons<'a, R: Registry>(&'a mut self, registry: &'a R) -> Commons<'a, Self::MultiRecipe> {
        Commons {
            multi_recipe: registry.registry_multi_recipe(self.recipe.as_ref().unwrap()),
            consumer_pole_id: self.pole.unwrap(),
            internal_power_buffer: &mut self.internal_power_buffer,
            status: &mut self.status,
            clicky_thing_attached: self.clicky_thing_attached,
            progress: &mut self.progress,
            inputs: self.input.clone(),
            outputs: self.output.clone()
        }
    }

    fn do_thing_machine<R: registry::Registry>(recipe: &Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng, registry: &R) {
        // machine finished the recipe (remaining ticks is zero, but no need to update it, as we invalidate `progress` anyways)
        // take items from input hatches
        for (recipe_input, hatch_input) in
            recipe.input.iter().zip(inputs.iter())
        {
            let hatch = &mut hatches[*hatch_input];
            let taken = hatch.take(recipe_input.id, recipe_input.count, registry);
        
            // the taken item here can *change* how the recipe behaves! or the outputs of the recipe in the for loop right after this one!
            // ooo.... interesting....
        }

        // put items in output hatches
        for (recipe_output, hatch_output) in
            recipe.output.iter().zip(outputs.iter())
        {
            let hatch = &mut hatches[*hatch_output];

            let item = recipe_output.generate(rng);
            
            hatch.accumulate::<R>(&item, registry);
        }
    }
}

pub fn pre_power_handle_generic_machine<R: registry::Registry, M: GenericMachine>(machines: &mut SlotMap<M::Key, M>, poles: &mut SlotMap<PoleKey, Pole>, hatches: &mut SlotMap<HatchKey, Hatch>, settings: &mut Settings, rng: &mut Rng, registry: &R) {
    for machine in machines.values_mut() {
        let Commons {
            multi_recipe,
            consumer_pole_id,
            internal_power_buffer,
            status,
            clicky_thing_attached,
            progress,
            inputs,
            outputs
        } = machine.get_commons(registry);

        if progress.is_none() {
            // linear search through all the sub-recipes until we find one that we can execute...
            for (recipe_index, recipe) in multi_recipe.inner().iter().enumerate() {
                let inputs_match_recipe_input = M::check_machine_input_requirements_valid(recipe, &inputs, hatches);
                let outputs_match_recipe_output = M::check_machine_output_requirements_valid(recipe, &outputs, hatches, registry);

                // TODO: implement machine configuration
                let configuration_matches = M::check_subrecipe_configuration_requirement_valid(recipe, ());

                if inputs_match_recipe_input && outputs_match_recipe_output && configuration_matches {
                    // godot::global::godot_print!("new progress");
                    // TODO: should we consume the input items (if any) here, instead of waiting till the recipe is complete first?

                    progress.replace(Progress {
                        ticks_remaining: NonZeroU16::new(recipe.ticks())
                            .expect("recipe ticks must not be zero"),
                        recipe_index,
                    });
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

        if let Some(progress) = progress {
            dbg!("what...");
            let recipe = multi_recipe.inner()[progress.recipe_index];

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
        }

    }
}

pub fn post_power_handle_generic_machine<R: registry::Registry, M: GenericMachine>(machines: &mut SlotMap<M::Key, M>, poles: &mut SlotMap<PoleKey, Pole>, hatches: &mut SlotMap<HatchKey, Hatch>, settings: &mut Settings, rng: &mut Rng, registry: &R) {
    for machine in machines.values_mut() {
        let Commons {
            multi_recipe,
            consumer_pole_id,
            internal_power_buffer,
            status,
            clicky_thing_attached,
            progress,
            inputs,
            outputs
        } = machine.get_commons(registry);

        if let Some(valid_progress) = progress.as_mut() {
            let recipe = multi_recipe.inner()[valid_progress.recipe_index];
            if settings.machine_require_power && *internal_power_buffer < recipe.load() {
                *status = MachineStatus::NonPowered;
                continue;
            }

            
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
    }
}