// Source-backed colour/light selection. The application explicitly chooses a
// static period/weather; this module creates no native day/weather scheduler.
const f=Math.fround;
const clamp=v=>Math.max(0,Math.min(1,v));
const unit=v=>{const n=Math.hypot(...v);if(!(n>0)||!Number.isFinite(n))throw new RangeError('Invalid native sun direction');return v.map(x=>f(x/n));};
export const APPLICATION_LIGHTING_PRESET=Object.freeze({profile:'native155',climate:'Temperate',period:'Afternoon',weather:'Normal',detail:'High'});
export function createNativeLightingContext(data,settings={}){
 const chosen={...APPLICATION_LIGHTING_PRESET,...settings};
 const {profile,climate,period,weather,detail}=chosen,source=data.profiles?.[profile];
 const todKey=source?.climates?.[climate]?.[period],tod=source?.timeOfDay?.[todKey];
 const rowKey=tod?.atmosphere?.[weather],row=source?.atmospheres?.[rowKey];
 if(!row||!['High','Low'].includes(detail))throw new RangeError('No declared native lighting selection');
 const brightness=f(data.sceneLight.selected.Dimmer),factor=data.nativeColorToFloat.floatFactor;
 const sourceRgb=rgba=>rgba.slice(0,3).map(x=>f(x*factor));
 const rgb=rgba=>rgba.slice(0,3).map(x=>f(f(x*factor)*brightness));
 const sourceRay=unit(tod.sunDirectionSource),direction=[-sourceRay[0],-sourceRay[2],sourceRay[1]].map(f);
 return Object.freeze({native:true,profile,climate,period,weather,detail,rowKey,todKey,brightness,
  ambient:rgb(row.sourceRGBA.ambient),sun:rgb(row.sourceRGBA.diffuse),specular:rgb(row.sourceRGBA.specular),direction,
  // 006CD700/006CD6D0 return NiLight +E8/+F4 without the +E4 dimmer.
  // 006CD670 returns the normalized source ray, before our Y-up adapter.
  sourceAmbient:sourceRgb(row.sourceRGBA.ambient),sourceDiffuse:sourceRgb(row.sourceRGBA.diffuse),sourceDirection:sourceRay,
  sourceTerrainDirection:unit(tod.sunDirectionSource.map(f)),
  terrain:data.terrain,fog:{rgba:row.sourceRGBA.fog.map(x=>f(x*factor)),start:row.fogStart,on:row.fogOn,intensity:row.fogIntensity}});
}
export function nativeModelPipeline(part,lighting){
 if(!lighting?.native)return {shader:0,textureScale:1};
 // 005F9100: a NiGeometry with a SkinInstance receives IndexPal20. The
 // portable animation stream already supplies transformed positions/normals.
 if(part.skinned)return {shader:lighting.detail==='Low'?2:1,textureScale:4};
 // 00957B70 selects TSS COLOROP=6 only for APPLY_MODULATE4X=4 on a base
 // texture. An absent texture and other apply modes retain their own path.
 const hasTexture=!!(part.textureUrl||part.texture||part.textureSource);
 return {shader:0,textureScale:hasTexture&&part.textureApplyMode===4?4:hasTexture&&part.textureApplyMode===3?2:1};
}
export function nativeIndexPalVertex(normal,lighting,{low=lighting.detail==='Low'}={}){
 const n=unit(normal),dot=low?.5:n.reduce((sum,value,i)=>sum+value*lighting.direction[i],0);
 return lighting.sun.map((sun,i)=>clamp(f(f(dot*sun)+lighting.ambient[i])));
}
export function nativeIndexPalStage(texel,vertexColor,materialAlpha=1){
 return [...texel.slice(0,3).map((value,i)=>clamp(value*vertexColor[i]*4)),texel[3]*materialAlpha];
}
