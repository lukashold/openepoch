//! Separate simulation, production-job, research and presentation namespaces.
//! Legacy aliases and encodings are unchanged outside the isolated UP6 feature.
#[cfg(feature = "ee2-roster6")]
pub type DefId = u16;
#[cfg(not(feature = "ee2-roster6"))]
pub type DefId = u8;
#[cfg(feature = "ee2-roster6")]
pub type QueueItem = u32;
#[cfg(not(feature = "ee2-roster6"))]
pub type QueueItem = u8;
pub type ResearchId = u16;
pub type RenderId = u32;
#[cfg(feature = "ee2-roster6")]
pub type TargetMask = &'static [u64];
#[cfg(not(feature = "ee2-roster6"))]
pub type TargetMask = u64;
pub const DEF_NONE: DefId = DefId::MAX;
pub const RESEARCH_BASE: QueueItem = if cfg!(feature = "ee2-roster6") {
    0x1_0000u32 as QueueItem
} else {
    64
};
pub const TECH_BASE: QueueItem = if cfg!(feature = "ee2-roster6") {
    0x2_0000u32 as QueueItem
} else {
    200
};
pub const EPOCH_JOB: QueueItem = if cfg!(feature = "ee2-roster6") {
    0x3_0000u32 as QueueItem
} else {
    255
};
pub const fn unit_job(id: DefId) -> QueueItem {
    id as QueueItem
}
pub const fn research_job(id: ResearchId) -> QueueItem {
    RESEARCH_BASE + id as QueueItem
}
pub const fn tech_job(id: u8) -> QueueItem {
    TECH_BASE + id as QueueItem
}
pub const fn mask(words: &'static [u64]) -> TargetMask {
    #[cfg(feature = "ee2-roster6")]
    {
        words
    }
    #[cfg(not(feature = "ee2-roster6"))]
    {
        if words.is_empty() {
            0
        } else {
            words[0]
        }
    }
}
pub fn mask_has(mask: TargetMask, id: DefId) -> bool {
    #[cfg(feature = "ee2-roster6")]
    {
        mask.get(id as usize / 64)
            .is_some_and(|w| w >> (id as usize % 64) & 1 != 0)
    }
    #[cfg(not(feature = "ee2-roster6"))]
    {
        id < 64 && mask >> id & 1 != 0
    }
}
/// Explicit wire job kind0=unit,1=research,2=independent technology,3=epoch.
pub fn encode_job(kind: u32, id: u32) -> Option<QueueItem> {
    if id > u16::MAX as u32 {
        return None;
    }
    #[cfg(feature = "ee2-roster6")]
    {
        match kind {
            0 => Some(id),
            1 if id < 180 => Some(RESEARCH_BASE + id),
            2 if id < crate::defs::TECHS.len() as u32 => Some(TECH_BASE + id),
            3 if id == 0 => Some(EPOCH_JOB),
            _ => None,
        }
    }
    #[cfg(not(feature = "ee2-roster6"))]
    {
        match kind {
            0 if id < 64 => Some(id as u8),
            1 if id < 180 => Some(RESEARCH_BASE + id as u8),
            2 if id < crate::defs::TECHS.len() as u32 => Some(TECH_BASE + id as u8),
            3 if id == 0 => Some(EPOCH_JOB),
            _ => None,
        }
    }
}
pub fn job_kind(job: QueueItem) -> u32 {
    if job == EPOCH_JOB {
        3
    } else if (RESEARCH_BASE..RESEARCH_BASE + 180).contains(&job) {
        1
    } else if job >= TECH_BASE {
        2
    } else {
        0
    }
}
pub const UNIT_RENDER_BASE: RenderId = 256;
pub fn unit_render(id: DefId) -> f32 {
    if cfg!(feature = "ee2-roster6") {
        UNIT_RENDER_BASE as f32 + id as f32
    } else {
        id as f32
    }
}
pub fn scene_render(legacy: u32) -> f32 {
    if !cfg!(feature = "ee2-roster6") {
        return legacy as f32;
    }
    match legacy {
        100..=109 => (legacy - 100) as f32,
        110 => 32.0,
        111 => 33.0,
        240..=243 => (legacy - 200) as f32,
        _ => legacy as f32,
    }
}
pub fn render_capacity() -> usize {
    if cfg!(feature = "ee2-roster6") {
        UNIT_RENDER_BASE as usize + crate::defs::DEFS.len()
    } else {
        256
    }
}
pub fn render_kind(id: RenderId) -> u32 {
    if id >= UNIT_RENDER_BASE && id < UNIT_RENDER_BASE + crate::defs::DEFS.len() as u32 {
        0
    } else if id <= 10 {
        1
    } else if (16..=22).contains(&id) {
        2
    } else if (40..=43).contains(&id) {
        3
    } else if matches!(id, 32 | 33) {
        4
    } else {
        u32::MAX
    }
}
pub fn render_source(id: RenderId) -> u32 {
    match render_kind(id) {
        0 => id - UNIT_RENDER_BASE,
        1 => id + 100,
        2 => id - 16,
        3 => id + 200,
        4 => id + 78,
        _ => u32::MAX,
    }
}

