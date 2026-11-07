const catalogs={
  'imperial-construction':'abd3fa9c75c0256a4edcacc976f85a59f4b3f4dacc8efa117bc54a0ac91f3737',
  mobilization:'17c0775a526dd3bdef596abae7133d9ec836d30401e902134bec8fbf40e21eb6',
  'late-leader-powers':'a470344a86b5aa3dca55b521ea1ac1e92908d34c64c963efa81f5bb1d77635d2',
};

async function pinnedAsset(path,sha256){
  const url=new URL(path,import.meta.url);
  url.searchParams.set('v',sha256.slice(0,16));
  const response=await fetch(url);
  if(!response.ok)return null;
  const bytes=await response.arrayBuffer();
  const digest=await crypto.subtle.digest('SHA-256',bytes);
  const hash=Array.from(new Uint8Array(digest),byte=>byte.toString(16).padStart(2,'0')).join('');
  return hash===sha256?{url,bytes}:null;
}

export async function loadLeaderCursors(enabled,group){
  if(!enabled||!Object.hasOwn(catalogs,group))return null;
  try{
    const asset=await pinnedAsset(`./local-original/${group}/manifest.json`,catalogs[group]);
    if(!asset)return null;
    const source=JSON.parse(new TextDecoder().decode(asset.bytes));
    if(source.format!==`ee2-native-${group}-v1`)return null;
    return (action,valid)=>{
      const cursor=source.cursors[action]?.[valid?'valid':'invalid'];
      if(!cursor)return null;
      const url=new URL(cursor.url,import.meta.url);
      url.searchParams.set('v',cursor.sha256.slice(0,16));
      return `url("${url.href}") ${cursor.hotspot.join(' ')}, ${valid?'crosshair':'not-allowed'}`;
    };
  }catch{return null;}
}

export async function loadImperialismBadge(enabled){
  if(!enabled)return null;
  try{
    const asset=await pinnedAsset('./local-original/late-leader-powers/icon_leaderpwr_imperialism_s.png',
      'c0c0d3b7561d1533e5f4fb5d516e5ae194e3126ce94b6bbd8f6ea118a2792c29');
    return asset?.url.href??null;
  }catch{return null;}
}
