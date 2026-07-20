use crate::{Item, LoadUnit};

pub trait Registry: Default {
    type Data: 'static;

    fn registry_item(id: u8) -> &'static RegistryItem<Self::Data>;
    fn registry_recipe(string_id: &str) -> &'static Recipe;

    fn stack_size(id: u8) -> u8 {
        Self::registry_item(id).stack_size
    }

    fn name(id: u8) -> &'static str {
        Self::registry_item(id).name
    }
}

#[derive(Default)]
pub struct DefaultRegistry;

pub struct DefaultRegistryItemData {
    pub item_model_resource: &'static str,
    pub item_material_resource: &'static str,
}

impl DefaultRegistry {
    pub const RAW_IRON_1: u8 = 1;
    pub const CRUSHED_IRON: u8 = 2;
    pub const IRON_DUST: u8 = 3;
    pub const IRON_INGOT: u8 = 4;

    pub const ITEMS: &[RegistryItem<DefaultRegistryItemData>] = &[
        RegistryItem {
            name: "invalid",
            stack_size: 0,
            data: DefaultRegistryItemData {
                item_model_resource: "",
                item_material_resource: "",
            },
        },
        RegistryItem {
            name: "Raw Iron",
            stack_size: 255,
            data: DefaultRegistryItemData {
                item_model_resource: "res://models/Ore Mesh A.blend",
                item_material_resource: "res://materials/raw_ore.tres",
            },
        },
        RegistryItem {
            name: "Crushed Iron",
            stack_size: 255,
            data: DefaultRegistryItemData {
                item_model_resource: "res://models/Dust Mesh A.blend",
                item_material_resource: "res://materials/crushed_ore.tres",
            },
        },
        RegistryItem {
            name: "Iron Dust",
            stack_size: 255,
            data: DefaultRegistryItemData {
                item_model_resource: "res://models/Dust Mesh B.blend",
                item_material_resource: "res://materials/crushed_ore.tres",
            },
        },
        RegistryItem {
            name: "Iron Ingot",
            stack_size: 255,
            data: DefaultRegistryItemData {
                item_model_resource: "res://models/Ingot Mesh.blend",
                item_material_resource: "res://materials/ingot.tres",
            },
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
        ticks: 32,
        load: 10,
    };

    pub const MINE_IRON_RECIPE: Recipe = Recipe {
        id: "mine_iron",
        name: "",
        input: &[],
        output: &[Item::one(Self::RAW_IRON_1)],
        ticks: 4,
        load: 1,
    };

    pub const RECIPES: &[&Recipe] = &[
        &Self::CRUSH_IRON_RECIPE,
        &Self::CRUSH_IRON_MORE_RECIPE,
        &Self::SMELT_IRON_RECIPE,
        &Self::MINE_IRON_RECIPE,        
    ];
}

impl Registry for DefaultRegistry {
    type Data = DefaultRegistryItemData;

    fn registry_item(id: u8) -> &'static RegistryItem<DefaultRegistryItemData> {
        &Self::ITEMS[id as usize]
    }

    fn registry_recipe(id: &str) -> &'static Recipe {
        &Self::RECIPES.iter().find(|x| x.id == id).unwrap()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct RegistryItem<Data> {
    pub name: &'static str,
    pub stack_size: u8,
    pub data: Data,
}

#[derive(Debug)]
pub struct Recipe {
    pub id: &'static str,
    pub name: &'static str,
    pub input: &'static [Item],
    pub output: &'static [Item],
    pub ticks: u16,
    pub load: LoadUnit,
}

#[derive(Debug)]
pub struct MinerRecipe {
    pub id: &'static str,
    pub name: &'static str,
    pub output: Item,
    pub ticks: u16,
    pub load: LoadUnit,
}