pub fn resource_render(legacy: u32) -> f32 {
    if cfg!(feature = "ee2-roster6") {
        (legacy - 100) as f32
    } else {
        legacy as f32
    }
}

pub fn decode_job(job: QueueItem) -> Option<(u32, u32)> {
    if job == EPOCH_JOB {
        return Some((3, 0));
    }
    if (RESEARCH_BASE..RESEARCH_BASE + 180).contains(&job) {
        return Some((1, (job - RESEARCH_BASE) as u32));
    }
    if job >= TECH_BASE && ((job - TECH_BASE) as usize) < crate::defs::TECHS.len() {
        return Some((2, (job - TECH_BASE) as u32));
    }
    #[cfg(feature = "ee2-roster6")]
    if job <= u16::MAX as u32 {
        return Some((0, job));
    }
    #[cfg(not(feature = "ee2-roster6"))]
    if job < 64 {
        return Some((0, job as u32));
    }
    None
}
#[cfg(all(test, feature = "ee2-roster6"))]
mod wide_tests {
    use super::*;
    #[test]
    fn namespaces_cross_old_boundaries_without_aliasing() {
        for id in [
            0, 56, 63, 64, 99, 100, 179, 243, 244, 255, 256, 310, 3035, 65535,
        ] {
            let q = encode_job(0, id).unwrap();
            assert_eq!(decode_job(q), Some((0, id)));
        }
        for id in 0..180 {
            let q = encode_job(1, id).unwrap();
            assert_eq!(decode_job(q), Some((1, id)));
            assert_ne!(q, id);
        }
        assert_eq!(encode_job(0, 65536), None);
        assert_eq!(encode_job(1, 180), None);
        assert_eq!(decode_job(0x100b4), None);
        assert_eq!(decode_job(0x40000), None);
        assert_eq!(decode_job(EPOCH_JOB), Some((3, 0)));
    }
    #[test]
    fn target_masks_reach_beyond_one_word() {
        const M: TargetMask = mask(&[0, 1, 0, 1u64 << 63, 1, 0, 0, 0]);
        assert!(mask_has(M, 64));
        assert!(mask_has(M, 255));
        assert!(mask_has(M, 256));
        assert!(!mask_has(M, 63));
        assert!(!mask_has(M, 257));
        assert!(!mask_has(M, 65535));
    }
    #[test]
    fn render_namespaces_are_disjoint() {
        for id in 0..crate::defs::DEFS.len() as DefId {
            let r = unit_render(id) as u32;
            assert!(r >= 256);
            assert_eq!(render_kind(r), 0);
            assert_eq!(render_source(r), id as u32);
        }
        for id in 0..11 {
            assert_eq!(render_kind(id), 1);
        }
        for id in 16..23 {
            assert_eq!(render_kind(id), 2);
        }
        assert_eq!(render_capacity(),256+crate::defs::DEFS.len());
    }
}
