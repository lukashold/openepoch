//! UP solo Regicide uses the native Conquest clause and the King ownership test.

use crate::{
    defs::{Def, A_INF, AIR, LAND, NOW, SEA, SUB, TC, W_MELEE, Wpn},
    ee2_sole_survivor::{SoleSurvivor, IDLE},
    game::{Game, DT},
    ids::{DefId, DEF_NONE},
    pack8_save,
};

pub(crate) const KING: DefId = 80;
pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"VCR1");
const HEADER: usize = 20;
const SOLE: u32 = 0x100;

pub(crate) const fn append_king(roster: [Def; 80]) -> [Def; 81] {
    let mut defs = [Def {
        name: "König",
        epoch: 0, until: 15, dom: LAND, building: false, size: 0.45,
        hp: 600.0, speed: 1.15, sight: 5.0, armor: A_INF,
        cost: [0; 3], time: 0.0, pop: 0, made_at: DEF_NONE, flags: 0,
        w: [Wpn { range: 0.8, dmg: 25.0, cd: 2.0, kind: W_MELEE, targets: LAND | SEA | AIR | SUB, splash: 0.0 }, NOW],
    }; 81];
    let mut id = 0;
    while id < roster.len() {
        defs[id] = roster[id];
        id += 1;
    }
    defs
}

// The active CSV's nonzero replacements are applied after its linked
// UpgradeAbilities. At Epoch XI the range changes while speed retains Epoch VI.
pub(crate) fn king_modifiers(epoch: u8, row: &mut [f32; 19]) {
    row[0] = 600.0 + 20.0 * f32::from(epoch);
    row[1] = 25.0 + 2.0 * f32::from(epoch);
    row[2] = if epoch < 5 { 1.15 } else { 3.0 };
    row[6] = if epoch < 10 { 0.0 } else { 5.0 - 0.8 };
}

#[derive(Clone, Copy)]
pub(crate) struct Regicide {
    pub(crate) state: SoleSurvivor,
    pub(crate) sole: bool,
}

impl Game {
    pub(crate) fn configure_match_regicide(
        &mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32, sole: u32,
    ) -> bool {
        if self.net || self.humans != 1 || self.local != 0 || self.nplayers < 2
            || self.free.len() < self.nplayers || sole > 1
        {
            return false;
        }
        let mut starts = [(0.0, 0.0); 4];
        for (seat, point) in starts[..self.nplayers].iter_mut().enumerate() {
            let Some(city) = self.ents[..self.high].iter()
                .find(|e| e.alive && e.def == TC && e.team as usize == seat) else {
                return false;
            };
            let Some(place) = self.regicide_spawn_point(city) else {
                return false;
            };
            *point = place;
        }
        if !self.configure_match_epoch_range(civs, pop, supplies, start, end) {
            return false;
        }
        self.regicide = Some(Regicide {
            state: SoleSurvivor { eliminated: 0, deadline: IDLE },
            sole: sole != 0,
        });
        for (seat, &(x, y)) in starts[..self.nplayers].iter().enumerate() {
            self.spawn(KING, seat as u8, x, y);
        }
        self.build_grid();
        self.update_fog();
        true
    }

    pub(crate) fn king_owners(&self) -> u8 {
        let mut mask = 0;
        for actor in &self.ents[..self.high] {
            if actor.alive && actor.hp > 0.0
                && (actor.def == KING || (0..actor.ncargo as usize)
                    .any(|slot| actor.cargo[slot] == KING && actor.cargo_hp[slot] > 0))
            {
                mask |= 1 << actor.team;
            }
        }
        mask
    }

