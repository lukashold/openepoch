//! QA-only Game construction with scoped mode 2 replay rows. No match commit.
use super::{archived_population::World0, geometry::Geometry, prepare_map, STAGE, VALLEY_SHA};
use crate::defs::{DefId, TC, WORKER};
use crate::game::{Game, Player, NOIDX};
use crate::map::tile_of;
use crate::game::BirthIdentity;

const SCOUT: DefId = 57;
const IDS: [u32; 6] = [0x0800002a, 0x0800002b, 0x0800002c, 0x1000002d, 0x1000002e, 0x1000002f];
const HEADER: usize = 64;
const SIZE: usize = HEADER + IDS.len() * 28;
const FOG_STRIDE: u32 = 4;
const FOG_ROWS: u32 = 71;

fn point_probe(x: f32, y: f32) -> Option<[u32; 2]> {
    // 005DDC77 adds this binary64 bias before the 00ACC7F0 shift.
    let cell = |position: f32| ((f64::from(position) + 103_079_215_104.0).to_bits() >> 16) as i32;
    let x = u32::try_from(cell(x)).ok()?;
    let y = u32::try_from(cell(y)).ok()?;
    let column = x >> 5;
    (column < FOG_STRIDE && y < FOG_ROWS).then_some([y * FOG_STRIDE + column, 0x8000_0000 >> (x & 31)])
}

#[derive(Clone, Copy)]
enum Type { CityCenter = 1, CitizenM, CitizenF, Scout }

impl Type {
    fn definition(self) -> DefId {
        match self {
            Self::CityCenter => TC,
            Self::CitizenM | Self::CitizenF => WORKER,
            Self::Scout => SCOUT,
        }
    }
}

struct Row {
    id: u32,
    kind: Type,
    xyz: [f32; 3],
    yaw: f32,
    region: i32,
    handle: u32,
}

pub(super) struct Owned {
    rows: Vec<Row>,
}

