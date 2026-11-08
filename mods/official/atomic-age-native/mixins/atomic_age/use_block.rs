fn run(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, claimed: &mut bool) {
    crate::game::atomic_age::use_item(game, stack, claimed);
}
