fn run(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::atomic_age::level_tick(game, dimension);
}
