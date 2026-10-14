use crate::scene::{Node, NodeId, Scene};
use glam::{Mat4, Vec2, Vec3, Vec4};
use std::fmt;

// Right-handed, Y-up; yaw zero looks from +Z. NDC uses X/Y [-1, 1], Y up, Z [0, 1].
// UI pixels map to NDC as (2*x/width - 1, 1 - 2*y/height).
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    fov_y: f32,
    near: f32,
    far: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidCameraInput;

impl fmt::Display for InvalidCameraInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("camera inputs must be finite and within valid projection ranges")
    }
}
impl std::error::Error for InvalidCameraInput {}

impl Default for Camera {
    fn default() -> Self {
        Self::new(
            Vec3::ZERO,
            0.6,
            0.4,
            8.0,
            45_f32.to_radians(),
            0.01,
            10_000.0,
        )
        .expect("default camera is valid")
    }
}

impl Camera {
    pub fn new(
        target: Vec3,
        yaw: f32,
        pitch: f32,
        distance: f32,
        fov_y: f32,
        near: f32,
        far: f32,
    ) -> Result<Self, InvalidCameraInput> {
        let camera = Self {
            target,
            yaw,
            pitch,
            distance,
            fov_y,
            near,
            far,
        };
        camera.validate()?;
        Ok(camera)
    }

    fn validate(&self) -> Result<(), InvalidCameraInput> {
        if !self.target.is_finite()
            || !self.yaw.is_finite()
            || !self.pitch.is_finite()
            || self.pitch.abs() >= std::f32::consts::FRAC_PI_2
            || !self.distance.is_finite()
            || self.distance <= 0.0
            || !self.fov_y.is_finite()
            || self.fov_y <= 0.0
            || self.fov_y >= std::f32::consts::PI
            || !self.near.is_finite()
            || self.near <= 0.0
            || !self.far.is_finite()
            || self.far <= self.near
            || !self.eye().is_finite()
            || !self.view_matrix().is_finite()
        {
            return Err(InvalidCameraInput);
        }
        Ok(())
    }

    pub fn target(&self) -> Vec3 {
        self.target
    }
    pub fn distance(&self) -> f32 {
        self.distance
    }
    pub fn eye(&self) -> Vec3 {
        self.target
            + self.distance
                * Vec3::new(
                    self.yaw.sin() * self.pitch.cos(),
                    self.pitch.sin(),
                    self.yaw.cos() * self.pitch.cos(),
                )
    }
    pub fn view_matrix(&self) -> Mat4 {
        let back = (self.eye() - self.target).normalize();
        let right = Vec3::Y.cross(back).normalize();
        let up = back.cross(right);
        let eye = self.eye();
        Mat4::from_rows(
            right.extend(-right.dot(eye)),
            up.extend(-up.dot(eye)),
            back.extend(-back.dot(eye)),
            Vec4::W,
        )
    }
    pub fn projection_matrix(&self, aspect: f32) -> Result<Mat4, InvalidCameraInput> {
        if !aspect.is_finite() || aspect <= 0.0 {
            return Err(InvalidCameraInput);
        }
        let y = 1.0 / (self.fov_y * 0.5).tan();
        let z = self.far / (self.near - self.far);
        let matrix = Mat4::from_cols(
            Vec4::new(y / aspect, 0.0, 0.0, 0.0),
            Vec4::new(0.0, y, 0.0, 0.0),
            Vec4::new(0.0, 0.0, z, -1.0),
            Vec4::new(0.0, 0.0, z * self.near, 0.0),
        );
        if !matrix.is_finite() {
            return Err(InvalidCameraInput);
        }
        Ok(matrix)
    }

