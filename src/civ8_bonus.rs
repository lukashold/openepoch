//! Isolated clean implementations of the directly exported custom trait leaves.
//! Numeric helpers use binary53/nearest intermediates followed by source f32
//! stores. The original callback's temporal x87 controlword is unproved.
pub const PERCENT_LITERAL:f32=f32::from_bits(0x3c23d70a);
pub const BANK_MAX:f32=999999.0;
///006E2B70 and008571B0 both use this exact DOUBLE-bias /__allshr16
///integer conversion. A premature f32 multiply/round is not equivalent.
pub fn cost_integer53(value:f64)->i32 {
    let biased=(value+0.5_f64)+103079215104.0_f64;
    ((biased.to_bits()as i64)>>16)as i32
}
///Normal cost modifier rounds first; German basic/special then rounds again.
///The caller supplies the normal modifier (handicap/area-effect/UP-hook
///construction is separate). Slots9Tech and10Pop must skip the German step.
pub fn natural_price53(raw:u16,normal:f32,german:Option<f32>)->u16 {
    let first=cost_integer53(raw as f64*normal as f64);
    let result=if let Some(german)=german{cost_integer53((first as f32)as f64*german as f64)}else{first};
    result.clamp(0,u16::MAX as i32)as u16
}
///Complete source cost-vector slice:006E2B70 writes0..9 only, so caller
///population10 is preserved.008571B0 modifies natural0..8 only afterward.
pub fn cost_vector53(raw:[u16;10],normal:f32,population:f32,german:Option<(f32,f32)>)->[f32;11]{
    let mut out=[0.0;11];
    for i in 0..10{out[i]=cost_integer53(raw[i]as f64*normal as f64)as f32;}
    out[10]=population;
    if let Some((basic,special))=german{
        for (i,value)in out[..9].iter_mut().enumerate(){
            let modifier=if i<4{basic}else{special};
            *value=cost_integer53(*value as f64*modifier as f64)as f32;
        }
    }
    out
}

fn bank_store(value:f64)->f32 {value.clamp(0.0,BANK_MAX as f64) as f32}

///005F3AB0 only credits a positive reward; it does not debit the old owner.
///The caller must first match the source unit-name list (Temple/University).
pub fn capture_credit(bank:f32,old_owner_bank:f32,amount:f32)->f32 {
    let reward=amount as f64*PERCENT_LITERAL as f64*old_owner_bank as f64;
    if reward>0.0 {bank_store(bank as f64+reward)} else {bank}
}

///005F3D60: lower epoch earns half, equal earns base, higher caps at3 steps.
///The source epoch difference is converted-owner epoch minus receiving epoch.
pub fn conversion_credit(bank:f32,epoch_difference:i32,amount:f32,per_epoch:f32)->f32 {
    let factor=if epoch_difference<0 {0.5_f64}
        else if epoch_difference==0 {1.0_f64}
        else {per_epoch as f64*PERCENT_LITERAL as f64*epoch_difference.min(3) as f64+1.0_f64};
    bank_store(factor*amount as f64+bank as f64)
}

///006E2680 stores required-count * first eligible Military-node price to f32,
///then adds the next Main-node price if there is a next epoch. 005F3F80 applies
///the custom amount percentage and stores the resulting tech bank once.
pub fn wonder_credit(bank:f32,military_price:f32,required_count:u32,next_main_price:Option<f32>,amount:f32)->f32 {
    let research_budget=(required_count as f64*military_price as f64) as f32;
    let total=research_budget as f64+next_main_price.unwrap_or(0.0) as f64;
    bank_store(total*amount as f64*PERCENT_LITERAL as f64+bank as f64)
}

///0053EB00 action5 ->005E0430, including the literal percent conversion.
pub fn cost_modifiers(basic_percent:f32,special_percent:f32)->(f32,f32) {
    ((basic_percent as f64*PERCENT_LITERAL as f64+1.0) as f32,
     (special_percent as f64*PERCENT_LITERAL as f64+1.0) as f32)
}

///Exact006E7AE0 flag/branch/host-count predicate for a node in the native
///current-epoch list. The builder of the flags is a separate runtime binding.
pub fn breakthrough_eligible(flags:u32,branch:u32,host_index:i32,host_count:u32)->bool {
    flags&(1<<3)==0 && branch!=5 && flags&(1<<2)==0 && flags&1!=0 &&
    flags&(1<<1)!=0 && flags&(1<<4)==0 && flags&(1<<6)==0 &&
    (host_index<0 || host_count>0) && branch!=1
}

