// Original Atomic Age portal geometry. Coordinates are in blocks, Y is up.
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

use std::{
    cell::{Cell, RefCell},
    time::Instant,
};
use wgpu::util::DeviceExt;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scene {
    pub pairs: Vec<[Option<Portal>; 2]>,
    pub blasts: Vec<Blast>,
    pub beams: Vec<Beam>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blast {
    pub center: [f64; 3],
    pub age: f32,
    pub kind: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Beam {
    pub from: [f64; 3],
    pub to: [f64; 3],
}
thread_local! {
    static SCENE: RefCell<(Scene, Instant)> = RefCell::new((Scene::default(), Instant::now()));
    static PASS: Cell<bool> = const { Cell::new(false) };
    static CLIP: Cell<Option<Vec4>> = const { Cell::new(None) };
    static GPU: RefCell<Option<EffectGpu>> = const { RefCell::new(None) };
    static TARGETS: RefCell<Vec<PortalTarget>> = const { RefCell::new(Vec::new()) };
    static READY: RefCell<Vec<(Portal, wgpu::TextureView)>> = const { RefCell::new(Vec::new()) };
    static SAVED_CONTROLS: Cell<Option<(bool,bool)>> = const {Cell::new(None)};
}
pub fn receive(scene: Scene) {
    SCENE.with(|s| *s.borrow_mut() = (scene, Instant::now()));
}
pub fn clear() {
    receive(Scene::default());
    READY.with(|r| r.borrow_mut().clear());
    TARGETS.with(|t| t.borrow_mut().clear());
    GPU.with(|g| *g.borrow_mut() = None);
}
pub fn has_portals() -> bool {
    SCENE.with(|s| {
        s.borrow()
            .0
            .pairs
            .iter()
            .any(|p| p[0].is_some() && p[1].is_some())
    })
}
fn snapshot() -> (Scene, f32) {
    SCENE.with(|s| {
        let s = s.borrow();
        (s.0.clone(), s.1.elapsed().as_secs_f32().min(1.0) * 20.)
    })
}
pub fn shake_at(position: DVec3) -> f32 {
    let (scene, elapsed) = snapshot();
    scene
        .blasts
        .iter()
        .map(|b| {
            let age = b.age + elapsed;
            let distance = position.distance(DVec3::from_array(b.center)) as f32;
            let arrival = distance / 2.;
            if age < arrival || age > arrival + 25. {
                0.
            } else {
                (1. - (age - arrival) / 25.) * (1. - distance / 180.).max(0.) * 2.8
            }
        })
        .fold(0., f32::max)
}
pub fn clip_view(mut view: super::view::View) -> super::view::View {
    if let Some(plane) = CLIP.with(Cell::get) {
        let eye_plane = view.view.inverse().transpose() * plane;
        view.projection = oblique(view.projection, eye_plane);
    }
    view
}

struct PortalTarget {
    device: wgpu::Device,
    texture: wgpu::Texture,
    depth: wgpu::TextureView,
    post: Option<super::world_post::Post>,
    size: [u32; 2],
    format: wgpu::TextureFormat,
}
impl PortalTarget {
    fn new(renderer: &super::Renderer, size: [u32; 2]) -> Self {
        let mut config = renderer.config.clone();
        [config.width, config.height] = size;
        config.format = renderer.frame_format;
        config.view_formats = vec![renderer.frame_format];
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST;
        Self {
            device: renderer.device.clone(),
            texture: super::Renderer::offscreen_target(&renderer.device, &config),
            depth: super::Renderer::depth(&renderer.device, &config),
            post: Some(super::world_post::Post::new(
                &renderer.device,
                renderer.frame_format,
            )),
            size,
            format: renderer.frame_format,
        }
    }
}

/// Two independently rendered views, never a screen capture or static portal picture.
/// The secondary pass has its own depth and temporal state and cannot recurse.
pub fn prepare(renderer: &mut super::Renderer, frame: &super::Frame<'_>) {
    super::native_render::set_mods_built_in(true);
    READY.with(|r| r.borrow_mut().clear());
    if frame.title {
        clear();
        return;
    }
    let (scene, _) = snapshot();
    let mut visible = Vec::new();
    for (a, b) in scene.pairs.iter().filter_map(|p| match p {
        [Some(a), Some(b)] => Some((a, b)),
        _ => None,
    }) {
        for (entry, exit) in [(a, b), (b, a)] {
            let d = frame.camera - entry.c();
            if d.length_squared() <= 112.0 * 112.0 && d.dot(entry.n()) > -0.12 {
                visible.push((entry.clone(), exit.clone(), d.length_squared()));
            }
        }
    }
    visible.sort_by(|a, b| a.2.total_cmp(&b.2));
    visible.truncate(2);
    if visible.is_empty() {
        return;
    }
    SAVED_CONTROLS.with(|s| s.set(Some((renderer.frame_gen.on, renderer.world_post.taa))));
    // Portal passes must not enter the present thread or reuse another camera's TAA history.
    renderer.frame_gen.stop();
    renderer.frame_gen.on = false;
    renderer.world_post.taa = false;
    let window_size = [renderer.config.width, renderer.config.height];
    let factor = (960.0 / window_size[0].max(1) as f32).min(1.0);
    let size = window_size.map(|v| ((v as f32 * factor).round() as u32).max(1));
    let mut targets = TARGETS.with(|t| std::mem::take(&mut *t.borrow_mut()));
    while targets.len() < visible.len() {
        targets.push(PortalTarget::new(renderer, size));
    }
    for (index, (entry, exit, _)) in visible.iter().enumerate() {
        if targets[index].size != size
            || targets[index].format != renderer.frame_format
            || targets[index].device != renderer.device
        {
            targets[index] = PortalTarget::new(renderer, size);
        }
        let target = &mut targets[index];
        let camera = entry.point_to(exit, frame.camera);
        let [yaw, pitch] = angles(entry.rotate_to(exit, direction(frame.yaw, frame.pitch)));
        let delta = (frame.camera - camera).as_vec3();
        let entities: Vec<_> = frame
            .entities
            .iter()
            .map(|v| {
                let mut v = *v;
                v.pos = (Vec3::from_array(v.pos) + delta).to_array();
                v
            })
            .collect();
        let hud = crate::hud::HudBatch::new(size);
        let portal_frame = subframe(frame, camera, yaw, pitch, &entities, &hud);
        let saved_surface = renderer.surface.take();
        let saved_hdr = renderer.hdr.take();
        let saved_native = renderer.native.take();
        let saved_pack = renderer.pack_post.take();
        let saved_offscreen = renderer.offscreen.replace(target.texture.clone());
        let saved_depth = std::mem::replace(&mut renderer.depth, target.depth.clone());
        let saved_post = std::mem::replace(&mut renderer.world_post, target.post.take().unwrap());
        [renderer.config.width, renderer.config.height] = size;
        let n = exit.n().as_vec3();
        CLIP.with(|p| {
            p.set(Some(
                n.extend((camera - exit.c()).dot(exit.n()) as f32 - 0.015),
            ))
        });
        PASS.with(|p| p.set(true));
        let result = renderer.draw_frame(&portal_frame, None);
        PASS.with(|p| p.set(false));
        CLIP.with(|p| p.set(None));
        target.post = Some(std::mem::replace(&mut renderer.world_post, saved_post));
        renderer.depth = saved_depth;
        renderer.offscreen = saved_offscreen;
        renderer.pack_post = saved_pack;
        renderer.native = saved_native;
        renderer.hdr = saved_hdr;
        renderer.surface = saved_surface;
        [renderer.config.width, renderer.config.height] = window_size;
        match result {
            Ok(true) => READY.with(|r| {
                r.borrow_mut().push((
                    entry.clone(),
                    target.texture.create_view(&Default::default()),
                ))
            }),
            Ok(false) => {}
            Err(e) => log::warn!("Atomic Age portal view: {e}"),
        }
    }
    TARGETS.with(|t| *t.borrow_mut() = targets);
}
pub fn finish(renderer: &mut super::Renderer) {
    if let Some((framegen, taa)) = SAVED_CONTROLS.with(|s| s.take()) {
        renderer.frame_gen.on = framegen;
        renderer.world_post.taa = taa;
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FxVertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    mode: f32,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    matrix: [[f32; 4]; 4],
    size: [f32; 4],
    time: [f32; 4],
}
struct EffectGpu {
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
}
impl EffectGpu {
    fn new(world: &super::native_render::WorldDrawn<'_>) -> Self {
        let device = world.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Atomic Age effects"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Atomic Age effects"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Atomic Age effects"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let attributes =
            wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x4,3=>Float32];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Atomic Age effects"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<FxVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: world.color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let white = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Atomic Age fallback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        world.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &white,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0, 0, 0, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        Self {
            device: device.clone(),
            format: world.color_format,
            layout,
            pipeline,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            white: white.create_view(&Default::default()),
        }
    }
    fn draw(
        &self,
        world: &mut super::native_render::WorldDrawn<'_>,
        vertices: &[FxVertex],
        texture: &wgpu::TextureView,
        seconds: f32,
    ) {
        if vertices.is_empty() {
            return;
        }
        let uniform = Uniform {
            matrix: world.jittered.to_cols_array_2d(),
            size: [
                world.size[0] as f32,
                world.size[1] as f32,
                world.depth_size[0] as f32,
                world.depth_size[1] as f32,
            ],
            time: [seconds, 0., 0., 0.],
        };
        let buffer = world
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Atomic Age uniforms"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let vertices_buffer = world
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Atomic Age particles"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let bind = world.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atomic Age effects"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(world.depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = world
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Atomic Age effects"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: world.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.set_vertex_buffer(0, vertices_buffer.slice(..));
        pass.draw(0..vertices.len() as u32, 0..1);
    }
}
fn quad(out: &mut Vec<FxVertex>, points: [Vec3; 4], color: [f32; 4], mode: f32) {
    for i in [0, 1, 2, 0, 2, 3] {
        out.push(FxVertex {
            position: points[i].to_array(),
            uv: [[-1., -1.], [-1., 1.], [1., 1.], [1., -1.]][i],
            color,
            mode,
        });
    }
}
fn billboard(
    out: &mut Vec<FxVertex>,
    center: Vec3,
    right: Vec3,
    up: Vec3,
    size: f32,
    color: [f32; 4],
    mode: f32,
) {
    quad(
        out,
        [
            center - right * size - up * size,
            center - right * size + up * size,
            center + right * size + up * size,
            center + right * size - up * size,
        ],
        color,
        mode,
    );
}
fn noise(i: usize) -> f32 {
    let n = (i as f32 * 127.1 + 311.7).sin() * 43758.547;
    n - n.floor()
}

pub fn draw(world: &mut super::native_render::WorldDrawn<'_>) {
    if PASS.with(Cell::get) {
        return;
    }
    let (scene, elapsed) = snapshot();
    if scene.pairs.is_empty() && scene.blasts.is_empty() && scene.beams.is_empty() {
        return;
    }
    GPU.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|g| g.format != world.color_format || g.device != *world.device)
        {
            *cache = Some(EffectGpu::new(world));
        }
        let gpu = cache.as_ref().unwrap();
        READY.with(|ready| {
            let ready = ready.borrow();
            for portal in scene.pairs.iter().flatten().flatten() {
                let texture = ready
                    .iter()
                    .find(|(p, _)| p.center == portal.center && p.normal == portal.normal)
                    .map_or(&gpu.white, |(_, t)| t);
                let c = (portal.c() - world.camera).as_vec3();
                let r = portal.right().as_vec3() * 0.91;
                let u = Vec3::Y * 1.41;
                let mut v = Vec::new();
                quad(
                    &mut v,
                    [c - r - u, c - r + u, c + r + u, c + r - u],
                    if portal.orange {
                        [1., 0.25, 0.01, 1.]
                    } else {
                        [0.01, 0.4, 1., 1.]
                    },
                    2.,
                );
                gpu.draw(world, &v, texture, elapsed / 20.);
            }
        });
        let right = world.view_proj.row(0).truncate().normalize_or_zero();
        let up = world.view_proj.row(1).truncate().normalize_or_zero();
        let mut puffs: Vec<(f32, Vec3, f32, [f32; 4], f32)> = Vec::new();
        let mut add = |position: DVec3, size: f32, color: [f32; 4], mode: f32| {
            let c = (position - world.camera).as_vec3();
            puffs.push((c.length_squared(), c, size, color, mode));
        };
        for blast in &scene.blasts {
            let age = blast.age + elapsed;
            let origin = DVec3::from_array(blast.center);
            if age < 0. {
                if blast.kind == 5 {
                    for i in 0..18 {
                        add(
                            origin + DVec3::Y * f64::from(-age * 1.4 + i as f32 * 1.5),
                            3. - i as f32 * 0.10,
                            [1., 0.35, 0.015, 0.9 - i as f32 * 0.04],
                            1.,
                        );
                    }
                    continue;
                }
                let size = 1.5 + (-age * 0.2).sin().abs();
                add(origin + DVec3::Y * 2., size, [1., 0.12, 0.01, 0.75], 1.);
                continue;
            }
            if age > 600. {
                continue;
            }
            let fade = ((600. - age) / 100.).clamp(0., 1.);
            if blast.kind == 1 {
                for i in 0..96 {
                    let a = i as f32 * std::f32::consts::TAU / 96. + age * 0.05;
                    let r = 5. + (age * 0.04).sin() * 2.;
                    add(
                        origin
                            + DVec3::new(
                                f64::from(a.cos() * r),
                                f64::from((i % 9) as f32 * 0.4),
                                f64::from(a.sin() * r),
                            ),
                        1.3,
                        [0.5, 0.04, 1., fade * 0.8],
                        1.,
                    );
                }
                continue;
            }
            if blast.kind == 2 {
                for i in 0..21 {
                    let t = age - i as f32 * 3.;
                    if !(0. ..35.).contains(&t) {
                        continue;
                    }
                    let a = i as f32 * 3. * 2.39996;
                    let r = (i as f32 * 3. + 3.).sqrt() * 4.;
                    let p = origin
                        + DVec3::new(
                            f64::from(a.cos() * r),
                            3. + f64::from(t * 0.2),
                            f64::from(a.sin() * r),
                        );
                    add(
                        p,
                        2. + t * 0.13,
                        [1. - t / 45., 0.25, 0.04, (1. - t / 35.) * 0.85],
                        if t < 8. { 1. } else { 0. },
                    );
                }
                continue;
            }
            if blast.kind == 3 {
                if age < 110. {
                    for i in 0..150 {
                        let a = noise(i + 100) * std::f32::consts::TAU;
                        let r = noise(i + 400) * 30.;
                        add(
                            origin
                                + DVec3::new(
                                    f64::from(a.cos() * r),
                                    f64::from(noise(i + 600) * 5. + age * 0.04),
                                    f64::from(a.sin() * r),
                                ),
                            3.5,
                            [0.9, 0.28, 0.025, (1. - age / 110.) * 0.75],
                            if age < 25. { 1. } else { 0. },
                        );
                    }
                }
                continue;
            }
            if blast.kind == 4 {
                if age < 120. {
                    for i in 0..65 {
                        let a = i as f32 * 2.39996 + age * 0.18;
                        let r = 2. + noise(i) * 3.;
                        add(
                            origin
                                + DVec3::new(
                                    f64::from(a.cos() * r),
                                    f64::from(noise(i + 70) * 10.),
                                    f64::from(a.sin() * r),
                                ),
                            1.5,
                            [0.6, 0.45, 0.24, (1. - age / 120.) * 0.6],
                            0.,
                        );
                    }
                }
                continue;
            }
            let scale = if blast.kind == 0 { 1. } else { 0.50 };
            if age < 40. {
                add(
                    origin + DVec3::Y * 4.,
                    (4. + age * 1.2) * scale,
                    [1., 0.45, 0.07, (1. - age / 40.) * 0.95],
                    1.,
                );
            }
            // Thin fast pressure front, followed by a rolling dirty dust wall.
            if age < 75. {
                let radius = (3. + age * 2.0) * scale;
                for i in 0..160 {
                    let a = i as f32 * std::f32::consts::TAU / 160.;
                    let pos = origin
                        + DVec3::new(
                            f64::from(a.cos() * radius),
                            0.7,
                            f64::from(a.sin() * radius),
                        );
                    add(
                        pos,
                        (1.2 + age * 0.055) * scale,
                        [0.95, 0.88, 0.70, (1. - age / 80.) * 0.85],
                        0.,
                    );
                    if i % 2 == 0 {
                        add(
                            pos - DVec3::new(f64::from(a.cos() * 3.), -1., f64::from(a.sin() * 3.)),
                            (2. + age * 0.06) * scale,
                            [0.35, 0.29, 0.21, (1. - age / 80.) * 0.6],
                            0.,
                        );
                    }
                }
            }
            let rise = (age * 0.40).min(62.) * scale;
            let cap = (7. + age.sqrt() * 2.2).min(38.) * scale;
            let hot = (1. - age / 160.).clamp(0., 1.);
            for i in 0..220 {
                let a = noise(i * 3) * std::f32::consts::TAU;
                let ring = 0.30 + noise(i * 3 + 1) * 0.7;
                let roll = age * 0.025 + noise(i * 3 + 2) * std::f32::consts::TAU;
                let r = cap * ring + roll.cos() * 2. * scale;
                let y =
                    5. * scale + rise + (1. - ring * ring) * cap * 0.46 + roll.sin() * cap * 0.17;
                let warmth = hot * (0.45 + noise(i + 19) * 0.4);
                add(
                    origin
                        + DVec3::new(f64::from(a.cos() * r), f64::from(y), f64::from(a.sin() * r)),
                    (3. + noise(i + 44) * 3.6) * scale,
                    [
                        0.20 + warmth * 0.8,
                        0.18 + warmth * 0.27,
                        0.15 + warmth * 0.025,
                        0.80 * fade,
                    ],
                    0.,
                );
            }
            for i in 0..130 {
                let h = noise(i + 700) * rise;
                let a = noise(i + 1000) * std::f32::consts::TAU + age * 0.03;
                let r = (2. + (h / (rise + 0.01)).powi(3) * 4.) * scale;
                add(
                    origin
                        + DVec3::new(f64::from(a.cos() * r), f64::from(h), f64::from(a.sin() * r)),
                    (2. + noise(i + 500) * 2.3) * scale,
                    [0.32 + hot * 0.55, 0.25 + hot * 0.12, 0.17, 0.7 * fade],
                    0.,
                );
            }
        }
        puffs.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut vertices = Vec::with_capacity(puffs.len() * 6 + scene.beams.len() * 12);
        for (_, c, size, color, mode) in puffs {
            billboard(&mut vertices, c, right, up, size, color, mode);
        }
        for beam in &scene.beams {
            if elapsed < 5. {
                let a = (DVec3::from_array(beam.from) - world.camera).as_vec3();
                let b = (DVec3::from_array(beam.to) - world.camera).as_vec3();
                let side = (b - a).cross(-a).normalize_or_zero();
                for (width, color) in [
                    (0.11, [1., 0.01, 0.005, 0.35]),
                    (0.028, [1., 0.22, 0.12, 1.]),
                ] {
                    let s = side * width;
                    quad(&mut vertices, [a - s, a + s, b + s, b - s], color, 3.);
                }
                billboard(&mut vertices, b, right, up, 0.5, [1., 0.2, 0.02, 0.9], 1.);
            }
        }
        gpu.draw(world, &vertices, &gpu.white, elapsed / 20.);
        let flash = scene
            .blasts
            .iter()
            .map(|b| {
                let t = b.age + elapsed;
                if (0. ..8.).contains(&t) {
                    (1. - t / 8.)
                        * (1. - world.camera.distance(DVec3::from_array(b.center)) as f32 / 256.)
                            .max(0.)
                } else {
                    0.
                }
            })
            .fold(0., f32::max)
            * 0.8;
        if flash > 0.001 {
            let mut flash_vertices = Vec::new();
            quad(
                &mut flash_vertices,
                [
                    Vec3::new(-1., -1., 0.),
                    Vec3::new(-1., 1., 0.),
                    Vec3::new(1., 1., 0.),
                    Vec3::new(1., -1., 0.),
                ],
                [1., 0.94, 0.78, flash],
                5.,
            );
            gpu.draw(world, &flash_vertices, &gpu.white, elapsed / 20.);
        }
    });
}

