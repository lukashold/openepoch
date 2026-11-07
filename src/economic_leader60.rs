//! Economic leader effects consumed by paid production and trade delivery.
//! Use the shared leader effect cadence and spatial adapter for admissions.

use super::*;
use super::crowns54::score::source_math;
use std::collections::BTreeSet;

pub(crate) const ECONOMIC_LEADER: DefId = 88;
const MAGIC: [u8; 4] = *b"ELP1";
const MOBILIZATION_SECONDS: f32 = 45.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Membership {
    leader: u32,
    host: u32,
    owner: u8,
}

#[derive(Clone, Copy)]
struct Mobilization {
    member: Membership,
    deadline: f32,
    admitted: bool,
}

#[derive(Default)]
pub(crate) struct EconomicLeaderEffects {
    customs: BTreeSet<Membership>,
    mobilization: Vec<Mobilization>,
}

fn customs_host(def: DefId) -> bool { matches!(def, MARKET | DOCK) }

// Runnable UP aliases whose active source family has Produce on BareBoneBuilding.
fn production_host(def: DefId) -> bool {
    matches!(def, TC | BARRACKS | DOCK | AIRFIELD | MILL | STABLE | MARKET | TEMPLE | UNIVERSITY | HERC_FACILITY)
}

fn add_scale(value: f32, factor: f32) -> f32 {
    source_math::f(source_math::add(f64::from(value), source_math::sub(f64::from(factor), 1.0))) as f32
}

fn host_live(game: &Game, member: Membership, host_type: impl Fn(DefId) -> bool) -> bool {
    let Some(host) = game.get(member.host).map(|index| game.ents[index]) else { return false; };
    host.hp > 0.0 && host.built >= 1.0 && host.team == member.owner && host_type(host.def)
}

fn leader_live(game: &Game, member: Membership) -> bool {
    let Some(leader) = game.get(member.leader).map(|index| game.ents[index]) else { return false; };
    leader.def == ECONOMIC_LEADER && leader.hp > 0.0 && leader.team == member.owner && leader.stuck != BOARDED
}

fn customs_live(game: &Game, member: Membership) -> bool {
    game.players[member.owner as usize].epoch < 5 && leader_live(game, member) && host_live(game, member, customs_host)
}

pub(super) fn advance_production(progress: f32, seconds: f32, factor: f32) -> f32 {
    let delta = source_math::div(f64::from(seconds), f64::from(factor));
    source_math::f(source_math::add(f64::from(progress), delta)) as f32
}

impl Game {
    pub(crate) fn configure_economic_leader_effects(&mut self) -> bool {
        if self.tick != 0 || self.net || self.bench != 0 || !self.crown_leaders || self.military_leader_powers.is_none()
            || self.economic_leader_effects.is_some() {
            return false;
        }
        self.economic_leader_effects = Some(EconomicLeaderEffects::default());
        true
    }

    pub(crate) fn mobilization_ready(&self, index: usize) -> bool {
        let actor = &self.ents[index];
        self.economic_leader_effects.is_some() && !self.net && self.winner == 0 && actor.alive && actor.hp > 0.0
            && actor.def == ECONOMIC_LEADER && actor.stuck != BOARDED
            && self.players[actor.team as usize].epoch < 5 && self.leader_power_reserve(index) >= 1.0
    }

    pub(crate) fn mobilization_target(&self, index: usize) -> bool {
        let host = &self.ents[index];
        host.alive && host.hp > 0.0 && host.built >= 1.0 && host.team == self.local && production_host(host.def)
    }

    pub(crate) fn order_leader_mobilization(&mut self, index: usize, host: u32) {
        let member = Membership { leader: self.h(index), host, owner: self.ents[index].team };
        if !self.mobilization_ready(index) || !host_live(self, member, production_host) { return; }
        self.native_stop_attack(index);
        let caster = &mut self.ents[index];
        caster.order = O_LEADER_MOBILIZATION;
        caster.otgt = host;
        caster.tgt = NONE;
        caster.stuck = 0;
    }

