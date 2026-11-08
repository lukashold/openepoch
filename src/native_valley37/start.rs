//! Bind a verified local replay to ordinary owned actors and resource tiles.
use super::{hash::sha256, start_catalog::RECEIPT_SHA, VALLEY_SHA};
use crate::defs::{DefId, TC, WORKER};
use crate::game::{Game, Player, NOIDX};
use crate::map::{tile_of, NN, R_NONE, R_URANIUM};
use crate::native_rng9::{CitizenIdentity, NativeRng};

const MAGIC: u32 = u32::from_le_bytes(*b"VS37");
const HEADER: usize = 64;
const ROW_BYTES: usize = 40;
const OWNED_IDS: [u32; 6] = [0x0800002a, 0x0800002b, 0x0800002c, 0x1000003d, 0x1000003e, 0x1000003f];
const SCOUT: DefId = 57;

#[derive(Clone, Copy)]
pub(super) struct SourceActor {
    pub(super) id: u32,
    pub(super) owner: u32,
    pub(super) role: u32,
    pub(super) definition: u32,
    pub(super) detail: u32,
    pub(super) xyz: [f32; 3],
    pub(super) yaw: f32,
    pub(super) health: f32,
}

pub(super) struct Start {
    pub site: u32,
    pub checkpoint: u32,
    pub rng: u32,
    pub counter: u32,
    pub input_sha: [u8; 32],
    pub(super) rows: Vec<SourceActor>,
    pub(super) authored_test: bool,
}

pub(crate) struct FixtureState {
    start: Start,
    owned_handles: [u32; 6],
    resource_rows: Vec<u16>,
    #[cfg(feature = "ee2-valley71-neutral-qa")]
    pub(super) neutral_opening: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_city_footprints: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_house_construction: bool,
}

impl Start {
    pub(super) const MAX_WIRE: usize = HEADER + 134 * ROW_BYTES;

    pub(super) fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < HEADER || data.len() > Self::MAX_WIRE { return None; }
        let input_sha = sha256(data);
        if !RECEIPT_SHA.contains(&input_sha) { return None; }
        let word = |i: usize| u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap());
        let [magic, version, site, checkpoint, rng, counter, count, resources] = std::array::from_fn(word);
        if magic != MAGIC || version != 1 || site > 1 || checkpoint > 1 || data[32..64] != VALLEY_SHA ||
            ![133, 134].contains(&counter) || count != counter || resources != 90 || data.len() != HEADER + count as usize * ROW_BYTES {
            return None;
        }
        let mut rows = Vec::with_capacity(count as usize);
        let mut owned = Vec::with_capacity(6);
        let mut resource_tiles = std::collections::BTreeSet::new();
        for (i, row) in data[HEADER..].chunks_exact(ROW_BYTES).enumerate() {
            let word = |i: usize| u32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap());
            let [id, owner, role, definition, detail, x, y, z, yaw, health] = std::array::from_fn(word);
            let xyz = [x, y, z].map(f32::from_bits);
            let [x, y, z] = xyz;
            let yaw = f32::from_bits(yaw);
            let health = f32::from_bits(health);
            if id & 0x07ff_ffff != i as u32 + 1 || id >> 27 != owner || owner > 2 || !(0.0..115.0).contains(&x) ||
                !(0.0..71.0).contains(&y) || !z.is_finite() || !yaw.is_finite() || !(0.0..=1.0).contains(&health) { return None; }
            match (role, owner) {
                (0, 0) if definition == 0 && detail == 0 => {},
                (1, 1 | 2) if [TC as u32, WORKER as u32, SCOUT as u32].contains(&definition) &&
                    (detail == 0 || definition == WORKER as u32 && detail == 1) => owned.push((id, definition)),
                (2, 0) if (1..=R_URANIUM as u32).contains(&definition) && (1..=u16::MAX as u32).contains(&detail) => {
                    if !resource_tiles.insert(tile_of(x, y)) { return None; }
                }
                _ => return None,
            }
            rows.push(SourceActor {id, owner, role, definition, detail, xyz, yaw, health});
        }
        if !owned.into_iter().eq(OWNED_IDS.into_iter().zip([TC, WORKER, SCOUT, TC, WORKER, SCOUT].map(u32::from))) ||
            resource_tiles.len() != resources as usize { return None; }
        Some(Self {site, checkpoint, rng, counter, input_sha, rows, authored_test: false})
    }

    pub(super) fn bind(self, mut game: Game) -> Option<Game> {
        for team in 0..2 {
            game.players[team] = Player {res: [400.0, 400.0, 200.0], population_limit: 250, alive: true,
                ai: team == 1, ai_next_attack: 20 * 240, ..Player::default()};
            game.extra_resources[team] = [400.0, 400.0, 200.0, 200.0, 200.0, 200.0];
            game.players[team].recompute();
        }
        if !game.configure_civilizations([4, 8, 14, 14]) { return None; }
        let mut resource_rows = vec![0; NN];
        let mut owned_handles = [u32::MAX; 6];
        let mut owned_index = 0;
        for (index, row) in self.rows.iter().enumerate() {
            let [x, y, _] = row.xyz;
            let tile = tile_of(x, y);
            if row.role == 2 {
                if game.map.res[tile] != R_NONE { return None; }
                game.map.res[tile] = row.definition as u8;
                game.map.amt[tile] = row.detail as u16;
                resource_rows[tile] = (index + 1) as u16;
            } else if row.role == 1 {
                let team = row.owner as u8 - 1;
                let definition = row.definition as DefId;
                let actor = game.spawn_oriented(definition, team, x, y, row.yaw);
                if actor == NOIDX { return None; }
                game.ents[actor].hp *= row.health;
                let handle = game.h(actor);
                owned_handles[owned_index] = handle;
                owned_index += 1;
                if definition == WORKER {
                    game.set_citizen_identity(actor, if row.detail == 1 { CitizenIdentity::Female } else { CitizenIdentity::Male });
                }
                if definition == TC {
                    let region = game.map.terr[tile] as usize;
                    if region >= game.map.tseeds.len() { return None; }
                    if !self.authored_test { game.territory.claims[region] = handle; }
                    game.map.starts[team as usize] = (x as usize, y as usize);
                }
            }
        }
        if !self.authored_test { game.source_birth.rng = NativeRng::from_state(self.rng); }
        game.valley_fixture = Some(FixtureState {start: self, owned_handles, resource_rows,
            #[cfg(feature = "ee2-valley71-neutral-qa")]
            neutral_opening: false,
            #[cfg(feature = "ee2-valley74-city-footprint")]
            native_city_footprints: false,
            #[cfg(feature = "ee2-valley74-city-footprint")]
            native_house_construction: false,
        });
        game.rebuild_territories();
        game.build_grid();
        game.update_fog();
        Some(game)
    }
}

