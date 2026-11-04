import assert from 'node:assert/strict';
import {readFileSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createOriginalAudio} from '../docs/original-audio.js';

// Verify every extracted asset against the source-derived manifest. No substitute
// or synthesized recording can satisfy this check by sharing a familiar filename.
const root=new URL('../docs/local-original/audio/',import.meta.url);
const extracted=JSON.parse(readFileSync(new URL('manifest.json',root)));
assert.ok(Object.keys(extracted.assets).length>=140);
for(const asset of Object.values(extracted.assets)){
 const bytes=readFileSync(new URL(asset.file,root));assert.equal(bytes.length,asset.bytes);
 assert.equal(createHash('sha256').update(bytes).digest('hex'),asset.sha256);
}
assert.deepEqual(extracted.voices['8'].select,['v_cit_m_sel_01_we.mp3','v_cit_m_sel_02_we.mp3','v_cit_m_sel_03_we.mp3','v_cit_m_sel_04_we.mp3']);
assert.deepEqual(Object.keys(extracted.voices).map(Number).sort((a,b)=>a-b),Array.from({length:39},(_,i)=>i+8));
assert.deepEqual(extracted.unresolvedVoiceEvents,[]);
assert.equal(extracted.voices['16'].source_visual,'HeavyInfantry1_11');
assert.equal(extracted.voices['16'].select[0],'v_HeavyInfantry1_11_sel01.mp3','Rifleman no longer uses obsolete LightInfantry adapter');
assert.equal(extracted.voices['41'].source_visual,'HeavyMounted2_13');
assert.ok(extracted.voices['41'].attack.includes('v_HeavyMounted2_13_spe01.mp3'));
assert.equal(extracted.voices['43'].source_visual,'LightMounted3');
assert.equal(extracted.voices['43'].select[0],'v_HeavyInfantry1_15_sel01.mp3','Native Mech upgrade deliberately names infantry recordings');
assert.ok(extracted.voices['43'].attack.includes('v_HeavyInfantry1_15_spe01.mp3'));
assert.equal(extracted.voices['26'].select[0],'v_HeavyMounted1_15_sel01.mp3','Native HeavyMounted1_13 upgrade deliberately names _15 recordings');
for(const id of [44,45,46])assert.ok(extracted.voices[id].select.length&&extracted.voices[id].order.length&&extracted.voices[id].attack.length);
const modelManifest=JSON.parse(readFileSync(new URL('../docs/local-original/models/manifest.json',import.meta.url)));
const progressions=modelManifest.models.filter(row=>row.progressionBinding&&row.id>=8&&row.id<=46);
assert.equal(extracted.voiceVariants.length,progressions.length);
for(const row of progressions){
 const p=row.progressionBinding,v=extracted.voiceVariants.find(v=>v.id===row.id&&v.epoch===p.epoch);
 assert.ok(v,`voice interval ${row.id}/${p.epoch}`);assert.equal(v.until,p.until);
 assert.equal(v.source_unit,p.sourceUnit);assert.equal(v.source_visual,p.sourceVisual);
 assert.equal(v.upgrade_source.line,p.upgradeSource.line);assert.equal(v.upgrade_source.sha256,p.upgradeSource.sha256);
 assert.equal(v.visual_source.sha256,p.visualSource.sha256);assert.ok(v.epoch>=0&&v.epoch<v.until&&v.until<=15);
}
const tankVariant=extracted.voiceVariants.find(v=>v.id===26&&v.epoch===14);
assert.equal(extracted.voices['26'].upgrade_source.line,65);assert.equal(tankVariant.upgrade_source.line,66);
assert.equal(tankVariant.source_unit,'HeavyTank');assert.equal(tankVariant.source_visual,'HeavyMounted1_15');
assert.equal(extracted.effects['23'].file,'fx_resource_wood.mp3');
assert.equal(extracted.effects['24'].file,'fx_resource_stone.mp3');
assert.equal(extracted.effects['25'].file,'fx_contruction_01.mp3');
assert.equal(extracted.buildingSelections['48'].event,'Mill_select');
assert.equal(extracted.buildingSelections['7'].event,'WallTower_select');
const herc=extracted.buildingSelections['55'];
assert.equal(herc.event,'Herc_plant_select');assert.equal(herc.unit_type,'HercFacility');assert.equal(herc.volume,.9);
assert.equal(herc.file.toLowerCase(),'fx_bld_herc_plant.mp3');assert.equal(herc.source_epoch,14);
assert.equal(herc.selection_source.line,129);assert.equal(herc.selection_source.column,'SNDSEL');
assert.equal(herc.event_source.line,396);
const fortress=extracted.buildingSelections['56'];
assert.equal(fortress.event,'Fortress_select');assert.equal(fortress.unit_type,'Fortress');
assert.equal(fortress.file,'Fx_Bld_Fortress.mp3');assert.equal(fortress.volume,.9);
assert.deepEqual(fortress.source_epochs,[1,4,6,8,10,12,14]);
assert.deepEqual(fortress.selection_sources.map(row=>row.line),[130,131,132,133,134,135,136]);
assert.equal(fortress.event_source.line,390);
for(const id of ['53','54']){
 assert.equal(extracted.buildingSelections[id],undefined,'No guessed selection sound for '+id);
 assert.ok(extracted.unresolvedBuildingSelections[id].sources.length>=3,'Explicit source gap '+id);
}
// Independently read the original DB rows and compressed sound entry. This
// verifies more than agreement between an output file and its own manifest.
const python=process.env.PYTHON||(existsSync('C:\\Python314\\python.exe')?'C:\\Python314\\python.exe':'python');
const originalCheck=[
 "import json,sys,pathlib,zipfile,csv,hashlib",
 "m=json.load(sys.stdin);root=pathlib.Path(m['source']);r=m['buildingSelections']['55']",
 "with zipfile.ZipFile(root/'zips/db.zip') as db,zipfile.ZipFile(root/'zips/sounds.zip') as sounds:",
 " for key in ['selection_source','event_source']:",
 "  s=r[key];raw=db.read(s['entry']);assert hashlib.sha256(raw).hexdigest()==s['sha256']",
 "  line=raw.decode('cp1252').replace('\\r','').splitlines()[s['line']-1]",
 "  if key=='selection_source':",
 "   header=next(csv.reader([raw.decode('cp1252').splitlines()[0].removeprefix('//').strip()]))",
 "   row=dict(zip(header,next(csv.reader([line]))));assert row['UNIT']=='HercFacility' and row['SNDSEL']=='[Herc_plant_select]'",
 "  else:",
 "   row=next(csv.reader([line]));assert row[0]==r['event'] and row[1].lower()==r['file'].lower()",
 "   assert float(row[2])==r['volume'] and row[3]=='0'",
 " asset=m['assets'][r['file'].lower()]",
 " assert asset['transform']=='byte-for-byte copy'",
 " assert hashlib.sha256(sounds.read(asset['source_entry'])).hexdigest()==asset['sha256']",
 " f=m['buildingSelections']['56']",
 " for s,epoch in zip(f['selection_sources'],f['source_epochs']):",
 "  raw=db.read(s['entry']);assert hashlib.sha256(raw).hexdigest()==s['sha256']",
 "  lines=raw.decode('cp1252').splitlines();header=next(csv.reader([lines[0].removeprefix('//').strip()]))",
 "  row=dict(zip(header,next(csv.reader([lines[s['line']-1]]))))",
 "  assert row['UNIT']=='Fortress' and int(row['EPOCH'])==epoch and row['SNDSEL'].lower()=='[fortress_select]'",
 "  assert row['SNDMOVE']=='[]' and row['SNDATTACK']=='[]'",
 " s=f['event_source'];raw=db.read(s['entry']);assert hashlib.sha256(raw).hexdigest()==s['sha256']",
 " row=next(csv.reader([raw.decode('cp1252').splitlines()[s['line']-1]]))",
 " assert row[0]==f['event'] and row[1]==f['file'] and float(row[2])==f['volume']",
 " asset=m['assets'][f['file'].lower()];assert asset['transform']=='byte-for-byte copy'",
 " assert hashlib.sha256(sounds.read(asset['source_entry'])).hexdigest()==asset['sha256']",
].join('\n');
const checked=spawnSync(python,['-c',originalCheck],{input:JSON.stringify(extracted),encoding:'utf8'});
assert.equal(checked.status,0,'Original HercFacility/Fortress source evidence: '+(checked.stderr||checked.error||''));
// All 39 mappings are checked against independently read native declarations,
// upgrade columns and recording rows; a filename that merely resembles the
// model stem cannot satisfy this (notably the intentional 43/_15 reuse).
const allVoicesCheck=[
 "import json,sys,pathlib,zipfile,csv,hashlib,re",
 "m=json.load(sys.stdin);root=pathlib.Path(m['source']);assert set(map(int,m['voices']))==set(range(8,47))",
 "with zipfile.ZipFile(root/'zips/db.zip') as db,zipfile.ZipFile(root/'zips/sounds.zip') as sounds:",
 " sound_lines=db.read('db/Audio/dbsound.csv').decode('cp1252').replace('\\r','').splitlines()",
 " events={r[0].lower():r for line in sound_lines if line.strip() and not line.lstrip().startswith('//') and len(r:=next(csv.reader([line])))>=6}",
 " for asset in m['assets'].values():",
 "  if asset.get('derived_from'):continue",
 "  raw=sounds.read(asset['source_entry']) if asset.get('source_archive')=='zips/sounds.zip' else (root/asset['source_file']).read_bytes()",
 "  assert hashlib.sha256(raw).hexdigest()==asset['sha256'] and len(raw)==asset['bytes']",
 " voice_cases=list(m['voices'].items())+[(str(v['id']),v) for v in m['voiceVariants']]",
 " for uid,v in voice_cases:",
 "  assert v['mapping_verified'] and v['select'] and v['order']",
 "  if uid=='8':",
 "   src=v['source'];raw=db.read(src['entry']);assert hashlib.sha256(raw).hexdigest()==src['sha256']",
 "   body=re.search(r'UnitType\\s+CitizenM\\s*\\{(.*?)(?=UnitType|UnitModel|\\Z)',raw.decode('cp1252').replace('\\r',''),re.S).group(1)",
 "   for action,field in [('select','sndSelection'),('order','sndCmdMove'),('attack','sndCmdAttack')]:",
 "    names=re.findall(r'\"([^\"]+)\"',re.search(field+r'\\s*=\\s*\\[([^]]*)\\]',body).group(1));assert v['events'][action]==[n+'_we' for n in names]",
 "  else:",
 "   src=v['upgrade_source'];raw=db.read(src['entry']);assert hashlib.sha256(raw).hexdigest()==src['sha256']",
 "   lines=raw.decode('cp1252').replace('\\r','').splitlines();header=next(csv.reader([lines[0].removeprefix('//').strip()]));row=dict(zip(header,next(csv.reader([lines[src['line']-1]]))))",
 "   assert row['VISUAL']==v['source_visual'] and row['UNIT']==v['source_unit'] and row['UPGRADE']==v['source_upgrade']",
 "   assert int(row['EPOCH'])==v['source_epoch']",
 "   visual=v['visual_source'];vr=db.read(visual['entry']);assert hashlib.sha256(vr).hexdigest()==visual['sha256']",
 "   line=vr.decode('cp1252').replace('\\r','').splitlines()[visual['line']-1];assert re.match(r'\\s*UnitModel\\s+'+re.escape(v['source_visual'])+r'\\b',line)",
 "   for action,column in src['columns'].items():assert v['events'][action]==re.findall(r'\\b\\w+\\b',row[column])",
 "  for action in ['select','order','attack']:",
 "   filenames=list(dict.fromkeys(events[e.lower()][1] for e in v['events'][action]));assert [f.lower() for f in v[action]]==[f.lower() for f in filenames],(uid,action)",
 "   for filename in filenames:",
 "    asset=m['assets'][filename.lower()];raw=sounds.read(asset['source_entry']);assert hashlib.sha256(raw).hexdigest()==asset['sha256']",
 "    for rec in asset['sound_records']:",
 "     table=rec['table'].split('!',1)[1];tr=db.read(table);assert hashlib.sha256(tr).hexdigest()==rec['table_sha256']",
 "     r=next(csv.reader([tr.decode('cp1252').replace('\\r','').splitlines()[rec['line']-1]]));assert r[0]==rec['name'] and r[1]==rec['filename']",
].join('\n');
const voicesChecked=spawnSync(python,['-c',allVoicesCheck],{input:JSON.stringify(extracted),encoding:'utf8'});
assert.equal(voicesChecked.status,0,'All current original UnitModel/upgrade/recording voice bindings: '+(voicesChecked.stderr||voicesChecked.error||''));
// The fixed visual bindings must also match the current numerical/identity
// audit, not just the importer's own hardcoded list.
const unitAudit=JSON.parse(readFileSync(new URL('../docs/ee2-units.json',import.meta.url)));
for(const id of Array.from({length:39},(_,i)=>i+8)){
 const source=unitAudit.rows.filter(row=>row.id===id).sort((a,b)=>a.epoch-b.epoch)[0];
 assert.equal(extracted.voices[id].source_visual,source.source_visual,`current visual source ${id}`);
}
const ambient=extracted.assets[extracted.ambient.loop.toLowerCase()];
assert.equal(ambient.derived_from,'ambloop_land_01_temp.wav');assert.equal(ambient.bits_per_sample,16);assert.equal(ambient.sample_rate,8000);assert.equal(ambient.channels,1);assert.equal(ambient.frames,132310);
const pcm=readFileSync(new URL(ambient.file,root));assert.equal(pcm.toString('ascii',0,4),'RIFF');assert.equal(pcm.readUInt16LE(20),1,'ambient playback is browser-compatible PCM');
assert.equal(pcm.readUInt16LE(22),1);assert.equal(pcm.readUInt32LE(24),8000);assert.equal(pcm.readUInt16LE(34),16);assert.equal(pcm.readUInt32LE(40),ambient.frames*2);
assert.equal(ambient.source_sha256,extracted.assets[ambient.derived_from.toLowerCase()].sha256,'PCM derivative retains original hash');

