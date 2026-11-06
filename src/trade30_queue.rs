//! Isolated host loading slot and wait-order policy for the original trade route.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trader {
    pub actor: u32,
    pub owner: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Loaded(Trader),
    Hostile(Trader),
}

#[derive(Default)]
pub struct Queue {
    active: Option<Trader>,
    progress: f32,
    waiting: Vec<Trader>,
}

impl Queue {
    pub fn active(&self) -> Option<Trader> { self.active }

    pub fn actors(&self) -> impl Iterator<Item = u32> + '_ {
        self.active.iter().chain(&self.waiting).map(|trader| trader.actor)
    }

    pub fn is_empty(&self) -> bool { self.active.is_none() && self.waiting.is_empty() }

    pub fn clear(&mut self) { *self = Self::default(); }

    pub fn remove(&mut self, actor: u32, is_alive: impl Fn(u32) -> bool) {
        if let Some(active) = self.active.filter(|active| active.actor == actor) {
            self.active = None;
            self.progress = 0.0;
            self.promote(Some(active.owner), is_alive);
        } else {
            self.waiting.retain(|trader| trader.actor != actor);
        }
    }

    pub fn snapshot(&self) -> (Option<Trader>, f32, &[Trader]) {
        (self.active, self.progress, &self.waiting)
    }

    pub fn restore(active: Trader, progress: f32, waiting: Vec<Trader>) -> Self {
        Self { active: Some(active), progress, waiting }
    }

    pub fn arrive(&mut self, trader: Trader, host_accepting: bool) {
        if self.active.is_none() && host_accepting {
            self.start(trader);
        } else {
            self.waiting.push(trader);
        }
    }

    pub fn update(&mut self, seconds: f32, time_to_load: f32, host_accepting: bool, hostile: bool,
                  can_exit: bool, is_alive: impl Fn(u32) -> bool) -> Option<Exit> {
        if !host_accepting { return None; }
        let Some(trader) = self.active else {
            self.promote(None, is_alive);
            return None;
        };
        if !is_alive(trader.actor) { return None; }
        self.progress = if time_to_load <= 0.0 { 1.0 } else {
            (f64::from(self.progress) + f64::from(seconds) / f64::from(time_to_load)) as f32
        };
        let exit = if self.progress >= 1.0 { Exit::Loaded(trader) } else if hostile { Exit::Hostile(trader) }
                   else { return None; };
        if !can_exit { return None; }
        self.active = None;
        self.promote(Some(trader.owner), is_alive);
        Some(exit)
    }

    fn start(&mut self, trader: Trader) {
        self.active = Some(trader);
        self.progress = 0.0;
    }

    fn promote(&mut self, previous_owner: Option<u8>, is_alive: impl Fn(u32) -> bool) {
        if let Some(previous_owner) = previous_owner {
            self.waiting.retain(|trader| is_alive(trader.actor));
            let best = self.waiting.iter().enumerate().min_by_key(|(_, trader)|
                (trader.owner <= previous_owner, trader.owner)).map(|(index, _)| index);
            if let Some(index) = best {
                let trader = self.waiting.remove(index);
                self.start(trader);
            }
        } else if let Some(index) = self.waiting.iter().position(|trader| is_alive(trader.actor)) {
            let trader = self.waiting[index];
            self.waiting.drain(..=index);
            self.start(trader);
        } else {
            self.waiting.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn trader(actor: u32, owner: u8) -> Trader { Trader { actor, owner } }

    #[test]
    fn five_seconds_requires_the_next_tick_after_float_accumulation() {
        let mut host = Queue::default();
        host.arrive(trader(1, 0), true);
        for _ in 0..100 { assert_eq!(host.update(0.05, 5.0, true, false, true, |_| true), None); }
        assert!(host.progress < 1.0);
        assert_eq!(host.update(0.05, 5.0, true, false, true, |_| true), Some(Exit::Loaded(trader(1, 0))));
    }

    #[test]
    fn first_slot_scans_only_until_a_live_trader() {
        let mut host = Queue::default();
        for unit in [trader(1, 0), trader(2, 4), trader(3, 0), trader(4, 1)] { host.arrive(unit, false); }
        host.update(0.05, 5.0, true, false, true, |id| id % 2 == 0);
        assert_eq!(host.active, Some(trader(2, 4)));
        assert_eq!(host.waiting, [trader(3, 0), trader(4, 1)]);
        assert_eq!(host.progress, 0.0);
    }

    #[test]
    fn completed_slot_rotates_player_order_and_keeps_fifo_within_player() {
        let mut host = Queue::default();
        host.arrive(trader(1, 2), true);
        for unit in [trader(2, 4), trader(3, 1), trader(4, 3), trader(5, 3), trader(6, 0)] { host.arrive(unit, true); }
        assert_eq!(host.update(0.05, 0.0, true, false, true, |id| id != 2), Some(Exit::Loaded(trader(1, 2))));
        assert_eq!(host.active, Some(trader(4, 3)));
        assert_eq!(host.progress, 0.0);
        assert_eq!(host.waiting, [trader(3, 1), trader(5, 3), trader(6, 0)]);
        host.update(0.05, 0.0, true, false, true, |_| true);
        assert_eq!(host.active, Some(trader(6, 0)));
    }

    #[test]
    fn completion_precedes_hostility_and_blocked_exit_preserves_the_loaded_slot() {
        let mut host = Queue::default();
        host.arrive(trader(1, 0), true);
        host.arrive(trader(2, 1), true);
        assert_eq!(host.update(0.05, 0.01, true, true, false, |_| true), None);
        assert!(host.progress > 1.0);
        assert_eq!(host.update(0.05, 0.01, true, true, true, |_| true), Some(Exit::Loaded(trader(1, 0))));
        assert_eq!(host.update(0.05, 5.0, true, true, true, |_| true), Some(Exit::Hostile(trader(2, 1))));
    }
}