fn word(bytes: &[u8], index: usize) -> u32 {
    let offset = index * 4;
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

impl Owned {
    fn decode(bytes: &[u8], world: &Geometry) -> Option<Self> {
        if bytes.len() != SIZE || bytes[..4] != *b"VOW1" || word(bytes, 1) != 1 ||
            bytes[32..HEADER] != VALLEY_SHA || word(bytes, 2) != 6 || word(bytes, 3) != 47 ||
            (4..8).any(|index| word(bytes, index) != 0) { return None; }
        let mut rows = Vec::with_capacity(IDS.len());
        for (slot, (bytes, id)) in bytes[HEADER..].chunks_exact(28).zip(IDS).enumerate() {
            if word(bytes, 0) != id { return None; }
            let kind = match (slot % 3, word(bytes, 1)) {
                (0, 1) => Type::CityCenter,
                (1, 2) => Type::CitizenM,
                (1, 3) => Type::CitizenF,
                (2, 4) => Type::Scout,
                _ => return None,
            };
            let xyz = [2, 3, 4].map(|index| f32::from_bits(word(bytes, index)));
            let [x, y, z] = xyz;
            let yaw = f32::from_bits(word(bytes, 5));
            let region = word(bytes, 6) as i32;
            if !(0.0..world.width as f32).contains(&x) || !(0.0..world.height as f32).contains(&y) ||
                !z.is_finite() || !yaw.is_finite() { return None; }
            let is_city = matches!(kind, Type::CityCenter);
            if is_city {
                let point = y as usize * world.width + x as usize;
                if region < 0 || world.territory_ids[point] != region { return None; }
            } else if region != -1 { return None; }
            rows.push(Row {id, kind, xyz, yaw, region, handle: u32::MAX});
        }
        if rows[0].region == rows[3].region { return None; }
        Some(Self { rows })
    }

    fn bind(&mut self, game: &mut Game) -> Option<()> {
        for team in 0..2 {
            game.players[team] = Player {res: [400.0, 400.0, 200.0], population_limit: 250, alive: true,
                ai: team == 1, ai_next_attack: 20 * 240, ..Player::default()};
            game.extra_resources[team] = [400.0, 400.0, 200.0, 200.0, 200.0, 200.0];
            game.players[team].recompute();
        }
        if !game.configure_civilizations([4, 8, 14, 14]) { return None; }
        for row in &mut self.rows {
            let team = (row.id >> 27) as u8 - 1;
            let [x, y, _] = row.xyz;
            let identity = match row.kind {
                Type::CitizenM => BirthIdentity::Male,
                Type::CitizenF => BirthIdentity::Female,
                _ => BirthIdentity::NonCitizen,
            };
            let slot = game.spawn_oriented_with_identity(row.kind.definition(), team, x, y, row.yaw, identity);
            if slot == NOIDX { return None; }
            row.handle = game.h(slot);
            match row.kind {
                Type::CityCenter => game.map.starts[team as usize] = (x as usize, y as usize),
                Type::CitizenM | Type::CitizenF | Type::Scout => {},
            }
        }
        game.build_grid();
        Some(())
    }
}

#[cfg(feature = "ee2-valley54-menu-qa")]
pub(super) fn authenticated_menu_fixture(data: &[u8], world: &Geometry) -> Option<super::start::Start> {
    use super::start::{SourceActor, Start};
    const QA_SITE_ZERO: [u8; 32] = [
        0x68, 0xca, 0x55, 0xd5, 0x81, 0xc3, 0xe8, 0x55, 0xa1, 0x7a, 0xad, 0x58, 0x46, 0x15, 0x75, 0xf9,
        0xa1, 0x60, 0xa9, 0xc6, 0xbd, 0x8f, 0xa9, 0x0e, 0x08, 0x1c, 0xbe, 0xce, 0xe0, 0x5b, 0xfe, 0xfc,
    ];
    let input_sha = super::hash::sha256(data);
    if input_sha != QA_SITE_ZERO { return None; }
    let owned = Owned::decode(data, world)?;
    let archive = World0::decode(world)?;
    let mut rows = Vec::with_capacity(47);
    rows.extend(archive.source_rows());
    rows.extend(owned.rows.into_iter().map(|row| SourceActor {
        id: row.id, owner: row.id >> 27, role: 1, definition: u32::from(row.kind.definition()),
        detail: u32::from(matches!(row.kind, Type::CitizenF)), xyz: row.xyz, yaw: row.yaw, health: 1.0,
    }));
    Some(Start {site: 0, checkpoint: 0, rng: 0, counter: 47, input_sha, rows, authored_test: true})
}

#[no_mangle]
pub extern "C" fn valley52_owned_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.owned_input.clear();
        if length != SIZE { return std::ptr::null_mut(); }
        stage.owned_input.resize(length, 0);
        stage.owned_input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn valley52_prepare_owned() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        let Some((archive, owned, game)) = Geometry::decode(&stage.input).and_then(|world| {
            let archive = World0::decode(&world)?;
            let mut owned = Owned::decode(&stage.owned_input, &world)?;
            let mut map = prepare_map(world)?;
            archive.bind_resources(&mut map)?;
            let mut game = Game::blank(map, 0, 1, 0, 1, 0, 2, false);
            owned.bind(&mut game)?;
            Some((archive, owned, game))
        }) else { return 0; };
        stage.game = Some(game);
        stage.archive = Some(archive);
        stage.owned = Some(owned);
        1
    })
}

#[no_mangle]
pub extern "C" fn valley52_owned_word(slot: usize, field: u32) -> u32 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some(row) = stage.owned.as_ref().and_then(|owned| owned.rows.get(slot)) else { return u32::MAX; };
        let Some(game) = stage.game.as_ref() else { return u32::MAX; };
        let Some(index) = game.get(row.handle) else { return u32::MAX; };
        let actor = &game.ents[index];
        match field {
            0 => row.id,
            1 => row.kind as u32,
            2 => row.handle,
            3 => u32::from(actor.def),
            4 => u32::from(actor.team),
            5 => actor.x.to_bits(),
            6 => actor.y.to_bits(),
            7 => row.xyz[2].to_bits(),
            8 => actor.rot.to_bits(),
            9 => actor.hp.to_bits(),
            10 => row.region as u32,
            11 => u32::from(game.civilizations[actor.team as usize]),
            12 => u32::from(game.map.blocked[tile_of(actor.x, actor.y)]),
            13 if actor.def == WORKER => game.source_birth.slots[index].identity,
            _ => u32::MAX,
        }
    })
}

#[no_mangle]
pub extern "C" fn valley52_mobile_probe(slot: usize, field: usize) -> u32 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some(row) = stage.owned.as_ref().and_then(|owned| owned.rows.get(slot)) else { return u32::MAX; };
        if matches!(row.kind, Type::CityCenter) { return u32::MAX; }
        let Some(game) = stage.game.as_ref() else { return u32::MAX; };
        let Some(index) = game.get(row.handle) else { return u32::MAX; };
        let actor = &game.ents[index];
        point_probe(actor.x, actor.y).and_then(|probe| probe.get(field).copied()).unwrap_or(u32::MAX)
    })
}
