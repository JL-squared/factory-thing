use arrayvec::ArrayVec;
use smallvec::SmallVec;

use crate::registry;



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PropertiesForSingleItem {
    pub temperature: u16,
    pub dirtyness: u8
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AveragedPropertiesForStack {
    pub temperature: f32,
    pub dirtyness: f32
}


// TODO: probably compress this at runtime
// probably using some sort of RLE
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Properties {
    // kelvin    
    // combine rule: average
    pub temperature: [u16; 256],
    
    // percentage
    // combine rule: maximum
    pub dirtyness: [u8; 256]
}

impl Properties {
    pub const NORMAL: Properties = Properties {
        temperature: [270; 256],
        dirtyness: [0; 256],
    };

    pub fn mem_copy_and_invalidate_src(src: &mut Properties, src_offset: u8, dst: &mut Properties, dst_offset: u8, count: u8) {
        fn inner<T: Copy+Default>(src: &mut [T], src_offset: usize, dst: &mut [T], dst_offset: usize, count: usize) {
            let src_slice = &mut src[src_offset..(src_offset+count)];
            let dst_slice = &mut dst[dst_offset..(dst_offset+count)];
            dst_slice.copy_from_slice(src_slice);
            src_slice.fill(T::default());
        }

        inner(&mut src.temperature, src_offset as usize, &mut dst.temperature, dst_offset as usize, count as usize);
        inner(&mut src.dirtyness, src_offset as usize, &mut dst.dirtyness, dst_offset as usize, count as usize);
    
    }
}

// pub enum ItemTakeResult {
//     Partial(u8),
//     FullStack
// } 

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub id: u8,
    pub count: u8,
    pub properties: Properties,
}

impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Item")
            .field("id", &self.id)
            .field("count", &self.count)
            .field("dirtyness", &&self.properties.dirtyness[0..(self.count as usize)])
            .field("temperature", &&self.properties.temperature[0..(self.count as usize)]).finish()
    }
}

impl Item {
    pub const fn one(id: u8) -> Self {
        Self { id, count: 1, properties: Properties::NORMAL }
    }

    pub fn full_stack<R: registry::Registry>(id: u8, registry: &R) -> Self {
        Self { id, count: registry.stack_size(id), properties: Properties::NORMAL }
    }

    pub const fn new(id: u8, count: u8) -> Self {
        Self { id, count, properties: Properties::NORMAL }
    }

    pub const fn invalid() -> Self {
        Self { id: 0, count: 0, properties: Properties::NORMAL }
    }

    pub const fn is_invalid(&self) -> bool {
        self.id == 0 && self.count == 0
    }

    pub fn is_full_stack<R: registry::Registry>(&self, registry: &R) -> bool {
        if self.is_invalid() {
            false
        } else {
            self.count == registry.stack_size(self.id)
        }
    }

    pub const fn invalidate(&mut self) {
        *self = Self::invalid();
        self.properties.temperature = [0; 256];
        self.properties.dirtyness = [0; 256];   
    }

    pub fn fill_properties(&mut self, mut callback: impl FnMut() -> PropertiesForSingleItem) {
        self.properties.temperature.fill(0);
        self.properties.dirtyness.fill(0);        
        
        for i in 0..self.count {
            let val = callback();
            self.properties.temperature[i as usize] = val.temperature;
            self.properties.dirtyness[i as usize] = val.dirtyness;
        }
    }

    pub fn averaged_properties(&self) -> AveragedPropertiesForStack {
        let c = self.count as usize;
        let cf = self.count as f32; 
        AveragedPropertiesForStack {
            temperature: self.properties.temperature[0..c].iter().map(|x| *x as u64).sum::<u64>() as f32 / cf,
            dirtyness: self.properties.dirtyness[0..c].iter().map(|x| *x as u64).sum::<u64>() as f32 / cf,
        }
    }