// Exercise the actual playback module against a minimal WebAudio graph, with
// asynchronous fetch/decode, rather than duplicating its choice logic in a test.
const sources=[];
class Node{constructor(){this.gain={value:0};this.pan={value:0};}connect(){}disconnect(){this.disconnected=true;}}
class Source extends Node{start(t,offset){this.offset=offset;this.started=true;sources.push(this);}stop(){this.stopped=true;}}
const context={currentTime:1,state:'running',createBufferSource:()=>new Source(),createGain:()=>new Node(),createStereoPanner:()=>new Node(),async decodeAudioData(bytes){const file=new TextDecoder().decode(bytes);return{file,duration:file.startsWith('music')?60:2};}};
const manifest={version:1,voices:{8:{select:['select.mp3'],order:['order.mp3'],build:['order.mp3'],attack:['worker-attack.mp3']},9:{select:['club.mp3'],order:['club-order.mp3'],attack:['club-attack.mp3']},26:{select:['tank.mp3'],order:['tank-order.mp3'],attack:['tank-attack.mp3']},41:{select:['gunship.mp3'],order:['gunship-order.mp3'],attack:['gunship-attack.mp3']},43:{select:['mech.mp3'],order:['mech-order.mp3'],attack:['mech-attack.mp3']}},
 voiceVariants:[{id:9,epoch:2,until:4,select:['sword-a.mp3','sword-b.mp3'],order:['sword-order.mp3'],attack:['sword-attack.mp3']},
  {id:9,epoch:6,until:8,select:['musket.mp3'],order:['musket-order.mp3'],attack:['musket-attack.mp3']},
  {id:26,epoch:14,until:15,select:['future-tank.mp3'],order:['future-tank-order.mp3'],attack:['future-tank-attack.mp3']}],
 effects:{23:{file:'wood.mp3',volume:.3}},buildingSelections:{0:{file:'building.mp3',volume:.9},55:{file:'herc.mp3',volume:.9}},music:{west:['music-west.mp3'],late:['music-late.mp3']},ambient:{loop:'ambient.wav',birds:['bird.mp3']}};