    pub(super) fn do_leader_mobilization(&mut self, index: usize, caster: &mut Ent) {
        let host = caster.otgt;
        Self::set_idle(caster);
        if !self.mobilization_ready(index) { return; }
        let source = self.h(index);
        if self.economic_attach_mobilization(source, host, self.sim_previous_elapsed(), false) {
            self.spend_unit_reserve(source,1.0);
        }
    }

    fn economic_attach_mobilization(&mut self, leader: u32, host: u32, accepted_at: f32, admitted: bool) -> bool {
        let Some(index) = self.get(leader) else { return false; };
        let owner = self.ents[index].team;
        let member = Membership { leader, host, owner };
        if self.players[owner as usize].epoch >= 5 || !leader_live(self, member) || !host_live(self, member, production_host)
            || !accepted_at.is_finite() || !(0.0..=self.sim_elapsed()).contains(&accepted_at) { return false; }
        let deadline = source_math::f(source_math::add(f64::from(accepted_at), f64::from(MOBILIZATION_SECONDS))) as f32;
        if deadline < self.sim_previous_elapsed() { return false; }
        let Some(effects) = &mut self.economic_leader_effects else { return false; };
        if effects.mobilization.len() == MAXE { return false; }
        effects.mobilization.push(Mobilization { member, deadline, admitted });
        true
    }

    pub(super) fn economic_trade_gold(&self, host: usize, gold: f32) -> f32 {
        let Some(effects) = &self.economic_leader_effects else { return gold; };
        if gold <= 0.0 { return gold; }
        let handle = self.h(host);
        let factor = effects.customs.iter().filter(|&&member| member.host == handle && customs_live(self, member))
            .fold(1.0, |factor, _| add_scale(factor, 2.0));
        source_math::f(source_math::mul(f64::from(gold), f64::from(factor))) as f32
    }

    pub(super) fn economic_production_factor(&self, host: usize, produced: DefId) -> f32 {
        let Some(effects) = &self.economic_leader_effects else { return 1.0; };
        let actor = self.ents[host];
        let handle = self.h(host);
        let crown = self.live_crown_production_factor(actor.team, actor.def, produced);
        // Native paid production reads attached effects before the later area-manager expiry sweep.
        effects.mobilization.iter().filter(|effect| effect.admitted && effect.member.host == handle
            && host_live(self, effect.member, production_host))
            .fold(crown, |factor, _| add_scale(factor, 0.5)).max(0.05)
    }

    pub(super) fn tick_economic_leader_effects(&mut self) {
        let Some(mut effects) = self.economic_leader_effects.take() else { return; };
        let now = self.sim_previous_elapsed();
        effects.customs.retain(|&member| customs_live(self, member));
        effects.mobilization.retain(|effect| now <= effect.deadline && host_live(self, effect.member, production_host));
        for effect in &mut effects.mobilization {
            if !effect.admitted && self.get(effect.member.host).is_some_and(|index| self.leader_area_event_due(index)) {
                effect.admitted = true;
            }
        }
        for source in 0..self.high {
            let actor = &self.ents[source];
            if !actor.alive || actor.hp <= 0.0 || actor.def != ECONOMIC_LEADER || actor.stuck == BOARDED
                || self.players[actor.team as usize].epoch >= 5 || !self.leader_area_event_due(source) { continue; }
            let leader = self.h(source);
            effects.customs.retain(|member| member.leader != leader);
            for target in 0..self.high {
                let host = &self.ents[target];
                if host.alive && host.hp > 0.0 && host.built >= 1.0 && host.team == actor.team && customs_host(host.def)
                    && super::military_leader59::in_radius(actor, host, 5.0) {
                    effects.customs.insert(Membership { leader, host: self.h(target), owner: actor.team });
                }
            }
        }
        self.economic_leader_effects = Some(effects);
    }
}