const SHADER: &str = r#"
struct Uniform {matrix:mat4x4<f32>,size:vec4<f32>,time:vec4<f32>};
@group(0) @binding(0) var<uniform> u:Uniform;
@group(0) @binding(1) var depth:texture_depth_2d;
@group(0) @binding(2) var portal:texture_2d<f32>;
@group(0) @binding(3) var linear:sampler;
struct V { @builtin(position) pos:vec4<f32>, @location(0) uv:vec2<f32>, @location(1) color:vec4<f32>, @location(2) mode:f32 };
@vertex fn vs(@location(0) p:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) color:vec4<f32>,@location(3) mode:f32)->V {
    var o:V;o.pos=u.matrix*vec4(p,1.);if mode>4.5 {o.pos=vec4(p,1.);}o.uv=uv;o.color=color;o.mode=mode;return o;
}
fn hash(p:vec2<f32>)->f32 {return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
fn noise(p:vec2<f32>)->f32 {let i=floor(p);let f=fract(p);let w=f*f*(3.-2.*f);return mix(mix(hash(i),hash(i+vec2(1.,0.)),w.x),mix(hash(i+vec2(0.,1.)),hash(i+vec2(1.,1.)),w.x),w.y);}
@fragment fn fs(v:V)->@location(0) vec4<f32> {
    if v.mode>4.5 {return v.color;}
    let xy=clamp(vec2<i32>(v.pos.xy/u.size.xy*u.size.zw),vec2<i32>(0),vec2<i32>(u.size.zw)-vec2<i32>(1));
    if v.pos.z+0.000008<textureLoad(depth,xy,0) {discard;}
    let radius=length(v.uv);
    if v.mode>1.5 && v.mode<2.5 {
        if radius>1. {discard;}
        let picture=textureSample(portal,linear,v.pos.xy/u.size.xy);
        let edge=smoothstep(0.86,0.99,radius);
        let ripple=0.8+0.2*sin(atan2(v.uv.y,v.uv.x)*27.+u.time.x*4.);
        return vec4(mix(picture.rgb,v.color.rgb*ripple,edge),1.);
    }
    if v.mode>2.5 {return v.color;}
    if radius>1. {discard;}
    if v.mode>0.5 {return vec4(v.color.rgb,min(1.,pow(max(0.,1.-radius),1.4)*v.color.a*2.));}
    let n=noise(v.uv*5.)*0.60+noise(v.uv*13.+u.time.x)*0.25+noise(v.uv*29.)*0.15;
    let alpha=(1.-smoothstep(0.42,1.,radius+(n-0.5)*0.24))*v.color.a;
    let shade=0.48+0.52*sqrt(max(0.,1.-dot(v.uv,v.uv)));
    return vec4(v.color.rgb*shade*(0.75+n*0.35),alpha);
}
"#;

#[cfg(test)]
mod render_tests {
    use super::*;
    #[test]
    fn effects_shader_passes_wgsl_validation() {
        let module = naga::front::wgsl::parse_str(SHADER).expect("Atomic Age WGSL parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("Atomic Age WGSL validates");
    }
    #[test]
    fn billboard_axes_stay_orthogonal_when_looking_up_and_turning() {
        for (yaw, pitch) in [(0., 0.), (90., 70.), (-135., -55.)] {
            let v = super::super::world_projection(70., 1.6, 512.)
                * crate::camera::view_matrix(yaw, pitch);
            let right = v.row(0).truncate().normalize();
            let up = v.row(1).truncate().normalize();
            assert!(right.dot(up).abs() < 1e-5);
            assert!(right.cross(up).length() > 0.999);
        }
    }
    #[test]
    fn gpu_effects_respect_terrain_depth_and_the_portal_oval() {
        let instance = crate::gfx::test_gpu::instance();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("Atomic Age GPU test skipped: no adapter");
            return;
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        eprintln!("Atomic Age GPU test: {}", adapter.get_info().name);
        let texture = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Atomic Age GPU test"),
                size: wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = texture(
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let color_view = color.create_view(&Default::default());
        let depth = texture(
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth_view = depth.create_view(&Default::default());
        let sampler = device.create_sampler(&Default::default());
        let indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 24,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });
        let mut outputs = Vec::new();
        for (z, mode) in [(0.25, 3.), (0.75, 3.), (0.75, 2.)] {
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0.5),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            }
            let mut world = super::super::native_render::WorldDrawn {
                device: &device,
                queue: &queue,
                encoder: &mut encoder,
                color: &color,
                color_view: &color_view,
                color_format: wgpu::TextureFormat::Rgba8Unorm,
                size: [16, 16],
                depth: &depth_view,
                depth_size: [16, 16],
                view_proj: Mat4::IDENTITY,
                jittered: Mat4::IDENTITY,
                camera: DVec3::ZERO,
                entities: &[],
                spans: &[],
                motion: &[],
                atlas: None,
                quad_indices: &indices,
                block_sprites: super::super::native_render::BlockSpriteArrays {
                    views: &[],
                    split: 1,
                    size: 1,
                    nearest: &sampler,
                    filtered: &sampler,
                },
            };
            let gpu = EffectGpu::new(&world);
            let mut vertices = Vec::new();
            quad(
                &mut vertices,
                [
                    Vec3::new(-1., -1., z),
                    Vec3::new(-1., 1., z),
                    Vec3::new(1., 1., z),
                    Vec3::new(1., -1., z),
                ],
                [1., 0., 0., 1.],
                mode,
            );
            gpu.draw(&mut world, &vertices, &gpu.white, 0.);
            let read = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 256 * 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                color.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: None,
                    },
                },
                color.size(),
            );
            queue.submit([encoder.finish()]);
            read.slice(..)
                .map_async(wgpu::MapMode::Read, |r| r.unwrap());
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let mapped = read.slice(..).get_mapped_range().unwrap();
            outputs.push(mapped.to_vec());
        }
        let center = 8 * 256 + 8 * 4;
        assert_eq!(
            &outputs[0][center..center + 3],
            &[0, 0, 0],
            "terrain hides a beam behind it"
        );
        assert_eq!(
            &outputs[1][center..center + 3],
            &[255, 0, 0],
            "a beam in front remains visible"
        );
        assert_eq!(
            &outputs[2][0..3],
            &[0, 0, 0],
            "portal corners stay outside its oval"
        );
        assert!(
            outputs[2][8 * 256 + 15 * 4] > 100,
            "the portal's rim is colored"
        );
    }
}

