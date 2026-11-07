const catalogs={
  'imperial-construction':'abd3fa9c75c0256a4edcacc976f85a59f4b3f4dacc8efa117bc54a0ac91f3737',
  mobilization:'17c0775a526dd3bdef596abae7133d9ec836d30401e902134bec8fbf40e21eb6',
};

export async function loadLeaderCursors(enabled,group){
  if(!enabled||!Object.hasOwn(catalogs,group))return null;
  try{
    const response=await fetch(new URL(`./local-original/${group}/manifest.json`,import.meta.url));
    if(!response.ok)return null;
    const bytes=await response.arrayBuffer();
    const digest=await crypto.subtle.digest('SHA-256',bytes);
    const hash=Array.from(new Uint8Array(digest),byte=>byte.toString(16).padStart(2,'0')).join('');
    if(hash!==catalogs[group])return null;
    const source=JSON.parse(new TextDecoder().decode(bytes));
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
