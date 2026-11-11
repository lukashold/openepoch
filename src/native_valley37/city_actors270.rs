//! Candidate actors for interactive City placement on the authenticated map.
use super::{city_minerals269::{BoxBound, PlacementBoxes}, fresh239::SCOUT};
use crate::{defs::{DEFS, HOUSE, TC, WORKER}, diplomacy::ALLIED, game::{BOARDED, Ent, Game}};

type TileBounds = (i32, i32, i32, i32);

pub(super) fn mobile_radius(actor: &Ent) -> Option<f32> {
    match actor.def {
        WORKER => Some(0.125),
        SCOUT => Some(0.4),
        _ => None,
    }
}

fn stationary_boxes(game: &Game, actor: &Ent) -> Option<PlacementBoxes> {
    let (footprint, undertexture, offset_x, open_sides) = match actor.def {
        HOUSE => ([3.0; 2], [1.5; 2], 0.0, 0),
        TC => ([4.0, 3.5], [4.0, 3.0], 0.25, 1),
        _ => return None,
    };
    let origin = [actor.x, actor.y, game.map.height(actor.x, actor.y)];
    let under_construction = actor.built < 1.0;
    let height = if under_construction { 2.0 } else { 1.0 };
    let make_box = |size, offset| {
        let bound = BoxBound::new(origin, size, height, actor.rot, offset);
        // 00871DD0 selects the alternate Type of a live Construction
        // record and then clears the factory's vertical center offset.
        if under_construction { bound.with_center_z(origin[2]) } else { bound }
    };
    Some(PlacementBoxes {
        outer: make_box(footprint, 0.0),
        inner: make_box(undertexture, offset_x),
        open_sides,
    })
}

fn portable_footprint_overlaps(actor: &Ent, (x, y, w, h): TileBounds) -> bool {
    let (ax, ay, aw, ah) = Game::building_bounds(actor);
    let (left, right) = (x.max(ax), (x + w).min(ax + aw));
    let (top, bottom) = (y.max(ay), (y + h).min(ay + ah));
    (top..bottom).any(|ty| (left..right).any(|tx| Game::building_blocks_tile(actor, tx, ty)))
}

impl Game {
    fn rect_mobile_blocks_city(&self, team: u8, actor: &Ent, outer: BoxBound) -> bool {
        let Some(radius) = mobile_radius(actor) else { return false; };
        if actor.team == team || actor.stuck == BOARDED || usize::from(actor.team) >= self.nplayers
        {
            return false;
        }
        // 007E3750 ignores these foreign units for the AI owner's
        // outgoing alliance toward the payer. Human allies still block.
        if self.rect_mobile_ally_of_site(actor.team, team) {
            return false;
        }
        let z = self.map.height(actor.x, actor.y) + 0.5;
        // Both cached vertical capsules have extent .5. An outer hit
        // never enters the Box-only clearance.
        outer.overlaps_upright_capsule([actor.x, actor.y, z], radius, 0.5)
    }

    pub(super) fn rect_mobile_ally_of_site(&self, actor_owner: u8, payer: u8) -> bool {
        usize::from(actor_owner) < self.nplayers
            && self.players[usize::from(actor_owner)].ai
            && self.relation(actor_owner, payer) == ALLIED
    }

    pub(crate) fn rect_city_actor_placement_blocked(&self, team: u8, site: &Ent, bounds: TileBounds) -> Option<bool> {
        if !self.rect_native_city_collision(site) {
            return None;
        }
        let city = self.rect_city_placement_boxes(site);
        Some(self.ents[..self.high].iter().any(|actor| {
            if !actor.alive || actor.hp <= 0.0 {
                return false;
            }
            if !DEFS[actor.def as usize].building {
                return self.rect_mobile_blocks_city(team, actor, city.outer);
            }
            // The original stationary foreign Construction check skips
            // the site before its first labor. Same-owner reservations count.
            if actor.team != team && actor.built == 0.0 {
                return false;
            }
            if let Some(existing) = stationary_boxes(self, actor) {
                return city.blocks(existing);
            }
            portable_footprint_overlaps(actor, bounds)
        }))
    }
}