fn subframe<'a>(
    frame: &'a super::Frame<'a>,
    camera: DVec3,
    yaw: f32,
    pitch: f32,
    entities: &'a [super::Vertex],
    hud: &'a crate::hud::HudBatch,
) -> super::Frame<'a> {
    super::Frame {
        camera: camera,
        yaw: yaw,
        pitch: pitch,
        fov_degrees: frame.fov_degrees.clone(),
        eye: camera,
        eye_rotation: [yaw, pitch],
        first_person: false,
        bob: Mat4::IDENTITY,
        spin: Mat4::IDENTITY,
        sky: frame.sky.clone(),
        shadow_scene_revision: frame.shadow_scene_revision.clone(),
        shadow_cache: false,
        distance: frame.distance.clone(),
        detail_distance: frame.detail_distance.clone(),
        depth_far: frame.depth_far.clone(),
        underwater: frame.underwater.clone(),
        water_plane: None,
        cave_culling: false,
        title: frame.title.clone(),
        sky_blocked: frame.sky_blocked.clone(),
        skybox: frame.skybox.clone(),
        end_sky: frame.end_sky.clone(),
        end_sky_sprite: frame.end_sky_sprite.clone(),
        boss_world_fog: frame.boss_world_fog.clone(),
        environment_fog: frame.environment_fog.clone(),
        water_fog: frame.water_fog.clone(),
        fog: frame.fog.clone(),
        dark_disc: frame.dark_disc.clone(),
        outline: Vec::new(),
        outline_after_terrain: frame.outline_after_terrain.clone(),
        outline_high_contrast: frame.outline_high_contrast.clone(),
        pvp: frame.pvp.clone(),
        debug_lines: &[],
        debug_lines_on_top: &[],
        debug_crosshair: &[],
        hud: hud,
        glint_alpha: frame.glint_alpha.clone(),
        glint_ticks: frame.glint_ticks.clone(),
        nether_lighting: frame.nether_lighting.clone(),
        entities: entities,
        static_block_entities: frame.static_block_entities,
        visible_static_block_entity_sections: frame.visible_static_block_entity_sections,
        static_block_entity_stats: frame.static_block_entity_stats.clone(),
        entity_motion: &[],
        entity_motion_posed: &[],
        glowing: &[],
        entity_spans: &[],
        skin: None,
        weather: &[],
        weather_time: frame.weather_time.clone(),
        dynamic_lights: frame.dynamic_lights,
        colored_lights: frame.colored_lights,
        emissive_layers: frame.emissive_layers,
        clouds: None,
        lightning: Default::default(),
        additive: &[],
        see_through: &[],
        world_primitives: Default::default(),
        shadow_only: &[],
        celestial: &[],
        stars: None,
        hand: &[],
        hand_items: &[],
        hand_fov_degrees: frame.hand_fov_degrees.clone(),
        screen: &[],
        pack: frame.pack.clone(),
        border: None,
        game_time: frame.game_time.clone(),
        shader_game_time: frame.shader_game_time.clone(),
        end_portal_sprites: frame.end_portal_sprites.clone(),
        crumbling_sprite: frame.crumbling_sprite.clone(),
        ticks: frame.ticks.clone(),
        darkness: frame.darkness.clone(),
        night_vision: frame.night_vision.clone(),
        block_flicker: frame.block_flicker.clone(),
        boss_darken: frame.boss_darken.clone(),
        menu_blur: None,
        chunk_fade: frame.chunk_fade.clone(),
        post_effects: crate::post_chain::Request {
            effects: &[],
            root: frame.post_effects.root,
            menu_blur_radius: 0.,
            globals: Default::default(),
        },
    }
}
