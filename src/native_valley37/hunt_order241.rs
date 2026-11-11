//! Initial Citizen ground hunting geometry and the observed direct attack clock.

pub const CITIZEN_RADIUS: f32 = 0.125;
pub const CITIZEN_ATTACK_RANGE: f32 = 0.3;
pub const CITIZEN_RELOAD: f32 = 3.0;
pub const CITIZEN_APPLY_TIME: f32 = 1.3;
pub const CITIZEN_DAMAGE: f32 = 5.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rounding {
    Nearest,
    Chop,
}

impl Rounding {
    fn store(self, value: f64) -> f32 {
        let rounded = value as f32;
        if self == Self::Chop && f64::from(rounded).abs() > value.abs() {
            f32::from_bits(rounded.to_bits() - 1)
        } else {
            rounded
        }
    }

    fn add53(self, left: f64, right: f64) -> f64 {
        let sum = left + right;
        if self == Self::Nearest || sum == 0.0 || !sum.is_finite() {
            return sum;
        }
        let moved = sum - left;
        let error = (left - (sum - moved)) + (right - moved);
        if (sum > 0.0 && error < 0.0) || (sum < 0.0 && error > 0.0) {
            f64::from_bits(sum.to_bits() - 1)
        } else {
            sum
        }
    }
}

/// Active animals_15.ddf LandAnimal circle, or its explicit Tiger override.
pub fn edible_land_radius(type_name: &str) -> Option<f32> {
    match type_name {
        "Deer" | "Goat" | "Llama" | "Wolf" | "WhiteWolf" => Some(0.225),
        "Tiger" => Some(0.4),
        _ => None,
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum RangePadding {
    Off,
    Ordinary,
    Long,
}

/// Normal 007FDF80 short-ground-attack maximum reach. Terrain, line of
/// shot, live target validity and aiming are separate original gates.
pub fn ground_reach(
    attacker: [f32; 2],
    target: [f32; 2],
    target_radius: f32,
    range: f32,
    rounding: Rounding,
) -> bool {
    reach(
        attacker,
        target,
        target_radius,
        range,
        rounding,
        RangePadding::Ordinary,
    )
}

fn reach(
    attacker: [f32; 2],
    target: [f32; 2],
    target_radius: f32,
    range: f32,
    rounding: Rounding,
    padding: RangePadding,
) -> bool {
    let margin = match padding {
        RangePadding::Off => 0.0,
        RangePadding::Ordinary => f64::from(0.2f32) + f64::from(0.05f32) + f64::from(0.05f32),
        RangePadding::Long => 1.0 + f64::from(0.05f32) + f64::from(0.05f32),
    };
    let own = rounding.store(margin + f64::from(CITIZEN_RADIUS) + f64::from(range));
    let max = rounding.add53(f64::from(own), f64::from(target_radius));
    let [dx, dy] = std::array::from_fn::<_, 2, _>(|i| {
        f64::from(rounding.store(rounding.add53(f64::from(target[i]), -f64::from(attacker[i]))))
    });
    rounding.add53(dx * dx, dy * dy) <= max * max
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AttackRound {
    pub next: f32,
    pub in_attack: bool,
    pub started: bool,
    pub executed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    None,
    Start,
    Fire,
    End,
    AimReset,
}

/// 007DB4E0 for an unpacked, valid, ordinary tag-1 actor with positive
/// ordnance. The caller owns the original aim/chase/animation admission.
pub fn step_ready(
    round: &mut AttackRound,
    now: f32,
    reload: f32,
    apply: f32,
    modifier: f32,
    aim_ready: bool,
    rounding: Rounding,
) -> Event {
    if !round.started || !round.in_attack {
        if round.in_attack {
            round.in_attack = false;
            round.started = false;
            round.executed = true;
            return Event::AimReset;
        }
        if !aim_ready {
            return Event::None;
        }
        round.in_attack = true;
    }
    let now = f64::from(now);
    let modifier = f64::from(modifier);
    let reload = f64::from(reload) * modifier;
    let apply = f64::from(apply) * modifier;
    if !round.started {
        if f64::from(round.next) <= now {
            round.next = rounding.store(rounding.add53(now, reload));
            round.started = true;
            round.executed = false;
            return Event::Start;
        }
    } else if !round.executed {
        if rounding.add53(f64::from(round.next), -now) < rounding.add53(reload, -apply) {
            round.executed = true;
            return Event::Fire;
        }
    } else if f64::from(round.next) <= now {
        round.started = false;
        return Event::End;
    }
    Event::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_original_modes() {
        let mut rounding = Rounding::Nearest;
        let mut round = AttackRound::default();
        let mut counts = [0; 3];
        let mut cases = 0;
        for line in include_str!("order-trace.txt").lines() {
            let mut words = line.split_ascii_whitespace();
            match words.next().unwrap() {
                "mode" => {
                    rounding = if words.next() == Some("0") {
                        Rounding::Nearest
                    } else {
                        Rounding::Chop
                    };
                    round = AttackRound::default();
                    counts = [0; 3];
                }
                "reach" => {
                    let mode = words.next().unwrap();
                    let x = f32::from_bits(u32::from_str_radix(words.next().unwrap(), 16).unwrap());
                    let expected = words.next() == Some("1");
                    let radius =
                        edible_land_radius(if mode == "3" { "Tiger" } else { "Deer" }).unwrap();
                    let actual = match mode {
                        "0" => reach(
                            [0.0; 2],
                            [x, 0.0],
                            radius,
                            CITIZEN_ATTACK_RANGE,
                            rounding,
                            RangePadding::Off,
                        ),
                        "2" => reach(
                            [0.0; 2],
                            [x, 0.0],
                            radius,
                            CITIZEN_ATTACK_RANGE,
                            rounding,
                            RangePadding::Long,
                        ),
                        _ => {
                            ground_reach([0.0; 2], [x, 0.0], radius, CITIZEN_ATTACK_RANGE, rounding)
                        }
                    };
                    assert_eq!(actual, expected, "{line}");
                    cases += 1;
                }
                "tick" => {
                    let now =
                        f32::from_bits(u32::from_str_radix(words.next().unwrap(), 16).unwrap());
                    let next = u32::from_str_radix(words.next().unwrap(), 16).unwrap();
                    let flags = u32::from_str_radix(words.next().unwrap(), 16).unwrap();
                    counts[2] += u32::from(!round.in_attack || !round.started);
                    let event = step_ready(
                        &mut round,
                        now,
                        CITIZEN_RELOAD,
                        CITIZEN_APPLY_TIME,
                        1.0,
                        true,
                        rounding,
                    );
                    counts[0] += u32::from(event == Event::Start);
                    counts[1] += u32::from(event == Event::Fire);
                    assert_eq!(round.next.to_bits(), next);
                    let actual = (u32::from(round.in_attack) << 16)
                        | (u32::from(round.started) << 8)
                        | u32::from(round.executed);
                    assert_eq!(actual, flags);
                    let expected: Vec<u32> = words.map(|s| s.parse().unwrap()).collect();
                    assert_eq!(expected, [counts[0], counts[1], counts[1], counts[2]]);
                    cases += 1;
                }
                _ => {}
            }
        }
        assert_eq!(cases, 66);
    }
}