impl FixtureState {
    pub(super) fn input_sha(&self) -> [u8; 32] { self.start.input_sha }

    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) fn authored_city_handles(&self) -> Option<[u32; 2]> {
        self.start.authored_test.then_some([self.owned_handles[0], self.owned_handles[3]])
    }

    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) fn authored_city_pose(&self, handle: u32) -> Option<[f32; 3]> {
        if !self.authored_city_handles()?.contains(&handle) { return None; }
        let slot = self.owned_handles.iter().position(|&entry| entry == handle)?;
        let row = self.start.rows.iter().filter(|row| row.role == 1).nth(slot)?;
        (row.definition == u32::from(TC)).then_some([row.xyz[0], row.xyz[1], row.yaw])
    }

    pub(crate) fn focus(&self, team: u8) -> Option<[f32; 2]> {
        self.start.rows.iter().find(|row| row.owner == u32::from(team) + 1 && row.role == 1 && row.definition == u32::from(TC))
            .map(|row| [row.xyz[0], row.xyz[1]])
    }

    pub(crate) fn resource_xy(&self, tile: usize, kind: u8) -> Option<(f32, f32)> {
        let row = self.start.rows.get((self.resource_rows.get(tile)?.checked_sub(1)?) as usize)?;
        (row.definition == u32::from(kind)).then_some((row.xyz[0], row.xyz[1]))
    }

    pub(super) fn query(&self, field: u32) -> u32 {
        match field {
            0 => self.start.site,
            1 => self.start.checkpoint,
            2 => self.start.rng,
            3 => self.start.counter,
            4 => self.start.rows.iter().filter(|row| row.role == 0).count() as u32,
            5 => self.resource_rows.iter().filter(|&&row| row != 0).count() as u32,
            6 => u32::from(self.start.authored_test),
            _ => u32::MAX,
        }
    }

    pub(super) fn handle(&self, source_id: u32) -> u32 {
        self.start.rows.iter().filter(|row| row.role == 1).position(|row| row.id == source_id)
            .map_or(u32::MAX, |i| self.owned_handles[i])
    }

    pub(super) fn source_word(&self, slot: usize, field: u32) -> u32 {
        let Some(row) = self.start.rows.get(slot) else { return u32::MAX; };
        match field {
            0 => row.id,
            1 => row.owner,
            2 => row.role,
            3 => row.definition,
            4 => row.detail,
            5..=7 => row.xyz[field as usize - 5].to_bits(),
            8 => row.yaw.to_bits(),
            9 => row.health.to_bits(),
            _ => u32::MAX,
        }
    }
}
