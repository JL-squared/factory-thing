use std::collections::HashMap;
use std::sync::LazyLock;

use crate::{Item, Miner};
use crate::common::*;

pub trait Registry {
    type Data: 'static;

    fn registry_item(&self, id: u8) -> &RegistryItem<Self::Data>;
    fn registry_recipe(&self, string_id: &str) -> &Recipe;
    fn registry_miner_recipe(&self, string_id: &str) -> &MinerRecipe;
    fn list_registry_items(&self) -> impl Iterator<Item = &RegistryItem<Self::Data>> {
        std::iter::empty()
    }

    fn stack_size(&self, id: u8) -> u8 {
        Self::registry_item(self, id).stack_size
    }

    fn name(&self, id: u8) -> &str {
        Self::registry_item(self, id).name
    }
}

struct ItemBuilder {
    item: RegistryItem<DefaultRegistryItemData>,
}

enum ItemModel {
    OreMeshA,
    OreMeshB,
    OreMeshC,
    
    DustMeshA,
    DustMeshB,
    IngotMesh,
}

enum ItemMaterial {
    RawOre,
    CrushedOre,
    Ingot
}

impl ItemBuilder {
    fn new(name: &'static str, stack_size: u8) -> Self {
        Self {
            item: RegistryItem {
                name,
                stack_size,
                data: DefaultRegistryItemData {
                    item_model_resource: "",
                    item_material_resource: "",
                }
            }
        }
    }

    fn with_model(mut self, model: ItemModel) -> Self {
        self.item.data.item_model_resource = match model {
            ItemModel::OreMeshA => "res://models/Ore Mesh A.blend",
            ItemModel::OreMeshB => "res://models/Ore Mesh B.blend",
            ItemModel::OreMeshC => "res://models/Ore Mesh C.blend",
            
            ItemModel::DustMeshA => "res://models/Dust Mesh A.blend",
            ItemModel::DustMeshB => "res://models/Dust Mesh B.blend",
            ItemModel::IngotMesh => "res://models/Ingot Mesh.blend",
        };
        self
    }

    fn with_material(mut self, material: ItemMaterial) -> Self {
        self.item.data.item_material_resource = match material {
            ItemMaterial::RawOre => "res://materials/raw_ore.tres",
            ItemMaterial::CrushedOre => "res://materials/crushed_ore.tres",
            ItemMaterial::Ingot => "res://materials/ingot.tres",
        };
        self
    }
}

struct RecipeBuilder {
    recipe: Recipe,
    inputs: Vec<Item>,
    outputs: Vec<Item>,
}
impl RecipeBuilder {
    fn new(id: &'static str, ticks: u16, load: isize) -> Self {
        Self {
            recipe: Recipe {
                id,
                name: id,
                input: &[],
                output: &[],
                ticks,
                load,
            },
            inputs: Default::default(),
            outputs: Default::default(),
        }
    }

    fn input(mut self, item: Item) -> Self {
        self.inputs.push(item);
        self
    }

    fn output(mut self, item: Item) -> Self {
        self.outputs.push(item);
        self
    }
}


struct MinerRecipeBuilder(MinerRecipe);
impl MinerRecipeBuilder {
    fn new(id: &'static str, ticks: u16, load: isize, item: Item) -> Self {
        Self(MinerRecipe {
            id,
            name: id,
            output: item,
            ticks,
            load,
        })
    }
}

#[derive(Default)]
pub struct DefaultRegistry {
    items: Vec<RegistryItem<DefaultRegistryItemData>>,
    recipes: HashMap<String, Recipe>,
    miner_recipes: HashMap<String, MinerRecipe>,
}


pub struct DefaultRegistryItemData {
    pub item_model_resource: &'static str,
    pub item_material_resource: &'static str,
}


impl DefaultRegistry {
    fn empty() -> Self {
        Self {
            items: vec![
                RegistryItem {
                    name: "invalid",
                    stack_size: 0,
                    data: DefaultRegistryItemData {
                        item_model_resource: "",
                        item_material_resource: "",
                    },
                },
            ],
            recipes: Default::default(),
            miner_recipes: Default::default(),
        }
    }

    fn insert_item(&mut self, builder: ItemBuilder) -> u8 {
        let id = self.items.len() as u8;
        self.items.push(builder.item);
        id
    }

