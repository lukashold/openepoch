use crate::math::bounded::{f,add as a,sub as s,mul as m,div as d,sqrt as q};
#[inline]fn ff(v:f64)->f32{f(v)as f32}
#[inline]fn length(x:f64,y:f64,z:f64)->f64{q(a(a(m(x,x),m(y,y)),m(z,z)))}
#[derive(Clone)]struct Spring{a:usize,b:usize,rest:f32}
pub struct Cloth{
 pub nx:usize,pub ny:usize,pub pos:Vec<f32>,pub vel:Vec<f32>,force:Vec<f32>,pub mass:Vec<f32>,pub inverse:Vec<f32>,pub pins:Vec<u8>,links:Vec<Spring>,
 pub seed:u32,pub steps:u32,pub delta:f64,pub half:f64,pub word:u32,pub wind:[f32;3],pub colors:Vec<f32>,pub normals:Vec<f32>,
 pub batch_forces:Option<[[f64;3];3]>,
}
impl Cloth{
 pub fn new(nx:usize,ny:usize,pos:Vec<f32>,pins:Vec<u8>)->Self{
  let n=nx*ny;let mut out=Self{nx,ny,pos,vel:vec![0.0;n*3],force:vec![0.0;n*3],mass:vec![0.0;n],inverse:vec![0.0;n],pins,links:Vec::new(),seed:12345,steps:0,delta:0.0,half:0.0,word:0,wind:[0.0;3],colors:vec![1.0;n*4],normals:vec![0.0;n*3],batch_forces:None};
  let triangle=f(d(100.0,(2*(nx-1)*(ny-1))as f64));
  for y in 0..ny{for x in 0..nx{
   let i=y*nx+x;
   let weight=if y==0&&x==0{1.0}else if y==ny&&x==0||y==0&&x==nx{2.0}else if y==ny&&x==nx{1.0}else if(y==0||y==ny)&&x!=0&&x!=nx{3.0}else{6.0};
   out.mass[i]=ff(m(m(weight,triangle),0.3333333432674408));out.inverse[i]=ff(d(1.0,out.mass[i]as f64));
   let mut append=|j:usize|{let v:Vec<f64>=(0..3).map(|axis|f(s(out.pos[i*3+axis]as f64,out.pos[j*3+axis]as f64))).collect();out.links.push(Spring{a:i*3,b:j*3,rest:ff(length(v[0],v[1],v[2]))});};
   if x<nx-1{append(i+1);}if y<ny-1{append(i+nx);}if x<nx-1&&y<ny-1{append(i+nx+1);}if x>0&&y<ny-1{append(i+nx-1);}
  }}out
 }
 fn substep(&mut self,dt:f64,gravity:[f64;3],wind:[f32;3]){
  for link in &self.links{
   let ia=link.a;let ib=link.b;
   let mut x=f(s(self.pos[ia]as f64,self.pos[ib]as f64));let mut y=f(s(self.pos[ia+1]as f64,self.pos[ib+1]as f64));let mut z=f(s(self.pos[ia+2]as f64,self.pos[ib+2]as f64));
   let len=length(x,y,z);if len==0.0{continue;}let inv=d(1.0,len);x=f(m(x,inv));y=f(m(y,inv));z=f(m(z,inv));
   let magnitude=-m(s(len,link.rest as f64),300.0);let fy=f(m(y,magnitude));let fz=f(m(z,magnitude));
   self.force[ia]=ff(a(m(x,magnitude),self.force[ia]as f64));self.force[ia+1]=ff(a(fy,self.force[ia+1]as f64));self.force[ia+2]=ff(a(fz,self.force[ia+2]as f64));
   self.force[ib]=ff(s(self.force[ib]as f64,m(x,magnitude)));self.force[ib+1]=ff(s(self.force[ib+1]as f64,fy));self.force[ib+2]=ff(s(self.force[ib+2]as f64,fz));
  }
  for i in 0..self.mass.len(){let at=i*3;
   if self.pins[i]==0{
    let gm=m(-32.17399978637695,self.mass[i]as f64);
    for axis in 0..3{self.force[at+axis]=ff(a(self.force[at+axis]as f64,f(m(gm,gravity[axis]))));}
    let mut x=-(self.vel[at]as f64);let mut y=-(self.vel[at+1]as f64);let mut z=-(self.vel[at+2]as f64);let speed=length(x,y,z);
    if speed>0.0{let inv=d(1.0,speed);x=f(m(x,inv));y=f(m(y,inv));z=f(m(z,inv));}
    let drag=m(m(speed,0.9900000095367432),speed);
    for(axis,normal)in[x,y,z].into_iter().enumerate(){self.force[at+axis]=ff(a(self.force[at+axis]as f64,f(m(normal,drag))));}
    for axis in 0..3{
     self.force[at+axis]=ff(a(self.force[at+axis]as f64,wind[axis]as f64));
     let acceleration=f(m(self.inverse[i]as f64,self.force[at+axis]as f64));let change=f(m(acceleration,dt));
     self.vel[at+axis]=ff(a(change,self.vel[at+axis]as f64));let distance=f(m(self.vel[at+axis]as f64,dt));self.pos[at+axis]=ff(a(distance,self.pos[at+axis]as f64));
    }
   }self.force[at]=0.0;self.force[at+1]=0.0;self.force[at+2]=0.0;
  }
 }
 pub fn step(&mut self,delta:f64,gravity:[f64;3],low:[f64;3],high:[f64;3]){
  let delta=f(delta).min(0.05000000074505806);let half=f(m(delta,0.5));
  for _ in 0..3{self.seed=self.seed.wrapping_mul(1664525).wrapping_add(1013904223);}self.word=(self.seed>>16)&32767;
  let mut wind=[0.0;3];for axis in 0..3{let lo=f(m(f(low[axis]),0.5));let hi=f(m(f(high[axis]),0.5));wind[axis]=ff(a(m(m(s(hi,lo),self.word as f64),0.000030518509447574615),lo));}
  let g=gravity.map(f);self.substep(half,g,wind);self.substep(half,g,wind);self.steps=self.steps.wrapping_add(1);self.delta=delta;self.half=half;self.wind=wind;
 }
 fn point(&self,i:usize)->[f64;3]{core::array::from_fn(|axis|self.pos[i*3+axis]as f64)}
 pub fn light(&mut self,ambient:[f64;3],diffuse:[f64;3],sun:[f64;3],uniform:bool){
  let n=self.nx*self.ny;
  for i in 0..n{
   if uniform{for axis in 0..3{self.colors[i*4+axis]=ff(a(ambient[axis],diffuse[axis]));}continue;}
   let y=i/self.nx;let x=i%self.nx;let corner=x==self.nx-1;let bottom=y==self.ny-1;
   let shared=self.point(if corner{i-1}else{i+1});let p=self.point(if bottom{i-self.nx}else{i+self.nx});let here=self.point(i);
   let mut va=[0.0;3];let mut vb=[0.0;3];
   for axis in 0..3{va[axis]=s(shared[axis],p[axis]);vb[axis]=s(shared[axis],here[axis]);
    if !bottom||if corner{axis<2}else{axis>0}{va[axis]=f(va[axis]);}if !bottom{vb[axis]=f(vb[axis]);}}
   let mut normal=[f(s(m(va[1],vb[2]),m(va[2],vb[1]))),f(s(m(va[2],vb[0]),m(va[0],vb[2]))),f(s(m(va[0],vb[1]),m(va[1],vb[0])))];
   let len=length(normal[0],normal[1],normal[2]);if len>0.0{let inv=d(1.0,len);normal=normal.map(|v|f(m(v,inv)));}
   for axis in 0..3{self.normals[i*3+axis]=normal[axis]as f32;}
   let order=if bottom{[2,1,0]}else{[2,0,1]};let dot=a(a(m(normal[order[0]],sun[order[0]]),m(normal[order[1]],sun[order[1]])),m(normal[order[2]],sun[order[2]])).abs();
   for axis in 0..3{self.colors[i*4+axis]=ff(a(m(dot,diffuse[axis]),ambient[axis]));}
  }
 }
}
