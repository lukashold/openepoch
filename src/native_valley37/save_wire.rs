//! Authored save locator. Only authenticated private input digests are stored.
use super::{hash::sha256, FixtureState};
use crate::game::Game;

pub(super) const MAGIC: u32 = u32::from_le_bytes(*b"V37S");
const HEADER: usize = 76;
const CHECKSUM: usize = 32;
#[cfg(feature = "ee2-valley198-waypoint-save")]
pub(super) const WAYPOINT_ROUTE_VERSION: u32 = 14;
pub(super) const VERSION: u32 = 1;
#[cfg(feature = "ee2-valley71-neutral-qa")]
pub(super) const NEUTRAL_OPENING_VERSION: u32 = 2;
#[cfg(feature = "ee2-valley74-city-footprint")]
const CITY_MASK_VERSION: u32 = 3;
#[cfg(feature = "ee2-valley74-city-footprint")]
const CITY_MASK_NEUTRAL_VERSION: u32 = 4;
#[cfg(feature = "ee2-valley74-city-footprint")]
const HOUSE_CONSTRUCTION_VERSION: u32 = 5;
#[cfg(feature = "ee2-valley74-city-footprint")]
const HOUSE_CONSTRUCTION_NEUTRAL_VERSION: u32 = 6;
#[cfg(feature = "ee2-valley74-city-footprint")]
const HOUSE_MOVEMENT_VERSION: u32 = 7;
#[cfg(feature = "ee2-valley74-city-footprint")]
const HOUSE_MOVEMENT_NEUTRAL_VERSION: u32 = 8;
#[cfg(feature = "ee2-valley74-city-footprint")]
const HOUSE_ASSIGNED_CLEARANCE_VERSION: u32 = 9;
#[cfg(feature = "ee2-valley74-city-footprint")]
const HOUSE_ASSIGNED_CLEARANCE_NEUTRAL_VERSION: u32 = 10;
#[cfg(feature = "ee2-valley74-city-footprint")]
const CITY_FOUNDATION_VERSION: u32 = 11;
#[cfg(feature = "ee2-valley74-city-footprint")]
const CITY_FOUNDATION_NEUTRAL_VERSION: u32 = 12;
#[cfg(feature = "ee2-valley163-forest-qa")]
const FOREST_REPLAY_VERSION: u32 = 13;
#[cfg(feature = "ee2-valley163-forest-qa")]
const FOREST: u32 = 1;
#[cfg(feature = "ee2-valley163-forest-qa")]
const CITY: u32 = 2;
#[cfg(feature = "ee2-valley163-forest-qa")]
const NEUTRAL: u32 = 4;
#[cfg(feature = "ee2-valley164-forest-play")]
const FOREST_PLAY: u32 = 8;
#[cfg(feature = "ee2-valley166-forest-clock")]
const FOREST_CLOCK: u32 = 16;

#[cfg(feature = "ee2-valley163-forest-qa")]
pub(super) fn forest_flags(fixture: &FixtureState) -> u32 {
    let flags = FOREST
        | if fixture.native_city_foundation { CITY } else { 0 }
        | if fixture.neutral_opening { NEUTRAL } else { 0 };
    #[cfg(feature = "ee2-valley164-forest-play")]
    if let Some(replay) = fixture.forest_replay.as_ref().filter(|replay| replay.playable) {
        #[cfg(feature = "ee2-valley166-forest-clock")]
        if replay.has_timing() { return flags | FOREST_PLAY | FOREST_CLOCK; }
        let _ = replay;
        return flags | FOREST_PLAY;
    }
    flags
}

pub(crate) struct Locator {
    pub(super) map: [u8; 32],
    pub(super) fixture: [u8; 32],
    #[cfg(feature = "ee2-valley198-waypoint-save")]
    pub(crate) route_resume: Option<super::route_save::Resume>,
    #[cfg(feature = "ee2-valley71-neutral-qa")]
    pub(super) neutral_opening: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_city_footprints: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_house_construction: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_house_movement: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_house_assigned_clearance: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_city_foundation: bool,
    #[cfg(feature = "ee2-valley163-forest-qa")]
    pub(super) forest_replay: Option<Vec<u8>>,
    #[cfg(feature = "ee2-valley164-forest-play")]
    pub(super) forest_play: bool,
    #[cfg(feature = "ee2-valley166-forest-clock")]
    pub(super) forest_clock: bool,
}