impl EconomicLeaderEffects {
    fn words(&self, mut write: impl FnMut(u32)) {
        write(self.customs.len() as u32);
        write(self.mobilization.len() as u32);
        for member in &self.customs {
            for word in [member.leader, member.host, u32::from(member.owner)] { write(word); }
        }
        for effect in &self.mobilization {
            let member = effect.member;
            for word in [member.leader, member.host, u32::from(member.owner), effect.deadline.to_bits(),
                u32::from(effect.admitted)] { write(word); }
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        write(u32::from_le_bytes(MAGIC));
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() { return core; }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC);
        self.words(|word| bytes.extend_from_slice(&word.to_le_bytes()));
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<EconomicLeaderEffects>)> {
    if !bytes.starts_with(&MAGIC) { return Some((bytes, None)); }
    let end = bytes.len().checked_sub(4)?;
    let checksum = u32::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    if crate::save::checksum(&bytes[..end]) != checksum { return None; }
    let read = |offset: usize| -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(offset..offset+4)?.try_into().ok()?))
    };
    let customs = read(4)? as usize;
    let mobilization = read(8)? as usize;
    if customs > 4 * MAXE || mobilization > MAXE { return None; }
    let core = 12 + customs * 12 + mobilization * 20;
    if core >= end { return None; }
    let read_member = |offset| -> Option<Membership> {
        let owner = read(offset+8)?;
        let leader = read(offset)?;
        let host = read(offset+4)?;
        if owner >= 4 || leader == host || [leader, host].iter().any(|&handle| (handle & 0xffff) as usize >= MAXE) {
            return None;
        }
        Some(Membership { leader, host, owner: owner as u8 })
    };
    let mut effects = EconomicLeaderEffects::default();
    for offset in (12..12 + customs * 12).step_by(12) {
        if !effects.customs.insert(read_member(offset)?) { return None; }
    }
    for offset in (12 + customs * 12..core).step_by(20) {
        let deadline = f32::from_bits(read(offset+12)?);
        if !deadline.is_finite() || deadline < MOBILIZATION_SECONDS { return None; }
        let admitted = match read(offset+16)? { 0 => false, 1 => true, _ => return None };
        effects.mobilization.push(Mobilization { member: read_member(offset)?, deadline, admitted });
    }
    Some((&bytes[core..end], Some(effects)))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn enable(game: &mut Game) {
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        assert!(game.configure_economic_leader_effects());
    }

    #[test]
    fn mobilization_changes_live_paid_work_and_replays_the_expiry_boundary() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        enable(&mut game);
        let host = game.ents[..game.high].iter().position(|actor| actor.alive && actor.team == 0 && actor.def == TC).unwrap();
        let x = game.ents[host].x;
        let y = game.ents[host].y;
        let leader = game.spawn(ECONOMIC_LEADER, 0, x+1.0, y);
        let house = game.spawn(HOUSE, 0, x+6.0, y);
        let opponent = game.spawn(TC, 1, x+20.0, y);
        assert!(game.train(host, WORKER as QueueItem));
        let before = game.queue_payment(host,0).price;
        let limit = game.queue_time_limit(host,0);
        assert!(!game.economic_attach_mobilization(game.h(leader),game.h(house),0.0,true));
        assert!(!game.economic_attach_mobilization(game.h(leader),game.h(opponent),0.0,true));
        assert!(game.economic_attach_mobilization(game.h(leader),game.h(host),0.0,true));
        game.tick();
        assert_eq!(game.ents[host].qt.to_bits(),0.01f32.to_bits());
        assert_eq!(game.queue_payment(host,0).duration,limit);
        assert_eq!(game.queue_payment(host,0).price,before);
        game.tick();
        let expected = source_math::f(f64::from(0.01f32) + 2.0 * f64::from(DT)) as f32;
        assert_eq!(game.ents[host].qt.to_bits(),expected.to_bits());
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.state_hash(),restored.state_hash());
        restored.save();
        assert_eq!(game.savebuf,restored.savebuf);
        for state in [&mut game,&mut restored] {
            state.kill(leader);
            assert_eq!(state.economic_production_factor(host,WORKER),0.5);
            state.tick = 901;
            assert_eq!(state.sim_previous_elapsed(),45.0);
            state.tick_economic_leader_effects();
            assert_eq!(state.economic_production_factor(host,WORKER),0.5);
            state.tick += 1;
            assert_eq!(state.economic_production_factor(host,WORKER),0.5);
            state.tick_economic_leader_effects();
            assert_eq!(state.economic_production_factor(host,WORKER),1.0);
        }
        assert_eq!(game.state_hash(),restored.state_hash());
    }

    #[test]
    fn paid_job_accumulates_crown_and_mobilization_in_one_source_factor() {
        const WAR_GALLEY: DefId = 29;
        let mut game = Game::new(7, 0, 0, 1, 0);
        enable(&mut game);
        let base = game.production_time(0,WAR_GALLEY);
        game.researched[0][0] |= 0x0fff;
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(1,9)); // Rule the Waves
        let host = game.spawn(DOCK,0,30.5,30.5);
        let leader = game.spawn(ECONOMIC_LEADER,0,30.5,32.5);
        assert_eq!(game.production_time(0,WAR_GALLEY),base);
        assert_eq!(game.economic_production_factor(host,WAR_GALLEY),0.8);
        let accepted = game.sim_previous_elapsed();
        assert!(game.economic_attach_mobilization(game.h(leader),game.h(host),accepted,true));
        assert_eq!(game.economic_production_factor(host,WAR_GALLEY),0.3);
        assert!(game.economic_attach_mobilization(game.h(leader),game.h(host),accepted,true));
        assert_eq!(game.economic_production_factor(host,WAR_GALLEY),0.05);
    }

    #[test]
    fn normal_mobilization_order_spends_only_on_a_live_producer_and_resumes_before_admission() {
        use crate::cmd::C_LEADER_MOBILIZATION;
        let mut game = Game::new(7, 0, 0, 1, 0);
        enable(&mut game);
        let host = game.ents[..game.high].iter().position(|actor| actor.alive && actor.team == 0 && actor.def == TC).unwrap();
        let x = game.ents[host].x;
        let y = game.ents[host].y;
        let leader = game.spawn(ECONOMIC_LEADER, 0, x+24.0, y);
        let enemy = game.spawn(TC, 1, x+8.0, y);
        let source = game.h(leader);
        let target = game.h(host);
        game.powers.as_mut().unwrap().recharge_selected(source);
        game.apply(0,C_LEADER_MOBILIZATION,0.0,0.0,game.h(enemy) as i32,&[source]);
        assert_ne!(game.ents[leader].order,O_LEADER_MOBILIZATION);
        assert_eq!(game.leader_power_reserve(leader),1.0);
        assert!(game.train(host,WORKER as QueueItem));
        game.apply(0,C_LEADER_MOBILIZATION,0.0,0.0,target as i32,&[source]);
        assert_eq!(game.ents[leader].order,O_LEADER_MOBILIZATION);
        assert_eq!(game.leader_power_reserve(leader),1.0);
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        for state in [&mut game,&mut restored] {
            state.tick();
            assert!(state.leader_power_reserve(leader)<0.1);
            assert_ne!(state.ents[leader].order,O_LEADER_MOBILIZATION);
            for _ in 0..25 {
                if state.economic_production_factor(host,WORKER)==0.5 { break; }
                state.tick();
            }
            assert_eq!(state.economic_production_factor(host,WORKER),0.5);
            let progress = state.ents[host].qt;
            state.tick();
            assert_eq!(state.ents[host].qt,advance_production(progress,DT,0.5));
        }
        assert_eq!(game.state_hash(),restored.state_hash());
    }
}
