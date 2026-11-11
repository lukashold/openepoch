//! Valley route and idle atlas resume over the exact authenticated opening atlas.
use super::atlas_runtime::{
    route_history::{Journal, EVENT_BYTES},
    route_motion::{RouteProgress, MAX_POINTS, MAX_ROUTES},
};
use crate::game::{Game, MAXE, NONE};
use std::collections::BTreeMap;
#[cfg(feature = "ee2-valley204-ground-motion")]
use super::atlas_runtime::ground_motion::NativeMotion;
#[cfg(feature = "ee2-valley215-citizen-contact")]
use super::atlas_runtime::contact_kernel::lifecycle::ForceState;

const ACTIVE_TAG: &[u8; 4] = b"VRM1";
const IDLE_TAG: &[u8; 4] = b"VRA1";
#[cfg(feature = "ee2-valley204-ground-motion")]
const MOTION_TAG: &[u8; 4] = b"VRM2";
#[cfg(feature = "ee2-valley215-citizen-contact")]
const CONTACT_TAG: &[u8; 4] = b"VRM3";
const MAP_SIZE: [f32; 2] = [115.0, 71.0];

pub(crate) struct Resume {
    history: Journal,
    active: BTreeMap<u32, RouteProgress>,
}

fn read_words<const N: usize>(bytes: &mut &[u8]) -> Option<[u32; N]> {
    let (row, tail) = bytes.split_at_checked(N * 4)?;
    *bytes = tail;
    Some(std::array::from_fn(|i| u32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap())))
}

fn write_words(out: &mut Vec<u8>, words: impl IntoIterator<Item = u32>) {
    for word in words { out.extend_from_slice(&word.to_le_bytes()); }
}

fn valid_position(point: [f32; 2]) -> bool {
    point.into_iter().zip(MAP_SIZE).all(|(value, limit)| (0.0..limit).contains(&value))
}

fn valid_route(route: &RouteProgress, generation: u32) -> bool {
    let count = route.waypoints.len();
    route.actor != NONE && (route.actor as u16 as usize) < MAXE
        && route.atlas_generation == generation
        && (3..=MAX_POINTS).contains(&count) && (1..count).contains(&usize::from(route.next))
        && valid_position(route.final_goal) && route.waypoints.iter().copied().all(valid_position)
        && route.waypoints[count - 1].map(f32::to_bits) == route.final_goal.map(f32::to_bits)
}

fn valid_actor(game: &Game, route: &RouteProgress) -> bool {
    route.issued_tick <= game.tick && route.valid_for_actor(game) && valid_motion(game, route)
}

fn valid_motion(_game: &Game, _route: &RouteProgress) -> bool {
    #[cfg(feature = "ee2-valley204-ground-motion")]
    if let Some(native) = _route.native {
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        if native.force.is_some_and(|force| !force.valid(_game.sim_elapsed())) {
            return false;
        }
        return _game.get(_route.actor).is_some_and(|index| native.matches(&_game.ents[index]));
    }
    true
}

