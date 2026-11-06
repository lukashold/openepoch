//! Isolated projection of an already-authorized original trade cargo quote.
//! The caller supplies the selected pool, endpoints and original RNG stream.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cargo {
    pub id: u32,
    pub bonus: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quote {
    pub cargo: u32,
    pub gold: f32,
    pub tech: f32,
}

pub struct Terms<'a> {
    pub gold_per_tile: f32,
    pub tech_per_tile: f32,
    pub tech_per_epoch: f32,
    pub tech_chances: &'a [f32],
}

pub fn quote(
    terms: &Terms<'_>, home: [f32; 2], destination: [f32; 2],
    same_owner: bool, epoch_gap: i32, cargo_pool: &[Cargo], carried_cargo: Option<u32>,
    mut next_word: impl FnMut() -> u16,
) -> Option<Quote> {
    let cargo = if let Some(id) = carried_cargo {
        Cargo { id, bonus: 0.0 }
    } else {
        if cargo_pool.is_empty() { return None; }
        cargo_pool[next_word() as usize % cargo_pool.len()]
    };
    let dx = f64::from(home[0]) - f64::from(destination[0]);
    let dy = f64::from(home[1]) - f64::from(destination[1]);
    let distance = f64::from(((dx * dx + dy * dy).sqrt()) as f32);
    let gold = ((1.0 + f64::from(cargo.bonus)) * f64::from(terms.gold_per_tile) * distance).max(1.0) as f32;
    let mut tech = 0.0;
    if !same_owner {
        let chance = (f64::from(next_word()) * f64::from(f32::from_bits(0x38000100))) as f32;
        if epoch_gap > 0 {
            let threshold = terms.tech_chances.get((epoch_gap - 1) as usize).or_else(|| terms.tech_chances.last());
            if threshold.is_some_and(|threshold| chance <= *threshold) {
                tech = (distance * f64::from(terms.tech_per_tile)
                    * (1.0 + f64::from(epoch_gap) * f64::from(terms.tech_per_epoch))).max(1.0) as f32;
            }
        }
    }
    Some(Quote { cargo: cargo.id, gold, tech })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CART: Terms<'_> = Terms {
        gold_per_tile: 0.2, tech_per_tile: 0.0, tech_per_epoch: 0.0,
        tech_chances: &[0.1, 0.15, 0.2, 0.25, 0.3],
    };
    const POOL: &[Cargo] = &[Cargo { id: 7, bonus: 0.0 }, Cargo { id: 19, bonus: 0.25 }];

    #[test]
    fn domestic_quote_picks_a_cargo_and_spends_no_tech_draw() {
        let mut words = [15].into_iter();
        let result = quote(&CART, [7.0, 8.0], [10.0, 12.0], true, 0, POOL, None, || words.next().unwrap());
        assert_eq!(result, Some(Quote { cargo: 19, gold: 1.25, tech: 0.0 }));
        assert!(words.next().is_none());
    }

    #[test]
    fn a_carried_cargo_has_no_new_draw_or_cargo_multiplier() {
        let result = quote(&CART, [0.0, 0.0], [20.0, 0.0], true, 0, &[], Some(19), || panic!("unexpected draw"));
        assert_eq!(result, Some(Quote { cargo: 19, gold: 4.0, tech: 0.0 }));
    }

    #[test]
    fn foreign_quote_always_spends_the_chance_draw_even_without_an_epoch_gap() {
        let mut words = [0, 0].into_iter();
        let result = quote(&CART, [1.0, 2.0], [2.0, 2.0], false, -3, POOL, None, || words.next().unwrap());
        assert_eq!(result, Some(Quote { cargo: 7, gold: 1.0, tech: 0.0 }));
        assert!(words.next().is_none());
    }

    #[test]
    fn equal_threshold_grants_the_one_point_minimum_even_at_zero_base_rate() {
        let terms = Terms { tech_chances: &[f32::from_bits(0x38000100) * 3277.0], ..CART };
        for (word, tech) in [(3276, 1.0), (3277, 1.0), (3278, 0.0)] {
            let result = quote(&terms, [0.0, 0.0], [20.0, 0.0], false, 7, &[], Some(7), || word);
            assert_eq!(result, Some(Quote { cargo: 7, gold: 4.0, tech }));
        }
    }
}
