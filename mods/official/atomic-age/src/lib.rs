//! A command-driven spectacle, using only Horizon's public mod API.
use horizon_mod_sdk::{Mod, Registration, chat, command, game, hud, input, render};

mod effects;
use effects::{Blast, CHARGE, END, Kind, Point, plan};

const COOLDOWN: u32 = 200;
const HELP: &str = "Aim at a block: /nuke /singularity /cluster /thermobaric /drill /meteor. Five-second fuse; /atomic cancel stops future waves. Cheats required. Destructive radius up to about 42 blocks.";

struct AtomicAge {
    blast: Option<Blast>,
    cooldown: u32,
}

fn clear_visuals() {
    hud::clear();
    render::set_hud(&[]);
}

fn overlay(age: u32, color: u32) {
    use render::{FillShape, Rect, Shape};
    let (w, h) = render::gui_size();
    // A brief translucent tint, not an opaque screen flash.
    let alpha = if (CHARGE..CHARGE + 15).contains(&age) {
        (CHARGE + 15 - age) * 4
    } else {
        0
    };
    if alpha == 0 {
        render::set_hud(&[]);
    } else {
        render::set_hud(&[Shape::Fill(FillShape {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
            },
            color: (alpha << 24) | color,
        })]);
    }
}

impl AtomicAge {
    fn available(&self) -> Result<(), String> {
        if self.blast.is_some() {
            return Err("A sequence is already active. /atomic cancel".into());
        }
        if self.cooldown > 0 {
            return Err(format!("Cooling down: {}s", self.cooldown.div_ceil(20)));
        }
        Ok(())
    }
    fn arm(&mut self, kind: Kind, hit: &game::Hit, dimension: String) {
        self.blast = Some(Blast {
            at: Point {
                x: f64::from(hit.block_x) + 0.5,
                y: f64::from(hit.block_y) + 1.0,
                z: f64::from(hit.block_z) + 0.5,
            },
            dimension,
            kind,
            age: 0,
        });
        self.cooldown = END + COOLDOWN;
    }
    fn detonate_block(&mut self) -> Result<(), String> {
        self.available()?;
        let player = game::get_player().ok_or("Join a world first")?;
        let hit = game::raycast(96.0).ok_or("Aim the detonator at a placed Atomic Age bomb")?;
        let kind = Kind::from_block(&hit.block).ok_or("That block is not an Atomic Age bomb")?;
        self.arm(kind, &hit, player.dimension);
        // Conditional replacement: never delete a different block if the target changed.
        command::run_quiet(&[format!(
            "execute if block {} {} {} {} run setblock {} {} {} minecraft:air",
            hit.block_x,
            hit.block_y,
            hit.block_z,
            kind.block_id(),
            hit.block_x,
            hit.block_y,
            hit.block_z
        )]);
        chat::show("Bomb armed. Five seconds to clear the area!");
        Ok(())
    }
}

impl Mod for AtomicAge {
    fn init() -> Result<(Self, Registration), String> {
        input::claim_items(&["atomic_age:detonator".into()]);
        Ok((
            Self {
                blast: None,
                cooldown: 0,
            },
            Registration::new()
                .command("nuke")
                .command("singularity")
                .command("cluster")
                .command("thermobaric")
                .command("drill")
                .command("meteor")
                .command("atomic")
                .on_tick(),
        ))
    }
    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name == "atomic" {
            return match args.trim() {
                "cancel" => {
                    self.blast = None;
                    clear_visuals();
                    Ok(Some(
                        "Sequence cancelled. Already spawned TNT cannot be recalled.".into(),
                    ))
                }
                "" | "help" => Ok(Some(HELP.into())),
                _ => Err("Use /atomic help or /atomic cancel".into()),
            };
        }
        let kind = Kind::from_command(name).ok_or("Unknown bomb")?;
        if !args.trim().is_empty() {
            return Err(format!("Use /{name} with no arguments; aim at a block."));
        }
        self.available()?;
        let player = game::get_player().ok_or("Join a world first")?;
        let hit = game::raycast(96.0).ok_or("Aim at a block within 96 blocks")?;
        self.arm(kind, &hit, player.dimension);
        Ok(Some(
            "Target locked. Detonation in 5 seconds. Move clear!".into(),
        ))
    }
    fn tick(&mut self, _: u64) {
        self.cooldown = self.cooldown.saturating_sub(1);
        for event in input::poll() {
            if matches!(event, input::Event::UseStart(_))
                && let Err(error) = self.detonate_block()
            {
                chat::show(&error);
            }
        }
        let Some(blast) = self.blast.as_mut() else {
            return;
        };
        // Never carry absolute target coordinates into a different world/dimension.
        if game::get_player().is_none_or(|p| p.dimension != blast.dimension) {
            self.blast = None;
            clear_visuals();
            return;
        }
        let lines = plan(blast);
        if !lines.is_empty() {
            command::run_quiet(&lines);
        }
        overlay(blast.age, blast.kind.color());
        if blast.age == CHARGE {
            render::shake(0.9, 2.0);
        } else if blast.age > CHARGE && blast.age < CHARGE + 80 && blast.age.is_multiple_of(20) {
            render::shake(0.3, 0.6);
        }
        if blast.age.is_multiple_of(10) {
            let title = format!("ATOMIC AGE / {}", blast.kind.label());
            hud::set_panel(
                hud::Corner::TopLeft,
                &[
                    hud::Line {
                        text: title,
                        color: blast.kind.color(),
                    },
                    hud::Line {
                        text: if blast.age < CHARGE {
                            format!("DETONATION: {:.1}s", f64::from(CHARGE - blast.age) / 20.0)
                        } else {
                            "SHOCKWAVE // KEEP CLEAR".into()
                        },
                        color: 0xFF5544,
                    },
                ],
            );
        }
        blast.age += 1;
        if blast.age > END {
            self.blast = None;
            clear_visuals();
        }
    }
}
horizon_mod_sdk::export_mod!(AtomicAge);
