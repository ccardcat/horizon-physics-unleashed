fn run(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) {
    crate::game::atomic_age::laser(game, stack);
}
