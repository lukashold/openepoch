use crate::cloth::Cloth;
#[test]fn extreme_f32_states_stay_inside_bounded_cloth_arithmetic(){
 for scale in[f32::MAX,f32::MIN_POSITIVE,f32::from_bits(1),1.0,1e10]{
  let mut c=Cloth::new(2,2,vec![scale,-scale,0.0,-scale,scale,scale,scale,scale,-scale,-scale,-scale,scale],vec![0;4]);
  for i in 0..16{c.step(if i%3==0{f64::from_bits(1)}else{0.05},[1.0,-1.0,1.0],[1e-200,-1e-200,0.0],[1e9,-1e9,1e9]);assert!(c.pos.iter().chain(&c.vel).all(|v|v.is_finite()));}
  c.light([f32::MAX as f64;3],[f32::from_bits(1)as f64;3],[f32::MAX as f64;3],false);assert!(c.colors.iter().all(|v|v.is_finite()));
 }
}
#[test]fn source_sized_grid_and_finite_context_domain(){
 for n in[2,3,5,8,255]{let positions=(0..n*n).flat_map(|i|[(i%n)as f32*0.01,(i/n)as f32*0.01,0.0]).collect();let mut c=Cloth::new(n,n,positions,vec![0;n*n]);c.step(0.05,[0.,0.,1.],[45.,0.,0.],[180.,0.,0.]);c.light([0.1f32 as f64;3],[0.5;3],[0.25,-0.5,0.75],false);assert!(c.pos.iter().chain(&c.colors).all(|v|v.is_finite()));}
}
