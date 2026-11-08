fn run(renderer: &mut crate::gfx::Renderer, frame: &crate::gfx::Frame<'_>) {
    crate::gfx::atomic_age::prepare(renderer, frame);
}
