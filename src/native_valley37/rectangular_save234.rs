//! Authored Death Valley2 checkpoint binding. Existing game trailers stay byte-for-byte.
use super::hash::sha256;
use super::population234_catalog::CARRIER_SHA;
use crate::{game::{Game, NONE}, map::{LEGACY_REGION_CAPACITY, MapLayout}};

pub(crate) const MAGIC: &[u8; 4] = b"DVS1";
const VERSION: u32 = 1;
const MUTABLE_VERSION: u32 = 2;
const WILDLIFE_VERSION: u32 = 3;
const DEER_VERSION: u32 = 4;
const DEER_FOOT_VERSION: u32 = 5;
const DEER_CONTACT_VERSION: u32 = 6;
pub(super) const ANIMAL_VERSION: u32 = 7;
const ANIMAL_SLOPE_VERSION: u32 = 8;
const ANIMAL_BIRTH_VERSION: u32 = 9;
const NATIVE_CARCASS_VERSION: u32 = 10;
const NATIVE_AIRCRAFT_VERSION: u32 = 11;
const AIRCRAFT_VISUAL_VERSION: u32 = 12;
const AI_CLOCK_VERSION: u32 = 13;
#[cfg(feature = "ee2-rect276-clearance-resume")]
const CLEARANCE_VERSION: u32 = 14;
#[cfg(feature = "ee2-rect280-farm-crop")]
const FARM_CROP_VERSION: u32 = 15;
#[cfg(feature = "ee2-rect280-farm-crop")]
const FARM_FAMILY_VERSION: u32 = 16;
#[cfg(feature = "ee2-rect280-farm-crop")]
const FARM_VISUAL_VERSION: u32 = 17;
#[cfg(feature = "ee2-rect280-farm-crop")]
const FARM_RECOVERY_VERSION: u32 = 18;
#[cfg(feature = "ee2-rect286-oil-visual")]
const OIL_VISUAL_VERSION: u32 = 19;
#[cfg(feature = "ee2-rect287-oil-economy")]
const OWNED_IDS_VERSION: u32 = 20;
#[cfg(feature = "ee2-rect289-citizen-work")]
const CITIZEN_WORK_VERSION: u32 = 21;
const AI_CLOCK_TAG: &[u8; 4] = b"DAI1";
pub(super) const DEER_TAG: &[u8; 4] = b"DVD1";
pub(super) const DEER_FOOT_TAG: &[u8; 4] = b"DVD2";
pub(super) const DEER_CONTACT_TAG: &[u8; 4] = b"DVD3";
const REGIONS: usize = 43;
const EXTRA: usize = REGIONS - LEGACY_REGION_CAPACITY;
const LAYOUT: [u32; 5] = [100, 300, 128, 512, REGIONS as u32];
const FIXED: usize = 4 + 4 + 4 + LAYOUT.len() * 4 + 3 * 32 + EXTRA * 4;
const DIGEST: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Binding {
    pub source: [u8; 32],
    pub terrain: [u8; 32],
    pub population: [u8; 32],
}

pub(crate) fn is_rectangular(game: &Game) -> bool {
    game.map.layout == MapLayout::DEATH_VALLEY2
}

pub(crate) fn binding(game: &Game) -> Option<Binding> {
    if !is_rectangular(game) || game.net || game.humans != 1 || !(1..=3).contains(&game.n_ai) ||
        game.map.tseeds.len() != REGIONS
    {
        return None;
    }
    let world = game.map.source_world.as_deref()?;
    let population = game.rect_population.as_ref()?.source();
    if !super::population234::Population::map_matches(world.source_sha, world.input_sha)
        || population.units().len() != world.pending_units as usize
        || population.forest().len() != world.pending_forest as usize
    {
        return None;
    }
    Some(Binding { source: world.source_sha, terrain: world.input_sha, population: CARRIER_SHA })
}

