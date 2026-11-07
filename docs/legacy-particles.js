// Legacy NiParticleSystemController subset used by the installed Priest FX.
// The caller supplies visual time and a presentation-only rand15 stream.
const f32=Math.fround;
const UNIT_RAND=f32(1/32767),BIPOLAR_RAND=f32(2/32767);
const SIZE_FLOOR=f32(.0001),GRAVITY_UNIT=f32(1.6);
const PREWARM_STEP=f32(1/30),PREWARM_EPSILON=f32(.0001);
const IDENTITY=[1,0,0,0,1,0,0,0,1];
const dot=(a,b)=>a.reduce((sum,x,i)=>sum+x*b[i],0);
const add=(a,b)=>a.map((x,i)=>f32(x+b[i]));
const sub=(a,b)=>a.map((x,i)=>f32(x-b[i]));
const scale=(a,n)=>a.map(x=>f32(x*n));
const row=(m,i)=>m.slice(i*3,i*3+3);
const column=(m,i)=>[m[i],m[3+i],m[6+i]];
const transform=(m,p)=>[0,1,2].map(i=>f32(dot(row(m,i),p)));
const inverse=(m,p)=>[0,1,2].map(i=>f32(dot(column(m,i),p)));
const xyz=v=>[v.x,v.y,v.z];
const rgba=v=>[v.r,v.g,v.b,v.a];
const quat=v=>[v.w,v.x,v.y,v.z];
const norm=q=>q.map(x=>x/Math.hypot(...q));

export function createLegacyRand(seed){
 let state=seed>>>0;
 return ()=>{state=(Math.imul(214013,state)+2531011)>>>0;return state>>>16&0x7fff;};
}

function interval(keys,time){
 let index=keys.findIndex(key=>key.Time>=time);
 if(index<0)index=keys.length-1;
 const b=keys[index];
 if(index===0||time>=b.Time)return [b,b,0];
 const a=keys[index-1];
 return [a,b,(time-a.Time)/(b.Time-a.Time)];
}

function keyVector(group,time,components){
 if(!group?.Keys?.length)return null;
 const [a,b,t]=interval(group.Keys,time),first=components(a.Value),last=components(b.Value);
 if(a===b)return first;
 if(group.Interpolation===1)return first.map((x,i)=>f32(x+(last[i]-x)*t));
 if(group.Interpolation!==2)throw Error('Unsupported Priest particle vector interpolation');
 const t2=t*t,t3=t2*t,incoming=components(b.Backward),outgoing=components(a.Forward);
 return first.map((x,i)=>f32((2*t3-3*t2+1)*x+(t3-2*t2+t)*outgoing[i]+(-2*t3+3*t2)*last[i]+(t3-t2)*incoming[i]));
}

function emitterRotation(emitter,time){
 const keys=emitter.emitterKeys['Quaternion Keys'];
 if(!keys?.length)return emitter.emitter.rotation;
 const [a,b,t]=interval(keys,time),first=norm(quat(a.Value));
 let last=norm(quat(b.Value)),angle=dot(first,last);
 if(angle<0){last=last.map(x=>-x);angle=-angle;}
 let q=first;
 if(a!==b){
  if(angle>.9995)q=norm(first.map((x,i)=>x+(last[i]-x)*t));
  else{
   const theta=Math.acos(Math.min(1,angle)),denominator=Math.sin(theta);
   const left=Math.sin((1-t)*theta)/denominator,right=Math.sin(t*theta)/denominator;
   q=first.map((x,i)=>x*left+last[i]*right);
  }
 }
 const [w,x,y,z]=q;
 return [1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w),2*(x*y+z*w),1-2*(x*x+z*z),
  2*(y*z-x*w),2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)].map(f32);
}

function birthRate(c){
 if(c['Use Birth Rate'])return c['Birth Rate'];
 const duration=c['Stop Time']-c['Start Time'];
 const divisor=duration>c.Lifetime?c.Lifetime:c['Emit Stop Time']-c['Emit Start Time'];
 return c['Num Particles']/divisor;
}

function validate(emitter){
 const c=emitter.controller;
 if(c.Frequency!==1||c.Phase!==0||c['Start Time']!==0||c['Stop Time']!==2||c['Reset Particle System']
    ||c['Spawn On Death']||c['Emitter Modifier']!==-1||c['Particle Collider']!==-1||!c['Static Target Bound']
    ||emitter.numActiveInFile!==0||xyz(c['Emitter Dimensions']).some(Boolean)
    ||emitter.parent.scale!==1||emitter.shape.scale!==1||emitter.emitter.scale!==1
    ||emitter.shape.translation.some(Boolean)||emitter.shape.rotation.some((x,i)=>x!==IDENTITY[i])
    ||emitter.emitterKeys.Scales['Num Keys']!==0
    ||emitter.emitterKeys['Quaternion Keys']?.some(key=>key.TBC?.t||key.TBC?.b||key.TBC?.c)
    ||emitter.modifiers.some(m=>!['NiParticleColorModifier','NiParticleGrowFade','NiGravity'].includes(m.class)
      ||m.class==='NiGravity'&&m.Type!==0))throw Error('Unsupported original Priest particle descriptor');
 return c;
}

