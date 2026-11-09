//! Negotiated multiplayer controller departure and its world outcome.

use crate::game::Game;
use crate::defs::FARM;

const MAGIC: u32 = u32::from_le_bytes(*b"MPR1");
const HEADER: usize = 24;
const TEAM_BITS: u8 = 0b11;

#[derive(Clone, Copy)]
pub(crate) struct Resignation {
    policy: u8,
    departed: u8,
    defeated: u8,
    finished: bool,
}

impl Resignation {
    pub(crate) fn observer(self, team: u8) -> bool { self.departed & (1 << team) != 0 }

    pub(crate) fn defeated(self, team: usize) -> bool { self.defeated & (1 << team) != 0 }
    pub(crate) fn defeated_mask(self) -> u8 { self.defeated }

    fn humans(self, game: &Game) -> usize {
        (0..game.humans as usize).filter(|&seat| game.players[seat].alive &&
            !game.players[seat].ai && !self.observer(seat as u8)).count()
    }

    fn words(self) -> [u32; 4] {
        [self.policy.into(), self.departed.into(), self.defeated.into(), u32::from(self.finished)]
    }

    fn active_shared_owner(self) -> bool { self.words() == [1, 0, 0, 0] }

    pub(crate) fn hash(self, mut mix: impl FnMut(u32)) {
        mix(MAGIC);
        for word in self.words() { mix(word); }
    }
}

impl Game {
    pub(crate) fn configure_shared_owner_termination(&mut self) {
        self.net_resignation = Some(Resignation { policy: 1, departed: 0, defeated: 0, finished: false });
    }

    pub(crate) fn network_resume_eligible(&self) -> bool {
        self.grouped_coop.is_none() && self.network_resume_world_active()
            && (2..=4).contains(&self.humans)
            && (0..self.humans).all(|team| self.network_departure_allowed(team as u8))
    }

    pub(crate) fn shared_owner_resume_eligible(&self) -> bool {
        self.network_resume_world_active() && self.humans == 1 && self.nplayers == 2 && self.n_ai == 1
            && self.players[0].alive && !self.players[0].ai && self.players[1].ai
            && self.net_resignation.is_some_and(Resignation::active_shared_owner)
    }

    pub(crate) fn network_resume_world_active(&self) -> bool {
        self.net && self.bench == 0 && (1..u32::MAX).contains(&self.tick) && self.winner == 0
            && self.network_resume_tempo.is_none() && self.source_clock.is_some_and(|clock| clock.network)
    }

    pub(crate) fn configure_network_resignation(&mut self, policy: u32) -> bool {
        if !self.net || !(2..=4).contains(&self.humans) || self.bench != 0 || self.tick != 0 || policy >= 8 ||
            self.net_resignation.is_some() || !self.outbox.is_empty() || !self.inbox.is_empty() { return false; }
        self.net_resignation = Some(Resignation { policy: policy as u8, departed: 0, defeated: 0, finished: false });
        true
    }

    pub(crate) fn network_observer(&self, team: u8) -> bool {
        self.net_resignation.is_some_and(|state| state.observer(team)) ||
            self.net && self.capitols.is_some_and(|rule| rule.state.eliminated(team as usize))
    }

    pub(crate) fn network_match_finished(&self) -> bool {
        self.net_resignation.is_some_and(|state| state.finished)
    }

    pub(crate) fn network_departure_allowed(&self, team: u8) -> bool {
        self.humans > 1 && team < self.humans as u8 && self.players[team as usize].alive && !self.players[team as usize].ai &&
            self.net_resignation.is_some_and(|state| !state.finished && !state.observer(team))
    }

    pub(crate) fn apply_network_departure(&mut self, team: u8) {
        if !self.network_departure_allowed(team) { return; }
        let mut state = self.net_resignation.unwrap();
        let seat = team as usize;
        let last_human = state.humans(self) == 1;
        state.departed |= 1 << team;
        if last_human || state.policy < 2 {
            state.defeated |= 1 << team;
            self.players[seat].alive = false;
        } else {
            // The source difficulty is retained by the lobby policy. The
            // existing skirmish controller runs the replacement seat.
            self.players[seat].ai = true;
        }
        self.net_resignation = Some(state);
        if !last_human && state.policy == 0 {
            let handles: Vec<_> = (0..self.high).filter(|&index| {
                let actor = self.ents[index];
                // 00872E70 excludes the Farm prefix for resource-only
                // actors with no instance in the protected ability slot.
                actor.alive && actor.team == team && actor.hp > 0.0 && actor.def != FARM
            }).map(|index| self.h(index)).collect();
            for handle in handles {
                if let Some(index) = self.get(handle) { self.kill(index); }
            }
            self.rebuild_territories();
            self.void_landless_alliances();
            self.prune_territory_offers();
        }
        self.refresh_display_fog();
    }

