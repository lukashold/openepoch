//! Authenticated Death Valley2 compressed forest and its mutable gather ledger.
use super::{
    forest_play164::actor_id,
    forest_registry::{ForestId, ForestRow},
    forest_replay163::{ForestReplay, WOOD},
    geometry::Geometry,
    population234::Population,
    rectangular_world::source_height,
    source_forest_grid::SourceForestGrid,
};
use crate::game::{Game, NONE};
use std::collections::BTreeMap;

const MAGIC: [u8; 4] = *b"DVF1";
const ROWS: usize = 1328;
const DIMENSIONS: [usize; 2] = [100, 300];
const TREE_EXTENT: f32 = f32::from_bits(0x3f47_1f0c);
const FERN_EXTENT: f32 = f32::from_bits(0x3eb5_04f3);
const HEADER: usize = 8;
const RECORD: usize = 16;

pub(crate) struct RectangularForest {
    rows: Vec<ForestRow>,
    by_cell: BTreeMap<[u16; 2], Vec<u32>>,
    grid: SourceForestGrid,
    pub(super) replay: ForestReplay,
}

fn gatherable(row: &ForestRow) -> bool {
    row.state == 0 && row.flags & 0x60 == 0x20
}

fn extent(row: &ForestRow) -> f32 {
    if row.flags == 0x180 { FERN_EXTENT } else { TREE_EXTENT }
}

fn compressed_grid(rows: &[ForestRow], promoted: &[Option<super::forest_replay163::Promoted>],
    exclude: Option<u32>) -> Option<SourceForestGrid> {
    let mut grid = SourceForestGrid::new(DIMENSIONS[0], DIMENSIONS[1])?;
    for (index, (row, actor)) in rows.iter().zip(promoted).enumerate() {
        if actor.is_some() || exclude == Some(index as u32) { continue; }
        let [x, y, _] = row.position;
        grid.register([x, y], [extent(row); 2], row.flags, row.state)?;
    }
    Some(grid)
}

impl RectangularForest {
    #[cfg(feature = "ee2-rect239-fresh")]
    pub(super) fn placement_plane(&self) -> Option<&[u32]> {
        self.grid.plane(1)
    }

    pub(super) fn new(world: &Geometry, population: &Population) -> Option<Self> {
        if !Population::map_matches(world.source_sha, world.input_sha) ||
            [world.width, world.height] != DIMENSIONS || population.forest().len() != ROWS ||
            world.pending_forest as usize != ROWS { return None; }
        let mut rows = Vec::with_capacity(ROWS);
        let mut by_cell = BTreeMap::<_, Vec<_>>::new();
        for source in population.forest() {
            if source.ordinal as usize != rows.len() { return None; }
            let name = population.types().get(source.type_index as usize)?.as_str();
            match (name, source.flags, source.state) {
                ("TRE_FIR" | "TRE_OAK2" | "TRE_OAK" | "TRE_MAPLE_01" | "TRE_MAPLE_02" |
                    "TRE_OAK_DEAD" | "TRE_BIRCH", 0x1a0, 0) => {}
                ("TRE_FERNTROP1" | "TRE_FERNTROP2", 0x180, 0) | ("TRE_PINE", 0x80, 3) => {}
                _ => return None,
            }
            let [x, y] = source.xy();
            // 00819B50 calls base-ground 0070FC10, then _ftol truncates the
            // unspilled return times 128 at 00819C8D.
            let height = source_height(world, &[], x, y, false) * 128.0;
            if !(0.0..65536.0).contains(&height) { return None; }
            let [x, y] = source.xy_words;
            rows.push(ForestRow { position: [x, y, height as u16], profile: u8::try_from(source.type_index).ok()?,
                flags: source.flags, state: source.state });
            by_cell.entry([x / 128, y / 128]).or_default().push(source.ordinal);
        }
        let replay = ForestReplay::playable_rows(&rows);
        let grid = compressed_grid(&rows, &replay.promoted, None)?;
        Some(Self { rows, by_cell, grid, replay })
    }

