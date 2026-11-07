const manifestPath='./local-original/ungarrison/manifest.json';
const manifestSha='2384d518bb99862e5954580c99723d1f1b34399eaf52a955efb7b562df05aa21';
const states=['idle','hover','pressed'];

export async function loadUngarrisonArt(enabled){
  if(!enabled)return null;
  try{
    const response=await fetch(new URL(manifestPath,import.meta.url));
    if(!response.ok)return null;
    const bytes=await response.arrayBuffer();
    const digest=await crypto.subtle.digest('SHA-256',bytes);
    const sha=Array.from(new Uint8Array(digest),value=>value.toString(16).padStart(2,'0')).join('');
    if(sha!==manifestSha)return null;
    const source=JSON.parse(new TextDecoder().decode(bytes));
    return source.format==='ee2-native-ungarrison-v1'?source:null;
  }catch{return null;}
}

export function ungarrisonIcon(art,theme){
  const frames=art?.profiles.packed7[theme];
  if(!frames)return null;
  const icon=document.createElement('span');
  icon.className='ungarrison-icon';
  icon.setAttribute('aria-hidden','true');
  for(const state of states){
    const frame=frames[state],url=new URL(frame.url,import.meta.url);
    url.searchParams.set('v',frame.sha256.slice(0,16));
    icon.style.setProperty(`--ungarrison-${state}`,`url("${url.href}")`);
  }
  return icon;
}
