#[cfg(test)]
mod test_registry {
    use crate::registry::*;
    use crate::items::*;

    #[derive(Default)]
    pub struct TestRegistry;    

    impl TestRegistry {
        pub const RAW_IRON_1: u8 = 1;
        pub const CRUSHED_IRON: u8 = 2;
        pub const IRON_DUST: u8 = 3;
        pub const IRON_INGOT: u8 = 4;   

        pub const ITEMS: &[RegistryItem<()>] = &[
            RegistryItem {
                name: "invalid",
                stack_size: 0,
                data: ()
            },
            RegistryItem {
                name: "Raw Iron",
                stack_size: 255,
                data: ()
            },
            RegistryItem {
                name: "Crushed Iron",
                stack_size: 255,
                data: ()
            },
            RegistryItem {
                name: "Iron Dust",
                stack_size: 255,
                data: ()
            },
            RegistryItem {
                name: "Iron Ingot",
                stack_size: 255,
                data: ()
            },
        ];  

        pub const CRUSH_IRON_RECIPE: Recipe = Recipe {
            id: "crush_iron",
            name: "",
            input: &[Item::one(Self::RAW_IRON_1)],
            output: &[Item::one(Self::CRUSHED_IRON)],
            ticks: 16,
            load: 10,
        };  

        pub const CRUSH_IRON_MORE_RECIPE: Recipe = Recipe {
            id: "crush_iron_more",
            name: "",
            input: &[Item::one(Self::CRUSHED_IRON)],
            output: &[Item::one(Self::IRON_DUST)],
            ticks: 16,
            load: 10,
        };  

        pub const SMELT_IRON_RECIPE: Recipe = Recipe {
            id: "smelt_iron",
            name: "",
            input: &[Item::one(Self::IRON_DUST)],
            output: &[Item::one(Self::IRON_INGOT)],
            ticks: 4,
            load: 0,
        };
    }   

    impl Registry for TestRegistry {
        type Data = (); 

        fn registry_item(id: u8) -> &'static RegistryItem<()> {
            &Self::ITEMS[id as usize]
        }