    pub(crate) fn source(&self, row: u32) -> Option<&ForestRow> {
        self.rows.get(row as usize)
    }

    pub(crate) fn cell(&self, x: u16, y: u16) -> &[u32] {
        self.by_cell.get(&[x, y]).map(Vec::as_slice).unwrap_or(&[])
    }

    pub(crate) fn point_flags(&self, x: i32, y: i32) -> u32 {
        self.grid.point_flags(x, y)
    }

    #[cfg(feature = "ee2-rect268-city-site")]
    pub(super) fn blocks_city_placement(&self, x: i32, y: i32) -> bool {
        self.grid.blocks_source_cell(x, y)
    }

    pub(super) fn gather_position(&self, row: u32) -> Option<(f32, f32)> {
        let source = self.source(row).filter(|row| gatherable(row))?;
        let [x, y, _] = source.position();
        Some((x, y))
    }

    #[cfg(feature = "ee2-rect239-fresh")]
    pub(super) fn gather_rows(&self) -> impl Iterator<Item = (u32, (f32, f32))> + '_ {
        (0..self.rows.len() as u32).filter_map(|row| self.gather_position(row).map(|point| (row, point)))
    }

    pub(super) fn promote(&mut self, row: u32) -> Option<()> {
        self.gather_position(row)?;
        let grid = compressed_grid(&self.rows, &self.replay.promoted, Some(row))?;
        self.replay.promote(ForestId(row), actor_id(row), WOOD)?;
        self.grid = grid;
        Some(())
    }

    fn records(&self) -> impl Iterator<Item = [u32; 4]> + '_ {
        self.replay.promoted.iter().enumerate().filter_map(|(row, actor)| {
            let actor = actor.as_ref()?;
            Some([row as u32, actor.wood.to_bits(), actor.last_chop.map_or(NONE, f32::to_bits),
                actor.exhaustion.map_or(NONE, f32::to_bits)])
        })
    }

    fn section(&self) -> Option<Vec<u8>> {
        let count = self.replay.promoted.iter().flatten().count() as u16;
        if count == 0 { return None; }
        let mut data = Vec::with_capacity(HEADER + usize::from(count) * RECORD);
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&(ROWS as u16).to_le_bytes());
        data.extend_from_slice(&count.to_le_bytes());
        for words in self.records() {
            for word in words { data.extend_from_slice(&word.to_le_bytes()); }
        }
        Some(data)
    }

    fn restore(&mut self, data: &[u8], now: f32) -> Option<()> {
        if !data.starts_with(&MAGIC) || !now.is_finite() || now < 0.0 { return None; }
        let rows = u16::from_le_bytes(data.get(4..6)?.try_into().ok()?) as usize;
        let count = u16::from_le_bytes(data.get(6..8)?.try_into().ok()?) as usize;
        if rows != ROWS || count == 0 || count > rows || data.len() != HEADER + count * RECORD { return None; }
        let mut replay = ForestReplay::playable_rows(&self.rows);
        let mut previous = None;
        for entry in data[HEADER..].chunks_exact(RECORD) {
            let word = |index: usize| u32::from_le_bytes(entry[index * 4..index * 4 + 4].try_into().unwrap());
            let row = word(0);
            if previous.is_some_and(|old| row <= old) { return None; }
            previous = Some(row);
            self.gather_position(row)?;
            let wood = f32::from_bits(word(1));
            if !(word(1) == 0 || (1.0..=WOOD).contains(&wood)) { return None; }
            let time = |index| {
                let bits = word(index);
                if bits == NONE { return Some(None); }
                let seconds = f32::from_bits(bits);
                (!seconds.is_sign_negative() && (0.0..=now).contains(&seconds)).then_some(Some(seconds))
            };
            let last_chop = time(2)?;
            let exhaustion = time(3)?;
            if (wood == 0.0 && (exhaustion.is_none() || exhaustion != last_chop)) ||
                (wood > 0.0 && exhaustion.is_some()) ||
                (wood < WOOD && last_chop.is_none()) { return None; }
            replay.promote(ForestId(row), actor_id(row), wood)?;
            let actor = replay.promoted.get_mut(row as usize)?.as_mut()?;
            actor.last_chop = last_chop;
            actor.exhaustion = exhaustion;
        }
        let grid = compressed_grid(&self.rows, &replay.promoted, None)?;
        self.replay = replay;
        self.grid = grid;
        Some(())
    }
}

