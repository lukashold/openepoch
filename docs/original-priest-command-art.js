const root = 'local-original/priest-commands/';
const checksum = 'da7eadac3d0082f19b75ec641594612be61938499cb7cea4ece52ceb37e4ca46';
const hash = value => /^[a-f0-9]{64}$/.test(value || '');

export async function loadPriestCommandArt({enabled, base=new URL('./',import.meta.url), fetchFile=globalThis.fetch}={}) {
  if(!enabled)return null;
  try {
    const response=await fetchFile(new URL(root+'manifest.json',base));
    if(!response.ok)return null;
    const bytes=await response.arrayBuffer();
    const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
    if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==checksum)return null;
    const catalog=JSON.parse(new TextDecoder().decode(bytes));
    if(catalog.format!=='ee2-native-priest-command-art-v1')return null;
    const urls=new Map();
    function asset(record) {
      if(!record?.url?.startsWith(root)||!/^[-\w./]+$/.test(record.url)||record.url.split('/').includes('..')||
          !hash(record.sha256))throw Error('Invalid Priest command sprite');
      const url=new URL(record.url,base);
      url.searchParams.set('v',record.sha256.slice(0,16));
      urls.set(record,`url("${url.href}")`);
    }
    for(const action of ['convert','bless']){
      const command=catalog.commands[action];
      for(const theme of [1,2,3])for(const state of ['idle','hover','pressed'])asset(command.buttons[theme][state]);
      for(const record of [command.cursors.valid,command.cursors.invalid]){
        if(record.hotspot?.length!==2||!record.hotspot.every(value=>value===2))return null;
        asset(record);
      }
    }
    const command=action=>Object.hasOwn(catalog.commands,action)?catalog.commands[action]:undefined;
    return Object.freeze({
      button:(action,theme,state)=>urls.get(command(action)?.buttons[theme]?.[state]),
      cursor(action,valid) {
        const record=command(action)?.cursors[valid?'valid':'invalid'];
        return record?`${urls.get(record)} ${record.hotspot.join(' ')}, ${valid?'crosshair':'not-allowed'}`:undefined;
      },
      label:action=>command(action)?.label.text,
      description:action=>command(action)?.description.text,
    });
  } catch {return null;}
}
