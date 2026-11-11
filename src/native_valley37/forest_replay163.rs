//! Guarded source-row and replay seam. The gameplay actor factory is external.
use super::{
    active_game,
    forest_registry::{ForestId, ForestRegistry},
    geometry::Geometry,
    source_forest_grid::SourceForestGrid,
};

const MAGIC: [u8; 4] = *b"VFR1";
pub(super) const WOOD: f32 = 1277.0;
const EXTENT: f32 = f32::from_bits(1061625612);
#[cfg(feature = "ee2-valley164-forest-play")]
const PLAY_MAGIC: [u8; 4] = *b"VFP1";
#[cfg(feature = "ee2-valley166-forest-clock")]
const CLOCK_MAGIC: [u8; 4] = *b"VFP2";

#[cfg(feature = "ee2-valley166-forest-clock")]
fn read_time(bytes: &[u8]) -> Option<Option<f32>> {
    let word = u32::from_le_bytes(bytes.try_into().ok()?);
    if word == u32::MAX { return Some(None); }
    let time = f32::from_bits(word);
    (time.is_finite() && !time.is_sign_negative()).then_some(Some(time))
}

pub(super) fn bind_heights(mut world: Geometry) -> Option<Geometry> {
    let Some(forest) = &world.forest else { return Some(world) };
    let mut rows = forest.source_rows.clone();
    for row in &mut rows {
        let [x, y, _] = row.position();
        let height = world.surface_height(&world.heights, x, y) * 128.0;
        if !height.is_finite() || !(0.0..65536.0).contains(&height) {
            return None;
        }
        row.position[2] = height as u16;
    }
    world.forest.as_mut()?.source_rows = rows;
    Some(world)
}

#[derive(Clone, Copy)]
pub(super) struct Promoted {
    pub(super) actor: u32,
    pub(super) wood: f32,
    #[cfg(feature = "ee2-valley166-forest-clock")]
    pub(super) last_chop: Option<f32>,
    #[cfg(feature = "ee2-valley166-forest-clock")]
    pub(super) exhaustion: Option<f32>,
}

pub(super) struct ForestReplay {
    pub(super) registry: ForestRegistry,
    pub(super) promoted: Vec<Option<Promoted>>,
    #[cfg(feature = "ee2-valley164-forest-play")]
    pub(super) playable: bool,
}

impl ForestReplay {
    #[cfg(feature = "ee2-rect235-forest")]
    pub(super) fn playable_rows(rows: &[super::forest_registry::ForestRow]) -> Self {
        Self {
            registry: ForestRegistry::new(rows.iter().copied()),
            promoted: vec![None; rows.len()],
            playable: true,
        }
    }

    pub(super) fn new(source: &SourceForestGrid) -> Self {
        Self {
            registry: ForestRegistry::new(source.source_rows.iter().copied()),
            promoted: vec![None; source.source_rows.len()],
            #[cfg(feature = "ee2-valley164-forest-play")]
            playable: false,
        }
    }

    pub(super) fn promote(&mut self, id: ForestId, actor: u32, wood: f32) -> Option<()> {
        if actor == 0 || actor == u32::MAX || !(0.0..=WOOD).contains(&wood)
            || self.promoted.iter().flatten().any(|row| row.actor == actor) {
            return None;
        }
        let slot = self.promoted.get_mut(id.0 as usize)?;
        self.registry.take(id)?;
        *slot = Some(Promoted { actor, wood,
            #[cfg(feature = "ee2-valley166-forest-clock")]
            last_chop: None,
            #[cfg(feature = "ee2-valley166-forest-clock")]
            exhaustion: None,
        });
        Some(())
    }

    #[cfg(feature = "ee2-valley166-forest-clock")]
    pub(super) fn has_timing(&self) -> bool {
        self.playable && self.promoted.iter().flatten().any(|row| row.last_chop.is_some())
    }

