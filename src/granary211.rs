//! The enabled UP Granary's ordinary Citizen construction and territory effect.

use crate::{
    defs::{Def, DefId, A_BLD, BUILT_BY_WORKER, LAND, NOW, WORKER},
    ee2_units::UnitRow,
    game::{Ent, Game},
    map::tile_of,
};

pub(crate) const ID: DefId = crate::fishing81::FISHING_SHIP + 1;
pub(crate) const SHAPE: [f32; 2] = [3.0, 4.5];
pub(crate) const PEER_SAVE_FLAG: u32 = 1 << 22;
const STOCK: [u16; 9] = [0, 750, 0, 500, 600, 0, 0, 0, 0];
const DEFINITION: Def = Def {
    name: "Windmühle (1.6)", epoch: 1, until: 15, dom: LAND, building: true, size: 4.5,
    hp: 3000.0, speed: 0.0, sight: 6.0, armor: A_BLD, cost: [0, 750, 0], time: 50.0,
    pop: 0, made_at: BUILT_BY_WORKER, flags: 0, w: [NOW; 2],
};

pub(crate) const fn append(existing: [Def; ID as usize]) -> [Def; ID as usize + 1] {
    let mut definitions = [DEFINITION; ID as usize + 1];
    let mut index = 0;
    while index < existing.len() {
        definitions[index] = existing[index];
        index += 1;
    }
    definitions
}

const EARLY: UnitRow = UnitRow {
    id: ID, epoch: 1, until: 5, source_epoch: 2, source_unit: "Granary",
    source_visual: "Granary_2", source_host: "Citizen", identity_verified: true, model_verified: true,
    hp: Some(3000.0), buildtime: Some(50.0), cost9: Some(STOCK), pop: None,
    pop_capacity: None, sight: Some(6.0), raw_size: Some(SHAPE), raw_footprint: Some(SHAPE),
    raw_speed: None, gather: None, farm_rate: None, carry: None, farm_carry: None, repair_rate: None,
    damage: None, range: None, reload: None, garrison_slots: None, generation: None,
    generation_resource: None, garrison_heal: None, garrison_attack_bonus: None,
    garrison_size: None, fortress_garrison: false, attack_missile: None,
};
const ROWS: [UnitRow; 4] = [
    EARLY,
    UnitRow { epoch: 5, until: 10, source_epoch: 6, source_visual: "Granary_6", hp: Some(4000.0), ..EARLY },
    UnitRow { epoch: 10, until: 14, source_epoch: 11, source_visual: "Granary_11",
        hp: Some(5000.0), sight: Some(8.0), ..EARLY },
    UnitRow { epoch: 14, until: 15, source_epoch: 15, source_visual: "Granary_15",
        hp: Some(10000.0), sight: Some(8.0), ..EARLY },
];

pub(crate) fn row(epoch: u8) -> Option<&'static UnitRow> {
    ROWS.iter().find(|row| (row.epoch..row.until).contains(&epoch))
}

pub(crate) fn name(epoch: u8) -> &'static str {
    match epoch {
        0..=9 => "Windmühle (1.6)",
        10..=13 => "Getreidespeicher (1.6)",
        _ => "Botanisches Modul (1.6)",
    }
}

impl Game {
    pub(crate) fn granary_available(&self) -> bool {
        self.bench == 0 && (!self.net || self.network_granary)
    }

    pub(crate) fn granary_limit_reached(&self, team: u8, territory: u8) -> bool {
        self.granary_sites.iter().map(|&index| &self.ents[index]).any(|site| site.hp > 0.0
            && (site.built >= 1.0 || site.team == team)
            && self.map.terr[self.map.tile_of(site.x, site.y)] == territory)
    }

    pub(crate) fn granary_farm_bonus(&self, worker: &Ent) -> f32 {
        if worker.def != WORKER || self.granary_sites.is_empty() { return 0.0; }
        let tile = self.map.tile_of(worker.x, worker.y);
        let territory = self.map.terr[tile];
        if self.territory_owner_at(tile) != worker.team || self.territory_is_disputed(territory as usize) { return 0.0; }
        self.granary_sites.iter().map(|&index| &self.ents[index]).filter(|site| site.hp > 0.0
            && site.team == worker.team && site.built >= 1.0
            && self.map.terr[self.map.tile_of(site.x, site.y)] == territory).count() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        defs::{DEFS, FARM},
        game::{NOIDX, O_GATHER},
        map::{R_FOOD, R_WOOD},
    };

    fn solo() -> Game {
        let mut game = Game::new(171, 1, 0, 1, 0);
        for player in &mut game.players[..2] { player.epoch = 1; }
        for team in 0..2 { game.rebuild_modifiers(team, false); }
        game
    }

