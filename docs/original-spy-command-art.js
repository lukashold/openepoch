const basic={root:'local-original/spy-commands/',checksum:'946b0edde9aa7591f7ae492d70fb9cf2878f2bddb6f0bc891b644e31713e0cbf',
  format:'ee2-native-spy-command-art-v1',actions:['gatherIntel','extraction'],extraction:true};
const advanced={root:'local-original/spy-advanced-commands/',checksum:'d8e9861e89890cf70d98c21aff3ebd198ab7dc9255872405f0bc4a0a86a2be4f',
  format:'ee2-native-spy-advanced-command-art-v1',actions:['sabotage','restore','contaminate','cleanup'],statuses:['sabotage','contamination']};
const hash=value=>/^[a-f0-9]{64}$/.test(value||'');

async function loadArt({root,checksum,format,actions,extraction,statuses=[]},
    {enabled,base=new URL('./',import.meta.url),fetchFile=globalThis.fetch}={}){
  if(!enabled)return null;
  try{
    const response=await fetchFile(new URL(root+'manifest.json',base));
    if(!response.ok)return null;
    const bytes=await response.arrayBuffer();
    const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
    if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==checksum)return null;
    const catalog=JSON.parse(new TextDecoder().decode(bytes));
    if(catalog.format!==format)return null;
    const urls=new Map();
    function asset(record){
      if(!record?.url?.startsWith(root)||!/^[-\w./]+$/.test(record.url)||record.url.split('/').includes('..')||
         !hash(record.sha256))throw Error('Invalid Spy command sprite');
      const url=new URL(record.url,base);
      url.searchParams.set('v',record.sha256.slice(0,16));
      urls.set(record,url.href);
    }
    for(const action of actions){
      const command=catalog.commands[action];
      for(const profile of ['packed5','packed7']){
        for(const theme of [1,2,3]){
          for(const state of ['idle','hover','pressed'])asset(command.buttons[profile][theme][state]);
        }
      }
      for(const record of Object.values(command.cursors)){
        if(record.hotspot?.length!==2||!record.hotspot.every(value=>value===2))return null;
        asset(record);
      }
    }
    for(const name of statuses){
      for(const profile of ['packed5','packed7'])asset(catalog.status[name][profile]);
    }
    if(statuses.length)asset(catalog.mixedGlow);
    let extractionVisuals;
    if(extraction){
      const {decal,timer}=catalog.extractionVisuals;
      asset(decal);
      extractionVisuals=Object.freeze({decal:Object.freeze({url:urls.get(decal),sha256:decal.sha256,radiusMode:decal.radiusMode}),
        timer:Object.freeze({radius:timer.radius,offset:timer.offset})});
    }
    const command=action=>Object.hasOwn(catalog.commands,action)?catalog.commands[action]:undefined;
    const css=record=>urls.has(record)?`url("${urls.get(record)}")`:undefined;
    return Object.freeze({
      button:(action,theme,state,profile='packed7')=>css(command(action)?.buttons[profile]?.[theme]?.[state]),
      cursor(action,valid){
        const key=action==='extraction'?'default':valid?'valid':'invalid';
        const record=command(action)?.cursors[key];
        const fallback=action==='extraction'?'default':valid?'crosshair':'not-allowed';
        return record?`${css(record)} ${record.hotspot.join(' ')}, ${fallback}`:undefined;
      },
      label:action=>command(action)?.label.text,
      description:action=>command(action)?.description.text,
      shortcut:action=>command(action)?.shortcut,
      targeted:action=>command(action)?.targeted,
      status:(name,profile='packed7')=>urls.get(catalog.status?.[name]?.[profile]),
      mixedGlow:urls.get(catalog.mixedGlow),
      extractionVisuals,
    });
  }catch{return null;}
}

export const loadSpyCommandArt=options=>loadArt(basic,options);
export const loadSpyAdvancedCommandArt=options=>loadArt(advanced,options);