        fn registry_recipe(string_id: &str) -> &'static Recipe {
            todo!()
        }
    }

    impl crate::simulation::Settings {
        fn testing() -> Self {
            Self {
                wire_damage_per_tick: None,
                belt_buffer_scaling_factor: 2f32,
                belt_ticks_between_transfers: 16,
                belt_transfer_size: 1,
                silo_transfer_size: 1,
                machine_require_clicky_thing_attached: false,
                wire_max_flow: crate::LoadUnit::MAX,
                machine_require_power: true,
            }
        }
    }


    impl<R: Registry> crate::simulation::Simulation<R> {
        pub fn testing() -> Self {
            Self {
                settings: crate::simulation::Settings::testing(),
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod power_tests {
    use super::test_registry::*;
    use crate::*;

    type TestGame = Simulation<TestRegistry>;

    #[test]
    fn simple_power() {
        let mut game = TestGame::testing();
        let a = game.add_generator(10);
        let b = game.add_consumer(10);
        let wire = game.add_wire(a, b);

        game.tick();

        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[wire].flow == 10);
    }

    #[test]
    fn simple_power_inv() {
        let mut game = TestGame::testing();
        let a = game.add_generator(10);
        let b = game.add_consumer(10);
        let wire = game.add_wire(b, a);

        game.tick();
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[wire].flow == -10);
    }

    #[test]
    fn simple_power_inv_2() {
        let mut game = TestGame::testing();
        let b = game.add_consumer(10);
        let a = game.add_generator(10);
        let wire = game.add_wire(a, b);

        game.tick();
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[wire].flow == 10);
    }

    #[test]
    fn poles_not_connected() {
        let mut game = TestGame::testing();
        let _ = game.add_consumer(10);
        let _ = game.add_generator(10);
        game.tick();
    }

    #[test]
    fn overloaded_wire() {
        let mut game = TestGame::testing();
        game.settings.wire_damage_per_tick = Some(1);

        let b = game.add_consumer(10);
        let a = game.add_generator(10);
        let wire = game.add_wire_with_max_flow(a, b, 1);

        assert_eq!(game.wires[wire].damage, 0);

        game.tick();
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[wire].flow == 10);
        assert_eq!(game.wires[wire].damage, 0);

        for _ in 0..254 {
            game.tick();
        }

        assert_eq!(game.wires[wire].damage, 254);

        game.tick();

        assert_eq!(game.wires.len(), 0);
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
    }

    #[test]
    fn simple_power_split_load_equally() {
        let mut game = TestGame::testing();
        let a = game.add_generator(10);
        let b = game.add_consumer(5);
        let c = game.add_consumer(5);
        
        let w1 = game.add_wire(a,b);
        let w2 = game.add_wire(a,c);
        

        game.tick();
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 5,
                ..
            }
        ));
        assert!(matches!(
            game.poles[c],
            Pole::Consumer {
                current_load: 5,
                ..
            }
        ));
        assert!(game.wires[w1].flow == 5);
        assert!(game.wires[w2].flow == 5);
    }

    #[test]
    fn simple_power_split_load_inequally() {
        let mut game = TestGame::testing();
        let a = game.add_generator(10);
        let b = game.add_consumer(7);
        let c = game.add_consumer(3);
        
        let w1 = game.add_wire(a,b);
        let w2 = game.add_wire(a,c);

        game.tick();
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 7,
                ..
            }
        ));
        assert!(matches!(
            game.poles[c],
            Pole::Consumer {
                current_load: 3,
                ..
            }
        ));
        assert!(game.wires[w1].flow == 7);
        assert!(game.wires[w2].flow == 3);
    }

    #[test]
    fn simple_power_chain() {
        let mut game = TestGame::testing();

        let mut poles = vec![];
        poles.push(Pole::Generator {
            max_load: 10,
            current_load: 0,
        });
        poles.extend(std::iter::repeat_with(|| Pole::Other).take(32));
        poles.push(Pole::Consumer {
            target_load: 10,
            current_load: 0,
        });
        let pole_keys = poles.into_iter().map(|p| game.poles.insert(p)).collect::<Vec<_>>();

        let wire_keys = game.add_wire_chain(&pole_keys);

        for pole_key in pole_keys.iter() {
            let pole = &game.poles[*pole_key];
            match pole {
                Pole::Generator { current_load, .. } => assert_eq!(*current_load, 0),
                Pole::Consumer { current_load, .. } => assert_eq!(*current_load, 0),
                Pole::Other => {}
            };
        }

        for wire_key in wire_keys.iter() {
            assert_eq!(game.wires[*wire_key].flow, 0);
        }

        game.tick();

        for pole_key in pole_keys.iter() {
            let pole = &game.poles[*pole_key];
            match pole {
                Pole::Generator { current_load, .. } => assert_eq!(*current_load, 10),
                Pole::Consumer { current_load, .. } => assert_eq!(*current_load, 10),
                Pole::Other => {}
            };
        }

        for wire_key in wire_keys.iter() {
            assert_eq!(game.wires[*wire_key].flow, 10);
        }
    }

    #[test]
    fn no_power_machine_no_items() {
        let mut game = Simulation::default();
        let a = game.add_generator(10);
        let (_, b) = game.add_machine(&TestRegistry::CRUSH_IRON_RECIPE);
        
        let wire = game.add_wire(a,b);
        
        game.tick();
        assert_no_power(game, a, b, wire);
    }

    #[test]
    fn no_power_machine_halt_reason_output_full() {
        let mut game = Simulation::default();
        let a = game.add_generator(10);
        let (m, b) = game.add_machine(&TestRegistry::CRUSH_IRON_RECIPE);
        
        *game.get_input_hatch_mut(m, 0) = Item::full_stack::<TestRegistry>(TestRegistry::RAW_IRON_1);
        *game.get_output_hatch_mut(m, 0) = Item::full_stack::<TestRegistry>(TestRegistry::CRUSHED_IRON);


        let wire = game.add_wire(a,b);
        
        game.tick();
        assert_no_power(game, a, b, wire);
    }

    #[test]
    fn no_power_machine_halt_reason_output_type_mismatch() {
        let mut game = Simulation::default();
        let a = game.add_generator(10);
        let (m, b) = game.add_machine(&TestRegistry::CRUSH_IRON_RECIPE);
        
        *game.get_input_hatch_mut(m, 0) = Item::full_stack::<TestRegistry>(TestRegistry::RAW_IRON_1);
        *game.get_output_hatch_mut(m, 0) = Item::one(TestRegistry::RAW_IRON_1);


        let wire = game.add_wire(a,b);
        
        game.tick();
        assert_no_power(game, a, b, wire);
    }

    fn assert_no_power(game: TestGame, a: PoleKey, b: PoleKey, wire: WireKey) {
        assert!(matches!(
            game.poles[a],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[b],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[wire].flow == 0);
    }
}

#[cfg(test)]
mod sink_source_tests {
    use super::test_registry::*;
    use crate::*;

    type TestGame = Simulation<TestRegistry>;
    

