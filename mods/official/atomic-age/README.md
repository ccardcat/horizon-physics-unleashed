# Atomic Age 0.3

Six destructive Rust/WASM bombs for **Horizon**, the Rust Minecraft reimplementation. This does not load in Java Minecraft/Fabric/Forge.

Aim at a block within 96 blocks, then use a command:

| Command | Physical payload | Choreography |
| --- | --- | --- |
| `/nuke` | 49 TNT; six expanding rings out to 36 blocks | Counter-rotating charge halos, flash, rising mushroom cap, ground shockwave |
| `/singularity` | 30 TNT in a compact implosion and expanding rings | Pulls non-player entities during charging, collapsing portal rings, orbiting purple jets |
| `/cluster` | 48 falling TNT with 48-tick fuses, spread across 35 blocks | Eight rotating drops, burning overhead halo and smoke |
| `/thermobaric` | 56 TNT in seven outward waves | Three-layer expanding fire hemisphere, then a lingering smoke dome |
| `/drill` | 27 TNT across nine depths, reaching 32 blocks below target | Electric charge rings, descending blast core, rising spark/debris plume |
| `/meteor` | Four falling charges from 48 blocks up, then 42 secondary charges | Orbital targeting column, descending fire trail, impact rings and lava particles |

Every attack has a five-second fuse, sound cues, its own HUD name/color, brief translucent screen tint and camera shake. The visual lava/fire is particles; no permanent lava or fire blocks are placed. A full sequence takes 13 seconds, followed by ten seconds of cooldown. One attack at a time. Commands are capped at 48 per hook (below the host's 64-per-tick execution budget); TNT is distributed over time instead of spawning in one stack. Performance still depends on terrain and hardware.

**Requires cheats/operator permissions. Real TNT damages entities and destroys destructible terrain**, subject to the game's normal explosion rules and block resistance. This is scripted TNT choreography, not a new blast-physics solver: bedrock remains resistant, there is no radiation simulation, and craters follow the terrain. Use a disposable creative world first. The visual radius is not a promise that every block inside a sphere is removed.

`/atomic help` lists attacks. `/atomic cancel` clears the HUD/tint and stops future commands. TNT already spawned remains live. Leaving the world or changing dimension also stops the sequence. The gravity well excludes players, dropped items and TNT. Camera shake follows the game's Distortion Effects setting; particles follow its particle settings.

## Build and install

From the repository root, using Rust stable:

```sh
rustup target add wasm32-unknown-unknown
cargo build --locked -p cargo-horizon-mod
cd mods/official/atomic-age
../../../target/debug/cargo-horizon-mod build --debug
../../../target/debug/cargo-horizon-mod check ../../../target/horizon-mod/atomic_age.hmod
../../../target/debug/cargo-horizon-mod test --hmod ../../../target/horizon-mod/atomic_age.hmod
```

Copy `target/horizon-mod/atomic_age.hmod` from the repository root into your game's `horizon-mods/` and enable Atomic Age in Options > Mods, including the experimental render capability. Horizon must be built with `horizon-client/mods`. For development, pass `--dev-mod /absolute/path/to/atomic_age.hmod` to that game build. The mod is off unless enabled.

## Tests

Build the package first as above, then from the repository root:

```sh
cargo test --locked -p atomic-age
cargo clippy --locked -p atomic-age --all-targets -- -D warnings
```

Four native tests cover all six payload schedules, fuses, command limits, depth/altitude, gravity exclusions and build-height clamps. Two integration tests load the built `.hmod` in Horizon's actual WASM runtime and exercise all six complete sequences, real TNT command output, cooldown/relaunch, cancellation, dimension changes and overlay cleanup. The TOML sandbox script covers registration, invalid targets, help and cancellation.

The runtime tests validate commands emitted by the mod; they do not simulate TNT physics or render the game. Terrain destruction, multiplayer permissions and visual appearance still require an in-game check with prepared Minecraft resources.

## Bomb blocks and creative inventory (0.3)

The **Bombs / Бомбы** creative tab contains six placeable blocks and a Remote Detonator / Пульт подрыва. Each bomb has a distinct colored hazard texture and English/Russian names. Place a bomb normally, hold the detonator, aim at the placed block and right-click. The block is removed and its matching five-second attack starts. The reusable detonator does not affect ordinary blocks; cooldown and the one-active-attack limit also apply here. Arming still requires cheats/operator permissions because the payload runs commands.

Restart the game after enabling/updating this content mod so its registries load. Existing slash commands remain available. This version declares content API 0.2 (`registries`), client input and chat capabilities; allow the experimental `input` and `render` capabilities when enabling it.

**The creative tab requires the Horizon client changes in this checkout.** Older builds reject the new tab content; use the updated client and packaging tool from this checkout. The client now reads `data/<enabled namespace>/horizon/creative_tabs/*.json` with `title`, `icon` and an `items` list, uses an unoccupied tab button, and adds these items to creative search. The tab title is localized with `itemGroup.<namespace>.<file name>`. Vanilla inventory is unchanged when no content namespace supplies tabs. If every button is occupied, an extra tab is refused with a log message rather than drawn on top of another tab.

Example: `/give @s atomic_age:nuclear_bomb` and `/give @s atomic_age:detonator` also obtain the items. Blocks use iron-block templates with reduced hardness/resistance; they are armed by the detonator, not automatically by redstone or flint and steel. Bombs broken normally drop their matching item.
