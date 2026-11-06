//! Source unit rows from active UP CSV5 and the TradeCart/MerchantShip DDFs.

use crate::defs::{Def, DefId, A_INF, A_SHIP, DOCK, LAND, MARKET, NOW, SEA};
use crate::ee2_units::UnitRow;

pub(crate) const TRADE_CART: DefId = 81;
pub(crate) const MERCHANT_SHIP: DefId = 82;

const CART: Def = Def {
    name: "Handelskarren", epoch: 1, until: 15, dom: LAND, building: false, size: 0.8,
    hp: 50.0, speed: 2.3, sight: 5.0, armor: A_INF, cost: [25, 25, 0], time: 26.0,
    pop: 1, made_at: MARKET, flags: 0, w: [NOW; 2],
};
const SHIP: Def = Def {
    name: "Handelsschiff", epoch: 0, until: 15, dom: SEA, building: false, size: 2.0,
    hp: 50.0, speed: 1.725, sight: 5.0, armor: A_SHIP, cost: [0, 40, 0], time: 32.5,
    pop: 1, made_at: DOCK, flags: 0, w: [NOW; 2],
};

pub(crate) const fn append_traders(existing: [Def; 81]) -> [Def; 83] {
    let mut result = [CART; 83];
    let mut index = 0;
    while index < existing.len() {
        result[index] = existing[index];
        index += 1;
    }
    result[MERCHANT_SHIP as usize] = SHIP;
    result
}

const CART_ROW: UnitRow = UnitRow {
    id: TRADE_CART, epoch: 1, until: 5, source_epoch: 2, source_unit: "TradeCart",
    source_visual: "TradeCart", source_host: "Market", identity_verified: true, model_verified: true,
    hp: Some(50.0), buildtime: Some(26.0), cost9: Some([25,25,0,0,0,0,0,0,0]),
    pop: Some(1), pop_capacity: None, sight: Some(5.0), raw_size: Some([0.4,0.4]),
    raw_footprint: None, raw_speed: Some(2.3), gather: None, farm_rate: None, carry: None,
    farm_carry: None, repair_rate: None, damage: None, range: None, reload: None, garrison_slots: None,
    generation: None, generation_resource: None, garrison_heal: None, garrison_attack_bonus: None,
    garrison_size: None, fortress_garrison: false, attack_missile: None,
};
const SHIP_ROW: UnitRow = UnitRow {
    id: MERCHANT_SHIP, epoch: 0, source_epoch: 1, source_unit: "MerchantShip", source_visual: "MerchantShip",
    source_host: "Dock", buildtime: Some(32.5), cost9: Some([0,40,0,0,20,0,0,0,0]),
    raw_size: Some([1.0,0.5]), raw_speed: Some(1.725), ..CART_ROW
};
const ROWS: [UnitRow; 6] = [
    CART_ROW,
    UnitRow { epoch: 5, until: 10, source_epoch: 6, source_visual: "TradeCart_6", hp: Some(75.0), ..CART_ROW },
    UnitRow { epoch: 10, until: 15, source_epoch: 11, source_visual: "TradeCart_11", hp: Some(100.0), ..CART_ROW },
    SHIP_ROW,
    UnitRow { epoch: 5, until: 10, source_epoch: 6, source_visual: "MerchantShip_6", raw_size: Some([1.2,0.4]), ..SHIP_ROW },
    UnitRow { epoch: 10, until: 15, source_epoch: 11, source_visual: "MerchantShip_11", raw_size: Some([1.6,0.3]), ..SHIP_ROW },
];

pub(crate) fn lookup(id: DefId, epoch: u8) -> Option<&'static UnitRow> {
    ROWS.iter().find(|row| row.id == id && (row.epoch..row.until).contains(&epoch))
}

pub(crate) fn is_trader(id: DefId) -> bool {
    matches!(id, TRADE_CART | MERCHANT_SHIP)
}
