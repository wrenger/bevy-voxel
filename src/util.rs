use std::f32::consts::PI;

use bevy::math::{IVec3, Quat, Vec3};
use serde::Deserialize;

#[repr(usize)]
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    #[serde(rename = "-x")]
    NegX,
    #[serde(rename = "-y")]
    NegY,
    #[serde(rename = "-z")]
    NegZ,
    #[serde(rename = "+x")]
    PosX,
    #[serde(rename = "+y")]
    PosY,
    #[serde(rename = "+z")]
    PosZ,
}

impl Direction {
    pub fn all() -> [Direction; 6] {
        [
            Self::NegX,
            Self::NegY,
            Self::NegZ,
            Self::PosX,
            Self::PosY,
            Self::PosZ,
        ]
    }
}

impl From<Direction> for Vec3 {
    fn from(d: Direction) -> Self {
        match d {
            Direction::NegX => -Vec3::X,
            Direction::NegY => -Vec3::Y,
            Direction::NegZ => -Vec3::Z,
            Direction::PosX => Vec3::X,
            Direction::PosY => Vec3::Y,
            Direction::PosZ => Vec3::Z,
        }
    }
}
impl From<Direction> for IVec3 {
    fn from(d: Direction) -> Self {
        match d {
            Direction::NegX => -IVec3::X,
            Direction::NegY => -IVec3::Y,
            Direction::NegZ => -IVec3::Z,
            Direction::PosX => IVec3::X,
            Direction::PosY => IVec3::Y,
            Direction::PosZ => IVec3::Z,
        }
    }
}

impl From<Direction> for Quat {
    fn from(d: Direction) -> Self {
        match d {
            Direction::NegX => Quat::from_rotation_y(PI / 2.0),
            Direction::NegY => Quat::from_rotation_x(-PI / 2.0),
            Direction::NegZ => Quat::IDENTITY,
            Direction::PosX => Quat::from_rotation_y(-PI / 2.0),
            Direction::PosY => Quat::from_rotation_x(PI / 2.0),
            Direction::PosZ => Quat::from_rotation_y(PI),
        }
    }
}
