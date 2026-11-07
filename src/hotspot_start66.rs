//! Existing source placement kernels, moved from the isolated World8 proof.
use crate::hotspot_rule66::{add_chop, mul_chop};

/// Raw positions from 00706BAF and 00707070 in the default inline PC53/chop
/// context. The scalar rounding follows the original stores. Portable libm
/// evaluates the final trigonometric pair before the directed XY stores.
pub(crate) fn generated_origins(width: usize, height: usize, seed: u32, count: u32) -> Option<Vec<[f32; 2]>> {
    if !(1..=4).contains(&count) || !(8..=32767).contains(&width) || !(8..=32767).contains(&height) {
        return None;
    }
    let dimensions = [width as f64, height as f64];
    let center = dimensions.map(|size| size * 0.5);
    let mut points = Vec::with_capacity(count as usize);
    points.push(center.map(|v| v as f32));
    if count == 1 { return Some(points); }
    let radius = dimensions.map(|size| {
        let half = mul_chop(size, 0.666).trunc() * 0.5;
        add_chop(half, -mul_chop(half, f64::from(0.2f32)).max(12.0)).max(3.0)
    });
    let mut random = seed;
    let mut draw = || {
        random = random.wrapping_mul(1664525).wrapping_add(1013904223);
        f64::from((random >> 16) & 0x7fff)
    };
    let scale = f64::from(f32::from_bits(0x3949_116d));
    let phase = f64::from(store_chop(mul_chop(draw(), scale)));
    let step = f64::from(f32::from_bits(0x40c9_0fdb));
    let extra = count - 1;
    for index in 0..extra {
        let offset = mul_chop(draw() * 0.5, scale);
        let numerator = add_chop(add_chop(mul_chop(f64::from(index), step), offset), phase);
        let divisor = f64::from(extra);
        let quotient = numerator / divisor;
        let angle = if quotient.mul_add(divisor, -numerator) > 0.0 {
            f64::from_bits(quotient.to_bits() - 1)
        } else { quotient };
        let (sin, cos) = angle.sin_cos();
        points.push(std::array::from_fn(|axis| {
            let direction = [cos, sin][axis];
            store_chop(add_chop(mul_chop(direction, radius[axis]), center[axis]))
        }));
    }
    Some(points)
}

