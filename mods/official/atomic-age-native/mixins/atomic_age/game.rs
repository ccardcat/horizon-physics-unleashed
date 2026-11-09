// Atomic Age native gameplay. All terrain edits happen on the authoritative server.
use super::{Game, NativeShake};
use glam::DVec3;
use horizon_client::{
    gfx::atomic_age::{self as visual, Beam, Blast, Portal, Scene},
    items::ItemStack,
    text::Text,
};
use horizon_world::registry::BlockStateId;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

const CHANNEL: &str = "atomic_age:scene";
const MAX_BLASTS: usize = 4;
#[derive(Default)]
struct LevelState {
    blasts: Vec<Blast>,
    pairs: HashMap<u128, [Option<Portal>; 2]>,
    pending: VecDeque<([f64; 3], f32, bool)>,
    beams: HashMap<u128, (Beam, u64)>,
    tick: u64,
    cooldown: HashMap<u128, u64>,
    previous: HashMap<u128, DVec3>,
}
thread_local! {
    static LEVELS:RefCell<HashMap<String,LevelState>>=RefCell::new(HashMap::new());
    static EXTRACTION:RefCell<Option<(Option<horizon_client::gfx::Frustum>,horizon_client::graphics::EntityCullingMode)>>=const{RefCell::new(None)};
}
pub(super) fn extract_begin(game: &mut Game<'_>) {
    if visual::has_portals() {
        EXTRACTION.with(|s| {
            *s.borrow_mut() = Some((game.view_frustum, game.settings.horizon.entity_culling))
        });
        game.view_frustum = None;
        game.settings.horizon.entity_culling = horizon_client::graphics::EntityCullingMode::Off;
    }
}
pub(super) fn extract_end(game: &mut Game<'_>) {
    if let Some((frustum, culling)) = EXTRACTION.with(|s| s.borrow_mut().take()) {
        game.view_frustum = frustum;
        game.settings.horizon.entity_culling = culling;
    }
}
fn take(dimension: &str) -> LevelState {
    LEVELS.with(|s| s.borrow_mut().remove(dimension).unwrap_or_default())
}
fn put(dimension: &str, state: LevelState) {
    LEVELS.with(|s| {
        s.borrow_mut().insert(dimension.to_owned(), state);
    });
}
fn sound(game: &mut Game<'_>, name: &str, pos: [f64; 3], volume: f32, pitch: f32) {
    game.native_play_sound_in_place(name, pos, 4, volume, pitch);
}
fn state_air(game: &Game<'_>) -> Option<BlockStateId> {
    game.native_default_state("minecraft:air")
}
fn bomb_kind(name: &str) -> Option<u8> {
    Some(match name.strip_prefix("atomic_age:")? {
        "nuclear_bomb" => 0,
        "singularity_bomb" => 1,
        "cluster_bomb" => 2,
        "thermobaric_bomb" => 3,
        "drill_bomb" => 4,
        "meteor_bomb" => 5,
        _ => return None,
    })
}
fn forward(yaw: f32, pitch: f32) -> DVec3 {
    visual::direction(yaw, pitch)
}

