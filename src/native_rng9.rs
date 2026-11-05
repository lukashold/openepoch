//! Own implementation of the source's shared u32 random-state operation.
//!
//! 00596D80 stores the two coefficients. 007F4DDA..007F4E0D advances that
//! same shared state for exact normal-production name "Citizen", then appends
//! M/F by bit16. This module never creates a seed or a private per-actor stream.
//! Initialization/consumption order must be supplied by its game integration.

pub const MULTIPLIER: u32 = 1_664_525;
pub const INCREMENT: u32 = 1_013_904_223;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeRng {
    state: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CitizenIdentity {
    Male = 0,
    Female = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedProduction<'a> {
    pub source_name: &'a str,
    pub citizen_identity: Option<CitizenIdentity>,
    pub consumed_draw: bool,
}

impl NativeRng {
    pub const fn from_state(state: u32) -> Self { Self { state } }
    pub const fn state(&self) -> u32 { self.state }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        self.state
    }

    /// Both the personality picker and the production branch can consume this
    /// one object. The picker uses its high fifteen bits; the Citizen branch
    /// uses their least significant bit. No renderer may call this method.
    pub fn next_word15(&mut self) -> u32 { (self.next_u32() >> 16) & 0x7fff }

    /// Only the source's ordinary unit-production branch calls this resolver.
    /// Literal CitizenM/F and all other names keep their exact identity and do
    /// not consume a draw. Source name matching is case sensitive.
    pub fn resolve_normal_unit<'a>(&mut self, requested: &'a str) -> ResolvedProduction<'a> {
        if requested != "Citizen" {
            return ResolvedProduction { source_name: requested, citizen_identity: None, consumed_draw: false };
        }
        let female = self.next_word15() & 1 != 0;
        ResolvedProduction {
            source_name: if female { "CitizenF" } else { "CitizenM" },
            citizen_identity: Some(if female { CitizenIdentity::Female } else { CitizenIdentity::Male }),
            consumed_draw: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../tools/fixtures/native-rng9-oracle.rs");

    #[test]
    fn source_coefficients_and_independent_affine_oracles() {
        assert_eq!(MULTIPLIER, 0x0019_660d);
        assert_eq!(INCREMENT, 0x3c6e_f35f);
        for &(seed, steps, expected) in VECTORS {
            let mut rng = NativeRng::from_state(seed);
            for _ in 0..steps { rng.next_u32(); }
            assert_eq!(rng.state(), expected, "seed={seed}, steps={steps}");
        }
    }

    #[test]
    fn literal_names_do_not_consume_or_randomize() {
        for name in ["CitizenM", "CitizenF", "citizen", "CITIZEN", "Citizen2", "Citizen ", "", "Scout", "Bürger"] {
            let mut rng = NativeRng::from_state(u32::MAX);
            let result = rng.resolve_normal_unit(name);
            assert_eq!(result.source_name, name);
            assert_eq!(result.citizen_identity, None);
            assert!(!result.consumed_draw);
            assert_eq!(rng.state(), u32::MAX);
        }
    }

    #[test]
    fn production_uses_updated_bit16_and_one_shared_draw() {
        for &(seed, steps, expected) in VECTORS {
            if steps == 0 { continue; }
            let mut rng = NativeRng::from_state(seed);
            for _ in 1..steps { rng.next_u32(); }
            let result = rng.resolve_normal_unit("Citizen");
            assert_eq!(rng.state(), expected);
            assert!(result.consumed_draw);
            let id = if (expected >> 16) & 1 != 0 { CitizenIdentity::Female } else { CitizenIdentity::Male };
            assert_eq!(result.citizen_identity, Some(id));
            assert_eq!(result.source_name, if id == CitizenIdentity::Female { "CitizenF" } else { "CitizenM" });
        }
    }

    #[test]
    fn personality_and_birth_share_the_same_supplied_state() {
        let mut shared = NativeRng::from_state(0x1234_5678);
        let mut raw = NativeRng::from_state(0x1234_5678);
        for n in 0..4096 {
            let word = raw.next_u32();
            if n % 3 == 0 {
                assert_eq!(shared.next_word15(), (word >> 16) & 0x7fff);
            } else {
                let citizen = shared.resolve_normal_unit("Citizen");
                assert_eq!(citizen.citizen_identity, Some(if word & 0x1_0000 == 0 { CitizenIdentity::Male } else { CitizenIdentity::Female }));
            }
            assert_eq!(shared.state(), raw.state());
        }
    }
}