pub(super) fn version(_fixture: &FixtureState) -> u32 {
    #[cfg(feature = "ee2-valley163-forest-qa")]
    if _fixture.forest_replay.is_some() { return FOREST_REPLAY_VERSION; }
    #[cfg(feature = "ee2-valley74-city-footprint")]
    if _fixture.native_city_foundation {
        return if _fixture.neutral_opening { CITY_FOUNDATION_NEUTRAL_VERSION } else { CITY_FOUNDATION_VERSION };
    }
    #[cfg(feature = "ee2-valley74-city-footprint")]
    if _fixture.native_house_assigned_clearance {
        return if _fixture.neutral_opening { HOUSE_ASSIGNED_CLEARANCE_NEUTRAL_VERSION } else { HOUSE_ASSIGNED_CLEARANCE_VERSION };
    }
    #[cfg(feature = "ee2-valley74-city-footprint")]
    if _fixture.native_house_movement {
        return if _fixture.neutral_opening { HOUSE_MOVEMENT_NEUTRAL_VERSION } else { HOUSE_MOVEMENT_VERSION };
    }
    #[cfg(feature = "ee2-valley74-city-footprint")]
    if _fixture.native_house_construction {
        return if _fixture.neutral_opening { HOUSE_CONSTRUCTION_NEUTRAL_VERSION } else { HOUSE_CONSTRUCTION_VERSION };
    }
    #[cfg(feature = "ee2-valley74-city-footprint")]
    if _fixture.native_city_footprints {
        return if _fixture.neutral_opening { CITY_MASK_NEUTRAL_VERSION } else { CITY_MASK_VERSION };
    }
    #[cfg(feature = "ee2-valley71-neutral-qa")]
    if _fixture.neutral_opening { return NEUTRAL_OPENING_VERSION; }
    VERSION
}

pub(crate) fn wrap_save(mut inner: Vec<u8>, game: &Game) -> Vec<u8> {
    let Some(map) = game.map.source_world.as_ref() else { return inner; };
    let Some(fixture) = game.valley_fixture.as_ref() else { return Vec::new(); };
    if inner.is_empty() || game.net || game.humans != 1 || game.n_ai != 1 { return Vec::new(); }
    let Ok(size) = u32::try_from(inner.len()) else { return Vec::new(); };
    let mut wire = Vec::with_capacity(HEADER + inner.len() + CHECKSUM);
    wire.extend_from_slice(&MAGIC.to_le_bytes());
    wire.extend_from_slice(&version(fixture).to_le_bytes());
    wire.extend_from_slice(&size.to_le_bytes());
    wire.extend_from_slice(&map.input_sha);
    wire.extend_from_slice(&fixture.input_sha());
    wire.append(&mut inner);
    #[cfg(feature = "ee2-valley163-forest-qa")]
    if let Some(replay) = &fixture.forest_replay {
        wire.extend_from_slice(&forest_flags(fixture).to_le_bytes());
        wire.extend_from_slice(&replay.encode());
    }
    wire.extend_from_slice(&sha256(&wire));
    #[cfg(feature = "ee2-valley198-waypoint-save")]
    if let Some(sidecar) = super::route_save::encode(game, true) {
        let Ok(size) = u32::try_from(wire.len()) else { return Vec::new(); };
        let mut outer = Vec::with_capacity(HEADER + wire.len() + sidecar.len() + CHECKSUM);
        outer.extend_from_slice(&MAGIC.to_le_bytes());
        outer.extend_from_slice(&WAYPOINT_ROUTE_VERSION.to_le_bytes());
        outer.extend_from_slice(&size.to_le_bytes());
        outer.extend_from_slice(&wire[12..HEADER]);
        outer.append(&mut wire);
        outer.extend_from_slice(&sidecar);
        outer.extend_from_slice(&sha256(&outer));
        return outer;
    }
    #[cfg(feature = "ee2-valley198-waypoint-save")]
    if !game.valley_routes.active.is_empty() { return Vec::new(); }
    wire
}

pub(crate) fn split_save(wire: &[u8]) -> Option<(&[u8], Option<Locator>)> {
    #[cfg(feature = "ee2-valley198-waypoint-save")]
    if wire.starts_with(&MAGIC.to_le_bytes()) && wire.get(4..8) == Some(&WAYPOINT_ROUTE_VERSION.to_le_bytes()) {
        let size = u32::from_le_bytes(wire.get(8..12)?.try_into().ok()?) as usize;
        let body_end = wire.len().checked_sub(CHECKSUM)?;
        let end = HEADER.checked_add(size)?;
        if end >= body_end || sha256(&wire[..body_end]) != wire[body_end..] { return None; }
        let (inner, locator) = split_legacy_save(wire.get(HEADER..end)?)?;
        let mut locator = locator?;
        if locator.map != wire[12..44] || locator.fixture != wire[44..HEADER] { return None; }
        locator.route_resume = Some(super::route_save::Resume::decode(&wire[end..body_end])?);
        return Some((inner, Some(locator)));
    }
    split_legacy_save(wire)
}

