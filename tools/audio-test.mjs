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
