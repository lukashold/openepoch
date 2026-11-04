// Independent checks of exported original geometry, texture payloads and animation
// buffers. These data checks do not replace visual skinning/animation comparison.
import assert from 'node:assert/strict';
import {readFileSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {inflateSync} from 'node:zlib';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';

const docs=new URL('../docs/',import.meta.url),docsPath=fileURLToPath(docs);
const manifest=JSON.parse(readFileSync(new URL('local-original/models/manifest.json',docs)));
assert.equal(manifest.format,'ee2-original-model-manifest-v1');
assert.ok(Array.isArray(manifest.models)&&manifest.models.length>=18,'at least the complete initial building/prop/citizen set');
assert.deepEqual(manifest.errors||[],[],'export must not hide failures');
const required=[...Array.from({length:57},(_,id)=>id),...Array.from({length:11},(_,i)=>100+i)];
for(const id of required)assert.ok(manifest.models.some(row=>row.id===id),`required original model ${id}`);
const fortresses=manifest.models.filter(row=>row.id===56);
assert.deepEqual(fortresses.map(row=>row.epoch),[0,3,5,7,9,11,13],'seven actual Fortress source visual stages');
assert.equal(fortresses.filter(row=>row.lodFarUrl).length,3,'only genuinely reducing native Fortress LODs');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const localFile=url=>{
 assert.equal(typeof url,'string');assert.ok(url.startsWith('local-original/'),`local original URL: ${url}`);
 const resolved=new URL(url,docs),filename=fileURLToPath(resolved),relative=path.relative(docsPath,filename);
 assert.ok(relative&&!relative.startsWith('..')&&!path.isAbsolute(relative),`asset remains inside docs/: ${url}`);
 assert.ok(existsSync(filename),`asset exists: ${url}`);return filename;
};
function finite(values,width,label){
 assert.ok(Array.isArray(values)&&values.length>0,`${label}: nonempty array`);
 assert.equal(values.length%width,0,`${label}: aligned width ${width}`);
 for(const n of values)assert.ok(Number.isFinite(n),`${label}: finite numeric value`);
}
function normalVectors(values,label){
 finite(values,3,label);
 for(let i=0;i<values.length;i+=3){const length=Math.hypot(values[i],values[i+1],values[i+2]);assert.ok(Math.abs(length-1)<.002,`${label}: normalized normal ${i/3} (${length})`);}
}
const crcTable=Uint32Array.from({length:256},(_,n)=>{let c=n;for(let j=0;j<8;j++)c=c&1?0xedb88320^(c>>>1):c>>>1;return c>>>0;});
function crc32(bytes){let c=0xffffffff;for(const b of bytes)c=crcTable[(c^b)&255]^(c>>>8);return(c^0xffffffff)>>>0;}
const textureFiles=new Map();
function verifyPng(url,metadata){
 if(textureFiles.has(url))return;
 const bytes=readFileSync(localFile(url));assert.equal(hash(bytes),metadata.png_sha256,`${url}: converted PNG hash`);
 assert.equal(bytes.subarray(0,8).toString('hex'),'89504e470d0a1a0a',`${url}: PNG signature`);
 let offset=8,width=0,height=0,depth=0,type=0,interlace=0,ended=false;const idat=[];
 while(offset<bytes.length){
  const size=bytes.readUInt32BE(offset),end=offset+12+size;assert.ok(end<=bytes.length,`${url}: complete PNG chunk`);
  const kind=bytes.toString('ascii',offset+4,offset+8),payload=bytes.subarray(offset+8,offset+8+size);
  assert.equal(crc32(bytes.subarray(offset+4,offset+8+size)),bytes.readUInt32BE(offset+8+size),`${url}: ${kind} CRC`);
  if(kind==='IHDR'){assert.equal(size,13);width=payload.readUInt32BE(0);height=payload.readUInt32BE(4);depth=payload[8];type=payload[9];interlace=payload[12];}
  if(kind==='IDAT')idat.push(payload);
  if(kind==='IEND'){ended=true;assert.equal(end,bytes.length,`${url}: no trailing corrupt payload`);break;}
  offset=end;
 }
 assert.ok(ended&&idat.length>0,`${url}: complete image payload`);
 assert.equal(width,metadata.width,`${url}: original top-mip width`);assert.equal(height,metadata.height,`${url}: original top-mip height`);
 assert.equal(depth,8,`${url}: 8-bit channels`);assert.equal(type,6,`${url}: RGBA pixels`);assert.equal(interlace,0);
 const pixels=inflateSync(Buffer.concat(idat)),rowBytes=width*4+1;
 assert.equal(pixels.length,rowBytes*height,`${url}: decoded pixel payload`);
 for(let row=0;row<height;row++)assert.ok(pixels[row*rowBytes]<=4,`${url}: valid PNG row filter`);
 textureFiles.set(url,{width,height});
}
function bounds(parts){
 const low=[Infinity,Infinity,Infinity],high=[-Infinity,-Infinity,-Infinity];
 for(const p of parts)for(let i=0;i<p.positions.length;i++){const axis=i%3;low[axis]=Math.min(low[axis],p.positions[i]);high[axis]=Math.max(high[axis],p.positions[i]);}
 return{low,high};
}
const sourceRecords=new Map(),models=new Map(),bindings=new Set();
function source(archive,entry,sha256,label){
 assert.equal(path.basename(archive),archive,`${label}: archive basename`);
 assert.ok(typeof entry==='string'&&entry.length>0,`${label}: original archive entry`);
 assert.match(sha256,/^[a-f0-9]{64}$/i,`${label}: SHA256`);
 const key=archive+'!'+entry,prior=sourceRecords.get(key);
 if(prior)assert.equal(prior.sha256,sha256,`${label}: consistent duplicate source hash`);
 sourceRecords.set(key,{archive,entry,sha256});
}
let partsTotal=0,verticesTotal=0,trianglesTotal=0,lods=0,animationFrames=0,animatedParts=0;
const epochs=new Set();
const resources={102:[5,'Iron',4],103:[0,'Food',1],104:[2,'Stone',1],105:[3,'Gold',1],106:[4,'Tin',1],107:[6,'Saltpeter',7],108:[7,'Oil',10],109:[8,'Uranium',13]};
for(const row of manifest.models){
 assert.ok(Number.isInteger(row.id)&&row.id>=0&&row.id<240,'valid renderer model ID');
 assert.ok(Number.isInteger(row.epoch)&&row.epoch>=0&&row.epoch<15,'valid epoch index');
 const binding=row.id+':'+row.epoch;assert.ok(!bindings.has(binding),`unique id/epoch binding ${binding}`);bindings.add(binding);epochs.add(row.epoch);
 const filename=localFile(row.url),model=models.get(filename)||JSON.parse(readFileSync(filename));models.set(filename,model);
 assert.equal(model.format,'ee2-original-mesh-v1');assert.deepEqual(model.source,row.source,`${row.url}: manifest/source agreement`);
 source(model.source.archive,model.source.entry,model.source.sha256,row.url);
 if(row.id===53||row.id===54||row.id>=102&&row.id<=110){
  const visual=row.resourceVisualBinding;
  assert.ok(visual,`${row.url}: declared original resource visual binding`);
  assert.deepEqual(model.resourceVisualBinding,visual,`${row.url}: matching resource source audit`);
  source(visual.database.archive,visual.database.entry,visual.database.sha256,row.url+' resource database');
  assert.equal(visual.database.archive,'db.zip');assert.equal(visual.database.entry,'db/Units/resources.ddf');
  const originalModel=path.basename(model.source.entry).replace(/\.nifcache$/i,'.nif').toLowerCase();
  assert.ok(visual.declaredModelChoices.some(name=>name.toLowerCase()===originalModel),`${row.url}: model is actually declared by the database visual`);
  if(resources[row.id]){
   const [id,name,epoch]=resources[row.id];
   assert.equal(visual.originalResourceId,id);assert.equal(visual.resourceType,name);assert.equal(visual.revealedOriginalEpoch,epoch);
   assert.equal(visual.resourceEnumSource.symbol,'kResourceType_'+name);
   source(visual.resourceEnumSource.archive,visual.resourceEnumSource.entry,visual.resourceEnumSource.sha256,row.url+' resource enum');
   assert.equal(visual.maxDirectWorkers,row.id===108||row.id===109?0:6,'original Oil/Uranium deposits require extraction buildings');
  }else if(row.id===110){
   assert.equal(visual.unitType,'ResourceDummy');assert.equal(visual.originalResourceId,undefined,'unknown rock has no invented resource identity');
  }else assert.equal(visual.unitType,row.id===53?'OilRig':'UraniumMine');
 }
 if(row.id===23){
  assert.equal(path.basename(model.source.entry).toLowerCase(),'llm11_guntank.nifcache','Tank/E11 follows the original LightMounted1_11 visual');
  assert.equal(row.roleBinding?.unitModel,'LightMounted1_11');
  for(const clip of ['idle','walk'])assert.ok(model.animationSources.some(animation=>animation.name===clip),'gun tank has source '+clip+' KF');
 }
 if(row.id===55){
  const visual=row.databaseVisualBinding;
  assert.deepEqual(model.databaseVisualBinding,visual,'Herc facility keeps its original building proof');
  assert.equal(visual.unitType,'HercFacility');assert.equal(visual.originalEpoch,14);assert.equal(row.epoch,13);
  assert.equal(path.basename(model.source.entry).toLowerCase(),'bld_herc_factory.nifcache','Herc facility is the declared native building, not a fabricated early tank factory');
  assert.equal(visual.database.entry,'db/Units/HERC_Facility.ddf');
  for(const record of [visual.database,visual.upgradeSource])source(record.archive,record.entry,record.sha256,'Herc facility source');
 }
 if(row.id===56){
  const visual=row.databaseVisualBinding;
  assert.deepEqual(model.databaseVisualBinding,visual,'Fortress source proof survives model serialization');
  assert.equal(visual.unitType,'Fortress');assert.equal(visual.originalEpoch,row.epoch+1);
  assert.equal(visual.database.entry,'db/Units/fortress.ddf');
  const actual=path.basename(model.source.entry).replace(/\.nifcache$/i,'').replace(/_we$/i,'').toLowerCase();
  assert.ok(visual.declaredModelChoices.some(name=>path.basename(name).replace(/\.nif$/i,'').toLowerCase()===actual),
   'regional cache has the exact declared model stem');
  for(const record of [visual.database,visual.upgradeSource])source(record.archive,record.entry,record.sha256,'Fortress source');
  if(row.epoch===13){
   assert.equal(visual.unboundAnimationSources.length,4,'real unbound door clips are kept as evidence');
   for(const record of visual.unboundAnimationSources)source(record.archive,record.entry,record.sha256,'unbound Fortress KF evidence');
   assert.ok(!(model.animationSources||[]).length,'no invented idle/attack assignment for unbound door clips');
  }
 }
 assert.ok(Array.isArray(model.parts)&&model.parts.length>0,`${row.url}: visible mesh parts`);
 const texByUrl=new Map((model.sourceTextures||[]).map(texture=>[texture.url,texture]));
 for(const texture of texByUrl.values()){
  verifyPng(texture.url,texture);
  source(texture.archive||'textures.zip',texture.archive_member,texture.source_sha256,texture.url);
  assert.ok(texture.mipmaps_in_source>=1,`${texture.url}: original mip chain exists`);
 }
 for(const lod of model.lodSelection||[]){
  assert.ok(Number.isInteger(lod.children)&&lod.children>=1,`${row.url}: LOD child count`);
  assert.equal(lod.selected,0,`${row.url}: nearest/highest LOD chosen, not combined LODs`);lods++;
 }
 const animByName=new Map();
 for(const animation of model.animationSources||[]){
  assert.ok(!animByName.has(animation.name),`${row.url}: unique animation name`);animByName.set(animation.name,animation);
  source(model.source.archive,animation.entry,animation.sha256,`${row.url} ${animation.name}`);
  const staticPose=animation.static===true;
  assert.ok(Number.isFinite(animation.duration)&&animation.duration>=0&&animation.duration<120,`${row.url}/${animation.name}: bounded duration`);
  assert.equal(animation.duration===0,staticPose,`${row.url}/${animation.name}: zero duration explicitly marks static source pose`);
  assert.ok(Number.isInteger(animation.frames)&&animation.frames>=(staticPose?1:2),`${row.url}/${animation.name}: source animation samples`);
  if(staticPose)assert.equal(animation.frames,1,`${row.url}/${animation.name}: one static pose, no fabricated loop duration`);
 }
 let triangles=0;
 for(const [partIndex,part]of model.parts.entries()){
  const label=`${row.url} part ${partIndex} (${part.name})`;
  finite(part.positions,3,label+' positions');normalVectors(part.normals,label+' normals');finite(part.uvs,2,label+' UVs');
  const vertices=part.positions.length/3;assert.equal(part.normals.length,vertices*3,label+' matching normals');assert.equal(part.uvs.length,vertices*2,label+' matching UVs');
  assert.ok(Array.isArray(part.indices)&&part.indices.length>0&&part.indices.length%3===0,label+' triangle index triples');
  for(const i of part.indices)assert.ok(Number.isInteger(i)&&i>=0&&i<vertices,label+' index in vertex buffer');
  if(part.textureUrl){assert.ok(texByUrl.has(part.textureUrl),label+' source texture metadata');localFile(part.textureUrl);}
  finite(part.color,4,label+' RGBA material');assert.equal(part.color.length,4,label+' RGBA material width');
  assert.ok(Number.isInteger(part.sourceNode)&&part.sourceNode>=0,label+' original node ID');
  assert.ok(!/_LOD[1-9]\d*\b/i.test(part.name),label+' lower LOD geometry excluded');
  if(part.colors){finite(part.colors,4,label+' vertex colours');assert.equal(part.colors.length,vertices*4,label+' matching vertex colours');}
  if(part.activities){assert.ok(Array.isArray(part.activities));assert.equal(new Set(part.activities).size,part.activities.length);for(const a of part.activities)assert.ok(Number.isInteger(a)&&a>=0&&a<=6,label+' known tool activity');}
  if(part.animations){
   animatedParts++;
   for(const [name,animation]of Object.entries(part.animations)){
    const source=animByName.get(name);assert.ok(source,label+' animation has original KF provenance');assert.equal(animation.duration,source.duration,label+' animation duration matches source metadata');
    assert.equal(!!animation.static,!!source.static,label+' static pose matches source metadata');
    assert.equal(animation.frames.length,animation.constantPose?1:source.frames,label+' animation frame count');if(animation.constantPose){assert.equal(animation.sampledFrames,source.frames,label+' constant part tested at every native sample');assert.ok(source.frames>1,label+' redundant samples really omitted');}
    for(const [frameIndex,frame]of animation.frames.entries()){
     finite(frame.positions,3,label+` ${name}/${frameIndex} positions`);normalVectors(frame.normals,label+` ${name}/${frameIndex} normals`);
     assert.equal(frame.positions.length,part.positions.length,label+' morph position buffer length');assert.equal(frame.normals.length,part.normals.length,label+' morph normal buffer length');animationFrames++;
    }
   }
  }
  partsTotal++;verticesTotal+=vertices;triangles+=part.indices.length/3;
 }
 assert.equal(triangles,row.triangles,`${row.url}: triangle count matches manifest`);trianglesTotal+=triangles;
 const n=model.normalization;assert.ok(n&&Number.isFinite(n.scale)&&n.scale>0,`${row.url}: positive scale`);finite(n.offset,3,'centering offset');assert.equal(n.offset.length,3);assert.equal(n.axis,'x,z,-y','documented axis conversion');
 finite(model.sourceBounds.min,3,'source lower bound');finite(model.sourceBounds.max,3,'source upper bound');
 const referenceParts=n.kind==='human'?model.parts.filter(part=>part.skinned):model.parts;
 assert.ok(referenceParts.length>0,`${row.url}: normalization reference geometry`);
 const {low,high}=bounds(referenceParts),epsilon=3e-5;
 for(let axis=0;axis<3;axis++){
  const expectedLow=(model.sourceBounds.min[axis]+n.offset[axis])*n.scale,expectedHigh=(model.sourceBounds.max[axis]+n.offset[axis])*n.scale;
  assert.ok(Math.abs(low[axis]-expectedLow)<epsilon&&Math.abs(high[axis]-expectedHigh)<epsilon,`${row.url}: exported geometry matches normalization metadata axis ${axis}`);
 }
 assert.ok(Math.abs(low[1])<epsilon,`${row.url}: grounded geometry`);
 assert.ok(Math.abs(low[0]+high[0])<epsilon&&Math.abs(low[2]+high[2])<epsilon,`${row.url}: centred footprint`);
 if(n.kind==='human'){
  assert.ok(Math.abs(high[1]-.85)<epsilon,`${row.url}: normalized human height`);
  if(row.id===8){
   for(const action of ['idle','walk','wood','mine','food','build','combat']){
    assert.ok(animByName.has(action),'citizen has '+action+' animation');
    const moves=referenceParts.some(part=>{const frames=part.animations?.[action]?.frames;if(!frames?.length)return false;return frames.some(frame=>frame.positions.some((v,i)=>Math.abs(v-frames[0].positions[i])>1e-4));});
    assert.ok(moves,'citizen '+action+' contains actual animated body motion');
   }
   for(const activity of [2,3,4,5,6])assert.ok(model.parts.some(part=>part.activities?.includes(activity)),'citizen tool for activity '+activity);
  }
 }else assert.ok(Math.abs(Math.max(high[0]-low[0],high[2]-low[2])-1)<epsilon,`${row.url}: normalized footprint width`);
}
assert.ok(lods>=1,'at least one genuine highest-detail LOD choice verified');
assert.ok(animatedParts>=1&&animationFrames>=56,'actual multi-action animation buffers were exported');

// Far meshes select genuine source LOD branches and keep the near mesh's exact
// entity coordinate space. Native far bounds may differ from near bounds:
// grounding or renormalizing them independently would make LOD switches jump.
const farModels=new Map();
let farBindings=0,farParts=0,farTriangles=0,farFrames=0,farSelections=0;
const canonicalAnimations=items=>(items||[]).map(item=>({...item,static:!!item.static}));
for(const row of manifest.models){
 if(!row.lodFarUrl)continue;
 farBindings++;
 const near=models.get(localFile(row.url)),filename=localFile(row.lodFarUrl);
 if(farModels.has(filename)){
  assert.equal(farModels.get(filename).nearModelUrl,row.url,'reused far mesh references the same near mesh');
  continue;
 }
 const far=JSON.parse(readFileSync(filename));farModels.set(filename,far);
 assert.equal(far.format,'ee2-original-mesh-v1');
 assert.equal(far.nearModelUrl,row.url,'far mesh records original near binding');
 assert.equal(far.lodPolicy,'furthest-native-range','far geometry comes from native LOD ranges');
 assert.deepEqual(far.normalization,near.normalization,'far mesh retains exact near normalization');
 assert.deepEqual(far.source,near.source,'near/far share the original source NIF and hash');
 for(const field of ['resourceVisualBinding','databaseVisualBinding'])if(near[field])assert.deepEqual(far[field],near[field],'far mesh retains database identity and source evidence');
 assert.deepEqual(far.sourceTextures,near.sourceTextures,'near/far share source texture provenance');
 assert.deepEqual(canonicalAnimations(far.animationSources),canonicalAnimations(near.animationSources),'near/far share KF source hashes, durations and sampling');
 finite(far.sourceBounds.min,3,row.lodFarUrl+' source lower bounds');
 finite(far.sourceBounds.max,3,row.lodFarUrl+' source upper bounds');
 assert.ok(Array.isArray(far.lodSelection)&&far.lodSelection.length>0,'far mesh has genuine native LOD selection');
 for(const selection of far.lodSelection){
  const ranges=selection.sourceRanges;
  assert.ok(Array.isArray(ranges)&&ranges.length===selection.children&&ranges.length>0,'far LOD source ranges match child count');
  let expected=0;
  for(let i=0;i<ranges.length;i++){
   assert.ok(Number.isFinite(ranges[i]['Near Extent'])&&Number.isFinite(ranges[i]['Far Extent']),'finite original LOD ranges');
   if(ranges[i]['Near Extent']>ranges[expected]['Near Extent'])expected=i;
  }
  assert.equal(selection.selected,expected,'furthest native source branch selected by maximum near extent');
  farSelections++;
 }
 const texByUrl=new Map((far.sourceTextures||[]).map(texture=>[texture.url,texture]));
 const animByName=new Map((far.animationSources||[]).map(animation=>[animation.name,animation]));
 assert.ok(Array.isArray(far.parts)&&far.parts.length>0,'native far mesh contains geometry');
 let triangles=0;
 for(const [partIndex,part]of far.parts.entries()){
  const label=row.lodFarUrl+' part '+partIndex+' ('+part.name+')';
  finite(part.positions,3,label+' positions');normalVectors(part.normals,label+' normals');finite(part.uvs,2,label+' UVs');
  const vertices=part.positions.length/3;
  assert.equal(part.normals.length,vertices*3,label+' matching normal count');
  assert.equal(part.uvs.length,vertices*2,label+' matching UV count');
  assert.ok(Array.isArray(part.indices)&&part.indices.length>0&&part.indices.length%3===0,label+' triangle triples');
  for(const index of part.indices)assert.ok(Number.isInteger(index)&&index>=0&&index<vertices,label+' index in vertex buffer');
  finite(part.color,4,label+' material RGBA');assert.equal(part.color.length,4);
  assert.ok(Number.isInteger(part.sourceNode)&&part.sourceNode>=0,label+' original node ID');
  if(part.textureUrl){assert.ok(texByUrl.has(part.textureUrl),label+' source texture metadata');verifyPng(part.textureUrl,texByUrl.get(part.textureUrl));}
  if(part.colors){finite(part.colors,4,label+' vertex RGBA');assert.equal(part.colors.length,vertices*4);}
  if(part.activities){assert.ok(Array.isArray(part.activities));for(const activity of part.activities)assert.ok(Number.isInteger(activity)&&activity>=0&&activity<=6,label+' original tool activity');}
  for(const [name,animation]of Object.entries(part.animations||{})){
   const source=animByName.get(name);assert.ok(source,label+' KF provenance');
   assert.equal(animation.duration,source.duration,label+' original KF duration');
   assert.equal(!!animation.static,!!source.static,label+' static KF flag');
   assert.equal(animation.frames.length,animation.constantPose?1:source.frames,label+' original KF sample count');if(animation.constantPose){assert.equal(animation.sampledFrames,source.frames,label+' constant part tested at every native sample');assert.ok(source.frames>1,label+' redundant samples really omitted');}
   for(const frame of animation.frames){
    finite(frame.positions,3,label+' animated positions');normalVectors(frame.normals,label+' animated normals');
    assert.equal(frame.positions.length,part.positions.length,label+' animation position buffer length');
    assert.equal(frame.normals.length,part.normals.length,label+' animation normal buffer length');farFrames++;
   }
  }
  triangles+=part.indices.length/3;farParts++;
 }
 assert.equal(row.lodNearTriangles,row.triangles,'near triangle metadata matches manifest');
 assert.equal(triangles,row.lodFarTriangles,'far triangle metadata matches generated geometry');
 assert.ok(triangles>0&&triangles<row.triangles,'far mesh is a real reduction of the same original model');
 farTriangles+=triangles;
}
assert.ok(farModels.size>0,'at least one genuine reduced native far mesh is available');

// Verify compressed original NIF, KF and texture sources with Python's independent
// standard-library ZIP reader. Only the selected entries are decompressed.
const installation=process.env.EE2_INSTALL||'C:\\Program Files (x86)\\GOG.com\\Empire Earth II';
const python=process.env.PYTHON||(existsSync('C:\\Python314\\python.exe')?'C:\\Python314\\python.exe':'python');
const script=String.raw`import sys,json,pathlib,zipfile,hashlib
root=pathlib.Path(sys.argv[1])/'zips';records=json.load(sys.stdin);archives={}
try:
 for record in records:
  name=record['archive']
  assert pathlib.PurePath(name).name==name, 'archive basename'
  if name not in archives: archives[name]=zipfile.ZipFile(root/name)
  raw=archives[name].read(record['entry'])
  assert hashlib.sha256(raw).hexdigest()==record['sha256'], name+'!'+record['entry']+' source hash mismatch'
 print(json.dumps({'entries':len(records),'archives':len(archives)}))
finally:
 for archive in archives.values(): archive.close()`;
const result=spawnSync(python,['-c',script,installation],{input:JSON.stringify([...sourceRecords.values()]),encoding:'utf8',maxBuffer:1024*1024});
assert.equal(result.status,0,'original ZIP provenance verification: '+(result.stderr||result.error||''));
const provenance=JSON.parse(result.stdout);
console.log('PASS native far LODs: '+farBindings+' bindings, '+farModels.size+' files, '+farParts+' parts, '+farTriangles+' triangles, '+farSelections+' source-range selections and '+farFrames+' animation frame buffers; exact near normalization and NIF/KF/texture provenance retained.');
console.log(`PASS original models: ${manifest.models.length} id/epoch bindings, ${models.size} mesh files, ${partsTotal} parts, ${verticesTotal} vertices, ${trianglesTotal} triangles; ${textureFiles.size} PNG payloads/dimensions/CRCs, ${lods} highest LOD selections, ${animationFrames} animation frame buffers and ${provenance.entries} original ZIP source hashes verified. Visual skinning comparison remains separate.`);
