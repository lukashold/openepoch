// Capture production renderer draw inputs for a native headless GLES replay.
// This is a test adapter, never a browser or app automation interface.
export class RecordingGL {
 constructor(){
  Object.assign(this,{ARRAY_BUFFER:34962,ELEMENT_ARRAY_BUFFER:34963,STATIC_DRAW:35044,STREAM_DRAW:35040,DYNAMIC_DRAW:35048,
   FLOAT:5126,UNSIGNED_INT:5125,UNSIGNED_SHORT:5123,UNSIGNED_BYTE:5121,TRIANGLES:4,TRIANGLE_STRIP:5,TEXTURE0:33984,TEXTURE1:33985,TEXTURE2:33986,TEXTURE3:33987,TEXTURE4:33988,TEXTURE5:33989,TEXTURE6:33990,TEXTURE_2D:3553,TEXTURE_2D_ARRAY:35866,
   RED:6403,R8:33321,R32F:33326,RGBA:6408,RGBA8:32856,RGBA32F:34836,VERTEX_SHADER:35633,FRAGMENT_SHADER:35632,COMPILE_STATUS:35713,LINK_STATUS:35714,
   TEXTURE_MIN_FILTER:10241,TEXTURE_MAG_FILTER:10240,TEXTURE_WRAP_S:10242,TEXTURE_WRAP_T:10243,LINEAR_MIPMAP_LINEAR:9987,LINEAR:9729,NEAREST:9728,REPEAT:10497,CLAMP_TO_EDGE:33071,UNPACK_FLIP_Y_WEBGL:37440,
   BLEND:3042,DEPTH_TEST:2929,STENCIL_TEST:2960,CULL_FACE:2884,DEPTH_WRITEMASK:2930,DEPTH_FUNC:2932,BLEND_SRC_RGB:32969,BLEND_DST_RGB:32968,BLEND_SRC_ALPHA:32971,BLEND_DST_ALPHA:32970,BLEND_EQUATION_RGB:32777,BLEND_EQUATION_ALPHA:34877,
   ZERO:0,ONE:1,SRC_COLOR:768,ONE_MINUS_SRC_COLOR:769,SRC_ALPHA:770,ONE_MINUS_SRC_ALPHA:771,DST_ALPHA:772,ONE_MINUS_DST_ALPHA:773,DST_COLOR:774,ONE_MINUS_DST_COLOR:775,SRC_ALPHA_SATURATE:776,
   NEVER:512,LESS:513,EQUAL:514,LEQUAL:515,GREATER:516,NOTEQUAL:517,GEQUAL:518,ALWAYS:519,FUNC_ADD:32774,KEEP:7680,INCR:7682,
   ACTIVE_TEXTURE:34016,CURRENT_PROGRAM:35725,VERTEX_ARRAY_BINDING:34229,ARRAY_BUFFER_BINDING:34964,TEXTURE_BINDING_2D:32873});
  this.next=1;this.unit=0;this.units=new Map();this.defaultVAO={attributes:new Map()};this.vao=this.defaultVAO;
  this.enabled=new Set([this.DEPTH_TEST]);this.depthWrite=true;this.depthComparison=this.LESS;this.blend=[this.ONE,this.ONE_MINUS_SRC_ALPHA,this.ONE,this.ONE_MINUS_SRC_ALPHA];this.equation=[this.FUNC_ADD,this.FUNC_ADD];
  this.draws=[];this.buffers=new Map();this.textures=new Map();this.programs=new Map();this.bufferCopies={};this.stencil={func:[this.ALWAYS,0,255],op:[this.KEEP,this.KEEP,this.KEEP],mask:255};this.flip=false;
 }
 object(kind){return {id:this.next++,kind};}
 createProgram(){const p={...this.object('program'),shaders:[],uniforms:{}};this.programs.set(p.id,p);return p;}
 createShader(type){return {...this.object('shader'),type};}shaderSource(s,source){s.source=source;}compileShader(){}getShaderParameter(){return true;}getShaderInfoLog(){return '';}
 attachShader(p,s){p.shaders.push(s);}deleteShader(){}linkProgram(){}getProgramParameter(){return true;}getProgramInfoLog(){return '';}
 getUniformLocation(program,name){return {program,name};}useProgram(p){this.program=p;}
 uniform(loc,kind,value){if(loc)loc.program.uniforms[loc.name]={kind,value:ArrayBuffer.isView(value)?Array.from(value):value};}
 uniform1i(l,v){this.uniform(l,'1i',v);}uniform1f(l,v){this.uniform(l,'1f',v);}uniform3f(l,x,y,z){this.uniform(l,'3fv',[x,y,z]);}
 uniform2fv(l,v){this.uniform(l,'2fv',v);}uniform3fv(l,v){this.uniform(l,'3fv',v);}uniform4fv(l,v){this.uniform(l,'4fv',v);}uniformMatrix4fv(l,t,v){this.uniform(l,'matrix4',v);}
 createBuffer(){const b={...this.object('buffer'),version:0};this.buffers.set(b.id,b);return b;}
 bindBuffer(target,b){if(target===this.ARRAY_BUFFER)this.array=b;else this.vao.element=b;}
 bufferData(target,data){const b=target===this.ARRAY_BUFFER?this.array:this.vao.element;b.data=ArrayBuffer.isView(data)?new data.constructor(data):new Uint8Array(data);b.version++;}
 bufferSubData(target,offset,data){const b=target===this.ARRAY_BUFFER?this.array:this.vao.element;new Uint8Array(b.data.buffer).set(new Uint8Array(data.buffer,data.byteOffset,data.byteLength),offset);b.version++;}
 createVertexArray(){return {...this.object('vao'),attributes:new Map()};}bindVertexArray(vao){this.vao=vao||this.defaultVAO;}
 enableVertexAttribArray(loc){this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),enabled:true});}
 vertexAttribPointer(loc,size,type,normalized,stride,offset){this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),buffer:this.array,size,type,normalized,stride,offset});}
 vertexAttribDivisor(loc,divisor){this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),divisor});}
 createTexture(){const t={...this.object('texture'),params:{}};this.textures.set(t.id,t);return t;}
 activeTexture(unit){this.unit=unit-this.TEXTURE0;}bindTexture(target,texture){this.units.set(`${this.unit}:${target}`,texture);}
 bound(target){return this.units.get(`${this.unit}:${target}`);}
 texImage2D(...a){const t=this.bound(a[0]);t.target=a[0];t.internal=a[2];if(a.length===6){Object.assign(t,{width:a[5].width,height:a[5].height,image:a[5].url,format:a[3],type:a[4]});}else Object.assign(t,{width:a[3],height:a[4],format:a[6],type:a[7],data:a[8]&&new a[8].constructor(a[8])});}
 texSubImage2D(target,level,x,y,width,height,format,type,data){const t=this.bound(target);if(x||y||width!==t.width||height!==t.height)throw Error('Capture only supports complete 2D updates');Object.assign(t,{format,type,data:new data.constructor(data)});}
 texStorage3D(target,levels,internal,width,height,depth){Object.assign(this.bound(target),{target,levels,internal,width,height,depth});}
 texSubImage3D(target,level,x,y,z,width,height,depth,format,type,data){
  const texture=this.bound(target);
  if(ArrayBuffer.isView(data))Object.assign(texture,{format,type,data:new data.constructor(data)});
  else if(data?.url){
   if(x||y||depth!==1||width!==texture.width||height!==texture.height)throw Error('Capture only supports complete native image layers');
   Object.assign(texture,{format,type});texture.images??={};texture.images[z]={image:data.url,width:data.width,height:data.height};
  }else throw Error('Unsupported captured 3D texture source');
 }
 texParameteri(target,key,value){this.bound(target).params[key]=value;}texParameterf(target,key,value){this.texParameteri(target,key,value);}generateMipmap(target){this.bound(target).mipmap=true;}pixelStorei(p,value){if(p===this.UNPACK_FLIP_Y_WEBGL)this.flip=value;}getExtension(){return null;}
 enable(cap){this.enabled.add(cap);}disable(cap){this.enabled.delete(cap);}isEnabled(cap){return this.enabled.has(cap);}depthMask(v){this.depthWrite=v;}depthFunc(v){this.depthComparison=v;}
 blendFunc(src,dst){this.blend=[src,dst,src,dst];}blendFuncSeparate(...v){this.blend=v;}blendEquation(v){this.equation=[v,v];}blendEquationSeparate(...v){this.equation=v;}
 stencilFunc(...v){this.stencil.func=v;}stencilOp(...v){this.stencil.op=v;}stencilMask(v){this.stencil.mask=v;}
 getParameter(p){return ({[this.DEPTH_WRITEMASK]:this.depthWrite,[this.DEPTH_FUNC]:this.depthComparison,[this.BLEND_SRC_RGB]:this.blend[0],[this.BLEND_DST_RGB]:this.blend[1],[this.BLEND_SRC_ALPHA]:this.blend[2],[this.BLEND_DST_ALPHA]:this.blend[3],[this.BLEND_EQUATION_RGB]:this.equation[0],[this.BLEND_EQUATION_ALPHA]:this.equation[1],
  [this.ACTIVE_TEXTURE]:this.TEXTURE0+this.unit,[this.CURRENT_PROGRAM]:this.program,[this.VERTEX_ARRAY_BINDING]:this.vao,[this.ARRAY_BUFFER_BINDING]:this.array,[this.TEXTURE_BINDING_2D]:this.bound(this.TEXTURE_2D),[this.UNPACK_FLIP_Y_WEBGL]:this.flip})[p];}
 bufferCopy(buffer){if(!buffer)return null;const key=`${buffer.id}:${buffer.version}`;if(!this.bufferCopies[key])this.bufferCopies[key]=encode(buffer.data);return key;}
 capture(mode,count,type,offset,instances=1){
  const attributes=Object.fromEntries([...this.vao.attributes].map(([loc,a])=>[loc,{...a,buffer:this.bufferCopy(a.buffer)}]));
  this.draws.push({program:this.program.id,mode,count,type,offset,instances,element:this.bufferCopy(this.vao.element),attributes,
   uniforms:structuredClone(this.program.uniforms),textures:Object.fromEntries([...this.units].filter(([,t])=>t).map(([key,t])=>[key,t.id])),
   state:{enabled:[...this.enabled],depthWrite:this.depthWrite,depthFunc:this.depthComparison,blend:[...this.blend],equation:[...this.equation],stencil:structuredClone(this.stencil)}});
 }
 drawElementsInstanced(mode,count,type,offset,instances){this.capture(mode,count,type,offset,instances);}drawElements(mode,count,type,offset){this.capture(mode,count,type,offset);}drawArrays(mode,first,count){this.capture(mode,count,null,first);}
 payload(){return {programs:Object.fromEntries([...this.programs].map(([id,p])=>[id,{vertex:p.shaders.find(s=>s.type===this.VERTEX_SHADER).source,fragment:p.shaders.find(s=>s.type===this.FRAGMENT_SHADER).source}])),
  buffers:this.bufferCopies,textures:Object.fromEntries([...this.textures].map(([id,t])=>[id,{...t,data:t.data?encode(t.data):null}])),draws:this.draws};}
}
function encode(data){return {type:data.constructor.name,base64:Buffer.from(data.buffer,data.byteOffset,data.byteLength).toString('base64')};}
