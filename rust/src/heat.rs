use std::{collections::{HashMap, HashSet}, ops::Div};

use vek::num_traits::Euclid;

const LOGICAL_CHUNK_SIZE: usize = 32;
const CHUNK_VOLUME: usize = LOGICAL_CHUNK_SIZE*LOGICAL_CHUNK_SIZE*LOGICAL_CHUNK_SIZE;
const PHYSICAL_CHUNK_SIZE: usize = 16;

pub fn try_offset_to_index(offset: vek::Vec3<i32>, size: usize) -> Option<usize> {
    if offset.cmpge(&vek::Vec3::broadcast(0)).reduce_and() && offset.cmplt(&vek::Vec3::broadcast(size as i32)).reduce_and() {
        let offset = offset.as_::<usize>();
        Some(offset.x + offset.y * size + offset.z * size * size)
    } else {
        None
    }
}

pub fn offset_to_index(offset: vek::Vec3<usize>, size: usize) -> usize {
    assert!(offset.cmpge(&vek::Vec3::broadcast(0)).reduce_and());
    assert!(offset.cmplt(&vek::Vec3::broadcast(size)).reduce_and());
    
    offset.x + offset.y * size + offset.z * size * size
}

pub fn index_to_offset(index: usize, size: usize) -> vek::Vec3<usize> {
    assert!(index < (size*size*size));
    
    let x: usize = index % size;
    let y = (index / size) % size;
    let z = index / (size*size);
    vek::Vec3::new(x,y,z)
}

const NEIGBOUR_OFFSETS_INCLUDING_SELF: &'static [vek::Vec3<i32>] = &[
    vek::Vec3::<i32>::new(0, 0, 0),
    
    vek::Vec3::<i32>::new(-1, 0, 0),
    vek::Vec3::<i32>::new(0, -1, 0),
    vek::Vec3::<i32>::new(0, 0, -1),

    vek::Vec3::<i32>::new(1, 0, 0),
    vek::Vec3::<i32>::new(0, 1, 0),
    vek::Vec3::<i32>::new(0, 0, 1),
];

// each heat chunk is 16m wide with 0.5m voxels
#[derive(Clone)]
struct HeatChunk {
    position: vek::Vec3<i32>,
    heat: Box<[f32; CHUNK_VOLUME]>,
}

#[derive(Default)]
struct Heat {
    sources: Vec<vek::Vec3<i32>>,
    sensors: Vec<(vek::Vec3<i32>, f32)>, 
    chunks: HashMap<vek::Vec3<i32>, HeatChunk>,
}

impl Heat {
    fn tick(&mut self) {
        let previous = self.chunks.clone();

        for (chunk_position, heat_chunk) in self.chunks.iter_mut() {
            for (i, value) in heat_chunk.heat.iter_mut().enumerate() {
                let chunk_local_position = index_to_offset(i, LOGICAL_CHUNK_SIZE).as_::<i32>();

                let mut average = 0f32;

                // read from neighbours using previous
                for offset in NEIGBOUR_OFFSETS_INCLUDING_SELF {
                    let offsetted = offset + chunk_local_position;

                    if let Some(local) = try_offset_to_index(offsetted, LOGICAL_CHUNK_SIZE) {
                        average += previous[chunk_position].heat[local];
                    } else {
                        let world_position = *chunk_position * LOGICAL_CHUNK_SIZE as i32 + offsetted;

                        // TODO: remove floating point for floor div
                        let new_chunk_position = (world_position.as_::<f32>() / LOGICAL_CHUNK_SIZE as f32).floor().as_::<i32>(); 

                        if let Some(other_chunk) = previous.get(&new_chunk_position) {
                            let new_chunk_local_position = world_position - new_chunk_position * LOGICAL_CHUNK_SIZE as i32;
                            let local = offset_to_index(new_chunk_local_position.as_::<usize>(), LOGICAL_CHUNK_SIZE); 
                            average += other_chunk.heat[local];
                        }
                    }
                }

                // calculate average
                average /= NEIGBOUR_OFFSETS_INCLUDING_SELF.len() as f32;

                // update self
                *value = average;
            }
        }

        for (voxel_position, sensor_value) in self.sensors.iter_mut() {
            
            // TODO: remove floating point for floor div
            let new_chunk_position = (voxel_position.as_::<f32>() / LOGICAL_CHUNK_SIZE as f32).floor().as_::<i32>(); 
            if let Some(other_chunk) = self.chunks.get(&new_chunk_position) {
                let new_chunk_local_position = *voxel_position - new_chunk_position * LOGICAL_CHUNK_SIZE as i32;
                let local = offset_to_index(new_chunk_local_position.as_::<usize>(), LOGICAL_CHUNK_SIZE); 
                *sensor_value = other_chunk.heat[local];
            }
        }

        for voxel_position in self.sources.iter() {
            
            // TODO: remove floating point for floor div
            let new_chunk_position = (voxel_position.as_::<f32>() / LOGICAL_CHUNK_SIZE as f32).floor().as_::<i32>(); 
            if let Some(other_chunk) = self.chunks.get_mut(&new_chunk_position) {
                let new_chunk_local_position = *voxel_position - new_chunk_position * LOGICAL_CHUNK_SIZE as i32;
                let local = offset_to_index(new_chunk_local_position.as_::<usize>(), LOGICAL_CHUNK_SIZE); 
                other_chunk.heat[local] = 100f32;
            }
        }
    }

    fn add_chunk(&mut self, chunk_position: vek::Vec3<i32>) {
        self.chunks.insert(chunk_position, HeatChunk {
            position: vek::Vec3::zero(),
            heat: Box::new([0f32; _]),
        });
    }

    fn add_source(&mut self, voxel_position: vek::Vec3<i32>) {
        self.sources.push(voxel_position);
    }

    fn add_sensor(&mut self, voxel_position: vek::Vec3<i32>) {
        self.sensors.push((voxel_position, 0f32));
    }

    fn get_sensors(&self) -> Vec<f32> {
        dbg!(&self.sensors);
        self.sensors.iter().map(|(a, b)| *b).collect::<Vec<_>>()
    }

    fn get_overall(&self) -> f32 {
        self.chunks.iter().map(|(_, chunk)| chunk.heat.iter().copied().sum::<f32>()).sum()
    }
}

// #[test]
// fn test() {
//     let mut heat = Heat::default();
//     heat.add_chunk(vek::Vec3::zero());
//     heat.add_sensor(vek::Vec3::zero());
//     heat.tick();
//     assert!(heat.get_sensors()[0] == 0f32);
// }


// #[test]
// fn test2() {
//     let mut heat = Heat::default();
//     heat.add_chunk(vek::Vec3::zero());
//     heat.add_sensor(vek::Vec3::new(0, 8, 0));
//     heat.add_source(vek::Vec3::zero());
    
//     heat.tick();
//     assert!(heat.get_sensors()[0] == 0f32);

//     for _ in 0..50 {
//         heat.tick();
//         heat.get_sensors();
//         //dbg!(heat.get_overall());
//     }

//     assert!(heat.get_sensors()[0] > 0f32);

// }