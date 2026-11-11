// Private installed assets only. No loading occurs unless both caller gates are
// explicitly true; event timing, actor transforms and audio playback stay outside.
import {compileWheatHarvest,createWheatHarvestRenderer} from './dv2-citizen-work-fx.mjs?v=work-events290-private';

export const CITIZEN_WORK_EVENT_ASSET_PINS=Object.freeze({
 manifest:Object.freeze({sha256:'bc2304a878295c2b4a086e0600f846f537fa40ee3fad8f78c470831a59e48dc2',length:5935}),
 mips:Object.freeze({sha256:'de93bf34f708701bd72c15ecd1dbc948eed358e66130910370a7e4d3863c4b95',length:4444}),
 sourceMetadata:Object.freeze({sha256:'b8799e977df813fbdcaa0d1e6eaa6c9160d28555bb810fa32cbef2245a9c69dc',length:18282}),
});
const require=(ok,message)=>{if(!ok)throw new Error('Citizen work private assets: '+message);};
const hex=buffer=>Array.from(new Uint8Array(buffer),byte=>byte.toString(16).padStart(2,'0')).join('');
const json=buffer=>JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(buffer));

// Texture upload restores every state it changes. All eight images are decoded
// from separate original compressed mip payloads; generateMipmap is never used.
function uploadTexture(gl,images){
 const oldActive=gl.getParameter(gl.ACTIVE_TEXTURE);
 gl.activeTexture(gl.TEXTURE0);
 const oldTexture=gl.getParameter(gl.TEXTURE_BINDING_2D);
 const stores=[gl.UNPACK_ALIGNMENT,gl.UNPACK_FLIP_Y_WEBGL,gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL,gl.UNPACK_COLORSPACE_CONVERSION_WEBGL]
  .map(key=>[key,gl.getParameter(key)]);
 let texture;
 try{
  texture=gl.createTexture();require(texture,'texture allocation failed');gl.bindTexture(gl.TEXTURE_2D,texture);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT,1);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL,false);gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL,gl.NONE);
  for(let level=0;level<images.length;level++)gl.texImage2D(gl.TEXTURE_2D,level,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,images[level]);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_BASE_LEVEL,0);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAX_LEVEL,7);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.REPEAT);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.REPEAT);
  return texture;
 }catch(error){if(texture)gl.deleteTexture(texture);throw error;}
 finally{
  for(const [key,value]of stores)gl.pixelStorei(key,value);
  gl.bindTexture(gl.TEXTURE_2D,oldTexture);gl.activeTexture(oldActive);
 }
}

