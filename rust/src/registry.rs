use std::collections::HashMap;
use std::sync::LazyLock;

use arrayvec::ArrayVec;

use crate::{Item, Miner};
use crate::common::*;

pub trait Registry {
    type Data: 'static;

    fn registry_item(&self, id: u8) -> &RegistryItem<Self::Data>;
    fn registry_recipe(&self, string_id: &str) -> &Recipe;
    fn registry_multi_recipe(&self, string_id: &str) -> &MultiRecipe;
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

struct MultiRecipeBuilder {
    recipes: Vec<Recipe>,
    id: &'static str,
}
impl MultiRecipeBuilder {
    fn new(id: &'static str) -> Self {
        Self {
            id,
            recipes: Vec::new()
        }
    }

    fn with(mut self, recipe: RecipeBuilder) -> Self {
        self.recipes.push(recipe.build());
        self
    }

    fn build(self) -> MultiRecipe {
        MultiRecipe {
            id: self.id,
            recipes: ArrayVec::from_iter(self.recipes),
        }
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
                input: Vec::default(),
                output: Vec::default(),
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

    fn build(self) -> Recipe {
        let RecipeBuilder {
            mut recipe,
            inputs,
            outputs,
        } = self;

        recipe.input = inputs;
        recipe.output = outputs;
        
        recipe
    }
}


struct MinerRecipeBuilder(MinerRecipe);
impl MinerRecipeBuilder {
    fn new(id: &'static str, ticks: u16, load: isize) -> Self {
        Self(MinerRecipe {
            id,
            name: id,
            outputs: vec![],
            ticks,
            load,
        })
    }

    fn add_possibility(mut self, item_id: u8, min_max_stack_size: std::ops::Range<u8>, min_max_dirtyness: std::ops::Range<u8>) -> Self {
        self.0.outputs.push(MinerPossibleOutput {
            item_id,
            min_max_stack_size,
            min_max_dirtyness,
        });
        self
    }
}

#[derive(Default)]
pub struct DefaultRegistry {
    items: Vec<RegistryItem<DefaultRegistryItemData>>,
    recipes: HashMap<String, Recipe>,
    multi_recipes: HashMap<String, MultiRecipe>,
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
            multi_recipes: Default::default(),
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
        self.insert_multi_recipe(MultiRecipeBuilder::new(builder.recipe.id).with(builder));
        // let recipe = builder.build();
        // self.recipes.insert(recipe.id.to_string(), recipe);
    }

    fn insert_multi_recipe(&mut self, builder: MultiRecipeBuilder) {
        let multi_recipe = builder.build();
        self.multi_recipes.insert(multi_recipe.id.to_string(), multi_recipe);
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


        reg.insert_multi_recipe(MultiRecipeBuilder::new("crush_iron")
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron)).output(Item::one(crushed_iron)))
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron_1)).output(Item::one(crushed_iron)))
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron_2)).output(Item::one(crushed_iron)))
        );
        
        reg.insert_recipe(RecipeBuilder::new("crush_iron_more", 32, 10).input(Item::one(crushed_iron)).output(Item::one(iron_dust)));
        reg.insert_recipe(RecipeBuilder::new("smelt_iron", 4, 1).input(Item::one(iron_dust)).output(Item::one(iron_ingot)));
        
        reg.insert_miner_recipe(
            MinerRecipeBuilder::new("mine_iron", 128, 30)
                .add_possibility(raw_iron, 1..5, 0..128)
                .add_possibility(raw_iron_1, 1..5, 0..128)
                .add_possibility(raw_iron_2, 1..5, 0..128)
        );

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

    fn registry_multi_recipe(&self, string_id: &str) -> &MultiRecipe {
        &self.multi_recipes.get(string_id).expect(&format!("multi-recipe '{string_id}' is not in registry"))
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
    pub input: Vec<Item>,
    pub output: Vec<Item>,
    pub ticks: u16,
    pub load: LoadUnit,
}


#[derive(Debug)]
pub struct MultiRecipe {
    pub id: &'static str,
    pub recipes: ArrayVec<Recipe, 32>,
}
impl MultiRecipe {
    pub(crate) fn new(recipe: Recipe) -> Self {
        Self {
            id: recipe.id,
            recipes: {
                let mut vec = ArrayVec::<Recipe, 32>::new_const();
                vec.push(recipe);
                vec
            },
        }
    }
}

#[derive(Debug)]
pub struct MinerPossibleOutput {
    pub item_id: u8,
    pub min_max_stack_size: std::ops::Range<u8>,
    pub min_max_dirtyness: std::ops::Range<u8>,
}

#[derive(Debug)]
pub struct MinerRecipe {
    pub id: &'static str,
    pub name: &'static str,
    pub outputs: Vec<MinerPossibleOutput>,
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

