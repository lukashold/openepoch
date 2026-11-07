const PREFIX = 'local-original/koth/';
const CATALOG = `${PREFIX}manifest.json`;
const CATALOG_SHA = '59e8a1322cfcf6e51905f043c0b887a2e56c97fc4e077f0959ccff7a4ca29cc7';
const SOURCE_MODEL_SHA = '7f949809ca58ac83c907d156dc4a3089f083a11b49dc94aabfef9e428d28fd3b';
const CUE = 'local-original/audio/Building_select.mp3';
const CUE_SHA = 'e25cb0a14c057f20d40fd32566a481927b8751f191b19622beb169cd67792646';
const SCALE = 0.02828427031636238;
const YAW = -1.5707963705062866;
const FULLMAP = 'local-original/fullmap-native/packed7/spr_fullmap_building.png';
const FAMILY = 'KingOfTheHill';

const CAPTURE = Object.freeze({
  self: {token:'msg_ingame_capture_KOTH_self', cue:'ui_capture_KOTH_self', spatial:false},
  other: {token:'msg_ingame_capture_KOTH_other', cue:'ui_capture_KOTH_other', spatial:false},
  others: {token:'msg_ingame_capture_KOTH_others', cue:'ui_capture_KOTH_others', spatial:true},
});

function asset(row,url) {
  return row?.url===url&&/^[a-f0-9]{64}$/.test(row.sha256)&&Number.isSafeInteger(row.bytes)&&row.bytes>0;
}

export async function loadKingOfTheHillPresentation({enabled,definition,profile='packed7',fetchFile=globalThis.fetch,
  bitmap=globalThis.createImageBitmap,base=import.meta.url}={}) {
  if(!enabled||!Number.isInteger(definition)||definition<0||definition>=65535||!globalThis.crypto?.subtle)return null;
  const read=async(url,digest)=>{
    if(!/^local-original\/[\w./-]+$/.test(url)||url.split('/').includes('..')||!(/^[a-f0-9]{64}$/).test(digest)){
      throw Error('Invalid original hill media reference');
    }
    const response=await fetchFile(new URL(url,base),{cache:'no-store'});
    if(!response.ok)throw Error('Original hill media unavailable');
    const data=await response.arrayBuffer();
    const sha=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',data)),n=>n.toString(16).padStart(2,'0')).join('');
    if(sha!==digest)throw Error('Original hill media hash differs');
    return data;
  };
  try {
    const source=JSON.parse(new TextDecoder().decode(await read(CATALOG,CATALOG_SHA)));
    if(source?.format!=='ee2-native-koth-presentation-v1'||source.sourceFamily!==FAMILY||
       source.visual?.defaultModel!=='amb_obelisk.nif'||source.visual.scaleType!=='kScaleBuilding'||
       source.visual.sourceScale!==SCALE||source.visual.yawOffset!==YAW||source.visual.state!=='Idle'||
       !asset(source.model,`${PREFIX}models/amb_obelisk.json`)||source.model.sourceSha256!==SOURCE_MODEL_SHA||
       source.textures?.length!==1||!asset(source.textures[0],`${PREFIX}textures/amb_obelisk.png`)||
       !asset(source.fullmap,FULLMAP)||!asset(source.captureAudio,CUE)||source.captureAudio.sha256!==CUE_SHA) return null;
    for(const [key,width] of [['packed5',64],['packed7',256]]){
      const row=source.icons?.[key];
      if(!asset(row,`${PREFIX}icons/${key}/icon_unit_default_building.png`)||row.width!==width||row.height!==width)return null;
    }
    const label=token=>source.labels?.[token]?.text??null;
    if(!['tx_utn_koth_name','tx_utn_koth_pname','tx_ig_vict_kingofthehill','tx_ig_vict_kingofthehill_desc',
      'vtt_unit_koth','tx_ig_koth_status','msg_ingame_capture_KOTH_w_self',...Object.values(CAPTURE).map(row=>row.token)]
      .every(token=>typeof label(token)==='string')) return null;
    const row=source.icons[profile]??source.icons.packed7;
    let image=null;
    if(bitmap){
      try{
        image=await bitmap(new Blob([await read(row.url,row.sha256)]));
        if(image.width!==row.width||image.height!==row.height){image.close?.();image=null;}
      }catch{/* A missing portrait leaves the existing UI fallback. */}
    }
    const modelRows=[{id:definition,assetKind:0,epoch:0,until:15,sourceFamily:FAMILY,kothObjective:true,
      url:source.model.url,sha256:source.model.sha256,source:{sha256:SOURCE_MODEL_SHA},verified:true,yawOffset:YAW,
      nativeScaleBinding:{verified:true,scaleType:'kScaleBuilding',sourceScale:SCALE,sourceSha256:SOURCE_MODEL_SHA}}];
    const textureHashes=new Map(source.textures.map(({url,sha256})=>[url,sha256]));
    return Object.freeze({modelRows,textureHashes,fullmap:source.fullmap,
      icon:id=>id===definition?image:null,
      name:(id,_epoch,count=1)=>id===definition?label(count>1?'tx_utn_koth_pname':'tx_utn_koth_name'):undefined,
      description:id=>id===definition?label('vtt_unit_koth'):undefined,
      modeName:label('tx_ig_vict_kingofthehill'),modeDescription:label('tx_ig_vict_kingofthehill_desc'),label,
      capture:(audience,{fromWorld=false}={})=>{
        if(!Object.hasOwn(CAPTURE,audience))return null;
        const cue=CAPTURE[audience];
        return {message:label(audience==='self'&&fromWorld?'msg_ingame_capture_KOTH_w_self':cue.token),
          sound:{name:cue.cue,url:CUE,sha256:CUE_SHA,volume:1,loop:false,spatial:cue.spatial}};
      },
      readAsset:record=>read(record.url,record.sha256),
    });
  }catch{
    return null;
  }
}
