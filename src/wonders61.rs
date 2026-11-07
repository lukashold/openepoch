//! The distinct regional Wonder types offered by the active Citizen tech tree.

use crate::{defs::{Def, DefId, A_BLD, BUILT_BY_WORKER, HOUSE, LAND, NOW, SEA}, ee2_units::UnitRow};

pub(crate) const FIRST: DefId = 90;
pub(crate) const END: DefId = FIRST + 12;
const COST: [u16; 9] = [500, 500, 0, 500, 0, 0, 0, 0, 0];
const TIME: f32 = 900.0;
const ARCHIVE_TAG: &[u8; 8] = b"WDR1\x01\x00\x00\x00";
pub(crate) const SPHINX: DefId = FIRST + 3;
pub(crate) const PARTHENON: DefId = FIRST;
pub(crate) const TOWER_OF_MOON_AND_STARS: DefId = FIRST + 6;
pub(crate) const DESALINATION_PLANT: DefId = FIRST + 5;
pub(crate) const SOLAR_POWER_STATION: DefId = FIRST + 11;
const PENTAGON: DefId = FIRST + 2;
const FORBIDDEN_PALACE: DefId = FIRST + 7;
const JAGUAR_TEMPLE: DefId = FIRST + 9;
pub(crate) const BRANDENBURG_GATE: DefId = FIRST + 1;
pub(crate) const SELIMIYE_MOSQUE: DefId = FIRST + 4;
pub(crate) const GREAT_PYRAMID: DefId = FIRST + 10;

pub(crate) enum Notice {
    Started = 36,
    Completed = 37,
    Destroyed = 38,
    Captured = 39,
}

#[derive(Clone, Copy)]
struct Wonder {
    name: &'static str,
    family: &'static str,
    start: u8,
    ttciv: u8,
    body: [f32; 2],
}

const WONDERS: [Wonder; 12] = [
    Wonder { name: "Parthenon", family: "WonderWestern_01", start: 0, ttciv: 15, body: [5.0, 3.0] },
    Wonder { name: "Brandenburger Tor", family: "WonderWestern_06", start: 5, ttciv: 15, body: [2.5, 5.5] },
    Wonder { name: "Pentagon", family: "WonderWestern_11", start: 10, ttciv: 15, body: [5.5, 5.5] },
    Wonder { name: "Sphinx", family: "WonderMiddleEast_01", start: 0, ttciv: 17, body: [4.0, 3.0] },
    Wonder { name: "Selimye-Moschee", family: "WonderMiddleEast_06", start: 5, ttciv: 17, body: [5.0, 4.0] },
    Wonder { name: "Entsalzungsanl.", family: "WonderMiddleEast_11", start: 10, ttciv: 17, body: [5.0, 5.0] },
    Wonder { name: "Turm des Mondes und der Sterne", family: "WonderFarEast_01", start: 0, ttciv: 16, body: [6.0, 6.0] },
    Wonder { name: "Verbotener Palast", family: "WonderFarEast_06", start: 5, ttciv: 16, body: [4.5, 5.5] },
    Wonder { name: "Roboterfabrik", family: "WonderFarEast_11", start: 10, ttciv: 16, body: [6.0, 4.0] },
    Wonder { name: "Tempel des großen Jaguars", family: "WonderMesoAmerican_01", start: 0, ttciv: 18, body: [3.0, 3.0] },
    Wonder { name: "Große Pyramide von Tenochtitlán", family: "WonderMesoAmerican_06", start: 5, ttciv: 18, body: [5.0, 4.5] },
    Wonder { name: "Solarkraftwerk", family: "WonderMesoAmerican_11", start: 10, ttciv: 18, body: [6.0, 6.0] },
];

impl Wonder {
    const fn hp(self) -> f32 { 6000.0 + self.start as f32 * 400.0 }

    const fn footprint(self) -> [f32; 2] { [self.body[0] + 1.5, self.body[1] + 1.5] }

    const fn definition(self) -> Def {
        let size = if self.body[0] >= self.body[1] { self.body[0] } else { self.body[1] };
        Def {
            name: self.name, epoch: self.start, until: 15, dom: LAND, building: true,
            size: size * core::f32::consts::SQRT_2, hp: self.hp(), speed: 0.0, sight: 5.0,
            armor: A_BLD, cost: [500, 500, 0], time: TIME, pop: 0, made_at: BUILT_BY_WORKER,
            flags: 0, w: [NOW; 2],
        }
    }

