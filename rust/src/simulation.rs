use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU16,
};

use fastrand::Rng;
use slotmap::{SlotMap, new_key_type};

use crate::{items::*, registry::{self, MinerRecipe, Recipe}};
use crate::handle::*;
use crate::common::*;
use super::systems::*;


#[derive(Default)]
pub struct Simulation<R: registry::Registry> {
    pub hatches: SlotMap<HatchKey, Hatch>,
    pub fluid_hatches: SlotMap<FluidHatchKey, FluidHatch>,
    pub pipes: SlotMap<PipeKey, Pipe>,
    pub machines: SlotMap<MachineKey, Machine>,
    pub miners: SlotMap<MinerKey, Miner>,
    pub splitters: SlotMap<SplitterKey, Splitter>,
    pub belts: SlotMap<BeltKey, Belt>,
    pub poles: SlotMap<PoleKey, Pole>,
    pub wires: SlotMap<WireKey, Wire>,
    pub silos: SlotMap<SiloKey, Silo>,
    pub fluid_buffers: SlotMap<FluidBufferKey, FluidBuffer>,
    
    pub settings: Settings,
    pub tick: u64,

    // used for testing...
    pub sources: Vec<(HatchKey, Item)>,
    pub sinks: Vec<HatchKey>,

    pub rng: fastrand::Rng,

    pub registry: R
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
            pipes,
            fluid_hatches,
            fluid_buffers,
            settings,
            miners,
            splitters,
            rng,
            registry,
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
        handle_belts::<R>(belts, hatches, tick, settings, &registry);
        handle_pipes::<R>(pipes, fluid_hatches, tick, settings);

        handle_generic_machine::<R, Machine>(machines, poles, hatches, settings, rng, &registry);
        handle_generic_machine::<R, Miner>(miners, poles, hatches, settings, rng, &registry);

        handle_silos::<R>(hatches, silos, settings, &registry);
        handle_splitters::<R>(hatches, splitters, settings, &registry);
        handle_buffers::<R>(fluid_hatches, fluid_buffers, settings);

        *tick += 1;
    }
}

impl<R: registry::Registry> Simulation<R> {
    pub fn add_miner_with_pole(&mut self, recipe: String, pole_key: PoleKey) -> MinerKey {
        self.poles[pole_key] = Pole::Consumer { target_load: 0, current_load: 0 };
        
        let miner = Miner {
            output: self.hatches.insert(Hatch::empty()),
            recipe: Some(recipe),
            progress: None,
            pole: Some(pole_key),
            ..Default::default()
        };

        self.miners.insert(miner)
    }

    pub fn add_machine_with_pole(&mut self, recipe: String, pole_key: PoleKey, input_hatches: u32, output_hatches: u32) -> MachineKey {
        self.poles[pole_key] = Pole::Consumer { target_load: 0, current_load: 0 };

        let input = (0..input_hatches).map(|_| self.hatches.insert(Hatch::empty())).collect::<Vec<_>>();
        let output = (0..output_hatches).map(|_| self.hatches.insert(Hatch::empty())).collect::<Vec<_>>();
        
        let machine = Machine {
            input,
            output,
            recipe: Some(recipe),
            progress: None,
            pole: Some(pole_key),
            ..Default::default()
        };

        self.machines.insert(machine)
    }

    pub fn add_machine(&mut self, recipe: String) -> (MachineKey, PoleKey) {
        let pole_id = self.poles.insert(Pole::Consumer { target_load: 0, current_load: 0 });

        // TODO: scale number of hatches with required recipe I/O
        let input = self.hatches.insert(Hatch::empty());
        let output = self.hatches.insert(Hatch::empty());

        let machine = Machine {
            input: vec![input],
            output: vec![output],
            recipe: Some(recipe),
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

    pub fn add_fluid_buffer(&mut self, consumer: bool, producer: bool) -> FluidBufferKey {
        let input = self.fluid_hatches.insert(FluidHatch::empty());
        let output = self.fluid_hatches.insert(FluidHatch::empty());

        self.fluid_buffers.insert(FluidBuffer {
            input,
            output,
            producer,
            consumer,
            fluid: 0
        })
    }

    pub fn remove_fluid_buffer(&mut self, key: FluidBufferKey) {
        let m = self.fluid_buffers.remove(key).unwrap();
        self.fluid_hatches.remove(m.input);
        self.fluid_hatches.remove(m.output);
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

    pub fn are_fluid_hatches_connected(&self, a: FluidHatchKey, b: FluidHatchKey) -> bool {
        self.pipes.values().any(|pipe| (pipe.pipe_start == a && pipe.pipe_end == b) || (pipe.pipe_start == b && pipe.pipe_end == a))
    }

    pub fn is_fluid_hatch_connected(&self, hatch: FluidHatchKey) -> bool {
        self.pipes.values().any(|pipe| pipe.pipe_start == hatch || pipe.pipe_end == hatch)
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
    pub fn add_belt_2(&mut self, output_hatch: HatchKey, input_hatch: HatchKey, length: BufferSize) -> BeltKey {
        assert!(!self.are_hatches_connected(output_hatch, input_hatch), "cannot add duplicate belt");
        assert!(output_hatch != input_hatch, "belt hatches must not be identical");

        let buffer_length = self.calculate_logical_belt_buffer_size(length);

        self.belts.insert(Belt::from_hatches(output_hatch, input_hatch, buffer_length))
    }

    pub fn add_pipe(&mut self, output_hatch: FluidHatchKey, input_hatch: FluidHatchKey, length: BufferSize) -> PipeKey {
        assert!(!self.are_fluid_hatches_connected(output_hatch, input_hatch), "cannot add duplicate pipe");
        assert!(output_hatch != input_hatch, "fluid hatches must not be identical");

        let buffer_length = self.calculate_logical_pipe_buffer_size(length);

        self.pipes.insert(Pipe::from_hatches(output_hatch, input_hatch, buffer_length))
    }

    pub fn calculate_logical_buffer_size(length: BufferSize, world_scaling_factor: f32) -> usize {
        match length {
            BufferSize::BufferLength(x) => x as f32,
            BufferSize::WorldLength(x) => x * world_scaling_factor,
        }.ceil().max(2f32) as usize
    }

    pub fn calculate_logical_belt_buffer_size(&self, length: BufferSize) -> usize {
        Self::calculate_logical_buffer_size(length, self.settings.belt_buffer_scaling_factor)
    }

    pub fn calculate_logical_pipe_buffer_size(&self, length: BufferSize) -> usize {
        Self::calculate_logical_buffer_size(length, self.settings.pipe_buffer_scaling_factor)
    }
    
    pub fn remove_belt(&mut self, belt: BeltKey) {
        self.belts.remove(belt);
    }

    pub fn remove_pipe(&mut self, pipe: PipeKey) {
        self.pipes.remove(pipe);
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