    pub fn orbit(&mut self, radians: Vec2) -> Result<(), InvalidCameraInput> {
        if !radians.is_finite() {
            return Err(InvalidCameraInput);
        }
        let mut next = self.clone();
        next.yaw = (next.yaw + radians.x).rem_euclid(std::f32::consts::TAU);
        let limit = std::f32::consts::FRAC_PI_2 - 0.001;
        next.pitch = (next.pitch + radians.y).clamp(-limit, limit);
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn pan(&mut self, world_units: Vec2) -> Result<(), InvalidCameraInput> {
        if !world_units.is_finite() {
            return Err(InvalidCameraInput);
        }
        let mut next = self.clone();
        let inverse_view = self.view_matrix().inverse();
        next.target += inverse_view.transform_vector3(Vec3::new(world_units.x, world_units.y, 0.0));
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn dolly(&mut self, factor: f32) -> Result<(), InvalidCameraInput> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(InvalidCameraInput);
        }
        let mut next = self.clone();
        next.distance *= factor;
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn frame(&mut self, node: &Node, aspect: f32) -> Result<(), InvalidCameraInput> {
        self.projection_matrix(aspect)?;
        let vertices = node.world_vertices();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for vertex in &vertices {
            min = min.min(*vertex);
            max = max.max(*vertex);
        }
        let target = min * 0.5 + max * 0.5;
        let radius = vertices
            .iter()
            .map(|v| v.distance(target))
            .fold(0.0_f32, f32::max);
        let half_angle = (self.fov_y * 0.5).min(((self.fov_y * 0.5).tan() * aspect).atan());
        let mut next = self.clone();
        next.target = target;
        next.distance = (radius / half_angle.sin())
            .max(self.near + radius)
            .max(self.near * 2.0)
            * 1.05;
        next.far = self.far.max(next.distance + radius + self.near);
        next.validate()?;
        *self = next;
        Ok(())
    }

    // Points outside depth clipping return None; lateral offscreen points retain their NDC coordinates.
    pub fn project(&self, world: Vec3, aspect: f32) -> Result<Option<Vec3>, InvalidCameraInput> {
        if !world.is_finite() {
            return Err(InvalidCameraInput);
        }
        let clip = self.projection_matrix(aspect)? * self.view_matrix() * world.extend(1.0);
        if !clip.is_finite() || clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w {
            return Ok(None);
        }
        let ndc = clip.truncate() / clip.w;
        if !ndc.is_finite() {
            return Err(InvalidCameraInput);
        }
        Ok(Some(ndc))
    }

    pub fn ray(&self, ndc: Vec2, aspect: f32) -> Result<Ray, InvalidCameraInput> {
        let projection = self.projection_matrix(aspect)?;
        if !ndc.is_finite() || ndc.abs().max_element() > 1.0 {
            return Err(InvalidCameraInput);
        }
        let view_direction = Vec3::new(
            ndc.x / projection.x_axis.x,
            ndc.y / projection.y_axis.y,
            -1.0,
        )
        .normalize();
        let direction = self
            .view_matrix()
            .inverse()
            .transform_vector3(view_direction)
            .normalize();
        if !direction.is_finite() {
            return Err(InvalidCameraInput);
        }
        Ok(Ray {
            origin: self.eye(),
            direction,
        })
    }

    pub fn pick(
        &self,
        scene: &Scene,
        ndc: Vec2,
        aspect: f32,
    ) -> Result<Option<NodeId>, InvalidCameraInput> {
        let ray = self.ray(ndc, aspect)?;
        let view = self.view_matrix();
        let mut closest = f32::INFINITY;
        let mut selected = None;
        for node in scene.nodes() {
            let vertices = node.world_vertices();
            for face in node.mesh().faces() {
                for triangle in [[face[0], face[1], face[2]], [face[0], face[2], face[3]]] {
                    let [a, b, c] = triangle.map(|i| vertices[i]);
                    if let Some(distance) = intersect(ray, a, b, c) {
                        let depth = -view
                            .transform_point3(ray.origin + distance * ray.direction)
                            .z;
                        if depth >= self.near && depth <= self.far && distance < closest {
                            closest = distance;
                            selected = Some(node.id());
                        }
                    }
                }
            }
        }
        Ok(selected)
    }
}

fn intersect(ray: Ray, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    // Double precision avoids a fixed world-unit epsilon rejecting very small valid triangles.
    let edge1 = b.as_dvec3() - a.as_dvec3();
    let edge2 = c.as_dvec3() - a.as_dvec3();
    let direction = ray.direction.as_dvec3();
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant == 0.0 {
        return None;
    }
    let t = ray.origin.as_dvec3() - a.as_dvec3();
    let u = t.dot(p) / determinant;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = t.cross(edge1);
    let v = direction.dot(q) / determinant;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = (edge2.dot(q) / determinant) as f32;
    (distance.is_finite() && distance >= 0.0).then_some(distance)
}
