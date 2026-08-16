use crate::registry;
use paste::*;

macro_rules! gen_enum_variants {
    ($name:ident, $plus:ident, $minus:ident) => {
        paste! {
            #[derive(Clone, Copy, PartialEq, Eq, Debug)]
            pub enum $name {
                [<Tera $plus>],
                [<Giga $plus>],
                [<Mega $plus>],
                [<Kilo $plus>],
                Meh,
                [<Kilo $minus>],
                [<Mega $minus>],
                [<Giga $minus>],
                [<Tera $minus>],
            }
        }
    };
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Temperature {
    Cold,

    Normal,
    
    Hot,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Softness {
    Soft,
    Hard,
}

gen_enum_variants!(Dirtyness, Pure, Dirty);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemProperties {
    pub temperature: Temperature,
    pub softness: Softness,
    pub dirtyness: Dirtyness,
}

impl ItemProperties {
    pub const fn default() -> Self {
        Self {
            temperature: Temperature::Normal,
            softness: Softness::Hard,
            dirtyness: Dirtyness::Meh,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Item {
    pub id: u8,
    pub count: u8,
    pub properties: Vec<ItemProperties>,
}

impl Item {
    pub const fn one(id: u8) -> Self {
        Self { id, count: 1, properties: ItemProperties::default() }
    }

    pub fn full_stack<R: registry::Registry>(id: u8) -> Self {
        Self { id, count: R::stack_size(id), properties: ItemProperties::default() }
    }

    pub const fn new(id: u8, count: u8) -> Self {
        Self { id, count, properties: ItemProperties::default() }
    }

    pub const fn invalid() -> Self {
        Self { id: 0, count: 0, properties: ItemProperties::default() }
    }

    pub const fn is_invalid(&self) -> bool {
        self.id == 0 && self.count == 0
    }

    pub const fn invalidate(&mut self) {
        *self = Self::invalid();
    }


    // `from` can be invalid
    // `to` can be invalid
    // this will do the transfer appropriately. modifies both
    pub fn transfer_limited<R: registry::Registry>(src: &mut Item, dst: &mut Item, max_transfer_count: u8) {
        if src.is_invalid() {
            return;
        }

        if dst.id != src.id && !dst.is_invalid() {
            // different ID, don't accumulate
            return;
        }

        // src ID is valid (but dst ID can be invalid), do transfer but STACK LIMITED
        let stack_size = R::stack_size(src.id);

        // remaining slots in DST that can be filled
        let remaining_slots = stack_size - if dst.is_invalid() { 0 } else { dst.count };

        // limited by src count as well
        let transferred_amount = remaining_slots.min(src.count);

        // limited by transfer count as well
        let transferred_amount = transferred_amount.min(max_transfer_count);
    
        assert!(transferred_amount <= stack_size);
        assert!(transferred_amount <= src.count);
        

        dst.count += transferred_amount;
        src.count -= transferred_amount;

        if dst.count > 0 {
            dst.id = src.id;
        }

        if src.count == 0 {
            src.invalidate();
        }
    }

    pub fn accumulate<R: registry::Registry>(&mut self, other: &Item) {
        assert!(self.is_invalid() || self.id == other.id);

        if !self.is_invalid() {
            let checked = self.count.checked_add(other.count);
            assert!(checked.is_some(), "accumulation count overflow integer type");
            let stack_size = R::stack_size(self.id);
            assert!(matches!(checked, Some(x) if x <= stack_size), "accumulation count overflow stack size");
        }

        self.id = other.id;
        self.count += other.count;
    }

    pub fn can_accumulate_from<R: registry::Registry>(&self, other: &Item) -> bool {
        if other.is_invalid() {
            return false;
        }

        if self.is_invalid() {
            true
        } else if self.id == other.id && self.count.checked_add(other.count).map(|res| res <= R::stack_size(self.id)).unwrap_or_default() {
            true
        } else {
            false
        }
    }

    pub fn display<R: registry::Registry>(&self) -> String {
        if self.is_invalid() {
            "Invalid".to_string()
        } else {
            format!("\"{}\" ({})", R::name(self.id), self.count)
        }
    }

    // assumes `other` is not invalid
    // assumes `other` has the same id as `self`
    pub const fn take(&mut self, other: &Item) {
        assert!(!other.is_invalid());
        assert!(self.id == other.id);


        self.count = self.count - other.count;

        if self.count == 0 {
            self.invalidate();
        }
    }
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

        fn registry_item(id: u8) -> &'static RegistryItem<()> {
            &Self::ITEMS[id as usize]
        }
        
        fn registry_recipe(string_id: &str) -> &'static registry::Recipe {
            todo!()
        }
    }


    #[test]
    fn single_item() {
        assert!(Item::invalid().is_invalid());

        let item = Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert!(!item.is_invalid());
        assert_eq!(item.id, ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert_eq!(item.count, 1);
    }

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

    #[test]
    fn accumulate_item() {
        let mut item = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        assert_eq!(item.count, 10);
        item.accumulate::<ItemTestRegistry>(&Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255));
        assert_eq!(item.count, 11);
    }

    #[test]
    #[should_panic]
    fn accumulate_item_stack_limit() {
        let mut item = Item::full_stack::<ItemTestRegistry>(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255);
        assert_eq!(item.count, ItemTestRegistry::stack_size(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255));
        item.accumulate::<ItemTestRegistry>(&Item::one(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255));
    }

    #[test]
    fn transfer_total_src_inalid_dst_invalid() {
        let mut src = Item::invalid();
        let mut dst = Item::invalid();
        assert!(src.is_invalid());
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert!(src.is_invalid());
        assert!(dst.is_invalid());
    }

    #[test]
    fn transfer_total_src_valid_dst_invalid() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::invalid();
        assert_eq!(src.count, 10);
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert_eq!(dst.count, 10);
        assert!(src.is_invalid());
    }

    #[test]
    fn transfer_total_src_valid_dst_valid_diff_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
    }

    #[test]
    fn transfer_total_src_valid_dst_valid_same_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert!(src.is_invalid());
        assert_eq!(dst.count, 25);
    }
    
    #[test]
    fn transfer_partial_src_valid_dst_invalid() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::invalid();
        assert_eq!(src.count, 10);
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 2);
        
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
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 2);
        
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
    }

    #[test]
    fn transfer_partial_src_valid_dst_valid_same_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 2);
        
        assert_eq!(src.count, 8);
        assert_eq!(dst.count, 17);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_invalid() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        let mut dst = Item::invalid();
        assert_eq!(src.count, 16);
        assert!(dst.is_invalid());
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert!(src.is_invalid());
        assert_eq!(dst.count, 16);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_valid_diff_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_255, 15);
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 15);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_valid_same_id() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 10);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 15);
        assert_eq!(src.count, 10);
        assert_eq!(dst.count, 15);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert_eq!(src.count, 9);
        assert_eq!(dst.count, 16);
    }

    #[test]
    fn transfer_partial_stack_size_limited_src_valid_dst_valid_same_id_2() {
        let mut src = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        let mut dst = Item::new(ItemTestRegistry::ITEM_WITH_STACK_SIZE_16, 16);
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 16);
        
        Item::transfer_limited::<ItemTestRegistry>(&mut src, &mut dst, 255);
        
        assert_eq!(src.count, 16);
        assert_eq!(dst.count, 16);
    }
}