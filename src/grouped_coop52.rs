//! Controller ordinals and world owners are separate in grouped multiplayer.

use crate::game::Game;

#[derive(Clone, Copy)]
pub(crate) struct GroupedCoop {
    layout: u32,
    active: u8,
}

impl GroupedCoop {
    fn new(controllers: u32, packed: u32) -> Option<(Self, u32)> {
        if !(3..=4).contains(&controllers) || packed >> (controllers * 2) != 0 { return None; }
        let mut owners = 0;
        for controller in 0..controllers {
            let owner = (packed >> (controller * 2)) & 3;
            if owner > owners { return None; }
            if owner == owners { owners += 1; }
        }
        if owners < 2 || owners == controllers { return None; }
        let layout = (controllers << 8) | packed;
        Some((Self::from_archive_layout(owners, layout)?, owners))
    }

    pub(crate) fn from_archive_layout(owners: u32, layout: u32) -> Option<Self> {
        let (controllers, packed) = (layout >> 8, layout & 255);
        if !(2..=3).contains(&owners) || !(2..=4).contains(&controllers) ||
            packed & 3 != 0 || packed >> (controllers * 2) != 0 ||
            (0..controllers).any(|controller| (packed >> (controller * 2)) & 3 >= owners) { return None; }
        Some(Self { layout, active: (1 << controllers) - 1 })
    }

    pub(crate) fn layout(self) -> u32 { self.layout }

    pub(crate) fn active_owner(self, controller: u32) -> Option<u8> {
        if controller >= self.layout >> 8 || self.active & (1 << controller) == 0 { return None; }
        Some(((self.layout >> (controller * 2)) & 3) as u8)
    }

    pub(crate) fn hash(self, mut mix: impl FnMut(u32)) {
        mix(u32::from_le_bytes(*b"GCP1"));
        mix(self.layout);
        mix(self.active.into());
    }
}

impl Game {
    pub(crate) fn new_network_grouped_coop(seed: u32, ai: u32, controllers: u32, packed: u32, local: u32) -> Option<Self> {
        let (grouping, owners) = GroupedCoop::new(controllers, packed)?;
        let owner = u32::from(grouping.active_owner(local)?);
        Self::new_grouped_world(seed, ai, owners, grouping, owner)
    }

    pub(crate) fn new_grouped_world(seed: u32, ai: u32, owners: u32, grouping: GroupedCoop, owner: u32) -> Option<Self> {
        if ai > 4 - owners { return None; }
        let mut game = if owners == 2 { Self::new(seed, ai, 0, owners, owner) }
            else { Self::new_network_roster(seed, ai, owners, owner)? };
        game.grouped_coop = Some(grouping);
        Some(game)
    }

    pub(crate) fn grouped_resume_layout(&self) -> Option<u32> {
        if !self.network_resume_world_active() || !self.network_departure_allowed(0) { return None; }
        let grouping = self.grouped_coop?;
        let eligible = (0..self.humans).filter(|&owner| self.network_departure_allowed(owner as u8))
            .fold(0u8, |mask, owner| mask | (1 << owner));
        let mut owners = 0u8;
        let (mut count, mut packed) = (0, 0);
        for ordinal in 0..(grouping.layout >> 8) {
            let Some(owner) = grouping.active_owner(ordinal) else { continue; };
            let bit = 1 << owner;
            if eligible & bit == 0 { continue; }
            owners |= bit;
            packed |= u32::from(owner) << (count * 2);
            count += 1;
        }
        if count < 2 || owners != eligible { return None; }
        Some((self.humans << 16) | (count << 8) | packed)
    }

    pub(crate) fn detach_grouped_controller(&mut self, controller: u32) -> bool {
        if controller == 0 || self.net_resignation.is_none() || self.network_match_finished() { return false; }
        let Some(grouping) = self.grouped_coop.as_mut() else { return false; };
        let Some(owner) = grouping.active_owner(controller) else { return false; };
        grouping.active &= !(1 << controller);
        if !(0..(grouping.layout >> 8)).any(|other| grouping.active_owner(other) == Some(owner)) {
            self.apply_network_departure(owner);
        }
        true
    }
}
