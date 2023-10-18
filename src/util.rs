use std::f32::consts::PI;
use std::ops::Range;

use bevy::math::{IVec3, Quat, UVec3, Vec3};
use serde::Deserialize;

/// 3d world direction.
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
    pub fn all() -> [Self; 6] {
        [
            Self::NegX,
            Self::NegY,
            Self::NegZ,
            Self::PosX,
            Self::PosY,
            Self::PosZ,
        ]
    }

    pub fn from_ivec3(v: IVec3) -> Option<Self> {
        match v {
            _ if v.x < 0 => Some(Self::NegX),
            _ if v.y < 0 => Some(Self::NegY),
            _ if v.z < 0 => Some(Self::NegZ),
            _ if v.x > 0 => Some(Self::PosX),
            _ if v.y > 0 => Some(Self::PosY),
            _ if v.z > 0 => Some(Self::PosZ),
            _ => None,
        }
    }

    pub fn ortho_vec3(self) -> (Vec3, Vec3) {
        let rot = Quat::from(self);
        let x = Vec3::X;
        let y = Vec3::Y;
        (rot * x, rot * y)
    }

    pub fn inverse(self) -> Self {
        match self {
            Self::NegX => Self::PosX,
            Self::NegY => Self::PosY,
            Self::NegZ => Self::PosZ,
            Self::PosX => Self::NegX,
            Self::PosY => Self::NegY,
            Self::PosZ => Self::NegZ,
        }
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

pub trait RangeExt<T> {
    /// Linear interpolates `t` between `start` and `end`.
    fn lerp(&self, t: T) -> T;
    /// Determines where a value lies between `start` and `end`.
    fn lerp_inv(&self, val: T) -> T;
}

impl RangeExt<f32> for Range<f32> {
    fn lerp(&self, t: f32) -> f32 {
        self.start + (self.end - self.start) * t
    }
    fn lerp_inv(&self, val: f32) -> f32 {
        ((val - self.start) / (self.end - self.start)).clamp(0.0, 1.0)
    }
}
impl RangeExt<f64> for Range<f64> {
    fn lerp(&self, t: f64) -> f64 {
        self.start + (self.end - self.start) * t
    }
    fn lerp_inv(&self, val: f64) -> f64 {
        ((val - self.start) / (self.end - self.start)).clamp(0.0, 1.0)
    }
}

/// Iterates over all coordinates in the cube betweed the `from` (inclusive) and `to` (exclusive) points.
///
/// Iteration order: XZY (out -> in)
pub fn for_uvec3(from: UVec3, to: UVec3, mut f: impl FnMut(UVec3)) {
    for x in from.x..to.x {
        for z in from.z..to.z {
            for y in from.y..to.y {
                f(UVec3::new(x, y, z))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Direction;
    use bevy::prelude::*;

    #[test]
    fn rotation() {
        let back = -Vec3::Z;
        for d in Direction::all() {
            assert_eq!((Quat::from(d) * back).round(), Vec3::from(d));
        }
        let forward = Vec3::Z;
        for d in Direction::all() {
            assert_eq!((Quat::from(d) * forward).round(), Vec3::from(d.inverse()));
        }

        let center = (Vec3::new(32.0, 32.0, 32.0) - 1.0) / 2.0;
        let pos = Vec3::new(4.0, 2.0, 0.0);
        let p = Quat::from(Direction::NegZ) * (pos - center) + center;
        assert_eq!(p.round(), pos);
        let p = Quat::from(Direction::NegX) * (pos - center) + center;
        assert_eq!(p.round(), Vec3::new(0.0, 2.0, 31.0 - 4.0));
        let p = Quat::from(Direction::PosX) * (pos - center) + center;
        assert_eq!(p.round(), Vec3::new(31.0, 2.0, 4.0));
        let p = Quat::from(Direction::PosZ) * (pos - center) + center;
        assert_eq!(p.round(), Vec3::new(31.0 - 4.0, 2.0, 31.0));
        let p = Quat::from(Direction::NegY) * (pos - center) + center;
        assert_eq!(p.round(), Vec3::new(4.0, 0.0, 31.0 - 2.0));
        let p = Quat::from(Direction::PosY) * (pos - center) + center;
        assert_eq!(p.round(), Vec3::new(4.0, 31.0, 2.0));
    }
}