fn store_chop(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest).abs() > value.abs() { f32::from_bits(nearest.to_bits() - 1) } else { nearest }
}
/// Original00870160's 4x4 fractional-cell table at00C6BB30. The source
/// multiplies fractions by-4, truncates, then subtracts the negative index
/// from the base address. These are footprint-corner offsets, not own tiles.
pub const SNAP_OFFSETS:[[f32;2];16]=[
 [0.0,0.0],[0.0,0.0],[1.0,0.0],[1.0,0.0],
 [0.0,0.0],[0.5,0.5],[0.5,0.5],[1.0,0.0],
 [0.0,1.0],[0.5,0.5],[0.5,0.5],[1.0,1.0],
 [0.0,1.0],[0.0,1.0],[1.0,1.0],[1.0,1.0],
];
/// XY footprint snap from00870160. Explicit source FSINCOS f32 stores avoid
/// a Rust/libm trigonometric equivalence claim. Coordinates outside the proven
/// signed16 map domain are rejected, not clamped or given a guessed snap.
pub fn snap_from_basis(origin:[f32;2],size:[f32;2],cos_sin:[f32;2])->Option<[f32;2]>{
 if origin.iter().chain(size.iter()).chain(cos_sin.iter()).any(|v|!v.is_finite())||size.iter().any(|&v|v<0.0){return None;}
 let[c,s]=cos_sin;let hx=size[0]as f64*0.5;let hy=size[1]as f64*0.5;
 // Unlike00715930, this routine keeps the half sizes on the x87 stack,
 // then explicitly stores the rotated half-corner and translated corner.
 let rotated=[(hx*c as f64-hy*s as f64)as f32,(hx*s as f64+hy*c as f64)as f32];
 let p=[origin[0]+rotated[0],origin[1]+rotated[1]];
 if p.iter().any(|v|!v.is_finite()||v.abs()>32767.0){return None;}
 let floor=p.map(|v|(((v as f64+103079215104.0).to_bits()>>16)as u32)as i32);
 let bins=[((p[0]as f64-floor[0]as f64)*4.0).trunc()as i32,((p[1]as f64-floor[1]as f64)*4.0).trunc()as i32];
 if bins.iter().any(|&v|!(0..4).contains(&v)){return None;}
 let offset=SNAP_OFFSETS[(bins[0]+bins[1]*4)as usize];
 let out=[(floor[0]as f64+offset[0]as f64-rotated[0]as f64)as f32,(floor[1]as f64+offset[1]as f64-rotated[1]as f64)as f32];
 out.iter().all(|v|v.is_finite()).then_some(out)
}
/// Candidate-order kernel from0082AF60. Height/snap/placement/flatten checks
/// remain in the source world context; a candidate is never itself a spawn.
/// Source00A20760 initializes00D0D8A0/A4 to(1,0), not a guessed random axis.
#[derive(Debug,Clone)]pub struct RingCandidates{origin:[f32;2],radius:f32,ring:u32,angle:f32,rings:u32,invalid:bool}
impl RingCandidates{
 pub fn new(origin:[f32;2],radius:f32,width:u32,height:u32)->Option<Self>{
  if width==0||height==0||width>32767||height>32767||origin.iter().any(|v|!v.is_finite())||!radius.is_finite()||radius<=0.0{return None;}
  // 0082B02A..6C: sqrt(worldWidth²+worldHeight²)/f32Radius,
  // biased-double integer extraction, then +1. No radius fallback.
  let distance=((width as f64).powi(2)+(height as f64).powi(2)).sqrt()/radius as f64;
  if !distance.is_finite()||distance>65534.0{return None;}
  let rings=(((distance+103079215104.0).to_bits()>>16)as u32).checked_add(1)?;
  Some(Self{origin,radius,ring:0,angle:0.0,rings,invalid:false})
 }
 pub fn next_with(&mut self,mut source_trig:impl FnMut(f32)->[f32;2])->Option<[f32;2]>{
  if self.invalid{return None;}
  let tau=f32::from_bits(0x40c90fdb);
  if self.angle>=tau{self.ring+=1;self.angle=0.0;}
  if self.ring>=self.rings{return None;}
  let [cos,sin]=source_trig(self.angle);
  if !cos.is_finite()||!sin.is_finite(){self.invalid=true;return None;}
  let distance=self.ring as f32*self.radius;
  //00A20800 has f32 trig/output stores before0082B1D0 adds origin.
  let rotated=[(cos as f64*distance as f64)as f32,(sin as f64*distance as f64)as f32];
  let point=[self.origin[0]+rotated[0],self.origin[1]+rotated[1]];
  let step=if self.ring==0{687.0}else{1.0/self.ring as f32};self.angle+=step;
  Some(point)
 }
}
/// 00715930 half-axis rotation and four corner stores, using the actual f32
/// cos/sin stores supplied by00A20800. Supplying a basis avoids asserting that
/// Rust/libm reproduces the original x87 FSINCOS in every FP context.
pub fn quad_from_basis(center:[f32;2],size:[f32;2],cos_sin:[f32;2])->Option<[[f32;2];4]>{
 if center.iter().chain(size.iter()).chain(cos_sin.iter()).any(|v|!v.is_finite())||size.iter().any(|&v|v<0.0){return None;}
 let half=[size[0]*0.5,size[1]*0.5];let[c,s]=cos_sin;
 let a=[(c as f64*half[0]as f64)as f32,(s as f64*half[0]as f64)as f32];
 let b=[(-s as f64*half[1]as f64)as f32,(c as f64*half[1]as f64)as f32];
 let out=[[1.0,1.0],[-1.0,1.0],[-1.0,-1.0],[1.0,-1.0]].map(|[sa,sb]|[
  (center[0]as f64+sa*a[0]as f64+sb*b[0]as f64)as f32,
  (center[1]as f64+sa*a[1]as f64+sb*b[1]as f64)as f32]);
 out.iter().flatten().all(|v|v.is_finite()).then_some(out)
}
/// 00A20840 stores each difference to f32, then tests strict per-axis bounds.
pub fn near_vertex(point:[f32;2],vertex:[f32;2])->bool{
 let threshold=f32::from_bits(0x3d4ccccd);(point[0]-vertex[0]).abs()<threshold&&(point[1]-vertex[1]).abs()<threshold
}
#[derive(Debug,Clone,PartialEq)]pub struct Raster{pub corners:Vec<[i16;2]>,pub contains_outside_corner:bool}
/// Integer-corner loop of007155C0 after its oriented quad is supplied.
/// Bounds use world dimensions, not corner-buffer dimensions; the endpoint
/// at exactly worldWidth/worldHeight is outside. Row-major order is retained.
pub fn raster_quad(quad:[[f32;2];4],world_width:u32,world_height:u32,skip_vertices:bool)->Option<Raster>{
 if world_width==0||world_height==0||world_width>32767||world_height>32767||quad.iter().flatten().any(|v|!v.is_finite()){return None;}
 let min_x=quad.iter().map(|v|v[0]).fold(f32::MAX,f32::min);let min_y=quad.iter().map(|v|v[1]).fold(f32::MAX,f32::min);
 let max_x=quad.iter().map(|v|v[0]).fold(-f32::MAX,f32::max);let max_y=quad.iter().map(|v|v[1]).fold(-f32::MAX,f32::max);
 let bounds=[min_x as f64,min_y as f64,max_x as f64+1.0,max_y as f64+1.0];
 if bounds.iter().any(|&v|v<i32::MIN as f64||v>i32::MAX as f64){return None;}
 let[x0,y0,x1,y1]=bounds.map(|v|v.trunc()as i32);
 let count=(x1 as i64-x0 as i64+1).checked_mul(y1 as i64-y0 as i64+1)?;if !(0..=4_000_000).contains(&count){return None;}
 let mut out=Raster{corners:Vec::new(),contains_outside_corner:false};
 for y in y0..=y1{for x in x0..=x1{let p=[x as f32,y as f32];
  if skip_vertices&&quad.iter().any(|&v|near_vertex(p,v)){continue;}
  let mut inside=true;
  for edge in 0..4{let a=quad[edge];let b=quad[(edge+1)%4];
   // First edge is kept on the source x87 stack; the other three use
   // explicit Vec2 subtraction stores before00716DC0 cross product.
   let(px,py,ex,ey)=if edge==0{(p[0]as f64-a[0]as f64,p[1]as f64-a[1]as f64,b[0]as f64-a[0]as f64,b[1]as f64-a[1]as f64)}else{
    ((p[0]-a[0])as f64,(p[1]-a[1])as f64,(b[0]-a[0])as f64,(b[1]-a[1])as f64)};
   if px*ey-py*ex>0.0{inside=false;break;}
  }
  if inside{if x>=0&&y>=0&&(x as u32)<world_width&&(y as u32)<world_height{out.corners.push([x as i16,y as i16]);}else{out.contains_outside_corner=true;}}
 }}Some(out)
}
