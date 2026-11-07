//! Per-player market quotes and ordered 100-resource transactions.

use super::*;
use super::crowns54::score::source_math;
use crate::economy::NATIVE_IDS;

const MAGIC: u32 = u32::from_le_bytes(*b"MRK1");
const GOLD: usize = 4;
const LOT: i32 = 100;
const SPREAD: u16 = 20;
const DEFAULT_BUY: u16 = 110;
const MIN_BUY: u16 = 40;
const MAX_BUY: u16 = 900;

pub(crate) struct Market { buy: [[u16; 8]; 4] }

impl Default for Market {
    fn default() -> Self { Self { buy: [[DEFAULT_BUY; 8]; 4] } }
}

impl Market {
    fn changes(&self) -> impl Iterator<Item = (usize, u16)> + '_ {
        self.buy.iter().flatten().copied().enumerate().filter(|&(_, buy)| buy != DEFAULT_BUY)
    }

    fn mask(&self) -> u32 {
        self.changes().fold(0, |mask, (slot, _)| mask | 1 << slot)
    }

    pub(crate) fn valid_players(&self, players: usize) -> bool {
        self.changes().all(|(slot, _)| slot / 8 < players)
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        let mask = self.mask();
        if mask == 0 { return; }
        write(MAGIC);
        write(mask);
        for (_, value) in self.changes() { write(u32::from(value)); }
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        let mask = self.mask();
        if mask == 0 || core.is_empty() { return core; }
        let mut bytes = Vec::with_capacity(core.len() + 12 + mask.count_ones() as usize * 2);
        bytes.extend_from_slice(&MAGIC.to_le_bytes());
        bytes.extend_from_slice(&mask.to_le_bytes());
        for (_, value) in self.changes() { bytes.extend_from_slice(&value.to_le_bytes()); }
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Market)> {
    if !bytes.starts_with(&MAGIC.to_le_bytes()) { return Some((bytes, Market::default())); }
    let mask = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?);
    let start = 8 + mask.count_ones() as usize * 2;
    let end = bytes.len().checked_sub(4)?;
    let checksum = u32::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    if mask == 0 || start >= end || crate::save::checksum(&bytes[..end]) != checksum { return None; }
    let mut market = Market::default();
    let mut values = bytes.get(8..start)?.chunks_exact(2);
    for (slot, buy) in market.buy.iter_mut().flatten().enumerate() {
        if mask & (1 << slot) == 0 { continue; }
        *buy = u16::from_le_bytes(values.next()?.try_into().ok()?);
        if !(MIN_BUY..=MAX_BUY).contains(buy) || *buy == DEFAULT_BUY { return None; }
    }
    Some((&bytes[start..end], market))
}

fn quote_slot(native: u8) -> Option<usize> {
    match native { 0..=2 => Some(native as usize), 4..=8 => Some(native as usize - 1), _ => None }
}

pub(crate) fn commodities(epoch: u8) -> [u8; 5] {
    match epoch {
        0..=2 => [0, 1, 2, 4, u8::MAX],
        3..=5 => [0, 1, 2, 4, 5],
        6..=8 => [0, 1, 2, 5, 6],
        9..=11 => [0, 1, 2, 6, 7],
        _ => [0, 1, 2, 7, 8],
    }
}

#[derive(Clone, Copy)]
struct Order {
    slot: usize,
    own_slot: usize,
    lots: u16,
    buy: bool,
    drift: u16,
}

impl Order {
    fn step(self, buy_price: u16) -> (u16, u16) {
        if self.buy {
            (buy_price, (buy_price + self.drift).min(MAX_BUY))
        } else {
            (buy_price - SPREAD, buy_price.saturating_sub(self.drift).max(MIN_BUY))
        }
    }
}

impl Game {
    fn market_order(&self, owner: u8, host: u32, native: u8, lots: i32) -> Option<Order> {
        if self.net && self.crown_rules() == 0 || self.bench != 0 || self.winner != 0 || owner as usize >= self.nplayers
            || !matches!(lots, -5 | -1 | 1 | 5) { return None; }
        let index = self.get(host)?;
        let actor = &self.ents[index];
        if actor.team != owner || actor.def != MARKET || actor.built < 1.0 || actor.hp <= 0.0 || self.sabotaged(index)
            || !commodities(self.players[owner as usize].epoch).contains(&native) { return None; }
        Some(Order { slot: quote_slot(native)?, own_slot: NATIVE_IDS.iter().position(|&id| id == native)?,
            lots: lots.unsigned_abs() as u16, buy: lots > 0,
            drift: if self.crown_market_stable(owner) { 3 } else { 5 } })
    }

