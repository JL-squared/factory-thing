use crate::handle::*;
use crate::common::*;
use crate::registry::*;
use slotmap::*;


// units are milliliters I suppose
pub type FluidType = i32;

#[derive(Debug)]
pub struct Pipe {
    pub pipe_start: FluidHatchKey,
    pub pipe_end: FluidHatchKey,
    pub buffer: Vec<FluidType>,
}
impl Pipe {
    pub(crate) fn from_hatches(output_hatch: FluidHatchKey, input_hatch: FluidHatchKey, buffer_length: usize) -> Pipe {
        Pipe {
            buffer: vec![0; buffer_length],
            pipe_start: output_hatch,
            pipe_end: input_hatch,
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

pub fn handle_buffers<R: Registry>(fluid_hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, fluid_buffer: &mut SlotMap<FluidBufferKey, FluidBuffer>, settings: &mut Settings) {
    for fluid_buffer in fluid_buffer.values_mut() {
        equalize_hatch(fluid_hatches, fluid_buffer.input, &mut fluid_buffer.fluid);
        equalize_hatch(fluid_hatches, fluid_buffer.output, &mut fluid_buffer.fluid);

        if fluid_buffer.consumer {
            fluid_buffer.fluid = 0;
        }
        
        if fluid_buffer.producer {
            fluid_buffer.fluid = 1000;
        }
    }
}


pub fn handle_pipes<R: Registry>(pipes: &mut SlotMap<PipeKey, Pipe>, fluid_hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, tick: &u64, settings: &mut Settings) {
    for pipe in pipes.values_mut() {
        let Pipe {
            pipe_start,
            pipe_end,
            ref mut buffer,
        } = *pipe;

        // don't do anything if belt points to invalid hatches
        if !fluid_hatches.contains_key(pipe_start) || !fluid_hatches.contains_key(pipe_end) {
            continue;
        }

        equalize_hatch(fluid_hatches, pipe_end, buffer.last_mut().unwrap());
        equalize_hatch(fluid_hatches, pipe_start, buffer.first_mut().unwrap());
        
        for i in 0..(buffer.len()-1) {
            let [a, b] = buffer.get_disjoint_mut([i, i+1]).unwrap();
            equalize_inner(a, b);
        }
    }
}

fn equalize_hatch(hatches: &mut SlotMap<FluidHatchKey, FluidHatch>, key: FluidHatchKey, hatch: &mut FluidType) {
    let dst = &mut hatches[key].fluid;
    let src = hatch;

    equalize_inner(dst, src);
}

fn equalize_inner(a: &mut FluidType, b: &mut FluidType) {
    fn z(a: i32) -> i32 {
        a.max(0)
    }
    
    let transfer = z(*a) - z(*b);
    

    let bounds = 20;
    let transfer = transfer.clamp(-bounds, bounds);
    let transfer = transfer;

    *a -= transfer;
    *b += transfer;
}