    #[test]
    fn era_rows_feed_the_normal_construction_and_asset_registry() {
        let mut game = solo();
        for (age, visual, hp, sight) in [
            (1, "Granary_2", 3000.0, 6.0), (5, "Granary_6", 4000.0, 6.0),
            (10, "Granary_11", 5000.0, 8.0), (14, "Granary_15", 10000.0, 8.0),
        ] {
            game.players[0].epoch = age;
            game.rebuild_modifiers(0, false);
            let profile = crate::ee2_units::lookup_class(ID, age).unwrap();
            assert_eq!(profile.source_visual, visual);
            assert_eq!(game.max_hp(0, ID), hp);
            assert_eq!(game.modifiers[0][ID as usize][5], sight);
            assert_eq!(game.production_time(0, ID), 50.0);
            assert_eq!(game.construction_cost(0, ID), STOCK);
            assert!(game.construction_available(0, ID));
        }
        game.players[0].epoch = 0;
        assert!(!game.construction_available(0, ID));
        assert_eq!(crate::ids::unit_render(ID), 365.0);
        assert_eq!(Game::footprint_axes(ID, 0), (3, 5));
        assert_eq!(Game::footprint_axes(ID, 2), (5, 3));
        assert_eq!(Game::footprint_axes(ID, 1), (0, 0));
    }

    #[test]
    fn reservation_counts_requesters_pending_and_everyones_completed_buildings() {
        let mut game = solo();
        let site = game.spawn(ID, 1, 70.5, 70.5);
        assert_ne!(site, NOIDX);
        let territory = game.map.terr[tile_of(70.5, 70.5)];
        game.ents[site].built = 0.0;
        assert!(!game.granary_limit_reached(0, territory));
        assert!(game.granary_limit_reached(1, territory));
        game.ents[site].built = 1.0;
        assert!(game.granary_limit_reached(0, territory));
        game.ents[site].hp = 0.0;
        assert!(!game.granary_limit_reached(0, territory));
    }

    #[test]
    fn live_farm_gathering_and_food_only_dropoff() {
        let mut game = solo();
        let site = game.spawn(ID, 0, 56.5, 46.5);
        let farm = game.spawn(FARM, 0, 60.5, 46.5);
        let citizen = game.spawn(WORKER, 0, 61.5, 46.5);
        assert!([site, farm, citizen].into_iter().all(|actor| actor != NOIDX));
        let (start_x, start_y) = game.map.starts[0];
        let territory = game.map.terr[tile_of(start_x as f32, start_y as f32)];
        for index in [site, farm, citizen] {
            let actor = game.ents[index];
            game.map.terr[tile_of(actor.x, actor.y)] = territory;
        }
        assert_eq!(game.towner[territory as usize], 0);
        game.ents[citizen].order = O_GATHER;
        game.ents[citizen].otgt = game.h(farm);
        game.ents[citizen].q[0] = u32::from(R_FOOD);
        game.ents[site].built = 0.0;
        game.tick();
        let base = game.ents[citizen].carry;
        assert!(base > 0.0);
        game.ents[citizen].carry = 0.0;
        game.ents[site].built = 1.0;
        assert_eq!(game.granary_farm_bonus(&game.ents[citizen]), 1.0);
        game.tick();
        assert_eq!(game.ents[citizen].carry, base * 2.0);

        let mut worker = game.ents[citizen];
        worker.x = 56.5;
        worker.y = 45.5;
        for (kind, takes_food) in [(R_FOOD, true), (R_WOOD, false)] {
            worker.carry = 6.0;
            worker.q[1] = 1;
            worker.q[2] = u32::from(kind);
            let before = game.resource(0, kind as usize - 1);
            let mut moving = false;
            game.return_cargo(&mut worker, &DEFS[WORKER as usize], &mut (0.0, 0.0), &mut moving);
            assert_eq!(game.resource(0, kind as usize - 1) - before, if takes_food { 6.0 } else { 0.0 });
        }
        game.kill(site);
        assert_eq!(game.granary_farm_bonus(&game.ents[citizen]), 0.0);
    }

    #[test]
    fn granary_save_rebuilds_its_live_registry() {
        let mut game = Game::new(171, 1, 0, 1, 0);
        assert!(game.configure_match_settings(0xeeee, 500, 2, 1));
        game.save();
        assert!(Game::load(&game.savebuf).is_some(), "ordinary old-format archive");
        let citizen = game.ents[..game.high].iter().position(|actor| actor.def == WORKER && actor.team == 0).unwrap();
        let tile = (43..62).flat_map(|y| (43..62).map(move |x| (x, y))).find(|&(x, y)|
            game.placement_error_for(0, ID, x, y, 2, &[citizen], false) == 0).unwrap();
        let site = game.begin_construction(0, ID, tile.0, tile.1, 2, &[citizen], false);
        assert_ne!(site, NOIDX);
        let handle = game.h(site);
        game.save();
        let mut restored = Game::load(&game.savebuf).expect("Granary foundation archive");
        assert_eq!(restored.get(handle), Some(site));
        assert!(restored.granary_sites.contains(&site));
        assert_eq!(restored.state_hash(), game.state_hash());
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
    }
}
