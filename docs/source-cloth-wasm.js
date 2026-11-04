// Portable, I/O-free adapter for an already instantiated own visual WASM.
// Private visual slots are never simulation entity handles. No global original
// FPU-context, visual scheduler, NativeDeath or save-format parity is implied.
export function createWasmSourceClothFactory(wasm){
 const required=['cloth_abi_revision','input_alloc','pins_alloc','cloth_create','cloth_drop','cloth_step','cloth_light','cloth_positions','cloth_velocity','cloth_mass','cloth_inverse','cloth_pins','cloth_colors','cloth_normals','cloth_len','cloth_seed','cloth_steps','cloth_release','cloth_delta','cloth_half','cloth_word','cloth_wind'];
 if(required.some(name=>typeof wasm?.[name]!=='function')||!wasm.memory?.buffer||wasm.cloth_abi_revision()!==1)throw new TypeError('Own visual cloth WASM ABI1 required');
 const records=new WeakMap();
 const binary32=v=>Number.isFinite(v)&&Math.fround(v)===v;
 const finiteVector=v=>v?.length===3&&Array.from(v).every(x=>Number.isFinite(x)&&Math.abs(x)<1e10);
 const requireMode=rounding=>{if(rounding!=='chop53')throw new RangeError('Own WASM cloth backend requires explicit chop53');};
 function get(cloth){const record=records.get(cloth);if(!record||record.disposed)throw new RangeError('Disposed or foreign private visual cloth');return record;}
 function floats(record,name,multiplier){return new Float32Array(wasm.memory.buffer,wasm[name](record.slot),record.count*multiplier);}
 function create(plane,{rounding}={}){
  requireMode(rounding);const {positions,nx,ny,pinned}=plane||{};
  if(!Number.isInteger(nx)||!Number.isInteger(ny)||nx<2||ny<2||nx>255||ny>255||positions?.length!==nx*ny*3||pinned?.length!==nx*ny||!Array.from(positions).every(v=>binary32(v)&&Math.abs(v)<1e10)||!Array.from(pinned).every(v=>v===0||v===1||v===false||v===true))throw new RangeError('Original local binary32 cloth grid and source pins required');
  // Copy first: caller inputs may themselves be views detached by allocation.
  const pointCopy=Float32Array.from(positions),pinCopy=Uint8Array.from(pinned);
  const pointPtr=wasm.input_alloc(pointCopy.length),pinPtr=wasm.pins_alloc(pinCopy.length);
  if(!pointPtr||!pinPtr)throw new RangeError('Own visual scratch allocation failed');
  new Float32Array(wasm.memory.buffer,pointPtr,pointCopy.length).set(pointCopy);new Uint8Array(wasm.memory.buffer,pinPtr,pinCopy.length).set(pinCopy);
  const slot=wasm.cloth_create(nx,ny)>>>0;if(slot===0xffffffff)throw new RangeError('Own visual cloth creation rejected');
  const record={slot,count:nx*ny,nx,ny,disposed:false};
  const cloth={rounding,
   get positions(){return floats(get(cloth),'cloth_positions',3);},get velocity(){return floats(get(cloth),'cloth_velocity',3);},get mass(){return floats(get(cloth),'cloth_mass',1);},get inverseMass(){return floats(get(cloth),'cloth_inverse',1);},
   get pins(){const r=get(cloth);return new Uint8Array(wasm.memory.buffer,wasm.cloth_pins(r.slot),r.count);},
   get seed(){return wasm.cloth_seed(get(cloth).slot)>>>0;},get steps(){return wasm.cloth_steps(get(cloth).slot)>>>0;},
   releasePins(){wasm.cloth_release(get(cloth).slot);},
   step(delta,{gravity,windMin,windMax}={}){
    const r=get(cloth);if(!Number.isFinite(delta)||delta<0||![gravity,windMin,windMax].every(finiteVector))throw new RangeError('Explicit finite source delta and force vectors required');
    if(!wasm.cloth_step(r.slot,delta,...gravity,...windMin,...windMax))throw new RangeError('Own visual cloth step rejected');
    return{delta:wasm.cloth_delta(r.slot),halfDelta:wasm.cloth_half(r.slot),randomWord:wasm.cloth_word(r.slot)>>>0,wind:Float32Array.from([0,1,2],axis=>wasm.cloth_wind(r.slot,axis)),seed:wasm.cloth_seed(r.slot)>>>0};
   },
   snapshot(){const r=get(cloth);return{format:'source-cloth-kernel-state-v1',nx:r.nx,ny:r.ny,rounding,seed:cloth.seed,steps:cloth.steps,positions:Array.from(cloth.positions),velocity:Array.from(cloth.velocity),pins:Array.from(cloth.pins)};},
  };
  records.set(cloth,record);return Object.freeze(cloth);
 }
 function shade(cloth,plane,lighting,{rounding,mode='normal',colors}={}){
  requireMode(rounding);const r=get(cloth);
  if(plane?.nx!==r.nx||plane?.ny!==r.ny||!['normal','uniform'].includes(mode))throw new RangeError('Source shading grid/quality mismatch');
  const {ambient,diffuse,sunDirection}=lighting||{};
  for(const v of[ambient,diffuse,sunDirection])if(v?.length!==3||!Array.from(v).every(binary32))throw new RangeError('Raw undimmed binary32 source lighting required');
  if(colors!==undefined&&(!(colors instanceof Float32Array)||colors.length!==r.count*4||colors.buffer===wasm.memory.buffer||!colors.every(Number.isFinite)))throw new RangeError('Separate finite color snapshot required');
  if(colors)floats(r,'cloth_colors',4).set(colors);
  if(!wasm.cloth_light(r.slot,mode==='uniform'?1:0,...ambient,...diffuse,...sunDirection))throw new RangeError('Own visual source shading rejected');
  // Stable frame copies survive later owner creation/memory growth and isolate
  // multiple render/reflection/shadow draws from mutable solver buffers.
  const resultColors=colors??new Float32Array(r.count*4);resultColors.set(floats(r,'cloth_colors',4));
  return{positions:floats(r,'cloth_positions',3).slice(),colors:resultColors,localNormals:mode==='normal'?floats(r,'cloth_normals',3).slice():null,sourceFunction:mode==='normal'?'00619FB0':'00619F20',dirtyMask:5,rounding};
 }
 function dispose(cloth){const r=records.get(cloth);if(!r||r.disposed)return false;wasm.cloth_drop(r.slot);r.disposed=true;return true;}
 return Object.freeze({revision:1,backend:'own-wasm-chop53-cloth-fast7',create,shade,dispose});
}
