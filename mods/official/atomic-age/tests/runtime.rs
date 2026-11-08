//! Exercise the packaged WASM through the same runtime used by the game.
//! Build atomic_age.hmod before running this target (see README).
use horizon_mods::{Hit, Mods, Output, Snapshot, testing::default_player};
use std::path::PathBuf;

fn start() -> Mods {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/horizon-mod/atomic_age.hmod");
    assert!(package.is_file(), "Build the .hmod first; see README");
    let mut mods = Mods::load(&[package], |_| true, |_| false);
    assert!(mods.is_running(), "{:?}", mods.entries()[0].status);
    mods.set_snapshot(snapshot());
    mods.drain();
    mods
}
fn snapshot() -> Snapshot {
    Snapshot {
        player: Some(default_player()),
        view_hit: Some((
            30.0,
            Hit {
                x: 0.5,
                y: 64.0,
                z: 30.5,
                block_x: 0,
                block_y: 63,
                block_z: 30,
                block: "minecraft:stone".into(),
            },
        )),
        gui_size: (427.0, 240.0),
        ..Snapshot::default()
    }
}
fn commands(outputs: Vec<Output>) -> Vec<String> {
    let mut lines = Vec::new();
    for out in outputs {
        match out {
            Output::QuietCommands { lines: batch, .. } => {
                assert!(batch.len() <= 48);
                lines.extend(batch);
            }
            Output::Stopped { reason, .. } => panic!("WASM stopped: {reason}"),
            _ => {}
        }
    }
    lines
}
#[test]
fn six_packaged_bombs_complete_with_real_payloads_without_trapping() {
    for (name, expected) in [
        ("nuke", 49),
        ("singularity", 30),
        ("cluster", 48),
        ("thermobaric", 56),
        ("drill", 27),
        ("meteor", 46),
    ] {
        let mut mods = start();
        assert!(mods.command(name));
        assert!(
            mods.drain()
                .iter()
                .any(|out| matches!(out, Output::Feedback { error: false, .. }))
        );
        let mut payloads = 0;
        let mut shakes = 0;
        for tick in 0..=261 {
            mods.tick(tick);
            let outputs = mods.drain();
            shakes += outputs
                .iter()
                .filter(|out| matches!(out, Output::Shake { .. }))
                .count();
            let lines = commands(outputs);
            let count = lines
                .iter()
                .filter(|s| s.starts_with("summon minecraft:tnt"))
                .count();
            if tick < 100 {
                assert_eq!(count, 0, "{name} detonated during fuse");
            }
            payloads += count;
        }
        assert_eq!(payloads, expected, "{name}");
        assert!(shakes >= 1);
        assert_eq!(mods.hud_shapes().count(), 0, "screen tint must clear");
        assert!(mods.is_running());
        assert!(mods.command(name));
        assert!(mods.drain().iter().any(|out| matches!(out, Output::Feedback { error: true, text } if text.contains("Cooling down"))));
        for tick in 262..=462 {
            mods.tick(tick);
            commands(mods.drain());
        }
        assert!(mods.command(name));
        assert!(
            mods.drain()
                .iter()
                .any(|out| matches!(out, Output::Feedback { error: false, .. }))
        );
    }
}
#[test]
fn cancel_and_dimension_change_stop_further_destruction_and_clear_overlays() {
    for dimension_change in [false, true] {
        let mut mods = start();
        mods.command("nuke");
        mods.drain();
        for tick in 0..=105 {
            mods.tick(tick);
            commands(mods.drain());
        }
        assert!(mods.hud_shapes().count() > 0);
        if dimension_change {
            let mut snapshot = snapshot();
            snapshot.player.as_mut().unwrap().dimension = "minecraft:the_nether".into();
            mods.set_snapshot(snapshot);
        } else {
            mods.command("atomic cancel");
        }
        for tick in 106..=300 {
            mods.tick(tick);
            assert!(
                !commands(mods.drain())
                    .iter()
                    .any(|s| s.starts_with("summon"))
            );
        }
        assert_eq!(mods.hud_shapes().count(), 0);
        assert!(mods.is_running());
    }
}

#[test]
fn detonator_arms_each_registered_block_and_refuses_other_targets() {
    use horizon_mods::HeldItem;
    let item = HeldItem {
        id: "atomic_age:detonator".into(),
        count: 1,
        slot: 0,
    };
    for (block, label) in [
        ("nuclear", "NUCLEAR"),
        ("singularity", "SINGULARITY"),
        ("cluster", "CLUSTER STORM"),
        ("thermobaric", "INFERNO"),
        ("drill", "EARTHBREAKER"),
        ("meteor", "ORBITAL HAMMER"),
    ] {
        let mut mods = start();
        let mut view = snapshot();
        view.view_hit.as_mut().unwrap().1.block = format!("atomic_age:{block}_bomb");
        mods.set_snapshot(view);
        mods.use_start(&item);
        mods.tick(0);
        let output = mods.drain();
        assert!(
            output
                .iter()
                .any(|out| matches!(out, Output::Chat { text, .. } if text.contains("Bomb armed")))
        );
        let lines = commands(output);
        assert!(lines.iter().any(|s| s.contains(&format!(
            "atomic_age:{block}_bomb run setblock 0 63 30 minecraft:air"
        ))));
        assert!(
            mods.panels()
                .any(|p| p.lines.iter().any(|l| l.text.contains(label)))
        );
        // A repeat click during charging must not remove another block.
        mods.use_start(&item);
        mods.tick(1);
        assert!(
            !commands(mods.drain())
                .iter()
                .any(|s| s.contains("setblock"))
        );
    }
    let mut mods = start();
    mods.use_start(&item);
    mods.tick(0);
    let output = mods.drain();
    assert!(output.iter().any(
        |out| matches!(out, Output::Chat { text, .. } if text.contains("not an Atomic Age bomb"))
    ));
    assert!(!commands(output).iter().any(|s| s.contains("setblock")));
}

#[test]
fn packaged_content_registers_six_placeable_bombs_and_one_detonator() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/horizon-mod/atomic_age.hmod");
    let package = horizon_mods::Package::read(&path).unwrap();
    let content = horizon_mods::Content::read(&path, &package.manifest).unwrap();
    assert_eq!(content.blocks.len(), 6);
    assert!(content.blocks.iter().all(|block| block.item));
    assert!(
        content
            .items
            .iter()
            .any(|item| item.id == "atomic_age:detonator")
    );
    assert!(
        content
            .files
            .iter()
            .any(|path| path == "data/atomic_age/horizon/creative_tabs/bombs.json")
    );
    assert_eq!(
        content.files.iter().filter(|p| p.ends_with(".png")).count(),
        7
    );
}
