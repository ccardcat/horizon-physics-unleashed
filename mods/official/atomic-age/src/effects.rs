//! Deterministic, bounded choreography. Every bomb emits real primed TNT.
use std::f64::consts::TAU;

pub const CHARGE: u32 = 100;
pub const END: u32 = 260;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Nuclear,
    Singularity,
    Cluster,
    Thermobaric,
    Drill,
    Meteor,
}
impl Kind {
    pub const ALL: [Self; 6] = [
        Self::Nuclear,
        Self::Singularity,
        Self::Cluster,
        Self::Thermobaric,
        Self::Drill,
        Self::Meteor,
    ];
    pub fn from_command(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.command() == name)
    }
    pub fn command(self) -> &'static str {
        match self {
            Self::Nuclear => "nuke",
            Self::Singularity => "singularity",
            Self::Cluster => "cluster",
            Self::Thermobaric => "thermobaric",
            Self::Drill => "drill",
            Self::Meteor => "meteor",
        }
    }
    pub fn block_id(self) -> &'static str {
        match self {
            Self::Nuclear => "atomic_age:nuclear_bomb",
            Self::Singularity => "atomic_age:singularity_bomb",
            Self::Cluster => "atomic_age:cluster_bomb",
            Self::Thermobaric => "atomic_age:thermobaric_bomb",
            Self::Drill => "atomic_age:drill_bomb",
            Self::Meteor => "atomic_age:meteor_bomb",
        }
    }
    pub fn from_block(id: &str) -> Option<Self> {
        let id = id.split('[').next()?;
        Self::ALL.into_iter().find(|k| k.block_id() == id)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Nuclear => "NUCLEAR",
            Self::Singularity => "SINGULARITY",
            Self::Cluster => "CLUSTER STORM",
            Self::Thermobaric => "INFERNO",
            Self::Drill => "EARTHBREAKER",
            Self::Meteor => "ORBITAL HAMMER",
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Self::Nuclear => 0xFFCC66,
            Self::Singularity => 0xB077FF,
            Self::Cluster => 0xFF6655,
            Self::Thermobaric => 0xFF8822,
            Self::Drill => 0x55DDFF,
            Self::Meteor => 0xFFAA55,
        }
    }
    fn spark(self) -> &'static str {
        match self {
            Self::Singularity => "portal",
            Self::Drill => "electric_spark",
            _ => "flame",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Point {
    fn coords(self) -> String {
        format!("{:.3} {:.3} {:.3}", self.x, self.y, self.z)
    }
    fn ring(self, radius: f64, angle: f64, height: f64) -> Self {
        Self {
            x: self.x + radius * angle.cos(),
            y: self.y + height,
            z: self.z + radius * angle.sin(),
        }
    }
}
pub struct Blast {
    pub at: Point,
    pub dimension: String,
    pub kind: Kind,
    pub age: u32,
}
fn particle(lines: &mut Vec<String>, at: Point, id: &str, spread: f64, count: u32) {
    lines.push(format!(
        "particle minecraft:{id} {} {spread} {spread} {spread} 0.03 {count} force",
        at.coords()
    ));
}
fn tnt(lines: &mut Vec<String>, mut at: Point, fuse: u16) {
    at.y = at.y.clamp(-60.0, 300.0);
    lines.push(format!(
        "summon minecraft:tnt {} {{fuse:{fuse}s}}",
        at.coords()
    ));
}
fn ring(lines: &mut Vec<String>, at: Point, radius: f64, height: f64, rotation: f64, id: &str) {
    for i in 0..12 {
        particle(
            lines,
            at.ring(radius, rotation + f64::from(i) * TAU / 12.0, height),
            id,
            0.15,
            2,
        );
    }
}
fn sound(lines: &mut Vec<String>, at: Point, id: &str, pitch: f32) {
    lines.push(format!(
        "playsound minecraft:{id} master @a {} 3 {pitch}",
        at.coords()
    ));
}
fn warheads(lines: &mut Vec<String>, b: &Blast, t: u32) {
    let at = b.at;
    match b.kind {
        Kind::Nuclear => {
            if t == 0 {
                tnt(lines, at, 0);
            }
            if (1..=60).contains(&t) && t.is_multiple_of(10) {
                for i in 0..8 {
                    tnt(
                        lines,
                        at.ring(
                            f64::from(t) * 0.6,
                            f64::from(i) * TAU / 8.0 + f64::from(t) * 0.02,
                            0.0,
                        ),
                        0,
                    );
                }
            }
        }
        Kind::Singularity => {
            // Dense implosion followed by a compact reverse shockwave.
            if t <= 40 && t.is_multiple_of(10) {
                for i in 0..6 {
                    tnt(
                        lines,
                        at.ring(2.0 + f64::from(t) * 0.25, f64::from(i) * TAU / 6.0, 0.0),
                        0,
                    );
                }
            }
        }
        Kind::Cluster => {
            if t <= 56 && t.is_multiple_of(8) {
                for i in 0..6 {
                    let angle = f64::from(i) * TAU / 6.0 + f64::from(t) * 0.17;
                    tnt(lines, at.ring(8.0 + f64::from(t) * 0.48, angle, 18.0), 48);
                }
            }
        }
        Kind::Thermobaric => {
            if t <= 60 && t.is_multiple_of(10) {
                for i in 0..8 {
                    tnt(
                        lines,
                        at.ring(2.0 + f64::from(t) * 0.43, f64::from(i) * TAU / 8.0, 0.0),
                        0,
                    );
                }
            }
        }
        Kind::Drill => {
            if t <= 64 && t.is_multiple_of(8) {
                let depth = -f64::from(t) * 0.5;
                // Charges within the rock excavate a vertical shaft without /fill or bedrock deletion.
                for i in 0..3 {
                    tnt(lines, at.ring(1.5, f64::from(i) * TAU / 3.0, depth), 0);
                }
            }
        }
        Kind::Meteor => {
            // A falling physical payload, then seven secondary impact rings.
            if t == 0 {
                for i in 0..4 {
                    tnt(lines, at.ring(1.0, f64::from(i) * TAU / 4.0, 48.0), 70);
                }
            }
            if (70..=118).contains(&t) && (t - 70).is_multiple_of(8) {
                for i in 0..6 {
                    tnt(
                        lines,
                        at.ring(
                            4.0 + f64::from(t - 70) * 0.65,
                            f64::from(i) * TAU / 6.0,
                            0.0,
                        ),
                        0,
                    );
                }
            }
        }
    }
}

pub fn plan(b: &Blast) -> Vec<String> {
    let mut lines = Vec::new();
    if b.age > END {
        return lines;
    }
    let at = b.at;
    let age = b.age;
    let rotation = f64::from(age) * 0.13;
    if age < CHARGE {
        if age.is_multiple_of(5) {
            let radius = 2.0 + f64::from(CHARGE - age) * 0.1;
            ring(&mut lines, at, radius, 0.3, rotation, b.kind.spark());
            // Counter-rotating elevated halo, closing like a targeting reticle.
            ring(&mut lines, at, radius * 0.6, 3.0, -rotation, "end_rod");
            if b.kind == Kind::Meteor {
                for h in 0..6 {
                    particle(
                        &mut lines,
                        at.ring(0.0, 0.0, f64::from(h) * 8.0),
                        "flame",
                        0.25,
                        4,
                    );
                }
            }
        }
        if age.is_multiple_of(20) {
            sound(
                &mut lines,
                at,
                "block.note_block.pling",
                0.6 + age as f32 / 100.0,
            );
        }
        if b.kind == Kind::Singularity && age.is_multiple_of(4) {
            lines.push(format!("execute positioned {} as @e[type=!player,type=!item,type=!tnt,distance=2..18] at @s facing {} run tp @s ^ ^ ^0.65", at.coords(), at.coords()));
        }
        return lines;
    }
    let t = age - CHARGE;
    warheads(&mut lines, b, t);
    if t == 0 {
        particle(&mut lines, at, "flash{color:[1.0,0.85,0.5]}", 0.0, 1);
        particle(&mut lines, at, "explosion_emitter", 2.0, 3);
        sound(&mut lines, at, "entity.generic.explode", 0.5);
    }
    if t.is_multiple_of(5) {
        match b.kind {
            Kind::Nuclear => {
                let rise = f64::from(t).min(90.0) * 0.5;
                particle(
                    &mut lines,
                    at.ring(0.0, 0.0, rise * 0.5),
                    "large_smoke",
                    2.0,
                    20,
                );
                ring(
                    &mut lines,
                    at,
                    4.0 + rise * 0.45,
                    rise,
                    rotation * 0.2,
                    "campfire_cosy_smoke",
                );
                if t < 60 {
                    ring(&mut lines, at, f64::from(t) * 0.7, 1.0, 0.0, "cloud");
                }
            }
            Kind::Singularity => {
                let radius = if t < 40 {
                    18.0 - f64::from(t) * 0.4
                } else {
                    2.0 + f64::from(t - 40) * 0.3
                };
                ring(&mut lines, at, radius, 2.0, -rotation, "portal");
                for i in 0..8 {
                    let angle = f64::from(i) * TAU / 8.0 + rotation;
                    particle(
                        &mut lines,
                        at.ring(4.0, angle, angle.sin() * 6.0 + 7.0),
                        "dragon_breath",
                        0.25,
                        4,
                    );
                }
            }
            Kind::Cluster => {
                ring(
                    &mut lines,
                    at,
                    10.0 + f64::from(t.min(60)) * 0.4,
                    18.0,
                    rotation,
                    "flame",
                );
                particle(&mut lines, at, "large_smoke", 8.0, 12);
            }
            Kind::Thermobaric => {
                // An expanding hemisphere, followed by lingering smoke.
                let radius = 2.0 + f64::from(t.min(60)) * 0.45;
                let id = if t < 65 { "flame" } else { "large_smoke" };
                for layer in 0..3 {
                    let phi = f64::from(layer) * 0.52;
                    ring(
                        &mut lines,
                        at,
                        radius * phi.cos(),
                        radius * phi.sin(),
                        rotation * 0.3,
                        id,
                    );
                }
            }
            Kind::Drill => {
                let depth = -f64::from(t.min(64)) * 0.5;
                particle(&mut lines, at.ring(0.0, 0.0, depth), "explosion", 1.0, 2);
                for i in 0..8 {
                    particle(
                        &mut lines,
                        at.ring(
                            3.0 + f64::from(t) * 0.04,
                            f64::from(i) * TAU / 8.0,
                            f64::from(t % 30),
                        ),
                        "electric_spark",
                        0.2,
                        4,
                    );
                }
                particle(&mut lines, at, "large_smoke", 3.0, 16);
            }
            Kind::Meteor => {
                if t < 70 {
                    let height = (48.0 + 105.84 * (1.0 - 0.98_f64.powf(f64::from(t)))
                        - 1.96 * f64::from(t))
                    .max(0.0);
                    for i in 0..8 {
                        particle(
                            &mut lines,
                            at.ring(0.0, 0.0, height + f64::from(i) * 1.8),
                            "flame",
                            1.0,
                            5,
                        );
                    }
                } else {
                    ring(
                        &mut lines,
                        at,
                        f64::from(t - 65) * 0.5,
                        1.0,
                        rotation,
                        "cloud",
                    );
                    particle(&mut lines, at, "lava", 4.0, 8);
                    particle(
                        &mut lines,
                        at.ring(0.0, 0.0, f64::from(t - 65) * 0.4),
                        "large_smoke",
                        4.0,
                        20,
                    );
                }
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    fn blast(kind: Kind, age: u32) -> Blast {
        Blast {
            at: Point {
                x: 10.0,
                y: 64.0,
                z: -20.0,
            },
            dimension: "minecraft:overworld".into(),
            kind,
            age,
        }
    }
    #[test]
    fn every_bomb_has_real_explosives_and_a_bounded_command_budget() {
        for (kind, expected) in Kind::ALL.into_iter().zip([49, 30, 48, 56, 27, 46]) {
            let mut count = 0;
            for age in 0..=END {
                let lines = plan(&blast(kind, age));
                assert!(lines.len() <= 48, "{kind:?} tick {age}: {}", lines.len());
                count += lines
                    .iter()
                    .filter(|s| s.starts_with("summon minecraft:tnt"))
                    .count();
                if age < CHARGE {
                    assert!(!lines.iter().any(|s| s.starts_with("summon")));
                }
            }
            assert_eq!(count, expected, "{kind:?}");
            assert!(plan(&blast(kind, END + 1)).is_empty());
        }
    }
    #[test]
    fn gravity_never_moves_players_and_only_runs_before_detonation() {
        for kind in Kind::ALL {
            for age in 0..=END {
                for line in plan(&blast(kind, age))
                    .iter()
                    .filter(|s| s.contains("tp @s"))
                {
                    assert_eq!(kind, Kind::Singularity);
                    assert!(age < CHARGE);
                    assert!(line.contains("type=!player,type=!item,type=!tnt"));
                }
            }
        }
    }
    #[test]
    fn payloads_have_distinct_geometry_and_timing() {
        assert!(
            plan(&blast(Kind::Drill, CHARGE + 64))
                .iter()
                .any(|s| s.starts_with("summon minecraft:tnt 11.500 32.000"))
        );
        assert!(
            plan(&blast(Kind::Cluster, CHARGE))
                .iter()
                .any(|s| s.contains("82.000") && s.contains("fuse:48s"))
        );
        assert!(
            plan(&blast(Kind::Meteor, CHARGE))
                .iter()
                .any(|s| s.contains("112.000") && s.contains("fuse:70s"))
        );
        assert!(
            plan(&blast(Kind::Nuclear, CHARGE + 60))
                .iter()
                .filter(|s| s.starts_with("summon"))
                .count()
                == 8
        );
    }
    #[test]
    fn tnt_is_clamped_to_build_height() {
        for kind in Kind::ALL {
            for y in [-64.0, 319.0] {
                let mut b = blast(kind, CHARGE);
                b.at.y = y;
                for age in CHARGE..=END {
                    b.age = age;
                    for line in plan(&b).iter().filter(|s| s.starts_with("summon")) {
                        let y: f64 = line.split_whitespace().nth(3).unwrap().parse().unwrap();
                        assert!((-60.0..=300.0).contains(&y));
                    }
                }
            }
        }
    }
}
