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
        Some((Self { layout: (controllers << 8) | packed, active: (1 << controllers) - 1 }, owners))
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
        if ai > 4 - owners { return None; }
        let owner = u32::from(grouping.active_owner(local)?);
        let mut game = if owners == 2 { Self::new(seed, ai, 0, owners, owner) }
            else { Self::new_network_roster(seed, ai, owners, owner)? };
        game.grouped_coop = Some(grouping);
        Some(game)
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