    #[test]
    fn test_belt_1() {
        let mut sim = TestGame::testing();
        sim.settings.belt_ticks_between_transfers = 0;
        sim.settings.belt_transfer_size = 255;

        let source = sim.add_source(Item::full_stack::<TestRegistry>(TestRegistry::CRUSHED_IRON));
        let sink = sim.add_sink();
        
        let belt = sim.add_belt_2(source, sink, BeltSize::BufferLength(10));

        sim.tick();

        assert!(sim.belts[belt].buffer[0].id == TestRegistry::CRUSHED_IRON);

        for _ in 0..10 {
            sim.tick();
        }

        dbg!(sim.belts[belt].buffer[9]);

        // we expect belt to be fully saturated by now
        assert!(sim.belts[belt].buffer.iter().all(|x| x.id == TestRegistry::CRUSHED_IRON));
    }
}

mod tests {
    /*
    use std::num::NonZeroU16;
    use crate::registry::TestRegistry;

    type TestGame = Game<TestRegistry>;


    use super::*;
    
    #[test]
    fn empty() {
        TestGame::testing().tick();
    }

    #[test]
    fn simple_machine_power() {
        let mut game = TestGame::testing();
        
        let generator_pole = game.add_generator(10);
        let (machine, machine_pole) = game.add_machine(&TestRegistry::CRUSH_IRON_RECIPE);
        let wire = game.add_wire(generator_pole, machine_pole);
        *game.get_input_hatch_mut(machine, 0) = TestRegistry::CRUSH_IRON_RECIPE.input[0];

        // no power yet
        assert!(matches!(
            game.poles[generator_pole],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[machine_pole],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[wire].flow == 0);
        assert!(game.machines[machine].progress.is_none());

        game.tick();

        // first tick simply sets recipe and TARGET load of consumer
        assert!(matches!(
            game.poles[generator_pole],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[machine_pole], Pole::Consumer { current_load: 0, target_load } if target_load == TestRegistry::CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[wire].flow == 0);
        assert!(game.machines[machine].progress.is_some());

        game.tick();

        dbg!(&game.poles);

        // second tick actually propagates power generation
        assert!(matches!(
            game.poles[generator_pole],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(
            matches!(game.poles[machine_pole], Pole::Consumer { current_load, target_load } if target_load == TestRegistry::CRUSH_IRON_RECIPE.load && current_load == TestRegistry::CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[wire].flow == 10);
        assert!(matches!(game.machines[machine].status, MachineStatus::None));
        assert!(matches!(game.machines[machine].progress, Some(Progress { slow_down_ticks_remaining, .. }) if slow_down_ticks_remaining.is_none() ));
    }
    */

