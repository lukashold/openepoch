//! Normalized construction health for new authenticated Valley City Centers.
use crate::defs::{DefId, TC};
use crate::game::{Ent, Game};
use crate::source_clock38::{add_positive_f64, divide_positive_f32_as_f64, store_positive_f32};

impl Game {
    pub(super) fn enable_valley_city_foundation(&mut self) -> Option<()> {
        let fixture = self.valley_fixture.as_mut()?;
        if self.net || !fixture.native_house_assigned_clearance { return None; }
        fixture.native_city_foundation = true;
        Some(())
    }

    pub(crate) fn uses_valley_city_foundation(&self, def: DefId) -> bool {
        def == TC && self.valley_fixture.as_ref().is_some_and(|fixture| fixture.native_city_foundation)
    }

    pub(crate) fn valley_city_awaiting_labor(&self, site: &Ent) -> bool {
        self.uses_valley_city_foundation(site.def) && site.built < 1.0 && site.prev == 0
    }

    pub(crate) fn valid_valley_city_foundation(&self, site: &Ent) -> bool {
        !self.uses_valley_city_foundation(site.def) ||
            ((0.0..=1.0).contains(&site.built) && site.prev <= 1 && (site.built < 1.0 || site.prev == 0))
    }

    pub(crate) fn advance_valley_city_foundation(site: &mut Ent, maximum: f32, seconds: f32, duration: f32, labor: bool) {
        if !(site.hp > 0.0 && site.built < 1.0 && seconds > 0.0 && duration > 0.0) { return; }
        // 007E31B3 starts labor. The immediate Imperial health event 00826280 does not.
        if labor { site.prev = 1; }
        let health = Self::valley_city_normalized(site, maximum);
        let work = divide_positive_f32_as_f64(seconds, duration);
        Self::set_valley_city_health(site, maximum, store_positive_f32(add_positive_f64(f64::from(health), work).min(1.0)));
    }

    pub(crate) fn damage_valley_city_foundation(&mut self, index: usize, amount: f32) -> bool {
        let site = self.ents[index];
        if !self.uses_valley_city_foundation(site.def) || site.built >= 1.0 { return false; }
        if site.hp <= 0.0 || amount <= 0.0 { return true; }
        let maximum = self.max_hp(site.team, site.def);
        let health = Self::valley_city_normalized(&site, maximum);
        // 00825FD8–0082602E stores absolute health before and after subtraction.
        let absolute = store_positive_f32(f64::from(health) * f64::from(maximum));
        let remaining = store_positive_f32((f64::from(absolute) - f64::from(amount)).max(0.0));
        let normalized = store_positive_f32(divide_positive_f32_as_f64(remaining, maximum));
        Self::set_valley_city_health(&mut self.ents[index], maximum, normalized);
        true
    }

    fn valley_city_normalized(site: &Ent, maximum: f32) -> f32 {
        // Reuse the stored source word while the portable HP projection is current.
        // A separate absolute health or max-HP adjustment is the next input.
        if site.built > 0.0 && site.built * maximum == site.hp {
            site.built
        } else {
            store_positive_f32(divide_positive_f32_as_f64(site.hp, maximum))
        }
    }

    fn set_valley_city_health(site: &mut Ent, maximum: f32, normalized: f32) {
        site.built = normalized;
        site.hp = normalized * maximum;
        if normalized >= 1.0 { site.prev = 0; }
    }
}
