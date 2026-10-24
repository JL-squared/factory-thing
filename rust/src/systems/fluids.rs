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
    pub volume: Vec<FluidType>,

    // analogous to "theoretical flow rate" between volume[i] and volume[i+1]
    // tries to take the value x from volume[i] and puts it in volume[i+1]
    pub flow_rate: Vec<FluidType>,

}
impl Pipe {
    pub(crate) fn from_hatches(output_hatch: FluidHatchKey, input_hatch: FluidHatchKey, buffer_length: usize) -> Pipe {
        Pipe {
            volume: vec![0; buffer_length],
            flow_rate: vec![0; buffer_length+1],
            pipe_start: output_hatch,
            pipe_end: input_hatch,
        }
    }
    
    pub(crate) fn stats(&self) -> PipeStats {
        
        let total_volume = self.volume.iter().sum::<i32>() as f32;
        // let total_velocity = self.velocities.iter().sum::<i32>() as f32;
        let total_velocity = 0f32;

        let normalized_volume = total_volume / self.volume.len() as f32;
        let normalized_velocity = total_velocity / self.volume.len() as f32;
        

        let normalized_volume_2 = total_volume / (self.volume.len() as f32-2f32);
        let normalized_velocity_2 = total_velocity / (self.volume.len() as f32-2f32);
        
        
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
    pub pressure: i32,
}

impl FluidHatch {
    pub fn empty() -> FluidHatch {
        FluidHatch {
            fluid: Default::default(),
            pressure: 0,
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


const MAX_TRANSFER_FROM_A_TO_B: i32 = 10000;
const MAX_VOLUME_PER_SLICE: i32 = 10000;
const PRODUCER_BUFFER_MAX_CAPACITY: i32 = 1000;


pub fn handle_buffers<R: Registry>(fluid_hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, fluid_buffer: &mut SlotMap<FluidBufferKey, FluidBuffer>, settings: &Settings) {
    for fluid_buffer in fluid_buffer.values_mut() {
        let mut vel = VelocityType::default();
        equalize_hatch(fluid_hatches, fluid_buffer.input, &mut fluid_buffer.fluid, &mut vel, settings);
        equalize_hatch(fluid_hatches, fluid_buffer.output, &mut fluid_buffer.fluid, &mut vel, settings);

        if fluid_buffer.consumer {
            dbg!(fluid_buffer.fluid);
            fluid_buffer.fluid = 0;
            fluid_hatches[fluid_buffer.input].pressure = 0;
            fluid_hatches[fluid_buffer.output].pressure = 0;
            
        }
        
        if fluid_buffer.producer {
            fluid_buffer.fluid = PRODUCER_BUFFER_MAX_CAPACITY;
            fluid_hatches[fluid_buffer.input].pressure = 100;
            fluid_hatches[fluid_buffer.output].pressure = 100;
        }
    }
}

// TODO: is there a way to create an abstracted "fluid simulation solver" trait that hides the internals of the fluid sim system?
pub fn handle_pipes<R: Registry>(pipes: &mut SlotMap<PipeKey, Pipe>, fluid_hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, tick: &u64, settings: &Settings) {
    for pipe in pipes.values_mut() {
        let Pipe {
            pipe_start,
            pipe_end,
            volume: ref mut buffer,
            flow_rate: ref mut pressure,
        } = *pipe;

        // don't do anything if belt points to invalid hatches
        if !fluid_hatches.contains_key(pipe_start) || !fluid_hatches.contains_key(pipe_end) {
            continue;
        }

        // TODO: find better implementation, this one (which is better than the wave propagation one) is still iffy

        // let hatch_pressure_differential = fluid_hatches[pipe_start].pressure - fluid_hatches[pipe_end].pressure;
        let mut hatch_pressure_differential = fluid_hatches[pipe_start].fluid - fluid_hatches[pipe_end].fluid;
        hatch_pressure_differential = hatch_pressure_differential.clamp(-100, 100); 

        // basically like throughput?
        let max_pressure = 100;

        if hatch_pressure_differential > 0 {
            // seed is pipe start
            pressure[0] = hatch_pressure_differential;

            // pressure loss per segment
            for i in 0..(pressure.len()-1) {
                pressure[i+1] = (pressure[i]-1).max(0);
            }
        } else {
            // seed is pipe end
            *pressure.last_mut().unwrap() = hatch_pressure_differential;
        
            for i in (0..(pressure.len()-1)).rev() {
                pressure[i] = (pressure[i+1]+1).min(0);
            }
        }

        for pressure in pressure.iter_mut() {
            *pressure = (*pressure).clamp(-max_pressure, max_pressure);
        }

        /*

        // clone because we do not want modifications in the loop to modify the next iteration
        let tmp_pressure = pressure.clone();

        for i in 0..pressure.len() {
            let current = tmp_pressure[i];

            if i>=1 {
                let left = tmp_pressure[i-1];
                
                if current > left && current > 0 && pressure[i] > 0{
                    let t = (left-current).max(30) / 2;
                    pressure[i] -= t;
                    pressure[i-1] += t;
                }
            }

            if i<(pressure.len()-1) {
                let right = tmp_pressure[i+1];

                if current > right && current > 0 && pressure[i] > 0 {
                    let t = (right-current).max(30) / 2;
                    pressure[i] -= t;
                    pressure[i+1] += t;
                }
            }
        }
        */
        
        /*
        for i in 0..(buffer.len()-1) {
            pressure[i] = (hatch_pressure_differential - i as i32).max;
        }
        */

        equalize_hatch(fluid_hatches, pipe_start, buffer.first_mut().unwrap(), &mut 0, settings);
        equalize_hatch(fluid_hatches, pipe_end, buffer.last_mut().unwrap(), &mut 0, settings);
        
        let mut tmp = vec![0; buffer.len()];

        for i in 0..(buffer.len()-1) {
            if buffer[i] > 0 {
                let t = pressure[i].min(buffer[i]);
                tmp[i] -= t;
                tmp[i+1] += t;
            }
        }


        for i in 0..(buffer.len()) {
            buffer[i] += tmp[i];
        }
    }
}

fn equalize_hatch(hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, key: FluidHatchKey, hatch: &mut FluidType, velocity: &mut VelocityType, settings: &Settings) {
    let dst = &mut hatches[key].fluid;
    let src = hatch;

    // let t = 100;
    let t = calculate_ideal_transfer_rate_from_a_to_b(dst, src, settings, true);
    // *velocity = t;

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
    let bounds = MAX_TRANSFER_FROM_A_TO_B;
    let transfer = transfer.clamp(-bounds, bounds);

    if limited {
        if transfer > 0 && *b > MAX_VOLUME_PER_SLICE {
            return 0;
        }

        if transfer < 0 && *a > MAX_VOLUME_PER_SLICE {
            return 0;
        }
    }

    transfer
}