pub(crate) struct Locator<'a> {
    pub binding: Binding,
    pub claims: [u32; EXTRA],
    pub stock: Option<&'a [u8]>,
    pub forest: Option<&'a [u8]>,
    pub wildlife: Option<&'a [u8]>,
    pub deer: Option<&'a [u8]>,
    pub animals: Option<&'a [u8]>,
    #[cfg(feature = "ee2-rect261-ai-clock")]
    pub ai_clock: Option<&'a [u8]>,
    #[cfg(feature = "ee2-rect276-clearance-resume")]
    pub clearance: Option<&'a [u8]>,
    #[cfg(feature = "ee2-rect280-farm-crop")]
    pub farm_crops: Option<&'a [u8]>,
    #[cfg(feature = "ee2-rect286-oil-visual")]
    pub oil_visuals: Option<&'a [u8]>,
    #[cfg(feature = "ee2-rect287-oil-economy")]
    pub owned_ids: Option<&'a [u8]>,
    #[cfg(feature = "ee2-rect289-citizen-work")]
    pub citizen_work: Option<&'a [u8]>,
}

pub(super) fn deer_version(section: &[u8]) -> Option<u32> {
    match section.get(..4)? {
        tag if tag == DEER_TAG => Some(DEER_VERSION),
        tag if tag == DEER_FOOT_TAG => Some(DEER_FOOT_VERSION),
        tag if tag == DEER_CONTACT_TAG => Some(DEER_CONTACT_VERSION),
        _ => None,
    }
}

fn animal_version(section: &[u8]) -> Option<u32> {
    match section.get(..4)? {
        b"DVA1" => Some(ANIMAL_VERSION),
        b"DVA2" => Some(ANIMAL_SLOPE_VERSION),
        b"DVA3" => Some(ANIMAL_BIRTH_VERSION),
        b"DVA4" => Some(NATIVE_CARCASS_VERSION),
        b"DVA5" => Some(NATIVE_AIRCRAFT_VERSION),
        b"DVA6" => Some(AIRCRAFT_VISUAL_VERSION),
        _ => None,
    }
}

pub(crate) fn wrap(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5]) -> Option<Vec<u8>> {
    wrap_sections(payload, binding, claims, sections, None, None, None, None, None, None)
}

#[cfg(feature = "ee2-rect261-ai-clock")]
pub(crate) fn wrap_with_ai(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: Option<&[u8]>) -> Option<Vec<u8>> {
    if ai_clock.is_none() { return wrap(payload, binding, claims, sections); }
    wrap_sections(payload, binding, claims, sections, ai_clock, None, None, None, None, None)
}

#[cfg(feature = "ee2-rect276-clearance-resume")]
pub(crate) fn wrap_with_clearance(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: &[u8], clearance: &[u8]) -> Option<Vec<u8>> {
    if !clearance.starts_with(super::clearance_resume276::TAG) { return None; }
    wrap_sections(payload, binding, claims, sections, Some(ai_clock), Some(clearance), None, None, None, None)
}

#[cfg(feature = "ee2-rect280-farm-crop")]
fn farm_crop_version(section: &[u8]) -> Option<u32> {
    if section.starts_with(super::farm_crop280::TAG) { return Some(FARM_CROP_VERSION); }
    #[cfg(feature = "ee2-rect281-farm-family")]
    if section.starts_with(super::farm_crop280::FAMILY_TAG) { return Some(FARM_FAMILY_VERSION); }
    #[cfg(feature = "ee2-rect282-farm-seeding")]
    if section.starts_with(super::farm_crop280::VISUAL_TAG) { return Some(FARM_VISUAL_VERSION); }
    #[cfg(feature = "ee2-rect283-farm-recovery")]
    if section.starts_with(super::farm_crop280::RECOVERY_TAG) { return Some(FARM_RECOVERY_VERSION); }
    None
}

#[cfg(feature = "ee2-rect280-farm-crop")]
pub(crate) fn wrap_with_farm_crops(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: &[u8], clearance: Option<&[u8]>, crops: &[u8]) -> Option<Vec<u8>> {
    if clearance.is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG)) { return None; }
    wrap_sections(payload, binding, claims, sections, Some(ai_clock), clearance, Some(crops), None, None, None)
}

#[cfg(feature = "ee2-rect286-oil-visual")]
pub(crate) fn wrap_with_oil_visuals(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: &[u8], clearance: Option<&[u8]>, crops: Option<&[u8]>,
    oil_visuals: &[u8]) -> Option<Vec<u8>> {
    if clearance.is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG)) { return None; }
    wrap_sections(payload, binding, claims, sections, Some(ai_clock), clearance, crops, Some(oil_visuals), None, None)
}