    // `from` can be invalid
    // `to` can be invalid
    // this will do the transfer appropriately. modifies both
    // returns the transferred count
    pub fn transfer_limited<R: registry::Registry>(src: &mut Item, dst: &mut Item, max_transfer_count: u8, registry: &R) -> u8 {
        /*
        if max_transfer_count == 0 {
            return 0
        }
        */

        if src.is_invalid() {
            return 0;
        }

        if dst.id != src.id && !dst.is_invalid() {
            // different ID, don't accumulate
            return 0;
        }

        // src ID is valid (but dst ID can be invalid), do transfer but STACK LIMITED
        let stack_size = registry.stack_size(src.id);

        // remaining slots in DST that can be filled
        let remaining_slots = stack_size - if dst.is_invalid() { 0 } else { dst.count };

        // limited by src count as well
        let transferred_amount = remaining_slots.min(src.count);

        // limited by transfer count as well
        let transferred_amount = transferred_amount.min(max_transfer_count);
    
        assert!(transferred_amount <= stack_size);
        assert!(transferred_amount <= src.count);
        
        // transfer item properties
        Properties::mem_copy_and_invalidate_src(&mut src.properties, src.count-transferred_amount, &mut dst.properties, dst.count, transferred_amount);

        dst.count += transferred_amount;
        src.count -= transferred_amount;

        if dst.count > 0 {
            dst.id = src.id;
        }

        if src.count == 0 {
            src.invalidate();
        }

        transferred_amount
    }

    pub fn accumulate<R: registry::Registry>(&mut self, other: &Item, registry: &R) {
        assert!(self.is_invalid() || self.id == other.id);

        if !self.is_invalid() {
            let checked = self.count.checked_add(other.count);
            assert!(checked.is_some(), "accumulation count overflow integer type");
            let stack_size = registry.stack_size(self.id);
            assert!(matches!(checked, Some(x) if x <= stack_size), "accumulation count overflow stack size");
        }

        self.id = other.id;
        self.count += other.count;
    }

    pub fn can_accumulate_from<R: registry::Registry>(&self, other: &Item, registry: &R) -> bool {
        if other.is_invalid() {
            return false;
        }

        if self.is_invalid() {
            true
        } else if self.id == other.id && self.count.checked_add(other.count).map(|res| res <= registry.stack_size(self.id)).unwrap_or_default() {
            true
        } else {
            false
        }
    }

    pub fn display<R: registry::Registry>(&self, registry: &R) -> String {
        if self.is_invalid() {
            "Invalid".to_string()
        } else {
            let props = self.averaged_properties();
            format!("\"{}\" ({}) d:{:2}/255, t:{:2}K", registry.name(self.id), self.count, props.dirtyness, props.temperature)
        }
    }

    /*
    // assumes `other` is not invalid
    // assumes `other` has the same id as `self`
    // assumes `self` contains more items than `other`
    pub const fn take(&mut self, other: &Item) {
        assert!(!other.is_invalid());
        assert!(self.id == other.id);
        assert!(self.count >= other.count);


        self.count = self.count - other.count;
        for offset in 0..other.count {
            src.properties.pop(src.count as usize - offset as usize);
            dst.properties.set(dst.count as usize + offset as usize, val);
        }

        if self.count == 0 {
            self.invalidate();
        }
    }
    */

    /*
    // returns the number of items actually taken
    // the returned value is in range of [0..count]
    pub const fn safe_take(&mut self, count: u8) -> u8 {
        // no-op
        if count == 0 {
            return 0;
        }

        let before = self.count;
        self.count = self.count.saturating_sub(count);

        if self.count == 0 {
            // entire stack was invalidated, we took the entire stack amount
            self.invalidate();
            before
        } else {
            // stack was not invalidated, we took `count` amount
            count
        }
    } 
    */
}

#[cfg(test)]
mod item_tests {
    use super::*;
    use crate::{registry::{Registry, RegistryItem}, simulation::*};

    
    #[derive(Default)]
    pub struct ItemTestRegistry;

    impl ItemTestRegistry {
        pub const ITEM_WITH_STACK_SIZE_1: u8 = 1;
        pub const ITEM_WITH_STACK_SIZE_16: u8 = 2;
        pub const ITEM_WITH_STACK_SIZE_255: u8 = 3;