    const fn row(self, index: usize) -> UnitRow {
        UnitRow {
            id: FIRST + index as DefId, epoch: self.start, until: 15, source_epoch: self.start + 1,
            source_unit: self.family, source_visual: self.family, source_host: "Citizen",
            identity_verified: true, model_verified: false,
            hp: Some(self.hp()), buildtime: Some(TIME), cost9: Some(COST), pop: Some(0), pop_capacity: None,
            sight: Some(5.0), raw_size: Some(self.body), raw_footprint: Some(self.footprint()),
            raw_speed: None, gather: None, farm_rate: None, carry: None, farm_carry: None, repair_rate: None,
            damage: None, range: None, reload: None, garrison_slots: None, generation: None,
            generation_resource: None, garrison_heal: None, garrison_attack_bonus: None,
            garrison_size: None, fortress_garrison: false, attack_missile: None,
        }
    }
}

pub(crate) const fn append(existing: [Def; FIRST as usize]) -> [Def; END as usize] {
    let mut rows = [WONDERS[0].definition(); END as usize];
    let mut index = 0;
    while index < existing.len() {
        rows[index] = existing[index];
        index += 1;
    }
    while index < rows.len() {
        rows[index] = WONDERS[index - existing.len()].definition();
        index += 1;
    }
    rows
}

const UNIT_ROWS: [UnitRow; WONDERS.len()] = {
    let mut rows = [WONDERS[0].row(0); WONDERS.len()];
    let mut index = 1;
    while index < rows.len() {
        rows[index] = WONDERS[index].row(index);
        index += 1;
    }
    rows
};

pub(crate) fn is_wonder(definition: DefId) -> bool { (FIRST..END).contains(&definition) }

fn index(definition: DefId) -> Option<usize> {
    is_wonder(definition).then(|| usize::from(definition - FIRST))
}

pub(crate) fn lookup(definition: DefId) -> Option<&'static UnitRow> {
    Some(&UNIT_ROWS[index(definition)?])
}

pub(crate) fn available(definition: DefId, civilization: i8, epoch: u8) -> bool {
    let Some(index) = index(definition) else { return false; };
    let row = WONDERS[index];
    epoch >= row.start && crate::civ8_gates::matches_ttciv(row.ttciv, civilization)
}

pub(crate) fn shape(definition: DefId, footprint: bool) -> Option<[f32; 2]> {
    let row = WONDERS[index(definition)?];
    let dimensions = if footprint { row.footprint() } else { row.body };
    Some(dimensions.map(|size| size * core::f32::consts::SQRT_2))
}

pub(crate) fn wrap_save(core: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(core.len() + 12);
    bytes.extend_from_slice(ARCHIVE_TAG);
    bytes.extend_from_slice(&core);
    bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
    bytes
}

pub(crate) fn split_save(bytes: &[u8]) -> Option<(&[u8], bool)> {
    if !bytes.starts_with(&ARCHIVE_TAG[..4]) { return Some((bytes, false)); }
    let end = bytes.len().checked_sub(4)?;
    let checksum = u32::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    if end <= ARCHIVE_TAG.len() || !bytes.starts_with(ARCHIVE_TAG)
        || crate::save::checksum(&bytes[..end]) != checksum { return None; }
    Some((&bytes[ARCHIVE_TAG.len()..end], true))
}

pub(crate) fn population_bonus(actor: &crate::game::Ent) -> u32 {
    if actor.def == DESALINATION_PLANT && actor.hp > 0.0 { 35 } else { 0 }
}

pub(crate) fn proselytizing_roll(word: u32) -> bool {
    // 008033AB uses the original f32 coefficient, then compares the
    // extended product without an intermediate f32 store.
    f64::from(word) * f64::from(1.0f32 / 32_767.0) <= f64::from(0.05f32)
}

impl crate::game::Game {
    fn completed_wonders(&self) -> impl DoubleEndedIterator<Item = &crate::game::Ent> {
        self.wonder_sites.iter().map(|&site| &self.ents[site]).filter(|e| e.alive && e.hp > 0.0 && e.built >= 1.0)
    }

    pub(crate) fn wonder_notice(&mut self, notice: Notice, definition: DefId, owner: u8) {
        if is_wonder(definition) { self.ev(notice as u32 as f32, owner as f32, definition as f32); }
    }