#[cfg(feature = "ee2-rect287-oil-economy")]
pub(crate) fn wrap_with_owned_ids(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: &[u8], clearance: Option<&[u8]>, crops: Option<&[u8]>,
    oil_visuals: Option<&[u8]>, owned_ids: &[u8]) -> Option<Vec<u8>> {
    if clearance.is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG)) { return None; }
    wrap_sections(payload, binding, claims, sections, Some(ai_clock), clearance, crops, oil_visuals, Some(owned_ids), None)
}

#[cfg(feature = "ee2-rect289-citizen-work")]
pub(crate) fn wrap_with_citizen_work(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: &[u8], clearance: Option<&[u8]>, crops: Option<&[u8]>,
    oil_visuals: Option<&[u8]>, owned_ids: Option<&[u8]>, citizen_work: &[u8]) -> Option<Vec<u8>> {
    if clearance.is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG)) { return None; }
    wrap_sections(payload, binding, claims, sections, Some(ai_clock), clearance, crops, oil_visuals, owned_ids, Some(citizen_work))
}

fn wrap_sections(payload: Vec<u8>, binding: &Binding, claims: &[u32],
    sections: [Option<&[u8]>; 5], ai_clock: Option<&[u8]>, clearance: Option<&[u8]>,
    farm_crops: Option<&[u8]>, oil_visuals: Option<&[u8]>, owned_ids: Option<&[u8]>, citizen_work: Option<&[u8]>) -> Option<Vec<u8>> {
    let size = u32::try_from(payload.len()).ok()?;
    if size == 0 || claims.len() < REGIONS || claims[REGIONS..].iter().any(|&claim| claim != NONE) {
        return None;
    }
    if sections.into_iter().flatten().any(<[u8]>::is_empty) || sections[3].is_some() && sections[2].is_none() { return None; }
    let count = sections.iter().rposition(Option::is_some)
        .map_or(VERSION, |last| (last as u32 + 1).max(MUTABLE_VERSION));
    let version = if let Some(section) = sections[4] { animal_version(section)? } else {
        sections[3].map_or(Some(count), deer_version)?
    };
    let (version, count) = if let Some(clock) = ai_clock {
        if version != AIRCRAFT_VISUAL_VERSION || !clock.starts_with(AI_CLOCK_TAG)
            || sections[3].is_some_and(|deer| deer_version(deer).is_none()) { return None; }
        (AI_CLOCK_VERSION, 6)
    } else { (version, count) };
    #[cfg(feature = "ee2-rect276-clearance-resume")]
    let (version, count) = if clearance.is_some() {
        if version != AI_CLOCK_VERSION { return None; }
        (CLEARANCE_VERSION, 7)
    } else { (version, count) };
    #[cfg(feature = "ee2-rect280-farm-crop")]
    let (version, count) = if let Some(crops) = farm_crops {
        if !matches!(version, AI_CLOCK_VERSION | CLEARANCE_VERSION) { return None; }
        (farm_crop_version(crops)?, 8)
    } else { (version, count) };
    #[cfg(feature = "ee2-rect286-oil-visual")]
    let (version, count) = if let Some(oil) = oil_visuals {
        if !(AI_CLOCK_VERSION..=FARM_RECOVERY_VERSION).contains(&version)
            || !oil.starts_with(super::oil_visual286::TAG) { return None; }
        (OIL_VISUAL_VERSION, 9)
    } else { (version, count) };
    #[cfg(feature = "ee2-rect287-oil-economy")]
    let (version, count) = if let Some(ids) = owned_ids {
        if !(AI_CLOCK_VERSION..=OIL_VISUAL_VERSION).contains(&version)
            || !ids.starts_with(super::owned_ids287::TAG) { return None; }
        (OWNED_IDS_VERSION, 10)
    } else { (version, count) };
    #[cfg(feature = "ee2-rect289-citizen-work")]
    let (version, count) = if let Some(work) = citizen_work {
        if !(AI_CLOCK_VERSION..=OWNED_IDS_VERSION).contains(&version)
            || !work.starts_with(super::citizen_work289::TAG) { return None; }
        (CITIZEN_WORK_VERSION, 11)
    } else { (version, count) };
    let mut output = Vec::with_capacity(FIXED + payload.len() + DIGEST);
    output.extend_from_slice(MAGIC);
    for word in [version, size].into_iter().chain(LAYOUT) {
        output.extend_from_slice(&word.to_le_bytes());
    }
    for hash in [&binding.source, &binding.terrain, &binding.population] {
        output.extend_from_slice(hash);
    }
    for claim in &claims[LEGACY_REGION_CAPACITY..REGIONS] {
        output.extend_from_slice(&claim.to_le_bytes());
    }
    output.extend_from_slice(&payload);
    if version != VERSION {
        let sections = sections.into_iter().chain([ai_clock, clearance, farm_crops, oil_visuals, owned_ids, citizen_work]).take(count as usize)
            .map(Option::unwrap_or_default);
        for section in sections.clone() {
            output.extend_from_slice(&u32::try_from(section.len()).ok()?.to_le_bytes());
        }
        for section in sections { output.extend_from_slice(section); }
    }
    output.extend_from_slice(&sha256(&output));
    Some(output)
}

