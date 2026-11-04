// Playback of original assets extracted from the user's local EE2 installation.
// Metadata and byte-for-byte source hashes live beside those assets. This module
// contains no recordings and remains usable when local-original is absent.
const DEFAULT_BASE=new URL('./local-original/audio/',import.meta.url);

export function createOriginalAudio(options={}) {
  const enabled=options.enabled ?? (typeof window!=='undefined' && new URLSearchParams(window.location?.search||'').get('rules')==='ee2');
  const fetchFile=options.fetch || ((...args)=>fetch(...args));
  const base=options.base || DEFAULT_BASE;
  let ctx=null,master=null,manifest=null,loading=null,muted=false,speech=true,musicEnabled=true,active=false;
  const buffers=new Map(),pending=new Map(),failed=new Set(),live=new Set(),counters=new Map();
  let musicSource=null,musicBuffer=null,musicFile=null,musicOffset=0,musicStarted=0,musicIndex=0,musicGeneration=0,musicPendingFile=null;
  let ambientSource=null,nextBird=0,currentEpoch=0,previousPeriod=-1;
  const stats={voicePlays:0,effectPlays:0,musicStarts:0,ambientStarts:0};

  function load(file) {
    if(!file||!ctx||failed.has(file))return Promise.resolve(null);
    if(buffers.has(file))return Promise.resolve(buffers.get(file));
    if(pending.has(file))return pending.get(file);
    const url=new URL(file,base),hash=manifest?.assets?.[file.toLowerCase()]?.sha256;
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
    let output=gain;
    if(ctx.createStereoPanner){const p=ctx.createStereoPanner();p.pan.value=Math.max(-1,Math.min(1,pan));gain.connect(p);output=p;}
    source.connect(gain);output.connect(master);source.originalGroup=group;live.add(source);
    source.originalCleanup=()=>{try{gain.disconnect();if(output!==gain)output.disconnect();}catch{}};
    source.onended=()=>{live.delete(source);source.originalCleanup();};
    source.start(0,Math.max(0,offset));return source;
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
    if(!canRun()||!manifest)return;
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
  async function prepare(context,destination) {
    if(!enabled||typeof context?.decodeAudioData!=='function')return false;
    ctx=context;master=destination;
    if(loading)return loading;
    loading=fetchFile(new URL('manifest.json',base),{cache:'no-cache'})
      .then(r=>{if(!r.ok)throw Error('No local originals');return r.json();})
      .then(data=>{
        if(data.version!==1||!data.effects||!data.voices)throw Error('Unsupported local audio manifest');
        manifest=data;
        const warm=[...[...Object.values(data.voices),...(data.voiceVariants||[])].flatMap(voice=>[...(voice.select||[]),...(voice.order||[]),...(voice.attack||[])]),...Object.values(data.effects).map(x=>x.file),...Object.values(data.buildingSelections||{}).map(x=>typeof x==='string'?x:x.file),data.ambient?.loop,...(data.ambient?.birds||[])];
        for(const file of new Set(warm))load(file);
        return true;
      }).catch(()=>false);
    return loading;
  }
  function acknowledge(def,action,epoch=0) {
    if(!manifest||muted||ctx?.state!=='running')return false;
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
    start(buffer,volume*(record.volume??1),pan,'effect');stats.effectPlays++;return true;
  }
  function music(running,epoch=0,enabledMusic=musicEnabled) {
    active=running;musicEnabled=enabledMusic;currentEpoch=epoch;
    if(!manifest)return false;
    if(!canRun()){stopMusic();stopAmbient();return true;}
    if(!musicEnabled)stopMusic();else advanceMusic();
    advanceAmbient();
    const tracks=currentEpoch>=10?manifest.music?.late:manifest.music?.west;
    return !musicEnabled||!!tracks?.some(file=>!failed.has(file));
  }
  return {
    prepare,acknowledge,effect,music,
    setMuted(value){muted=value;if(value){stopMusic();stopAmbient();for(const source of [...live])stop(source);}},
    setSpeechEnabled(value){speech=value;if(!value)stopVoices();},
    setMusicEnabled(value){musicEnabled=value;if(!value)stopMusic();},
    status(){return {enabled,ready:!!manifest,buffers:buffers.size,failed:[...failed],activeVoices:[...live].filter(x=>x.originalGroup==='voice').length,music:!!musicSource,ambient:!!ambientSource,...stats};}
  };
}
