//! Immutable World8 catalog decoder and height queries.
#![allow(dead_code)]
use super::hash::sha256;
use super::{forest_io::decode_valley_forest, source_forest_grid::SourceForestGrid};

pub const INPUT_MAGIC:u32=0x31493857;
pub(crate) const MAX_INPUT:usize=16*1024*1024;
#[derive(Clone,Debug)]
pub struct NativeTerritory {pub source_id:i32,pub center:[f32;2],pub source_owner:i32}
#[derive(Clone,Debug)]pub struct CliffSurface{pub tile:usize,pub triangles:Vec<[[f32;3];3]>}
#[derive(Clone,Debug)]
pub struct Geometry {
 pub width:usize,pub height:usize,pub source_sha:[u8;32],pub input_sha:[u8;32],
 pub heights:Vec<f32>,pub materials:Vec<u16>,pub tints:Vec<u32>,pub move_palette:Vec<u8>,
 pub patterns:Vec<u32>,pub territory_ids:Vec<i32>,pub territories:Vec<NativeTerritory>,
 pub starts:Vec<([f32;2],i32)>,pub pending_units:u32,pub pending_forest:u32,
 pub source_cliffs:u32,pub source_edges:u32,pub wire:Vec<u8>,
 pub surfaces:Vec<CliffSurface>,
 pub forest:Option<SourceForestGrid>,
 #[cfg(feature = "ee2-valley56-terrain-qa")]
 pub(super) initial_terrain_tree:Option<super::terrain_tree::Tree>,
}
struct Reader<'a>{data:&'a[u8],at:usize}
impl<'a>Reader<'a>{
 fn take(&mut self,n:usize)->Option<&'a[u8]>{let end=self.at.checked_add(n)?;let out=self.data.get(self.at..end)?;self.at=end;Some(out)}
 fn u32(&mut self)->Option<u32>{Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))}
 fn i32(&mut self)->Option<i32>{Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))}
 fn f32(&mut self)->Option<f32>{let v=f32::from_le_bytes(self.take(4)?.try_into().ok()?);v.is_finite().then_some(v)}
}
impl Geometry {
 pub fn decode(data:&[u8])->Option<Self>{
  if data.len()>MAX_INPUT{return None;}
  let input_sha=sha256(data);
  // This first slice accepts the two independently decoded input catalog
  // records. It does not trust caller-authored terrain/DB projections merely
  // because they carry an original source filename/hash.
  if !super::catalog::INPUT_SHA.contains(&input_sha){return None;}
  let mut r=Reader{data,at:0};if r.u32()?!=INPUT_MAGIC||r.u32()?!=2{return None;}
  let width=r.u32()?as usize;let height=r.u32()?as usize;
  if !(1..=256).contains(&width)||!(1..=256).contains(&height){return None;}
  let cells=width.checked_mul(height)?;if cells>65536{return None;}
  let corners=(width+1).checked_mul(height+1)?;let source_len=r.u32()?as usize;let palette_len=r.u32()?as usize;let nterr=r.u32()?as usize;
  if source_len>MAX_INPUT||palette_len==0||palette_len>65536||nterr==0||nterr>31{return None;}
  let pending_units=r.u32()?;let pending_forest=r.u32()?;let nstarts=r.u32()?as usize;let source_cliffs=r.u32()?;let source_edges=r.u32()?;
  let nsurfaces=r.u32()?as usize;if nsurfaces>cells{return None;}
  if !(1..=4).contains(&nstarts){return None;}
  let source_sha=r.take(32)?.try_into().ok()?;
  let mut heights=Vec::with_capacity(corners);for _ in 0..corners{heights.push(r.f32()?);}
  let mut materials=Vec::with_capacity(corners);for _ in 0..corners{let v=u16::from_le_bytes(r.take(2)?.try_into().ok()?);if v as usize>=palette_len{return None;}materials.push(v);}
  let mut tints=Vec::with_capacity(corners);for _ in 0..corners{tints.push(r.u32()?);}
  let move_palette=r.take(palette_len)?.to_vec();if move_palette.iter().any(|&v|v>7){return None;}
  let mut patterns=Vec::with_capacity(cells);for _ in 0..cells{patterns.push(r.u32()?);}
  let mut surfaces=Vec::with_capacity(nsurfaces);
  for _ in 0..nsurfaces{let tile=r.u32()?as usize;let n=r.u32()?as usize;if tile>=cells||n>10000||surfaces.iter().any(|t:&CliffSurface|t.tile==tile){return None;}
   let mut triangles=Vec::with_capacity(n);for _ in 0..n{let mut tri=[[0.0;3];3];for v in &mut tri{for c in v{*c=r.f32()?;}}triangles.push(tri);}surfaces.push(CliffSurface{tile,triangles});
  }
  let mut territory_ids=Vec::with_capacity(cells);for _ in 0..cells{territory_ids.push(r.i32()?);}
  let mut territories=Vec::with_capacity(nterr);
  for _ in 0..nterr{
   let source_id=r.i32()?;let center=[r.f32()?,r.f32()?];let source_owner=r.i32()?;
   if source_id<0||source_owner!=0||territories.iter().any(|t:&NativeTerritory|t.source_id==source_id){return None;}
   territories.push(NativeTerritory{source_id,center,source_owner});
  }
  if territory_ids.iter().any(|&v|v!= -1&&!territories.iter().any(|t|t.source_id==v)){return None;}
  let mut starts=Vec::with_capacity(nstarts);
  for _ in 0..nstarts{let p=[r.f32()?,r.f32()?];let priority=r.i32()?;if p[0]<0.0||p[1]<0.0||p[0]>=width as f32||p[1]>=height as f32{return None;}starts.push((p,priority));}
  let raw=r.take(source_len)?;if sha256(raw)!=source_sha||r.at!=data.len(){return None;}
  let forest=if source_sha==super::VALLEY_SHA{Some(decode_valley_forest(raw,width,height,pending_forest as usize)?)}else{None};
  Some(Self{width,height,source_sha,input_sha,heights,materials,tints,move_palette,patterns,territory_ids,territories,starts,pending_units,pending_forest,source_cliffs,source_edges,surfaces,forest,wire:data.to_vec(),
   #[cfg(feature = "ee2-valley56-terrain-qa")]
   initial_terrain_tree:None,
  })
 }
 pub fn source_territory(&self,compact:u8)->Option<i32>{self.territories.get(compact as usize).map(|t|t.source_id)}
 /// Source0070FC10/00710000 base-height triangle query. This is separate
 /// from006FE4D0's deformed NIF surface query. The existing source oracle uses
 /// binary53/nearest at the biased-index store; global original x87 context
 /// and full actor placement constraints are not asserted by this adapter.
 pub fn base_height(&self,hm:&[f32],x:f32,y:f32)->f32{
  if !x.is_finite()||!y.is_finite()||x<0.0||y<0.0||x>self.width as f32||y>self.height as f32{return f32::NAN;}
  let index=|v:f32|(((v as f64+103079215104.0).to_bits()>>16)as u32)as i32;
  let ix=index(x);let iy=index(y);let fx=x-ix as f32;let fy=y-iy as f32;
  let sample=|dx:i32,dy:i32|hm[dy.clamp(0,self.height as i32)as usize*(self.width+1)+dx.clamp(0,self.width as i32)as usize];
  let a=sample(ix,iy);let b=sample(ix+1,iy);let c=sample(ix,iy+1);let d=sample(ix+1,iy+1);
  let pattern=if ix>=0&&iy>=0&&ix<self.width as i32&&iy<self.height as i32{self.patterns[iy as usize*self.width+ix as usize]}else{0};
  if pattern>>16&255!=0{
   let sum=fx+fy;let diagonal=c-b;
   if sum>1.0{((1.0-fx as f64)*diagonal as f64+(sum as f64-1.0)*(d as f64-b as f64)+b as f64)as f32}
   else{(fy as f64*diagonal as f64+(1.0-sum as f64)*(a as f64-b as f64)+b as f64)as f32}
  }else{let diagonal=d-a;
   if fy<fx{(fy as f64*diagonal as f64+(fx as f64-fy as f64)*(b as f64-a as f64)+a as f64)as f32}
   else{(fx as f64*diagonal as f64+(fy as f64-fx as f64)*(c as f64-a as f64)+a as f64)as f32}
  }
 }
 pub fn surface_height(&self,hm:&[f32],x:f32,y:f32)->f32{
  if !x.is_finite()||!y.is_finite()||x<0.0||y<0.0||x>self.width as f32||y>self.height as f32{return f32::NAN;}
  let index=|v:f32|(((v as f64+103079215104.0).to_bits()>>16)as u32)as i32;
  let ix=index(x);let iy=index(y);
  if ix>=0&&iy>=0&&ix<self.width as i32&&iy<self.height as i32{
   if let Some(surface)=self.surfaces.iter().find(|s|s.tile==iy as usize*self.width+ix as usize){
    for tri in &surface.triangles{if let Some(height)=triangle_height(tri,x,y){return height;}}
   }
  }
  // Source006FE4D0 also falls back to0070FC10 if no enclosing triangle exists.
  self.base_height(hm,x,y)
 }
}
pub fn triangle_height(tri:&[[f32;3];3],x:f32,y:f32)->Option<f32>{
 let[a,b,c]=*tri;let bx=(b[0]-a[0])as f64;let by=(b[1]-a[1])as f64;let cx=(c[0]-a[0])as f64;let cy=(c[1]-a[1])as f64;let px=(x-a[0])as f64;let py=(y-a[1])as f64;
 let mut d=cy*bx-cx*by;let mut wb=px*cy-py*cx;let mut wc=py*bx-px*by;if d==0.0{return None;}
 if d<0.0{d=-d;wb=-wb;wc=-wc;}if wb<0.0||wc<0.0||wb+wc>d{return None;}
 Some((((b[2]as f64-a[2]as f64)*wb+(c[2]as f64-a[2]as f64)*wc)/d+a[2]as f64)as f32)
}
