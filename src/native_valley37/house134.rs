//! Paid House health in the authenticated private Valley skirmish.
use crate::defs::{DefId, DEFS, HOUSE, LAND};
use crate::game::{Ent, Game, BOARDED};
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

    pub(super) fn enable_valley_house_movement(&mut self) -> Option<()> {
        let fixture = self.valley_fixture.as_mut()?;
        if !fixture.native_house_construction { return None; }
        fixture.native_house_movement = true;
        Some(())
    }

    pub(crate) fn uses_valley_house_movement(&self, def: DefId) -> bool {
        def == HOUSE && self.valley_fixture.as_ref().is_some_and(|fixture| fixture.native_house_movement)
    }

    pub(crate) fn valley_house_reserves_placement_only(&self, site: &Ent) -> bool {
        site.built == 0.0 && self.uses_valley_house_movement(site.def)
    }

    pub(crate) fn valley_house_reservation_overlaps(&self, bounds: (i32, i32, i32, i32)) -> bool {
        if !self.uses_valley_house_movement(HOUSE) { return false; }
        let (x, y, width, height) = bounds;
        self.ents[..self.high].iter().any(|site| {
            if !site.alive || !self.valley_house_reserves_placement_only(site) { return false; }
            let (sx, sy, sw, sh) = Self::building_bounds(site);
            x < sx + sw && sx < x + width && y < sy + sh && sy < y + height
        })
    }

    pub(crate) fn valley_house_footprint_clear(&self, site: &Ent) -> bool {
        let (x, y, width, height) = Self::building_bounds(site);
        !self.ents[..self.high].iter().any(|actor| {
            let definition = &DEFS[actor.def as usize];
            actor.alive && actor.hp > 0.0 && actor.stuck != BOARDED && !definition.building && definition.dom == LAND
                && (x as f32..(x + width) as f32).contains(&actor.x)
                && (y as f32..(y + height) as f32).contains(&actor.y)
        })
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