impl Game {
    pub(crate) fn attach_rect_forest(&mut self) -> Option<()> {
        if self.rect_forest.is_some() { return None; }
        let world = self.map.source_world.as_deref()?;
        let population = self.rect_population.as_ref()?.source();
        self.rect_forest = Some(RectangularForest::new(world, population)?);
        Some(())
    }

    pub(crate) fn rect_forest_section(&self) -> Option<Vec<u8>> {
        self.rect_forest.as_ref()?.section()
    }

    pub(crate) fn restore_rect_forest_section(&mut self, bytes: &[u8]) -> Option<()> {
        let now = self.sim_elapsed();
        self.rect_forest.as_mut()?.restore(bytes, now)
    }

    pub(crate) fn hash_rect_forest(&self, mix: &mut impl FnMut(u32)) {
        let Some(forest) = &self.rect_forest else { return; };
        let mut records = forest.records().peekable();
        if records.peek().is_none() { return; }
        mix(u32::from_le_bytes(MAGIC));
        for row in records {
            for word in row { mix(word); }
        }
    }
}

#[no_mangle]
pub extern "C" fn native_rect_forest_source_word(row: u32, field: u32) -> u32 {
    let Some(source) = super::active_game().and_then(|game| game.rect_forest.as_ref())
        .and_then(|forest| forest.source(row)) else { return NONE; };
    match field {
        0..=2 => u32::from(source.position[field as usize]),
        3 => u32::from(source.profile),
        4 => u32::from(source.flags),
        5 => u32::from(source.state),
        _ => NONE,
    }
}

#[no_mangle]
pub extern "C" fn native_rect_forest_cell_word(x: u32, y: u32, field: u32) -> u32 {
    let Some(forest) = super::active_game().and_then(|game| game.rect_forest.as_ref()) else { return NONE; };
    if x >= DIMENSIONS[0] as u32 || y >= DIMENSIONS[1] as u32 { return NONE; }
    let rows = forest.cell(x as u16, y as u16);
    if field == 0 { return rows.len() as u32; }
    rows.get(field as usize - 1).copied().unwrap_or(NONE)
}

