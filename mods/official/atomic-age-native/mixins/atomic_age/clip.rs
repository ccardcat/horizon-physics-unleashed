fn run(yaw: f32, pitch: f32, fov_degrees: f32, aspect: f32, far: f32) -> crate::gfx::view::View {
    crate::gfx::atomic_age::clip_view(crate::gfx::view::View { view:crate::camera::view_matrix(yaw,pitch), projection:crate::gfx::world_projection(fov_degrees,aspect,far), aspect, tan_half_fov:(fov_degrees.to_radians()/2.).tan() })
}
