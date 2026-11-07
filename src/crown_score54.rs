//! Crown score accumulation and the periodic award selector.

#[path = "../visual-cloth7/src/math.rs"]
#[allow(dead_code, unused_imports)]
pub(crate) mod source_math;

use std::cmp::Ordering;

pub const KINDS: usize = 3;
const POSSESSION_SECONDS: f64 = 300.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Counters {
    pub researched: [i32; KINDS],
    pub mobile_kills: i32,
    pub military_units: i32,
    pub building_kills: i32,
    pub buildings_converted: i32,
    pub military_buildings: i32,
    pub economic_buildings: i32,
    pub trade_routes: i32,
    pub gathered_natural: i32,
    pub trade_gold: i32,
    pub territories: i32,
    pub roads_and_bridges: i32,
    pub humans: i32,
    pub stolen_natural: i32,
    pub cultural_units: i32,
    pub non_buildings_converted: i32,
    pub wonders: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub epoch: u8,
    pub epoch_technologies: [u8; KINDS],
    pub adjustment: [f32; KINDS],
    pub counters: Counters,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rating {
    pub score: f32,
    pub threshold: u32,
    pub eligible_rank: u8,
}

fn total(factors: &[(i32, f32)], adjustment: f32) -> f32 {
    let subtotal = factors.iter().fold(0.0, |score, &(count, weight)| {
        let product = source_math::mul(f64::from(count), f64::from(weight));
        source_math::f(source_math::add(product, score))
    });
    source_math::f(source_math::add(subtotal, f64::from(adjustment))) as f32
}

fn scores(player: Player) -> [f32; KINDS] {
    let c = player.counters;
    [
        total(&[(c.researched[0], 1.0), (c.mobile_kills, 1.0), (c.military_units, 1.0),
            (c.building_kills, 1.0), (c.buildings_converted, 1.25), (c.military_buildings, 1.0),
            (c.wonders, 10.0)], player.adjustment[0]),
        total(&[(c.researched[1], 1.0), (c.economic_buildings, 1.0), (c.trade_routes, 5.0),
            (c.gathered_natural, 0.05), (c.trade_gold, 0.05), (c.wonders, 10.0)], player.adjustment[1]),
        total(&[(c.researched[2], 1.0), (c.territories, 10.0), (c.roads_and_bridges, 0.1),
            (c.humans, 1.0), (c.stolen_natural, 0.001), (c.cultural_units, 2.0),
            (c.non_buildings_converted, 2.0), (c.wonders, 10.0)], player.adjustment[2]),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Holder {
    pub owner: u8,
    pub epoch: u8,
    pub expires: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Award {
    Unchanged,
    Revoked(Holder),
    Assigned { previous: Option<Holder>, current: Holder },
}

pub struct Scoreboard<const N: usize> {
    pub ratings: [[Rating; KINDS]; N],
    pub holders: [Option<Holder>; KINDS],
}

impl<const N: usize> Default for Scoreboard<N> {
    fn default() -> Self {
        Self { ratings: [[Rating::default(); KINDS]; N], holders: [None; KINDS] }
    }
}

impl<const N: usize> Scoreboard<N> {
    pub fn due(tick: u32, step: f32) -> bool {
        if !step.is_finite() || step <= 0.0 { return false; }
        let period = source_math::div(1.0, f64::from(step)).ceil() as i32;
        (tick as i32) % period.max(1) == 0
    }

    pub fn update(&mut self, tick: u32, now: f32, step: f32, disabled: bool,
        players: &[Option<Player>; N]) -> [Award; KINDS] {
        assert!(N <= 10);
        if !Self::due(tick, step) {
            return [Award::Unchanged; KINDS];
        }

        for (row, player) in self.ratings.iter_mut().zip(players) {
            *row = match player {
                Some(player) => scores(*player).map(|score| Rating {
                    score, threshold: u32::from(player.epoch) * 50, eligible_rank: 0,
                }),
                None => [Rating::default(); KINDS],
            };
        }

        std::array::from_fn(|kind| {
            let mut eligible: Vec<usize> = players.iter().enumerate()
                .filter_map(|(index, player)| player.filter(|p| p.epoch_technologies[kind] == 4).map(|_| index))
                .collect();
            eligible.sort_by(|&left, &right| {
                self.ratings[right][kind].score.partial_cmp(&self.ratings[left][kind].score).unwrap_or(Ordering::Equal)
            });
            for (rank, &index) in eligible.iter().enumerate() {
                self.ratings[index][kind].eligible_rank = rank as u8 + 1;
            }

            let previous = self.holders[kind];
            if previous.is_some_and(|holder| now <= holder.expires && players[holder.owner as usize].is_some()) {
                return Award::Unchanged;
            }
            let winner = eligible.first().filter(|_| !disabled);
            let next = winner.map(|&owner| Holder {
                owner: owner as u8, epoch: players[owner].unwrap().epoch,
                expires: source_math::f(source_math::add(f64::from(now), POSSESSION_SECONDS)) as f32,
            });
            self.holders[kind] = next;
            match (previous, next) {
                (_, Some(current)) => Award::Assigned { previous, current },
                (Some(previous), None) => Award::Revoked(previous),
                (None, None) => Award::Unchanged,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(epoch: u8, epoch_technologies: [u8; KINDS]) -> Player {
        Player { epoch, epoch_technologies, adjustment: [0.0; KINDS], counters: Counters::default() }
    }

    #[test]
    fn factor_order_and_single_stores() {
        let mut player = fixture(15, [4, 0, 4]);
        player.adjustment = [-1.5, 0.25, -0.75];
        player.counters = Counters {
            researched: [7, 12, 17], mobile_kills: 32, military_units: 14, building_kills: 3,
            buildings_converted: 7, military_buildings: 6, economic_buildings: 5,
            trade_routes: 2, gathered_natural: 181, trade_gold: 93, territories: 3,
            roads_and_bridges: 11, humans: 26, stolen_natural: 237, cultural_units: 4,
            non_buildings_converted: 6, wonders: 2,
        };
        assert_eq!(scores(player).map(f32::to_bits), [0x42b28000, 0x4273cccc, 0x42e32c8b]);
    }

    #[test]
    fn threshold_is_reported_and_rank_only_counts_four_technologies() {
        let mut higher_score = fixture(4, [3, 5, 0]);
        higher_score.counters.mobile_kills = 500;
        let players = [Some(fixture(5, [4, 4, 0])), Some(higher_score), Some(fixture(7, [4, 4, 0]))];
        let mut crowns = Scoreboard::default();
        assert_eq!(crowns.update(19, 0.95, 0.05, false, &players), [Award::Unchanged; KINDS]);
        assert_eq!(crowns.update(20, 1.0, 0.05, false, &players)[0], Award::Assigned {
            previous: None, current: Holder { owner: 0, epoch: 5, expires: 301.0 },
        });
        assert_eq!(crowns.ratings.map(|r| r[0].eligible_rank), [1, 0, 2]);
        assert_eq!(crowns.ratings[0][0], Rating { score: 0.0, threshold: 250, eligible_rank: 1 });
        assert_eq!(crowns.ratings[1][0].score, 500.0);
        assert_eq!(crowns.holders[2], None);
    }

    #[test]
    fn deadline_equality_and_reassignment_use_the_next_due_update() {
        let mut players = [Some(fixture(5, [4, 0, 0])), Some(fixture(5, [4, 0, 0]))];
        let mut crowns = Scoreboard::default();
        crowns.update(20, 1.0, 0.05, false, &players);
        players[1].as_mut().unwrap().counters.mobile_kills = 50;
        assert_eq!(crowns.update(6020, 301.0, 0.05, false, &players)[0], Award::Unchanged);
        assert_eq!(crowns.update(6021, 301.05, 0.05, false, &players)[0], Award::Unchanged);
        assert!(matches!(crowns.update(6040, 302.0, 0.05, false, &players)[0],
            Award::Assigned { previous: Some(Holder { owner: 0, .. }), current: Holder { owner: 1, .. } }));
        players[1] = None;
        assert!(matches!(crowns.update(6060, 303.0, 0.05, true, &players)[0],
            Award::Revoked(Holder { owner: 1, .. })));
    }
}
