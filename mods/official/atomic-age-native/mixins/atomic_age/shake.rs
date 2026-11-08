fn run(game: &crate::game::Game<'_>, shake: &mut crate::game::NativeShake) {
    crate::game::atomic_age::shake(game, shake);
}