    #[cfg(feature = "ee2-valley166-forest-clock")]
    pub(super) fn timing_within(&self, now: f32) -> bool {
        self.promoted.iter().flatten().all(|row| row.last_chop.is_none_or(|time| time <= now))
    }

    fn obstruction(&self, dimensions: [u32; 2]) -> Option<SourceForestGrid> {
        let mut grid = SourceForestGrid::new(dimensions[0] as usize, dimensions[1] as usize)?;
        for index in 0..self.promoted.len() {
            if let Some(row) = self.registry.get(ForestId(index as u32)) {
                let [x, y, _] = row.position;
                grid.register([x, y], [EXTENT; 2], row.flags, row.state)?;
            }
        }
        Some(grid)
    }

    pub(super) fn encode(&self) -> Vec<u8> {
        #[cfg(feature = "ee2-valley164-forest-play")]
        if self.playable {
            let magic = PLAY_MAGIC;
            #[cfg(feature = "ee2-valley166-forest-clock")]
            let timed = self.has_timing();
            #[cfg(feature = "ee2-valley166-forest-clock")]
            let magic = if timed { CLOCK_MAGIC } else { magic };
            let mut bytes = magic.to_vec();
            bytes.extend_from_slice(&(self.promoted.len() as u16).to_le_bytes());
            bytes.extend_from_slice(&(self.promoted.iter().flatten().count() as u16).to_le_bytes());
            for (index, row) in self.promoted.iter().enumerate() {
                if let Some(row) = row {
                    bytes.extend_from_slice(&(index as u32).to_le_bytes());
                    bytes.extend_from_slice(&row.wood.to_bits().to_le_bytes());
                    #[cfg(feature = "ee2-valley166-forest-clock")]
                    if timed {
                        for time in [row.last_chop, row.exhaustion] {
                            bytes.extend_from_slice(&time.map_or(u32::MAX, f32::to_bits).to_le_bytes());
                        }
                    }
                }
            }
            return bytes;
        }
        let mut bytes = MAGIC.to_vec();
        let count = self.promoted.iter().flatten().count() as u16;
        bytes.extend_from_slice(&(self.promoted.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        for (index, row) in self.promoted.iter().enumerate() {
            if let Some(row) = row {
                bytes.extend_from_slice(&(index as u16).to_le_bytes());
                bytes.extend_from_slice(&(row.wood as u16).to_le_bytes());
                bytes.extend_from_slice(&row.actor.to_le_bytes());
            }
        }
        bytes
    }

    pub(super) fn decode(source: &SourceForestGrid, bytes: &[u8], playable: bool, timed: bool) -> Option<Self> {
        let magic = match (playable, timed) {
            (false, false) => MAGIC,
            #[cfg(feature = "ee2-valley164-forest-play")]
            (true, false) => PLAY_MAGIC,
            #[cfg(feature = "ee2-valley166-forest-clock")]
            (true, true) => CLOCK_MAGIC,
            _ => return None,
        };
        if !bytes.starts_with(&magic) { return None }
        let rows = u16::from_le_bytes(bytes.get(4..6)?.try_into().ok()?) as usize;
        let count = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as usize;
        let stride = if timed { 16 } else { 8 };
        if rows != source.source_rows.len() || ((!playable || timed) && count == 0) || count > rows || bytes.len() != 8 + count * stride {
            return None;
        }
        let mut replay = Self::new(source);
        #[cfg(feature = "ee2-valley164-forest-play")]
        { replay.playable = playable; }
        let mut previous = None;
        for entry in bytes[8..].chunks_exact(stride) {
            #[cfg(feature = "ee2-valley164-forest-play")]
            if playable {
                let index = u32::from_le_bytes(entry[..4].try_into().ok()?);
                let wood = f32::from_le_bytes(entry[4..8].try_into().ok()?);
                if previous.is_some_and(|last| index <= last) || index as usize >= rows
                    || !(wood.to_bits() == 0 || (1.0..=WOOD).contains(&wood)) { return None }
                previous = Some(index);
                replay.promote(ForestId(index), super::forest_play164::actor_id(index), wood)?;
                #[cfg(feature = "ee2-valley166-forest-clock")]
                if timed {
                    let last_chop = read_time(&entry[8..12])?;
                    let exhaustion = read_time(&entry[12..16])?;
                    // Current playable trees deplete only on the tick that sets the chop clock.
                    // Old VFP1 rows can be loaded into this same sidecar with both clocks absent.
                    if exhaustion != if wood == 0.0 { last_chop } else { None } { return None; }
                    let actor = replay.promoted[index as usize].as_mut()?;
                    actor.last_chop = last_chop;
                    actor.exhaustion = exhaustion;
                }
                continue;
            }
            let index = u32::from(u16::from_le_bytes(entry[..2].try_into().ok()?));
            if previous.is_some_and(|last| index <= last) { return None }
            previous = Some(index);
            let wood = u16::from_le_bytes(entry[2..4].try_into().ok()?);
            let actor = u32::from_le_bytes(entry[4..].try_into().ok()?);
            replay.promote(ForestId(index), actor, f32::from(wood))?;
        }
        #[cfg(feature = "ee2-valley166-forest-clock")]
        if timed && !replay.has_timing() { return None; }
        Some(replay)
    }
}

#[no_mangle]
pub extern "C" fn valley163_source_tree(row: u32, field: u32) -> u32 {
    let Some(tree) = active_game().and_then(|game| {
        game.map.source_world.as_deref()?.forest.as_ref()?.source_rows.get(row as usize)
    }) else { return u32::MAX };
    match field {
        0..=2 => u32::from(tree.position[field as usize]),
        3 => u32::from(tree.profile),
        4 => u32::from(tree.flags),
        5 => u32::from(tree.state),
        _ => u32::MAX,
    }
}

#[no_mangle]
pub extern "C" fn valley163_qa_promote(row: u32, actor: u32) -> u32 {
    if active_game().is_none() { return 0 }
    let game = crate::g();
    if game.net || game.humans != 1 || game.n_ai != 1 { return 0 }
    #[cfg(feature = "ee2-valley164-forest-play")]
    if game.valley_forest_play().is_some() { return 0 }
    let Some(fixture) = game.valley_fixture.as_mut().filter(|fixture| {
        fixture.native_house_assigned_clearance && (fixture.neutral_opening || fixture.forest_replay.is_some())
    }) else { return 0 };
    let Some(source) = game.map.source_world.as_deref().and_then(|world| world.forest.as_ref()) else { return 0 };
    if fixture.forest_replay.is_none() {
        let mut replay = ForestReplay::new(source);
        if replay.promote(ForestId(row), actor, WOOD).is_none() { return 0 }
        fixture.forest_replay = Some(replay);
        return 1;
    }
    fixture.forest_replay.as_mut().unwrap().promote(ForestId(row), actor, WOOD).is_some().into()
}

#[no_mangle]
pub extern "C" fn valley163_promoted_tree(row: u32, field: u32) -> u32 {
    let Some(state) = active_game().and_then(|game| {
        game.valley_fixture.as_ref()?.forest_replay.as_ref()?.promoted.get(row as usize)?.as_ref()
    }) else { return u32::MAX };
    match field {
        0 => state.actor,
        1 => state.wood as u32,
        _ => u32::MAX,
    }
}

#[no_mangle]
pub extern "C" fn valley163_compressed_obstruction(x: i32, y: i32) -> u32 {
    let Some(game) = active_game() else { return u32::MAX };
    let Some(source) = game.map.source_world.as_deref().and_then(|world| world.forest.as_ref()) else { return u32::MAX };
    let Some(replay) = game.valley_fixture.as_ref().and_then(|fixture| fixture.forest_replay.as_ref()) else {
        return source.point_flags(x, y);
    };
    replay.obstruction(source.dimensions()).map_or(u32::MAX, |grid| grid.point_flags(x, y))
}
