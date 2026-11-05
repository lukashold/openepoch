//! Paid branch of 006E40F0, 00537B40 duration and 008054B0 area-time factors.

use crate::ai9_count::Precision53;
use crate::ai9_research_start::PaidResearch;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct ActiveResearch {
    pub index: i32,
    pub host_handle: i32,
    pub elapsed: f32,
    pub duration: f32,
    pub cost: [f32; 11],
}

pub enum PlayerTimeScale {
    /// 00537B40 with a null player uses the source's 1.0 literal.
    Neutral,
    /// Nonproduction factor at 005E2500(Player)+24.
    Technology(f32),
    /// 005E0100/005E0110 load the Citizen/other word of Player+920,
    /// then multiply by Player+928. The resolver supplies the selected word.
    Production { selected: f32, global: f32 },
}

pub struct ResearchTiming<'a> {
    /// The live node+18 time word.
    pub node_seconds: f32,
    pub player_scale: PlayerTimeScale,
    /// Ordered effective type-77 area-effect values returned by 00805A60.
    pub area_scales: &'a [f32],
    /// Live C62D54. The PE initializer is F32 0.05.
    pub minimum_area_scale: f32,
    pub precision: Precision53,
}

impl ResearchTiming<'_> {
    pub fn duration(self) -> f32 {
        let mut area = 1.0_f32;
        for &scale in self.area_scales {
            let delta = self.precision.add(scale as f64, -1.0);
            area = self
                .precision
                .store_f32(self.precision.add(delta, area as f64));
        }
        if self.minimum_area_scale > area {
            area = self.minimum_area_scale;
        }

        let player_scale = match self.player_scale {
            PlayerTimeScale::Neutral => 1.0,
            PlayerTimeScale::Technology(scale) => scale as f64,
            PlayerTimeScale::Production { selected, global } => {
                self.precision.mul(selected as f64, global as f64)
            }
        };
        let node_time = self.precision.mul(player_scale, self.node_seconds as f64);
        self.precision
            .store_f32(self.precision.mul(node_time, area as f64))
    }

    pub fn active(self, paid: PaidResearch) -> ActiveResearch {
        ActiveResearch {
            index: paid.index,
            host_handle: -1,
            elapsed: 0.0,
            duration: self.duration(),
            cost: paid.cost,
        }
    }
}

/// Supplied only for the local player selected by the UI virtual at +10.
#[derive(Clone, Copy)]
pub struct LocalStartUi {
    /// Original node definition+24, independent of the computed duration.
    pub definition_seconds: f32,
    pub suppressed: bool,
    /// Original node definition+0, where -1 suppresses research/upgrade text.
    pub text_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartNotice {
    pub index: i32,
    pub message_key: Option<&'static str>,
    pub sound_key: &'static str,
}

impl LocalStartUi {
    pub fn notice(self, index: i32, category: i32) -> Option<StartNotice> {
        if self.definition_seconds.partial_cmp(&1.0) != Some(core::cmp::Ordering::Greater)
            || self.suppressed
        {
            return None;
        }
        let message_key = match (category, self.text_id) {
            (1, _) => Some("msg_ig_techtree_advBegin"),
            (_, -1) => None,
            (5, _) => Some("msg_ig_techtree_impbegun"),
            _ => Some("msg_ig_techtree_resbegun"),
        };
        Some(StartNotice {
            index,
            message_key,
            sound_key: "ui_research_start",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_uses_ordered_area_stores_floor_and_unrounded_node_time() {
        for precision in [Precision53::NearestEven, Precision53::TowardZero] {
            let time = |scales: &[f32], player_scale| {
                ResearchTiming {
                    node_seconds: 32.0,
                    player_scale,
                    area_scales: scales,
                    minimum_area_scale: 0.0625,
                    precision,
                }
                .duration()
            };
            assert_eq!(time(&[], PlayerTimeScale::Neutral), 32.0);
            assert_eq!(time(&[0.75, 1.5], PlayerTimeScale::Technology(0.5)), 20.0);
            assert_eq!(time(&[0.25, 0.5], PlayerTimeScale::Neutral), 2.0);
            let near_one = f32::from_bits(1.0_f32.to_bits() + 1);
            assert_eq!(
                time(
                    &[4.0, near_one, near_one, near_one, near_one],
                    PlayerTimeScale::Neutral
                ),
                128.0
            );
            assert_eq!(
                time(
                    &[1.25],
                    PlayerTimeScale::Production {
                        selected: 0.625,
                        global: 0.75
                    }
                ),
                18.75
            );
        }
        let scale = f32::from_bits(1.0_f32.to_bits() + 1);
        let computed = ResearchTiming {
            node_seconds: scale,
            player_scale: PlayerTimeScale::Technology(scale),
            area_scales: &[f32::from_bits(1.0_f32.to_bits() - 2)],
            minimum_area_scale: 0.05,
            precision: Precision53::TowardZero,
        }
        .duration();
        assert_eq!(computed.to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn notice_uses_definition_time_and_retains_sound_without_text() {
        let notice = |seconds, suppressed, category, text_id| {
            LocalStartUi {
                definition_seconds: seconds,
                suppressed,
                text_id,
            }
            .notice(77, category)
        };
        for seconds in [-1.0, 1.0, f32::NAN] {
            assert_eq!(notice(seconds, false, 1, 1), None);
        }
        assert_eq!(notice(2.0, true, 1, 1), None);
        assert_eq!(
            notice(2.0, false, 1, -1).unwrap().message_key,
            Some("msg_ig_techtree_advBegin")
        );
        assert_eq!(
            notice(2.0, false, 5, 42).unwrap().message_key,
            Some("msg_ig_techtree_impbegun")
        );
        assert_eq!(
            notice(2.0, false, 2, 42).unwrap().message_key,
            Some("msg_ig_techtree_resbegun")
        );
        assert_eq!(
            notice(2.0, false, 5, -1),
            Some(StartNotice {
                index: 77,
                message_key: None,
                sound_key: "ui_research_start"
            })
        );
    }
}