fn split_sections(wire: &[u8], end: usize, body_end: usize, count: usize) -> Option<[Option<&[u8]>; 11]> {
    let mut sections = [None; 11];
    let mut start = end.checked_add(count * 4)?;
    for (i, section) in sections[..count].iter_mut().enumerate() {
        let at = end.checked_add(i * 4)?;
        let len = u32::from_le_bytes(wire.get(at..at.checked_add(4)?)?.try_into().ok()?) as usize;
        let stop = start.checked_add(len)?;
        if len != 0 { *section = Some(wire.get(start..stop)?); }
        start = stop;
    }
    (start == body_end).then_some(sections)
}

pub(crate) fn split(wire: &[u8]) -> Option<(&[u8], Option<Locator<'_>>)> {
    if !wire.starts_with(MAGIC) {
        return Some((wire, None));
    }
    let word = |at: usize| Some(u32::from_le_bytes(wire.get(at..at + 4)?.try_into().ok()?));
    let version = word(4)?;
    let size = word(8)? as usize;
    let end = FIXED.checked_add(size)?;
    let body_end = wire.len().checked_sub(DIGEST)?;
    if size == 0 || end > body_end
        || LAYOUT.iter().enumerate().any(|(i, &expected)| word(12 + 4 * i) != Some(expected))
        || sha256(&wire[..body_end]) != wire[body_end..]
    {
        return None;
    }
    let [stock, forest, wildlife, deer, animals, _ai_clock, _clearance, _farm_crops, _oil_visuals, _owned_ids, _citizen_work] = match version {
        VERSION if end == body_end => [None; 11],
        #[cfg(feature = "ee2-rect289-citizen-work")]
        CITIZEN_WORK_VERSION => {
            let sections = split_sections(wire, end, body_end, 11)?;
            if !valid_clock_sections(&sections)
                || sections[6].is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG))
                || sections[7].is_some_and(|row| farm_crop_version(row).is_none())
                || sections[8].is_some_and(|row| !row.starts_with(super::oil_visual286::TAG))
                || sections[9].is_some_and(|row| !row.starts_with(super::owned_ids287::TAG))
                || !sections[10]?.starts_with(super::citizen_work289::TAG) { return None; }
            sections
        }
        #[cfg(feature = "ee2-rect287-oil-economy")]
        OWNED_IDS_VERSION => {
            let sections = split_sections(wire, end, body_end, 10)?;
            if !valid_clock_sections(&sections)
                || sections[6].is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG))
                || sections[7].is_some_and(|row| farm_crop_version(row).is_none())
                || sections[8].is_some_and(|row| !row.starts_with(super::oil_visual286::TAG))
                || !sections[9]?.starts_with(super::owned_ids287::TAG) { return None; }
            sections
        }
        #[cfg(feature = "ee2-rect286-oil-visual")]
        OIL_VISUAL_VERSION => {
            let sections = split_sections(wire, end, body_end, 9)?;
            if !valid_clock_sections(&sections)
                || sections[6].is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG))
                || sections[7].is_some_and(|row| farm_crop_version(row).is_none())
                || !sections[8]?.starts_with(super::oil_visual286::TAG) { return None; }
            sections
        }
        #[cfg(feature = "ee2-rect280-farm-crop")]
        FARM_CROP_VERSION | FARM_FAMILY_VERSION | FARM_VISUAL_VERSION | FARM_RECOVERY_VERSION => {
            let sections = split_sections(wire, end, body_end, 8)?;
            if !valid_clock_sections(&sections)
                || sections[6].is_some_and(|row| !row.starts_with(super::clearance_resume276::TAG))
                || farm_crop_version(sections[7]?) != Some(version) { return None; }
            sections
        }
        #[cfg(feature = "ee2-rect276-clearance-resume")]
        CLEARANCE_VERSION => {
            let sections = split_sections(wire, end, body_end, 7)?;
            if !valid_clock_sections(&sections)
                || !sections[6]?.starts_with(super::clearance_resume276::TAG) { return None; }
            sections
        }
        #[cfg(feature = "ee2-rect261-ai-clock")]
        AI_CLOCK_VERSION => {
            let sections = split_sections(wire, end, body_end, 6)?;
            if !valid_clock_sections(&sections) { return None; }
            sections
        }
        MUTABLE_VERSION | WILDLIFE_VERSION | DEER_VERSION | DEER_FOOT_VERSION | DEER_CONTACT_VERSION | ANIMAL_VERSION | ANIMAL_SLOPE_VERSION | ANIMAL_BIRTH_VERSION | NATIVE_CARCASS_VERSION | NATIVE_AIRCRAFT_VERSION | AIRCRAFT_VISUAL_VERSION => {
            if version >= WILDLIFE_VERSION && !cfg!(feature = "ee2-rect242-hunt")
                || version == DEER_VERSION && !cfg!(feature = "ee2-rect246-deer-live")
                || version == DEER_FOOT_VERSION && !cfg!(feature = "ee2-rect247-deer-foot")
                || version == DEER_CONTACT_VERSION && !cfg!(feature = "ee2-rect249-deer-contact")
                || version >= ANIMAL_VERSION && !cfg!(feature = "ee2-rect250-animals")
                || version == ANIMAL_SLOPE_VERSION && !cfg!(feature = "ee2-rect251-animal-slope")
                || version == ANIMAL_BIRTH_VERSION && !cfg!(feature = "ee2-rect252-animal-birth")
                || version == NATIVE_CARCASS_VERSION && !cfg!(feature = "ee2-rect253-native-carcass")
                || version == NATIVE_AIRCRAFT_VERSION && !cfg!(feature = "ee2-rect254-aircraft")
                || version == AIRCRAFT_VISUAL_VERSION && !cfg!(feature = "ee2-rect255-aircraft-visual") { return None; }
            let count = if version >= ANIMAL_VERSION { 5 } else { version.min(DEER_VERSION) as usize };
            let sections = split_sections(wire, end, body_end, count)?;
            if count == 2 && sections[..2] == [None, None]
                || matches!(count, 3 | 4) && sections[2].is_none() || count == 4 && sections[3].is_none()
                || count == 5 && (sections[4].is_none() || sections[3].is_some() && sections[2].is_none())
            { return None; }
            if sections[3].is_some_and(|deer| deer_version(deer).is_none_or(|v| v != version && count != 5)) { return None; }
            if sections[4].is_some_and(|animals| animal_version(animals) != Some(version)) { return None; }
            sections
        }
        _ => return None,
    };
    let binding = Binding {
        source: wire[32..64].try_into().ok()?,
        terrain: wire[64..96].try_into().ok()?,
        population: wire[96..128].try_into().ok()?,
    };
    let mut claims = [NONE; EXTRA];
    for (index, claim) in claims.iter_mut().enumerate() {
        *claim = word(128 + 4 * index)?;
    }
    Some((&wire[FIXED..end], Some(Locator { binding, claims, stock, forest, wildlife, deer, animals,
        #[cfg(feature = "ee2-rect261-ai-clock")]
        ai_clock: _ai_clock,
        #[cfg(feature = "ee2-rect276-clearance-resume")]
        clearance: _clearance,
        #[cfg(feature = "ee2-rect280-farm-crop")]
        farm_crops: _farm_crops,
        #[cfg(feature = "ee2-rect286-oil-visual")]
        oil_visuals: _oil_visuals,
        #[cfg(feature = "ee2-rect287-oil-economy")]
        owned_ids: _owned_ids,
        #[cfg(feature = "ee2-rect289-citizen-work")]
        citizen_work: _citizen_work,
    })))
}

