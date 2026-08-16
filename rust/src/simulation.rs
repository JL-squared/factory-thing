use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU16,
};

use fastrand::Rng;
use slotmap::{SlotMap, new_key_type};




use crate::{items::*, registry::{self, MinerRecipe, Recipe}};
use crate::handle::*;

pub type LoadUnit = isize;
pub type HealthUnit = u8;

#[derive(Debug)]
pub struct Belt {
    pub belt_start: HatchKey,
    pub belt_end: HatchKey,
    pub buffer: Vec<Item>,
    pub last_transfer_tick: u64,
}

pub enum BeltSize {
    BufferLength(u32),
    WorldLength(f32),
}

#[derive(Debug)]
pub struct Hatch {
    pub buffer: Item,
}

impl Hatch {
    pub const fn item(id: u8, count: u8) -> Self {
        Self {
            buffer: Item { id, count, properties: ItemProperties::default() },
        }
    }

    pub const fn empty() -> Self {
        Self {
            buffer: Item::invalid(),
        }
    }
}

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
    pub recipe: Option<&'static Recipe>,
    pub progress: Option<Progress>,
    pub status: MachineStatus,
    pub pole: Option<PoleKey>,
    pub clicky_thing_attached: bool,
    pub internal_power_buffer: LoadUnit,
}

#[derive(Default, Debug)]
pub struct Miner {
    pub output: HatchKey,
    pub recipe: Option<&'static MinerRecipe>,
    pub progress: Option<Progress>,
    pub status: MachineStatus,
    pub pole: Option<PoleKey>,
    pub clicky_thing_attached: bool,
    pub internal_power_buffer: LoadUnit,
}

#[derive(Default, Debug)]
pub struct Splitter {
    pub input: HatchKey,
    pub output: Vec<HatchKey>,
    pub toggle: bool,
}


#[derive(Debug)]
pub enum Pole {
    // generator pole
    Generator {
        max_load: LoadUnit,
        current_load: LoadUnit,
    },

    // consumer pole
    Consumer {
        target_load: LoadUnit,
        current_load: LoadUnit,
    },

    // non powered pole
    Other,
}

#[derive(Debug)]
pub struct Wire {
    pub a: PoleKey,
    pub b: PoleKey,
    pub flow: LoadUnit,
    pub max_flow: LoadUnit,
    pub damage: HealthUnit,
}

#[derive(Debug)]
pub struct Silo {
    pub input: HatchKey,
    pub output: HatchKey,
    pub stack: Vec<Item>,
}

pub struct Settings {
    pub wire_damage_per_tick: Option<u8>,
    pub wire_max_flow: LoadUnit,
    pub belt_buffer_scaling_factor: f32,
    pub belt_ticks_between_transfers: u64,
    pub belt_transfer_size: u8,
    pub silo_transfer_size: u8,
    pub machine_require_clicky_thing_attached: bool,
    pub machine_require_power: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wire_damage_per_tick: None,
            wire_max_flow: LoadUnit::MAX,
            belt_buffer_scaling_factor: 2f32,
            belt_ticks_between_transfers: 16,
            belt_transfer_size: 1,
            silo_transfer_size: 1,
            machine_require_clicky_thing_attached: true,
            machine_require_power: true,
        }
    }
}

#[derive(Default)]
pub struct Simulation<R: registry::Registry> {
    pub hatches: SlotMap<HatchKey, Hatch>,
    pub machines: SlotMap<MachineKey, Machine>,
    pub miners: SlotMap<MinerKey, Miner>,
    pub splitters: SlotMap<SplitterKey, Splitter>,
    pub belts: SlotMap<BeltKey, Belt>,
    pub poles: SlotMap<PoleKey, Pole>,
    pub wires: SlotMap<WireKey, Wire>,
    pub silos: SlotMap<SiloKey, Silo>,
    pub settings: Settings,
    pub tick: u64,

    // used for testing...
    pub sources: Vec<(HatchKey, Item)>,
    pub sinks: Vec<HatchKey>,

    pub rng: fastrand::Rng,

    pub registry: R
}


