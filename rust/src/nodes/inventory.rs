use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct InventoryNode {
    base: Base<Node3D>,
    
}

#[godot_api]
impl INode3D for InventoryNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
        }
    }

    fn ready(&mut self) {
    }
}