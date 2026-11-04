mod math;
mod cloth;
#[cfg(test)]mod tests;
use core::ptr::addr_of_mut;
static mut INPUT:Vec<f32>=Vec::new();
static mut PINS:Vec<u8>=Vec::new();
static mut CLOTH:Vec<Option<cloth::Cloth>>=Vec::new();
fn slot(id:u32)->Option<&'static mut cloth::Cloth>{unsafe{(&mut *addr_of_mut!(CLOTH)).get_mut(id as usize)?.as_mut()}}
#[no_mangle]pub extern "C" fn cloth_abi_revision()->u32{1}
#[no_mangle]pub extern "C" fn input_alloc(n:u32)->*mut f32{if n>196605{return core::ptr::null_mut();}unsafe{let b=&mut *addr_of_mut!(INPUT);b.resize(n as usize,0.0);b.as_mut_ptr()}}
#[no_mangle]pub extern "C" fn pins_alloc(n:u32)->*mut u8{if n>65535{return core::ptr::null_mut();}unsafe{let b=&mut *addr_of_mut!(PINS);b.resize(n as usize,0);b.as_mut_ptr()}}
#[no_mangle]pub extern "C" fn cloth_reset(){unsafe{(&mut *addr_of_mut!(CLOTH)).clear();}}
#[no_mangle]pub extern "C" fn cloth_create(nx:u32,ny:u32)->u32{
 if !(2..=255).contains(&nx)||!(2..=255).contains(&ny){return u32::MAX;}
 unsafe{let p=&*addr_of_mut!(INPUT);let pins=&*addr_of_mut!(PINS);let n=(nx*ny)as usize;if p.len()!=n*3||pins.len()!=n||!p.iter().all(|v|v.is_finite()&&v.abs()<1e10)||!pins.iter().all(|&v|v<=1){return u32::MAX;}
 let c=cloth::Cloth::new(nx as usize,ny as usize,p.clone(),pins.clone());let all=&mut *addr_of_mut!(CLOTH);if let Some(i)=all.iter().position(Option::is_none){all[i]=Some(c);i as u32}else{all.push(Some(c));(all.len()-1)as u32}}
}
#[no_mangle]pub extern "C" fn cloth_drop(id:u32)->u32{unsafe{if let Some(s)=(&mut *addr_of_mut!(CLOTH)).get_mut(id as usize){*s=None;1}else{0}}}
#[no_mangle]pub extern "C" fn cloth_step(id:u32,dt:f64,gx:f64,gy:f64,gz:f64,lx:f64,ly:f64,lz:f64,hx:f64,hy:f64,hz:f64)->u32{
 if !dt.is_finite()||dt<0.0||![gx,gy,gz,lx,ly,lz,hx,hy,hz].iter().all(|v|v.is_finite()&&v.abs()<1e10){return 0;}
 if let Some(c)=slot(id){c.step(dt,[gx,gy,gz],[lx,ly,lz],[hx,hy,hz]);1}else{0}
}
#[no_mangle]pub extern "C" fn cloth_release(id:u32)->u32{if let Some(c)=slot(id){c.pins.fill(0);1}else{0}}
#[no_mangle]pub extern "C" fn cloth_forces(id:u32,gx:f64,gy:f64,gz:f64,lx:f64,ly:f64,lz:f64,hx:f64,hy:f64,hz:f64)->u32{
 if ![gx,gy,gz,lx,ly,lz,hx,hy,hz].iter().all(|v|v.is_finite()&&v.abs()<1e10){return 0;}
 if let Some(c)=slot(id){c.batch_forces=Some([[gx,gy,gz],[lx,ly,lz],[hx,hy,hz]]);1}else{0}
}
#[no_mangle]pub extern "C" fn cloth_step_all(dt:f64)->u32{
 if !dt.is_finite()||dt<0.0{return u32::MAX;}
 unsafe{let all=&mut *addr_of_mut!(CLOTH);if all.iter().flatten().any(|c|c.batch_forces.is_none()){return u32::MAX;}
 let mut count=0;for c in all.iter_mut().flatten(){let[g,l,h]=c.batch_forces.unwrap();c.step(dt,g,l,h);count+=1;}count}
}
#[no_mangle]pub extern "C" fn cloth_seed(id:u32)->u32{slot(id).map_or(0,|c|c.seed)}
#[no_mangle]pub extern "C" fn cloth_steps(id:u32)->u32{slot(id).map_or(0,|c|c.steps)}
#[no_mangle]pub extern "C" fn cloth_word(id:u32)->u32{slot(id).map_or(0,|c|c.word)}
#[no_mangle]pub extern "C" fn cloth_delta(id:u32)->f64{slot(id).map_or(0.0,|c|c.delta)}
#[no_mangle]pub extern "C" fn cloth_half(id:u32)->f64{slot(id).map_or(0.0,|c|c.half)}
#[no_mangle]pub extern "C" fn cloth_wind(id:u32,axis:u32)->f64{slot(id).map_or(0.0,|c|c.wind.get(axis as usize).copied().unwrap_or(0.0)as f64)}
#[no_mangle]pub extern "C" fn cloth_len(id:u32)->u32{slot(id).map_or(0,|c|c.mass.len()as u32)}
macro_rules! ptr_export{($name:ident,$field:ident,$ty:ty)=>{#[no_mangle]pub extern "C" fn $name(id:u32)->*const $ty{slot(id).map_or(core::ptr::null(),|c|c.$field.as_ptr())}};}
ptr_export!(cloth_positions,pos,f32);ptr_export!(cloth_velocity,vel,f32);ptr_export!(cloth_mass,mass,f32);ptr_export!(cloth_inverse,inverse,f32);ptr_export!(cloth_pins,pins,u8);ptr_export!(cloth_colors,colors,f32);ptr_export!(cloth_normals,normals,f32);
#[no_mangle]pub extern "C" fn cloth_light(id:u32,uniform:u32,ar:f64,ag:f64,ab:f64,dr:f64,dg:f64,db:f64,sx:f64,sy:f64,sz:f64)->u32{
 if uniform>1||![ar,ag,ab,dr,dg,db,sx,sy,sz].iter().all(|v|v.is_finite()&&(*v as f32)as f64==*v){return 0;}
 if let Some(c)=slot(id){c.light([ar,ag,ab],[dr,dg,db],[sx,sy,sz],uniform==1);1}else{0}
}
#[no_mangle]pub extern "C" fn math_add(a:f64,b:f64)->f64{math::add(a,b)}
#[no_mangle]pub extern "C" fn math_sub(a:f64,b:f64)->f64{math::sub(a,b)}
#[no_mangle]pub extern "C" fn math_mul(a:f64,b:f64)->f64{math::mul(a,b)}
#[no_mangle]pub extern "C" fn math_div(a:f64,b:f64)->f64{math::div(a,b)}
#[no_mangle]pub extern "C" fn math_sqrt(a:f64)->f64{math::sqrt(a)}
#[no_mangle]pub extern "C" fn math_store32(a:f64)->f64{math::f(a)}