fn split_legacy_save(wire: &[u8]) -> Option<(&[u8], Option<Locator>)> {
    if !wire.starts_with(&MAGIC.to_le_bytes()) { return Some((wire, None)); }
    if wire.len() < HEADER + CHECKSUM { return None; }
    let version = u32::from_le_bytes(wire[4..8].try_into().ok()?);
    match version {
        VERSION => (),
        #[cfg(feature = "ee2-valley71-neutral-qa")]
        NEUTRAL_OPENING_VERSION => (),
        #[cfg(feature = "ee2-valley74-city-footprint")]
        CITY_MASK_VERSION | CITY_MASK_NEUTRAL_VERSION | HOUSE_CONSTRUCTION_VERSION | HOUSE_CONSTRUCTION_NEUTRAL_VERSION |
        HOUSE_MOVEMENT_VERSION | HOUSE_MOVEMENT_NEUTRAL_VERSION |
        HOUSE_ASSIGNED_CLEARANCE_VERSION | HOUSE_ASSIGNED_CLEARANCE_NEUTRAL_VERSION |
        CITY_FOUNDATION_VERSION | CITY_FOUNDATION_NEUTRAL_VERSION => (),
        #[cfg(feature = "ee2-valley163-forest-qa")]
        FOREST_REPLAY_VERSION => (),
        _ => return None,
    }
    let size = u32::from_le_bytes(wire[8..12].try_into().ok()?) as usize;
    let end = HEADER.checked_add(size)?;
    #[cfg(feature = "ee2-valley163-forest-qa")]
    let (flags, forest_replay) = {
        let body_end = wire.len().checked_sub(CHECKSUM)?;
        if size == 0 || end > body_end || sha256(&wire[..body_end]) != wire[body_end..] { return None; }
        if version == FOREST_REPLAY_VERSION {
            let sidecar = &wire[end..body_end];
            let flags = u32::from_le_bytes(sidecar.get(..4)?.try_into().ok()?);
            let allowed = FOREST | CITY | NEUTRAL;
            #[cfg(feature = "ee2-valley164-forest-play")]
            let allowed = allowed | FOREST_PLAY;
            let stride = 8;
            #[cfg(feature = "ee2-valley166-forest-clock")]
            let (allowed, stride) = {
                let timed = flags & FOREST_CLOCK != 0;
                if timed && flags & FOREST_PLAY == 0 { return None; }
                (allowed | FOREST_CLOCK, if timed { 16 } else { stride })
            };
            if !(12..=12 + 335 * stride).contains(&sidecar.len()) || flags & FOREST == 0 || flags & !allowed != 0 { return None; }
            (flags, Some(sidecar[4..].to_vec()))
        } else {
            if end != body_end { return None; }
            (0, None)
        }
    };
    #[cfg(not(feature = "ee2-valley163-forest-qa"))]
    if size == 0 || end.checked_add(CHECKSUM)? != wire.len() || sha256(&wire[..end]) != wire[end..] { return None; }
    #[cfg(feature = "ee2-valley71-neutral-qa")]
    let neutral_opening = version == NEUTRAL_OPENING_VERSION;
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let neutral_opening = neutral_opening || matches!(version, CITY_MASK_NEUTRAL_VERSION | HOUSE_CONSTRUCTION_NEUTRAL_VERSION |
        HOUSE_MOVEMENT_NEUTRAL_VERSION | HOUSE_ASSIGNED_CLEARANCE_NEUTRAL_VERSION | CITY_FOUNDATION_NEUTRAL_VERSION);
    #[cfg(feature = "ee2-valley163-forest-qa")]
    let neutral_opening = neutral_opening || flags & NEUTRAL != 0;
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let native_city_foundation = matches!(version, CITY_FOUNDATION_VERSION | CITY_FOUNDATION_NEUTRAL_VERSION);
    #[cfg(feature = "ee2-valley163-forest-qa")]
    let native_city_foundation = native_city_foundation || flags & CITY != 0;
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let native_house_assigned_clearance = native_city_foundation || matches!(version, HOUSE_ASSIGNED_CLEARANCE_VERSION | HOUSE_ASSIGNED_CLEARANCE_NEUTRAL_VERSION);
    #[cfg(feature = "ee2-valley163-forest-qa")]
    let native_house_assigned_clearance = native_house_assigned_clearance || forest_replay.is_some();
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let native_house_movement = native_house_assigned_clearance || matches!(version, HOUSE_MOVEMENT_VERSION | HOUSE_MOVEMENT_NEUTRAL_VERSION);
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let native_house_construction = native_house_movement || matches!(version, HOUSE_CONSTRUCTION_VERSION | HOUSE_CONSTRUCTION_NEUTRAL_VERSION);
    let locator = Locator {map: wire[12..44].try_into().ok()?, fixture: wire[44..76].try_into().ok()?,
        #[cfg(feature = "ee2-valley198-waypoint-save")]
        route_resume: None,
        #[cfg(feature = "ee2-valley71-neutral-qa")]
        neutral_opening,
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_city_footprints: native_house_construction || matches!(version, CITY_MASK_VERSION | CITY_MASK_NEUTRAL_VERSION),
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_house_construction,
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_house_movement,
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_house_assigned_clearance,
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_city_foundation,
        #[cfg(feature = "ee2-valley163-forest-qa")]
        forest_replay,
        #[cfg(feature = "ee2-valley164-forest-play")]
        forest_play: flags & FOREST_PLAY != 0,
        #[cfg(feature = "ee2-valley166-forest-clock")]
        forest_clock: flags & FOREST_CLOCK != 0,
    };
    Some((&wire[HEADER..end], Some(locator)))
}