    pub(crate) fn market_quote(&self, owner: u8, host: u32, native: u8, lots: i32) -> Option<(u16, bool)> {
        let order = self.market_order(owner, host, native, lots)?;
        let mut buy_price = self.market.buy[owner as usize][order.slot];
        let mut total = 0;
        for _ in 0..order.lots {
            let (price, next) = order.step(buy_price);
            total += price;
            buy_price = next;
        }
        let available = if order.buy { self.resource(owner, GOLD) >= f32::from(total) }
            else { self.resource(owner, order.own_slot) >= f32::from(order.lots) * LOT as f32 };
        Some((total, available))
    }

    pub(crate) fn market_trade(&mut self, owner: u8, host: u32, native: u8, lots: i32) -> bool {
        let Some(order) = self.market_order(owner, host, native, lots) else { return false; };
        if !self.market_quote(owner, host, native, lots).is_some_and(|(_, available)| available) { return false; }
        let mut buy_price = self.market.buy[owner as usize][order.slot];
        for _ in 0..order.lots {
            let (price, next) = order.step(buy_price);
            let (quantity, gold) = if order.buy { (LOT, -i32::from(price)) } else { (-LOT, i32::from(price)) };
            for (slot, delta) in [(order.own_slot, quantity), (GOLD, gold)] {
                let total = source_math::add(f64::from(self.resource(owner, slot)), f64::from(delta));
                self.set_resource(owner, slot, source_math::f(total.clamp(0.0, 999999.0)) as f32);
            }
            buy_price = next;
        }
        self.market.buy[owner as usize][order.slot] = buy_price;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_lots_are_atomic_per_owner_and_quotes_resume() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        let city = game.spawn(MARKET, 0, 80.0, 80.0);
        let rival = game.spawn(MARKET, 1, 90.0, 90.0);
        let (market, foreign) = (game.h(city), game.h(rival));
        game.set_resource(0, GOLD, 599.0);
        assert_eq!(game.market_quote(0, market, 0, 5), Some((600, false)));
        assert_eq!(game.market_quote(0, market, 0, -5), Some((400, false)));
        let hash = game.state_hash();
        assert!(!game.market_trade(0, market, 0, 5));
        assert!(!game.market_trade(0, foreign, 0, 1));
        assert!(!game.market_trade(0, market, 3, 1));
        assert!(!game.market_trade(0, market, 8, 1));
        assert_eq!(game.state_hash(), hash);
        game.set_resource(0, GOLD, 600.0);
        let food = game.resource(0, 0);
        game.apply(0, crate::cmd::C_MARKET, 0.0, 0.0, 5, &[market]);
        assert_eq!(game.resource(0, 0), food + 500.0);
        assert_eq!(game.resource(0, GOLD), 0.0);
        assert_eq!(game.market_quote(0, market, 0, 1), Some((135, false)));
        assert_eq!(game.market_quote(1, foreign, 0, 1).unwrap().0, 110);
        game.save();
        let bytes = game.savebuf.clone();
        let mut restored = Game::load(&bytes).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.market_quote(0, market, 0, -5), Some((525, true)));
        restored.save();
        assert_eq!(restored.savebuf, bytes);
    }

    #[test]
    fn crown_market_stability_slows_only_its_holders_future_drift() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        let index = game.spawn(MARKET, 0, 80.0, 80.0);
        let market = game.h(index);
        let gold = 1300.0;
        game.set_resource(0, GOLD, gold);
        assert!(game.configure_crowns(false));
        game.researched[0][0] |= 0b1111 << 4;
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(1, 6));
        assert_eq!(game.market_quote(0, market, 1, 5), Some((580, true)));
        assert_eq!(game.market_quote(0, market, 1, -5).unwrap().0, 420);
        assert!(game.market_trade(0, market, 1, 5));
        assert_eq!(game.resource(0, GOLD), gold - 580.0);
        assert_eq!(game.market_quote(0, market, 1, 1), Some((125, true)));
        game.tick = 6040;
        game.tick_crowns();
        assert_eq!(game.market_quote(0, market, 1, 5), Some((675, true)));
    }
}
