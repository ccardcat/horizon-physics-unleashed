fn run(value: bool) -> bool {
    value && !crate::gfx::atomic_age::has_portals()
}
