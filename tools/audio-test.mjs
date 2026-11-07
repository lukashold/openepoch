import assert from 'node:assert/strict';
let clock=1000,spoken=[],oscillators=0,muted=[];
globalThis.performance={now:()=>clock};
class Param{setValueAtTime(){} exponentialRampToValueAtTime(){} setTargetAtTime(v){muted.push(v)}}
class SoundNode{constructor(){this.gain=new Param();this.frequency=new Param();this.Q=new Param();this.pan=new Param();}connect(){} start(){oscillators++;}stop(){}}
class AudioContext{constructor(){this.currentTime=1;this.sampleRate=8;this.state='running';this.destination={};}createGain(){return new SoundNode()}createDynamicsCompressor(){return new SoundNode()}createBuffer(){return {getChannelData:()=>new Float32Array(8)}}createOscillator(){return new SoundNode()}createStereoPanner(){return new SoundNode()}createBufferSource(){return new SoundNode()}createBiquadFilter(){return new SoundNode()}}
class Utterance{constructor(text){this.text=text}}
globalThis.SpeechSynthesisUtterance=Utterance;
globalThis.window={AudioContext,SpeechSynthesisUtterance:Utterance,speechSynthesis:{getVoices:()=>[{lang:'de-DE',localService:true}],cancel(){},speak:u=>spoken.push(u)}};
const a=await import('../docs/audio.js');a.unlock();a.music(true,0);assert.ok(oscillators>=2);
assert.equal(a.acknowledge(8,'select'),true);assert.equal(spoken[0].text,'Bereit.');
assert.equal(a.acknowledge(8,'order'),false,'rate limit');clock+=1000;assert.equal(a.acknowledge(8,'order'),true);assert.equal(spoken.length,2);
a.setMuted(true);clock+=1000;assert.equal(a.acknowledge(8,'order'),false);assert.equal(muted.at(-1),0);a.setMuted(false);a.setSpeechEnabled(false);assert.equal(a.acknowledge(9,'select'),false);
a.setSpeechEnabled(true);window.speechSynthesis.getVoices=()=>[];assert.equal(a.acknowledge(8,'select'),false);a.setMusicEnabled(false);const n=oscillators;a.music(true,0);assert.equal(oscillators,n);
console.log('PASS audio: music scheduling, local speech, cooldown, mute, missing-voice fallback and controls');

const {createOriginalAudio}=await import('../docs/original-audio.js');
const recording='shared.mp3',fetches=[];
function family(name,sha256){
 const event=`${name}-selected`;
 return {format:'ee2-native-family-audio-v1',assets:{[recording]:{sha256}},events:{[event]:{file:recording,volume:1}},
  families:{[name]:{role:'voice',intervals:[{epoch:10,until:15,sourceVisual:name,
   effectiveEvents:{select:[event],order:[],attack:[]},effectiveEventSources:{select:name}}]}}};
}
const source=family('Base','a'.repeat(64));
const mixer=createOriginalAudio({enabled:true,familyManifest:source,base:new URL('https://local.test/audio/'),fetch:async url=>{
 if(url.pathname.endsWith('.json'))return {ok:true,json:async()=>({version:1,voices:{},effects:{}})};
 fetches.push(url.href);
 return {ok:true,arrayBuffer:async()=>new ArrayBuffer(1)};
}});
mixer.setDefinitionBindings({wasm:{definition_count:()=>106,definition_flags:()=>2},
 bindings:['Base','Shared','Conflict'].map((name,index)=>({assetKind:0,id:102+index,epoch:10,sourceFamily:name,sourceVisual:name})),
 additionalFamilyManifest:[family('Shared','a'.repeat(64)),family('Conflict','b'.repeat(64))]});
const context=new AudioContext();context.decodeAudioData=async()=>({duration:1});
assert.equal(await mixer.prepare(context,new SoundNode()),true);
await new Promise(resolve=>setImmediate(resolve));
assert.equal(mixer.acknowledge(102,'select',10),true);
assert.equal(mixer.acknowledge(103,'select',10),true);
assert.equal(mixer.acknowledge(104,'select',10),false);
assert.deepEqual(fetches,['https://local.test/audio/shared.mp3?v=aaaaaaaaaaaaaaaa']);
console.log('PASS original audio: exact shared recording accepted and conflicting digest rejected');
