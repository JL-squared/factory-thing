
pub struct Settings {
    pub wire_damage_per_tick: Option<u8>,
    pub wire_max_flow: LoadUnit,
    pub belt_buffer_scaling_factor: f32,
    pub belt_ticks_between_transfers: u64,
    pub belt_transfer_size: u8,
    pub silo_transfer_size: u8,
    pub machine_require_clicky_thing_attached: bool,
    pub machine_require_power: bool,


    pub pipe_buffer_scaling_factor: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wire_damage_per_tick: None,
            wire_max_flow: LoadUnit::MAX,
            belt_buffer_scaling_factor: 2f32,
            belt_ticks_between_transfers: 16,
            belt_transfer_size: 1,
            silo_transfer_size: 1,
            machine_require_clicky_thing_attached: false,
            machine_require_power: true,
            pipe_buffer_scaling_factor: 2f32,
        }
    }
}


pub enum BufferSize {
    BufferLength(u32),
    WorldLength(f32),
}



pub type LoadUnit = isize;
pub type HealthUnit = u8;
