//! Paid House health in the authenticated private Valley skirmish.
use crate::defs::{DefId, HOUSE};
use crate::game::{Ent, Game};
use crate::source_clock38::{add_positive_f64, divide_positive_f32_as_f64, store_positive_f32};

impl Game {
    pub(super) fn enable_valley_house_construction(&mut self) -> Option<()> {
        let fixture = self.valley_fixture.as_mut()?;
        if !fixture.native_city_footprints || fixture.query(6) != 1 || self.net { return None; }
        fixture.native_house_construction = true;
        Some(())
    }

    pub(crate) fn uses_valley_house_construction(&self, def: DefId) -> bool {
        def == HOUSE && self.valley_fixture.as_ref().is_some_and(|fixture| fixture.native_house_construction)
    }

    pub(crate) fn valley_house_awaiting_labor(&self, actor: &Ent) -> bool {
        actor.built == 0.0 && self.uses_valley_house_construction(actor.def)
    }

    pub(crate) fn advance_valley_house_health(site: &mut Ent, maximum: f32, seconds: f32, duration: f32) {
        if !(site.hp > 0.0 && site.built < 1.0 && seconds > 0.0 && duration > 0.0) { return; }

        // Keep the exact last normalized work word when combat health has not
        // changed. After a hit or max-HP change, derive it from current health.
        let normalized = if site.built > 0.0 && site.built * maximum == site.hp {
            site.built
        } else {
            store_positive_f32(divide_positive_f32_as_f64(site.hp, maximum))
        };
        let work = divide_positive_f32_as_f64(seconds, duration);
        site.built = store_positive_f32(add_positive_f64(f64::from(normalized), work).min(1.0));
        site.hp = site.built * maximum;
    }
}
