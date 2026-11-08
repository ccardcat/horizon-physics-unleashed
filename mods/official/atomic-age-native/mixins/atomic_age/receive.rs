fn run(game: &mut crate::game::Game<'_>, channel: &str, data: &[u8]) {
    crate::game::atomic_age::receive(game, channel, data);
}