    pub(crate) fn owns_completed_wonder(&self, owner: u8, definition: DefId) -> bool {
        self.completed_wonders().any(|e| e.team == owner && e.def == definition)
    }

    pub(crate) fn completed_wonder_owners(&self, definition: DefId) -> u8 {
        self.completed_wonders().filter(|e| e.def == definition)
            .fold(0, |owners, actor| owners | (1 << actor.team))
    }

    pub(crate) fn count_completed_wonders(&self, owner: u8, definition: DefId) -> usize {
        self.completed_wonders().filter(|e| e.team == owner && e.def == definition).count()
    }

    pub(crate) fn wonder_capture_yield(&self, victim: u8) -> f32 {
        // 008028B0 reads type 0x29 from the victim, even if the captor
        // is a third player. Each application adds its deviation from 1.
        self.completed_wonders().rev().filter(|e| e.def == FORBIDDEN_PALACE)
            .fold(1.0f32, |combined, palace| {
                let factor = if palace.team == victim { 0.0f32 } else { 1.35f32 };
                (f64::from(combined) + f64::from(factor) - 1.0) as f32
            }).max(0.0)
    }

    pub(crate) fn defended_house(&self, actor: &crate::game::Ent, definition: &Def) -> Option<Def> {
        if actor.def != HOUSE || !self.owns_completed_wonder(actor.team, PENTAGON) { return None; }
        let epoch = self.players[actor.team as usize].epoch;
        let attack = crate::ee2_up15_attack::lookup(HOUSE, epoch)?;
        let stats = crate::ee2_units::lookup_class(HOUSE, epoch)?;
        let mut defended = *definition;
        defended.w[0].kind = attack.visual_kind;
        defended.w[0].dmg = stats.damage?;
        defended.w[0].cd = stats.reload?;
        defended.w[0].targets = LAND | SEA;
        Some(defended)
    }

    pub(crate) fn art_of_war_bonus(&self, actor: &crate::game::Ent) -> f32 {
        use crate::map::tile_of;
        let temples = self.count_completed_wonders(actor.team, JAGUAR_TEMPLE);
        if temples == 0 || !crate::ee2_combat::land_effect_target(actor.def, self.players[actor.team as usize].epoch) {
            return 0.0;
        }
        let territory = self.map.terr[tile_of(actor.x, actor.y)];
        let owner = self.towner[territory as usize];
        if owner == 255 || owner == actor.team { return 0.0; }
        let houses = self.ents[..self.high].iter().filter(|house| house.alive && house.hp > 0.0
            && house.built >= 1.0 && house.def == HOUSE && house.team == owner
            && self.map.terr[tile_of(house.x, house.y)] == territory).count();
        (houses as f32 * 0.03).min(0.5) * temples as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::{BARRACKS, HOUSE, MILL, TC, WORKER}, game::{Game, NOIDX, O_BUILD}, map::{tile_of, N, NN, R_NONE, T_GRASS}};

    #[test]
    fn all_regional_citizen_nodes_open_in_the_authored_epochs() {
        for (civilization, first) in [(0, FIRST), (8, FIRST + 3), (5, FIRST + 6), (11, FIRST + 9)] {
            for epoch in 0..15 {
                let offered: Vec<_> = (FIRST..END).filter(|&id| available(id, civilization, epoch)).collect();
                assert_eq!(offered, (first..first + u16::from(epoch / 5) + 1).collect::<Vec<_>>());
            }
        }
        assert!((FIRST..END).all(|id| !available(id, 14, 14)));
        assert_eq!(lookup(FIRST + 9).unwrap().source_unit, "WonderMesoAmerican_01");
        assert_eq!(lookup(FIRST + 11).unwrap().hp, Some(10000.0));
    }