    pub(crate) fn fullmap_king_markers(&self) -> impl Iterator<Item = (u8, f32, f32, bool)> + '_ {
        (0..self.nplayers as u8).filter_map(move |owner| {
            if !self.players[owner as usize].alive {
                return None;
            }
            self.ents[..self.high].iter().enumerate().find_map(|(index, actor)| {
                if !actor.alive || actor.hp <= 0.0 || actor.team != owner {
                    return None;
                }
                let stationed = (0..actor.ncargo as usize)
                    .any(|slot| actor.cargo[slot] == KING && actor.cargo_hp[slot] > 0);
                // 00761FD0 follows the King to its garrison host and, by default,
                // reveals that mark without the free-standing actor's LOS test.
                (stationed || (actor.def == KING && self.visible(index)))
                    .then_some((owner, actor.x, actor.y, stationed))
            })
        })
    }

    pub(crate) fn poll_regicide_outcome(&mut self, mut rule: Regicide) {
        let candidates = rule.state.active(self.nplayers);
        let mut survivors = candidates;
        for (seat, player) in self.players[..self.nplayers].iter().enumerate() {
            if !player.alive {
                survivors &= !(1 << seat);
            }
        }
        let allied = crate::ee2_conquest::survivors_are_allied(self);
        let mut won = false;
        if allied {
            if rule.sole && survivors.count_ones() > 1 {
                self.poll_sole_disband(&mut rule.state, candidates);
            } else {
                won = true;
            }
        } else {
            if rule.sole && survivors.count_ones() == 2 && survivors != candidates {
                self.clear_sole_diplomacy_offers();
            }
            // VCRegicide+50 is reached by 005B3110 only for a nonallied pair.
            survivors &= self.king_owners();
        }
        rule.state.eliminated |= candidates & !survivors;
        for (seat, player) in self.players[..self.nplayers].iter_mut().enumerate() {
            player.alive = survivors & (1 << seat) != 0;
        }
        let local_alive = survivors & (1 << self.local) != 0;
        if !local_alive {
            self.winner = 2;
        } else if won {
            self.winner = 1;
        }
        self.regicide = Some(rule);
    }

    pub(crate) fn restore_regicide(&mut self, rule: Regicide, outcome: u8) -> Option<()> {
        let state = rule.state;
        let now = self.tick as f32 * DT;
        let seats = (1u8 << self.nplayers) - 1;
        if self.net || self.humans != 1 || self.nplayers < 2
            || self.territory_victory.is_some() || self.sole_survivor.is_some()
            || state.eliminated & !seats != 0
            || (state.deadline != IDLE
                && (!rule.sole || !(10.0..=now + 10.0).contains(&state.deadline)))
            || (self.tick == 0 && (outcome != 0 || state.eliminated != 0 || state.deadline != IDLE
                || self.king_owners() != seats))
            || state.eliminated(0) != (outcome == 2)
            || (outcome == 1 && rule.sole && state.active(self.nplayers) != 1)
            || self.players[..self.nplayers].iter().enumerate()
                .any(|(seat, player)| state.eliminated(seat) && player.alive)
        {
            return None;
        }
        self.regicide = Some(rule);
        self.winner = outcome;
        Some(())
    }
}