    pub(crate) fn finish_network_match(&mut self) -> bool {
        let Some(mut state) = self.net_resignation else { return false; };
        if state.finished { return true; }
        self.refresh_match_alive();
        self.release_inactive_territories();
        let crown_winner = self.network_crown_winning_seat();
        let resolved = if let Some(rule) = self.capitols {
            self.poll_network_capitols(rule)
        } else {
            crown_winner.is_some() || crate::ee2_conquest::survivors_are_allied(self)
        };
        if !resolved && state.humans(self) != 0 { return false; }
        state.finished = true;
        self.net_resignation = Some(state);
        self.winner = self.project_network_outcome(state, crown_winner);
        true
    }

    fn project_network_outcome(&self, state: Resignation, crown_winner: Option<usize>) -> u8 {
        let seat = self.local as usize;
        let won = crown_winner.map_or(self.players[seat].alive, |winner| winner == seat);
        if won && !state.observer(self.local) { 1 } else { 2 }
    }

    pub(crate) fn wrap_network_resignation(&self, core: Vec<u8>) -> Vec<u8> {
        let Some(state) = self.net_resignation else { return core; };
        if core.is_empty() { return core; }
        let mut out = Vec::with_capacity(HEADER + core.len() + 4);
        for word in std::iter::once(MAGIC).chain(state.words()).chain(std::iter::once(core.len() as u32)) {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend(core);
        out.extend_from_slice(&crate::pack8_save::checksum(&out).to_le_bytes());
        out
    }

    pub(crate) fn restore_network_resignation(&mut self, state: Option<Resignation>) -> Option<()> {
        if self.net && self.humans == 1 {
            let state = state.filter(|state| state.active_shared_owner())?;
            if self.solo_resigned || self.bench != 0 { return None; }
            self.net_resignation = Some(state);
            self.refresh_display_fog();
            return Some(());
        }
        let Some(state) = state else { return Some(()); };
        if !self.net || !(2..=4).contains(&self.humans) || self.solo_resigned || self.bench != 0 { return None; }
        for seat in 0..self.humans as usize {
            let player = &self.players[seat];
            if state.defeated(seat) && (player.alive || player.ai) { return None; }
            if state.observer(seat as u8) && !state.defeated(seat) && (state.policy < 2 || !player.ai) { return None; }
            if !state.observer(seat as u8) && player.ai { return None; }
        }
        let crown_winner = state.finished.then(|| self.network_crown_winning_seat()).flatten();
        if state.finished && crown_winner.is_none() && state.humans(self) != 0 && !crate::ee2_conquest::survivors_are_allied(self) { return None; }
        self.net_resignation = Some(state);
        if state.finished {
            self.winner = self.project_network_outcome(state, crown_winner);
        }
        self.refresh_display_fog();
        Some(())
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<Resignation>)> {
    split_for_owners(bytes, TEAM_BITS)
}

pub(crate) fn split_for_owners(bytes: &[u8], team_bits: u8) -> Option<(&[u8], Option<Resignation>)> {
    let word = |offset: usize| Some(u32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?));
    if word(0) != Some(MAGIC) { return Some((bytes, None)); }
    let policy = u8::try_from(word(4)?).ok().filter(|&value| value < 8)?;
    let departed = u8::try_from(word(8)?).ok().filter(|&value| value & !team_bits == 0)?;
    let defeated = u8::try_from(word(12)?).ok().filter(|&value| value & !departed == 0)?;
    let finished = match word(16)? { 0 => false, 1 => true, _ => return None };
    let end = HEADER.checked_add(word(20)? as usize)?;
    if end <= HEADER || end.checked_add(4)? != bytes.len() ||
        crate::pack8_save::checksum(bytes.get(..end)?) != word(end)? { return None; }
    Some((bytes.get(HEADER..end)?, Some(Resignation { policy, departed, defeated, finished })))
}
