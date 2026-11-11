//! Native NearWater predicate projected onto the current independent terrain.
//! 00862220 case3 ->00716c70 ->00716880(center,4,out,0,1,1,0).
//! Original cells6(River) are excluded;7(Ocean) and5 are accepted. The current
//! grid has no native cell taxonomy: deep water is the explicit accepted-water
//! adapter, while its combined river/shallows class is conservatively excluded.
//! Original coast orientation, native height raster and rectangle sampling
//! remain separate from this bounded placement restriction.
use crate::map::*;

pub fn approximate_distance(dx:f32,dy:f32)->f64 {
    let a=dx.abs()as f64;let b=dy.abs()as f64;
    let (large,small)=if a>b{(a,b)}else{(b,a)};
    if large==0.0 {0.0}else{large+(small/large)*small*0.4569999873638153_f64}
}

pub fn near_accepted_water(map:&Map,x:f32,y:f32)->bool {
    const RADIUS:f32=4.0;
    let [width, height] = map.simulation_dimensions().map(|side| side as i32);
    for yy in ((y-RADIUS).floor()as i32).max(0)..=((y+RADIUS).ceil()as i32).min(height-1) {
        for xx in ((x-RADIUS).floor()as i32).max(0)..=((x+RADIUS).ceil()as i32).min(width-1) {
            let t=map.tile_index(xx as usize, yy as usize);
            if map.ter[t]==T_DEEP && map.height(xx as f32,yy as f32)<0.0
                && approximate_distance(xx as f32-x,yy as f32-y)<=RADIUS as f64 {return true;}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn distance_matches_native_constant_and_inclusive_boundary() {
        assert_eq!(approximate_distance(0.0,0.0),0.0);
        assert_eq!(approximate_distance(-4.0,0.0),4.0);
        assert_eq!(approximate_distance(2.0,3.0),3.0+2.0/3.0*2.0*0.4569999873638153);
        assert!(approximate_distance(3.0,3.0)>4.0);
    }
    #[test] fn inland_rejects_shallow_adapter_rejects_deep_negative_height_accepts() {
        let mut map=Map::new(7,2);map.ter.fill(T_GRASS);map.hm.fill(0.5);
        assert!(!near_accepted_water(&map,100.0,100.0));
        let t=100*N+104;map.ter[t]=T_SHALLOW;
        map.hm.fill(-0.2);
        assert!(!near_accepted_water(&map,100.0,100.0));
        map.ter[t]=T_DEEP;
        assert!(near_accepted_water(&map,100.0,100.0));
        assert!(!near_accepted_water(&map,99.99,100.0));
    }
}