impl Resume {
    pub(super) fn decode(bytes: &[u8]) -> Option<Self> {
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let contact = bytes.strip_prefix(CONTACT_TAG);
        #[cfg(feature = "ee2-valley204-ground-motion")]
        let motion = bytes.strip_prefix(MOTION_TAG);
        #[cfg(not(feature = "ee2-valley204-ground-motion"))]
        let motion: Option<&[u8]> = None;
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let motion = contact.or(motion);
        let (mut bytes, idle) = if let Some(body) = bytes.strip_prefix(IDLE_TAG) {
            (body, true)
        } else {
            (motion.or_else(|| bytes.strip_prefix(ACTIVE_TAG))?, false)
        };
        let [events, routes] = read_words(&mut bytes)?;
        if (routes == 0) != idle || routes as usize > MAX_ROUTES { return None; }
        let (history, rest) = bytes.split_at_checked((events as usize).checked_mul(EVENT_BYTES)?)?;
        let history = Journal::decode(history, events as usize)?;
        bytes = rest;
        let mut active = BTreeMap::new();
        #[cfg(feature = "ee2-valley204-ground-motion")]
        let mut has_native = false;
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let mut has_force = false;
        for _ in 0..routes {
            let [actor, issued_tick, atlas_generation, next, count, x, y] = read_words(&mut bytes)?;
            if !(3..=MAX_POINTS).contains(&(count as usize)) || bytes.len() < count as usize * 8
                || active.last_key_value().is_some_and(|(&last, _)| actor <= last) { return None; }
            let mut waypoints = Vec::with_capacity(count as usize);
            for _ in 0..count { waypoints.push(read_words::<2>(&mut bytes)?.map(f32::from_bits)); }
            #[cfg(feature = "ee2-valley204-ground-motion")]
            let native = if motion.is_some() {
                match read_words::<3>(&mut bytes)? {
                    [0, 0, 0] => None,
                    [1, forward, turn] => {
                        let native = NativeMotion { forward: f32::from_bits(forward), turn: f32::from_bits(turn),
                            #[cfg(feature = "ee2-valley215-citizen-contact")] force: None };
                        if !native.valid() { return None; }
                        has_native = true;
                        Some(native)
                    }
                    #[cfg(feature = "ee2-valley215-citizen-contact")]
                    [2, forward, turn] if contact.is_some() => {
                        let force = ForceState::from_words(read_words::<7>(&mut bytes)?)?;
                        let native = NativeMotion { forward: f32::from_bits(forward), turn: f32::from_bits(turn),
                            force: Some(force) };
                        if !native.valid() { return None; }
                        has_native = true;
                        has_force = true;
                        Some(native)
                    }
                    _ => return None,
                }
            } else {
                None
            };
            let route = RouteProgress {
                actor, issued_tick, atlas_generation, next: u16::try_from(next).ok()?,
                final_goal: [x, y].map(f32::from_bits), waypoints,
                #[cfg(feature = "ee2-valley204-ground-motion")]
                native,
            };
            if !valid_route(&route, events) { return None; }
            active.insert(actor, route);
        }
        #[cfg(feature = "ee2-valley204-ground-motion")]
        if motion.is_some() != has_native { return None; }
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        if contact.is_some() != has_force { return None; }
        bytes.is_empty().then_some(Self { history, active })
    }
}

pub(super) fn encode(game: &Game, revalidate_path: bool) -> Option<Vec<u8>> {
    let active = &game.valley_routes.active;
    if !game.valley_routes.enabled || active.len() > MAX_ROUTES { return None; }
    let atlas = game.valley_fixture.as_ref()?.atlas.as_ref()?;
    let (history, generation) = atlas.route_history_for_save(game)?;
    if active.iter().any(|(&handle, route)| handle != route.actor || !valid_route(route, generation)
        || !valid_actor(game, route) || revalidate_path && !game.valley_matches_saved_waypoints(route)) { return None; }
    let tag = if active.is_empty() { IDLE_TAG } else { ACTIVE_TAG };
    #[cfg(feature = "ee2-valley204-ground-motion")]
    let motion = active.values().any(|route| route.native.is_some());
    #[cfg(feature = "ee2-valley204-ground-motion")]
    let tag = if motion { MOTION_TAG } else { tag };
    #[cfg(feature = "ee2-valley215-citizen-contact")]
    let tag = if active.values().any(|route| route.native.is_some_and(|native| native.force.is_some())) {
        CONTACT_TAG
    } else { tag };
    let mut output = tag.to_vec();
    write_words(&mut output, [history.len() as u32, active.len() as u32]);
    history.encode_into(&mut output);
    for route in active.values() {
        write_words(&mut output, [route.actor, route.issued_tick, route.atlas_generation, u32::from(route.next),
            route.waypoints.len() as u32, route.final_goal[0].to_bits(), route.final_goal[1].to_bits()]);
        write_words(&mut output, route.waypoints.iter().flatten().map(|value| value.to_bits()));
        #[cfg(feature = "ee2-valley204-ground-motion")]
        if motion {
            if let Some(native) = route.native {
                #[cfg(feature = "ee2-valley215-citizen-contact")]
                if let Some(force) = native.force {
                    write_words(&mut output, [2, native.forward.to_bits(), native.turn.to_bits()]);
                    write_words(&mut output, force.words());
                    continue;
                }
                write_words(&mut output, [1, native.forward.to_bits(), native.turn.to_bits()]);
            } else {
                write_words(&mut output, [0; 3]);
            }
        }
    }
    Some(output)
}

pub(crate) fn restore(game: &mut Game, resume: Resume) -> Option<()> {
    let mut atlas = game.valley_fixture.as_mut()?.atlas.take()?;
    atlas.replay_route_history(game, resume.history)?;
    game.valley_fixture.as_mut()?.atlas = Some(atlas);
    if resume.active.values().any(|route| !valid_actor(game, route) || !game.valley_matches_saved_waypoints(route)) {
        return None;
    }
    game.valley_routes.enabled = true;
    game.valley_routes.active = resume.active;
    Some(())
}

#[no_mangle]
pub extern "C" fn valley198_waypoint_archive_version() -> u32 {
    super::save_wire::WAYPOINT_ROUTE_VERSION
}
