//! Source construction health for explicitly enabled generated solo City Centers.
use crate::defs::{DefId, TC};
use crate::game::{Ent, Game};
use crate::save::checksum;
use crate::source_clock38::{add_positive_f64, divide_positive_f32_as_f64, store_positive_f32};

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"CSH1");

pub(crate) fn wrap(mut inner: Vec<u8>, enabled: bool) -> Vec<u8> {
    if !enabled || inner.is_empty() { return inner; }
    let mut bytes = Vec::with_capacity(inner.len() + 8);
    bytes.extend_from_slice(&MAGIC.to_le_bytes());
    bytes.append(&mut inner);
    bytes.extend_from_slice(&checksum(&bytes).to_le_bytes());
    bytes
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], bool)> {
    let Some(payload) = bytes.strip_prefix(&MAGIC.to_le_bytes()) else { return Some((bytes, false)); };
    let (body, tail) = bytes.split_at_checked(bytes.len().checked_sub(4)?)?;
    if body.len() <= 4 || checksum(body) != u32::from_le_bytes(tail.try_into().ok()?) { return None; }
    Some((&payload[..payload.len() - 4], true))
}

impl Game {
    pub(crate) fn configure_generated_city_foundations(&mut self) -> bool {
        if !self.supports_generated_city_foundations() || self.generated_city_foundations || self.tick != 0
            || !self.outbox.is_empty() || !self.inbox.is_empty()
            || self.ents[..self.high].iter().any(|site| site.alive && site.def == TC && site.built < 1.0)
        { return false; }
        self.generated_city_foundations = true;
        true
    }

    pub(crate) fn supports_generated_city_foundations(&self) -> bool {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.map.source_world.is_some() { return false; }
        !self.net && self.humans == 1 && self.bench == 0
    }

    pub(crate) fn uses_generated_city_foundation(&self, def: DefId) -> bool {
        self.generated_city_foundations && def == TC
    }

    pub(crate) fn generated_city_awaiting_labor(&self, site: &Ent) -> bool {
        self.uses_generated_city_foundation(site.def) && site.built < 1.0 && site.prev == 0
    }

    pub(crate) fn valid_generated_city_foundation(&self, site: &Ent) -> bool {
        !self.uses_generated_city_foundation(site.def) ||
            ((0.0..=1.0).contains(&site.built) && site.prev <= 1 && (site.built < 1.0 || site.prev == 0))
    }

    pub(crate) fn advance_generated_city_foundation(site: &mut Ent, maximum: f32, seconds: f32, duration: f32, labor: bool) {
        if !(site.hp > 0.0 && site.built < 1.0 && seconds > 0.0 && duration > 0.0) { return; }
        if labor { site.prev = 1; }
        let health = normalized_health(site, maximum);
        let increment = divide_positive_f32_as_f64(seconds, duration);
        set_health(site, maximum, store_positive_f32(add_positive_f64(f64::from(health), increment).min(1.0)));
    }

    pub(crate) fn damage_generated_city_foundation(&mut self, index: usize, amount: f32) -> bool {
        let site = self.ents[index];
        if !self.uses_generated_city_foundation(site.def) || site.built >= 1.0 { return false; }
        if site.hp <= 0.0 || amount <= 0.0 { return true; }
        let maximum = self.max_hp(site.team, site.def);
        let health = normalized_health(&site, maximum);
        // The original ordinary damage path stores absolute health on each
        // side of the subtraction before normalizing the result again.
        let absolute = store_positive_f32(f64::from(health) * f64::from(maximum));
        let remaining = if amount >= absolute { 0.0 } else {
            store_positive_f32(add_positive_f64(f64::from(absolute), -f64::from(amount)))
        };
        set_health(&mut self.ents[index], maximum, store_positive_f32(divide_positive_f32_as_f64(remaining, maximum)));
        true
    }
}

fn normalized_health(site: &Ent, maximum: f32) -> f32 {
    if site.built > 0.0 && site.built * maximum == site.hp { site.built }
    else { store_positive_f32(divide_positive_f32_as_f64(site.hp, maximum)) }
}

fn set_health(site: &mut Ent, maximum: f32, normalized: f32) {
    site.built = normalized;
    site.hp = normalized * maximum;
    if normalized >= 1.0 { site.prev = 0; }
}
