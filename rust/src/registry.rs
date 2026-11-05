use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::LazyLock;

use arrayvec::ArrayVec;
use petgraph::visit::EdgeRef;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeMachineType {
    Crusher,
    Miner,
    Smelter,
    AlloySmelter,
    Unknown
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeType {
    MachineRecipe {
        recipe_type: RecipeMachineType,
        recipe_index: usize,
    },
    Item {
        id: u8,
        name: &'static str,
    }
}


// TODO: instead of panicking in the branches, return an Option instead
impl NodeType {
    fn unwrap_item(&self) -> u8 {
        match self {
            NodeType::MachineRecipe { recipe_type, recipe_index } => panic!(),
            NodeType::Item { id, ..} => *id,
        }
    }

    fn unwrap_recipe_type(&self) -> RecipeMachineType {
        match self {
            NodeType::MachineRecipe { recipe_type, recipe_index } => *recipe_type,
            NodeType::Item { .. } => panic!(),
        }
    }
    
    fn unwrap_item_name(&self) -> &'static str {
        match self {
            NodeType::MachineRecipe { recipe_type, recipe_index } => panic!(),
            NodeType::Item { name, .. } => name,
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecipeTreeNodeType2 {
    MachineRecipe {
        rtype: RecipeMachineType,
    },
    Item {
        name: &'static str,
        count: usize
    }
}

#[derive(Debug)]
enum RecipeTreeEdgeType {
    Input(usize),
    Output(usize)
}

#[derive(Debug)]
enum EdgeType {
    Input {
        count: u8
    },
    Output {
        // this output is `unique` if the recipe can produce only one of these unique outputs
        // ex: miners can produce *one* of the rawiron1, rawiron2, rawiron3 at any given time
        unique: bool,
        stack_size: u8,
    }
}

// TODO: instead of panicking in the branches, return an Option instead
impl EdgeType {
    fn unwrap_input_item_count(&self) -> u8 {
        match self {
            EdgeType::Input { count } => *count,
            EdgeType::Output { .. } => panic!(),
        }
    }
    fn unwrap_output_item_count(&self) -> u8 {
        match self {
            EdgeType::Input { count } => panic!(),

            // TODO: handle probability shit
            EdgeType::Output { stack_size, .. } => *stack_size,
        }
    }
    fn unwrap_output_unique(&self) -> bool {
        match self {
            EdgeType::Input { count } => panic!(),

            // TODO: handle probability shit
            EdgeType::Output { unique, .. } => *unique,
        }
    }
    
}

#[derive(Debug)]
enum NodeVisitType {
    Machine {
        index: petgraph::graph::NodeIndex,
        process_count: usize,
    },
    Item { 
        index: petgraph::graph::NodeIndex,
        item_count: usize
    }
}

pub struct NodeVisitItem { 
    item_node_index: petgraph::graph::NodeIndex,
    item_count: usize
}

#[derive(Clone, Copy)]
pub struct NodeVisitMachine {
    recipe_node_index: petgraph::graph::NodeIndex,
    process_count: usize,
    source_item_that_requested_the_visit_of_this_node: petgraph::graph::NodeIndex,
    target_source_item_count: usize,
    recipe_type: RecipeMachineType,
}


#[derive(Default, Debug)]
struct PersistentTotalRecipeTree {
    graph: petgraph::Graph<RecipeTreeNodeType2, RecipeTreeEdgeType>,
    index_mapping_function: HashMap<petgraph::graph::NodeIndex, petgraph::graph::NodeIndex>,
    inverse_index_mapping_function: HashMap<petgraph::graph::NodeIndex, petgraph::graph::NodeIndex>,

}

impl PersistentTotalRecipeTree {
    fn add_recipe_node(&mut self, index: petgraph::prelude::NodeIndex, recipe_type: RecipeMachineType)  {
        let index2 = self.index_mapping_function.entry(index).or_insert_with(|| self.graph.add_node(RecipeTreeNodeType2::MachineRecipe { rtype: recipe_type }));
        self.inverse_index_mapping_function.insert(*index2, index);
    }
    
    fn add_item_node(&mut self, index: petgraph::prelude::NodeIndex, name: &'static str, add: usize)  {
        let idx = self.index_mapping_function.entry(index).or_insert_with(|| self.graph.add_node(RecipeTreeNodeType2::Item { count: 0, name }));
        self.inverse_index_mapping_function.insert(*idx, index);
    
        if let RecipeTreeNodeType2::Item { count, .. } = &mut self.graph[*idx] {
            *count += add;
        };
    }
    
    fn add_recipe_input(&mut self, recipe_node_index: petgraph::prelude::NodeIndex, input_item_node_index: petgraph::prelude::NodeIndex, input_count: usize) {
        let recipe_node_index_tree = self.index_mapping_function[&recipe_node_index];
        let item_node_index_tree = self.index_mapping_function[&input_item_node_index];
        self.graph.add_edge(item_node_index_tree, recipe_node_index_tree, RecipeTreeEdgeType::Input(input_count as usize));

    }
    
    fn add_recipe_output(&mut self, recipe_node_index: petgraph::prelude::NodeIndex, output_item_node_index: petgraph::prelude::NodeIndex, output_count: usize) {
        let recipe_node_index_tree = self.index_mapping_function[&recipe_node_index];
        let item_node_index_tree = self.index_mapping_function[&output_item_node_index];
        self.graph.add_edge(recipe_node_index_tree, item_node_index_tree, RecipeTreeEdgeType::Input(output_count as usize));
    }
}


#[derive(Debug)]
pub struct RecipeTree {
    starting_index: (petgraph::graph::NodeIndex,usize),
    // raw_resources_used: HashMap<u8, usize>,
    // unexpanded_resources: HashMap<u8, usize>,
    // unused_byproducts: HashMap<u8, usize>,
    inner: PersistentTotalRecipeTree,
}

impl RecipeTree {
    fn new(starting_index: (petgraph::graph::NodeIndex,usize)) -> Self {
        Self {
            starting_index,
            // raw_resources_used: Default::default(),
            // unexpanded_resources: Default::default(),
            // unused_byproducts: Default::default(),
            inner: Default::default()
        }
    }

    // solves dependencies in the order they come in
    // always picks the first recipe, but we should tweak this so that "cheaper" recipes are chosen first
    pub fn solve_all(&mut self, indexer: &RecipeIndexer) {
        let mut stack = Vec::<(petgraph::graph::NodeIndex,usize)>::new();
        stack.push(self.starting_index);

        while let Some(value) = stack.pop() {
            let results = self.get_possible_crafting_recipes_for_item(indexer, value);

            if !results.is_empty() {            
                let recipe_inputs = self.select_recipe_for_item_node(indexer, results[0]);
                stack.extend(recipe_inputs);
            }
        }
    }

    // returns the multiple crafting recipes that we can use to compute the dependency
    pub fn get_possible_crafting_recipes_for_item(&mut self, indexer: &RecipeIndexer, (item, count): (petgraph::graph::NodeIndex,usize)) -> Vec<NodeVisitMachine> {     
        let edges_towards_a = indexer.graph.edges_directed(item, petgraph::Incoming);
        
        let mut possible_crafting_recipes: Vec<NodeVisitMachine> = Vec::default();

        self.inner.add_item_node(item, indexer.graph[item].unwrap_item_name(), count);

        // machine -> `item=node`
        // each separate edge is a "possible way" to create that item.
        // each of those branches can also have divergent branches
        // maybe doing this recursively could be better? as we could lazily avoid doing any computation unless the user wants to figure out how to craft dependencies 
        for edge in edges_towards_a {
            let unwrap_output_item_count = edge.weight().unwrap_output_item_count() as usize;
            let target_process_count =  usize::div_ceil(count, unwrap_output_item_count);
            let machine_node_index = edge.source();
            possible_crafting_recipes.push(NodeVisitMachine {
                recipe_node_index: machine_node_index,
                process_count: target_process_count,
                source_item_that_requested_the_visit_of_this_node: item,
                target_source_item_count: target_process_count * unwrap_output_item_count,
                recipe_type: indexer.graph[machine_node_index].unwrap_recipe_type()
            });
        }


        possible_crafting_recipes
    }

    // select a specific crafting recipe and returns a list of the input item dependencies
    pub fn select_recipe_for_item_node(&mut self, indexer: &RecipeIndexer, visit: NodeVisitMachine) -> Vec<(petgraph::graph::NodeIndex, usize)> {
        self.inner.add_recipe_node(visit.recipe_node_index, visit.recipe_type);

        let current_node_index = visit.recipe_node_index;            
        let edges_towards_a = indexer.graph.edges_directed(current_node_index, petgraph::Incoming);
        let edges_from_a = indexer.graph.edges_directed(current_node_index, petgraph::Outgoing);        
        let mut pre_reqs = Vec::<(petgraph::graph::NodeIndex, usize)>::default();

        // item -> `machine=node`
        for edge in edges_towards_a {
            let target_item_count = visit.process_count * (edge.weight().unwrap_input_item_count() as usize);
            let input_item_node_index = edge.source();
            pre_reqs.push((input_item_node_index, target_item_count));
            self.inner.add_item_node(input_item_node_index, indexer.graph[input_item_node_index].unwrap_item_name(), target_item_count);
            self.inner.add_recipe_input(visit.recipe_node_index, input_item_node_index, target_item_count);
        }
        
        // `machine=node` -> byproduct
        // ONLY for non-unique recipes (as those are considered generating "byproducts")
        // TODO: figure out a more robust way of handling that
        for edge in edges_from_a {
            let target_item_count = visit.process_count * (edge.weight().unwrap_output_item_count() as usize);
            let target = edge.target();

            if !edge.weight().unwrap_output_unique() {
                self.inner.add_item_node(target, indexer.graph[target].unwrap_item_name(), target_item_count);
                self.inner.add_recipe_output(visit.recipe_node_index, target, target_item_count);
            }
        }

        pre_reqs
    }
}

#[derive(Default, Debug)]
pub struct RecipeIndexer {
    item_indices_to_nodes: HashMap<u8, petgraph::graph::NodeIndex>,
    graph: petgraph::Graph<NodeType, EdgeType>
}

impl RecipeIndexer {
    fn register_item(&mut self, item: u8, name: &'static str) {
        let index = self.graph.add_node(NodeType::Item { id: item, name });
        self.item_indices_to_nodes.insert(item, index);
    }
    
    fn register_recipe_dependency(&mut self, multi_recipe: &MultiRecipe, recipe_type: RecipeMachineType) {
        for (recipe_index, recipe) in multi_recipe.recipes.iter().enumerate() {
            let unique_machine_node_for_recipe = self.graph.add_node(NodeType::MachineRecipe {
                recipe_type,
                recipe_index,
            });
            for single_input in recipe.input.iter() {
                let item_node = self.item_indices_to_nodes[&single_input.id];
                let weight = EdgeType::Input { count: single_input.count };
                self.graph.add_edge(item_node, unique_machine_node_for_recipe, weight);
            }

            for per_hatch_output in recipe.output.iter() {
                for single_output in per_hatch_output.iter() {
                    let item_node = self.item_indices_to_nodes[&single_output.item_id];
                    let weight: EdgeType = EdgeType::Output { stack_size: *single_output.min_max_stack_size.start(), unique: false };
                    self.graph.add_edge(unique_machine_node_for_recipe, item_node, weight);   
                }
            }
        }
    }

    fn register_miner_recipe(&mut self, recipe: &MinerRecipe) {
        let miner_node = self.graph.add_node(NodeType::MachineRecipe { recipe_type: RecipeMachineType::Miner, recipe_index: 0 });
        for output in recipe.outputs.iter() {
            let item_node = self.item_indices_to_nodes[&output.item_id];
            let weight: EdgeType = EdgeType::Output { stack_size: *output.min_max_stack_size.start(), unique: true };
            self.graph.add_edge(miner_node, item_node, weight);
        }
    }
    
    pub fn compute_recipe_requirements(&self, id: u8, count: usize) -> RecipeTree {
        let starting_node = self.item_indices_to_nodes[&id];

        if petgraph::algo::is_cyclic_directed(&self.graph) {
            panic!("indexed graph is cyclic. should not happen");
        }
        
        RecipeTree::new((starting_node, count))
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
                    flags: RegistryItemFlags::empty()
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

    fn with_flags(mut self, flags: RegistryItemFlags) -> Self {
        self.item.data.flags = flags;
        self
    }
}

pub struct MultiRecipeBuilder {
    rtype: RecipeMachineType,
    recipes: Vec<Recipe>,
    id: &'static str,
}
impl MultiRecipeBuilder {
    pub fn new(id: &'static str, rtype: RecipeMachineType) -> Self {
        Self {
            rtype,
            id,
            recipes: Vec::new()
        }
    }

    pub fn with(mut self, recipe: RecipeBuilder) -> Self {
        self.recipes.push(recipe.build());
        self
    }

    pub fn build(self) -> MultiRecipe {
        MultiRecipe {
            rtype: self.rtype,
            id: self.id,
            recipes: ArrayVec::from_iter(self.recipes),
        }
    }
}


pub struct RecipeBuilder {
    recipe: Recipe,
    inputs: Vec<Item>,
    outputs: Vec<Vec<ItemGenerator>>,
}
impl RecipeBuilder {
    pub fn new(id: &'static str, ticks: u16, load: isize) -> Self {
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

    pub fn input(mut self, item: Item) -> Self {
        self.inputs.push(item);
        self
    }

    pub fn output(mut self, item: Item) -> Self {
        self.outputs.push(vec![ItemGenerator::with_default(item)]);
        self
    }

    pub fn output_2<const N: usize>(mut self, items: [ItemGenerator; N]) -> Self {
        self.outputs.push(items.into_iter().collect());
        self
    }
    /*

    pub fn output_with_properties(mut self, item_id: u8, min_max_stack_size: std::ops::RangeInclusive<u8>, min_max_dirtyness: std::ops::RangeInclusive<u8>, probability: Option<f32>) -> Self {
        self.outputs.push(ItemGenerator {
            item_id,
            min_max_stack_size,
            min_max_dirtyness,
            probability
        });
        self
    }
    */

    pub fn build(self) -> Recipe {
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

    fn add_possibility(mut self, item_id: u8, min_max_stack_size: std::ops::RangeInclusive<u8>, min_max_dirtyness: std::ops::RangeInclusive<u8>, probability: Option<f32>) -> Self {
        self.0.outputs.push(ItemGenerator {
            item_id,
            min_max_stack_size,
            min_max_dirtyness,
            probability
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
    indexer: RecipeIndexer,
}

bitflags::bitflags! {
    pub struct RegistryItemFlags: u32 {
        const RAW = 1 << 0;
        const DUST = 1 << 1;
        const INGOT = 1 << 2;
        
        const CONDUCTIVE = 1 << 30;
        const MAGNETIC = 1 << 31;
    }
}

pub struct DefaultRegistryItemData {
    pub item_model_resource: &'static str,
    pub item_material_resource: &'static str,
    pub flags: RegistryItemFlags,
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
                        flags: RegistryItemFlags::empty(),
                    },
                },
            ],
            indexer: Default::default(),
            recipes: Default::default(),
            multi_recipes: Default::default(),
            miner_recipes: Default::default(),
        }
    }

    fn insert_item(&mut self, builder: ItemBuilder) -> u8 {
        let id = self.items.len() as u8;
        self.indexer.register_item(id, builder.item.name);
        self.items.push(builder.item);
        id
    }

    fn insert_miner_recipe(&mut self, builder: MinerRecipeBuilder) {
        self.indexer.register_miner_recipe(&builder.0);
        self.miner_recipes.insert(builder.0.id.to_string(), builder.0);
    }

    fn insert_recipe(&mut self, builder: RecipeBuilder) {
        let multi_recipe = MultiRecipeBuilder::new(builder.recipe.id, RecipeMachineType::Unknown).with(builder).build();
        self.indexer.register_recipe_dependency(&multi_recipe,  multi_recipe.rtype);
        self.multi_recipes.insert(multi_recipe.id.to_string(), multi_recipe);
    }

    fn insert_multi_recipe(&mut self, builder: MultiRecipeBuilder) {
        let multi_recipe = builder.build();
        self.indexer.register_recipe_dependency(&multi_recipe, multi_recipe.rtype);
        self.multi_recipes.insert(multi_recipe.id.to_string(), multi_recipe);
    }

    pub fn build() -> Self {
        let mut reg = Self::empty();

        let raw_coal = reg.insert_item(
            ItemBuilder::new("raw coal", 255)
            .with_model(ItemModel::OreMeshA)
            .with_material(ItemMaterial::RawOre)
            .with_flags(RegistryItemFlags::RAW)
        );

        let crushed_coal = reg.insert_item(
            ItemBuilder::new("crushed coal", 255)
            .with_model(ItemModel::DustMeshA)
            .with_material(ItemMaterial::CrushedOre)
            .with_flags(RegistryItemFlags::RAW)
        );

        let coal_dust = reg.insert_item(
            ItemBuilder::new("coal dust", 255)
            .with_model(ItemModel::DustMeshB)
            .with_material(ItemMaterial::CrushedOre)
            .with_flags(RegistryItemFlags::DUST)
        );

        let raw_iron = reg.insert_item(
            ItemBuilder::new("raw iron 1", 255)
            .with_model(ItemModel::OreMeshA)
            .with_material(ItemMaterial::RawOre)
            .with_flags(RegistryItemFlags::RAW)
        );

        let raw_iron_1 = reg.insert_item(
            ItemBuilder::new("raw iron 2", 255)
            .with_model(ItemModel::OreMeshB)
            .with_material(ItemMaterial::RawOre)
            .with_flags(RegistryItemFlags::RAW)
        );

        let raw_iron_2 = reg.insert_item(
            ItemBuilder::new("raw iron 3", 255)
            .with_model(ItemModel::OreMeshC)
            .with_material(ItemMaterial::RawOre)
            .with_flags(RegistryItemFlags::RAW)
        );

        let crushed_iron = reg.insert_item(
            ItemBuilder::new("crushed iron", 255)
            .with_model(ItemModel::DustMeshA)
            .with_material(ItemMaterial::CrushedOre)
            .with_flags(RegistryItemFlags::RAW)
        );

        let iron_dust = reg.insert_item(
            ItemBuilder::new("iron dust", 255)
            .with_model(ItemModel::DustMeshB)
            .with_material(ItemMaterial::CrushedOre)
            .with_flags(RegistryItemFlags::DUST | RegistryItemFlags::MAGNETIC)
        );

        let iron_ingot = reg.insert_item(
            ItemBuilder::new("iron ingot", 255)
            .with_model(ItemModel::IngotMesh)
            .with_material(ItemMaterial::Ingot)
            .with_flags(RegistryItemFlags::INGOT | RegistryItemFlags::MAGNETIC | RegistryItemFlags::CONDUCTIVE)
        );

        let steel_dust = reg.insert_item(
            ItemBuilder::new("steel dust", 255)
            .with_model(ItemModel::DustMeshB)
            .with_material(ItemMaterial::CrushedOre)
            .with_flags(RegistryItemFlags::DUST | RegistryItemFlags::MAGNETIC | RegistryItemFlags::CONDUCTIVE)
        );


        reg.insert_miner_recipe(
            MinerRecipeBuilder::new("mine_iron", 128, 30)
                .add_possibility(raw_iron, 1..=4, 0..=127, None)
                .add_possibility(raw_iron_1, 1..=4, 0..=127, None)
                .add_possibility(raw_iron_2, 1..=4, 0..=127, None)
        );

        reg.insert_miner_recipe(
            MinerRecipeBuilder::new("mine_coal", 128, 30)
                .add_possibility(raw_coal, 1..=4, 0..=127, None)
        );

        reg.insert_multi_recipe(MultiRecipeBuilder::new("crusher", RecipeMachineType::Crusher)
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron)).output(Item::one(crushed_iron)))
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron_1)).output(Item::one(crushed_iron)))
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(raw_iron_2)).output(Item::one(crushed_iron)))
            .with(RecipeBuilder::new("crush_iron", 16, 10).input(Item::one(crushed_iron)).output(Item::one(iron_dust)))

            .with(RecipeBuilder::new("crush_coal", 16, 10).input(Item::one(raw_coal)).output(Item::one(crushed_coal)))
            .with(RecipeBuilder::new("crush_coal", 16, 10).input(Item::one(crushed_coal)).output(Item::one(coal_dust)))
        );
        
        reg.insert_multi_recipe(MultiRecipeBuilder::new("smelter", RecipeMachineType::Smelter)
            .with(RecipeBuilder::new("smelt", 4, 1).input(Item::one(iron_dust)).output(Item::one(iron_ingot)))
        );
        
        reg.insert_multi_recipe(MultiRecipeBuilder::new("alloy_smelter", RecipeMachineType::AlloySmelter)
            .with(RecipeBuilder::new("steel", 16, 10)
                .input(Item::one(crushed_coal)).input(Item::one(iron_dust))
                .output(Item::one(steel_dust)))
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
pub struct ItemGenerator {
    pub item_id: u8,
    pub min_max_stack_size: std::ops::RangeInclusive<u8>,
    pub min_max_dirtyness: std::ops::RangeInclusive<u8>,
    pub probability: Option<f32>,
}

impl ItemGenerator {
    pub(crate) fn with_default(item: Item) -> Self {
        Self {
            item_id: item.id,
            min_max_stack_size: item.count..=item.count,
            min_max_dirtyness: 0..=0,
            probability: None,
        }
    }

    pub(crate) fn new(id: u8, count: u8) -> Self {
        Self::with_default(Item::new(id, count))
    }

    pub(crate) fn with_probability(mut self, probs: f32) -> Self {
        self.probability = Some(probs);
        self
    }

    pub(crate) fn generate(&self, rng: &mut fastrand::Rng) -> Option<Item> {
        if let Some(p) = self.probability {
            if p < rng.f32_inclusive() {
                return None
            }
        }

        let mut item = Item::invalid();
        item.id = self.item_id;
        item.count = rng.u8(self.min_max_stack_size.clone());
        item.fill_properties(|| {
            crate::items::PropertiesForSingleItem {
                temperature: 270,
                dirtyness: rng.u8(self.min_max_dirtyness.clone()),
            }
        });

        Some(item)
    }
}

#[derive(Debug)]
pub struct Recipe {
    pub id: &'static str,
    pub name: &'static str,
    pub input: Vec<Item>,
    pub output: Vec<Vec<ItemGenerator>>,
    pub ticks: u16,
    pub load: LoadUnit,
}


#[derive(Debug)]
pub struct MultiRecipe {
    pub rtype: RecipeMachineType,
    pub id: &'static str,
    pub recipes: ArrayVec<Recipe, 32>,
}
impl MultiRecipe {
    pub(crate) fn new(recipe: Recipe) -> Self {
        Self {
            rtype: RecipeMachineType::Unknown,
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
pub struct MinerRecipe {
    pub id: &'static str,
    pub name: &'static str,
    pub outputs: Vec<ItemGenerator>,
    pub ticks: u16,
    pub load: LoadUnit,
}



#[cfg(test)]
mod default_registry_tests {
    use super::*;

    #[test]
    fn test_indexer() {
        let registry = DefaultRegistry::build();
        dbg!(&registry.indexer.graph);
        
        for idx in 0..(registry.items.len()-1) {
            let item_idx = (idx+1) as u8;
            let mut recipe_tree = registry.indexer.compute_recipe_requirements(item_idx, 1);
            recipe_tree.solve_all(&registry.indexer);
            dbg!(registry.registry_item(item_idx).name);
            
            // dbg!(recipe_tree.raw_resources_used);
            // dbg!(recipe_tree.unexpanded_resources);
            dbg!(&recipe_tree.inner);

            /*
            let result = petgraph::algo::toposort(&recipe_tree.inner.graph, None).unwrap();
            let mapped = result.into_iter().map(|x| {
                let y = recipe_tree.inner.inverse_index_mapping_function[&x];
                let item_id = registry.indexer.graph[y];
                (recipe_tree.inner.graph[x], item_id)
            }).collect::<Vec<_>>();
            dbg!(mapped);
            */
            
            // let raw_resources_uses = recipe_tree.list_total_prerequisite_resources();
            // assert!(recipe_tree.list_total_prerequisite_resources().len() >= 1);
            // dbg!(recipe_tree);
        }    
    }
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