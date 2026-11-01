
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
    // invariant: the n-head elements of the array are items that are contiguous and valid
    // upholding this invariant makes some functions easier
    pub buffer: [Item; 3],
}

impl Hatch {
    pub const fn item(id: u8, count: u8) -> Self {
        Self {
            buffer: [Item::new(id, count), Item::invalid(), Item::invalid()],
        }
    }

    pub const fn empty() -> Self {
        Self {
            buffer: [Item::invalid(); 3],
        }
    }

    pub fn can_accumulate_from<R: Registry>(&self, item: &Item, registry: &R) -> bool {
        // first, check if there are any empty slots
        // if there are, then we can fit the new item in there
        if self.buffer.iter().any(|slot| slot.is_invalid()) {
            return true;
        } 

        // if there are no empty slots, check if the item can "stack" into one of the existing slots
        // if not (i.e IDs do not match up) then we cannot accumulate
        if self.buffer.iter().all(|slot| slot.id != item.id) {
            return false;
        }
        
        // IDs will match up so we can export this outside the loop
        let stack_size = registry.stack_size(item.id);
        
        // sum up the empty space
        let empty_space = self.buffer.iter().map(|slot| (stack_size - slot.count) as usize).sum::<usize>();
    
        
        (item.count as usize) < empty_space
    }

    pub(crate) fn can_accumulate_from_without_partial_takes_overflow_conservative<R: Registry>(&self, item: &Item, registry: &R) -> bool {
        self.buffer.iter().any(|buffer_item| {
            // if the item is the same, must make sure that we have enough space in the hatch to place it 
            let same_id = buffer_item.id == item.id;
            let stack_size = registry.stack_size(buffer_item.id);

            // this CAN overflow if stack size is at MAX
            // if we know it will overflow, then we cannot process the recipe
            let opt_non_overflowing_enough_space_considering_stack_size = buffer_item.count.checked_add(item.count).map(|x| x <= stack_size);
            same_id && opt_non_overflowing_enough_space_considering_stack_size.unwrap_or_default() || buffer_item.is_invalid()
        })
    }
    
    // there are two ways the expected behaviour could.. behave:
    // 1. this visits only one item in the buffer, does the take, and returns
    // 2. visits multiple items in the buffer, takes as much as needed, and returns sum (basically a "dedupe" step)
    // we will stick with 2. 
    pub(crate) fn take_limited<R: Registry>(&mut self, transfer_size: u8, registry: &R) -> Item {
        let mut countdown_transfer_size = transfer_size;

        self.buffer.iter_mut().filter(|x| !x.is_invalid()).fold(Item::invalid(), |mut acc, x| {
            let transferred = Item::transfer_limited(x, &mut acc, countdown_transfer_size, registry);
            countdown_transfer_size = countdown_transfer_size.saturating_sub(transferred);
            acc
        })
    }

    pub(crate) fn transfer_to_item_limited<R: Registry>(&mut self, item: &mut Item, transfer_size: u8, registry: &R) {
        let mut countdown_transfer_size = transfer_size;

        for slot in self.buffer.iter_mut() {
            let transferred = Item::transfer_limited(slot, item, countdown_transfer_size, registry);
            countdown_transfer_size = countdown_transfer_size.saturating_sub(transferred);
        }
    }

    pub(crate) fn transfer_from_item_limited<R: Registry>(&mut self, item: &mut Item, transfer_size: u8, registry: &R) {
        let mut countdown_transfer_size = transfer_size;

        for slot in self.buffer.iter_mut() {
            let transferred = Item::transfer_limited(item, slot, countdown_transfer_size, registry);
            countdown_transfer_size = countdown_transfer_size.saturating_sub(transferred);
        }
    }
    
    pub(crate) fn is_invalid(&self) -> bool {
        self.buffer.iter().all(|item| item.is_invalid())
    }
    
    pub(crate) fn fill(&mut self, item: Item) {
        self.buffer.fill(item);
    }
    
