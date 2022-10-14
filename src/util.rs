use std::f32::consts::PI;
use std::ops::Range;

use bevy::math::{IVec3, Quat, Vec3};
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

pub trait RangeExt {
    /// Linear interpolates `t` between `start` and `end`.
    fn lerp(&self, t: f32) -> f32;
    /// Determines where a value lies between `start` and `end`.
    fn lerp_inv(&self, val: f32) -> f32;
}

impl RangeExt for Range<f32> {
    fn lerp(&self, t: f32) -> f32 {
        self.start + (self.end - self.start) * t
    }
    fn lerp_inv(&self, val: f32) -> f32 {
        ((val - self.start) / (self.end - self.start)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Range3 {
    off: IVec3,
    max: IVec3,
    i: usize,
}

impl Range3 {
    pub fn new(from: IVec3, to: IVec3) -> Range3 {
        let max = (to - from).abs().max(IVec3::ONE);
        let off = (to - from).min(from - to);
        Self { off, max, i: 0 }
    }

    fn ivec3(&self) -> Option<IVec3> {
        let p = IVec3::new(
            self.i as i32 % self.max.y,
            (self.i as i32 / self.max.y) % self.max.x,
            (self.i as i32 / self.max.y) / self.max.x,
        );
        if p.z < self.max.z {
            Some(p + self.off)
        } else {
            None
        }
    }
}

impl Iterator for Range3 {
    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;
        self.ivec3()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.max.x * self.max.y * self.max.z;
        (size as usize, Some(size as usize))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        let size = self.max.x * self.max.y * self.max.z;
        size as usize
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if let Some(c) = self.len().checked_sub(1) {
            self.nth(c)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.i += n;
        self.ivec3()
    }
}

impl DoubleEndedIterator for Range3 {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.i.checked_sub(1) {
            self.i = i;
            self.ivec3()
        } else {
            None
        }
    }
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        if let Some(i) = self.i.checked_sub(n) {
            self.i = i;
            self.ivec3()
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Range3 {
    fn len(&self) -> usize {
        let size = self.max.x * self.max.y * self.max.z;
        size as usize
    }
}

#[cfg(test)]
mod test {
    use super::{Direction, Range3};
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

    #[test]
    fn range_3() {
        let mut range = Range3::new(IVec3::ZERO, IVec3::ONE);
        assert_eq!(range.count(), 1);
        assert_eq!(range.len(), 1);
        assert_eq!(range.next(), Some(IVec3::ZERO));
    }
}