fn handle_splitters<R: registry::Registry>(hatches: &mut SlotMap<HatchKey, Hatch>, splitters: &mut SlotMap<SplitterKey, Splitter>, settings: &mut Settings) {
    for splitter in splitters.values_mut() {
        let disjoint_hatches = hatches.get_disjoint_mut([splitter.input, splitter.output[0], splitter.output[1]]).unwrap();
        let [input, output_1, output_2] = disjoint_hatches.map(|x| &mut x.buffer);        

        if !input.is_invalid() {
            if splitter.toggle {
                Item::transfer_limited::<R>(input, output_1, settings.silo_transfer_size);
            } else {
                Item::transfer_limited::<R>(input, output_2, settings.silo_transfer_size);
            }

            splitter.toggle = !splitter.toggle;
        }
    }
}


fn handle_silos<R: registry::Registry>(hatches: &mut SlotMap<HatchKey, Hatch>, silos: &mut SlotMap<SiloKey, Silo>, settings: &mut Settings) {
    for silo in silos.values_mut() {
        let input = &mut hatches[silo.input].buffer;
        let mut taken = Item::invalid();
        Item::transfer_limited::<R>(input, &mut taken, settings.silo_transfer_size);

        if !taken.is_invalid() {
            silo.stack.push(taken);
        }

        if let Some(last) = silo.stack.last_mut() {
            let predicate = hatches[silo.output].buffer.can_accumulate_from::<R>(last);

            if predicate {
                let output = &mut hatches[silo.output].buffer;
                Item::transfer_limited::<R>(last, output, settings.silo_transfer_size);

                if last.is_invalid() {
                    silo.stack.pop().unwrap();
                }
            }
        }
    }
}

struct Commons<'a, R: GenericRecipe> {
    recipe: &'a R,
    consumer_pole_id: PoleKey,
    internal_power_buffer: &'a mut LoadUnit,
    status: &'a mut MachineStatus,
    clicky_thing_attached: bool,
    progress: &'a mut Option<Progress>,

    inputs: Vec<HatchKey>,
    outputs: Vec<HatchKey>,
    
}

trait GenericRecipe {
    fn load(&self) -> LoadUnit;
    fn ticks(&self) -> u16;
    fn inputs(&self) -> Vec<Item>;
    fn outputs(&self) -> Vec<Item>;
}

trait GenericMachine {
    type Key: slotmap::Key;
    type Recipe: GenericRecipe;

    fn get_commons<'a>(&'a mut self) -> Commons<'a, Self::Recipe>;
    fn do_thing_machine<R: registry::Registry>(recipe: &Self::Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng) {}
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


    fn get_commons<'a>(&'a mut self) -> Commons<'a, Self::Recipe> {
        Commons {
            recipe: self.recipe.unwrap(),
            consumer_pole_id: self.pole.unwrap(),
            internal_power_buffer: &mut self.internal_power_buffer,
            status: &mut self.status,
            clicky_thing_attached: self.clicky_thing_attached,
            progress: &mut self.progress,
            inputs: Vec::default(),
            outputs: vec![self.output]
        }
    }

    fn do_thing_machine<R: registry::Registry>(recipe: &Self::Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng) {
        if rng.bool() {
            let buffer = &mut hatches[outputs[0]].buffer;
            buffer.accumulate::<R>(&recipe.output);
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


    fn get_commons<'a>(&'a mut self) -> Commons<'a, Self::Recipe> {
        Commons {
            recipe: self.recipe.unwrap(),
            consumer_pole_id: self.pole.unwrap(),
            internal_power_buffer: &mut self.internal_power_buffer,
            status: &mut self.status,
            clicky_thing_attached: self.clicky_thing_attached,
            progress: &mut self.progress,
            inputs: self.input.clone(),
            outputs: self.output.clone()
        }
    }

    fn do_thing_machine<R: registry::Registry>(recipe: &Self::Recipe, inputs: &[HatchKey], outputs: &[HatchKey], hatches: &mut SlotMap<HatchKey, Hatch>, rng: &mut Rng) {
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
            buffer.accumulate::<R>(recipe_output);
        }
    }
}


