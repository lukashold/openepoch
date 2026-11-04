// Native NIF values are kept separate from the renderer's illumination adapter.
// The original game's complete shader/texture-stage pipeline is not inferred
// from a material flag or an enum diagnostic string.
export const DEFAULT_MODEL_LIGHTING=Object.freeze({
 ambient:Object.freeze([.64,.67,.71]),sun:Object.freeze([.5,.46,.38]),
 direction:Object.freeze([.42,.82,.39]),
});
const rgb=(value,fallback)=>Array.isArray(value)&&value.length===3&&value.every(Number.isFinite)?value:fallback;
export function modelMaterial(part){
 const m=part.material,properties=part.nativeProperties||{},vertex=properties.NiVertexColorProperty||part.vertexColorMode||{};
 const legacy=part.color||[1,1,1,1];
 return {
  ambient:rgb(m?.ambient,legacy.slice(0,3)),diffuse:rgb(m?.diffuse,legacy.slice(0,3)),
  specular:rgb(m?.specular,[0,0,0]),emissive:rgb(m?.emissive,[0,0,0]),
  alpha:Number.isFinite(m?.alpha)?m.alpha:(legacy[3]??1),
  glossiness:Math.max(0,Number.isFinite(m?.glossiness)?m.glossiness:0),
  // An absent NiSpecularProperty must not turn on all nonblack specular values.
  specularEnabled:!!(properties.NiSpecularProperty?.Flags&1),
  vertexMode:vertex['Vertex Mode']??0,lightingMode:vertex['Lighting Mode']??1,
 };
}
export function alphaState(part){
 const prop=part.nativeProperties?.NiAlphaProperty,flags=prop?.Flags??part.alpha?.flags??0;
 return {flags,blend:!!(flags&1),source:(flags>>1)&15,destination:(flags>>5)&15,
  test:!!(flags&512),testFunction:(flags>>10)&7,threshold:(prop?.Threshold??part.alpha?.threshold??0)/255};
}
export function depthState(part){
 const prop=part.nativeProperties?.NiZBufferProperty;
 return prop?{test:!!(prop.Flags&1),write:!!(prop.Flags&2),function:prop.Function??3}:{test:true,write:true,function:3};
}
// NIF TestFunction numeric order, shared by depth and alpha comparison fields.
export function alphaPasses(alpha,threshold,fn){
 return [()=>true,()=>alpha<threshold,()=>alpha===threshold,()=>alpha<=threshold,
  ()=>alpha>threshold,()=>alpha!==threshold,()=>alpha>=threshold,()=>false][fn]?.()??false;
}
export function materialLightReference(material,texel,normal,view,sun=DEFAULT_MODEL_LIGHTING,vertex=[1,1,1,1]){
 const norm=v=>{const n=Math.hypot(...v)||1;return v.map(x=>x/n);};
 const n=norm(normal),l=norm(sun.direction),eye=norm(view),dot=Math.max(0,n.reduce((s,v,i)=>s+v*l[i],0));
 const h=norm(l.map((v,i)=>v+eye[i])),shine=material.specularEnabled&&dot>0?Math.pow(Math.max(0,n.reduce((s,v,i)=>s+v*h[i],0)),material.glossiness):0;
 const ambient=material.vertexMode===2?vertex:material.ambient,diffuse=material.vertexMode===2?vertex:material.diffuse;
 const emission=material.vertexMode===1?vertex:material.emissive;
 return [0,1,2].map(i=>texel[i]*(emission[i]+(material.lightingMode===0?0:ambient[i]*sun.ambient[i]+diffuse[i]*sun.sun[i]*dot))+
  (material.lightingMode===0?0:material.specular[i]*sun.sun[i]*shine));
}