export async function loadCitizenWorkEventAssets(gl,{
 enabled=false,privateMode=false,
 manifestUrl=new URL('./local-original/citizen-work-events/manifest.json',import.meta.url),
 fetchFile=globalThis.fetch,
 decodeImage=async bytes=>globalThis.createImageBitmap(new Blob([bytes],{type:'image/png'}),
  {imageOrientation:'none',premultiplyAlpha:'none',colorSpaceConversion:'none'}),
}={}){
 if(enabled!==true||privateMode!==true)return null;
 require(typeof fetchFile==='function'&&typeof decodeImage==='function','fetch/image decoder unavailable');
 const url=new URL(manifestUrl,import.meta.url),origin=new URL(import.meta.url).origin;
 require(url.origin===origin,'manifest must share module origin');
 require(url.pathname.endsWith('/local-original/citizen-work-events/manifest.json'),'unexpected private manifest path');
 const base=new URL('../../',url);
 const resolve=path=>{
  require(typeof path==='string'&&path.startsWith('local-original/'),'asset outside private directory');
  const resolved=new URL(path,base);require(resolved.origin===url.origin&&resolved.href.startsWith(new URL('local-original/',base).href),'asset URL escaped private directory');
  return resolved;
 };
 const verified=async(assetURL,pin)=>{
  require(/^[0-9a-f]{64}$/.test(pin?.sha256)&&Number.isSafeInteger(pin.length)&&pin.length>0,'invalid asset pin');
  const response=await fetchFile(assetURL,{credentials:'same-origin',cache:'no-store'});
  require(response?.ok,'HTTP failure '+assetURL.pathname);
  const bytes=await response.arrayBuffer();require(bytes.byteLength===pin.length,'length mismatch '+assetURL.pathname);
  require(hex(await globalThis.crypto.subtle.digest('SHA-256',bytes))===pin.sha256,'SHA256 mismatch '+assetURL.pathname);
  return bytes;
 };
 const manifest=json(await verified(url,CITIZEN_WORK_EVENT_ASSET_PINS.manifest));
 require(manifest.format==='ee2-citizen-work-events-assets-v1','manifest format differs');
 const fx=manifest.fx?.FX_Wheat_Harvest_1;
 require(fx?.sourceVisual==='FX_Wheat_Harvest_1'&&fx.tagPoint==='ROOT'&&fx.deletionMode==='eSTATECHANGE','visual declaration differs');
 const mips=json(await verified(resolve('local-original/citizen-work-events/wheat-mips.json'),CITIZEN_WORK_EVENT_ASSET_PINS.mips));
 const sourceMetadata=json(await verified(resolve('local-original/citizen-work-events/source-metadata.json'),CITIZEN_WORK_EVENT_ASSET_PINS.sourceMetadata));
 require(sourceMetadata.format==='ee2-citizen-work-source-metadata-v1'&&sourceMetadata.emitterCenterZ===.5&&
  sourceMetadata.attachmentScale==='parentSourceScale','source metadata differs');
 require(mips.format==='ee2-source-wheat-texture-mips-v1'&&mips.mipCount===8&&mips.pixelFormat==='DXT3'&&
  mips.baseTexture?.filterMode===2&&mips.baseTexture?.clampMode===3&&mips.levels?.length===8,'source mip declaration differs');
 require(mips.source.sha256===fx.texture.source.sha256&&mips.levels[0].sha256===fx.texture.sha256,'texture source differs');
 const assets=[{kind:'model',pin:fx},...Object.entries(manifest.sounds).map(([name,pin])=>({kind:'sound',name,pin})),
  ...mips.levels.map((pin,level)=>{require(pin.level===level&&pin.width===Math.max(1,64>>level)&&pin.height===Math.max(1,128>>level),'mip dimensions differ');return {kind:'mip',level,pin};})];
 // Await all reads before returning failures, so no discarded requests outlive a
 // failed loader. Nothing has been allocated in GL at this point.
 const reads=await Promise.allSettled(assets.map(async asset=>({...asset,url:resolve(asset.pin.url),bytes:await verified(resolve(asset.pin.url),asset.pin)})));
 const failed=reads.find(row=>row.status==='rejected');if(failed)throw failed.reason;
 const loaded=reads.map(row=>row.value),model=compileWheatHarvest(json(loaded.find(row=>row.kind==='model').bytes));
 const sounds=Object.freeze(Object.fromEntries(loaded.filter(row=>row.kind==='sound').map(row=>[row.name,Object.freeze({...row.pin,url:row.url.href,bytes:row.bytes})])));
 const decoded=await Promise.allSettled(loaded.filter(row=>row.kind==='mip').map(async row=>{
  const image=await decodeImage(row.bytes);
  if(!image||image.width!==row.pin.width||image.height!==row.pin.height){image?.close?.();throw new Error('Citizen work private assets: decoded mip dimensions differ');}
  return image;
 }));
 const images=decoded.filter(row=>row.status==='fulfilled').map(row=>row.value);
 const imageFailure=decoded.find(row=>row.status==='rejected');
 if(imageFailure){for(const image of images)image.close?.();throw imageFailure.reason;}
 let texture,renderer;
 try{texture=uploadTexture(gl,images);renderer=createWheatHarvestRenderer(gl,{model,texture,precision:'nearest'});}
 catch(error){if(texture)gl.deleteTexture(texture);throw error;}
 finally{for(const image of images)image.close?.();}
 let disposed=false;
 return Object.freeze({manifest,sounds,model,renderer,mips,sourceMetadata,precision:'nearest',
  attachmentScale:sourceMetadata.attachmentScale,emitterCenterZ:Math.fround(sourceMetadata.emitterCenterZ),
  dispose(){if(!disposed){disposed=true;try{renderer.dispose();}finally{gl.deleteTexture(texture);}}},
 });
}