        pub const ITEMS: &[RegistryItem<()>] = &[
            RegistryItem {
                name: "invalid",
                stack_size: 0,
                data: ()
            },
            RegistryItem {
                name: "Item with stack size 1",
                stack_size: 1,
                data: ()
            },
            RegistryItem {
                name: "Item with stack size 16",
                stack_size: 16,
                data: ()
            },
            RegistryItem {
                name: "Iron with stack size 255",
                stack_size: 255,
                data: ()
            },
        ];
    }

    impl Registry for ItemTestRegistry {
        type Data = ();

        fn registry_item(&self, id: u8) -> &RegistryItem<()> {
            &Self::ITEMS[id as usize]
        }
        
        fn registry_recipe(&self, string_id: &str) -> &registry::Recipe {
            todo!()
        }

        fn registry_multi_recipe(&self, string_id: &str) -> &registry::MultiRecipe {
            todo!()
        }
        
        fn registry_miner_recipe(&self, string_id: &str) -> &registry::MinerRecipe {
            todo!()
        }
    }

    static REGISTRY: ItemTestRegistry = ItemTestRegistry;


    #[test]
    fn single_item() {
        assert!(Item::invalid().is_invalid());

        let item = Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert!(!item.is_invalid());
        assert_eq!(item.id, ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert_eq!(item.count, 1);
    }

    /*
    #[test]
    fn invalidate_item() {
        let mut item = Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert!(!item.is_invalid());
        item.invalidate();
        assert!(item.is_invalid());
        
        let mut item = Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert!(!item.is_invalid());
        item.take(&Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255));
        assert!(item.is_invalid());
    }

    #[test]
    fn take_item() {
        let mut item = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        assert_eq!(item.count, 10);
        item.take(&Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255));
        assert_eq!(item.count, 9);
    }

    #[test]
    #[should_panic]
    fn take_item_too_much() {
        let mut item = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 1);
        assert_eq!(item.count, 1);

        // should cause a subtract with overflow; panic
        item.take(&Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10));
    }
    */

    #[test]
    fn accumulate_item() {
        let mut item = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        assert_eq!(item.count, 10);
        item.accumulate::<ItemTestRegistry>(&Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255), &REGISTRY);
        assert_eq!(item.count, 11);
    }

    #[test]
    #[should_panic]
    fn accumulate_item_stack_limit() {
        let mut item = Item::full_stack::<ItemTestRegistry>(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, &REGISTRY);
        assert_eq!(item.count, REGISTRY.stack_size(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255));
        item.accumulate::<ItemTestRegistry>(&Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255), &REGISTRY);
    }

    #[test]
    fn transfer_total_src_inalid_dst_invalid() {
        let mut src = Item::invalid();
        let mut dst = Item::invalid();
        assert!(src.is_invalid());
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert!(src.is_invalid());
        assert!(dst.is_invalid());
    }

    #[test]
    fn transfer_total_src_valid_dst_invalid() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::invalid();
        assert_eq!(src.count, 10);
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert_eq!(dst.count, 10);
        assert!(src.is_invalid());
    }

    #[test]
    fn transfer_total_src_valid_dst_valid_diff_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
    }

    #[test]
    fn transfer_total_src_valid_dst_valid_same_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert!(src.is_invalid());
        assert_eq!(dst.count, 25);
    }
    
    #[test]
    fn transfer_partial_src_valid_dst_invalid() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::invalid();
        assert_eq!(src.count, 10);
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 2, &REGISTRY);
        
        assert_eq!(src.count, 8);
        assert_eq!(src.id, ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert_eq!(dst.count, 2);
        assert_eq!(dst.id, ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
    }

    #[test]
    fn transfer_partial_src_valid_dst_valid_diff_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 2, &REGISTRY);
        
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
    }

    #[test]
    fn transfer_partial_src_valid_dst_valid_same_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 2, &REGISTRY);
        
        assert_eq!(src.count, 8);
        assert_eq!(dst.count, 17);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_invalid() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        let mut dst = Item::invalid();
        assert_eq!(src.count, 16);
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert!(src.is_invalid());
        assert_eq!(dst.count, 16);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_valid_diff_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 15);
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 15);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_valid_same_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert_eq!(src.count, 9);
        assert_eq!(dst.count, 16);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_valid_same_id_2() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 16);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255, &REGISTRY);
        
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 16);
    }
}