const fetchMock=async url=>({ok:true,json:async()=>manifest,arrayBuffer:async()=>new TextEncoder().encode(String(url).split('/').at(-1)).buffer});
const tick=()=>new Promise(resolve=>setImmediate(resolve));
const a=createOriginalAudio({enabled:true,fetch:fetchMock,base:new URL('https://local.test/audio/')});
assert.equal(await a.prepare(context,new Node()),true);await tick();
assert.equal(a.acknowledge(8,'select'),true);assert.equal(sources.at(-1).buffer.file,'select.mp3');
assert.equal(a.acknowledge(8,'build'),true);assert.equal(sources.at(-1).buffer.file,'order.mp3');assert.equal(a.status().activeVoices,1);
// A renderer may warm these clips up front; a cold request itself must still
// load rather than playing a different unit's recording.
for(const [id,file] of [[41,'gunship-attack.mp3'],[43,'mech-attack.mp3']]){
 a.acknowledge(id,'attack');await tick();assert.equal(a.acknowledge(id,'attack'),true);assert.equal(sources.at(-1).buffer.file,file);
}
function expectVoice(id,action,epoch,file){assert.equal(a.acknowledge(id,action,epoch),true);assert.equal(sources.at(-1).buffer.file,file);}
expectVoice(9,'select',0,'club.mp3');expectVoice(9,'select',1,'club.mp3');
expectVoice(9,'select',2,'sword-a.mp3');expectVoice(9,'select',6,'musket.mp3');
expectVoice(9,'select',3,'sword-b.mp3'); // returning to a native interval resumes its own cycle
expectVoice(9,'select',4,'club.mp3');expectVoice(9,'select',5,'club.mp3');expectVoice(9,'select',8,'club.mp3');
expectVoice(9,'order',2,'sword-order.mp3');expectVoice(9,'attack',2,'sword-attack.mp3');expectVoice(9,'attack',6,'musket-attack.mp3');
expectVoice(26,'attack',13,'tank-attack.mp3');expectVoice(26,'attack',14,'future-tank-attack.mp3');expectVoice(26,'attack',15,'tank-attack.mp3');
a.setSpeechEnabled(false);assert.equal(a.status().activeVoices,0);assert.equal(a.acknowledge(8,'select'),false);assert.equal(a.acknowledge(0,'select'),true,'building effects independent of voices');
assert.equal(a.acknowledge(55,'select'),true,'Dynamic building55 binding uses actual playback path');assert.equal(sources.at(-1).buffer.file,'herc.mp3');
assert.equal(a.acknowledge(53,'select'),false,'OilRig work loop is not invented as selection');assert.equal(a.acknowledge(54,'select'),false,'Uranium work loop is not invented as selection');
assert.equal(a.effect(23,.5,.3),true);assert.equal(sources.at(-1).buffer.file,'wood.mp3');assert.equal(a.effect(24),false,'unavailable mapping uses caller fallback');
assert.equal(a.music(true,0,true),true);await tick();assert.equal(a.status().music,true);assert.equal(a.status().ambient,true);
assert.equal(a.status().musicStarts,1);context.currentTime=7;a.music(false,0,true);assert.equal(a.status().music,false);assert.equal(a.status().ambient,false);
a.music(true,0,true);assert.equal(a.status().musicStarts,2);assert.equal(sources.findLast(s=>s.originalGroup==='music').offset,6,'pause resumes at previous track position');
a.setMusicEnabled(false);a.music(true,0,false);assert.equal(a.status().music,false);assert.equal(a.status().ambient,true,'music toggle preserves environmental audio');
a.setMuted(true);assert.equal(a.status().ambient,false);assert.equal(a.effect(23),false);assert.equal(a.acknowledge(0,'select'),false);
assert.equal(a.acknowledge(55,'select'),false,'New building selection respects mute');
a.setMuted(false);a.setMusicEnabled(true);a.music(true,10,true);await tick();assert.equal(sources.findLast(s=>s.originalGroup==='music').buffer.file,'music-late.mp3');
const missing=createOriginalAudio({enabled:true,fetch:async()=>({ok:false}),base:new URL('https://local.test/audio/')});assert.equal(await missing.prepare(context,new Node()),false);assert.equal(missing.music(true),false);assert.equal(missing.acknowledge(8,'select'),false);assert.equal(missing.effect(23),false);
console.log('PASS originals: 39 independently source-verified UnitModel voice bindings + native progression intervals, exact original upgrade rows, interval boundaries/base fallback/per-variant cycles, attack 41/43 playback, audio/PCM bytes, pause/resume, music/voice/mute and fallback');
