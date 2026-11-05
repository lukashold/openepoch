//! The wallet updates at 005E4060 and 005E4120, with their statistics counter.

use crate::ai9_count::{Precision53, BIAS};

pub struct StartAccounts<'a> {
    /// The ten F32 words at Player+9C, in source resource order.
    pub balance: &'a mut [f32; 10],
    /// Container+68 for the tree owner. None when C9C400 bit 9 is set.
    pub natural_spent: Option<&'a mut [f32; 9]>,
    pub precision: Precision53,
}

impl StartAccounts<'_> {
    pub fn debit(self, cost: &[f32; 11]) {
        self.apply(cost, -1.0);
    }

    pub fn refund(self, cost: &[f32; 11]) {
        self.apply(cost, 1.0);
    }

    fn apply(self, cost: &[f32; 11], sign: f64) {
        let Self {
            balance,
            mut natural_spent,
            precision,
        } = self;
        for (index, balance) in balance.iter_mut().enumerate() {
            let biased = precision.add(precision.add(cost[index] as f64, 0.5), BIAS);
            let charged = ((biased.to_bits() as i64) >> 16) as i32;
            if charged <= 0 {
                continue;
            }

            let charge = precision.store_f32(charged as f64);
            let change = sign * charge as f64;
            let remainder = precision.add(*balance as f64, change);
            *balance = precision.store_f32(remainder.clamp(0.0, 999_999.0));
            if let Some(spent) = natural_spent
                .as_mut()
                .and_then(|spent| spent.get_mut(index))
            {
                *spent = precision.store_f32(precision.add(*spent as f64, -change));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    include!("../tools/fixtures/ai9-research-debit-vectors.rs");

    #[test]
    fn independent_fraction_words_cover_native_bias_and_bank_stores() {
        for &(chop, refund, slot, cost, bank, spent, bank_after, spent_after) in DEBIT_VECTORS {
            let precision = if chop {
                Precision53::TowardZero
            } else {
                Precision53::NearestEven
            };
            let mut balance = [f32::from_bits(bank); 10];
            let mut natural_spent = [f32::from_bits(spent); 9];
            let mut price = [0.0; 11];
            price[slot] = f32::from_bits(cost);
            let accounts = StartAccounts {
                balance: &mut balance,
                natural_spent: Some(&mut natural_spent),
                precision,
            };
            if refund {
                accounts.refund(&price);
            } else {
                accounts.debit(&price);
            }
            for (index, value) in balance.into_iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    if index == slot { bank_after } else { bank },
                    "balance {chop}/{refund}/{slot}/{cost:08x}/{bank:08x}"
                );
            }
            for (index, value) in natural_spent.into_iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    if index == slot { spent_after } else { spent },
                    "spent {chop}/{refund}/{slot}/{cost:08x}/{spent:08x}"
                );
            }
        }
    }

    #[test]
    fn stats_disable_does_not_disable_the_wallet_or_charge_slot_ten() {
        let mut balance = [40.0; 10];
        let mut cost = [0.0; 11];
        cost[0] = 2.25;
        cost[9] = 2.75;
        cost[10] = 1_000.0;
        StartAccounts {
            balance: &mut balance,
            natural_spent: None,
            precision: Precision53::NearestEven,
        }
        .debit(&cost);
        assert_eq!(
            balance,
            [38.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 37.0]
        );
    }
}
