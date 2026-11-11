const WHITE=255;
const ONE_SECOND=1;
const f32=Math.fround;

// The original map texture allocates one square power-of-two level. Its
// outer row and column and the padding retain their initial opaque alpha.
export function createFogPlane(width,height){
  if(![width,height].every(size=>Number.isInteger(size)&&size>=3&&size<=1024))throw new RangeError('Fog map size');
  const side=2**Math.ceil(Math.log2(Math.max(width,height)));
  return {width,height,side,opacity:new Uint8Array(side*side).fill(WHITE),remaining:0,uploads:0};
}

export function updateFogPlane(plane,visibility,stride,seconds){
  if(!Number.isFinite(seconds)||seconds<0)throw new RangeError('Fog elapsed time');
  plane.remaining=f32(plane.remaining-f32(seconds));
  if(plane.remaining>0)return false;
  if(!(visibility instanceof Uint8Array)||!Number.isInteger(stride)||stride<plane.width||visibility.length<plane.height*stride)
    throw new RangeError('Fog display visibility plane');
  const {width,height,side,opacity}=plane;
  for(let z=1;z<height-1;z++)for(let x=1;x<width-1;x++){
    const v=visibility[z*stride+x];
    if(v!==0&&v!==128&&v!==255)throw new RangeError('Fog display visibility value');
    opacity[z*side+x]=v===128?128:WHITE-v;
  }
  plane.remaining=ONE_SECOND;
  plane.uploads++;
  return true;
}

export function createPangeaMapFog(gl){
  const texture=gl.createTexture();
  const extension=gl.getExtension('EXT_texture_filter_anisotropic');
  const anisotropy=extension?Math.min(4,gl.getParameter(extension.MAX_TEXTURE_MAX_ANISOTROPY_EXT)):1;
  let plane=null,ready=false,error='';
  return {
    reset(width,height){
      plane=createFogPlane(width,height);
      ready=false;
      error='';
    },
    update(visibility,stride,seconds){
      if(!plane||error)return;
      try{
        if(!updateFogPlane(plane,visibility,stride,seconds))return;
      }catch(reason){
        error=String(reason);
        ready=false;
        return;
      }
      gl.activeTexture(gl.TEXTURE7);
      gl.bindTexture(gl.TEXTURE_2D,texture);
      if(plane.uploads===1){
        gl.texImage2D(gl.TEXTURE_2D,0,gl.R8,plane.side,plane.side,0,gl.RED,gl.UNSIGNED_BYTE,plane.opacity);
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
        if(extension)gl.texParameterf(gl.TEXTURE_2D,extension.TEXTURE_MAX_ANISOTROPY_EXT,anisotropy);
      }else{
        gl.texSubImage2D(gl.TEXTURE_2D,0,0,0,plane.side,plane.side,gl.RED,gl.UNSIGNED_BYTE,plane.opacity);
      }
      ready=true;
      gl.activeTexture(gl.TEXTURE0);
    },
    get binding(){return ready?{texture,side:plane.side}:null;},
    get metrics(){return {ready,error,side:plane?.side??0,uploads:plane?.uploads??0,anisotropy};},
  };
}
