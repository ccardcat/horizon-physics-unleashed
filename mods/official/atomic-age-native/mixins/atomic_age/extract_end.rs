fn run(game: &mut crate::game::Game<'_>, value: (Vec<Vertex>, Arc<[StaticSectionSnapshot]>, Vec<[i32; 3]>, StaticCacheFrameStats,)) -> (Vec<Vertex>, Arc<[StaticSectionSnapshot]>, Vec<[i32; 3]>, StaticCacheFrameStats,) {
    crate::game::atomic_age::extract_end(game); value
}
