use crate::{FluidType, Item, registry::Registry};

#[derive(Default)]
pub struct Inventory {
    // TODO: how should item stacks be handled?
    items: Vec<Item>,

    // TODO: need multiple fluid types 
    // need buckets / containers for realism cause that'd be cool
    fluids: FluidType
}

impl Inventory {

    pub fn has_crafting_requirement(&self, crafting_input: &Item) -> bool {
        self.items.iter().any(|item| item.id == crafting_input.id && item.count >= crafting_input.count)
    }

    fn try_craft_once<R: Registry>(&mut self, crafting_recipe_id: u8) {
        if let Some(recipe) = R::registry_crafting_recipe(crafting_recipe_id) {
            for input in recipe.input {
                if self.has_crafting_requirement(input) {
                    // TODO crafting shit
                }
            }
        }
    }
}