    pub(crate) fn contains_item_of_id(&self, id: u8) -> bool {
        self.buffer.iter().any(|item| item.id == id)
    }

    pub(crate) fn count_item_of_id(&self, id: u8) -> usize {
        self.buffer.iter().filter(|item| item.id == id).map(|item| item.count as usize).sum::<usize>()
    }

    pub(crate) fn invalidate(&mut self) {
        for ele in self.buffer.iter_mut() {
            ele.invalidate();
        }
    }
    
    pub(crate) fn full(source_item: Item) -> Hatch {
        Self {
            buffer: [source_item; 3]
        }
    }
    
    pub(crate) fn input_item_requirement(&self, recipe_input_item: &Item) -> bool {
        self.buffer.iter().any(|buffer_item| {
            buffer_item.id == recipe_input_item.id && buffer_item.count >= recipe_input_item.count
        })
    }

    pub fn accumulate<R: Registry>(&mut self, other: &Item, registry: &R) {
        assert!(self.is_invalid() || self.can_accumulate_from(other, registry));
        let mut clone = other.clone();

        for buffer_item in self.buffer.iter_mut() {
            Item::transfer_limited(&mut clone, buffer_item, 255, registry); 
        } 

        assert!(clone.is_invalid());
    }

    // assumes `other` is not invalid
    // returns the result of the item take
    pub fn take(&mut self, other: &Item) {
        assert!(!other.is_invalid());

        let mut clone = other.clone();
        
        for buffer_item in self.buffer.iter_mut() {     
            if buffer_item.id == clone.id {
                clone.count -= buffer_item.safe_take(clone.count);
            }       
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

        /*
        if !input.is_invalid() {
            if splitter.toggle {
                Item::transfer_limited::<R>(input, output_1, settings.silo_transfer_size, registry);
            } else {
                Item::transfer_limited::<R>(input, output_2, settings.silo_transfer_size, registry);
            }

            splitter.toggle = !splitter.toggle;
        }
        */
    }
}


pub fn handle_silos<R: Registry>(hatches: &mut SlotMap<HatchKey, Hatch>, silos: &mut SlotMap<SiloKey, Silo>, settings: &mut Settings, registry: &R) {
    for silo in silos.values_mut() {
        let taken = hatches[silo.input].take_limited::<R>(settings.silo_transfer_size, registry);
        // let input = &mut hatches[silo.input].buffer;
        // let mut taken = Item::invalid();
        // Item::transfer_limited::<R>(input, &mut taken, settings.silo_transfer_size, registry);

        if !taken.is_invalid() {
            silo.stack.push(taken);
        }

        if let Some(last) = silo.stack.last_mut() {
            let predicate = hatches[silo.output].can_accumulate_from::<R>(last, registry);

            if predicate {
                hatches[silo.output].transfer_from_item_limited(last, settings.silo_transfer_size, registry);                

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
            let predicate = input_hatch.is_invalid() || (input_hatch.can_accumulate_from::<R>(buffer.last().unwrap(), registry) || buffer.last().unwrap().is_invalid());

            // order of operations:
            // transfer last element in belt buffer to input hatch
            // roll elements in belt buffer one to the right
            // transfer output hatch item to first element in belt buffer
            if predicate {
                *last_transfer_ticks = *tick;

                // godot::global::godot_print!("belt shifting");

                // element at index buffer.len()-1 is belt output
                let input_hatch = &mut hatches[belt_end];
                input_hatch.transfer_from_item_limited(buffer.last_mut().unwrap(), settings.belt_transfer_size, registry);

                // I'm shifting... I'm shifting....
                buffer.rotate_right(1);

                // first element must be reset to invalid now...
                buffer[0].invalidate();

                // element at index 0 is the belt input
                // belt takes input from hatch...
                let output_hatch = &mut hatches[belt_start];
                output_hatch.transfer_to_item_limited(&mut buffer[0], settings.belt_transfer_size, registry);
            }
        }
    }
}