fn run(stack: &horizon_client::items::ItemStack, duration: &mut i32) {
    crate::game::atomic_age::duration(stack, duration);
}
