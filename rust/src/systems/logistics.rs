
use crate::handle::*;
use crate::items::*;
use crate::common::*;
use crate::registry::*;
use slotmap::*;

#[derive(Debug)]
pub struct Belt {
    pub belt_start: HatchKey,
    pub belt_end: HatchKey,
    pub buffer: Vec<Item>,
    pub last_transfer_tick: u64,
}
impl Belt {
    pub(crate) fn from_hatches(output_hatch: HatchKey, input_hatch: HatchKey, buffer_length: usize) -> Belt {
        Self {
            belt_start: output_hatch,
            belt_end: input_hatch,
            buffer: vec![Item::invalid(); buffer_length],
            last_transfer_tick: 0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.iter().all(|x| *x == Item::invalid())
    }

    pub fn is_saturated(&self) -> bool {
        self.buffer.iter().all(|x| *x != Item::invalid())
    }

    pub fn first_item(&self) -> Item {
        self.buffer[0]
    }
}

#[derive(Debug)]
pub struct Hatch {
    pub buffer: Item,
}

impl Hatch {
    pub const fn item(id: u8, count: u8) -> Self {
        Self {
            buffer: Item { id, count },
        }
    }

    pub const fn empty() -> Self {
        Self {
            buffer: Item::invalid(),
        }
    }
}



#[derive(Debug)]
pub struct Silo {
    pub input: HatchKey,
    pub output: HatchKey,
    pub stack: Vec<Item>,
}



#[derive(Default, Debug)]
pub struct Splitter {
    pub input: HatchKey,
    pub output: Vec<HatchKey>,
    pub toggle: bool,
}



pub fn handle_splitters<R: Registry>(hatches: &mut SlotMap<HatchKey, Hatch>, splitters: &mut SlotMap<SplitterKey, Splitter>, settings: &mut Settings, registry: &R) {
    for splitter in splitters.values_mut() {
        let disjoint_hatches = hatches.get_disjoint_mut([splitter.input, splitter.output[0], splitter.output[1]]).unwrap();
        let [input, output_1, output_2] = disjoint_hatches.map(|x| &mut x.buffer);        

        if !input.is_invalid() {
            if splitter.toggle {
                Item::transfer_limited::<R>(input, output_1, settings.silo_transfer_size, registry);
            } else {
                Item::transfer_limited::<R>(input, output_2, settings.silo_transfer_size, registry);
            }

            splitter.toggle = !splitter.toggle;
        }
    }
}


pub fn handle_silos<R: Registry>(hatches: &mut SlotMap<HatchKey, Hatch>, silos: &mut SlotMap<SiloKey, Silo>, settings: &mut Settings, registry: &R) {
    for silo in silos.values_mut() {
        let input = &mut hatches[silo.input].buffer;
        let mut taken = Item::invalid();
        Item::transfer_limited::<R>(input, &mut taken, settings.silo_transfer_size, registry);

        if !taken.is_invalid() {
            silo.stack.push(taken);
        }

        if let Some(last) = silo.stack.last_mut() {
            let predicate = hatches[silo.output].buffer.can_accumulate_from::<R>(last, registry);

            if predicate {
                let output = &mut hatches[silo.output].buffer;
                Item::transfer_limited::<R>(last, output, settings.silo_transfer_size, registry);

                if last.is_invalid() {
                    silo.stack.pop().unwrap();
                }
            }
        }
    }
}



pub fn handle_belts<R: Registry>(belts: &mut SlotMap<BeltKey, Belt>, hatches: &mut SlotMap<HatchKey, Hatch>, tick: &u64, settings: &mut Settings, registry: &R) {
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
            let predicate = input_hatch.buffer.is_invalid() || (input_hatch.buffer.can_accumulate_from::<R>(buffer.last().unwrap(), registry) || buffer.last().unwrap().is_invalid());

            // order of operations:
            // transfer last element in belt buffer to input hatch
            // roll elements in belt buffer one to the right
            // transfer output hatch item to first element in belt buffer
            if predicate {
                *last_transfer_ticks = *tick;

                // godot::global::godot_print!("belt shifting");

                // element at index buffer.len()-1 is belt output
                let input_hatch = &mut hatches[belt_end];
                Item::transfer_limited::<R>(buffer.last_mut().unwrap(), &mut input_hatch.buffer, settings.belt_transfer_size, registry);

                // I'm shifting... I'm shifting....
                buffer.rotate_right(1);

                // first element must be reset to invalid now...
                buffer[0].invalidate();

                // element at index 0 is the belt input
                // belt takes input from hatch...
                let output_hatch = &mut hatches[belt_start];
                Item::transfer_limited::<R>(&mut output_hatch.buffer, &mut buffer[0], settings.belt_transfer_size, registry);
            }
        }
    }
}
