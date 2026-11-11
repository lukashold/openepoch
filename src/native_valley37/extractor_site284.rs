//! Extractor placement on authenticated World0 resource actors.
use super::{active_game, population234::UnitKind};
use crate::{
    cmd::C_RECT_EXTRACTOR,
    defs::{DefId, OIL_RIG, URANIUM_MINE},
    game::Game,
    placement::{PlacementSite, ROTATION_STEPS},
};

// 0085F890 stores sqrt(2)/2, then 00861730 uses half that word.
const SITE_RADIUS: f64 = f32::from_bits(0x3eb5_04f3) as f64;
// Installed validator context +74/+84, before the broad-phase padding.
pub(crate) const OIL_BOX_WIDTH: f32 = 2.0 * f32::from_bits(0x3f86_7c07);

fn resource_type(def: DefId) -> Option<u8> {
    match def {
        OIL_RIG => Some(7),
        URANIUM_MINE => Some(8),
        _ => None,
    }
}

impl Game {
    pub(crate) fn uses_rect_extractor_sites(&self, def: DefId) -> bool {
        self.rect_population.is_some() && resource_type(def).is_some()
    }

    pub(crate) fn rect_extractor_resource(&self, def: DefId, x: f32, y: f32) -> Option<u32> {
        let resource = resource_type(def)?;
        if !self.map.layout.contains_point(x, y) { return None; }
        let at = [x, y, self.map.height(x, y)];
        self.rect_population.as_ref()?.source().units().iter()
            .filter(|unit| unit.kind == UnitKind::Mineral && unit.resource == resource && unit.owner() == 0)
            .find(|unit| {
                let squared = at.into_iter().zip(unit.xyz)
                    .map(|(a, b)| (f64::from(a) - f64::from(b)).powi(2)).sum::<f64>();
                squared < SITE_RADIUS * SITE_RADIUS
            }).map(|unit| unit.packed_id)
    }

    pub(crate) fn rect_extractor_site(&self, def: u32, x: f32, y: f32, turn: u32) -> Option<PlacementSite> {
        let def = DefId::try_from(def).ok()?;
        if !self.uses_rect_extractor_sites(def) || turn >= ROTATION_STEPS
            || !self.map.layout.contains_point(x, y)
        { return None; }
        Some(PlacementSite::valley_cursor(def, x, y, Self::angle_for_turn(turn)))
    }

    pub(crate) fn apply_rect_extractor(&mut self, team: u8, x: f32, y: f32, packed: u32, ids: &[usize]) {
        if team != self.local || self.winner != 0 { return; }
        if let Some(site) = self.rect_extractor_site(packed & 0xffff, x, y, packed >> 16) {
            self.begin_construction_at(team, site, ids, true);
        }
    }
}

#[no_mangle]
pub extern "C" fn native_rect_extractor_error(def: u32, x: f32, y: f32, turn: u32) -> u32 {
    let Some(site) = active_game().and_then(|game| game.rect_extractor_site(def, x, y, turn)) else { return 1; };
    let game = crate::g();
    let ids = crate::selected(game);
    game.placement_error_at(game.local, site, &ids, true)
}

#[no_mangle]
pub extern "C" fn cmd_rect_extractor(def: u32, x: f32, y: f32, turn: u32) -> u32 {
    if !active_game().is_some_and(|game| game.winner == 0)
        || native_rect_extractor_error(def, x, y, turn) != 0
    { return 0; }
    let game = crate::g();
    let handles = crate::handles(game, &crate::selected(game));
    game.submit(C_RECT_EXTRACTOR, x, y, (def | (turn << 16)) as i32, &handles);
    1
}