/// Bounded loaded-world ray. The face is the axis crossed into the hit cell.
fn trace(game: &Game<'_>, eye: DVec3, dir: DVec3) -> Option<([i32; 3], [i32; 3], DVec3)> {
    let mut previous = eye.floor().as_ivec3();
    for i in 1..=480 {
        let point = eye + dir * (i as f64 * 0.20);
        let cell = point.floor().as_ivec3();
        if !game.world.chunks.contains_key(&(cell.x >> 4, cell.z >> 4)) {
            return None;
        }
        let state = game.world.block(cell.x, cell.y, cell.z);
        let facts = game.res.facts().get(state);
        if !matches!(
            facts.name.as_ref(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        ) {
            let delta = previous - cell;
            let face = if delta.x != 0 {
                [delta.x.signum(), 0, 0]
            } else if delta.y != 0 {
                [0, delta.y.signum(), 0]
            } else {
                [0, 0, delta.z.signum()]
            };
            return Some((cell.to_array(), face, point));
        }
        previous = cell;
    }
    None
}
fn send_scene(game: &mut Game<'_>, dimension: &str, state: &LevelState) {
    let scene = Scene {
        pairs: state.pairs.values().cloned().collect(),
        blasts: state.blasts.clone(),
        beams: state
            .beams
            .values()
            .filter(|(_, tick)| state.tick.saturating_sub(*tick) <= 2)
            .map(|(beam, _)| beam.clone())
            .collect(),
    };
    let Ok(bytes) = serde_json::to_vec(&scene) else {
        return;
    };
    for player in game.native_server_players() {
        if player.dimension == dimension {
            let _ = game.native_net_send_to_player(player.uuid, CHANNEL, &bytes);
        }
    }
}

pub(super) fn use_item(game: &mut Game<'_>, stack: &ItemStack, claimed: &mut bool) {
    if !game.native_authoritative() {
        return;
    }
    let Some(player) = game.native_acting_player() else {
        return;
    };
    let eye = DVec3::from_array(player.position) + DVec3::Y * f64::from(player.eye_height);
    let dir = forward(player.yaw, player.pitch);
    match stack.item.as_ref() {
        "atomic_age:detonator" => {
            *claimed = true;
            let Some((pos, _, _)) = trace(game, eye, dir) else {
                return;
            };
            let Some(kind) = bomb_kind(&game.native_block(pos)) else {
                return;
            };
            let mut state = take(&player.dimension);
            if state.blasts.len() >= MAX_BLASTS
                || state
                    .cooldown
                    .get(&player.uuid)
                    .is_some_and(|until| *until > state.tick)
            {
                game.native_action_bar(player.uuid, Text::tr("atomic_age.busy", &[]));
                put(&player.dimension, state);
                return;
            }
            if let Some(air) = state_air(game)
                && game.native_set_block(&player.dimension, pos, air, 3)
            {
                let center = [
                    pos[0] as f64 + 0.5,
                    pos[1] as f64 + 0.5,
                    pos[2] as f64 + 0.5,
                ];
                state.blasts.push(Blast {
                    center,
                    age: -60.,
                    kind,
                });
                state.cooldown.insert(player.uuid, state.tick + 40);
                sound(game, "minecraft:block.beacon.activate", center, 3., 0.55);
                game.native_action_bar(player.uuid, Text::tr("atomic_age.armed", &[]));
                send_scene(game, &player.dimension, &state);
            }
            put(&player.dimension, state);
        }
        "atomic_age:portal_gun" => {
            *claimed = true;
            let Some((pos, face, point)) = trace(game, eye, dir) else {
                return;
            };
            if face[1] != 0 || face == [0, 0, 0] {
                game.native_action_bar(player.uuid, Text::tr("atomic_age.wall_only", &[]));
                return;
            }
            let lower = [pos[0], (point.y - 1.).floor() as i32, pos[2]];
            for y in 0..2 {
                let block = game.world.block(lower[0], lower[1] + y, lower[2]);
                let facts = game.res.facts().get(block);
                if !facts.full_collision || facts.block_entity || facts.hardness < 0. {
                    game.native_action_bar(player.uuid, Text::tr("atomic_age.portal_surface", &[]));
                    return;
                }
                let free = [lower[0] + face[0], lower[1] + y, lower[2] + face[2]];
                if game
                    .res
                    .facts()
                    .get(game.world.block(free[0], free[1], free[2]))
                    .full_collision
                {
                    return;
                }
            }
            let orange = game.native_sneaking();
            let portal = Portal {
                center: [
                    lower[0] as f64 + 0.5 + face[0] as f64 * 0.507,
                    lower[1] as f64 + 1.,
                    lower[2] as f64 + 0.5 + face[2] as f64 * 0.507,
                ],
                normal: face.map(f64::from),
                orange,
            };
            let mut state = take(&player.dimension);
            if state.pairs.len() >= 8 && !state.pairs.contains_key(&player.uuid) {
                put(&player.dimension, state);
                return;
            }
            let pair = state.pairs.entry(player.uuid).or_default();
            if pair[usize::from(!orange)]
                .as_ref()
                .is_some_and(|other| (other.c() - portal.c()).length() < 2.)
            {
                put(&player.dimension, state);
                return;
            }
            pair[usize::from(orange)] = Some(portal.clone());
            sound(
                game,
                "minecraft:block.respawn_anchor.charge",
                portal.center,
                1.2,
                if orange { 0.7 } else { 1.3 },
            );
            send_scene(game, &player.dimension, &state);
            put(&player.dimension, state);
        }
        "atomic_age:portal_reset" => {
            *claimed = true;
            let mut state = take(&player.dimension);
            state.pairs.remove(&player.uuid);
            send_scene(game, &player.dimension, &state);
            put(&player.dimension, state);
        }
        "atomic_age:emergency_stop" => {
            *claimed = true;
            let mut state = take(&player.dimension);
            state.blasts.clear();
            state.pending.clear();
            state.beams.clear();
            send_scene(game, &player.dimension, &state);
            put(&player.dimension, state);
        }
        _ => {}
    }
}

pub(super) fn duration(stack: &ItemStack, duration: &mut i32) {
    if stack.item.as_ref() == "atomic_age:laser_cannon" {
        *duration = 72000;
    }
}
pub(super) fn laser(game: &mut Game<'_>, stack: &ItemStack) {
    if stack.item.as_ref() != "atomic_age:laser_cannon" || !game.native_authoritative() {
        return;
    }
    let Some(player) = game.native_acting_player() else {
        return;
    };
    if matches!(player.game_mode, 2 | 3) {
        return;
    }
    let eye = DVec3::from_array(player.position) + DVec3::Y * f64::from(player.eye_height);
    let dir = forward(player.yaw, player.pitch);
    let hit = trace(game, eye, dir);
    let endpoint = hit.as_ref().map_or(eye + dir * 96., |(_, _, point)| *point);
    let mut state = take(&player.dimension);
    state.beams.insert(
        player.uuid,
        (
            Beam {
                from: (eye + dir * 0.5 - DVec3::Y * 0.16).to_array(),
                to: endpoint.to_array(),
            },
            state.tick,
        ),
    );
    if state.tick % 2 == 0
        && let Some((pos, _, _)) = hit
    {
        let block = game.world.block(pos[0], pos[1], pos[2]);
        let facts = game.res.facts().get(block);
        // Ordinary unbreakable blocks remain unbreakable. Explosions apply real entity damage too.
        if facts.hardness >= 0. && !facts.block_entity {
            game.native_explode(
                &player.dimension,
                endpoint.to_array(),
                1.25,
                true,
                super::native_explosions::NativeBlast::Tnt,
            );
            if let Some(air) = state_air(game) {
                game.native_set_block(&player.dimension, pos, air, 3);
            }
        }
        if state.tick % 6 == 0 {
            sound(
                game,
                "minecraft:block.fire.extinguish",
                endpoint.to_array(),
                0.45,
                1.65,
            );
        }
    }
    put(&player.dimension, state);
}

fn queue_blasts(state: &mut LevelState, blast: &Blast) {
    let age = blast.age as i32;
    let p = DVec3::from_array(blast.center);
    let mut add =
        |at: DVec3, radius: f32, fire: bool| state.pending.push_back((at.to_array(), radius, fire));
    match blast.kind {
        0 => {
            if age == 0 {
                add(p, 18., true);
            }
            if (8..=48).contains(&age) && age % 8 == 0 {
                let radius = age as f64 * 1.2;
                let count = (radius * std::f64::consts::TAU / 16.).ceil() as usize;
                for i in 0..count {
                    let a = i as f64 * std::f64::consts::TAU / count as f64;
                    add(
                        p + DVec3::new(a.cos() * radius, -1.5, a.sin() * radius),
                        9.,
                        true,
                    );
                }
            }
        }
        1 => {
            if age == 80 {
                add(p, 14., false);
            } else if age == 100 {
                add(p, 7., false);
            }
        }
        2 => {
            if (0..=60).contains(&age) && age % 3 == 0 {
                let a = age as f64 * 2.39996;
                let r = (age as f64 + 3.).sqrt() * 4.;
                add(p + DVec3::new(a.cos() * r, 3., a.sin() * r), 5., true);
            }
        }
        3 => {
            if age == 0 {
                add(p, 12., true);
            }
            if (5..=60).contains(&age) && age % 5 == 0 {
                let a = age as f64 * 2.39996;
                add(p + DVec3::new(a.cos() * 22., 0., a.sin() * 22.), 8., true);
            }
        }
        4 => {
            if (0..=100).contains(&age) && age % 5 == 0 {
                add(p - DVec3::Y * (age as f64 * 0.5), 4., false);
            }
        }
        5 => {
            if age == 0 {
                add(p, 22., true);
            }
            if (5..=45).contains(&age) && age % 5 == 0 {
                let a = age as f64 * 2.39996;
                add(p + DVec3::new(a.cos() * 25., 0., a.sin() * 25.), 8., true);
            }
        }
        _ => {}
    }
}
pub(super) fn level_tick(game: &mut Game<'_>, dimension: &str) {
    if !game.native_authoritative() || game.dim.id() != dimension {
        return;
    }
    let mut state = take(dimension);
    state.tick += 1;
    for blast in state.blasts.clone() {
        if blast.age < 0. && blast.age as i32 % 20 == 0 {
            sound(
                game,
                "minecraft:block.note_block.pling",
                blast.center,
                2.,
                0.65 + (-blast.age / 100.),
            );
        }
        if blast.age == 0. {
            sound(
                game,
                "minecraft:entity.generic.explode",
                blast.center,
                12.,
                0.5,
            );
        }
        if blast.age >= 0. {
            queue_blasts(&mut state, &blast);
        }
        if blast.kind == 1 && (0.0..80.0).contains(&blast.age) {
            for mob in game.native_mobs(dimension) {
                let delta = DVec3::from_array(blast.center) - DVec3::from_array(mob.pos);
                if delta.length_squared() < 36. * 36. {
                    game.native_push_mob(
                        dimension,
                        mob.id,
                        (delta.normalize_or_zero() * 0.25).to_array(),
                    );
                }
            }
        }
    }
    // Bounded work per tick: no synchronous radius-100 sphere of block writes.
    for _ in 0..2 {
        if let Some((at, radius, fire)) = state.pending.pop_front() {
            game.native_explode(
                dimension,
                at,
                radius,
                fire,
                super::native_explosions::NativeBlast::Tnt,
            );
        }
    }
    for blast in &mut state.blasts {
        blast.age += 1.;
    }
    state.blasts.retain(|b| b.age < 600.);
    state
        .beams
        .retain(|_, (_, tick)| state.tick.saturating_sub(*tick) < 4);
    let players = game.native_server_players();
    for player in players.iter().filter(|p| p.dimension == dimension) {
        let foot = DVec3::from_array(player.position);
        let center = foot + DVec3::Y * 0.9;
        let previous = state.previous.insert(player.uuid, center).unwrap_or(center);
        if state
            .cooldown
            .get(&player.uuid)
            .is_some_and(|until| *until > state.tick)
        {
            continue;
        }
        let mut passage = None;
        for pair in state.pairs.values() {
            if let [Some(a), Some(b)] = pair {
                for (entry, exit) in [(a, b), (b, a)] {
                    // Travel when the capsule reaches the plane; walls stay intact and no blocks are carved.
                    if entry
                        .crossing(previous - entry.n() * 0.34, center - entry.n() * 0.34)
                        .is_some()
                    {
                        let mut to = entry.point_to(exit, foot);
                        let distance = (to - exit.c()).dot(exit.n());
                        to += exit.n() * (0.36 - distance);
                        let look = visual::angles(
                            entry.rotate_to(exit, forward(player.yaw, player.pitch)),
                        );
                        let velocity =
                            entry.rotate_to(exit, (center - previous).clamp_length_max(4.));
                        passage = Some((to, look, velocity));
                        break;
                    }
                }
            }
            if passage.is_some() {
                break;
            }
        }
        if let Some((to, [yaw, pitch], velocity)) = passage {
            if game.native_teleport_player(player.uuid, dimension, to.to_array(), yaw, pitch) {
                state.cooldown.insert(player.uuid, state.tick + 8);
                state.previous.insert(player.uuid, to + DVec3::Y * 0.9);
                if let Ok(bytes) = serde_json::to_vec(&velocity.to_array()) {
                    let _ =
                        game.native_net_send_to_player(player.uuid, "atomic_age:momentum", &bytes);
                }
                sound(
                    game,
                    "minecraft:entity.enderman.teleport",
                    to.to_array(),
                    0.6,
                    1.5,
                );
            }
        }
    }
    let online: std::collections::HashSet<_> = players.iter().map(|p| p.uuid).collect();
    state.pairs.retain(|uuid, _| online.contains(uuid));
    state.previous.retain(|uuid, _| online.contains(uuid));
    state
        .cooldown
        .retain(|uuid, until| online.contains(uuid) && *until > state.tick);
    if state.tick % 2 == 0 {
        send_scene(game, dimension, &state);
    }
    put(dimension, state);
}
pub(super) fn receive(game: &mut Game<'_>, channel: &str, data: &[u8]) {
    if channel == CHANNEL {
        if let Ok(scene) = serde_json::from_slice::<Scene>(data) {
            visual::receive(scene);
        }
    } else if channel == "atomic_age:momentum" {
        if let Ok([x, y, z]) = serde_json::from_slice::<[f64; 3]>(data)
            && [x, y, z].iter().all(|v| v.is_finite() && v.abs() <= 4.)
        {
            if let Some(p) = game.player.as_mut() {
                p.velocity = horizon_core::geometry::Vec3::new(x, y, z);
                p.fall_distance = 0.;
            }
        }
    }
}
pub(super) fn reset() {
    LEVELS.with(|s| s.borrow_mut().clear());
    visual::clear();
}
pub(super) fn shake(game: &Game<'_>, shake: &mut NativeShake) {
    let Some(player) = game.native_player() else {
        return;
    };
    let eye = DVec3::new(player.x, player.y, player.z);
    let amount = visual::shake_at(eye);
    let t = game.world.game_time as f32;
    shake.pitch += (t * 1.91).sin() * amount;
    shake.yaw += (t * 2.31).cos() * amount * 0.7;
    shake.roll += (t * 1.37).sin() * amount * 0.3;
}
