//! Staged native mode 2 archive data. A fixture with owned actors is still required to commit.
use super::{forest_io::{Chunk, NO_LINK}, geometry::Geometry, prepare_map, STAGE};
use crate::{game::Game, map::{tile_of, Map, R_GOLD, R_METAL, R_NONE, R_OIL, R_SALTPETER, R_STONE, R_TIN, R_URANIUM}};

struct Actor {
    source_id: u32,
    xyz: [f32; 3],
    yaw: f32,
    health: f32,
    flags: u32,
    visibility: u32,
    resource: u8,
    #[cfg(feature = "ee2-valley190-point-move")]
    scenery: Option<super::scenery::Footprint>,
}

pub(super) struct World0 {
    actors: Vec<Actor>,
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

impl Actor {
    fn decode(data: &[u8], width: usize, height: usize) -> Option<Self> {
        let source_id = word(data, 0)?;
        let xyz = [4, 8, 12].map(|at| word(data, at).map(f32::from_bits));
        let xyz = [xyz[0]?, xyz[1]?, xyz[2]?];
        let yaw = f32::from_bits(word(data, 16)?);
        let health = f32::from_bits(word(data, 20)?);
        let name_end = 26 + u16::from_le_bytes(data.get(24..26)?.try_into().ok()?) as usize;
        let tail = data.get(data.len().checked_sub(16)?..)?;
        let flags = word(tail, 8)?;
        let visibility = word(tail, 12)?;
        if !(1..=41).contains(&source_id) || !(0.0..width as f32).contains(&xyz[0]) ||
            !(0.0..height as f32).contains(&xyz[1]) || !xyz[2].is_finite() || !yaw.is_finite() || health != 1.0 ||
            name_end > data.len() - 16 || word(tail, 0)? != u32::MAX || !f32::from_bits(word(tail, 4)?).is_finite() ||
            flags != 0 || visibility != u32::MAX { return None; }
        let name = data.get(26..name_end)?;
        let resource = match name {
            b"ResourceStone" => R_STONE,
            b"ResourceGold" => R_GOLD,
            b"ResourceTin" => R_TIN,
            b"ResourceIron" => R_METAL,
            b"ResourceSaltPeter" => R_SALTPETER,
            b"ResourceOil" => R_OIL,
            b"ResourceUranium" => R_URANIUM,
            b"CORPSE1" | b"CORPSE2" | b"CORPSE4" | b"DESERTROCKS_02" | b"DESERTROCKS_03" |
            b"DESERTROCKS_04" | b"DestroyedMill_12" | b"DestroyedTemple_10" | b"MIDDLE_SIGNPOST" |
            b"ResourceDummy" => R_NONE,
            _ => return None,
        };
        #[cfg(feature = "ee2-valley190-point-move")]
        let scenery = super::scenery::Footprint::from_archive(name, [xyz[0], xyz[1]], yaw);
        Some(Self { source_id, xyz, yaw, health, flags, visibility, resource,
            #[cfg(feature = "ee2-valley190-point-move")]
            scenery,
        })
    }

    fn tile(&self) -> usize { tile_of(self.xyz[0], self.xyz[1]) }
}

impl World0 {
    #[cfg(feature = "ee2-valley190-point-move")]
    pub(super) fn scenery(&self) -> Vec<super::scenery::Footprint> {
        self.actors.iter().filter_map(|actor| actor.scenery).collect()
    }

    #[cfg(feature = "ee2-valley54-menu-qa")]
    pub(super) fn source_rows(&self) -> impl Iterator<Item = super::start::SourceActor> + '_ {
        self.actors.iter().map(|actor| super::start::SourceActor {
            id: actor.source_id, owner: 0, role: if actor.resource == R_NONE { 0 } else { 2 },
            definition: u32::from(actor.resource), detail: if actor.resource == R_NONE { 0 } else { 1000 },
            xyz: actor.xyz, yaw: actor.yaw, health: actor.health,
        })
    }

    pub(super) fn decode(world: &Geometry) -> Option<Self> {
        let raw_len = word(&world.wire, 16)? as usize;
        let raw = world.wire.get(world.wire.len().checked_sub(raw_len)?..)?;
        let version = Chunk::read(raw, 14, raw.len())?;
        let root = Chunk::read(raw, version.sibling, raw.len())?;
        if root.tag != b"Wrld" || word(raw, root.payload)? != 2 { return None; }
        let inner = root.find_unique(raw, b"Wrld")?;
        let player = inner.find_unique(raw, b"Plyr")?;
        if word(raw, player.payload)? != 0 { return None; }
        let mut actors = Vec::with_capacity(41);
        let mut next = inner.child;
        while next != NO_LINK {
            let chunk = Chunk::read(raw, next, inner.end)?;
            if chunk.tag == b"Unit" {
                let actor = Actor::decode(raw.get(chunk.payload..chunk.end)?, world.width, world.height)?;
                if actors.len() >= 41 || actors.iter().any(|row: &Actor| row.source_id == actor.source_id) { return None; }
                actors.push(actor);
            }
            next = chunk.sibling;
        }
        (actors.len() == 41).then_some(Self { actors })
    }

    pub(super) fn bind_resources(&self, map: &mut Map) -> Option<()> {
        let mut counts = [0u8; R_URANIUM as usize + 1];
        for actor in &self.actors {
            counts[actor.resource as usize] += 1;
            if actor.resource == R_NONE { continue; }
            let tile = actor.tile();
            if map.res[tile] != R_NONE { return None; }
            map.res[tile] = actor.resource;
            map.amt[tile] = 1000;
        }
        (counts == [13, 0, 0, 4, 4, 4, 4, 4, 4, 4]).then_some(())
    }
}

#[no_mangle]
pub extern "C" fn valley47_prepare_archive() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        let Some((archive, game)) = Geometry::decode(&stage.input).and_then(|world| {
            let archive = World0::decode(&world)?;
            let mut map = prepare_map(world)?;
            archive.bind_resources(&mut map)?;
            Some((archive, Game::blank(map, 0, 1, 0, 1, 0, 2, false)))
        }) else { return 0; };
        stage.game = Some(game);
        stage.archive = Some(archive);
        #[cfg(feature = "ee2-valley52-owned-qa")]
        { stage.owned = None; }
        1
    })
}

#[no_mangle]
pub extern "C" fn valley47_archive_word(slot: usize, field: u32) -> u32 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some(row) = stage.archive.as_ref().and_then(|archive| archive.actors.get(slot)) else { return u32::MAX; };
        let Some(game) = stage.game.as_ref() else { return u32::MAX; };
        match field {
            0 => row.source_id,
            1..=3 => row.xyz[field as usize - 1].to_bits(),
            4 => row.yaw.to_bits(),
            5 => row.health.to_bits(),
            6 => row.flags,
            7 => row.visibility,
            8 => u32::from(row.resource),
            9 => u32::from(game.map.res[row.tile()]),
            10 => u32::from(game.map.amt[row.tile()]),
            _ => u32::MAX,
        }
    })
}
