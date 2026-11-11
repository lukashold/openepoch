import {createUnitReplyPicker} from './original-citizen-replies.js?v=native112voice';
import {loadInfantryReplyCatalog} from './original-infantry-replies.js?v=infantry157';
import {createTypedAudioBindings} from './original-audio-bindings.js?v=native8';
import {createRegionalMusicPlayback} from './original-regional-music.js?v=regional12';
import {originalOneShotAudible,originalSpatialMix} from './original-spatial-audio.js?v=native39audio';
import {createOriginalAmbientPlayback} from './original-ambient.js?v=native106';
import {playbackRate} from './air-flight-audio.js?v=native65-herc';
import {createGranaryLoopPlayback} from './original-granary.js?v=granary211';
import {createOilRigLoopPlayback, OIL_RIG_CUE} from './dv2-oil-rig-audio.js?v=oil-presentation286-private';
import {createCitizenWorkPlayback} from './dv2-citizen-work-audio.mjs?v=work-events290-private';
import {advancePitch} from './dv2-work-event-clock.mjs?v=work-events290-private';
// Playback of original assets extracted from the user's local EE2 installation.
// Metadata and byte-for-byte source hashes live beside those assets. This module
// contains no recordings and remains usable when local-original is absent.
const DEFAULT_BASE=new URL('./local-original/audio/',import.meta.url);
const STONE_THROWER_LAUNCH=26;
const UNIT_REPLIES=['citizen-reply','submarine-reply','infantry-reply'];
// Process/module lifetime for this work-cue stream: scene/reset/configure never
// reseed it. Existing guided/flight streams retain their independent behavior;
// complete native cross-subsystem RNG interleaving is not claimed here.
let citizenWorkPitchSeed=12345;
const PRIEST_CUES=Object.freeze({
  32:{file:'Building_select.mp3',sha256:'e25cb0a14c057f20d40fd32566a481927b8751f191b19622beb169cd67792646',volume:1,maximumInstances:1},
  33:{file:'priest/fx_priest_convert.mp3',sha256:'89e788acae48316721f25c00a602c13616551a387fbc32638259e4b8259e7ea8',volume:.7,maximumInstances:4},
});

