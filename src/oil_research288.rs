//! GatherOil's permanent research percentage applies to the absolute rig rate.
use crate::{defs::{DefId, OIL_RIG}, game::{source_math as native, Game}};

const OIL_NODES: [usize; 3] = [37, 97, 157];

pub(crate) fn rate_effect(def: DefId, field: u8, method: u8) -> bool {
    def == OIL_RIG && field == 16 && method == 0
}

fn upgraded_rate(permanent: i16) -> f32 {
    if permanent == 0 { return 0.75; }
    // Original GatherRate descriptor: signed permanent percent, binary32 .01,
    // retained PC53/RC_CHOP products, then one binary32 store to the ability.
    let scale = native::mul(f64::from(100 + i32::from(permanent)), f64::from(0.01_f32));
    native::f(native::mul(scale, 0.75)) as f32
}

impl Game {
    pub(crate) fn oil_research_rate(&self, team: u8) -> f32 {
        let permanent = OIL_NODES.into_iter().filter(|&node| self.has_research(team, node)).count() as i16 * 10;
        upgraded_rate(permanent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oil_rate_and_generated_research_targets_match_installed_declarations() {
        let mut found = Vec::new();
        for (index, node) in crate::ee2_research::NODES.iter().enumerate() {
            for effect in node.effects {
                if crate::ids::mask_has(effect.mask, OIL_RIG) && rate_effect(OIL_RIG, effect.field, effect.method) {
                    assert_eq!(effect.value, 10.0);
                    found.push(index);
                }
            }
        }
        assert_eq!(found, OIL_NODES);
        assert_eq!((0..=3).map(|n| upgraded_rate(n * 10).to_bits()).collect::<Vec<_>>(),
            [0x3f400000, 0x3f533332, 0x3f666666, 0x3f799999]);
    }

    #[test]
    fn oil_research_is_separate_from_area_multiplier_and_survives_rebuild() {
        let mut game = Game::new(2, 0, 0, 1, 0);
        for (learned, node) in OIL_NODES.into_iter().enumerate() {
            game.researched[0][node / 32] |= 1 << (node % 32);
            game.rebuild_modifiers(0, false);
            assert_eq!(game.oil_research_rate(0).to_bits(), upgraded_rate((learned as i16 + 1) * 10).to_bits());
            assert_eq!(game.modifiers[0][OIL_RIG as usize][16], 1.0);
            assert_eq!(game.oil_research_rate(1), 0.75);
        }
    }
}