    fn insert_miner_recipe(&mut self, builder: MinerRecipeBuilder) {
        self.miner_recipes.insert(builder.0.id.to_string(), builder.0);
    }

    fn insert_recipe(&mut self, builder: RecipeBuilder) {
        let RecipeBuilder {
            mut recipe,
            inputs,
            outputs,
        } = builder;

        // safe-ish to do this because the registry lives as long as the program lives
        recipe.input = inputs.leak::<'static>();
        recipe.output = outputs.leak::<'static>();
        
        self.recipes.insert(recipe.id.to_string(), recipe);
    }

    pub fn build() -> Self {
        let mut reg = Self::empty();

        let raw_iron = reg.insert_item(
            ItemBuilder::new("raw iron 1", 255)
            .with_model(ItemModel::OreMeshA)
            .with_material(ItemMaterial::RawOre)
        );

        let raw_iron_1 = reg.insert_item(
            ItemBuilder::new("raw iron 2", 255)
            .with_model(ItemModel::OreMeshB)
            .with_material(ItemMaterial::RawOre)
        );

        let raw_iron_2 = reg.insert_item(
            ItemBuilder::new("raw iron 3", 255)
            .with_model(ItemModel::OreMeshC)
            .with_material(ItemMaterial::RawOre)
        );

        let crushed_iron = reg.insert_item(
            ItemBuilder::new("crushed iron", 255)
            .with_model(ItemModel::DustMeshA)
            .with_material(ItemMaterial::CrushedOre)
        );

        let iron_dust = reg.insert_item(
            ItemBuilder::new("iron dust", 255)
            .with_model(ItemModel::DustMeshB)
            .with_material(ItemMaterial::CrushedOre)
        );

        let iron_ingot = reg.insert_item(
            ItemBuilder::new("iron ingot", 255)
            .with_model(ItemModel::IngotMesh)
            .with_material(ItemMaterial::Ingot)
        );

        reg.insert_recipe(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron)).output(Item::one(crushed_iron)));
        reg.insert_recipe(RecipeBuilder::new("crush_iron_more", 32, 10).input(Item::one(crushed_iron)).output(Item::one(iron_dust)));
        reg.insert_recipe(RecipeBuilder::new("smelt_iron", 4, 1).input(Item::one(iron_dust)).output(Item::one(iron_ingot)));
        
        reg.insert_miner_recipe(MinerRecipeBuilder::new("mine_iron", 128, 30, Item::one(raw_iron)));

        reg
    }
}

impl Registry for DefaultRegistry {
    type Data = DefaultRegistryItemData;

    fn registry_item(&self, id: u8) -> &RegistryItem<DefaultRegistryItemData> {
        &self.items.get(id as usize).expect("item id invalid")
    }

    fn registry_recipe(&self, string_id: &str) -> &Recipe {
        &self.recipes.get(string_id).expect(&format!("recipe '{string_id}' is not in registry"))
    }

    fn registry_miner_recipe(&self, string_id: &str) -> &MinerRecipe {
        &self.miner_recipes.get(string_id).expect(&format!("miner recipe '{string_id}' is not in registry"))
    }
    
    fn list_registry_items(&self) -> impl Iterator<Item = &RegistryItem<Self::Data>> {
        self.items.iter()
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


/*
#[derive(Debug)]
pub struct MachineArchetype {
    pub recipe: Vec<(RecipeRequirement, Recipe)>,
}

#[derive(Debug)]
pub struct RecipeRequirement {
    pub modules_attached: Vec<crate::modules::ModuleType>,
}
*/

// (a,b) -> c
// 8a -> 8x + 8y
// 4x+4y+4b -> z + d'
// 4b+z -> 1c

// procedural resource
/*
resource::builder()
    .add_raw_ore()
    .add_crushed()
    .add_dust()
    .add_pulverized()
    .add_aqeous()
    .add_molten()
    .implements_property(ItemProperty::Magnetized)
    .implements_property(ItemProperty::Temperature)
    .implements_property(ItemProperty::Malleability)
    .implements_property(ItemProperty::Dirtyness)
*/

// procedural recipe builder
/*
// crush tree: raw ore -> crushed -> dust -> pulverized
recipe::crush_recipe_builder()
    .input( { resource : variant } )
    .load(123)
    .time(1s)
    .output( { resource : next variant in "crush" tree } )
*/

