import {createTypedAudioBindings} from './original-audio-bindings.js?v=native8';
import {createRegionalMusicPlayback} from './original-regional-music.js?v=regional12';
import {originalOneShotAudible} from './original-spatial-audio.js?v=native39audio';
// Playback of original assets extracted from the user's local EE2 installation.
// Metadata and byte-for-byte source hashes live beside those assets. This module
// contains no recordings and remains usable when local-original is absent.
const DEFAULT_BASE=new URL('./local-original/audio/',import.meta.url);
const STONE_THROWER_LAUNCH=26;

export function createOriginalAudio(options={}) {
  const enabled=options.enabled ?? (typeof window!=='undefined' && new URLSearchParams(window.location?.search||'').get('rules')==='ee2');
  const fetchFile=options.fetch || ((...args)=>fetch(...args));
  const base=options.base || DEFAULT_BASE;
  let ctx=null,soundBus=null,musicBus=null,manifest=null,loading=null,muted=false,speech=true,musicEnabled=true,ambienceEnabled=true,active=false;
  const buffers=new Map(),pending=new Map(),failed=new Set(),live=new Set(),counters=new Map();
  const takeoffBuffers=new Map(),takeoffLoads=new Map();
  let takeoffEvents=new Map();
  const wheelLoops=new Map();
  let wheelSession=null;
  let musicSource=null,musicBuffer=null,musicFile=null,musicOffset=0,musicStarted=0,musicIndex=0,musicGeneration=0,musicPendingFile=null;
  let ambientSource=null,nextBird=0,currentEpoch=0,previousPeriod=-1;
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
          if(Object.keys(added).some(name=>Object.hasOwn(inherited,name)))throw Error('Overlapping native sound family');
          combined[key]={...inherited,...added};
        }
        typed=createTypedAudioBindings({...definitionSettings,familyManifest:combined});
        selectedFamilyManifest=combined;
      }catch{/* A missing or inconsistent optional family keeps existing voices. */}
    }
  }
  function setDefinitionBindings({wasm=null,bindings=[],familyManifestURL:url=familyManifestURL,
    citizenPresentation:citizens=citizenPresentation,regionalMusic:useRegionalMusic=regionalMusicEnabled,
    playbackManifestURL:playbackURL=playbackManifestURL,additionalFamilyManifest:additional=null}={}) {
    if(url!==familyManifestURL){familyManifest=null;familyManifestURL=url;}
    citizenPresentation=citizens;
    regionalMusicEnabled=useRegionalMusic;
    playbackManifestURL=playbackURL;
    if (!regionalMusicEnabled) regionalMusic?.reset();
    definitionSettings={wasm,bindings};
    additionalFamilyManifest=additional;
    bindFamilies();
    if(manifest)prepareFamilyBindings();
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
  function start(buffer,volume=1,pan=0,group='effect',loop=false,offset=0) {
    if(!buffer||muted||!ctx||ctx.state!=='running')return null;
    if(group==='effect'&&live.size>32)return null;
    const source=ctx.createBufferSource();source.buffer=buffer;source.loop=loop;
    const gain=ctx.createGain();gain.gain.value=Math.max(0,Math.min(1,volume));
    const panner=ctx.createStereoPanner?.();
    const output=panner??gain;
    if(panner){panner.pan.value=Math.max(-1,Math.min(1,pan));gain.connect(panner);}
    source.connect(gain);output.connect(group==='music'?musicBus:soundBus);source.originalGroup=group;
    if(group==='wheel'){
      source.updateSpatialMix=(volume,pan)=>{
        gain.gain.value=Math.max(0,Math.min(1,volume));
        if(panner)panner.pan.value=Math.max(-1,Math.min(1,pan));
      };
    }else live.add(source);
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
    const load=fetchFile(url,{cache:'force-cache'}).then(response=>{
      if(!response.ok)throw Error('Original audio unavailable');
      return response.arrayBuffer();
    }).then(async bytes=>{
      const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
      if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==event.sha256)throw Error('Original audio checksum differs');
      return ctx.decodeAudioData(bytes);
    }).then(buffer=>{takeoffBuffers.set(event.sha256,buffer);return buffer;}).catch(()=>null);
    takeoffLoads.set(event.sha256,load);
    return load;
  }
  function bindTakeoffEvents(events){
    takeoffEvents=events??new Map();
    for(const event of takeoffEvents.values())loadVerifiedEffect(event);
  }
  async function playInterfaceCue(event){
    if(!enabled||muted||!Number.isFinite(event?.gain))return false;
    const buffer=await loadVerifiedEffect(event);
    const played=!!start(buffer,event.gain);
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
  function stopVoices() {for(const source of [...live])if(source.originalGroup==='voice')stop(source);}
  function stopMusic(preserve=true) {
    musicGeneration++;
    if(musicSource&&preserve&&ctx)musicOffset+=Math.max(0,ctx.currentTime-musicStarted);
    stop(musicSource);musicSource=null;
    if(!preserve){musicOffset=0;musicBuffer=null;musicFile=null;musicPendingFile=null;}
  }
  function stopAmbient(){stop(ambientSource);ambientSource=null;}
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
  async function prepare(context,destination,musicDestination=destination) {
    if(!enabled||typeof context?.decodeAudioData!=='function')return false;
    ctx=context;soundBus=destination;musicBus=musicDestination;
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
        await prepareFamilyBindings();
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
    advanceAmbient();
    if (regional) return true;
    const tracks=currentEpoch>=10?manifest.music?.late:manifest.music?.west;
    return !musicEnabled||!!tracks?.some(file=>!failed.has(file));
  }
  return {
    prepare,acknowledge,citizenDeath,effect,stoneThrowerLaunch,music,menuMusic,setDefinitionBindings,updateStoneThrowerWheels,stopStoneThrowerWheels,
    bindTakeoffEvents,playTakeoff,stopTakeoff,playInterfaceCue,
    role:(id,epoch)=>typed.role(id,epoch),
    resetRegionalMusic(){regionalMusic?.reset();},
    rebindRegionalMusic(){regionalMusic?.rebind();},
    setMuted(value){muted=value;if(value){regionalMusic?.suspend();stopMusic();stopAmbient();stopStoneThrowerWheels();for(const source of [...live])stop(source);}},
    setSpeechEnabled(value){speech=value;if(!value)stopVoices();},
    setMusicEnabled(value){musicEnabled=value;if(!value){regionalMusic?.suspend();stopMusic();}},
    setAmbienceEnabled(value){ambienceEnabled=value;if(!value){stopAmbient();for(const source of [...live])if(source.originalGroup==='ambient')stop(source);}},
    status(){const regional=regionalMusic?.status()??null;return {typedAudio:typed.status(),citizens:citizenPresentation?.status()||null,enabled,ready:!!manifest,buffers:buffers.size,failed:[...failed],activeVoices:[...live].filter(x=>x.originalGroup==='voice').length,music:!!musicSource||!!regional?.active,regionalMusic:regional,ambient:!!ambientSource,...stats};}
  };
}
