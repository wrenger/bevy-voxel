use bevy::math::IVec3;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;

use crate::chunk::Chunk;

struct World {
    chunks: HashMap<IVec3, Chunk>,
}


// TODO: Load / unload chunks
