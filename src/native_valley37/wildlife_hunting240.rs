//! Source-backed Death Valley2 animal deaths and food-hunt selection.
//! Native handles, grid enumeration and the real queue validator are caller inputs.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FoodStock {
    pub amount: u16,
    pub max_workers: u8,
    pub exhaustible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorpseType {
    pub name: &'static str,
    pub animation: &'static str,
    pub food: Option<FoodStock>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimalProfile {
    pub hp: u16,
    pub corpse: Option<CorpseType>,
}

include!("animal_profiles240.rs");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub xyz: [f32; 3],
    pub yaw: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorpseSpawn {
    pub owner: u32,
    pub corpse_type: CorpseType,
    pub pose: Pose,
}

/// Call only for a matching live AnimalManager registration. The host allocates a fresh actor.
pub fn registered_death(profile: AnimalProfile, pose: Pose) -> Option<CorpseSpawn> {
    Some(CorpseSpawn {
        owner: 0,
        corpse_type: profile.corpse?,
        pose,
    })
}

pub const ATTACK_TAG: u32 = 1;
pub const GATHER_TAG: u32 = 9;
pub const HUNT_GRID_RADIUS: f32 = 5.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Task {
    pub tag: u32,
    pub target: u32,
}

/// The target's native attacker list resolves to these current actors in source order.
/// An absent row denotes a stale or otherwise unresolved source handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attacker {
    pub handle: u32,
    pub current: Option<Task>,
    pub has_gather_ability: bool,
}

/// Submit the requests through ordinary gather admission and append only on success.
/// The source wrapper can request tag 9 even when its downstream validation rejects it.
pub fn corpse_gather_requests(
    dead_animal: u32,
    new_corpse: u32,
    attackers: &[Option<Attacker>],
) -> impl Iterator<Item = (u32, Task)> + '_ {
    attackers.iter().flatten().filter_map(move |actor| {
        (actor.has_gather_ability
            && actor.current
                == Some(Task {
                    tag: ATTACK_TAG,
                    target: dead_animal,
                }))
        .then_some((
            actor.handle,
            Task {
                tag: GATHER_TAG,
                target: new_corpse,
            },
        ))
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HuntCandidate {
    pub handle: u32,
    pub xyz: [f32; 3],
    pub profile: AnimalProfile,
}

/// 007EF4A0 after the native selector-1, radius-5 grid query. Supply its
/// ordered, already admitted moving actors. Uses the nearest53 context.
pub fn nearest_food_prey(origin: [f32; 3], candidates: &[HuntCandidate]) -> Option<u32> {
    let mut best = f32::MAX;
    let mut choice = None;
    for candidate in candidates {
        if candidate
            .profile
            .corpse
            .and_then(|corpse| corpse.food)
            .is_none()
        {
            continue;
        }
        let [dx, dy, dz] =
            std::array::from_fn::<_, 3, _>(|i| f64::from(candidate.xyz[i]) - f64::from(origin[i]));
        let distance = (dz * dz + dy * dy) + dx * dx;
        if distance < f64::from(best) {
            best = distance as f32;
            choice = Some(candidate.handle);
        }
    }
    choice
}
