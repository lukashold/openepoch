//! Ledger effects of an already computed original trade delivery.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Account {
    pub gold: f32,
    pub technology: f32,
}

const FOREIGN_SHARE: f64 = 0.2f32 as f64;

fn add(balance: f32, value: f64) -> f32 {
    (f64::from(balance) + value).clamp(0.0, 999999.0) as f32
}

pub fn deliver(owner: &mut Account, foreign_destination: Option<(&mut Account, bool)>, amount: Account) {
    if amount.gold > 0.0 {
        owner.gold = add(owner.gold, f64::from(amount.gold));
        if let Some((other, paid_at_destination)) = foreign_destination {
            let share = FOREIGN_SHARE * f64::from(amount.gold);
            owner.gold = add(owner.gold, share);
            if paid_at_destination { other.gold = add(other.gold, share); }
        }
    }
    if amount.technology > 0.0 {
        owner.technology = add(owner.technology, f64::from(amount.technology));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const START: Account = Account { gold: 20.0, technology: 5.0 };
    const LOAD: Account = Account { gold: 40.0, technology: 1.0 };

    #[test]
    fn only_the_owner_receives_technology_and_domestic_gold() {
        let mut owner = START;
        deliver(&mut owner, None, LOAD);
        assert_eq!(owner, Account { gold: 60.0, technology: 6.0 });
    }

    #[test]
    fn foreign_route_pays_the_owner_bonus_on_both_legs_and_the_partner_once() {
        let (mut owner, mut partner) = (START, START);
        deliver(&mut owner, Some((&mut partner, true)), LOAD);
        assert_eq!(owner, Account { gold: 68.0, technology: 6.0 });
        assert_eq!(partner, Account { gold: 28.0, technology: 5.0 });
        deliver(&mut owner, Some((&mut partner, false)), LOAD);
        assert_eq!(owner, Account { gold: 116.0, technology: 7.0 });
        assert_eq!(partner, Account { gold: 28.0, technology: 5.0 });
    }

    #[test]
    fn both_gold_credits_and_the_technology_credit_saturate() {
        let (mut owner, mut partner) = (Account {gold:999956.0,technology:999998.5}, START);
        deliver(&mut owner, Some((&mut partner, true)), LOAD);
        assert_eq!(owner, Account {gold:999999.0,technology:999999.0});
        assert_eq!(partner, Account {gold:28.0,technology:5.0});
    }
}