fn handle_generic_machine<R: registry::Registry, M: GenericMachine>(machines: &mut SlotMap<M::Key, M>, poles: &mut SlotMap<PoleKey, Pole>, hatches: &mut SlotMap<HatchKey, Hatch>, settings: &mut Settings, rng: &mut Rng) {
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
        } = machine.get_commons();

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
                M::do_thing_machine::<R>(recipe, &inputs, &outputs, hatches, rng);

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
                        let stack_size = R::stack_size(buffer.id);

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

fn handle_belts<R: registry::Registry>(belts: &mut SlotMap<BeltKey, Belt>, hatches: &mut SlotMap<HatchKey, Hatch>, tick: &mut u64, settings: &mut Settings) {
    for belt in belts.values_mut() {
        let Belt {
            belt_start,
            belt_end,
            ref mut buffer,
            last_transfer_tick: ref mut last_transfer_ticks,
        } = *belt;

        // don't do anything if belt points to invalid hatches
        if !hatches.contains_key(belt_start) || !hatches.contains_key(belt_end) {
            continue;
        }

        // impossible that it overflows because self.tick > last_transfer_ticks, always
        if (*tick - *last_transfer_ticks) >= settings.belt_ticks_between_transfers {
            // godot::global::godot_print!("belt tick");
        
            let input_hatch = &hatches[belt_end];
            let predicate = input_hatch.buffer.is_invalid() || (input_hatch.buffer.can_accumulate_from::<R>(buffer.last().unwrap()) || buffer.last().unwrap().is_invalid());

            // order of operations:
            // transfer last element in belt buffer to input hatch
            // roll elements in belt buffer one to the right
            // transfer output hatch item to first element in belt buffer
            if predicate {
                *last_transfer_ticks = *tick;

                // godot::global::godot_print!("belt shifting");

                // element at index buffer.len()-1 is belt output
                let input_hatch = &mut hatches[belt_end];
                Item::transfer_limited::<R>(buffer.last_mut().unwrap(), &mut input_hatch.buffer, settings.belt_transfer_size);

                // I'm shifting... I'm shifting....
                buffer.rotate_right(1);

                // first element must be reset to invalid now...
                buffer[0].invalidate();

                // element at index 0 is the belt input
                // belt takes input from hatch...
                let output_hatch = &mut hatches[belt_start];
                Item::transfer_limited::<R>(&mut output_hatch.buffer, &mut buffer[0], settings.belt_transfer_size);
            }
        }
    }
}