export function createOriginalAudio(options={}) {
  const enabled=options.enabled ?? (typeof window!=='undefined' && new URLSearchParams(window.location?.search||'').get('rules')==='ee2');
  const fetchFile=options.fetch || ((...args)=>fetch(...args));
  const base=options.base || DEFAULT_BASE;
  let ctx=null,soundBus=null,musicBus=null,manifest=null,loading=null,muted=false,speech=true,musicEnabled=true,ambienceEnabled=true,active=false;
  const buffers=new Map(),pending=new Map(),failed=new Set(),live=new Set(),counters=new Map();
  const pickUnitReply=options.citizenReplySeconds&&options.presentationRandom?
    createUnitReplyPicker(options.presentationRandom,options.citizenReplySeconds):null;
  let infantryRepliesEnabled=false,infantryReplies=null,infantryLoading=null;
  const takeoffBuffers=new Map(),takeoffLoads=new Map();
  let takeoffEvents=new Map();
  let priestCues=false;
  let guidedImpact=null,guidedPitchSeed=12345;
  let citizenWork=null,citizenWorkEvents=new Map();
  const wheelLoops=new Map();
  let wheelSession=null,granaryLoops=null,oilRigLoops=null;
  let musicSource=null,musicBuffer=null,musicFile=null,musicOffset=0,musicStarted=0,musicIndex=0,musicGeneration=0,musicPendingFile=null;
  let ambientSource=null,nextBird=0,currentEpoch=0,previousPeriod=-1;
  let ambientPresentation=null,nativeAmbient=null,nativeAmbientOwnsMix=false;
  let citizenPresentation=options.citizenPresentation||null;
  let regionalMusicEnabled=!!options.regionalMusic, regionalMusic=null;
  let playbackManifestURL='manifest.json';
  let familyManifest=options.familyManifest||null,familyManifestURL=options.familyManifestURL||'native-families.json',definitionSettings={wasm:null,bindings:[]},typed=createTypedAudioBindings({familyManifest});
  let additionalFamilyManifest=null,selectedFamilyManifest=familyManifest;
  function bindFamilies() {
    selectedFamilyManifest=familyManifest;
    try { typed=createTypedAudioBindings({...definitionSettings,familyManifest}); }
    catch { familyManifest=selectedFamilyManifest=null;typed=createTypedAudioBindings(definitionSettings); }
    const additions=Array.isArray(additionalFamilyManifest)?additionalFamilyManifest:[additionalFamilyManifest];
    for(const addition of additions){
      if(addition?.format!=='ee2-native-family-audio-v1')continue;
      try{
        const combined={...addition};
        for(const key of ['families','events','assets']){
          const inherited=selectedFamilyManifest?.[key]||{},added=addition[key]||{};
          for(const [name,record] of Object.entries(added)){
            if(!Object.hasOwn(inherited,name))continue;
            // A different source family can reference the same original recording.
            const sha=record?.sha256;
            if(key!=='assets'||!/^[a-f0-9]{64}$/.test(sha)||sha!==inherited[name]?.sha256)
              throw Error('Overlapping native sound family');
          }
          combined[key]={...added,...inherited};
        }
        typed=createTypedAudioBindings({...definitionSettings,familyManifest:combined});
        selectedFamilyManifest=combined;
      }catch{/* A missing or inconsistent optional family keeps existing voices. */}
    }
  }
  function setDefinitionBindings({wasm=null,bindings=[],familyManifestURL:url=familyManifestURL,
    citizenPresentation:citizens=citizenPresentation,regionalMusic:useRegionalMusic=regionalMusicEnabled,
    playbackManifestURL:playbackURL=playbackManifestURL,additionalFamilyManifest:additional=null,
    infantryReplies:useInfantry=infantryRepliesEnabled}={}) {
    if(url!==familyManifestURL){familyManifest=null;familyManifestURL=url;}
    citizenPresentation=citizens;
    regionalMusicEnabled=useRegionalMusic;
    playbackManifestURL=playbackURL;
    infantryRepliesEnabled=useInfantry;
    if (!regionalMusicEnabled) regionalMusic?.reset();
    definitionSettings={wasm,bindings};
    additionalFamilyManifest=additional;
    bindFamilies();
    if(manifest){prepareFamilyBindings();prepareInfantryReplies();}
  }
  const stats={voicePlays:0,effectPlays:0,musicStarts:0,ambientStarts:0};

  function load(file) {
    if(!file||!ctx||failed.has(file))return Promise.resolve(null);
    if(buffers.has(file))return Promise.resolve(buffers.get(file));
    if(pending.has(file))return pending.get(file);
    const url=new URL(file,base),hash=manifest?.assets?.[file.toLowerCase()]?.sha256||selectedFamilyManifest?.assets?.[file]?.sha256||citizenPresentation?.assetHash(file);
    if(hash)url.searchParams.set('v',hash.slice(0,16));
    const p=fetchFile(url,{cache:'force-cache'})
      .then(r=>{if(!r.ok)throw Error('Original audio unavailable');return r.arrayBuffer();})
      .then(bytes=>ctx.decodeAudioData(bytes))
      .then(buffer=>{buffers.set(file,buffer);pending.delete(file);return buffer;})
      .catch(()=>{failed.add(file);pending.delete(file);return null;});
    pending.set(file,p);return p;
  }
  function stop(source) {
    if(!source)return;
    try{source.stop();}catch{}
    try{source.disconnect();}catch{}
    source.originalCleanup?.();
    live.delete(source);
  }
  function start(buffer,volume=1,pan=0,group='effect',loop=false,offset=0,rate=1) {
    if(!buffer||muted||!ctx||ctx.state!=='running')return null;
    if(group==='effect'&&live.size>32)return null;
    let source;
    try{source=ctx.createBufferSource();source.buffer=buffer;source.loop=loop;}
    catch{try{source?.disconnect();}catch{}return null;}
    // Native pitch RNG is consumed only after source/sample allocation succeeds.
    const suppliedRate=typeof rate==='function'?rate():rate;
    if(suppliedRate!==1)source.playbackRate.value=suppliedRate;
    const gain=ctx.createGain();gain.gain.value=Math.max(0,Math.min(1,volume));
    const panner=ctx.createStereoPanner?.();
    const output=panner??gain;
    if(panner){panner.pan.value=Math.max(-1,Math.min(1,pan));gain.connect(panner);}
    source.connect(gain);output.connect(group==='music'?musicBus:soundBus);source.originalGroup=group;
    if(group==='wheel'||group==='granary-idle'||group==='oil-rig'||group.startsWith('citizen-work:')){
      source.updateSpatialMix=(volume,pan,fadeSeconds=0)=>{
        const level=Math.max(0,Math.min(1,volume));
        if(group==='oil-rig'){
          const now=ctx.currentTime;
          gain.gain.cancelScheduledValues(now);
          gain.gain.setValueAtTime(level,now);
          if(fadeSeconds>0)gain.gain.linearRampToValueAtTime(0,now+fadeSeconds);
        }else gain.gain.value=level;
        if(panner)panner.pan.value=Math.max(-1,Math.min(1,pan));
      };
    }
    if(group!=='wheel')live.add(source);
    source.originalCleanup=()=>{try{gain.disconnect();if(output!==gain)output.disconnect();}catch{}};
    source.onended=()=>{live.delete(source);source.originalCleanup();};
    source.start(0,Math.max(0,offset));return source;
  }
  function stopStoneThrowerWheels() {
    for(const {source} of wheelLoops.values())stop(source);
    wheelLoops.clear();
    wheelSession=null;
  }
  function loadVerifiedEffect(event){
    if(!ctx||!event)return Promise.resolve(null);
    if(takeoffLoads.has(event.sha256))return takeoffLoads.get(event.sha256);
    const url=new URL(event.url);
    url.searchParams.set('v',event.sha256.slice(0,16));
    // A private loader may already hold the encoded file. Copy before the
    // asynchronous digest/decode so caller mutation cannot change checked data.
    const encoded=event.bytes instanceof ArrayBuffer?Promise.resolve(event.bytes.slice(0)):
      fetchFile(url,{cache:'force-cache'}).then(response=>{
      if(!response.ok)throw Error('Original audio unavailable');
      return response.arrayBuffer();
    });
    const load=encoded.then(async bytes=>{
      const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
      if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==event.sha256)throw Error('Original audio checksum differs');
      return ctx.decodeAudioData(bytes);
    }).then(buffer=>{takeoffBuffers.set(event.sha256,buffer);return buffer;}).catch(()=>null);
    takeoffLoads.set(event.sha256,load);
    return load;
  }
  function configureCitizenWork(catalog){
    citizenWork?.reset();citizenWork=null;citizenWorkEvents=new Map();
    if(!enabled||!catalog)return false;
    const sounds=catalog.sounds??catalog;
    const entries=sounds instanceof Map?[...sounds]:Object.entries(sounds);
    const events=new Map(entries.map(([name,cue])=>{
      if(typeof name!=='string'||!cue||cue.name!==undefined&&cue.name!==name||
          !/^[a-f0-9]{64}$/.test(cue.sha256??'')||!Number.isInteger(cue.sampleRate)||cue.sampleRate<=0)
        throw Error('Invalid verified Citizen work sound');
      return [name,{...cue,name,url:new URL(cue.url,import.meta.url).href,
        volume:cue.volume??cue.gain,loop:cue.loop===false?0:cue.loop}];
    }));
    if(!events.size)throw Error('Empty Citizen work sound catalog');
    const playback=createCitizenWorkPlayback({events,ready:()=>enabled&&!muted&&ctx?.state==='running',
      buffer:cue=>takeoffBuffers.get(cue.sha256),load:loadVerifiedEffect,start,stop,
      clock:()=>ctx?.currentTime,mix:originalSpatialMix,
      frequencyFor:cue=>{
        const pitch=advancePitch(citizenWorkPitchSeed,cue.pitchVariation,'nearest');
        citizenWorkPitchSeed=pitch.seed;
        // Encoded sample frequency is authoritative; AudioContext decode may
        // resample the buffer. Zero variation never consumes RNG or sets pitch.
        return pitch.consumed?pitch.frequency/cue.sampleRate:1;
      }});
    citizenWorkEvents=events;citizenWork=playback;
    if(ctx)void playback.preload();
    return true;
  }
  function citizenWorkReady(name){
    if(!citizenWork||muted||ctx?.state!=='running')return false;
    if(name!==undefined){const cue=citizenWorkEvents.get(name);return !!cue&&takeoffBuffers.has(cue.sha256);}
    return [...citizenWorkEvents.values()].every(cue=>takeoffBuffers.has(cue.sha256));
  }
  function citizenWorkAudioStatus(){
    return {configured:!!citizenWork,ready:citizenWorkReady(),
      samples:[...citizenWorkEvents].map(([name,cue])=>({name,ready:takeoffBuffers.has(cue.sha256)})),
      voices:citizenWork?.voices()??[],pitchProfile:'nearest',
      pitchScope:'Process/module work-cue stream; existing legacy guided and flight streams remain independent.'};
  }
  function bindTakeoffEvents(events){
    takeoffEvents=events??new Map();
    for(const event of takeoffEvents.values())loadVerifiedEffect(event);
  }
  function preloadPriestCues(){
    if(!enabled||!priestCues)return;
    for(const cue of Object.values(PRIEST_CUES))loadVerifiedEffect({...cue,url:new URL(cue.file,base)});
  }
  function bindPriestCues(active){
    priestCues=active;
    preloadPriestCues();
  }
  function bindGuidedImpact(event){
    guidedImpact=event;
    if(enabled)loadVerifiedEffect(event);
  }
  function resetGuidedImpact(){
    for(const source of [...live])if(source.originalGroup==='guided-impact')stop(source);
  }
  function playGuidedImpact(volume,pan){
    if(!enabled||!guidedImpact||muted||ctx?.state!=='running')return false;
    const buffer=takeoffBuffers.get(guidedImpact.sha256);
    if(!buffer)return false;
    const same=[...live].filter(source=>source.originalGroup==='guided-impact');
    if(same.length>=guidedImpact.maxInstances)stop(same[0]);
    if(!originalOneShotAudible(volume))return true;
    guidedPitchSeed=(Math.imul(guidedPitchSeed,1664525)+1013904223)>>>0;
    const rate=playbackRate((guidedPitchSeed>>>16)&0x7fff,guidedImpact);
    if(!start(buffer,volume*guidedImpact.gain,pan,'guided-impact',false,0,rate))return false;
    stats.effectPlays++;
    return true;
  }
  function playPriestCue(kind,volume,pan){
    const cue=priestCues&&PRIEST_CUES[kind];
    if(!cue||!ctx||ctx.state!=='running'||!originalOneShotAudible(volume))return false;
    const buffer=takeoffBuffers.get(cue.sha256);
    if(!buffer)return false;
    const now=ctx.currentTime;
    const matching=[...live].filter(source=>source.originalEffect===kind);
    if(matching.length>=cue.maximumInstances){
      let oldest;
      for(const source of matching){
        if(source.originalStartedAt<now&&(!oldest||source.originalStartedAt<oldest.originalStartedAt))oldest=source;
      }
      if(!oldest)return true;
      stop(oldest);
    }
    const source=start(buffer,volume*cue.volume,pan,'effect');
    if(source){source.originalEffect=kind;source.originalStartedAt=now;stats.effectPlays++;}
    return !!source;
  }
  async function playInterfaceCue(event,pan=0){
    if(!enabled||muted||!Number.isFinite(event?.gain))return false;
    const buffer=event.preloadedOnly?takeoffBuffers.get(event.sha256):await loadVerifiedEffect(event);
    const maximum=event.maximumInstances;
    const group=Number.isInteger(maximum)&&maximum>0?'verified:'+event.sha256:'effect';
    if(group!=='effect'&&[...live].filter(source=>source.originalGroup===group).length>=maximum)return false;
    const played=!!start(buffer,event.gain,pan,group);
    if(played)stats.effectPlays++;
    return played;
  }
  function stopTakeoff(){
    for(const source of [...live])if(source.originalGroup.startsWith('takeoff:'))stop(source);
  }
  function playTakeoff(name,volume,pan){
    const event=takeoffEvents.get(name);
    if(!event||volume<=.01)return false;
    const group='takeoff:'+event.sha256;
    if([...live].filter(source=>source.originalGroup===group).length>=event.maximumInstances)return false;
    return !!start(takeoffBuffers.get(event.sha256),volume*event.volume,pan,group);
  }
  function updateStoneThrowerWheels(session, tracks, audible=volume=>volume>=.02) {
    const event=manifest?.nativeStoneThrowerEffects?.events?.fx_move_wheeled_04;
    if(muted||ctx?.state!=='running'||!session||!tracks?.length||!event?.loop||!event.file
      ||!Number.isInteger(event.maxInstances)||event.maxInstances<1||!Number.isFinite(event.volume)){
      stopStoneThrowerWheels();
      return;
    }
    if(wheelSession!==session)stopStoneThrowerWheels();
    wheelSession=session;
    const wanted=new Map(tracks.filter(track=>audible(track.volume)).map(track=>[track.handle,track]));
    for(const [handle,row] of wheelLoops){
      if(wanted.get(handle)?.clip===row.clip)continue;
      stop(row.source);
      wheelLoops.delete(handle);
    }
    const buffer=buffers.get(event.file);
    if(!buffer){if(wanted.size)load(event.file);return;}
    for(const track of [...wanted.values()].sort((a,b)=>b.volume-a.volume||a.handle-b.handle)){
      const previous=wheelLoops.get(track.handle), volume=event.volume*track.volume;
      if(previous){previous.source.updateSpatialMix(volume,track.pan);continue;}
      if(wheelLoops.size>=event.maxInstances)continue;
      const source=start(buffer,volume,track.pan,'wheel',true);
      if(!source)continue;
      wheelLoops.set(track.handle,{clip:track.clip,source});
      const ended=source.onended;
      source.onended=()=>{
        ended();
        if(wheelLoops.get(track.handle)?.source===source)wheelLoops.delete(track.handle);
      };
    }
  }
  function configureGranaryLoop(event) {
    granaryLoops?.reset();
    granaryLoops=event?createGranaryLoopPlayback(event,{
      ready:()=>!muted&&ctx?.state==='running',buffer:file=>buffers.get(file),load,start,stop,
    }):null;
  }
  function configureOilRigLoop(active) {
    if (!enabled || !active) { oilRigLoops?.reset(); oilRigLoops=null; return; }
    oilRigLoops??=createOilRigLoopPlayback({
      ready:()=>!muted&&ctx?.state==='running',
      buffer:()=>takeoffBuffers.get(OIL_RIG_CUE.sha256),
      load:()=>loadVerifiedEffect(OIL_RIG_CUE),
      clock:()=>ctx.currentTime,start,stop,
    });
    return loadVerifiedEffect(OIL_RIG_CUE);
  }
  function stopVoices(includeUnitReplies=false) {
    for(const source of [...live])if(source.originalGroup==='voice'||
      includeUnitReplies&&UNIT_REPLIES.includes(source.originalGroup))stop(source);
  }
  function stopMusic(preserve=true) {
    musicGeneration++;
    if(musicSource&&preserve&&ctx)musicOffset+=Math.max(0,ctx.currentTime-musicStarted);
    stop(musicSource);musicSource=null;
    if(!preserve){musicOffset=0;musicBuffer=null;musicFile=null;musicPendingFile=null;}
  }
  function stopAmbient(){stop(ambientSource);ambientSource=null;}
  function resetWorldAmbience(){nativeAmbient?.reset();nativeAmbientOwnsMix=false;}
  function stopLegacyAmbient(){stopAmbient();for(const source of [...live])if(source.originalGroup==='ambient')stop(source);}
  function frameAmbient(frame){
    if(!ambientPresentation||!ctx||!soundBus||ctx.state!=='running')return;
    nativeAmbient??=createOriginalAmbientPlayback({presentation:ambientPresentation,context:ctx,destination:soundBus,random:options.presentationRandom});
    const owns=nativeAmbient.frame({...frame,running:frame.running&&ambienceEnabled&&!muted});
    if(owns&&!nativeAmbientOwnsMix)stopLegacyAmbient();
    nativeAmbientOwnsMix=owns;
  }
  function canRun(){return active&&!muted&&ctx?.state==='running';}
  function chooseTrack(epoch) {
    const period=epoch>=10?1:0;
    if(period!==previousPeriod){stopMusic(false);musicIndex=0;previousPeriod=period;}
    const list=(period?manifest.music?.late:manifest.music?.west)?.filter(file=>!failed.has(file));
    return list?.length?list[musicIndex%list.length]:null;
  }
  function advanceMusic() {
    if(!manifest||!canRun()||!musicEnabled||musicSource)return;
    const file=chooseTrack(currentEpoch);
    if(!file)return;
    if(musicFile!==file){stopMusic(false);musicFile=file;}
    if(!musicBuffer){
      if(musicPendingFile===file)return;
      musicPendingFile=file;
      const generation=musicGeneration;
      load(file).then(buffer=>{if(musicPendingFile===file)musicPendingFile=null;if(generation===musicGeneration&&musicFile===file&&buffer){musicBuffer=buffer;advanceMusic();}});
      return;
    }
    if(musicOffset>=musicBuffer.duration){musicOffset=0;musicIndex++;musicFile=null;musicBuffer=null;advanceMusic();return;}
    musicSource=start(musicBuffer,.32,0,'music',false,musicOffset);musicStarted=ctx.currentTime;
    if(musicSource){
      stats.musicStarts++;const source=musicSource;
      source.onended=()=>{live.delete(source);source.originalCleanup();if(source===musicSource){musicSource=null;musicOffset=0;musicBuffer=null;musicFile=null;musicIndex++;advanceMusic();}};
    }
  }
  function advanceAmbient() {
    if(!ambienceEnabled||!canRun()||!manifest)return;
    const file=manifest.ambient?.loop;
    if(!ambientSource&&file){
      const buffer=buffers.get(file);
      if(buffer){ambientSource=start(buffer,.12,0,'ambient',true);if(ambientSource)stats.ambientStarts++;}
      else load(file);
    }
    if(ctx.currentTime>=nextBird){
      nextBird=ctx.currentTime+8+Math.random()*11;
      const birds=manifest.ambient?.birds||[];
      const bird=birds[Math.floor(Math.random()*birds.length)];
      const buffer=buffers.get(bird);
      if(buffer)start(buffer,.11,(Math.random()-.5)*1.4,'ambient');else load(bird);
    }
  }
  async function prepareFamilyBindings(){
    if(!familyManifest){try{const response=await fetchFile(new URL(familyManifestURL,base),{cache:'no-cache'});if(response.ok)familyManifest=await response.json();}catch{}}
    bindFamilies();
    for(const file of [...typed.usedFiles(),...(citizenPresentation?.usedFiles()||[])])load(file);
  }
  function prepareInfantryReplies(){
    if(!infantryRepliesEnabled)return null;
    infantryLoading??=loadInfantryReplyCatalog(fetchFile,base).then(catalog=>{
      infantryReplies=catalog;
      for(const record of catalog.records)loadVerifiedEffect(record);
    }).catch(()=>{});
    return infantryLoading;
  }
  async function prepare(context,destination,musicDestination=destination) {
    if(!enabled||typeof context?.decodeAudioData!=='function')return false;
    ctx=context;soundBus=destination;musicBus=musicDestination;
    preloadPriestCues();
    loadVerifiedEffect(guidedImpact);
    if(oilRigLoops)loadVerifiedEffect(OIL_RIG_CUE);
    if(citizenWork)void citizenWork.preload();
    for(const event of takeoffEvents.values())loadVerifiedEffect(event);
    if(loading)return loading;
    const readManifest=async file=>{
      const response=await fetchFile(new URL(file,base),{cache:'no-cache'});
      if(!response.ok)throw Error('No local originals');
      const data=await response.json();
      if(data.version!==1||!data.effects||!data.voices)throw Error('Unsupported local audio manifest');
      return data;
    };
    const file=playbackManifestURL;
    loading=readManifest(file).catch(error=>{if(file==='manifest.json')throw error;return readManifest('manifest.json');})
      .then(async data=>{
        manifest=data;
        const warm=[...[...Object.values(data.voices),...(data.voiceVariants||[])].flatMap(voice=>[...(voice.select||[]),...(voice.order||[]),...(voice.attack||[])]),...Object.values(data.effects).map(x=>x.file),...Object.values(data.buildingSelections||{}).map(x=>typeof x==='string'?x:x.file),data.ambient?.loop,...(data.ambient?.birds||[])];
        for(const file of new Set(warm))load(file);
        await Promise.all([prepareFamilyBindings(),prepareInfantryReplies()]);
        return true;
      }).catch(()=>false);
    return loading;
  }
  function playBinding(plan, volume=1, pan=0) {
    if (!plan || plan.group === 'voice' && !speech) return false;
    const index = counters.get(plan.key) || 0;
    const record = plan.records[index % plan.records.length];
    const buffer = buffers.get(record.file);
    if (!buffer) { load(record.file); return false; }
    if (plan.group === 'voice') stopVoices();
    if (!start(buffer, volume * plan.gain * record.volume, pan, plan.group)) return false;
    counters.set(plan.key, index + 1);
    if (plan.group === 'voice') stats.voicePlays++;
    else stats.effectPlays++;
    return true;
  }
  function playUnitReply(plan,scope,group,verified=false) {
    if(!pickUnitReply||!scope||!manifest||muted||!speech||ctx?.state!=='running'||!plan?.records.length)return false;
    const buffer=record=>verified?takeoffBuffers.get(record.sha256):buffers.get(record.file);
    const missing=plan.records.filter(record=>!buffer(record));
    if(missing.length){for(const record of missing){if(verified)loadVerifiedEffect(record);else load(record.file);}return false;}
    const selected=pickUnitReply(plan,scope);
    if(selected===null)return true;
    if(!selected||!start(buffer(selected),plan.gain*selected.volume,0,group))return false;
    stats.voicePlays++;
    return true;
  }
  function playInfantryReply(def,action,epoch,scope) {
    if(!infantryRepliesEnabled)return false;
    return playUnitReply(infantryReplies?.plan(def,action,epoch),scope,'infantry-reply',true);
  }
  function playCitizenReply(def,action,epoch,context,scope) {
    if(!['select','order'].includes(action)||!citizenPresentation?.isSourceContext(def,context))return false;
    const plan=citizenPresentation.resolveAudio(def,action,epoch,context);
    return playUnitReply(plan,scope,'citizen-reply');
  }
  function playSubmarineReply(def,action,epoch,scope) {
    if(!scope)return false;
    const stagedEarly=scope.submarineStage===10&&def===34&&epoch===10;
    if(!stagedEarly&&(def!==36||(epoch!==12&&epoch!==13)))return false;
    if(!['select','order','attack'].includes(action))return true;
    const voice=manifest?.voices[String(def)];
    const visual=stagedEarly?'Submarine_11':'Submarine_13';
    const files=voice?.source_visual===visual?voice[action]:null;
    if(!files?.length)return false;
    const records=files.map(file=>({file,volume:manifest.assets?.[file.toLowerCase()]?.sound_records?.[0]?.volume}));
    if(records.some(record=>record.volume!==1))return false;
    return playUnitReply({sourceIdentity:36,action,records,gain:1},scope,'submarine-reply');
  }
  function citizenDeath(context, volume, pan) {
    if (!manifest || muted || ctx?.state !== 'running') return false;
    return playBinding(citizenPresentation?.resolveAudio(8, 'death', context.epoch, context), volume, pan);
  }
  function acknowledge(def,action,epoch=0,context=undefined) {
    if(!manifest||muted||ctx?.state!=='running')return false;
    const sourceCitizen=citizenPresentation?.isSourceContext(def,context);
    const plan=sourceCitizen?citizenPresentation.resolveAudio(def,action,epoch,context):typed.resolve(def,action,epoch);
    if(plan)return playBinding(plan);
    if(sourceCitizen||!typed.isLegacy(def))return false;
    if(action==='select'&&manifest.buildingSelections?.[def]){
      const record=manifest.buildingSelections[def];
      const file=typeof record==='string'?record:record.file;const buffer=buffers.get(file);
      if(!buffer){load(file);return false;}
      start(buffer,record.volume??.9,0,'effect');stats.effectPlays++;return true;
    }
    if(!speech)return false;
    let voice=manifest.voices[String(def)],variantEpoch=-1;
    for(const row of manifest.voiceVariants||[])if(row.id===def&&row.epoch<=epoch&&epoch<row.until&&row.epoch>variantEpoch){voice=row;variantEpoch=row.epoch;}
    const list=voice?.[action]||voice?.order;
    if(!list?.length)return false;
    const key=def+':'+variantEpoch+':'+action,index=counters.get(key)||0,file=list[index%list.length];
    const buffer=buffers.get(file);
    if(!buffer){load(file);return false;}
    stopVoices();start(buffer,.75,0,'voice');counters.set(key,index+1);stats.voicePlays++;return true;
  }
  function effect(kind,volume=1,pan=0) {
    if(!manifest||muted||ctx?.state!=='running')return false;
    const record=manifest.effects[String(kind)];if(!record)return false;
    const buffer=buffers.get(record.file);if(!buffer){load(record.file);return false;}
    if (record.maxInstances>0 && [...live].filter(source=>source.originalEffect===kind).length>=record.maxInstances) return true;
    const source=start(buffer,volume*(record.volume??1),pan,'effect');
    if(source)source.originalEffect=kind;
    stats.effectPlays++;return true;
  }
  function stoneThrowerLaunch(volume,pan) {
    const event=manifest?.nativeStoneThrowerEffects?.events?.Trebuchet_firing1;
    if (!event || muted || ctx?.state!=='running') return false;
    const buffer=buffers.get(event.file);
    if (!buffer) { if (originalOneShotAudible(volume)) load(event.file); return false; }
    const activeShots=[...live].filter(source=>source.originalEffect===STONE_THROWER_LAUNCH);
    if (event.maxInstances>0 && activeShots.length>=event.maxInstances) stop(activeShots[0]);
    if (originalOneShotAudible(volume)) {
      const source=start(buffer,volume*event.volume,pan,'effect');
      if (source) { source.originalEffect=STONE_THROWER_LAUNCH;stats.effectPlays++; }
    }
    return true;
  }
  function regionalPlayback() {
    if (!regionalMusicEnabled || !ctx || !musicBus) return null;
    if (!regionalMusic) {
      regionalMusic = createRegionalMusicPlayback(ctx, musicBus, {
        fetchFile, random: options.presentationRandom, onStart: () => stats.musicStarts++,
      });
    }
    return regionalMusic;
  }
  function menuMusic(enabledMusic=musicEnabled) {
    active = false;
    musicEnabled = enabledMusic;
    stopAmbient();
    stopStoneThrowerWheels();
    granaryLoops?.reset();
    oilRigLoops?.reset();
    citizenWork?.reset();
    stopTakeoff();
    if (musicFile) stopMusic(false);
    return regionalPlayback()?.update({mode: 'menu', running: !muted, enabled: musicEnabled}) ?? false;
  }
  function music(running,epoch=0,enabledMusic=musicEnabled,civilization=0) {
    active=running;musicEnabled=enabledMusic;currentEpoch=epoch;
    if(!manifest)return false;
    const regional = regionalPlayback()?.update({running:canRun(), enabled:musicEnabled, civilization, epoch});
    if(!canRun()){stopMusic();stopAmbient();return true;}
    if (regional) {
      if (musicFile) stopMusic(false);
    } else if(!musicEnabled)stopMusic();else advanceMusic();
    if(!nativeAmbientOwnsMix)advanceAmbient();
    if (regional) return true;
    const tracks=currentEpoch>=10?manifest.music?.late:manifest.music?.west;
    return !musicEnabled||!!tracks?.some(file=>!failed.has(file));
  }
  return {
    prepare,acknowledge,playCitizenReply,playSubmarineReply,playInfantryReply,citizenDeath,effect,stoneThrowerLaunch,music,menuMusic,setDefinitionBindings,updateStoneThrowerWheels,stopStoneThrowerWheels,
    configureGranaryLoop,
    updateGranaryLoops(world,tracks,audible){granaryLoops?.update(world,tracks,audible);},
    stopGranaryLoops(){granaryLoops?.reset();},
    configureOilRigLoop,
    updateOilRigLoops(world,tracks,view){oilRigLoops?.update(world,tracks,view);},
    stopOilRigLoops(){oilRigLoops?.reset();},
    oilRigAudioStatus(){return oilRigLoops?.active()??[];},
    configureCitizenWork,citizenWorkReady,citizenWorkAudioStatus,
    preloadCitizenWork(){return citizenWork?.preload()??Promise.resolve([]);},
    updateCitizenWork(session,view){citizenWork?.update(session,view);},
    playCitizenWork(name,actor,view){return citizenWork?.play(name,actor,view)??false;},
    resetCitizenWork(){citizenWork?.reset();},
    bindPriestCues,playPriestCue,bindGuidedImpact,playGuidedImpact,resetGuidedImpact,
    bindTakeoffEvents,playTakeoff,stopTakeoff,prepareVerifiedEffect:loadVerifiedEffect,playInterfaceCue,frameAmbient,resetWorldAmbience,
    setAmbientPresentation(presentation){resetWorldAmbience();nativeAmbient=null;ambientPresentation=presentation;},
    role:(id,epoch)=>typed.role(id,epoch),
    resetRegionalMusic(){regionalMusic?.reset();},
    rebindRegionalMusic(){regionalMusic?.rebind();},
    setMuted(value){muted=value;if(value){regionalMusic?.suspend();stopMusic();stopAmbient();resetWorldAmbience();stopStoneThrowerWheels();granaryLoops?.reset();oilRigLoops?.reset();citizenWork?.reset();for(const source of [...live])stop(source);}},
    setSpeechEnabled(value){speech=value;if(!value)stopVoices(true);},
    setMusicEnabled(value){musicEnabled=value;if(!value){regionalMusic?.suspend();stopMusic();}},
    setAmbienceEnabled(value){ambienceEnabled=value;if(!value){stopLegacyAmbient();resetWorldAmbience();}},
    status(){const regional=regionalMusic?.status()??null;return {typedAudio:typed.status(),citizens:citizenPresentation?.status()||null,enabled,ready:!!manifest,buffers:buffers.size,failed:[...failed],activeVoices:[...live].filter(x=>x.originalGroup==='voice'||UNIT_REPLIES.includes(x.originalGroup)).length,music:!!musicSource||!!regional?.active,regionalMusic:regional,ambient:!!ambientSource,worldAmbience:nativeAmbient?.status()??null,...stats};}
  };
}
