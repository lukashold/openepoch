const PREFIX='local-original/capitols/';
const MANIFEST=`${PREFIX}manifest.json`;
const SHA='df0e7c2ebc0aa6b1bf5ae66502b85041fd42241affae283bc013bf0a50943d29';
const fallback=[
  'Du hast dein Kapitol von %ENEMY% zurückgewonnen.',
  '%PLAYER% hat dir sein Kapitol wieder abgenommen.',
  'Du hast ein Kapitol von %ENEMY% erobert und an %ALLY% zurückgegeben.',
  '%PLAYER% hat dir ein Kapitol abgenommen und es an %ALLY% zurückgegeben.',
  '%PLAYER% hat dein Kapitol von %ENEMY% zurückgeholt.',
  'Du hast das Kapitol von %ENEMY% eingenommen.',
  '%PLAYER% hat dein Kapitol eingenommen. Erobere es innerhalb von %MINUTES% Minuten zurück.',
];

export async function loadCapitolFeedback(enabled,fetchFile=globalThis.fetch,base=import.meta.url) {
  if(!enabled||!globalThis.crypto?.subtle)return null;
  try{
    const response=await fetchFile(new URL(MANIFEST,base));
    if(!response.ok)return null;
    const data=await response.arrayBuffer();
    const bytes=new Uint8Array(await crypto.subtle.digest('SHA-256',data));
    const digest=Array.from(bytes,value=>value.toString(16).padStart(2,'0')).join('');
    if(digest!==SHA)return null;
    const manifest=JSON.parse(new TextDecoder().decode(data));
    if(manifest.format!=='ee2-capitol-feedback-v1'||manifest.events.length!==fallback.length)return null;
    return manifest.events.map(({text,sound})=>({text,sound:{url:new URL(PREFIX+sound.file,base),
      sha256:sound.sha256,gain:sound.gain}}));
  }catch{return null;}
}

export function capitolFeedback(kind,seats,minutes,name,original) {
  const route=kind-148;
  if(!Number.isInteger(route)||!fallback[route])return null;
  const names={PLAYER:name(seats>>>4),ENEMY:name((seats>>>2)&3),ALLY:name(seats&3),MINUTES:minutes};
  const entry=original?.[route];
  const message=(entry?.text??fallback[route]).replace(/%(PLAYER|ENEMY|ALLY|MINUTES)%/g,(_,key)=>names[key]);
  return {message,sound:entry?.sound,lost:route===1||route===3||route===6};
}