    /*
    #[test]
    fn simple_machine_underpowered() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 2,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 0,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1)],
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                progress: None,
                pole: Some(PoleId::from(1)),
                ..Default::default()
            }],
            ..Default::default()
        };

        // no power yet
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        // first tick simply sets recipe and TARGET load of consumer
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load: 0, target_load } if target_load == CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_some());

        game.tick();

        dbg!(&game.poles);

        // second tick actually propagates power generation
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 2,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load, target_load } if target_load == CRUSH_IRON_RECIPE.load && current_load == 2)
        );
        assert!(game.wires[0].flow == 2);
        assert!(matches!(game.machines[0].status, MachineStatus::Underpowered));
        assert!(matches!(game.machines[0].progress, Some(Progress { slow_down_ticks_remaining, .. }) if slow_down_ticks_remaining.is_some() ));        
    }

    
    #[test]
    fn simple_machine_non_powered() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 0,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 0,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1)],
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                progress: None,
                pole: Some(PoleId::from(1)),
                ..Default::default()
            }],
            ..Default::default()
        };

        // no power yet
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        // first tick simply sets recipe and TARGET load of consumer
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load: 0, target_load } if target_load == CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_some());

        game.tick();

        dbg!(&game.poles);

        // second tick actually propagates power generation
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load, target_load } if target_load == CRUSH_IRON_RECIPE.load && current_load == 0)
        );
        assert!(game.wires[0].flow == 0);
        assert!(matches!(game.machines[0].status, MachineStatus::NonPowered));
        assert!(matches!(game.machines[0].progress, Some(Progress { slow_down_ticks_remaining, .. }) if slow_down_ticks_remaining == NonZeroU16::new(u16::MAX) ));        
    }

    #[test]
    fn simple_machine_missing_input_ingredients() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::empty()],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert!(game.machines[0].progress.is_none());

        game.tick();

        // still none because the input hatch does not have the required input
        assert!(game.machines[0].progress.is_none());
    }

    #[test]
    fn correct_tick_amount() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert!(game.machines[0].progress.is_none());

        // first tick will set the PROGRESS ticks to `CRUSH_IRON_RECIPE.ticks``
        game.tick();

        // elapsed `CRUSH_IRON_RECIPE.ticks`-1...
        for i in 0..(CRUSH_IRON_RECIPE.ticks - 1) {
            let expected_ticks_remaining = CRUSH_IRON_RECIPE.ticks - i;

            assert!(game.machines[0].progress.is_some());
            assert!(
                matches!(game.machines[0].progress, Some(Progress { ticks_remaining, .. }) if ticks_remaining == NonZeroU16::new(expected_ticks_remaining).unwrap())
            );

            game.tick();
        }

        game.tick();

        // should now be none because in TOTAL ever since the first tick, CRUSH_IRON_RECIPE.ticks have occurred
        assert!(game.machines[0].progress.is_none());
    }

    #[test]
    fn belt_simple_test() {
        let belts = vec![Belt {
            belt_start: HatchReference {
                machine_index: 0,
                hatch_index: 0,
            },
            belt_end: HatchReference {
                machine_index: 1,
                hatch_index: 0,
            },
            buffer: vec![Item::invalid(); 64],
        }];

        let mut game = Game {
            machines: vec![
                placeholder_machine_with_output_hatch(Item {
                    id: IRON_DUST,
                    count: 1,
                }),
                placeholder_machine_with_input_hatch(Item::invalid()),
            ],
            belts,
            poles: vec![],
            wires: vec![],
        };

        assert!(game.belts[0].buffer[0].is_invalid());

        for i in 0..65 {
            println!("starting tick {i}");
            game.tick();

            for buffer_element_index in 0..64 {
                dbg!(buffer_element_index);
                let item = game.belts[0].buffer[buffer_element_index];
                dbg!(item);

                if buffer_element_index == i {
                    assert_eq!(item.id, IRON_DUST);
                    assert_eq!(item.count, 1);
                } else {
                    assert!(item.is_invalid());
                }
            }
        }
    }

    #[test]
    fn craft_tick_order() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::item(RAW_IRON_1, CRUSH_IRON_RECIPE.input[0].count)],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());
        assert!(game.machines[0].progress.is_none());

        assert_eq!(game.machines[0].input[0].buffer, CRUSH_IRON_RECIPE.input[0]);

        // first tick to start the recipe process
        game.tick();

        let initial_ticks = NonZeroU16::new(CRUSH_IRON_RECIPE.ticks).unwrap();
        assert!(matches!(
            game.machines[0].progress,
            Some(Progress {
                ticks_remaining,
                ..
            }) if initial_ticks == ticks_remaining
        ));

        // tick through recipe ticks...
        for _ in 0..(initial_ticks.get()) {
            game.tick();
        }

        assert!(game.machines[0].input[0].buffer.is_invalid());
        assert!(game.machines[0].progress.is_none());

        assert_eq!(
            game.machines[0].output[0].buffer,
            CRUSH_IRON_RECIPE.output[0]
        );
    }

    #[test]
    fn craft_batch() {
        let batch_count = 10;

        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::item(
                    RAW_IRON_1,
                    batch_count * CRUSH_IRON_RECIPE.input[0].count,
                )],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());

        for _ in 0..(CRUSH_IRON_RECIPE.ticks as usize * (batch_count as usize + 1)) {
            game.tick();
        }

        assert_eq!(game.machines[0].output[0].buffer.id, CRUSHED_IRON);
        assert_eq!(
            game.machines[0].output[0].buffer.count,
            CRUSH_IRON_RECIPE.output[0].count * batch_count
        );
    }

    #[test]
    fn craft_missing_inputs() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::empty()],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());

        game.tick();

        assert!(game.machines[0].output[0].buffer.is_invalid());
        assert_eq!(game.machines[0].status, MachineStatus::RecipeInputResourcesMismatchOrEmpty);
    }

    
    #[test]
    fn craft_missing_inputs_partial() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::item(RAW_IRON_1, 1)],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());

        game.tick();

        assert!(game.machines[0].output[0].buffer.is_invalid());
        assert_eq!(game.machines[0].status, MachineStatus::RecipeInputResourcesMismatchOrEmpty);
    }

    #[test]
    fn craft_full_stack_outputs() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch { buffer: CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE.input[0] }],
                output: vec![Hatch { buffer: Item::full_stack(CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE.output[0].id) }],
                recipe: Some(&CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert_eq!(game.machines[0].status, MachineStatus::None);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        assert_eq!(game.machines[0].status, MachineStatus::RecipeOutputResourcesMismatchOrFull);
    }

    #[test]
    fn craft_full_mismatch_id_outputs() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch { buffer: CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE.input[0] }],
                output: vec![Hatch { buffer: Item::full_stack(IRON_DUST) }],
                recipe: Some(&CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert_eq!(game.machines[0].status, MachineStatus::None);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        assert!(game.machines[0].progress.is_none());
        assert_eq!(game.machines[0].status, MachineStatus::RecipeOutputResourcesMismatchOrFull);
    }
    */
}