#[cfg(feature = "ee2-rect261-ai-clock")]
fn valid_clock_sections(sections: &[Option<&[u8]>]) -> bool {
    !sections[3].is_some_and(|deer| sections[2].is_none() || deer_version(deer).is_none())
        && sections[4].and_then(animal_version) == Some(AIRCRAFT_VISUAL_VERSION)
        && sections[5].is_some_and(|clock| clock.starts_with(AI_CLOCK_TAG))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refresh_digest(wire: &mut [u8]) {
        let end = wire.len() - DIGEST;
        let digest = sha256(&wire[..end]);
        wire[end..].copy_from_slice(&digest);
    }

    #[cfg(feature = "ee2-rect286-oil-visual")]
    #[test]
    fn oil_visual_extends_farm_archives_only_when_present() {
        let binding = Binding { source: [1; 32], terrain: [2; 32], population: [3; 32] };
        let claims = [NONE; 64];
        let payload = b"VCA1 payload";
        let sections = [None, None, None, None, Some(b"DVA6 animal".as_slice())];
        let ai = b"DAI1 fixture";
        let clearance = b"DCE1 fixture";
        let oil = b"OIV1 record";
        for (tag, version) in [(b"FCP1", 15), (b"FCP2", 16), (b"FCP3", 17), (b"FCP4", 18)] {
            let prior = wrap_with_farm_crops(payload.to_vec(), &binding, &claims, sections,
                ai, Some(clearance), tag).unwrap();
            assert_eq!(prior[4], version);
            let (_, locator) = split(&prior).unwrap();
            let locator = locator.unwrap();
            assert_eq!(locator.farm_crops, Some(tag.as_slice()));
            assert!(locator.oil_visuals.is_none());
            assert_eq!(wrap_with_farm_crops(payload.to_vec(), &binding, &claims, sections,
                ai, locator.clearance, locator.farm_crops.unwrap()).unwrap(), prior);

            let extended = wrap_with_oil_visuals(payload.to_vec(), &binding, &claims, sections,
                ai, Some(clearance), Some(tag), oil).unwrap();
            let (inner, locator) = split(&extended).unwrap();
            let locator = locator.unwrap();
            assert_eq!(extended[4], 19);
            assert_eq!(inner, payload);
            assert_eq!(locator.farm_crops, Some(tag.as_slice()));
            assert_eq!(locator.oil_visuals, Some(oil.as_slice()));
            let mut invalid = extended;
            let start = invalid.len() - DIGEST - oil.len();
            invalid[start] = b'X';
            refresh_digest(&mut invalid);
            assert!(split(&invalid).is_none());
        }
    }

    #[test]
    fn binds_distinct_layout_population_and_high_claims_to_inner_save() {
        let binding = Binding { source: [1; 32], terrain: [2; 32], population: [3; 32] };
        let payload = b"OEU7 contents";
        let mut claims = [NONE; 64];
        claims[32] = 0x1_0000;
        claims[42] = 0x2_0010;
        let wire = wrap(payload.to_vec(), &binding, &claims, [None; 5]).unwrap();
        let (inner, locator) = split(&wire).unwrap();
        let locator = locator.unwrap();
        assert_eq!(inner, payload);
        assert_eq!(locator.binding, binding);
        assert_eq!(locator.claims, claims[32..43]);
        for at in [4, 8, 16, 24, 28, 32, 64, 96, 128, FIXED, wire.len() - 1] {
            let mut invalid = wire.clone();
            invalid[at] ^= 1;
            assert!(split(&invalid).is_none());
        }
        let mut invalid = wire.clone();
        invalid[16] ^= 1;
        refresh_digest(&mut invalid);
        assert!(split(&invalid).is_none());
        claims[43] = 0;
        assert!(wrap(payload.to_vec(), &binding, &claims, [None; 5]).is_none());
        assert_eq!(split(payload).unwrap().0, payload);
    }

    #[cfg(feature = "ee2-rect255-aircraft-visual")]
    #[test]
    fn existing_v12_keeps_its_five_lengths_and_dva6() {
        let binding = Binding { source: [1; 32], terrain: [2; 32], population: [3; 32] };
        let claims = [NONE; 64];
        let payload = b"VCA1 payload";
        let sections = [None, None, None, None, Some(b"DVA6 animal".as_slice())];
        let wire = wrap(payload.to_vec(), &binding, &claims, sections).unwrap();
        let tail = FIXED + payload.len();
        assert_eq!(&wire[..8], b"DVS1\x0c\0\0\0");
        assert_eq!(&wire[tail..wire.len() - DIGEST],
            b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x0b\0\0\0DVA6 animal");
        let (inner, locator) = split(&wire).unwrap();
        let locator = locator.unwrap();
        assert_eq!(inner, payload);
        assert_eq!(locator.animals, sections[4]);
        #[cfg(feature = "ee2-rect261-ai-clock")]
        {
            assert!(locator.ai_clock.is_none());
            assert_eq!(wrap_with_ai(payload.to_vec(), &binding, &claims, sections, None).unwrap(), wire);
        }
        #[cfg(not(feature = "ee2-rect261-ai-clock"))]
        {
            let mut unsupported = wire;
            unsupported[4] = AI_CLOCK_VERSION as u8;
            unsupported.splice(tail + 20..tail + 20, 4u32.to_le_bytes());
            let end = unsupported.len() - DIGEST;
            unsupported.splice(end..end, *AI_CLOCK_TAG);
            refresh_digest(&mut unsupported);
            assert!(split(&unsupported).is_none());
        }
    }

    #[cfg(feature = "ee2-rect261-ai-clock")]
    #[test]
    fn v13_requires_independent_dva6_and_dai1_sections() {
        let binding = Binding { source: [1; 32], terrain: [2; 32], population: [3; 32] };
        let claims = [NONE; 64];
        let payload = b"VCA1 payload";
        let animals = b"DVA6 animal";
        let ai = b"DAI1 fixture";
        let sections = [Some(b"DVR1 stock".as_slice()), Some(b"DVF1 forest".as_slice()),
            Some(b"DVH1 hunt".as_slice()), Some(b"DVD3 deer".as_slice()), Some(animals.as_slice())];
        let wire = wrap_with_ai(payload.to_vec(), &binding, &claims, sections, Some(ai)).unwrap();
        let tail = FIXED + payload.len();
        assert_eq!(&wire[..8], b"DVS1\x0d\0\0\0");
        let lengths = sections.into_iter().flatten().chain([ai.as_slice()]).map(|row| row.len() as u32);
        for (entry, expected) in wire[tail..tail + 24].chunks_exact(4).zip(lengths) {
            assert_eq!(entry, expected.to_le_bytes());
        }
        let (inner, locator) = split(&wire).unwrap();
        let locator = locator.unwrap();
        assert_eq!(inner, payload);
        assert_eq!([locator.stock, locator.forest, locator.wildlife, locator.deer, locator.animals], sections);
        assert_eq!(locator.ai_clock, Some(ai.as_slice()));

        let animal_at = tail + 24 + sections[..4].iter().flatten().map(|row| row.len()).sum::<usize>();
        let ai_at = animal_at + animals.len();
        for at in [tail + 20, animal_at + 3, ai_at, ai_at + 3] {
            let mut invalid = wire.clone();
            invalid[at] ^= 1;
            refresh_digest(&mut invalid);
            assert!(split(&invalid).is_none());
        }
        let mut no_ai = wire.clone();
        no_ai[tail + 20..tail + 24].fill(0);
        no_ai.drain(ai_at..ai_at + ai.len());
        refresh_digest(&mut no_ai);
        assert!(split(&no_ai).is_none());

        let mut no_animals = sections;
        no_animals[4] = None;
        assert!(wrap_with_ai(payload.to_vec(), &binding, &claims, no_animals, Some(ai)).is_none());
        no_animals[4] = Some(b"DVA5 animal");
        assert!(wrap_with_ai(payload.to_vec(), &binding, &claims, no_animals, Some(ai)).is_none());
        assert!(wrap_with_ai(payload.to_vec(), &binding, &claims, sections, Some(b"DAI2 fixture")).is_none());
    }
}