fn word(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, game: &Game) -> Vec<u8> {
    let Some(rule) = game.regicide else { return payload; };
    if payload.is_empty() {
        return payload;
    }
    let flags = u32::from(rule.state.eliminated) | if rule.sole { SOLE } else { 0 };
    let mut out = Vec::with_capacity(HEADER + payload.len() + 4);
    for value in [MAGIC, rule.state.deadline.to_bits(), flags, game.rule_save_outcome().into(), payload.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], Option<(Regicide, u8)>)> {
    if word(buf, 0) != Some(MAGIC) {
        return Some((buf, None));
    }
    let flags = word(buf, 8)?;
    if flags & !(SOLE | 0xf) != 0 {
        return None;
    }
    let state = SoleSurvivor { eliminated: flags as u8, deadline: f32::from_bits(word(buf, 4)?) };
    let outcome = u8::try_from(word(buf, 12)?).ok().filter(|&value| value <= 2)?;
    let end = HEADER.checked_add(word(buf, 16)? as usize)?;
    if end.checked_add(4)? != buf.len() || pack8_save::checksum(buf.get(..end)?) != word(buf, end)? {
        return None;
    }
    match word(buf, HEADER)? {
        pack8_save::MAGIC | crate::match_epoch_range13::MAGIC | crate::ai9_game_save::MAGIC => {}
        _ => return None,
    }
    Some((buf.get(HEADER..end)?, Some((Regicide { state, sole: flags & SOLE != 0 }, outcome))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::{FORTRESS, WORKER}, diplomacy::ALLIED, game::NOIDX, map::tile_of};

    fn start(opponents: u32, epoch: u32, sole: u32) -> Game {
        let mut game = Game::new(7, opponents, 0, 1, 0);
        let civs = if opponents == 1 { 0xee10 } else { 0xe210 };
        assert!(game.configure_match_regicide(civs, 1000, 2, epoch, 14, sole));
        for player in &mut game.players[..game.nplayers] { player.ai = false; }
        game
    }

    fn king(game: &Game, owner: u8) -> usize {
        game.ents[..game.high].iter().position(|e| e.alive && e.def == KING && e.team == owner).unwrap()
    }

    #[test]
    fn native_start_creates_one_movable_king_per_seat_on_valid_land_in_every_epoch() {
        for epoch in 0..15 {
            let mut game = start(1, epoch, 1);
            assert_eq!(game.king_owners(), 3);
            assert_eq!(game.ents[..game.high].iter().filter(|e| e.alive && e.def == KING).count(), 2);
            for owner in 0..2 {
                let e = game.ents[king(&game, owner)];
                assert!(game.map.pass[0][tile_of(e.x, e.y)]);
                assert_eq!(e.hp, 600.0 + 20.0 * epoch as f32);
                assert_eq!(game.effective_weapon_damage(owner, KING, 25.0), 25.0 + 2.0 * epoch as f32);
            }
            game.save();
            assert!(Game::load(&game.savebuf).is_some());
        }
    }

    #[test]
    fn hostile_king_loss_marks_defeat_before_the_next_winner_poll() {
        let mut game = start(1, 0, 1);
        let rival = king(&game, 1);
        game.ents[rival].hp = 0.0;
        game.tick();
        assert!(!game.players[1].alive && game.winner == 0);
        assert!(game.ents[..game.high].iter().any(|e| e.alive && e.team == 1 && e.def == WORKER));
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        game.tick();
        loaded.tick();
        assert_eq!(game.winner, 1);
        assert_eq!(game.state_hash(), loaded.state_hash());
        loaded.save();
        assert_eq!(Game::load(&loaded.savebuf).unwrap().winner, 1);
    }

    #[test]
    fn shared_conquest_branch_precedes_king_loss_when_all_remaining_seats_are_allied() {
        for sole in 0..2 {
            let mut game = start(2, 0, sole);
            game.diplomacy.relation[1] = ALLIED;
            game.diplomacy.relation[4] = ALLIED;
            let rival = king(&game, 1);
            game.ents[rival].hp = 0.0;
            for e in &mut game.ents[..game.high] { if e.team == 2 { e.hp = 0.0; } }
            game.tick();
            assert!(game.players[1].alive && !game.players[2].alive);
            assert_eq!(game.winner, if sole == 0 { 1 } else { 0 });
            if sole == 1 {
                for _ in 0..200 { game.tick(); }
                assert!(game.allied(0, 1));
                game.tick();
                assert!(game.hostile(0, 1) && game.players[1].alive);
                game.tick();
                assert!(!game.players[1].alive && game.winner == 0);
                game.tick();
                assert_eq!(game.winner, 1);
            }
        }
    }

    #[test]
    fn king_can_hide_in_a_fortress_and_loss_of_the_host_counts_as_defeat() {
        let mut game = start(1, 5, 1);
        let index = king(&game, 0);
        let e = game.ents[index];
        let fort = game.spawn(FORTRESS, 0, e.x, e.y);
        assert_ne!(fort, NOIDX);
        game.order_board(&[index], fort);
        game.tick();
        assert_eq!(game.ents[fort].ncargo, 1);
        assert_eq!(game.ents[fort].cargo[0], KING);
        assert_eq!(game.king_owners(), 3);
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        loaded.ents[fort].hp = 0.0;
        loaded.tick();
        assert_eq!(loaded.winner, 2);
    }

    #[test]
    fn fullmap_crown_follows_an_enemy_garrison_beyond_line_of_sight() {
        let mut game = start(1, 5, 1);
        let rival = king(&game, 1);
        let actor = game.ents[rival];
        let tile = tile_of(actor.x, actor.y);
        assert_eq!(game.fullmap_king_markers().map(|entry| entry.0).collect::<Vec<_>>(), [0]);
        game.team_vis[tile] = 255;
        assert!(game.fullmap_king_markers().any(|entry| entry == (1, actor.x, actor.y, false)));

        game.team_vis[tile] = 0;
        let fort = game.spawn(FORTRESS, 1, actor.x, actor.y);
        assert_ne!(fort, NOIDX);
        game.order_board(&[rival], fort);
        game.tick();
        assert!(!game.visible(fort));
        assert!(game.fullmap_king_markers().any(|entry| entry == (1, actor.x, actor.y, true)));
        game.save();
        let restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(restored.fullmap_king_markers().collect::<Vec<_>>(), game.fullmap_king_markers().collect::<Vec<_>>());
        game.players[1].alive = false;
        assert_eq!(game.fullmap_king_markers().map(|entry| entry.0).collect::<Vec<_>>(), [0]);
    }
}