    #[test]
    fn paid_turned_wonder_reserves_its_plot_and_restores_its_body_after_save() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_civilizations([1, 8, 14, 14]));
        assert!(game.configure_priest_skirmish());
        game.players[0].res = [1500.0; 3];
        game.extra_resources[0].fill(1500.0);
        let worker = game.ents[..game.high].iter().position(|e| e.alive && e.team == 0 && e.def == WORKER).unwrap();
        game.map.ter.fill(T_GRASS);
        game.map.res.fill(R_NONE);
        game.map.amt.fill(0);
        for i in 0..NN { game.map.upd_pass(i); }
        assert_eq!(Game::footprint_axes(FIRST, 0), (10, 7));
        assert_eq!(Game::footprint_axes(FIRST, 2), (7, 10));
        assert_eq!(Game::footprint_axes(FIRST, 1), (0, 0));
        let (x, y) = (90..160).flat_map(|y| (90..160).map(move |x| (x, y)))
            .find(|&(x, y)| game.placement_geometry_error(0, FIRST, x, y, 2) == 0).unwrap();
        assert_eq!(game.begin_construction(0, SPHINX, x, y, 2, &[worker], false), NOIDX);
        let index = game.begin_construction(0, FIRST, x, y, 2, &[worker], false);
        assert_ne!(index, NOIDX);
        let wonder = game.ents[index];
        assert_eq!((wonder.x, wonder.y, wonder.rot, wonder.built), (x as f32 + 3.5, y as f32 + 5.0, Game::angle_for_turn(2), 0.0));
        assert_eq!(game.players[0].res, [1000.0, 1000.0, 1500.0]);
        assert_eq!(game.extra_resources[0][0], 1000.0);
        assert_eq!(game.ents[worker].order, O_BUILD);
        assert_eq!(game.ents[worker].otgt, game.h(index));
        assert_eq!(game.production_time(0, FIRST), 900.0);
        let (bx, by, width, height) = Game::building_bounds(&wonder);
        assert!(width < height);
        assert!(game.map.blocked[by as usize * N + bx as usize]);
        assert!(!game.map.blocked[y as usize * N + x as usize]);
        assert_eq!(game.placement_geometry_error(0, HOUSE, x - 1, y - 1, 0), 1);
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.state_hash(), game.state_hash());
        assert_eq!(Game::building_bounds(&loaded.ents[index]), (bx, by, width, height));
        assert!(loaded.map.blocked[by as usize * N + bx as usize]);
        loaded.save();
        assert_eq!(loaded.savebuf, game.savebuf);
    }

    #[test]
    fn parthenon_requires_completed_barracks_and_mill_for_a_colony() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        let site = (30..215).flat_map(|y| (30..215).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let here = game.map.terr[tile_of(x as f32, y as f32)];
                here == game.map.terr[tile_of(x as f32 + 8.0, y as f32)] && game.towner[here as usize] == 255
            }).unwrap();
        let (x, y) = (site.0 as f32, site.1 as f32);
        let territory = game.map.terr[tile_of(x, y)] as usize;
        let parthenon = game.spawn(PARTHENON, 0, 127.0, 127.0);
        let barracks = game.spawn(BARRACKS, 0, x, y);
        let mill = game.spawn(MILL, 0, x + 8.0, y);
        game.ents[mill].built = 0.5;
        game.rebuild_territories();
        assert_eq!(game.towner[territory], 255);
        game.ents[mill].built = 1.0;
        game.rebuild_territories();
        assert_eq!(game.towner[territory], 0);
        assert_eq!(game.territory.claims[territory], game.h(barracks));
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.towner[territory], 0);
        loaded.kill(parthenon);
        assert_eq!(loaded.towner[territory], 255);
    }

    #[test]
    fn live_sphinx_protects_capture_targets_and_farseeing_paints_native_sight() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        let sphinx = game.spawn(SPHINX, 0, 95.0, 95.0);
        let house = game.spawn(HOUSE, 0, 120.0, 120.0);
        let enemy_city = game.spawn(TC, 1, 210.0, 210.0);
        game.team_vis[NN + tile_of(120.0, 120.0)] = 255;
        assert!(!game.capture_allowed(1, house));
        game.ents[sphinx].built = 0.5;
        assert!(game.capture_allowed(1, house));
        game.update_fog();
        assert_ne!(game.real_fog(0, tile_of(210.0, 210.0)), 255);
        assert_ne!(game.real_fog(0, tile_of(132.0, 120.0)), 255);
        let tower = game.spawn(TOWER_OF_MOON_AND_STARS, 0, 85.0, 85.0);
        game.update_fog();
        assert_eq!(game.real_fog(0, tile_of(game.ents[enemy_city].x, game.ents[enemy_city].y)), 255);
        assert_eq!(game.real_fog(0, tile_of(132.0, 120.0)), 255);
        game.kill(tower);
        game.update_fog();
        assert_eq!(game.real_fog(0, tile_of(210.0, 210.0)), 128);
    }

    #[test]
    fn finished_player_attached_effects_change_quotes_and_population() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        let original = game.construction_cost(0, HOUSE);
        let solar = game.spawn(SOLAR_POWER_STATION, 0, 100.0, 100.0);
        game.ents[solar].built = 0.5;
        assert_eq!(game.construction_cost(0, HOUSE), original);
        game.ents[solar].built = 1.0;
        assert_eq!(game.construction_cost(0, HOUSE), original.map(|price| price / 2));
        assert_eq!(game.unit_cost(0, HOUSE), original);
        let cap = game.players[0].cap;
        let plant = game.spawn(DESALINATION_PLANT, 0, 124.0, 124.0);
        assert_eq!(game.players[0].cap, cap + 35);
        game.kill(plant);
        assert_eq!(game.players[0].cap, cap);
        game.kill(solar);
        assert_eq!(game.construction_cost(0, HOUSE), original);
    }

    #[test]
    fn pentagon_enables_an_actual_house_attack_and_palace_changes_target_yield() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        game.players[0].epoch = 10;
        game.rebuild_modifiers(0, false);
        for player in &mut game.players[..game.nplayers] { player.ai = false; }
        let house = game.spawn(HOUSE, 0, 125.0, 125.0);
        let worker = game.spawn(WORKER, 1, 127.0, 125.0);
        let target = game.h(worker);
        let hp = game.ents[worker].hp;
        for _ in 0..32 { game.tick(); }
        assert_eq!(game.ents[worker].hp, hp);
        let pentagon = game.spawn(PENTAGON, 0, 94.0, 94.0);
        let profile = game.defended_house(&game.ents[house], &crate::defs::DEFS[HOUSE as usize]).unwrap();
        assert_eq!(profile.w[0].dmg, 70.0);
        for _ in 0..70 { game.tick(); }
        assert!(game.get(target).is_none() || game.ents[worker].hp < hp);
        game.kill(pentagon);
        assert!(game.defended_house(&game.ents[house], &crate::defs::DEFS[HOUSE as usize]).is_none());

        let palace = game.spawn(FORBIDDEN_PALACE, 0, 110.0, 110.0);
        assert_eq!(game.wonder_capture_yield(0), 0.0);
        assert_eq!(game.wonder_capture_yield(1), 1.35);
        game.kill(palace);
        assert_eq!(game.wonder_capture_yield(1), 1.0);
    }

    #[test]
    fn wonder_kill_rewards_apply_positive_scales_and_bonuses_once() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        game.players[1].epoch = 10;
        game.rebuild_modifiers(1, false);
        let victim = 26;
        let price = game.unit_cost(1, victim)[4];
        assert!(price > 0);
        let gate = game.spawn(BRANDENBURG_GATE, 0, 90.0, 100.0);
        let mosque = game.spawn(SELIMIYE_MOSQUE, 0, 110.0, 100.0);
        let gold = game.resource(0, 4);
        let expected_gold = (f64::from(gold) + f64::from(price) * f64::from(0.2f32)) as f32;
        game.combat_kill_payment(0, 9, 1, victim);
        assert_eq!(game.resource(0, 4), expected_gold);
        assert_eq!(game.research_points[0], 0.201f32);
        game.combat_kill_payment(0, 9, 0, victim);
        assert_eq!(game.resource(0, 4), expected_gold);
        assert_eq!(game.research_points[0], 0.201f32);
        game.kill(gate);
        game.kill(mosque);
        game.combat_kill_payment(0, 9, 1, victim);
        assert_eq!(game.resource(0, 4), expected_gold);
        assert_eq!(game.research_points[0], 0.201f32);
    }

    #[test]
    fn jaguar_temple_reads_the_foreign_territory_owners_completed_houses() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        let enemy_city = game.ents[..game.high].iter().find(|e| e.alive && e.team == 1 && e.def == TC).copied().unwrap();
        let x = enemy_city.x;
        let y = enemy_city.y;
        let soldier = game.spawn(9, 0, x, y);
        let house = game.spawn(HOUSE, 1, x, y);
        assert_eq!(game.art_of_war_bonus(&game.ents[soldier]), 0.0);
        let temple = game.spawn(JAGUAR_TEMPLE, 0, 90.0, 90.0);
        assert_eq!(game.art_of_war_bonus(&game.ents[soldier]), 0.03);
        game.ents[house].built = 0.99;
        assert_eq!(game.art_of_war_bonus(&game.ents[soldier]), 0.0);
        game.ents[house].built = 1.0;
        game.kill(temple);
        assert_eq!(game.art_of_war_bonus(&game.ents[soldier]), 0.0);
    }
}
