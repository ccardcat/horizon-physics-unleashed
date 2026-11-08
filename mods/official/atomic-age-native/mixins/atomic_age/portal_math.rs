//! Original Atomic Age portal geometry. Coordinates are in blocks, Y is up.
use glam::{DVec3, Mat4, Vec3, Vec4};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Portal {
    pub center: [f64; 3],
    pub normal: [f64; 3],
    pub orange: bool,
}
impl Portal {
    pub fn n(&self) -> DVec3 {
        DVec3::from_array(self.normal)
    }
    pub fn c(&self) -> DVec3 {
        DVec3::from_array(self.center)
    }
    pub fn right(&self) -> DVec3 {
        DVec3::Y.cross(self.n())
    }
    pub fn rotate_to(&self, exit: &Self, vector: DVec3) -> DVec3 {
        -exit.right() * vector.dot(self.right()) + DVec3::Y * vector.y
            - exit.n() * vector.dot(self.n())
    }
    pub fn point_to(&self, exit: &Self, point: DVec3) -> DVec3 {
        exit.c() + self.rotate_to(exit, point - self.c())
    }
    /// The swept body centre must cross from the front inside the usable opening.
    pub fn crossing(&self, previous: DVec3, current: DVec3) -> Option<DVec3> {
        let a = (previous - self.c()).dot(self.n());
        let b = (current - self.c()).dot(self.n());
        if a <= 0.0 || b > 0.0 || a - b < 1e-9 {
            return None;
        }
        let hit = previous.lerp(current, a / (a - b));
        let delta = hit - self.c();
        (delta.dot(self.right()).abs() < 0.40 && delta.y.abs() < 0.45).then_some(hit)
    }
}

/// Reversed-Z oblique near plane: w-z >= 0 becomes the destination's front half-space.
pub fn oblique(projection: Mat4, plane: Vec4) -> Mat4 {
    let q = projection.inverse() * Vec4::new(plane.x.signum(), plane.y.signum(), 0.0, 1.0);
    let denom = plane.dot(q);
    if !denom.is_finite() || denom <= 1e-6 {
        return projection;
    }
    let mut rows = projection.transpose().to_cols_array_2d();
    rows[2] = (Vec4::from_array(rows[3]) - plane / denom).to_array();
    Mat4::from_cols_array_2d(&rows).transpose()
}

pub fn direction(yaw: f32, pitch: f32) -> DVec3 {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    DVec3::new(-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos())
}
pub fn angles(direction: DVec3) -> [f32; 2] {
    let d = direction.normalize_or_zero();
    [
        (-d.x).atan2(d.z).to_degrees() as f32,
        (-d.y).asin().to_degrees() as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn portal(center: [f64; 3], normal: [f64; 3]) -> Portal {
        Portal {
            center,
            normal,
            orange: false,
        }
    }
    #[test]
    fn passage_rotates_position_look_and_momentum_without_changing_speed() {
        let a = portal([0., 10., 0.], [0., 0., 1.]);
        let b = portal([20., 10., 30.], [1., 0., 0.]);
        let p = DVec3::new(0.25, 10.2, 2.);
        assert!((a.point_to(&b, p) - DVec3::new(18., 10.2, 30.25)).length() < 1e-9);
        let v = DVec3::new(1., -2., -3.);
        let rotated = a.rotate_to(&b, v);
        assert!((rotated - DVec3::new(3., -2., 1.)).length() < 1e-9);
        assert!((rotated.length() - v.length()).abs() < 1e-9);
        assert!((b.point_to(&a, a.point_to(&b, p)) - p).length() < 1e-9);
    }
    #[test]
    fn swept_crossing_catches_fast_motion_and_rejects_the_frame_and_back_side() {
        let p = portal([0., 1., 0.], [0., 0., 1.]);
        assert!(
            p.crossing(DVec3::new(0., 1., 4.), DVec3::new(0., 1., -4.))
                .is_some()
        );
        assert!(
            p.crossing(DVec3::new(1., 1., 1.), DVec3::new(1., 1., -1.))
                .is_none()
        );
        assert!(
            p.crossing(DVec3::new(0., 1., -1.), DVec3::new(0., 1., 1.))
                .is_none()
        );
    }
    #[test]
    fn oblique_depth_rejects_geometry_behind_the_exit() {
        let projection = Mat4::perspective_infinite_reverse_rh(70f32.to_radians(), 1.6, 0.05);
        let clipped = oblique(projection, Vec4::new(0., 0., -1., -2.));
        let near = clipped * Vec4::new(0., 0., -2., 1.);
        let behind = clipped * Vec4::new(0., 0., -1., 1.);
        let front = clipped * Vec4::new(0., 0., -8., 1.);
        assert!((near.w - near.z).abs() < 1e-5);
        assert!(behind.w - behind.z < 0.);
        assert!(front.w - front.z > 0. && front.z >= 0.);
        assert_eq!(clipped.x_axis.x, projection.x_axis.x);
    }
    #[test]
    fn look_direction_round_trips() {
        for (yaw, pitch) in [(0., 0.), (90., 30.), (-135., -60.)] {
            let [y, p] = angles(direction(yaw, pitch));
            assert!((y - yaw).abs() < 1e-4 && (p - pitch).abs() < 1e-4);
        }
    }
}
