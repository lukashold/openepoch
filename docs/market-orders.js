const commodityNames=['Nahrung','Holz','Stein','Gold','Zinn','Eisen','Salpeter','Öl','Uran'];
const resourceKeys=['food','wood','stone','gold','tin','iron','saltpeter','oil','uranium'];
const actionKeys=['food','wood','stone','special1','special2'];
const hotkeys=['F','W','S','Z','Y'];
const positions=[[3,1],[2,1],[1,1],[2,3],[1,3]];
const marketManifest='local-original/market-ui/manifest.json';
const marketSha='efd54265fc495f719210e76ffea90852b1ea26e5c8b746fdceea2cb866dfee95';

export async function loadMarketArt(enabled){
  if(!enabled)return null;
  try{
    const response=await fetch(new URL(marketManifest,import.meta.url));
    if(!response.ok)return null;
    const bytes=await response.arrayBuffer();
    const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
    if(Array.from(digest,value=>value.toString(16).padStart(2,'0')).join('')!==marketSha)return null;
    const source=JSON.parse(new TextDecoder().decode(bytes));
    if(source.format!=='ee2-native-market-ui-v1')return null;
    return source;
  }catch{return null;}
}

function asset(record){
  if(!record)return '';
  const url=new URL(record.url,import.meta.url);
  url.searchParams.set('v',record.sha256.slice(0,16));
  return url.href;
}

export function marketOrders(wasm,host){
  if(typeof wasm.market_commodity!=='function'||host===0xffffffff)return [];
  const orders=[];
  for(let position=0;position<5;position++){
    const resource=wasm.market_commodity(position)>>>0;
    const name=commodityNames[resource];
    if(!name)continue;
    for(const side of [1,-1]){
      const normal=wasm.market_quote(host,resource,side),shift=wasm.market_quote(host,resource,side*5);
      if(normal&&shift)orders.push({host,position,resource,side,name,normal,shift});
    }
  }
  return orders;
}

export function marketOrderButton(order,submit,art,theme){
  const {host,position,resource,side,normal,shift}=order;
  const verb=side===1?'buy':'sell',resourceKey=resourceKeys[resource];
  const name=art?.resourceLabels[resourceKey]??order.name;
  const action=art?.actions[`unit_market_${verb}_${actionKeys[position]}`];
  const label=action?.label??(side===1?'Kaufen':'Verkaufen');
  const button=document.createElement('button');
  button.type='button';button.className='act market-action';button.classList.toggle('no',!(normal>>>16));
  button.dataset.marketResource=String(resource);button.dataset.marketSide=verb;
  const [row,column]=positions[position];
  button.style.gridRow=String(row);button.style.gridColumn=String(column+(side===1?0:1));
  button.title=`${label} 100 ${name} für ${normal&0xffff} Gold. Umschalt: 500 für ${shift&0xffff} Gold.${action?.help?' '+action.help:''}`;
  button.setAttribute('aria-label',button.title);
  button.disabled=!(normal>>>16);
  const caption=document.createElement('small');caption.textContent=`${name}\n${label}`;
  const profile=art?.profiles.packed7,faces=profile?.themes[theme]?.[verb];
  const glyph=asset(profile?.glyphs[resourceKey]);
  if(faces&&glyph){
    button.dataset.originalMarket='1';
    for(const state of ['idle','hover','pressed'])button.style.setProperty(`--market-${state}`,`url("${asset(faces[state])}")`);
    const image=document.createElement('img');image.src=glyph;image.alt='';button.append(image);
  }
  for(const [mode,value] of [['plain',normal],['shift',shift]]){
    const price=document.createElement('b');price.className=`market-price-${mode}`;
    price.textContent=`${value&0xffff}${faces&&glyph?'':' G'}`;button.append(price);
  }
  button.append(caption);
  button.onclick=event=>submit(host,resource,side*(event.shiftKey?5:1));
  return button;
}

export function marketOrderHotkey(wasm,host,event){
  if(event.altKey||event.metaKey)return null;
  const position=hotkeys.indexOf(event.key.toUpperCase());
  if(position<0)return null;
  const side=event.ctrlKey?-1:1;
  return marketOrders(wasm,host).find(order=>order.position===position&&order.side===side)??null;
}