fn handle_power(poles: &mut SlotMap<PoleKey, Pole>, wires: &mut SlotMap<WireKey, Wire>) {
    // create adjacency map that stores neighbouring poles
    let mut lookup = HashMap::<PoleKey, Vec<(PoleKey, WireKey)>>::new();
    for (wire_key, Wire { a, b, .. }) in wires.iter() {
        lookup
            .entry(*a)
            .or_default()
            .push((*b, wire_key));
        lookup
            .entry(*b)
            .or_default()
            .push((*a, wire_key));
    }

    // reset load of poles
    for pole in poles.values_mut() {
        match pole {
            Pole::Generator { current_load, .. } => {
                *current_load = 0;
            }
            Pole::Consumer { current_load, .. } => {
                *current_load = 0;
            }
            Pole::Other => {}
        }
    }

    // reset wire flow
    for wire in wires.values_mut() {
        wire.flow = 0;
    }

    // get all pole consumers
    let consumers = poles
        .iter()
        .filter_map(|(index, pole)| {
            if let Pole::Consumer { target_load, .. } = pole {
                Some((index, *target_load))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // for each consumer, it will run a BFS starting at the consumer pole and grow until it gets enough power 
    for (consumer_index, consumer_target_load) in consumers {
        assert!(consumer_target_load >= 0);
        let mut consumer_current_load = 0;

        let mut backtracking = HashMap::<PoleKey, (PoleKey, WireKey)>::new();
        let mut generators_used = HashSet::<(PoleKey, LoadUnit)>::new();
        let mut visited = HashSet::<PoleKey>::new();

        let mut queue = VecDeque::<PoleKey>::new();

        // in case of pole but not connected to anything
        if lookup.contains_key(&consumer_index) {
            queue.push_back(consumer_index);
        }

        // simple BFS shortest-path search to find enough load to satisfy consumer
        'pathfind: while let Some(index) = queue.pop_front() {
            let neighbours = &lookup[&index];

            for (neighbour_index, wire_index) in neighbours {
                let consumer_remaining_load_to_satisfy =
                    consumer_target_load - consumer_current_load;

                assert!(consumer_remaining_load_to_satisfy >= 0);

                // consumer has fully satisfied load, we can exit early
                if consumer_remaining_load_to_satisfy == 0 {
                    break 'pathfind;
                }

                let neighbour = &mut poles[*neighbour_index];

                if visited.insert(*neighbour_index) {
                    match neighbour {
                        Pole::Generator {
                            max_load,
                            current_load: current_generator_load,
                        } => {
                            // calculate the generator's remaining load
                            let generator_remaining_load = *max_load - *current_generator_load;

                            // calculcate how much load the consumer should take off of that
                            let consumer_taken_load = generator_remaining_load
                                .min(consumer_remaining_load_to_satisfy);

                            // add load to consumer
                            consumer_current_load += consumer_taken_load;

                            // add load to generator
                            *current_generator_load += consumer_taken_load;

                            backtracking.insert(*neighbour_index, (index, *wire_index));
                            generators_used.insert((*neighbour_index, consumer_taken_load));
                        }
                        Pole::Consumer { .. } => {}
                        Pole::Other => {
                            queue.push_back(*neighbour_index);
                            backtracking.insert(*neighbour_index, (index, *wire_index));
                        }
                    }
                }
            }
        }

        match &mut poles[consumer_index] {
            Pole::Consumer { current_load, .. } => {
                *current_load = consumer_current_load;
            }
            _ => unreachable!(),
        }

        // starting from the used generators, backtrack towards the consumer and modify wire flow along the way
        for (generator_used, load) in generators_used {
            // starts at generator
            let mut opt_pole = Some(generator_used);

            // `pole` is the pole closer to the generator
            while let Some(pole) = opt_pole.take() {
                // eventually this will reach the consumer itself
                // `new_pole_id` is the pole closer to the consumer
                if let Some((new_pole_id, wire_index)) = backtracking.get(&pole).copied() {
                    opt_pole = Some(new_pole_id);

                    let wire = &mut wires[wire_index];
                    if wire.a == pole && wire.b == new_pole_id {
                        // `a` is closer to gen
                        // `b` is closer to con
                        wires[wire_index].flow += load; // flow from `a` to `b` is positive
                    } else if wire.b == pole && wire.a == new_pole_id {
                        // `a` is closer to con
                        // `b` is closer to gen
                        wires[wire_index].flow -= load; // flow from `a` to `b` is negative
                    } else {
                        unreachable!();
                    }
                }
            }
        }
    }
}

fn handle_wire_damage(wires: &mut SlotMap<WireKey, Wire>, settings: &mut Settings) {
    // before we reset wire flow, check for max flow and do damage tick
    if let Some(wire_damage_per_tick) = settings.wire_damage_per_tick {
        for wire in wires.values_mut() {
            if wire.flow.abs() > wire.max_flow {
                wire.damage = wire.damage.saturating_add(wire_damage_per_tick);
            }
        }

        wires.retain(|_, w| w.damage < u8::MAX);
    }
}



impl<R: registry::Registry> Simulation<R> {
    pub fn tick(&mut self) {
        let Self {
            machines,
            belts,
            poles,
            wires,
            hatches,
            tick,
            silos,
            settings,
            miners,
            splitters,
            rng,
            ..
        } = self;

        handle_wire_damage(wires, settings);

        handle_power(poles, wires);

        // update debug sources
        for (source, item) in self.sources.iter() {
            let hatch = &mut hatches[*source];
            hatch.buffer = *item;            
        }

        // update debug sinks
        for sink in self.sinks.iter() {
            let hatch = &mut hatches[*sink];
            hatch.buffer.invalidate();
        }

        // doing belt logic before machine logic fixes "sigle-tick idle" issue when belt transfer speed === machine recipe tick speed (halting on input materials) 
        handle_belts::<R>(belts, hatches, tick, settings);

        handle_generic_machine::<R, Machine>(machines, poles, hatches, settings, rng);
        handle_generic_machine::<R, Miner>(miners, poles, hatches, settings, rng);

        handle_silos::<R>(hatches, silos, settings);
        handle_splitters::<R>(hatches, splitters, settings);

        *tick += 1;
    }
}

impl<R: registry::Registry> Simulation<R> {
    pub fn add_miner_with_pole(&mut self, recipe: &'static MinerRecipe, pole_key: PoleKey) -> MinerKey {
        self.poles[pole_key] = Pole::Consumer { target_load: 0, current_load: 0 };
        
        let miner = Miner {
            output: self.hatches.insert(Hatch::empty()),
            recipe: Some(&recipe),
            progress: None,
            pole: Some(pole_key),
            ..Default::default()
        };

        self.miners.insert(miner)
    }

    pub fn add_machine_with_pole(&mut self, recipe: &'static Recipe, pole_key: PoleKey, input_hatches: u32, output_hatches: u32) -> MachineKey {
        self.poles[pole_key] = Pole::Consumer { target_load: 0, current_load: 0 };

        let input = (0..input_hatches).map(|_| self.hatches.insert(Hatch::empty())).collect::<Vec<_>>();
        let output = (0..output_hatches).map(|_| self.hatches.insert(Hatch::empty())).collect::<Vec<_>>();
        
        let machine = Machine {
            input,
            output,
            recipe: Some(&recipe),
            progress: None,
            pole: Some(pole_key),
            ..Default::default()
        };

        self.machines.insert(machine)
    }

    pub fn add_machine(&mut self, recipe: &'static Recipe) -> (MachineKey, PoleKey) {
        let pole_id = self.poles.insert(Pole::Consumer { target_load: 0, current_load: 0 });

        // TODO: scale number of hatches with required recipe I/O
        let input = self.hatches.insert(Hatch::empty());
        let output = self.hatches.insert(Hatch::empty());

        let machine = Machine {
            input: vec![input],
            output: vec![output],
            recipe: Some(&recipe),
            progress: None,
            pole: Some(pole_id),
            ..Default::default()
        };

        let machine_id = self.machines.insert(machine);
        (machine_id, pole_id)
    }

    pub fn remove_machine(&mut self, key: MachineKey) {
        let m = self.machines.remove(key).unwrap();
        for hatch in m.input.iter().chain(m.output.iter()) {
            self.hatches.remove(*hatch).unwrap();
        }
    }

    pub fn remove_miner(&mut self, key: MinerKey) {
        let m = self.miners.remove(key).unwrap();
        self.hatches.remove(m.output).unwrap();
    }

    pub fn add_silo(&mut self) -> SiloKey {
        let input = self.hatches.insert(Hatch::empty());
        let output = self.hatches.insert(Hatch::empty());

        self.silos.insert(Silo {
            input,
            output,
            stack: Vec::default(),
        })
    }

    pub fn remove_silo(&mut self, key: SiloKey) {
        let m = self.silos.remove(key).unwrap();
        self.hatches.remove(m.input);
        self.hatches.remove(m.output);
    }

    // not really adding...
    pub fn add_generator_with_pole(&mut self, max_load: LoadUnit, pole_key: PoleKey) {
        self.poles[pole_key] = Pole::Generator { max_load, current_load: 0 }
    }
    
    pub fn add_infinite_generator(&mut self) -> PoleKey {
        self.add_generator(LoadUnit::MAX)
    }

    pub fn add_generator(&mut self, max_load: LoadUnit) -> PoleKey {
        self.poles.insert(Pole::Generator { max_load, current_load: 0 })
    }

    pub fn add_consumer(&mut self, target_load: LoadUnit) -> PoleKey {
        self.poles.insert(Pole::Consumer { target_load, current_load: 0 })
    }

    pub fn add_pole(&mut self) -> PoleKey {
        self.poles.insert(Pole::Other)
    }

    pub fn remove_pole(&mut self, key: PoleKey) {
        self.poles.remove(key);

        // remove associated wires
        self.wires.retain(|_, w| !(w.a == key || w.b == key));
    } 

    pub fn are_poles_connected(&self, a: PoleKey, b: PoleKey) -> bool {
        self.wires.values().any(|wire| (wire.a == a && wire.b == b) || (wire.a == b && wire.b == a))
    }

    pub fn are_hatches_connected(&self, a: HatchKey, b: HatchKey) -> bool {
        self.belts.values().any(|belt| (belt.belt_start == a && belt.belt_end == b) || (belt.belt_start == b && belt.belt_end == a))
    }

    pub fn is_hatch_connected(&self, hatch: HatchKey) -> bool {
        self.belts.values().any(|belt| belt.belt_start == hatch || belt.belt_end == hatch)
    }
    
    pub fn add_wire_with_max_flow(&mut self, a: PoleKey, b: PoleKey, max_flow: LoadUnit) -> WireKey {
        assert!(!self.are_poles_connected(a, b), "cannot add duplicate wire");
        assert!(a != b, "wire poles must not be identical");

        self.wires.insert(Wire {
            a, b,
            flow: 0,
            damage: 0,
            max_flow,
        })
    }

    // TODO: add wire tiers
    pub fn add_wire(&mut self, a: PoleKey, b: PoleKey) -> WireKey {
        self.add_wire_with_max_flow(a, b, self.settings.wire_max_flow)
    }

    pub fn remove_wire(&mut self, wire: WireKey) {
        self.wires.remove(wire);
    }

    pub fn add_wire_chain(&mut self, poles: &[PoleKey]) -> Vec<WireKey> {
        poles.windows(2).into_iter().map(|window| {
            self.add_wire(window[0], window[1])
        }).collect::<Vec<WireKey>>()
    }

    // TODO: add belt tiers
    pub fn add_belt_2(&mut self, output_hatch: HatchKey, input_hatch: HatchKey, length: BeltSize) -> BeltKey {
        assert!(!self.are_hatches_connected(output_hatch, input_hatch), "cannot add duplicate belt");
        assert!(output_hatch != input_hatch, "belt hatches must not be identical");

        let buffer_length = self.calculate_belt_buffer_size(length);

        self.belts.insert(Belt {
            belt_start: output_hatch,
            belt_end: input_hatch,
            buffer: vec![Item::invalid(); buffer_length],
            last_transfer_tick: 0
        })
    }

    pub fn calculate_belt_buffer_size(&self, length: BeltSize) -> usize {
        match length {
            BeltSize::BufferLength(x) => x as f32,
            BeltSize::WorldLength(x) => x * self.settings.belt_buffer_scaling_factor,
        }.ceil().max(2f32) as usize
    }
    
    pub fn remove_belt(&mut self, belt: BeltKey) {
        self.belts.remove(belt);
    }
    
    
    pub fn get_input_hatch_mut(&mut self, machine_id: MachineKey, hatch_index: usize) -> &mut Item {
        &mut self.hatches[self.machines[machine_id].input[hatch_index]].buffer
    }

    pub fn get_output_hatch_mut(&mut self, machine_id: MachineKey, hatch_index: usize) -> &mut Item {
        &mut self.hatches[self.machines[machine_id].output[hatch_index]].buffer
    }

    pub fn add_source(&mut self, source_item: Item) -> HatchKey {
        let key = self.hatches.insert(Hatch {
            buffer: source_item,
        });

        self.sources.push((key, source_item));

        key
    }

    pub fn add_sink(&mut self) -> HatchKey {
        let key = self.hatches.insert(Hatch {
            buffer: Item::invalid(),
        });

        self.sinks.push(key);

        key
    }
    
    pub fn add_splitter(&mut self) -> SplitterKey {
        let input = self.hatches.insert(Hatch::empty());
        let output = (0..2).into_iter().map(|x| self.hatches.insert(Hatch::empty())).collect::<Vec<_>>();
        
        self.splitters.insert(Splitter {
            input,
            output,
            toggle: false,
        })
    }

    pub fn remove_splitter(&mut self, key: SplitterKey) {
        let s = self.splitters.remove(key).unwrap();
        self.hatches.remove(s.input).unwrap();
        self.hatches.remove(s.output[0]).unwrap();
        self.hatches.remove(s.output[1]).unwrap();        
    }
}