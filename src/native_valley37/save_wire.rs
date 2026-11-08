//! Authored save locator. Only authenticated private input digests are stored.
use super::{hash::sha256, FixtureState};
use crate::game::Game;

pub(super) const MAGIC: u32 = u32::from_le_bytes(*b"V37S");
const HEADER: usize = 76;
const CHECKSUM: usize = 32;
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

pub(crate) struct Locator {
    pub(super) map: [u8; 32],
    pub(super) fixture: [u8; 32],
    #[cfg(feature = "ee2-valley71-neutral-qa")]
    pub(super) neutral_opening: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_city_footprints: bool,
    #[cfg(feature = "ee2-valley74-city-footprint")]
    pub(super) native_house_construction: bool,
}

pub(super) fn version(_fixture: &FixtureState) -> u32 {
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
    wire.extend_from_slice(&sha256(&wire));
    wire
}

pub(crate) fn split_save(wire: &[u8]) -> Option<(&[u8], Option<Locator>)> {
    if !wire.starts_with(&MAGIC.to_le_bytes()) { return Some((wire, None)); }
    if wire.len() < HEADER + CHECKSUM { return None; }
    let version = u32::from_le_bytes(wire[4..8].try_into().ok()?);
    match version {
        VERSION => (),
        #[cfg(feature = "ee2-valley71-neutral-qa")]
        NEUTRAL_OPENING_VERSION => (),
        #[cfg(feature = "ee2-valley74-city-footprint")]
        CITY_MASK_VERSION | CITY_MASK_NEUTRAL_VERSION | HOUSE_CONSTRUCTION_VERSION | HOUSE_CONSTRUCTION_NEUTRAL_VERSION => (),
        _ => return None,
    }
    let size = u32::from_le_bytes(wire[8..12].try_into().ok()?) as usize;
    let end = HEADER.checked_add(size)?;
    if size == 0 || end.checked_add(CHECKSUM)? != wire.len() || sha256(&wire[..end]) != wire[end..] { return None; }
    #[cfg(feature = "ee2-valley71-neutral-qa")]
    let neutral_opening = version == NEUTRAL_OPENING_VERSION;
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let neutral_opening = neutral_opening || matches!(version, CITY_MASK_NEUTRAL_VERSION | HOUSE_CONSTRUCTION_NEUTRAL_VERSION);
    #[cfg(feature = "ee2-valley74-city-footprint")]
    let native_house_construction = matches!(version, HOUSE_CONSTRUCTION_VERSION | HOUSE_CONSTRUCTION_NEUTRAL_VERSION);
    let locator = Locator {map: wire[12..44].try_into().ok()?, fixture: wire[44..76].try_into().ok()?,
        #[cfg(feature = "ee2-valley71-neutral-qa")]
        neutral_opening,
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_city_footprints: native_house_construction || matches!(version, CITY_MASK_VERSION | CITY_MASK_NEUTRAL_VERSION),
        #[cfg(feature = "ee2-valley74-city-footprint")]
        native_house_construction,
    };
    Some((&wire[HEADER..end], Some(locator)))
}
