fn run(renderer: &mut crate::gfx::Renderer, result: Result<bool>) -> Result<bool> {
    crate::gfx::atomic_age::finish(renderer); result
}