#[no_mangle]
pub extern "C" fn native_rect_forest_flags(x: i32, y: i32) -> u32 {
    super::active_game().and_then(|game| game.rect_forest.as_ref()).map_or(0, |forest| forest.point_flags(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cmd::C_VALLEY_FOREST_GATHER, defs::{TC, WORKER}, game::O_VALLEY_FOREST, map::{R_NONE, R_WOOD}};
    use super::super::rectangular_world::*;

    #[cfg(feature = "ee2-rect239-fresh")]
    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn fresh_death_valley2_ai_gathers_authored_stocks_and_restores_live_orders() {
        use crate::game::O_GATHER;
        use super::super::fresh239::native_rect_commit;
        use super::super::rectangular_world::spatial_tests::prepare_authenticated_inputs;

        prepare_authenticated_inputs();
        assert_eq!(native_rect_commit(171, 3), 1);
        let game = crate::g();
        assert!(game.map.res.iter().all(|&resource| resource == R_NONE));
        let mut world0 = [false; 3];
        let mut forest = [false; 3];
        let mut deposit = [false; 3];
        let mut loaded: Option<(Game, u32)> = None;
        for tick in 1..=2500 {
            let bank: [[f32; 9]; 3] = std::array::from_fn(|seat| {
                std::array::from_fn(|resource| game.resource((seat + 1) as u8, resource))
            });
            game.tick();
            for actor in &game.ents[..game.high] {
                if !actor.alive || actor.team == 0 || actor.def != WORKER { continue; }
                let seat = usize::from(actor.team) - 1;
                world0[seat] |= actor.order == O_GATHER && game.world0_gather_order(actor).is_some();
                forest[seat] |= actor.order == O_VALLEY_FOREST;
            }
            for (seat, initial) in bank.iter().enumerate() {
                deposit[seat] |= initial.iter().enumerate().any(|(resource, &previous)| {
                    game.resource((seat + 1) as u8, resource) > previous
                });
            }
            if let Some((restored, remaining)) = &mut loaded {
                if *remaining != 0 {
                    restored.tick();
                    assert_eq!(restored.state_hash(), game.state_hash());
                    *remaining -= 1;
                }
                if *remaining == 0 && deposit == [true; 3] {
                    println!("tick {tick}: World0 {world0:?}, compressed forest {forest:?}, delivered {deposit:?}");
                    break;
                }
            } else if forest.contains(&true) && world0.contains(&true) {
                game.save();
                let mut restored = Game::load(&game.savebuf).expect("AI authored gather orders save");
                assert_eq!(restored.state_hash(), game.state_hash());
                restored.save();
                assert_eq!(restored.savebuf, game.savebuf);
                loaded = Some((restored, 64u32));
            }
        }
        assert!(loaded.is_some(), "missing a live authored forest and stock save");
        assert_eq!(world0, [true; 3], "AI should work original Unit stocks");
        assert_eq!(deposit, [true; 3], "AI should deliver actual resources");
    }

    fn game() -> Game {
        let terrain = crate::native_valley37::private_test_fixtures::read("world.w8in");
        let population = crate::native_valley37::private_test_fixtures::read("world0.dvp1");
        let ptr = native_rect_alloc(terrain.len());
        assert!(!ptr.is_null());
        unsafe { std::ptr::copy_nonoverlapping(terrain.as_ptr(), ptr, terrain.len()); }
        assert_eq!(native_rect_prepare(), 1);
        let ptr = native_rect_population_alloc(population.len());
        assert!(!ptr.is_null());
        unsafe { std::ptr::copy_nonoverlapping(population.as_ptr(), ptr, population.len()); }
        assert_eq!(native_rect_population_prepare(), 1);
        staged_game(7, 1).expect("exact authenticated rectangular forest")
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn death_valley2_forest_exact_rows_planes_and_cells() {
        let game = game();
        let forest = game.rect_forest.as_ref().unwrap();
        let population = game.rect_population.as_ref().unwrap().source();
        let mut counts = [0; 4];
        let mut expected = [[false; 30_000]; 2];
        for source in population.forest() {
            let ordinal = source.ordinal;
            let row = forest.source(ordinal).unwrap();
            let [x, y] = source.xy_words;
            assert_eq!([row.position[0], row.position[1]], source.xy_words);
            assert_eq!(u16::from(row.profile), source.type_index);
            assert_eq!((row.flags, row.state), (source.flags, source.state));
            assert!(forest.cell(x / 128, y / 128).contains(&ordinal));
            assert_eq!(forest.gather_position(ordinal).is_some(), source.flags == 0x1a0);
            let half = f64::from(extent(row)) * 0.5;
            let [fx, fy] = source.xy().map(f64::from);
            let bound = |value: f64| (((value as f32 as f64 + 103_079_215_104.0).to_bits() >> 16) as u32) as i32;
            for y in bound(fy - half).max(0)..=bound(fy + half).min(299) {
                for x in bound(fx - half).max(0)..=bound(fx + half).min(99) {
                    for (plane, flag) in [0x20, 0x100].into_iter().enumerate() {
                        if source.flags & flag != 0 { expected[plane][y as usize * 100 + x as usize] = true; }
                    }
                }
            }
            counts[0] += usize::from(y >= 256 * 128);
            counts[1] += usize::from(gatherable(row));
            counts[2] += usize::from(row.flags == 0x180);
            counts[3] += usize::from(row.state == 3);
        }
        assert_eq!(counts, [555, 1312, 12, 4]);
        let duplicates = forest.by_cell.values().filter(|ids| ids.len() > 1).count();
        assert_eq!(duplicates, 115);
        assert!(forest.by_cell.values().all(|ids| ids.windows(2).all(|pair| pair[0] < pair[1])));
        for y in 0..300 {
            for x in 0..100 {
                let tile = y as usize * 100 + x as usize;
                let bits = u32::from(expected[0][tile]) | u32::from(expected[1][tile]) << 1;
                assert_eq!(forest.point_flags(x, y), bits);
            }
        }
        let plane_counts = expected.map(|plane| plane.into_iter().filter(|&flag| flag).count());
        assert!(plane_counts[1] > plane_counts[0]);
        println!("rows={ROWS} south={} standing={} ferns={} stumps={} shared_cells={duplicates} planes={plane_counts:?}",
            counts[0], counts[1], counts[2], counts[3]);
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn death_valley2_forest_real_cut_deposit_and_section_restore() {
        let mut game = game();
        for team in 0..2 {
            game.players[team].alive = true;
            game.players[team].population_limit = 250;
            game.players[team].res = [400.0; 3];
            game.players[team].recompute();
            game.rebuild_modifiers(team as u8, false);
        }
        let near = game.rect_forest.as_ref().unwrap();
        let (row, tree, spot) = (0..ROWS as u32).find_map(|row| {
            let (x, y) = near.gather_position(row)?;
            let (cx, cy) = (x as usize, y as usize);
            if y < 260.0 || cx < 8 || cx >= 92 || cy >= 292 { return None; }
            let land = |x, y| game.map.pass[0][game.map.tile_index(x, y)];
            if !(cx - 6..cx + 5).all(|ix| (cy - 3..cy + 4).all(|iy| land(ix, iy))) { return None; }
            Some((row, (x,y), (cx as f32 - 5.0, cy as f32)))
        }).expect("southern forest next to clear approach and dropoff");
        game.spawn(TC, 0, spot.0, spot.1);
        let opponent = game.map.starts[0];
        game.spawn(TC, 1, opponent.0 as f32, opponent.1 as f32);
        let citizen = game.spawn(WORKER, 0, tree.0 - 0.5, tree.1);
        game.update_fog();
        let handle = game.h(citizen);
        let tile = game.map.tile_of(tree.0, tree.1);
        assert_eq!(game.map.res[tile], R_NONE);
        assert!(game.rect_forest_section().is_none());
        if let Some(path) = std::env::var_os("EE2_RECT_FOREST_FIXTURE_OUT") {
            game.save();
            assert!(game.savebuf.starts_with(b"DVS1"));
            std::fs::write(path, &game.savebuf).expect("private pre-gather fixture");
        }
        for passive in [639, 644, 648, 722] {
            game.apply(0, C_VALLEY_FOREST_GATHER, 0.0, 0.0, passive, &[handle]);
            assert!(game.rect_forest_section().is_none());
        }
        game.apply(0, C_VALLEY_FOREST_GATHER, 0.0, 0.0, row as i32, &[handle]);
        assert_eq!(game.ents[citizen].order, O_VALLEY_FOREST);
        assert_eq!(game.ents[citizen].otile, tile as u32);
        let actor = game.rect_forest.as_ref().unwrap().replay.promoted[row as usize].unwrap();
        assert_eq!(actor.actor, actor_id(row));
        assert_eq!(actor.wood.to_bits(), WOOD.to_bits());
        for _ in 0..8 {
            game.tick();
            if game.ents[citizen].carry > 0.0 { break; }
        }
        let wood = game.rect_forest.as_ref().unwrap().replay.promoted[row as usize].unwrap().wood;
        assert_eq!(wood.to_bits(), 0x449f_9ea4);
        assert_eq!(game.ents[citizen].q[2], u32::from(R_WOOD));
        assert_eq!(game.resource(0, 1), 400.0);
        let section = game.rect_forest_section().unwrap();
        let mut resumed = self::game();
        resumed.tick = game.tick;
        resumed.restore_rect_forest_section(&section).unwrap();
        assert_eq!(resumed.rect_forest_section().as_ref(), Some(&section));
        assert_eq!(game.rect_forest.as_ref().unwrap().grid.plane(0), resumed.rect_forest.as_ref().unwrap().grid.plane(0));
        let mut malformed = section.clone();
        malformed[8..12].copy_from_slice(&639u32.to_le_bytes());
        assert!(resumed.restore_rect_forest_section(&malformed).is_none());
        assert_eq!(resumed.rect_forest_section().as_ref(), Some(&section));
        let mut future = section.clone();
        future[16..20].copy_from_slice(&(game.sim_elapsed() + 1.0).to_bits().to_le_bytes());
        assert!(resumed.restore_rect_forest_section(&future).is_none());
        assert_eq!(resumed.rect_forest_section().as_ref(), Some(&section));
        let mut duplicate = section.clone();
        duplicate[6..8].copy_from_slice(&2u16.to_le_bytes());
        duplicate.extend_from_slice(&section[8..]);
        assert!(resumed.restore_rect_forest_section(&duplicate).is_none());
        assert_eq!(resumed.rect_forest_section().as_ref(), Some(&section));
        assert!(game.valid_valley_forest_orders(game.high));
        let chop_tick = game.tick;

        // A loaded low-stock actor reaches the native final-cut threshold on
        // the next actual simulation step. No cargo is added on that step.
        resumed.players = game.players;
        for team in 0..2 { resumed.rebuild_modifiers(team, false); }
        resumed.high = game.high;
        resumed.ents[..game.high].copy_from_slice(&game.ents[..game.high]);
        let carry = resumed.ents[citizen].carry;
        let mut low_stock = section.clone();
        low_stock[12..16].copy_from_slice(&1.025f32.to_bits().to_le_bytes());
        resumed.restore_rect_forest_section(&low_stock).unwrap();
        assert!(resumed.valid_valley_forest_orders(resumed.high));
        resumed.tick();
        let exhausted = resumed.rect_forest.as_ref().unwrap().replay.promoted[row as usize].unwrap();
        assert_eq!(exhausted.wood.to_bits(), 0);
        assert_eq!(exhausted.actor, actor.actor);
        assert_eq!(exhausted.last_chop, exhausted.exhaustion);
        assert!(exhausted.exhaustion.is_some());
        assert_eq!(resumed.ents[citizen].carry.to_bits(), carry.to_bits());
        let exhausted_section = resumed.rect_forest_section().unwrap();
        let mut stump_restore = self::game();
        stump_restore.tick = resumed.tick;
        stump_restore.restore_rect_forest_section(&exhausted_section).unwrap();
        assert_eq!(stump_restore.rect_forest_section().as_ref(), Some(&exhausted_section));

        for _ in 0..800 {
            game.tick();
            if game.resource(0, 1) > 400.0 { break; }
        }
        assert_eq!(game.resource(0, 1), 415.0);
        assert_eq!(game.map.res[tile], R_NONE);
        assert_eq!(game.map.amt[tile], 0);
        println!("ordinal={row} xy={tree:?} tile={tile} actor={:08x} first_chop={chop_tick} wood={:08x} deposit_tick={} wood_bank=415 exhausted={:08x} exhaustion_clock={:08x}",
            actor.actor, wood.to_bits(), game.tick, exhausted.wood.to_bits(), exhausted.exhaustion.unwrap().to_bits());
    }
}