///006E7AE0 advances the shared source RNG once only when candidates exist.
///Multiplier/increment and initial shared state are explicit inputs: their
///current construction path is not guessed from an uninitialized PE BSS cell.
pub fn choose_breakthrough(state:&mut u32,multiplier:u32,increment:u32,candidates:&[u16])->Option<u16> {
    if candidates.is_empty() {return None;}
    *state=state.wrapping_mul(multiplier).wrapping_add(increment);
    let slot=((*state>>16)&0x7fff) as usize%candidates.len();
    Some(candidates[slot])
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn capture_is_credit_without_transfer(){
        assert_eq!(capture_credit(3.0,100.0,10.0),13.0);
        assert_eq!(capture_credit(3.0,0.0,10.0),3.0);
        assert_eq!(capture_credit(999998.0,100.0,10.0),BANK_MAX);
        assert_eq!(capture_credit(3.0,100.0,-10.0),3.0);
    }
    #[test]fn conversion_epoch_bands_and_cap(){
        assert_eq!(conversion_credit(0.0,-9,3.0,25.0),1.5);
        assert_eq!(conversion_credit(0.0,0,3.0,25.0),3.0);
        assert_eq!(conversion_credit(0.0,1,3.0,25.0),3.75);
        assert_eq!(conversion_credit(0.0,2,3.0,25.0),4.5);
        assert_eq!(conversion_credit(0.0,3,3.0,25.0),5.25);
        assert_eq!(conversion_credit(0.0,14,3.0,25.0),5.25);
        assert_eq!(conversion_credit(999998.0,3,3.0,25.0),BANK_MAX);
    }
    #[test]fn wonder_is_full_epoch_budget_not_only_advance_price(){
        assert_eq!(wonder_credit(0.0,8.0,6,Some(56.0),100.0),104.0);
        assert_eq!(wonder_credit(3.0,8.0,6,Some(56.0),100.0),107.0);
        assert_eq!(wonder_credit(0.0,200.0,6,None,100.0),1200.0);
    }
    #[test]fn german_basic_and_special_are_distinct(){assert_eq!(cost_modifiers(0.0,-25.0),(1.0,0.75));}
    #[test]fn costs_round_in_two_distinct_source_stages(){
        assert_eq!(natural_price53(55,0.9,Some(0.75)),38);
        assert_eq!((55.0_f32*0.9*0.75).round()as u16,37);
        assert_eq!(natural_price53(15,0.9,Some(0.75)),11);
        assert_eq!((15.0_f32*0.9*0.75).round()as u16,10);
        assert_eq!(natural_price53(55,0.9,None),50);
        assert_eq!(natural_price53(55,0.9,Some(1.0)),50);
    }
    #[test]fn german_never_modifies_tech_or_population(){
        let raw=[55;10];let normal=cost_vector53(raw,0.9,3.0,None);
        let german=cost_vector53(raw,0.9,3.0,Some((1.0,0.75)));
        assert_eq!(&german[..4],&normal[..4]);assert_eq!(&german[4..9],&[38.0;5]);
        assert_eq!(german[9].to_bits(),normal[9].to_bits());assert_eq!(german[10].to_bits(),3.0f32.to_bits());
    }
    #[test]fn breakthrough_masks_branch_and_host(){
        assert!(breakthrough_eligible(3,2,-1,0));
        assert!(breakthrough_eligible(3,4,47,1));
        assert!(!breakthrough_eligible(3,2,47,0));
        for bit in [2,3,4,6]{assert!(!breakthrough_eligible(3|1<<bit,2,-1,0));}
        for branch in [1,5]{assert!(!breakthrough_eligible(3,branch,-1,0));}
        assert!(!breakthrough_eligible(1,2,-1,0));
        assert!(!breakthrough_eligible(2,2,-1,0));
    }
    #[test]fn source_rng_consumption_is_explicit(){
        let mut state=0x1234;
        assert_eq!(choose_breakthrough(&mut state,7,3,&[]),None);
        assert_eq!(state,0x1234);
        let next=state.wrapping_mul(7).wrapping_add(3);
        let values=[1,9,20,123];
        assert_eq!(choose_breakthrough(&mut state,7,3,&values),Some(values[((next>>16)&0x7fff)as usize%4]));
        assert_eq!(state,next);
    }
}