export function createLegacyEmitter(emitter,rand15){
 const c=validate(emitter),rate=birthRate(c);
 const start=c['Emit Start Time'],stop=c['Emit Stop Time'],life=c.Lifetime;
 const parent=emitter.parent,capacity=c['Num Particles']-1;
 const active=[];
 let previous=0,cursor=0,births=0,draws=0,fresh=true;
 const rand=()=>{draws++;return rand15();};

 function updateParticle(p,time,delta){
  for(const modifier of emitter.modifiers){
   switch(modifier.class){
    case 'NiGravity': {
     const d=xyz(modifier.Direction),from=sub(xyz(modifier.Position),p.position);
     const attenuation=modifier.Decay?Math.exp(-modifier.Decay*Math.abs(dot(from,d))):1;
     const acceleration=scale(scale(d,attenuation),modifier.Force);
     p.velocity=add(p.velocity,scale(scale(acceleration,GRAVITY_UNIT),f32(time-p.updated)));
     break;
    }
    case 'NiParticleColorModifier':
     p.color=keyVector(modifier.keys.Data,f32(p.age/p.life),rgba).map(x=>Math.max(0,Math.min(1,x)));
     break;
    case 'NiParticleGrowFade': {
     const remaining=f32(p.life-p.age);
     const grow=p.age<modifier.Grow?p.age/modifier.Grow:1;
     const fade=remaining<modifier.Fade?remaining/modifier.Fade:1;
     p.radiusFactor=f32(Math.max(SIZE_FLOOR,Math.min(grow,fade)));
     break;
    }
   }
  }
  p.position=p.position.map((x,i)=>f32(x+p.velocity[i]*delta));
  p.updated=time;
 }

 function spawn(time,origin,rotation){
  // 009C3131 consumes all eight draws even for zero author variations.
  const age=f32(rand()*(time-cursor)*UNIT_RAND);
  const speed=f32(c.Speed+(rand()*UNIT_RAND-.5)*c['Speed Variation']);
  const decl=f32(c.Declination+(rand()*BIPOLAR_RAND-1)*c['Declination Variation']);
  const plane=c['Planar Angle']+(rand()*BIPOLAR_RAND-1)*c['Planar Angle Variation'];
  const dir=[f32(Math.sin(decl)*Math.cos(plane))*speed,Math.sin(decl)*Math.sin(plane)*speed,Math.cos(decl)*speed].map(f32);
  const lifetime=f32(life+(rand()*UNIT_RAND-.5)*c['Lifetime Variation']);
  const dimY=rand(),dimX=rand(),dimZ=rand();
  const offset=[dimX,dimY,dimZ].map((r,i)=>f32((r*UNIT_RAND-.5)*xyz(c['Emitter Dimensions'])[i]));
  const local=p=>inverse(parent.rotation,transform(rotation,p));
  const p={age,life:lifetime,updated:f32(time-age),position:add(origin,local(offset)),
   velocity:local(dir),color:rgba(c['Initial Color']),radiusFactor:1};
  births++;
  if(age>lifetime)return;
  updateParticle(p,time,age);
  active.push(p);
 }

 function tick(time,poseAt){
   const dt=f32(time-previous);
   for(let i=0;i<active.length;){
    const p=active[i];p.age=f32(p.age+dt);
    if(p.age>p.life){active[i]=active.at(-1);active.pop();continue;}
    updateParticle(p,time,dt);i++;
   }
   const from=Math.max(start,cursor),to=Math.min(stop,time);
   let next=time;
   if(time>=start&&to>from){
    const requested=Math.trunc((to-from)*rate);
    next=f32(cursor+requested/rate);
    if(requested&&active.length<capacity){
     const keys=emitter.emitterKeys;
     const center=keyVector(keys.Translations,poseAt,xyz)??emitter.emitter.translation;
     const origin=inverse(parent.rotation,sub(center,parent.translation));
     const rotation=emitterRotation(emitter,poseAt);
     const count=Math.min(requested,capacity-active.length);
     for(let i=0;i<count;i++)spawn(time,origin,rotation);
    }
   }
   cursor=next;previous=time;
 }

 return {
  step(at){
   const time=f32(at);
   if(!Number.isFinite(time)||time<previous||time>2)throw RangeError('Nonmonotonic original Priest particle time');
   if(time===previous)return;
   if(fresh){
    // 009C1F10 marks the empty instance for the first controller update.
    // Its first lower advance resets both time cursors before creating births.
    if(time-start>PREWARM_EPSILON){
     previous=cursor=start;
     for(let warm=start;time-warm>PREWARM_EPSILON;warm=f32(warm+PREWARM_STEP))tick(warm,time);
    }else previous=cursor=time;
    fresh=false;
   }
   tick(time,time);
  },
  snapshot(){
   return active.map(p=>({position:add(parent.translation,transform(parent.rotation,p.position)),
    radius:f32(c['Initial Size']*p.radiusFactor),color:p.color.slice(),age:p.age,life:p.life}));
  },
  stats(){return {births,draws,active:active.length,cursor,previous};},
 };
}
