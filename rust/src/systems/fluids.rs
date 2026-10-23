use crate::handle::*;
use crate::common::*;
use crate::registry::*;
use slotmap::*;


// units are milliliters I suppose
pub type FluidType = i32;
pub type VelocityType = i32;

#[derive(Debug)]
pub struct PipeStats {
    pub total_volume: f32,
    pub total_velocity: f32,
    pub normalized_volume: f32,
    pub normalized_velocity: f32,
    pub normalized_volume_2: f32,
    pub normalized_velocity_2: f32
    
}

#[derive(Debug)]
pub struct Pipe {
    pub pipe_start: FluidHatchKey,
    pub pipe_end: FluidHatchKey,

    // volume buffer
    pub buffer: Vec<FluidType>,

    // velocity buffer
    pub velocities: Vec<VelocityType>,
}
impl Pipe {
    pub(crate) fn from_hatches(output_hatch: FluidHatchKey, input_hatch: FluidHatchKey, buffer_length: usize) -> Pipe {
        Pipe {
            buffer: vec![0; buffer_length],
            velocities: vec![0; buffer_length+1],
            pipe_start: output_hatch,
            pipe_end: input_hatch,
        }
    }
    
    pub(crate) fn stats(&self) -> PipeStats {
        
        let total_volume = self.buffer.iter().sum::<i32>() as f32;
        let total_velocity = self.velocities.iter().sum::<i32>() as f32;

        let normalized_volume = total_volume / self.buffer.len() as f32;
        let normalized_velocity = total_velocity / self.buffer.len() as f32;
        

        let normalized_volume_2 = total_volume / (self.buffer.len() as f32-2f32);
        let normalized_velocity_2 = total_velocity / (self.buffer.len() as f32-2f32);
        
        
        PipeStats {
            total_volume,
            total_velocity,
            normalized_volume,
            normalized_velocity,
            normalized_volume_2,
            normalized_velocity_2,
        }
    }
}


#[derive(Debug)]
pub struct FluidHatch {
    pub fluid: FluidType,
}

impl FluidHatch {
    pub fn empty() -> FluidHatch {
        FluidHatch {
            fluid: Default::default(),
        }
    }
}



#[derive(Debug)]
pub struct FluidBuffer {
    pub input: FluidHatchKey,
    pub output: FluidHatchKey,
    pub producer: bool,
    pub consumer: bool,
    pub fluid: FluidType,
}

pub fn handle_buffers<R: Registry>(fluid_hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, fluid_buffer: &mut SlotMap<FluidBufferKey, FluidBuffer>, settings: &Settings) {
    for fluid_buffer in fluid_buffer.values_mut() {
        let mut vel = VelocityType::default();
        equalize_hatch(fluid_hatches, fluid_buffer.input, &mut fluid_buffer.fluid, &mut vel, settings);
        equalize_hatch(fluid_hatches, fluid_buffer.output, &mut fluid_buffer.fluid, &mut vel, settings);

        if fluid_buffer.consumer {
            fluid_buffer.fluid = 0;
        }
        
        if fluid_buffer.producer {
            fluid_buffer.fluid = 1000;
        }
    }
}


pub fn handle_pipes<R: Registry>(pipes: &mut SlotMap<PipeKey, Pipe>, fluid_hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, tick: &u64, settings: &Settings) {
    for pipe in pipes.values_mut() {
        let Pipe {
            pipe_start,
            pipe_end,
            ref mut buffer,
            ref mut velocities,
        } = *pipe;

        // reset velocities
        velocities.fill(0);

        // don't do anything if belt points to invalid hatches
        if !fluid_hatches.contains_key(pipe_start) || !fluid_hatches.contains_key(pipe_end) {
            continue;
        }

        dbg!(fluid_hatches[pipe_start].fluid);
        dbg!(fluid_hatches[pipe_end].fluid);
        

        //  pipe start hatch <-> [buffer] <-> pipe end hatch

        equalize_hatch(fluid_hatches, pipe_start, buffer.first_mut().unwrap(), &mut 0i32, settings);
        equalize_hatch(fluid_hatches, pipe_end, buffer.last_mut().unwrap(), &mut 0i32, settings);
        
        // *tmp_modifs.first_mut().unwrap() = equalize_hatch_store_to_temp(fluid_hatches, pipe_end, buffer.first().unwrap(), settings);
        // *tmp_modifs.last_mut().unwrap() = equalize_hatch_store_to_temp(fluid_hatches, pipe_start, buffer.last().unwrap(), settings);
        
        // temporary buffer to accumulate the values into
        let mut tmp_modifs = vec![0i32; buffer.len()];
        for i in 0..(buffer.len()-1) {
            let [a, b] = [buffer[i], buffer[i+1]];
            tmp_modifs[i] += calculate_ideal_transfer_rate_from_a_to_b(&a, &b, settings, true);
            // tmp_modifs[i] = 1;
        }

        // *tmp_modifs.first_mut().unwrap() = equalize_hatch_store_to_temp(fluid_hatches, pipe_end, buffer.first().unwrap(), settings);
        // *tmp_modifs.last_mut().unwrap() = equalize_hatch_store_to_temp(fluid_hatches, pipe_start, buffer.last().unwrap(), settings);

        // apply the result of the buffer at the end
        for i in 0..(buffer.len()-1) {
            let from_a_to_b = tmp_modifs[i];

            // a
            buffer[i] -= from_a_to_b;
            
            // b
            buffer[i+1] += from_a_to_b;
            
            // buffer[i] += tmp_modifs[i];
            velocities[i] = tmp_modifs[i];
        }
    }
}

fn equalize_hatch(hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, key: FluidHatchKey, hatch: &mut FluidType, velocity: &mut VelocityType, settings: &Settings) {
    let dst = &mut hatches[key].fluid;
    let src = hatch;

    let t = calculate_ideal_transfer_rate_from_a_to_b(dst, src, settings, true);
    *velocity = t;

    *dst -= t;
    *src += t;
}


fn equalize_hatch_store_to_temp(hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, key: FluidHatchKey, hatch: &FluidType, settings: &Settings) -> FluidType {
    let dst = &mut hatches[key].fluid;
    let src = hatch;

    calculate_ideal_transfer_rate_from_a_to_b(dst, src, settings, true)
}


// where ideal transfer rate `t` is just `(max(a,0)-max(b,0)) / 2`
fn calculate_ideal_transfer_rate_from_a_to_b(a: &FluidType, b: &FluidType, settings: &Settings, limited: bool) -> FluidType {
    fn z(a: i32) -> i32 {
        a.max(0)
    }
    
    let transfer = (z(*a) - z(*b)) / 2;
    let bounds = 100;
    let transfer = transfer.clamp(-bounds, bounds);

    if limited {
        if transfer > 0 && *b > 300 {
            return 0;
        }

        if transfer < 0 && *a > 300 {
            return 0;
        }
    }

    transfer
}