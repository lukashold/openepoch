//! Paid Valley construction reservations keep their original debit until completion or retirement.

use crate::{
    defs::{DefId, DEFS},
    game::{Ent, Game, MAXE},
    save::checksum,
};
use std::collections::BTreeMap;

const TAG: &[u8; 4] = b"CBP1";
const RECORD_BYTES: usize = 24;

#[derive(Clone, Copy)]
pub(crate) struct Payment {
    definition: DefId,
    cost: [u16; 9],
}

pub(crate) type Payments = BTreeMap<u32, Payment>;

fn pending(actor: &Ent) -> bool {
    actor.alive && actor.hp > 0.0 && (0.0..1.0).contains(&actor.built)
        && DEFS.get(actor.def as usize).is_some_and(|def| def.building)
}

impl Game {
    pub(crate) fn record_construction_payment(&mut self, index: usize, cost: [u16; 9]) {
        if self.map.source_world.is_some() {
            self.construction_payments.insert(self.h(index), Payment { definition: self.ents[index].def, cost });
        }
    }

    pub(crate) fn cancel_unfinished_construction(&mut self, index: usize) -> bool {
        let actor = self.ents[index];
        if self.map.source_world.is_none() || !pending(&actor) {
            return false;
        }
        if let Some(payment) = self.construction_payments.remove(&self.h(index)) {
            self.pay(actor.team, &payment.cost, -1.0);
        }
        // Retire before any worker can advance this foundation in the same step.
        self.ents[index].hp = 0.0;
        self.kill(index);
        self.sync_valley_atlas();
        self.prune_valley_routes();
        true
    }

    #[cfg(feature = "ee2-valley205-completed-refund")]
    pub(crate) fn refund_completed_valley_building(&mut self, index: usize) {
        use crate::defs::{BARRACKS, HOUSE, TEMPLE, TOWER};

        let actor = &self.ents[index];
        let stock = match actor.def {
            HOUSE => [0, 50, 0, 50, 0, 0, 0, 0, 0],
            BARRACKS => [0, 250, 0, 0, 0, 0, 0, 0, 0],
            TOWER => [0, 150, 0, 150, 0, 0, 0, 0, 0],
            TEMPLE if actor.ncargo == 0 => [0, 0, 0, 250, 0, 0, 0, 0, 0],
            _ => return,
        };
        if actor.built != 1.0 || !actor.hp.is_finite() || actor.hp <= 0.0 {
            return;
        }
        if self.net || self.winner != 0 || self.valley_fixture.is_none() || self.map.source_world.is_none()
            || self.territory_victory.is_some() || self.sole_survivor.is_some() || self.regicide.is_some()
            || self.koth.is_some() || self.crown_victory.is_some() || self.capitols.is_some() || self.hotspots.is_some()
        {
            return;
        }
        let team = actor.team;
        if self.players[team as usize].epoch != 0 || self.construction_cost(team, actor.def) != stock {
            return;
        }
        // The original stores the scaled quote as f32, then credits whole resources.
        for (slot, cost) in stock.into_iter().enumerate().filter(|&(_, cost)| cost != 0) {
            self.add_resource(team, slot, (f32::from(cost) * 0.1_f32).round());
        }
    }

    pub(crate) fn settle_construction_payments(&mut self) {
        let entities = &self.ents;
        self.construction_payments.retain(|&handle, payment| {
            let actor = &entities[handle as u16 as usize];
            pending(actor) && actor.def == payment.definition && u32::from(actor.gen) == handle >> 16
        });
    }

    pub(crate) fn hash_construction_payments(&self, mix: &mut impl FnMut(u32)) {
        if self.construction_payments.is_empty() { return; }
        mix(u32::from_le_bytes(*TAG));
        for (&handle, payment) in &self.construction_payments {
            mix(handle);
            mix(u32::from(payment.definition));
            for value in payment.cost { mix(u32::from(value)); }
        }
    }

    pub(crate) fn append_construction_payments(&self, save: &mut Vec<u8>) {
        if save.is_empty() || self.construction_payments.is_empty() { return; }
        let start = save.len();
        for (&handle, payment) in &self.construction_payments {
            save.extend_from_slice(&handle.to_le_bytes());
            save.extend_from_slice(&payment.definition.to_le_bytes());
            for value in payment.cost { save.extend_from_slice(&value.to_le_bytes()); }
        }
        let sum = checksum(&save[start..]);
        save.extend_from_slice(&(self.construction_payments.len() as u32).to_le_bytes());
        save.extend_from_slice(&sum.to_le_bytes());
        save.extend_from_slice(TAG);
    }

    pub(crate) fn restore_construction_payments(&mut self, payments: Payments) -> Option<()> {
        if !payments.is_empty() && self.map.source_world.is_none() { return None; }
        for (&handle, payment) in &payments {
            let actor = &self.ents[self.get(handle)?];
            if !pending(actor) || actor.def != payment.definition { return None; }
        }
        self.construction_payments = payments;
        Some(())
    }
}

pub(crate) fn split(save: &[u8]) -> Option<(&[u8], Payments)> {
    let mut payments = Payments::new();
    let Some(framed) = save.strip_suffix(TAG) else { return Some((save, payments)); };
    let (body, footer) = framed.split_at_checked(framed.len().checked_sub(8)?)?;
    let count = u32::from_le_bytes(footer[..4].try_into().ok()?) as usize;
    let stored = u32::from_le_bytes(footer[4..].try_into().ok()?);
    if !(1..=MAXE).contains(&count) { return None; }
    let (base, rows) = body.split_at_checked(body.len().checked_sub(count * RECORD_BYTES)?)?;
    if base.is_empty() || checksum(rows) != stored { return None; }
    for row in rows.chunks_exact(RECORD_BYTES) {
        let handle = u32::from_le_bytes(row[..4].try_into().ok()?);
        if (handle as u16 as usize) >= MAXE || payments.last_key_value().is_some_and(|(&last, _)| handle <= last) {
            return None;
        }
        let definition = u16::from_le_bytes(row[4..6].try_into().ok()?);
        let cost = std::array::from_fn(|i| u16::from_le_bytes([row[6 + 2 * i], row[7 + 2 * i]]));
        payments.insert(handle, Payment { definition, cost });
    }
    Some((base, payments))
}
