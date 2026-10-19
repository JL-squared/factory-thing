use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use slotmap::SlotMap;

use crate::handle::*;
use crate::common::*;


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


pub fn handle_power(poles: &mut SlotMap<PoleKey, Pole>, wires: &mut SlotMap<WireKey, Wire>) {
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

pub fn handle_wire_damage(wires: &mut SlotMap<WireKey, Wire>, settings: &mut Settings) {
    // before we reset wire flow, check for max flow and do damage tick
    if let Some(wire_damage_per_tick) = settings.wire_damage_per_tick {
        for wire in wires.values_mut() {
            if wire.flow.abs() > wire.max_flow {
                wire.damage = wire.damage.saturating_add(wire_damage_per_tick);
            }
        }

        // this is safe, because the Godot side will automatically destroy the appropriate wire nodes from the world if they have invalid keys to the simulation wires
        wires.retain(|_, w| w.damage < u8::MAX);
    }
}

