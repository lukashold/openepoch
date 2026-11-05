use crate::{game::*,defs::*};
fn game()->Game{Game::new(7,1,0,1,0)}
#[test]fn eighty_source_definitions_and_cross64_ids(){
    assert_eq!(DEFS.len(),80);assert_eq!(DEFS[64].name,"UniqueUnitBabylonian2");
    assert_eq!(DEFS[79].name,"UniqueUnitTurkish1");
    assert_eq!(crate::ids::decode_job(crate::ids::unit_job(79)),Some((0,79)));
    assert_eq!(crate::ids::render_source(crate::ids::unit_render(79)as crate::ids::RenderId),79);
}
#[test]fn all_fourteen_civs_keep_native_unique_gates(){
    for civ in 0..14{
        let mut g=game();assert!(g.configure_civilizations([civ,14,14,14]));
        for p in crate::civ8_gates::CANDIDATES{
            assert_eq!(g.definition_civ_available(0,p.proposed_def),p.ttciv==civ+1);
        }
        for def in 0..61{assert!(g.definition_civ_available(0,def));}
        assert!(!g.configure_civilizations([civ,14,14,14]));
    }
}
#[test]fn unresolved_random_and_unused_seat_configuration_rejected(){
    let mut g=game();let before=g.state_hash();
    assert!(!g.configure_civilizations([15,14,14,14]));
    assert!(!g.configure_civilizations([0,14,0,14]));
    assert_eq!(g.state_hash(),before);
    assert!(g.configure_civilizations([14;4]));
    assert!(!g.configure_civilizations([0,14,14,14]));
}
#[test]fn pending_payments_close_civilization_configuration(){
    let mut g=game();let tc=g.ents[..g.high].iter().position(|e|e.alive&&e.team==0&&e.def==TC).unwrap();
    assert!(g.train(tc,57));let before=g.state_hash();let price=g.queue_payment(tc,0);
    assert!(!g.configure_civilizations([0,14,14,14]));
    assert_eq!(g.state_hash(),before);assert_eq!(g.queue_payment(tc,0),price);
}
#[test]fn source_starting_traits_change_real_statistics_and_costs(){
    let mut german=game();let mut english=game();
    let hp=german.max_hp(0,9);let sail=english.effective_weapon_damage(0,58,DEFS[58].w[0].dmg);
    let special=german.unit_cost(0,19);
    assert!(german.configure_civilizations([0,14,14,14]));
    assert!(english.configure_civilizations([1,14,14,14]));
    assert!((german.max_hp(0,9)-hp*1.1).abs()<0.001);
    assert!((english.effective_weapon_damage(0,58,DEFS[58].w[0].dmg)-sail*1.1).abs()<0.001);
    for slot in 0..9{
        let expected=if crate::economy::NATIVE_IDS[slot]<4{special[slot]}else{(special[slot]as f32*0.75).round()as u16};
        assert_eq!(german.unit_cost(0,19)[slot],expected);
    }
}
#[test]fn far_east_population_trait_and_civ_save_roundtrip(){
    let mut g=game();let cap=g.modifiers[0][TC as usize][18];
    assert!(g.configure_civilizations([5,14,14,14]));
    assert_eq!(g.modifiers[0][TC as usize][18],cap*1.25);
    g.build_grid();g.refresh_display_fog();g.save();let bytes=g.savebuf.clone();
    #[cfg(feature="ee2-pack8")]let inner=crate::pack8_save::split(&bytes).unwrap().0;
    #[cfg(not(feature="ee2-pack8"))]let inner=&bytes[..];
    assert_eq!(u32::from_le_bytes(inner[..4].try_into().unwrap()),if cfg!(feature="ee2-citizen9"){0x3955454f}else{crate::civ8_save::MAGIC});
    let mut loaded=Game::load(&bytes).expect("actual8 save");
    assert_eq!(loaded.civilizations,g.civilizations);assert_eq!(loaded.state_hash(),g.state_hash());
    loaded.save();assert_eq!(loaded.savebuf,bytes);
    assert!(!loaded.configure_civilizations([0,14,14,14]));
}
#[test]fn malformed_civ_descriptor_tail_and_legacy_downgrade_are_rejected(){
    let mut g=game();assert!(g.configure_civilizations([3,14,14,14]));
    g.build_grid();g.refresh_display_fog();g.save();let b=g.savebuf.clone();
    let start=b.len()-52-36;
    for at in [0,12,start,start+4,start+8,start+12,start+16,start+20,start+24,start+28,start+32,b.len()-52,b.len()-48,b.len()-44,b.len()-40,b.len()-8,b.len()-4]{
        let mut corrupt=b.clone();corrupt[at..at+4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Game::load(&corrupt).is_none(),"malformed offset{at}");
    }
    let mut downgraded=b[..start].to_vec();
    downgraded[..4].copy_from_slice(&crate::paid_queue7::SAVE_MAGIC.to_le_bytes());
    crate::paid_queue7::append_descriptor(&mut downgraded);
    assert!(Game::load(&downgraded).is_none());
    for chop in 1..=88{assert!(Game::load(&b[..b.len()-chop]).is_none());}
}
#[test]fn source_main6_obsoletes_front_and_pending_unique_jobs_with_paid_refund(){
    let mut g=game();assert!(g.configure_civilizations([3,14,14,14]));
    for slot in 0..9{let old=g.resource(0,slot);g.add_resource(0,slot,10000.0-old);}
    g.players[0].epoch=4;g.rebuild_modifiers(0,false);
    let b=g.spawn(BARRACKS,0,70.5,70.5);g.ents[b].built=1.0;g.build_grid();
    let before:Vec<f32>=(0..9).map(|r|g.resource(0,r)).collect();
    for _ in 0..5{assert!(g.train(b,61));}
    g.tick();assert!(g.ents[b].qt>0.0);
    let paid:Vec<_>=(0..5).map(|p|g.queue_payment(b,p)).collect();
    assert!(paid[0].duration>=0.0);assert!(paid[1..].iter().all(|p|p.duration<0.0));
    g.players[0].epoch=5;g.rebuild_modifiers(0,false);g.apply_paid_main(0,4);
    assert_eq!(g.ents[b].qn,0);assert_eq!(g.ents[b].qt,0.0);
    for slot in 0..9{assert_eq!(g.resource(0,slot),before[slot]);}
    assert!(!g.can_train(b,61));
}
#[test]fn installed_research_discount_then_german_iron_billing_is_two_stage(){
    let mut g=game();assert!(g.configure_civilizations([0,14,14,14]));
    g.players[0].epoch=5;g.rebuild_modifiers(0,false);
    let raw=crate::ee2_units::lookup_class(22,5).unwrap().cost9.unwrap();assert_eq!(raw[2],55);
    let node=&crate::ee2_research::NODES[26];
    assert!(node.effects.iter().any(|e|e.field==4&&e.value == -10.0&&crate::ids::mask_has(e.mask,22)));
    g.researched[0][26/32]|=1<<(26%32);g.rebuild_modifiers(0,false);
    assert_eq!(g.unit_cost(0,22)[2],38); // declared nearest53 model; not all nativeRC contexts.
}
