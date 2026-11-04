// OpenEpoch front end: input, camera, WebGL2 drawing, HUD, sound, saves, networking.
// The simulation lives in openepoch.wasm (Rust); this file never touches game rules.
import { buildAtlas, tintAtlas, CELL } from './atlas.js?v=fortress1';
import { buildModels } from './models.js?v=fortress1';
import { parseCheat } from './cheats.js';
import { researchPanel } from './research.js?v=research3';
import { diplomacyPanel } from './diplomacy.js?v=fortress1';
import {enhanceOriginalDiplomacy} from './original-diplomacy.js?v=native6';
import {loadNativeFontUI} from './original-font-ui.js?v=native6';
import {territoryClaimMessage} from './territory-help.js?v=fortress1';
import * as sfx from './audio.js?v=native8';
import {loadOriginalAssets,loadOriginalEpochIcons,applyOriginalInterface,originalIcon} from './original-assets.js?v=native6';
import {loadOriginalLabels,escapeLabelHtml} from './original-labels.js?v=native6';
import {loadNativeHud} from './original-hud.js?v=native6';
import {loadOriginalModels} from './original-renderer.js?v=pack-sails8';
import {loadSourceSailRuntime} from './source-sail-runtime.js?v=sails7';
import {createRenderAssetMapping} from './original-render-namespace.js?v=native6';
import {createFrontendNamespace} from './render-namespace.js?v=native8';
import {createRenderActorMetadata} from './render-actor-metadata.js?v=pack8';
import {createPackRenderMetadata} from './original-pack-states.js?v=pack8';
import {loadOriginalGround} from './original-ground.js?v=native6';
import {loadOriginalTerrain} from './original-terrain.js?v=native6';
import {loadOriginalWater} from './original-water.js?v=water-farm3';
import {createOriginalReflection} from './original-reflection.js?v=visual-seasons2';
import {normalizeSeasonSettings} from './original-seasons.js';
import {createSeasonSession,peerSeasonSettings,savedSeasonSettings} from './season-session.js';
import {UP15_PROFILE_ENABLED,resolveRulesProfile,rulesProfileURL,loadRulesWasm,savedRulesProfileMatches} from './rules-profile.js?v=native8';
import {createEconomyBar,RESOURCE_NAMES} from './economy-bar.js?v=fortress1';
import { Lockstep, PROTOCOL_VERSION, host as rtcHost, join as rtcJoin } from './net.js?v=native8';
import {createFullscreenController} from './fullscreen.js?v=building2';
import {createCivilizationSetup,civilizationChoice,resolveCivilizations,checkedCivilizations,applyMatchSettings} from './civilization-setup.js?v=native8';
import {createBuildingControls,nextBuildingTurn,placementMessage} from './building-controls.js?v=fortress1';
import {updateOriginalCamera,originalCameraRay,originalFractionForZoom,normalizeOriginalHeading,rotateOriginalCameraHeading} from './original-camera.js?v=heading2';

const $ = id => document.getElementById(id);
const canvas = $('gl');
const gl = canvas.getContext('webgl2', { antialias: true, alpha: false, stencil: true, powerPreference: 'high-performance' });
if (!gl) { document.body.innerHTML = '<p style="padding:40px">OpenEpoch needs WebGL2. Please use a current browser.</p>'; throw new Error('no webgl2'); }

const MAP = 256, TICK = 0.05, STRIDE = 12;
const FOV = 35 * Math.PI / 180, PITCH = 55 * Math.PI / 180, TANF = Math.tan(FOV / 2);
const TEAM = [[0.27, 0.59, 1.0], [0.94, 0.27, 0.24], [0.31, 0.86, 0.39], [0.96, 0.82, 0.27], [1, 1, 1], [1, 1, 1], [1, 1, 1], [0.3, 0.75, 1]];

// ---------------------------------------------------------------- wasm
const profileSelect=$('rules-profile');
for(const option of profileSelect.options)if(option.value==='up15')option.disabled=!UP15_PROFILE_ENABLED;
profileSelect.onchange=()=>location.assign(rulesProfileURL(location.href,profileSelect.value).href);
function rulesLoadFailure(error){
 $('rules-note').textContent=error.message;
 for(const id of ['play','mpopen','continue','previous-save'])$(id).disabled=true;
 $('play').textContent='Regelversion konnte nicht geladen werden';
}
let rulesProfile;
try{rulesProfile=resolveRulesProfile(location.href);}catch(error){rulesLoadFailure(error);throw error;}
profileSelect.value=rulesProfile.id;
const referenceMode=rulesProfile.reference,qaMode=rulesProfile.qa;
try { const preference=localStorage.getItem('openepoch-map-visibility');if(['0','1','2'].includes(preference))$('map-visibility').value=preference; } catch { /* preferences are optional */ }
$('map-visibility').onchange=()=>{try{localStorage.setItem('openepoch-map-visibility',$('map-visibility').value);}catch{/* private storage */}};
function menuSeasonSettings(){
  return normalizeSeasonSettings({cycle:$('season-cycle').checked,startMonth:+$('season-start').value,minutesPerSeason:+$('season-length').value});
}
function setSeasonControls(settings){
  $('season-cycle').checked=settings.cycle;
  $('season-start').value=settings.startMonth;
  $('season-length').value=settings.minutesPerSeason;
  $('season-length').disabled=!settings.cycle;
}
$('season-options').hidden=!referenceMode;
try{const saved=localStorage.getItem('openepoch-seasons');if(saved)setSeasonControls(peerSeasonSettings(JSON.parse(saved)));}catch{/* optional preferences */}
for(const id of ['season-cycle','season-start','season-length'])$(id).onchange=()=>{
  const settings=menuSeasonSettings();setSeasonControls(settings);
  try{localStorage.setItem('openepoch-seasons',JSON.stringify(settings));}catch{/* private storage */}
};
if (referenceMode) {
  $('play').disabled=true;$('play').textContent='Originalgrafiken laden …';
  $('supplies').innerHTML='<option value="3">Wenig</option><option value="4">Defensiv</option><option value="0">Standard</option><option value="1" selected>Viel</option><option value="2">Deathmatch</option>';
  document.title = 'OpenEpoch · EE2 Rekonstruktion';
  $('terr').parentElement.title='Deine Gebiete. Angrenzend genügt ein Stadtzentrum; abgelegene Gebiete benötigen zusätzlich eine Festung.';
  $('cheat-rule').textContent='give tech erhöht das reguläre Forschungskonto um 50 Technologiepunkte. Technologien und Epochenwechsel werden mit diesen Punkten bezahlt.';
  $('rules-note').textContent = '15 Epochen, neun Rohstoffe, Originalkosten und Forschung durch Bürger in Universitäten. Erweitere dein Reich mit Stadtzentren und Festungen. Originalgrafiken, Stimmen und Musik liegen im Spielordner. Einheitenangebot und Spezialfähigkeiten werden weiter ergänzt.';
  // Both multiplayer peers must explicitly match this ruleset.
}
let wasm;
try{wasm=await loadRulesWasm(rulesProfile,import.meta.url);}catch(error){rulesLoadFailure(error);throw error;}
// Always take the pointer BEFORE touching memory.buffer: the call may grow (and detach) the memory.
const u8 = (ptr, len) => new Uint8Array(wasm.memory.buffer, ptr, len);
const f32 = (ptr, len) => new Float32Array(wasm.memory.buffer, ptr, len);
const dec = new TextDecoder();
const assetMapping=createRenderAssetMapping(wasm);
const renderNamespace=createFrontendNamespace(wasm,{referenceMode,assetMapping});
const packMetadata=createPackRenderMetadata(wasm),packActorReader=packMetadata?createRenderActorMetadata(wasm):null;
const civilizationSetup=createCivilizationSetup({wasm,document,anchor:$('pop-limit').closest('.row')});
if(referenceMode){
  const row=document.createElement('div');row.className='row';
  const label=document.createElement('label');label.htmlFor='start-epoch';label.textContent='Startepoche';
  const select=document.createElement('select');select.id='start-epoch';
  for(let i=0;i<wasm.epoch_count();i++){const option=document.createElement('option');option.value=i;option.textContent=`${i+1} · ${dec.decode(u8(wasm.epoch_name_ptr(i),wasm.epoch_name_len(i)))}`;select.append(option);}
  const requested=+(new URLSearchParams(location.search).get('epoch')||0);if(requested>=0&&requested<wasm.epoch_count())select.value=requested;
  row.append(label,select);$('pop-limit').closest('.row').before(row);
}

// ---------------------------------------------------------------- GL setup
function shader(type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src); gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) + src);
  return s;
}
function program(vs, fs) {
  const p = gl.createProgram();
  gl.attachShader(p, shader(gl.VERTEX_SHADER, vs)); gl.attachShader(p, shader(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {};
  const n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) { const name = gl.getActiveUniform(p, i).name.replace('[0]', ''); u[name] = gl.getUniformLocation(p, name); }
  return { p, u };
}

const NOISE = `
float hash(vec2 p){ p = fract(p*vec2(123.34, 456.21)); p += dot(p, p+45.32); return fract(p.x*p.y); }
float noise(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1,0)),f.x), mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y); }`;
const FOG = `float fogAt(vec2 w){ return texture(uFog, w/${MAP}.).r; }
vec3 fogged(vec3 c, float f){ return c*smoothstep(.02,.45,f)*mix(.5,1.,smoothstep(.55,.95,f)); }`;
const SUN = 'const vec3 SUN = vec3(0.42, 0.82, 0.39);';

const terrain = program(`#version 300 es
layout(location=0) in vec3 aP; layout(location=1) in vec3 aN; uniform mat4 uVP; out vec3 vP; out vec3 vN;
void main(){ vP = aP; vN = aN; gl_Position = uVP*vec4(aP,1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog, uTerr; uniform highp sampler2DArray uOriginalTerrain;uniform bool uOriginal;
uniform vec3 uTeam[8]; uniform float uPx; in vec3 vP; in vec3 vN; out vec4 o;
${NOISE}${FOG}${SUN}
void main(){
  vec2 w = vP.xz, uv = w/${MAP}.;
  float visibility=fogAt(w);if(visibility<.015){o=vec4(0.,0.,0.,1.);return;}
  vec4 m = texture(uMap, uv);
  vec3 c = m.rgb;
  float grass = smoothstep(1.02,1.26,c.g/(max(c.r,c.b)+.001));
  float groundPatch=noise(w*.32)*.6+noise(w*1.1)*.4;
  if(uOriginal){
    vec2 tiled=w*.22;
    float beach=smoothstep(.54,.7,m.r)*(1.-grass);
    float stone=(1.-smoothstep(.035,.1,abs(m.r-m.g)))*(1.-beach)*(1.-grass);
    float forest=grass*(1.-smoothstep(.22,.33,m.r));
    if(m.a<.7)c=texture(uOriginalTerrain,vec3(tiled,6.)).rgb;
    else if(beach>.99)c=texture(uOriginalTerrain,vec3(tiled,3.)).rgb;
    else if(stone>.99)c=texture(uOriginalTerrain,vec3(tiled,4.)).rgb;
    else{
      vec3 base=texture(uOriginalTerrain,vec3(tiled,0.)).rgb;
      vec3 patchGrass=texture(uOriginalTerrain,vec3(tiled+vec2(.31,.23),1.)).rgb;
      vec3 soil=texture(uOriginalTerrain,vec3(tiled,2.)).rgb;
      c=mix(base,patchGrass,smoothstep(.38,.72,groundPatch)*.55);
      c=mix(c,soil,forest*.28+smoothstep(.68,.85,groundPatch)*.18);
      if(beach>.01)c=mix(c,texture(uOriginalTerrain,vec3(tiled,3.)).rgb,beach);
      if(stone>.01)c=mix(c,texture(uOriginalTerrain,vec3(tiled,4.)).rgb,stone);
    }
  }else{
    float n=noise(w*.7)*.35+noise(w*5.)*.25+noise(w*23.)*.4;
    vec3 turf=mix(c*vec3(.83,.92,.77),vec3(.39,.36,.20),smoothstep(.53,.76,groundPatch)*.4);
    float blades=pow(noise(w*16.),2.);turf*=.94+blades*.18*clamp(1.-uPx*18.,0.,1.);
    c=mix(c,turf,grass)*(.88+.3*n);
  }
  float lit = .55 + .6*max(dot(normalize(vN), SUN), 0.);
  c *= lit;
  c *= mix(1., .5, clamp(-vP.y*.9, 0., 1.));                       // sea bed fades with depth
  // territory borders
  vec4 t = texture(uTerr, uv);
  float d = max(.09, uPx*1.3)/${MAP}.;
  float e = 0.;
  e = max(e, step(.001, abs(texture(uTerr, uv+vec2(d,0.)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv-vec2(d,0.)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv+vec2(0.,d)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv-vec2(0.,d)).r - t.r)));
  int owner = int(t.g*255.+.5);
  vec3 tc = owner > 0 ? uTeam[owner-1] : vec3(.9);
  c = mix(c, tc, owner > 0 ? .06 + e*.7 : e*.2);
  o = vec4(fogged(c, visibility), 1.);
}`);

const water = program(`#version 300 es
layout(location=0) in vec2 aP; uniform mat4 uVP; out vec2 vW;
void main(){ vW = aP; gl_Position = uVP*vec4(aP.x, 0., aP.y, 1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog;uniform highp sampler2DArray uOriginalTerrain;uniform bool uOriginal;
uniform float uTime; in vec2 vW; out vec4 o;
${NOISE}${FOG}
void main(){
  vec2 uv = vW/${MAP}.;
  if (uv.x<0.||uv.y<0.||uv.x>1.||uv.y>1.) { o = vec4(.012,.03,.05,1.); return; }
  vec4 m = texture(uMap, uv);
  if(m.a>=.72)discard;
  float visibility=fogAt(vW);if(visibility<.015)discard;
  float waves=sin(vW.x*2.1+uTime*1.3)+sin(vW.y*1.8-uTime*.9);
  float wv = .5 + waves*.12 + noise(vW*.7+uTime*.1)*.15;
  vec3 c = mix(vec3(.05,.2,.36), vec3(.1,.42,.58), smoothstep(0.,.3,m.a))*(.75+.5*wv) + vec3(.9,.95,1.)*smoothstep(.7,.9,wv)*.14;
  c += vec3(.55,.75,.85)*pow(max(0.,sin(vW.x*.7+vW.y*1.2+uTime*.6)),24.)*.12;
  if(uOriginal){
    vec3 original=texture(uOriginalTerrain,vec3(vW*.16+vec2(uTime*.009,-uTime*.005),5.)).rgb;
    c=mix(c,original*(.85+.15*wv),.52);
  }
  float foam = smoothstep(.1,.36,m.a)*(1.-smoothstep(.36,.6,m.a));
  c += foam*(.2 + .14*sin(uTime*1.7 + wv*9.));
  float a = (1.-smoothstep(.4,.72,m.a))*.74;
  o = vec4(fogged(c, visibility)*a, a);
}`);

const INST = `layout(location=3) in vec4 iA; layout(location=4) in vec4 iB; layout(location=5) in vec4 iC;`;
const models = program(`#version 300 es
layout(location=0) in vec3 aP; layout(location=1) in vec3 aN; layout(location=2) in vec4 aCol; layout(location=6) in float aJoint; ${INST}
uniform mat4 uVP; uniform highp int uShadow; out vec3 vN; out vec4 vCol; out vec2 vW; out vec3 vLocal; out float vLy; flat out float vJoint; flat out vec4 vB;
void main(){
  float c=cos(iA.w), s=sin(iA.w);
  vec3 local=aP, normal=aN;
  float phase=iC.z*8.+iA.x*.7+iA.y*.3, moving=min(iC.y,1.);
  bool human=iB.x>=8. && iB.x<=17.;
  if(human){
    int bone=int(aJoint+.5), activity=int(iC.w+.5);
    float side=bone==1||bone==3?-1.:1.;
    float angle=0.; vec3 pivot=vec3(0.);
    if(bone==1||bone==2){
      pivot=vec3(0.,.31,side*.075); angle=sin(phase)*side*.65*moving;
    }else if(bone==3||bone==4||bone>=6){
      pivot=vec3(0.,.52,side*.18); angle=-sin(phase)*side*.45*moving;
      if(activity>=2 && activity<=5){
        float stroke=sin(iC.z*(activity==4?4.:7.)+iA.x);
        angle=activity==4?.35+stroke*.2:.65+stroke*.75;
        if(bone==3)angle*=.6;
      }else if(activity==6){ angle=.8+sin(iC.z*9.)*.22; }
    }else if(bone==5){ pivot=vec3(0.,.59,0.); angle=sin(iC.z*1.2+iA.x)*.035; }
    if(bone>0){
      float ca=cos(angle),sa=sin(angle);vec3 q=local-pivot;
      local=vec3(q.x*ca-q.y*sa,q.x*sa+q.y*ca,q.z)+pivot;
      normal=vec3(aN.x*ca-aN.y*sa,aN.x*sa+aN.y*ca,aN.z);
    }
    local.y+=abs(sin(phase))*.018*moving;
    if(activity==4)local.x+=sin(iC.z*4.)*.025;
  }else if(iB.x==43.){
    if(aP.y<.25)local.x+=sin(phase+sign(aP.z)*1.57)*.12*moving;
    local.y+=abs(sin(phase))*.025*moving;
  }
  vec3 p=local*iC.x;
  if(iB.x==46.)p.y+=.12+sin(iC.z*2.+iA.x)*.025;
  if(iB.x>=29. && iB.x<=36.)p.y+=sin(iC.z*1.4+iA.x)*.015;
  vec3 w = vec3(iA.x + p.x*c - p.z*s, iA.z + p.y, iA.y + p.x*s + p.z*c);
  if(uShadow==1){
    w.xz -= vec2(.62,.44)*max(p.y,0.);
    w.y = iA.z+.035;
  }
  vLocal=aP;
  vN = vec3(normal.x*c-normal.z*s,normal.y,normal.x*s+normal.z*c);
  vCol = aCol; vW = w.xz; vLy = aP.y; vB = iB; if(human)vB.z=iC.w; vJoint=aJoint;
  gl_Position = uVP*vec4(w,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uFog; uniform vec3 uTeam[8]; uniform highp int uShadow;
in vec3 vN; in vec4 vCol; in vec2 vW; in vec3 vLocal; in float vLy; flat in float vJoint; flat in vec4 vB; out vec4 o;
${FOG}${SUN}${NOISE}
void main(){
  int fl = int(vB.w+.5);
  if(vB.x==8. && vJoint>=6.){
    int activity=int(vB.z+.5);
    if((vJoint==6. && activity!=2)||(vJoint==7. && activity!=3)||(vJoint==8. && activity!=5)||(vJoint==9. && activity!=4))discard;
  }
  bool tree = vB.x==100. || vB.x==101.;
  if(tree && vLocal.y>.38 && noise(vLocal.xz*65.+vLocal.y*17.)<.16) discard;
  if(uShadow==1){
    if((fl&24)!=0 || fogAt(vW)<.15) discard;
    o=vec4(vec3(.018,.025,.014)*.18,.18); return;
  }
  vec3 c = vCol.a > .5 ? uTeam[int(vB.y+.5)]*vCol.r : vCol.rgb;
  float a = 1.;
  vec3 n = normalize(vN);
  c *= vec3(.36,.43,.52) + vec3(.8,.72,.57)*max(dot(n, SUN), 0.) + .13*max(n.y, 0.);
  c *= mix(.72,1.,smoothstep(0.,.35,vLy));
  if(tree){
    if(vLocal.y>.38) c*=.68+.5*noise(vLocal.xz*63.+vLocal.y*30.);
    else c*=.72+.35*noise(vLocal.xz*80.);
  }
  if((fl&64)!=0 && vCol.a<.5){
    float grain=noise(vLocal.xz*70.+vLocal.y*50.);
    c*=.9+.17*grain;
    if(abs(n.y)<.5){
      vec2 wall=vec2(abs(n.x)>.5?vLocal.z:vLocal.x,vLocal.y);
      vec2 bricks=vec2(wall.x*14.+floor(wall.y*18.)*.5,wall.y*18.);
      vec2 edge=abs(fract(bricks)-.5);
      float mortar=smoothstep(.43,.49,max(edge.x,edge.y));
      c*=1.-mortar*.22;
    }
  }
  if ((fl&64)!=0 && vB.z < 1.) {                                   // under construction: the building rises
    if (vLy > .03 + vB.z*.95) discard;
    c = mix(vec3(dot(c, vec3(.33)))*vec3(1.,.9,.7), c, vB.z);
  }
  if ((fl&4)!=0) c = mix(c, vec3(1.), .5);
  if ((fl&8)!=0) { c = mix(c, vec3(.3,1.,.4), .4); a = .6; }
  if ((fl&16)!=0) { c = mix(c, vec3(1.,.15,.15), .65); a = .6; }
  if ((fl&32)==0) { float f = fogAt(vW); c = fogged(c, f); if (f < .05) discard; }
  o = vec4(c*a, a);
}`);

const quads = program(`#version 300 es
layout(location=0) in vec2 aC; ${INST}
uniform mat4 uVP; uniform vec3 uRight, uUp;uniform float uPx;out vec2 vL; out vec2 vW; flat out vec4 vB;
void main(){
  vec3 base = vec3(iA.x, iA.z, iA.y); float s = iC.x;
  bool flat_ = iB.x < 240.5 || iB.x > 242.5;                       // rings and shadows lie on the ground
  float sy=iB.x>240.5&&iB.x<241.5?min(s,32.*uPx):s;
  vec3 w = flat_ ? base + vec3(aC.x, 0., aC.y)*s : base + uRight*aC.x*s + uUp*aC.y*sy;
  vL = aC; vW = base.xz; vB = iB;
  gl_Position = uVP*vec4(w,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uFog; uniform vec3 uTeam[8]; uniform float uPx;
in vec2 vL; in vec2 vW; flat in vec4 vB; out vec4 o;
${FOG}
void main(){
  int team = int(vB.y+.5); float r = length(vL)*2.; vec4 col;
  if (vB.x < 240.5) {                                              // selection ring
    float w = clamp(uPx*1.6, .03, .2);
    float a = smoothstep(.86-w,.86-w*.5,r)*(1.-smoothstep(.86+w*.5,.86+w,r));
    col = vec4(mix(vec3(.6,1.,.6), uTeam[team], .35),1.)*a*.95;
  } else if (vB.x < 241.5) {                                       // health / progress bar
    float y = abs(vL.y), x = vL.x+.5;
    if (y > .075) discard;
    vec3 c = team==7 ? vec3(.3,.75,1.) : mix(vec3(.95,.2,.15), vec3(.25,.9,.3), smoothstep(.2,.7,vB.z));
    if (x > vB.z) c = vec3(.1);
    if (y > .045) c = vec3(0.);
    col = vec4(c,1.);
  } else if (vB.x < 242.5) {                                       // explosion
    float t = vB.z, q = r/(.3+.7*t);
    float a = (1.-smoothstep(.55,1.,q))*(1.-t*t);
    vec3 c = mix(vec3(1.,.95,.7), vec3(1.,.4,.05), clamp(q*.7+t*.6,0.,1.));
    col = vec4(mix(c, vec3(.15), smoothstep(.5,1.,t))*a, a*.7);
  } else {                                                         // aircraft shadow
    float a = (1.-smoothstep(.5,1.,r))*.34;
    col = vec4(0.,0.,0.,a)*smoothstep(.3,.9,fogAt(vW));
  }
  o = col;
}`);

function tex(unit, filter) {
  const t = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}
gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
const mapTexture=tex(1, gl.LINEAR); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, MAP, MAP, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
const fogTexture=tex(2, gl.LINEAR); gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, MAP, MAP, 0, gl.RED, gl.UNSIGNED_BYTE, null);
const territoryTexture=tex(3, gl.NEAREST); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, MAP, MAP, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
function bindTexture(unit,texture,target=gl.TEXTURE_2D){gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(target,texture);}

// 2D icons for the HUD come from the sprite atlas; the world itself is 3D
const iconAtlas = tintAtlas(buildAtlas(), TEAM[0].map(v => v * 255));

// terrain mesh: (MAP+1)^2 vertices, heights filled in per game
const HN = MAP + 1;
const vaoT = gl.createVertexArray(); gl.bindVertexArray(vaoT);
const terrBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, terrBuf);
gl.bufferData(gl.ARRAY_BUFFER, HN * HN * 24, gl.DYNAMIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
{
  const idx = new Uint32Array(MAP * MAP * 6);
  for (let y = 0, k = 0; y < MAP; y++) for (let x = 0; x < MAP; x++) {
    const a = y * HN + x, b = a + 1, c = a + HN, d = c + 1;
    idx[k++] = a; idx[k++] = c; idx[k++] = b; idx[k++] = b; idx[k++] = c; idx[k++] = d;
  }
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);
}
let heights = new Float32Array(HN * HN);
function uploadTerrain() {
  heights = f32(wasm.hmap_ptr(), HN * HN).slice();
  originalGround.updateTerrain(heights);
  originalModels.updateTerrain?.(heights);
  uploadOriginalTerrainClasses();
  const v = new Float32Array(HN * HN * 6), h = (x, y) => heights[Math.min(MAP, Math.max(0, y)) * HN + Math.min(MAP, Math.max(0, x))];
  for (let y = 0, k = 0; y < HN; y++) for (let x = 0; x < HN; x++) {
    const nx = h(x - 1, y) - h(x + 1, y), nz = h(x, y - 1) - h(x, y + 1), l = Math.hypot(nx, 2, nz);
    v[k++] = x; v[k++] = h(x, y); v[k++] = y; v[k++] = nx / l; v[k++] = 2 / l; v[k++] = nz / l;
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, terrBuf); gl.bufferSubData(gl.ARRAY_BUFFER, 0, v);
}
function uploadOriginalTerrainClasses(){
  const types=wasm.terrain_types_ptr&&wasm.terrain_types_len?.()===MAP*MAP?u8(wasm.terrain_types_ptr(),MAP*MAP).slice():undefined;
  originalTerrain.updateMap({heights,types});
  originalWater.updateMap({heights,types});
}
const groundAt = (x, y) => {
  x = Math.min(MAP - 0.001, Math.max(0, x)); y = Math.min(MAP - 0.001, Math.max(0, y));
  const xi = x | 0, yi = y | 0, fx = x - xi, fy = y - yi, i = yi * HN + xi;
  const a = heights[i] + (heights[i + 1] - heights[i]) * fx, b = heights[i + HN] + (heights[i + HN + 1] - heights[i + HN]) * fx;
  return Math.max(0, a + (b - a) * fy);
};

const vaoW = gl.createVertexArray(); gl.bindVertexArray(vaoW);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-400, -400, 656, -400, -400, 656, 656, 656]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

const instBuf = gl.createBuffer();
function instanced(vao) {
  gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
  for (const loc of [3, 4, 5]) { gl.enableVertexAttribArray(loc); gl.vertexAttribDivisor(loc, 1); }
}
/** Point the instance attributes at instance `first` (WebGL2 has no base-instance draw). */
function instOffset(first) {
  for (const loc of [3, 4, 5]) gl.vertexAttribPointer(loc, 4, gl.FLOAT, false, STRIDE * 4, first * STRIDE * 4 + (loc - 3) * 16);
}
const mesh = buildModels();
const vaoM = gl.createVertexArray(); gl.bindVertexArray(vaoM);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer()); gl.bufferData(gl.ARRAY_BUFFER, mesh.data, gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, mesh.stride, 0);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, mesh.stride, 12);
gl.enableVertexAttribArray(2); gl.vertexAttribPointer(2, 4, gl.FLOAT, false, mesh.stride, 24);
gl.enableVertexAttribArray(6); gl.vertexAttribPointer(6,1,gl.FLOAT,false,mesh.stride,40);
instanced(vaoM);
const vaoQ = gl.createVertexArray(); gl.bindVertexArray(vaoQ);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-.5, -.5, .5, -.5, -.5, .5, .5, .5]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
instanced(vaoQ);

gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
const teamFlat = TEAM.flat();
for (const pr of [terrain, water, models, quads]) {
  gl.useProgram(pr.p);
  if (pr.u.uMap) gl.uniform1i(pr.u.uMap, 1);
  if (pr.u.uFog) gl.uniform1i(pr.u.uFog, 2);
  if (pr.u.uTerr) gl.uniform1i(pr.u.uTerr, 3);
  if (pr.u.uTeam) gl.uniform3fv(pr.u.uTeam, teamFlat);
}
const originalEnabled=referenceMode&&new URLSearchParams(location.search).get('assets')!=='procedural';
const originalAssets=await loadOriginalAssets(gl,originalEnabled);
const originalLabels=await loadOriginalLabels(rulesProfile,{definitionCount:renderNamespace.definitionCount,...(rulesProfile.ruleset===2?{manifestURL:'./local-original/labels-native-civ8.json'}:{})});
const originalModels=await loadOriginalModels(gl,originalEnabled,TEAM,(loaded,total)=>{
  $('play').textContent=`Originalgrafiken laden · ${Math.round(loaded/total*100)} %`;
},{assetMapping,packAnimations:rulesProfile.ruleset===2,dynamicSails:rulesProfile.ruleset===2&&typeof wasm.render_meta_version==='function',lightingSettings:{profile:rulesProfile.ruleset===1?'base':'native155'},...(rulesProfile.ruleset===2?{manifestURL:'./local-original/models/manifest-civ8.json',scaleManifestURL:'./local-original/model-scales-civ8.json',teamManifestURL:'./local-original/team-textures/manifest-civ8.json'}:{})});
const sourceSails=await loadSourceSailRuntime({wasm,namespace:renderNamespace,models:originalModels,enabled:originalEnabled&&rulesProfile.ruleset===2,decorateMetadata:metadata=>packMetadata?packMetadata.snapshot(metadata):metadata});
if(sourceSails?.metrics.error)console.warn('Source sails:',sourceSails.metrics.error);
sfx.configureOriginalDefinitions({wasm,bindings:originalModels.manifest?.models||[],...(rulesProfile.ruleset===2?{familyManifestURL:'native-families-civ8.json'}:{})});
const originalGround=await loadOriginalGround(gl,originalEnabled,{assetMapping});
const originalTerrain=await loadOriginalTerrain(gl,originalEnabled,TEAM);
originalTerrain.setLightingContext?.(originalModels.lighting);
const originalWater=await loadOriginalWater(gl,originalEnabled);
const seasonSession=createSeasonSession(wasm,originalTerrain,originalGround,originalModels);
const originalReflection=createOriginalReflection(gl);
let reflectionError=null;
await loadOriginalEpochIcons(originalAssets,originalModels.manifest);
if(qaMode){
  canvas.dataset.originalGroundBindings=String(originalGround.entries.size);
  canvas.dataset.originalGroundErrors=JSON.stringify(originalGround.errors);
  canvas.dataset.originalGroundPolicy=originalGround.policy;
  canvas.dataset.originalModelBindings=String(originalModels.manifest?.models?.length||0);
  canvas.dataset.originalEpochIcons=String([...originalAssets.epochIcons.values()].reduce((n,list)=>n+list.filter(row=>row.nativeExact).length,0));
  canvas.dataset.originalTerrainPolicy=originalTerrain.policy;
  canvas.dataset.originalTerrainErrors=JSON.stringify(originalTerrain.errors);
  canvas.dataset.originalWaterErrors=JSON.stringify(originalWater.errors);
}
applyOriginalInterface(originalAssets);
const economyBar=createEconomyBar(originalAssets,referenceMode);
const nativeHud=await loadNativeHud(originalAssets,referenceMode);
for(const pr of [terrain,water]){
  gl.useProgram(pr.p);gl.uniform1i(pr.u.uOriginalTerrain,4);gl.uniform1i(pr.u.uOriginal,originalAssets.enabled?1:0);
}
if(originalAssets.enabled){
  $('rules-note').textContent='Lokaler Nachbau mit mitgelieferten Originalgrafiken und Originalklängen. 15 Epochen, neun Rohstoffe, Forschung, Diplomatie und Festungen. Wähle deine Kartenaufdeckung für das neue Scharmützel.';
}
if(originalEnabled&&(!originalAssets.enabled||originalModels.available.size<67)){
  $('rules-note').textContent='Einige Originalmedien fehlen. Entpacke das vollständige Spielpaket und lasse den Spielordner zusammen. Bis dahin verwendet das Spiel die vorhandenen Ersatzgrafiken.';
}
$('play').disabled=false;$('play').textContent='Scharmützel starten';

// ---------------------------------------------------------------- state
const cam = { x: 128, y: 128, zoom: 28, heading:0 };
const mouse = { x: 0, y: 0, down: false, sx: 0, sy: 0, drag: false, pan: false, inside: false };
const keys = new Set();
let running = false, paused = false, speed = 1, acc = 0, last = 0, bench = 0;
let net = null;          // { ls: Lockstep, ch: RTCDataChannel } in multiplayer
let ghost = null;        // { id, size, name, turn } while placing a building
let placementKey='',lastWheelRotation=0;
let attackMode = false;  // A pressed: next left click is attack-move
let lastWorkSound = 0;
let ui = null, actSig = '', lastGroup = { n: -1, t: 0 }, lastClick = 0;
let simMs = 0, frameMs = 16.7, worstMs = 0, fogTick = -1, lastUi = 0, lastMini = 0, lastAuto = 0, waitSince = 0;
const SPEEDS = [0.5, 1, 2, 4];

// ---------------------------------------------------------------- camera
// At heading0 the eye sits south of and above the look-at point.
const view = { eye: [0, 0, 0], f: [0, -Math.sin(PITCH), -Math.cos(PITCH)], u: [0, Math.cos(PITCH), -Math.sin(PITCH)], vp: new Float32Array(16) };
function updateCamera() {
  cam.x = Math.min(MAP, Math.max(0, cam.x)); cam.y = Math.min(MAP, Math.max(0, cam.y));
  if(referenceMode){
    updateOriginalCamera(cam,view,innerWidth,innerHeight,groundAt);
    if(running)wasm.set_eye(...view.eye);
    return;
  }
  cam.zoom = Math.min(80, Math.max(6, cam.zoom));
  const aspect = innerWidth / innerHeight, dist = innerHeight / 2 / (cam.zoom * TANF);
  const e = view.eye = [cam.x, dist * Math.sin(PITCH), cam.y + dist * Math.cos(PITCH)];
  const f = view.f, u = view.u, near = Math.max(0.5, dist * 0.08), far = dist * 3.5 + 120, t = 1 / TANF;
  const tx = -e[0], ty = -(u[1] * e[1] + u[2] * e[2]), tz = f[1] * e[1] + f[2] * e[2];
  // projection * view, written out (right = +x, so the view matrix is sparse)
  const A = (far + near) / (near - far), B = 2 * far * near / (near - far), m = view.vp;
  m[0] = t / aspect; m[1] = 0; m[2] = 0; m[3] = 0;
  m[4] = 0; m[5] = t * u[1]; m[6] = -A * f[1]; m[7] = f[1];
  m[8] = 0; m[9] = t * u[2]; m[10] = -A * f[2]; m[11] = f[2];
  m[12] = t / aspect * tx; m[13] = t * ty; m[14] = A * tz + B; m[15] = -tz;
  if (running) wasm.set_eye(e[0], e[1], e[2]);   // so clicks land on what is drawn, even on aircraft
}
function panCameraScreen(right,down){
  const r=referenceMode?(view.groundRight||[1,0]):[1,0],f=referenceMode?(view.groundForward||[0,-1]):[0,-1];
  cam.x+=r[0]*right-f[0]*down;cam.y+=r[1]*right-f[1]*down;
}
function rotateView(delta){
  if(!referenceMode||!running)return;
  rotateOriginalCameraHeading(cam,delta);updateCamera();mouse.inside=false;
}
function ray(px, py) {
  if(referenceMode)return originalCameraRay(view,px,py,innerWidth,innerHeight);
  const nx = (px / innerWidth * 2 - 1) * TANF * innerWidth / innerHeight, ny = (1 - py / innerHeight * 2) * TANF;
  return [nx, view.f[1] + view.u[1] * ny, view.f[2] + view.u[2] * ny];
}
/** Screen pixel -> point on the y = 0 plane. */
function toPlane(px, py) {
  const d = ray(px, py), e = view.eye, t = -e[1] / Math.min(d[1], -1e-4);
  return [e[0] + d[0] * t, e[2] + d[2] * t];
}
/** Screen pixel -> point on the actual terrain (ray-marched against the height map). */
function toWorld(px, py) {
  const d = ray(px, py), e = view.eye, dy = Math.min(d[1], -1e-4);
  let t = Math.max(0,(3.2 - e[1]) / dy), prev = t;
  const end = -e[1] / dy;
  for (; t < end; prev = t, t += 0.2) if (e[1] + dy * t <= groundAt(e[0] + d[0] * t, e[2] + d[2] * t)) break;
  t = Math.min(t, end);
  for (let i = 0; i < 6; i++) { const m = (prev + t) / 2; if (e[1] + dy * m <= groundAt(e[0] + d[0] * m, e[2] + d[2] * m)) t = m; else prev = m; }
  return [e[0] + d[0] * t, e[2] + d[2] * t];
}
const corners = () => [toPlane(0, 0), toPlane(innerWidth, 0), toPlane(innerWidth, innerHeight), toPlane(0, innerHeight)];
function viewRect() {
  const c = corners();
  return [Math.min(...c.map(p => p[0])) - 2, Math.min(...c.map(p => p[1])) - 2, Math.max(...c.map(p => p[0])) + 2, Math.max(...c.map(p => p[1])) + 6];
}

function toast(msg) { const t = $('toast'); t.textContent = msg; t.style.opacity = 1; clearTimeout(toast.h); toast.h = setTimeout(() => t.style.opacity = 0, 2200); }
const buildingControls=createBuildingControls({rotate:rotateBuilding,cancel:cancelBuilding,automatic(){if(ghost){ghost.manual=false;placementKey='';updatePlacement();}}});
function cancelBuilding(){ghost=null;placementKey='';buildingControls.hide();delete canvas.dataset.placement;canvas.style.cursor='default';}
function rotateBuilding(direction,fine=false){
  if(!ghost)return;
  if(!wasm.cmd_place_rotated)return toast('Lade die aktuelle Spielfassung neu, um Gebäude zu drehen.');
  ghost.turn=nextBuildingTurn(ghost.turn,direction,fine);ghost.manual=true;placementKey='';updatePlacement();
}
function footprintSize(){return ghost?(wasm.footprint_size?.(ghost.id,ghost.turn)||ghost.size):0;}
function ghostTile() {
  let [wx, wy] = toWorld(mouse.x, mouse.y);
  let size=footprintSize();
  if(ghost.id===5&&!ghost.manual&&wasm.dock_orientation){
    size=wasm.footprint_size(ghost.id,0)||ghost.size;ghost.turn=0;
    const tx=Math.round(wx-size/2),ty=Math.round(wy-size/2),turn=wasm.dock_orientation(tx,ty);
    if(turn<8)ghost.turn=turn;
  }
  if(referenceMode&&(ghost.id===53||ghost.id===54)){
    const direction=ray(mouse.x,mouse.y),distance=(groundAt(wx,wy)-view.eye[1])/direction[1];
    const resource=originalModels.pickResource(view.eye,direction,distance+.02);
    if(resource)return [Math.floor(resource[0])-Math.floor(size/2),Math.floor(resource[1])-Math.floor(size/2)];
  }
  return [Math.round(wx - size / 2), Math.round(wy - size / 2)];
}
function updatePlacement(now=performance.now()){
  if(!ghost){buildingControls.hide();placementKey='';delete canvas.dataset.placement;return;}
  const [tx,ty]=ghostTile(),key=[ghost.id,ghost.turn,tx,ty,ui?.sel?.n,ui?.sel?.def,Math.floor(now/250)].join(':');
  if(key===placementKey)return;placementKey=key;
  const error=wasm.place_error_rotated?wasm.place_error_rotated(ghost.id,tx,ty,ghost.turn):wasm.place_error(ghost.id,tx,ty);
  const area=wasm.territory_at(tx+footprintSize()/2,ty+footprintSize()/2),owner=wasm.territory_owner(area);
  const notes=[];
  if(error===0&&ghost.id!==0&&owner===255)notes.push('Neutrales Land: doppelte Bauzeit');
  if(referenceMode&&error===0&&(ghost.id===0||ghost.id===56)&&owner===255){
    const info=ui?.diplomacy?.territories?.find(territory=>territory.id===area);
    if(info)notes.push(territoryClaimMessage(info,{building:true}));
  }
  const note=notes.join(' · ');
  buildingControls.update({name:ghost.name,id:ghost.id,turn:ghost.turn,error,size:footprintSize(),manual:ghost.manual,note});
  if(qaMode)canvas.dataset.placement=JSON.stringify({id:ghost.id,turn:ghost.turn,tx,ty,error,size:footprintSize()});
}

let researchWasPaused=false;
const sciencePanel=researchPanel({wasm,
 onOrder(){sfx.play(20);lastUi=0;},
 onOpen(){researchWasPaused=paused;if(!net)paused=true;keys.clear();mouse.inside=false;},
 onClose(){paused=researchWasPaused;$('pause').textContent=paused?'Fortsetzen':'Pause';lastUi=0;}
});
$('research-button').style.display=referenceMode?'':'none';
$('research-button').onclick=()=>{if(running&&ui)sciencePanel.open(ui);};
const diplomacy=diplomacyPanel({wasm,referenceMode,
 onOrder(){sfx.play(20);lastUi=0;},
 onOpen(){keys.clear();cancelMapGesture();ghost=null;attackMode=false;$('tip').style.display='none';},
 onClose(){keys.clear();mouse.inside=false;lastUi=0;},
 isPaused:()=>paused&&!net,
 resume(){if(paused&&!net)togglePause();},
 onTerritoryFocus(territory){cam.x=territory.x;cam.y=territory.y;updateCamera();toast(territory.name);}
});
const nativeDiplomacy=await enhanceOriginalDiplomacy(diplomacy,{wasm,enabled:referenceMode&&originalAssets.enabled});
document.body.dataset.originalDiplomacy=String(nativeDiplomacy.enabled);
const nativeBitmapFonts=await loadNativeFontUI({enabled:referenceMode&&originalAssets.enabled,hud:nativeHud,diplomacy:nativeDiplomacy});
document.body.dataset.originalBitmapFonts=String(nativeBitmapFonts.enabled);
$('diplomacy-button').onclick=()=>{if(running&&!bench)diplomacy.open();};

// ---------------------------------------------------------------- starting, saving, loading
function begin() {
  saveSession=Date.now();
  ghost = null; attackMode = false; paused = false; speed = 1; acc = 0; fogTick = -1; actSig = '@refresh'; worstMs = 0; ui = null;
  $('speed').textContent = '1×'; $('pause').textContent = 'Pause';
  for (const id of ['speed', 'pause', 'save']) $(id).style.display = net ? 'none' : '';
  $('menu').style.display = 'none'; $('end').style.display = 'none';
  $('camera-tools').hidden=!referenceMode;
  uploadTerrain();
  wasm.events_clear();
  running = true; last = performance.now(); lastAuto = last;
}
function start(nAi, seed, benchUnits, humans = 1, local = 0) {
  bench = benchUnits;
  seasonSession.configure(menuSeasonSettings());
  wasm.init(seed >>> 0, nAi, benchUnits, humans, local);
  if(!benchUnits&&civilizationSetup.enabled){
    const players=Math.max(2,Math.min(4,nAi+humans));
    applyMatchSettings(wasm,{civilizations:civilizationSetup.resolve(players),popLimit:+$('pop-limit').value,supplies:+$('supplies').value,epoch:+$('start-epoch').value},players);
  }else if(!benchUnits&&humans===1){
    wasm.skirmish_setup(+$('pop-limit').value, +$('supplies').value);
    if(referenceMode)wasm.skirmish_epoch(+$('start-epoch').value);
  }
  if(!benchUnits)wasm.skirmish_visibility(+$('map-visibility').value);
  cam.x = benchUnits ? MAP / 2 : wasm.start_x(); cam.y = benchUnits ? MAP / 2 : wasm.start_y(); cam.zoom = benchUnits ? 11 : 28;
  cam.heading=0;
  if(referenceMode)cam.nativeFraction=benchUnits?originalFractionForZoom(11,innerWidth,innerHeight):.2;
  begin();
}

const db = () => new Promise((res, rej) => {
  const r = indexedDB.open(rulesProfile.database, 1);
  r.onupgradeneeded = () => r.result.createObjectStore('saves');
  r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error);
});
const dbDo = async (mode, fn) => { const d = await db(); return new Promise((res, rej) => { const rq = fn(d.transaction('saves', mode).objectStore('saves')); rq.onsuccess = () => res(rq.result); rq.onerror = () => rej(rq.error); }); };
let saveSession=Date.now();
async function saveGame(silent) {
  if (!running || bench || net) { if (!silent) toast('Saving is only available in single-player games'); return; }
  const ptr = wasm.save(), bytes = u8(ptr, wasm.save_len()).slice();
  try {
    const record={bytes,cam:{...cam},when:Date.now(),epoch:ui?ui.ename:'',session:saveSession,seasons:seasonSession.settings,rules:rulesProfile.ruleset,profile:rulesProfile.id};
    const d=await db();
    await new Promise((resolve,reject)=>{
      const tx=d.transaction('saves','readwrite'),store=tx.objectStore('saves'),old=store.get('slot1');
      old.onsuccess=()=>{if(old.result&&old.result.session!==saveSession)store.put(old.result,'previous');store.put(record,'slot1');};
      tx.oncomplete=()=>{d.close();resolve();};tx.onabort=()=>{d.close();reject(tx.error);};tx.onerror=()=>reject(tx.error);
    });
    if (!silent) toast(`Spielstand gespeichert (${(bytes.length / 1024) | 0} KB)`);
    refreshContinue();
  } catch (e) { if (!silent) toast('Could not save: ' + e.message); }
}
async function loadGame(slot='slot1') {
  const s = await dbDo('readonly', st => st.get(slot)).catch(() => null);
  if (!s) return toast('No saved game found');
  if(!savedRulesProfileMatches(s,rulesProfile))return toast('Dieser Spielstand gehört zu einer anderen Regelversion. Wähle im Menü die passende Regelversion.');
  let seasons;try{seasons=savedSeasonSettings(s);}catch{return toast('Die Jahreszeiteneinstellungen des Spielstands sind ungültig.');}
  const ptr = wasm.load_alloc(s.bytes.length);
  u8(ptr, s.bytes.length).set(s.bytes);
  if (!wasm.load_apply()) return toast('Dieser Spielstand ist mit dieser Regelversion nicht kompatibel.');
  net = null; bench = 0; Object.assign(cam, s.cam);
  cam.heading=normalizeOriginalHeading(s.cam?.heading??0);
  seasonSession.configure(seasons);setSeasonControls(seasons);
  if(referenceMode)cam.nativeFraction=Number.isFinite(s.cam?.nativeFraction)?s.cam.nativeFraction:originalFractionForZoom(s.cam?.zoom??28,innerWidth,innerHeight);
  begin(); toast('Spielstand geladen');
}
async function refreshContinue() {
  const s = await dbDo('readonly', st => st.get('slot1')).catch(() => null);
  $('continue').style.display = savedRulesProfileMatches(s,rulesProfile) ? '' : 'none';
  if (s) $('continue').textContent = `Fortsetzen (${s.epoch || 'Spielstand'}, ${new Date(s.when).toLocaleString()})`;
  const previous=await dbDo('readonly',st=>st.get('previous')).catch(()=>null);
  $('previous-save').style.display=savedRulesProfileMatches(previous,rulesProfile)?'':'none';
  if(previous)$('previous-save').textContent=`Vorheriger Spielstand (${previous.epoch||'Scharmützel'})`;
}
$('previous-save').onclick=()=>loadGame('previous');
refreshContinue();

// ---------------------------------------------------------------- simulation stepping & sound
function advance(dt, now) {
  if (net) {
    acc = Math.min(acc + dt, TICK * 6);                 // allow a short catch-up burst, never a spiral
    let steps = 0; const t0 = performance.now();
    while (acc >= TICK && steps < 8 && net.ls.step()) { acc -= TICK; steps++; }
    if (steps) { simMs += ((performance.now() - t0) / steps - simMs) * 0.1; waitSince = now; }
  } else if (!paused) {
    acc += dt * speed;
    let steps = 0; const t0 = performance.now();
    while (acc >= TICK && steps < 8) { wasm.tick(); acc -= TICK; steps++; }
    if (steps) simMs += ((performance.now() - t0) / steps - simMs) * 0.1;
    if (acc > TICK) acc = TICK;                          // drop time instead of stalling the frame
  }
  // sound cues the simulation queued up
  const n = wasm.events_len();
  if (n) {
    const ev = f32(wasm.events_ptr(), n), range = 26 + 700 / cam.zoom;
    for (let i = 0; i < n; i += 3) {
      const kind = ev[i], dx = ev[i + 1] - cam.x, dy = ev[i + 2] - cam.y;
      if (kind >= 12) sfx.play(kind, 1, 0);
      else { const v = Math.max(0, 1 - Math.hypot(dx, dy) / range); sfx.play(kind, v * v * Math.min(1, 0.35 + cam.zoom / 40), dx / range * 1.4); }
    }
    wasm.events_clear();
  }
}
// A hidden tab gets no animation frames, but a multiplayer peer must keep simulating.
setInterval(() => {
  if (!running || !net || !document.hidden) return;
  const now = performance.now(), dt = Math.min(1.5, (now - last) / 1000); last = now;
  for (let t = dt; t > 0; t -= 0.3) advance(Math.min(t, 0.3), now);
}, 200);

// ---------------------------------------------------------------- frame loop
function frame(now) {
  const dt = Math.min(0.1, (now - last) / 1000); last = now;
  if (!running) { sfx.music(false); return; }
  sfx.music(!paused, ui?.epoch || 0);
  frameMs += (dt * 1000 - frameMs) * 0.05;
  worstMs = Math.max(worstMs * 0.995, dt * 1000);

  const pan = 22 / cam.zoom * 60 * dt * (keys.has('Shift') ? 2 : 1);
  const rotating=referenceMode&&!ghost&&keys.has('Control');
  if(rotating){
    // Installed hotkeys bind Ctrl+arrows; angular speed is an input adapter.
    const turn=(keys.has('ArrowLeft')?1:0)-(keys.has('ArrowRight')?1:0);
    if(turn)rotateOriginalCameraHeading(cam,turn*dt*Math.PI/2*(keys.has('Shift')?3:1));
    updateCamera();
  }
  let moveRight=0,moveDown=0;
  if ((!rotating&&keys.has('ArrowLeft')) || (mouse.inside && mouse.x < 4)) moveRight-=pan;
  if ((!rotating&&keys.has('ArrowRight')) || (mouse.inside && mouse.x > innerWidth - 5)) moveRight+=pan;
  if ((!rotating&&keys.has('ArrowUp')) || (mouse.inside && mouse.y < 4)) moveDown-=pan;
  if ((!rotating&&keys.has('ArrowDown')) || (mouse.inside && mouse.y > innerHeight - 5)) moveDown+=pan;
  panCameraScreen(moveRight,moveDown);
  updateCamera();

  advance(dt, now);
  const alpha = paused || (net && net.ls.waiting) ? 1 : Math.min(1, acc / TICK);
  if (!net && !bench && now - lastAuto > 180000) { lastAuto = now; saveGame(true); }

  const dpr = Math.min(+($('quality').value), devicePixelRatio || 1);
  const w = Math.round(innerWidth * dpr), h = Math.round(innerHeight * dpr);
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  gl.viewport(0, 0, w, h);

  // textures that change
  if (wasm.map_dirty()) { bindTexture(1,mapTexture); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, u8(wasm.map_ptr(), MAP * MAP * 4)); uploadOriginalTerrainClasses(); }
  if (wasm.terr_dirty()) { const p = wasm.terr_ptr(); bindTexture(3,territoryTexture); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, u8(p, MAP * MAP * 4)); }
  const tickNow = wasm.game_tick()>>>0;
  const seasonPhase=seasonSession.update();
  if(qaMode)canvas.dataset.season=JSON.stringify(seasonPhase);
  if (fogTick !== tickNow || fogTick < 0) {
    fogTick = tickNow;
    bindTexture(2,fogTexture); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RED, gl.UNSIGNED_BYTE, u8(wasm.fog_ptr(), MAP * MAP));
  }

  gl.clearColor(0.012, 0.03, 0.05, 1); gl.clearStencil(0); gl.stencilMask(255); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
  gl.enable(gl.DEPTH_TEST); gl.depthMask(true); gl.disable(gl.BLEND);
  const px = 1 / cam.zoom;

  const [x0, y0, x1, y1] = viewRect();
  let gid = -1, gx = 0, gy = 0;
  if (ghost) { gid = ghost.id; [gx, gy] = ghostTile(); }
  updatePlacement(now);
  const n = (wasm.render_rotated||wasm.render)(alpha, x0, y0, x1, y1, gid, gx, gy,ghost?.turn||0);
  const ranges = n ? new Uint32Array(wasm.memory.buffer, wasm.ranges_ptr(), renderNamespace.rangeLength) : null;
  const renderEpochs=[0,1,2,3].map(t=>wasm.stat(t,0));
  // Own visual clock: once before every scene pass. Pause/wait freezes cloth;
  // solo speed scales the explicit delta. Native helpers apply their own cap.
  let actorMetadata=sourceSails?.advance({count:n,instances:f32(wasm.inst_ptr(),n*STRIDE),epochs:renderEpochs,
    deltaSeconds:dt*(net?1:speed),runPhysics:!paused&&!(net&&net.ls.waiting)})??null;
  if(!actorMetadata&&packMetadata)actorMetadata=packMetadata.snapshot(packActorReader.snapshot({count:n,instances:f32(wasm.inst_ptr(),n*STRIDE),namespace:renderNamespace}));
  if(qaMode&&sourceSails)canvas.dataset.sourceSails=JSON.stringify(sourceSails.metrics);
  let reflection=null;
  if(originalTerrain.enabled&&!reflectionError){
    const scale=+$('quality').value>1?1:.5;
    try{
      reflection=originalReflection.render({vp:view.vp,eye:view.eye,width:Math.max(1,Math.round(w*scale)),height:Math.max(1,Math.round(h*scale)),
        drawScene:pass=>{
          originalTerrain.drawTerrain({vao:vaoT,px,fogTexture,territoryTexture,...pass});
          if(n)originalModels.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,epochs:renderEpochs,zoom:cam.zoom,actorMetadata,...pass});
        }});
    }catch(error){reflectionError=String(error);console.warn('Water reflection:',error);}
    if(qaMode){canvas.dataset.reflection=JSON.stringify(originalReflection.metrics);canvas.dataset.reflectionError=reflectionError||'';}
  }

  bindTexture(2,fogTexture);
  if(originalTerrain.enabled){
    originalTerrain.drawTerrain({vao:vaoT,vp:view.vp,px,fogTexture,territoryTexture});
  }else{
    bindTexture(1,mapTexture);bindTexture(3,territoryTexture);bindTexture(4,originalAssets.terrainTexture,gl.TEXTURE_2D_ARRAY);
    gl.useProgram(terrain.p); gl.bindVertexArray(vaoT);
    gl.uniformMatrix4fv(terrain.u.uVP, false, view.vp); gl.uniform1f(terrain.u.uPx, px);
    gl.drawElements(gl.TRIANGLES, MAP * MAP * 6, gl.UNSIGNED_INT, 0);
  }

  if (n) {
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, renderNamespace.fallbackInstances(f32(wasm.inst_ptr(), n * STRIDE)), gl.STREAM_DRAW);
    originalGround.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,vp:view.vp,
      epochs:renderEpochs,isUrbanAt:(x,z)=>!!wasm.ground_urban_at(x,z)});
    if(qaMode)canvas.dataset.originalGroundDrawn=String(originalGround.metrics.instances);
    if(!paused && now-lastWorkSound>500){
      lastWorkSound=now;
      const instances=f32(wasm.inst_ptr(),n*STRIDE),workerId=renderNamespace.unitRenderId(8),first=ranges[workerId*2],count=ranges[workerId*2+1];
      let closest=Infinity,hit=-1;
      for(let j=first;j<first+count;j++){
        const k=j*STRIDE,action=instances[k+11];if(action!==2 && action!==3 && action!==5)continue;
        const d=Math.hypot(instances[k]-cam.x,instances[k+1]-cam.y);if(d<closest){closest=d;hit=k;}
      }
      if(hit>=0 && closest<22){const action=instances[hit+11];sfx.play(action===2?23:action===3?24:25,.55*(1-closest/22),(instances[hit]-cam.x)/22);}
    }
    // Planar geometry shadows, stamped once per pixel with stencil. Terrain
    // slopes remain approximate; this adds silhouettes rather than dark circles.
    if(gl.getContextAttributes().stencil && +$('quality').value>1){
      gl.enable(gl.BLEND); gl.depthMask(false); gl.enable(gl.STENCIL_TEST);
      gl.stencilFunc(gl.EQUAL,0,255); gl.stencilOp(gl.KEEP,gl.KEEP,gl.INCR);
      gl.useProgram(models.p); gl.bindVertexArray(vaoM);
      gl.uniformMatrix4fv(models.u.uVP,false,view.vp); gl.uniform1i(models.u.uShadow,1);
      for(let id=0;id<renderNamespace.slotCount;id++){
        if(originalModels.available.has(id))continue;
        const template=renderNamespace.fallbackModelId(id);
        if(template<0||template>=104||(template>=29&&template<100))continue;
        const cnt=ranges[id*2+1]; if(!cnt||!mesh.count[template])continue;
        instOffset(ranges[id*2]); gl.drawArraysInstanced(gl.TRIANGLES,mesh.first[template],mesh.count[template],cnt);
      }
      originalModels.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,vp:view.vp,epochs:renderEpochs,zoom:cam.zoom,eye:view.eye,shadow:true,actorMetadata});
      gl.disable(gl.STENCIL_TEST); gl.depthMask(true);
    }
    // one instanced draw per model type
    gl.enable(gl.BLEND);
    gl.useProgram(models.p); gl.bindVertexArray(vaoM); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.uniformMatrix4fv(models.u.uVP, false, view.vp); gl.uniform1i(models.u.uShadow,0);
    for (let id = 0; id < renderNamespace.slotCount; id++) {
      if(originalModels.available.has(id))continue;
      const template=renderNamespace.fallbackModelId(id);
      if(template<0||template>=240)continue;
      const cnt = ranges[id * 2 + 1];
      if (!cnt || !mesh.count[template]) continue;
      instOffset(ranges[id * 2]);
      gl.drawArraysInstanced(gl.TRIANGLES, mesh.first[template], mesh.count[template], cnt);
    }
    originalModels.draw({data:f32(wasm.inst_ptr(),n*STRIDE),ranges,vp:view.vp,epochs:renderEpochs,zoom:cam.zoom,eye:view.eye,actorMetadata});
  }

  // Native base-water depth/alpha followed by the separate reflection overlay.
  gl.enable(gl.BLEND); gl.depthMask(false);
  if(originalWater.enabled){
    originalWater.drawWater({vp:view.vp,time:now/1000,fogTexture,reflectionTexture:reflection?.texture,reflectionVP:reflection?.vp});
    if(qaMode)canvas.dataset.originalWater=JSON.stringify(originalWater.metrics);
  }else if(originalTerrain.enabled){
    originalTerrain.drawWater({vao:vaoW,vp:view.vp,time:now/1000,eye:view.eye,fogTexture,reflectionTexture:reflection?.texture,reflectionVP:reflection?.vp});
  }else{
    bindTexture(1,mapTexture);bindTexture(4,originalAssets.terrainTexture,gl.TEXTURE_2D_ARRAY);
    gl.useProgram(water.p); gl.bindVertexArray(vaoW);
    gl.uniformMatrix4fv(water.u.uVP, false, view.vp); gl.uniform1f(water.u.uTime, now / 1000);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  if (n) {
    gl.useProgram(quads.p); gl.bindVertexArray(vaoQ); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.uniformMatrix4fv(quads.u.uVP, false, view.vp); gl.uniform1f(quads.u.uPx, px);
    gl.uniform3fv(quads.u.uRight,view.right||[1,0,0]); gl.uniform3fv(quads.u.uUp, view.u);
    for (const template of [243, 240, 242, 241]) {
      const id=renderNamespace.quadRenderId(template);            // shadows, rings, explosions, then bars on top of everything
      const cnt = ranges[id * 2 + 1];
      if (!cnt) continue;
      if (template === 241) gl.disable(gl.DEPTH_TEST);
      instOffset(ranges[id * 2]);
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, cnt);
    }
  }

  if (now - lastUi > 100) { lastUi = now; updateUi(n, now); }
  if (now - lastMini > 250) { lastMini = now; drawMini(); }
}

// ---------------------------------------------------------------- HUD
const HOT = ['Q', 'W', 'E', 'R', 'T', 'Z', 'X', 'C', 'V', 'B'];
function icon(id, px) {
  const c = document.createElement('canvas'); c.width = c.height = px;
  const original=originalIcon(originalAssets,id,ui?.epoch||0);
  if(original){
    const scale=Math.min(px/original.width,px/original.height),w=original.width*scale,h=original.height*scale;
    c.getContext('2d').drawImage(original,(px-w)/2,(px-h)/2,w,h);return c;
  }
  const fallbackId=renderNamespace.unitFallbackId(id);
  if(fallbackId>=0)c.getContext('2d').drawImage(iconAtlas, (fallbackId % 16) * CELL, ((fallbackId / 16) | 0) * CELL, CELL, CELL, 0, 0, px, px);
  return c;
}
const glyph = (ch, px) => { const d = document.createElement('div'); d.style.cssText = `font-size:${px * 0.66}px;height:${px}px;line-height:${px}px`; d.textContent = ch; return d; };
const GLYPH = { e: '⬆', r: '⚗', u: '⚓', p:'⇄' };
const LABELS = {'Worker':'Bürger','Town Center':'Stadtzentrum','House':'Haus','Farm':'Bauernhof','Barracks':'Kaserne','Workshop':'Werkstatt','Dock':'Hafen','Airfield':'Flugplatz','Watchtower':'Wachturm','Next Epoch':'Nächste Epoche','University':'Universität','Mill':'Mühle','Stable':'Stall','Market':'Markt','Temple':'Tempel','Warehouse':'Lagerhaus','Oil Rig':'Bohrturm','Uranium Mine':'Uranmine','HercFacility':'HERC-Fabrik','Fortress':'Festung'};
const label = (name,id,count=1) => originalLabels.name(id,ui?.epoch??0,LABELS[name]||name,count);
const actionUnitId = a => a&&(a.k==='b'||a.k==='t')?a.id:undefined;
const ACTIVITIES = ['Bereit','Unterwegs','Holz fällen','Erz abbauen','Nahrung sammeln','Bauen','Im Kampf'];
function costText(c) {return c.flatMap((value,slot)=>value?[`${value} ${!referenceMode&&slot===2?'Metall':RESOURCE_NAMES[slot]}`]:[]).join(' · ')||'Kostenlos';}

function updateUi(drawn, now) {
  const p = wasm.ui();
  ui = JSON.parse(dec.decode(u8(p, wasm.ui_len())));
  if(typeof wasm.pack_selected_count==='function'&&wasm.pack_selected_count()>0){
    ui.act.push({k:'p',id:0,name:'Einpacken',ok:true,tip:'Die ausgewählten Einheiten für den Marsch einpacken.'},
      {k:'p',id:1,name:'Auspacken',ok:true,tip:'Die ausgewählten Einheiten in Stellung bringen.'});
  }
  for (let i = 0; i < 3; i++) $('r' + i).textContent = ui.res[i];
  economyBar.update(ui);
  $('pop').textContent = `${ui.pop}/${ui.cap}`;
  $('epoch').textContent = ui.ename;
  $('science').textContent = referenceMode ? `${ui.research.points.toFixed(1)} TP` : '';
  $('terr').textContent = ui.terr;
  $('idle').textContent = `Freie Bürger: ${ui.idle}`;
  $('idle').style.visibility = ui.idle && !bench ? 'visible' : 'hidden';
  let status = `${Math.round(1000 / frameMs)} fps · worst ${worstMs.toFixed(0)} ms · sim ${simMs.toFixed(2)} ms/tick · ${ui.n} entities · ${drawn} drawn`;
  if(bench)status+=` · originals CPU ${(originalModels.metrics.colorMs+originalModels.metrics.shadowMs).toFixed(1)} ms / ${originalModels.metrics.calls} draws / ${(originalModels.metrics.triangles/1000).toFixed(0)}k tris / GPU ${originalModels.metrics.gpuMs.toFixed(1)}ms`;
  if (net) {
    if (net.ls.desync) status = '⚠ OUT OF SYNC with the other player · ' + status;
    else if (net.ls.alone) status = 'other player left · ' + status;
    else if (net.ls.waiting && now - waitSince > 600) status = 'waiting for the other player… · ' + status;
    else status = 'multiplayer · ' + status;
  }
  $('perf').textContent = status;

  const s = ui.sel, info = $('info');
  if (!s.n) info.innerHTML = bench ? '<h3>Stress test</h3><span class="dim">Two armies are kept at full strength and thrown at each other. Zoom and pan freely; you can still select and command the blue side.</span>'
    : `<span class="dim">Keine Einheit ausgewählt</span>${ui.techs.length ? '<div class="types">' + ui.techs.map(t => `<span>${t}</span>`).join('') + '</div>' : ''}`;
  else {
    let h = `<h3>${s.n > 1 && s.types.length > 1 ? s.n + ' Einheiten' : escapeLabelHtml(label(s.name,s.def,s.n)) + (s.n > 1 ? ' ×' + s.n : '')}</h3>`;
    if (s.n === 1) {
      h += `<div class="hp"><i style="width:${100 * s.hp / s.max}%"></i></div>${s.hp} / ${s.max} HP`;
      if (renderNamespace.isMobile(s.def)) h += `<div class="activity">${ACTIVITIES[s.activity] || 'Bereit'}</div>`;
      if (s.dmg) h += ` · ${s.dmg} dmg · range ${s.rng}`;
      if (s.built < 1) h += `<br>Im Bau: ${Math.round(s.built * 100)}%`;
    }
    if(referenceMode&&s.def===56&&s.cargomax)h+=`<br>Garnison: ${s.cargoUnits??s.cargo} ${(s.cargoUnits??s.cargo)===1?'Einheit':'Einheiten'} · ${s.cargo} / ${s.cargomax} Plätze<br>Angriffsbonus: +${s.garrisonAttackBonus??s.cargo*3} %${s.garrisonHealRate?` · Heilung ${s.garrisonHealRate} HP/s`:""}`;
    else if (s.cargomax) h += s.def===47 ? `<br>Forscher: ${s.cargo} / ${s.cargomax}<br>Forschung insgesamt: +${(ui.research.rate*60).toFixed(1)} TP / Minute` : s.def===53||s.def===54 ? `<br>Förderarbeiter: ${s.cargo} / ${s.cargomax}<br>+${(s.cargo*.75).toFixed(2)} ${s.def===53?'Öl':'Uran'} / Sekunde` : referenceMode&&(s.def===0||s.def===52) ? `<br>Einquartiert: ${s.cargo} / ${s.cargomax}<br>Abgabebonus: +${s.cargo*2} %` : `<br>Passagiere: ${s.cargo} / ${s.cargomax}`;
    if(s.carrying>0)h+=`<br>${s.returning?'Bringt zurück':'Trägt'}: ${s.carrying.toFixed(1)} ${RESOURCE_NAMES[s.carriedResource-1]}`;
    if (s.types.length > 1) h += '<div class="types">' + s.types.map(t => `<span>${escapeLabelHtml(label(t[2],t[0],t[1]))} ×${t[1]}</span>`).join('') + '</div>';
    if (s.q.length) h += `<div class="q" id="queue"></div><div class="hp"><i style="width:${s.qp * 100}%;background:#4aa3ff"></i></div>`;
    info.innerHTML = h;
    const q = $('queue');
    if (q) { for (const id of s.q) {const job=renderNamespace.queueJob(id);q.append(!job?glyph('?',30):job.kind===3?glyph('⬆',30):job.kind===1||job.kind===2?glyph('⚗',30):icon(job.value,30));} q.title = 'Rechtsklick oder Rücktaste: letzten Auftrag abbrechen'; }
  }

  const sig = ui.epoch+':'+ui.act.map(a => a.k + a.id).join(',');
  const acts = $('acts');
  if (sig !== actSig) {
    actSig = sig; acts.innerHTML = '';
    ui.act.forEach((a, i) => {
      const b = document.createElement('button'); b.className = 'act'; b.dataset.i = i;
      b.append(GLYPH[a.k] ? glyph(GLYPH[a.k], 44) : icon(a.id, 44));
      const sm = document.createElement('small'); sm.textContent = label(a.name,actionUnitId(a)); b.append(sm);
      if (HOT[i]) { const k = document.createElement('kbd'); k.textContent = HOT[i]; b.append(k); }
      b.onclick = () => doAct(i);
      b.oncontextmenu = e => { e.preventDefault(); wasm.cmd_cancel(); };
      b.onmouseenter = () => { const a = ui.act[i]; if (!a) return; const t = $('tip'); t.innerHTML = `<b>${escapeLabelHtml(label(a.name,actionUnitId(a)))}</b> <span class="c">${a.tp ? a.tp+' Technologiepunkte' : a.k === 'u'||a.k==='p' ? '' : costText(a.cost)}</span><br>${originalLabels.description(actionUnitId(a),ui.epoch)?escapeLabelHtml(originalLabels.description(actionUnitId(a),ui.epoch))+'<br>':''}${escapeLabelHtml(a.tip)}`; t.style.display = 'block'; };
      b.onmouseleave = () => $('tip').style.display = 'none';
      acts.append(b);
    });
  }
  [...acts.children].forEach((b, i) => b.classList.toggle('no', !ui.act[i].ok));
  sciencePanel.refresh(ui);
  diplomacy.refresh(ui);
  $('diplomacy-button').disabled=!!bench;
  nativeHud.update(ui,seasonSession.settings);

  if (ui.win && !bench && $('end').style.display !== 'flex') {
    diplomacy.close();
    $('endtext').textContent = ui.win === 1 ? 'Victory!' : 'Defeat';
    $('end').style.display = 'flex';
    sfx.play(ui.win === 1 ? 13 : 11);
  }
  nativeBitmapFonts.update();
}

function doAct(i) {
  const a = ui && ui.act[i];
  if (!a) return;
  if (a.k === 'b') { ghost = { id: a.id, size: a.size,name:label(a.name,a.id),turn:0,manual:false }; placementKey='';attackMode = false; sfx.play(20);updatePlacement();return; }
  if (a.k === 'u') { wasm.cmd_unload(); sfx.play(22); return; }
  if(a.k==='p'){if(typeof wasm.cmd_pack==='function'&&wasm.cmd_pack(a.id)){if(!selectedUnitVoice('order'))sfx.play(20);}else sfx.play(21);lastUi=0;return;}
  if (renderNamespace.wide?wasm.cmd_queue_job(a.id):wasm.cmd_train(a.id)) sfx.play(20);
  else { sfx.play(21); toast(a.ok ? 'Warteschlange voll' : a.k==='e' && referenceMode && ui.research.count<6 ? 'Zuerst sechs Technologien dieser Epoche erforschen' : a.tp && ui.research.points<a.tp ? 'Technologiepunkte fehlen – Bürger in Universitäten schicken' : a.k === 't' && ui.pop >= ui.cap ? 'Mehr Häuser bauen' : 'Rohstoffe fehlen'); }
}

const mini = $('mini'), mctx = mini.getContext('2d');
function drawMini() {
  const p = wasm.minimap();
  mctx.putImageData(new ImageData(new Uint8ClampedArray(wasm.memory.buffer, p, MAP * MAP * 4), MAP, MAP), 0, 0);
  const c = corners();
  mctx.strokeStyle = '#fff'; mctx.lineWidth = 1.5; mctx.beginPath();
  c.forEach((q, i) => i ? mctx.lineTo(q[0], q[1]) : mctx.moveTo(q[0], q[1]));
  mctx.closePath(); mctx.stroke();
}
function miniWorld(e) { const r = mini.getBoundingClientRect(); return [(e.clientX - r.left) / r.width * MAP, (e.clientY - r.top) / r.height * MAP]; }
mini.addEventListener('mousedown', e => {
  if(modalOpen())return;
  const [x, y] = miniWorld(e);
  if (e.button === 2) { wasm.cmd_context(x, y, 0); if (!selectedUnitVoice('order')) sfx.play(22); } else { cam.x = x; cam.y = y; mini.dragging = true; }
});
addEventListener('mousemove', e => { if (!modalOpen()&&mini.dragging) [cam.x, cam.y] = miniWorld(e); });
addEventListener('mouseup', () => mini.dragging = false);
mini.addEventListener('contextmenu', e => e.preventDefault());

// ---------------------------------------------------------------- input
const box = $('box');
const modalOpen=()=>$('cheat-dialog').open||$('research-dialog').open||diplomacy.isOpen();
function cancelMapGesture(){mouse.down=false;mouse.drag=false;mouse.pan=false;mouse.inside=false;mini.dragging=false;box.style.display='none';canvas.style.cursor='default';}
const fullscreen=createFullscreenController({buttons:[$('fullscreen'),$('menu-fullscreen')],notify:toast,
  onChange(){keys.clear();cancelMapGesture();updateCamera();}});
addEventListener('pointerdown', sfx.unlock, { once: false });
canvas.addEventListener('contextmenu', e => e.preventDefault());
canvas.addEventListener('mousedown', e => {
  mouse.x = e.clientX; mouse.y = e.clientY;
  if (!running||modalOpen()) return;
  const [wx, wy] = toWorld(e.clientX, e.clientY);
  if (e.button === 1) { mouse.pan = true; e.preventDefault(); return; }
  if (e.button === 2) {
    if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; return; }
    const workers=ui?.sel?.def===8||ui?.sel?.types?.some(type=>type[0]===8);
    let target=null;
    if(workers){
      const direction=ray(e.clientX,e.clientY),groundDistance=(groundAt(wx,wy)-view.eye[1])/direction[1];
      target=originalModels.pickResource(view.eye,direction,groundDistance+.02);
      if(referenceMode&&target){
        const kind=wasm.resource_kind(Math.floor(target[1])*MAP+Math.floor(target[0]));
        if(kind&&!wasm.resource_active(ui.local,kind-1)){toast('Dieses Vorkommen ist in dieser Epoche nicht nutzbar.');return;}
        if(kind===8||kind===9){toast(kind===8?'Öl benötigt einen Bohrturm mit einquartierten Bürgern.':'Uran benötigt eine Uranmine mit einquartierten Bürgern.');return;}
      }
    }
    wasm.cmd_context(target?.[0]??wx, target?.[1]??wy, 0);
    if (!selectedUnitVoice('order') && ui && ui.sel.n) sfx.play(22);
    return;
  }
  if (ghost) {
    const [tx, ty] = ghostTile(), err = wasm.place_error_rotated?wasm.place_error_rotated(ghost.id,tx,ty,ghost.turn):wasm.place_error(ghost.id,tx,ty);
    const placed=!err&&(wasm.cmd_place_rotated?wasm.cmd_place_rotated(ghost.id,tx,ty,ghost.turn):wasm.cmd_place(ghost.id,tx,ty));
    if (placed) { if (!selectedUnitVoice('build')) sfx.play(20); if (!e.shiftKey) cancelBuilding(); } else { sfx.play(21); toast(placementMessage(err||1,ghost.id)); }
    return;
  }
  if (attackMode) { wasm.cmd_context(wx, wy, 1); if (!selectedUnitVoice('attack')) sfx.play(22); attackMode = false; canvas.style.cursor = 'default'; return; }
  mouse.down = true; mouse.drag = false; mouse.sx = e.clientX; mouse.sy = e.clientY;
});
addEventListener('mousemove', e => {
  if(modalOpen()){mouse.inside=false;return;}
  if(ghost&&e.target!==canvas&&!mouse.pan){mouse.inside=false;return;}
  if (mouse.pan) panCameraScreen(-e.movementX/cam.zoom,-e.movementY/(cam.zoom*Math.sin(view.pitch??PITCH)));
  mouse.x = e.clientX; mouse.y = e.clientY; mouse.inside = true;
  if (mouse.down) {
    if (Math.abs(mouse.x - mouse.sx) + Math.abs(mouse.y - mouse.sy) > 6) mouse.drag = true;
    if (mouse.drag) {
      box.style.display = 'block';
      box.style.left = Math.min(mouse.x, mouse.sx) + 'px'; box.style.top = Math.min(mouse.y, mouse.sy) + 'px';
      box.style.width = Math.abs(mouse.x - mouse.sx) + 'px'; box.style.height = Math.abs(mouse.y - mouse.sy) + 'px';
    }
  }
});
document.addEventListener('mouseleave', () => mouse.inside = false);
addEventListener('mouseup', e => {
  if(modalOpen()){cancelMapGesture();return;}
  if (e.button === 1) mouse.pan = false;
  if (e.button !== 0 || !mouse.down) return;
  mouse.down = false; box.style.display = 'none';
  const add = e.shiftKey ? 1 : 0;
  if (mouse.drag) {
    // a screen rectangle is a trapezoid on the ground under a tilted camera
    const l = Math.min(mouse.x, mouse.sx), r = Math.max(mouse.x, mouse.sx), t = Math.min(mouse.y, mouse.sy), b = Math.max(mouse.y, mouse.sy);
    wasm.select_quad(...toWorld(l, t), ...toWorld(r, t), ...toWorld(r, b), ...toWorld(l, b), add);
  } else {
    const [wx, wy] = toWorld(mouse.x, mouse.y);
    const now = performance.now(), dbl = now - lastClick < 300; lastClick = now;
    wasm.select_point(wx, wy, add, dbl ? 1 : 0, ...viewRect());
  }
  if (!selectedUnitVoice('select')) sfx.play(20, .35);
  lastUi = 0;
});
canvas.addEventListener('wheel', e => {
  if(modalOpen())return;
  e.preventDefault();
  if(ghost&&e.shiftKey){const now=performance.now();if(now-lastWheelRotation>80&&e.deltaY){lastWheelRotation=now;rotateBuilding(e.deltaY>0?1:-1,true);}return;}
  const before = toPlane(e.clientX, e.clientY);
  const desiredZoom=cam.zoom*Math.exp(-e.deltaY*.0012);
  if(referenceMode)cam.nativeFraction=originalFractionForZoom(desiredZoom,innerWidth,innerHeight);
  else cam.zoom=desiredZoom;
  updateCamera();
  const after = toPlane(e.clientX, e.clientY);     // keep the point under the cursor fixed
  cam.x += before[0] - after[0]; cam.y += before[1] - after[1]; updateCamera();
}, { passive: false });

function setSpeed(i) { if (net) return; speed = SPEEDS[Math.min(SPEEDS.length - 1, Math.max(0, i))]; $('speed').textContent = speed + '×'; }
function togglePause() { if (net) return toast('No pausing in multiplayer'); paused = !paused; $('pause').textContent = paused ? 'Fortsetzen' : 'Pause'; }
function jumpToSelection() { const x = wasm.sel_center(0); if (x >= 0) { cam.x = x; cam.y = wasm.sel_center(1); } }
function goHome(){const territory=ui?.diplomacy?.territories?.find(area=>area.owner===ui.local);cam.x=territory?.x??wasm.start_x();cam.y=territory?.y??wasm.start_y();}
function idleWorker() { const i = wasm.select_idle_worker(); if (i !== -1 && i >>> 0 !== 0xffffffff) { cam.x = wasm.ent_x(i); cam.y = wasm.ent_y(i); lastUi = 0; selectedUnitVoice('select'); } }
function toggleMute() { sfx.setMuted(!sfx.isMuted()); $('mute').textContent = sfx.isMuted() ? 'Ton aus' : 'Ton an'; }
function selectedUnitVoice(action) {
  const ptr = wasm.ui(); const state = JSON.parse(dec.decode(u8(ptr, wasm.ui_len()))),selected=state.sel;
  if (selected.n) return sfx.acknowledge(selected.def, action,state.epoch);
  return false;
}

let cheatWasPaused = false;
function closeCheats() {
  if (!$('cheat-dialog').open) return;
  $('cheat-dialog').close(); paused = cheatWasPaused;
  $('pause').textContent = paused ? 'Fortsetzen' : 'Pause';
}
function openCheats() {
  if (!running) return;
  if (net) return toast('Cheats sind nur im Einzelspieler verfügbar.');
  if ($('cheat-dialog').open) return;
  cheatWasPaused = paused; paused = true; keys.clear(); mouse.inside = false;
  $('cheat-result').textContent = `Cheats ${wasm.cheat_status(0) ? 'aktiv' : 'inaktiv'} · Sofortbau ${wasm.cheat_status(1) ? 'an' : 'aus'} · Technologiepunkte: ${wasm.cheat_status(2)}`;
  $('cheat-dialog').showModal(); $('cheat-input').value = ''; $('cheat-input').focus();
}
$('cheats').onclick = openCheats;
$('cheat-close').onclick = closeCheats;
$('cheat-dialog').addEventListener('cancel', e => { e.preventDefault(); closeCheats(); });
$('cheat-form').addEventListener('submit', e => {
  e.preventDefault(); const code = parseCheat($('cheat-input').value);
  if (code === null) { $('cheat-result').textContent = 'Unbekannter Cheat. Verfügbare Befehle stehen oben.'; return; }
  const result = wasm.cheat(code);
  const messages = ['Cheats aktiviert.', 'Cheats deaktiviert; Sofortbau aus.', referenceMode?'+10.000 von jedem der neun Rohstoffe.':'+10.000 Nahrung, Holz und Metall.', result === 2 ? 'Sofortbau aus.' : 'Sofortbau an.', '+50 Technologiepunkte.'];
  $('cheat-result').textContent = result === 3 ? 'Nur im Einzelspieler verfügbar.' : result === 0 ? 'Zuerst icheat eingeben.' : messages[code] + ` Guthaben: ${wasm.cheat_status(2)} Technologiepunkte.`;
  $('cheat-input').value = ''; $('cheat-input').focus(); lastUi = 0;
});
addEventListener('keydown', e => {
  if(e.key==='F11'){e.preventDefault();if(!e.repeat)fullscreen.toggle();return;}
  if ($('cheat-dialog').open || $('research-dialog').open || diplomacy.isOpen()) return;
  if(e.target.closest?.('#building-controls,#camera-tools')&&['Enter',' '].includes(e.key))return;
  if (e.key === 'Enter' && !['INPUT','SELECT','TEXTAREA','BUTTON'].includes(e.target.tagName)) { e.preventDefault(); openCheats(); return; }
  if (['INPUT', 'SELECT', 'TEXTAREA'].includes(e.target.tagName)) return;
  keys.add(e.key);
  if (!running) return;
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if(ghost&&!e.metaKey&&(e.key==='Control'||k==='R'||e.code==='Comma'||e.code==='Period')){
    e.preventDefault();if(!e.repeat)rotateBuilding(e.code==='Comma'?-1:1,e.shiftKey);return;
  }
  if (e.code.startsWith('Digit')) {
    const n = +e.code[5];
    if (e.ctrlKey) { wasm.group_set(n); toast(`Group ${n} set`); }
    else if (wasm.group_get(n)) { const now = performance.now(); if (lastGroup.n === n && now - lastGroup.t < 400) jumpToSelection(); lastGroup = { n, t: now }; lastUi = 0; }
    e.preventDefault(); return;
  }
  if ((e.ctrlKey || e.metaKey) && k === 'S') { e.preventDefault(); saveGame(); return; }
  if(referenceMode&&!ghost&&e.ctrlKey&&['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key)){e.preventDefault();keys.add('Control');return;}
  if (e.ctrlKey || e.metaKey) return;
  const hi = HOT.indexOf(k);
  if (hi >= 0 && ui && ui.act[hi]) { doAct(hi); return; }
  switch (k) {
    case 'A': attackMode = true; ghost = null; canvas.style.cursor = 'crosshair'; break;
    case 'S': wasm.cmd_stop(); break;
    case 'U': wasm.cmd_unload(); break;
    case 'M': toggleMute(); break;
    case 'Delete': wasm.cmd_delete(); break;
    case 'Backspace': wasm.cmd_cancel(); break;
    case 'Escape': if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; } else wasm.select_none(); lastUi = 0; break;
    case 'F': idleWorker(); break;
    case 'F4': e.preventDefault();if(!bench)diplomacy.open();break;
    case 'H': goHome(); break;
    case ' ': jumpToSelection(); e.preventDefault(); break;
    case 'P': togglePause(); break;
    case '+': case '=': setSpeed(SPEEDS.indexOf(speed) + 1); break;
    case '-': setSpeed(SPEEDS.indexOf(speed) - 1); break;
    case 'F9': if (!net) { wasm.grant(1000); toast('+1000 resources (sandbox cheat)'); } break;
  }
});
addEventListener('keyup', e => keys.delete(e.key));
addEventListener('blur', () => { keys.clear(); cancelMapGesture(); });

$('idle').onclick = idleWorker;
$('camera-left').onclick=()=>rotateView(Math.PI/4);
$('camera-right').onclick=()=>rotateView(-Math.PI/4);
$('camera-north').onclick=()=>{if(referenceMode&&running){cam.heading=0;updateCamera();mouse.inside=false;}};
$('pause').onclick = togglePause;
$('mute').onclick = toggleMute;
$('music').onclick = () => { sfx.setMusicEnabled(!sfx.isMusicEnabled()); $('music').textContent = sfx.isMusicEnabled() ? 'Musik an' : 'Musik aus'; };
$('voices').onclick = () => { sfx.setSpeechEnabled(!sfx.isSpeechEnabled()); $('voices').textContent = sfx.isSpeechEnabled() ? 'Stimmen an' : 'Stimmen aus'; };
$('save').onclick = () => saveGame();
$('speed').onclick = () => setSpeed((SPEEDS.indexOf(speed) + 1) % SPEEDS.length);
function toMenu() {
  $('camera-tools').hidden=true;
  cancelBuilding();
  diplomacy.close();
  if (running && !net && !bench) saveGame(true);
  if (net) { try { net.ch.close(); net.pc.close(); } catch { /* already gone */ } net = null; }
  running = false; $('menu').style.display = 'flex'; $('end').style.display = 'none'; $('mp').style.display = 'none';
}
$('quit').onclick = toMenu; $('again').onclick = toMenu;
const seedOf = () => { const v = parseInt($('seed').value, 10); return Number.isFinite(v) && v > 0 ? v : (Math.random() * 2 ** 31) | 0 || 1; };
$('play').onclick = () => { net = null; start(+$('nai').value, seedOf(), 0); };
$('continue').onclick = () => loadGame();
for (const b of document.querySelectorAll('[data-bench]')) b.onclick = () => { net = null; start(1, seedOf(), +b.dataset.bench); };

// ---------------------------------------------------------------- multiplayer lobby
const mpStatus = t => $('mpstatus').textContent = t;
let pending = null;
function channel(local) {
  let ch = null, pc = null, rejected = false;
  return {
    open(c) {
      ch = c;
      ch.send(JSON.stringify({t:'hello', protocol:PROTOCOL_VERSION, rules:wasm.ruleset(),civilizationChoice:civilizationSetup.choice(0)}));
      mpStatus('Verbunden – Spielversion wird geprüft …');
    },
    message(d) {
      if (typeof d === 'string') {
        let m; try { m = JSON.parse(d);if(!m||typeof m!=='object'||Array.isArray(m))throw new Error('invalid message'); } catch { rejected = true; mpStatus('Ungültige Verbindungsdaten.'); ch.close(); return; }
        if(rejected)return;
        if (m.t === 'hello') {
          if (m.protocol !== PROTOCOL_VERSION || m.rules !== wasm.ruleset()) { rejected = true; mpStatus('Unterschiedliche Spielversion oder Regelprofile. Beide müssen denselben Stand öffnen.'); ch.close(); return; }
          if(civilizationSetup.enabled){try{civilizationChoice(m.civilizationChoice);}catch{return reject('Ungültige Völkerwahl des Mitspielers.');}}
          else if(m.civilizationChoice!=null)return reject('Die Völkerwahl passt nicht zur Regelversion.');
          if (local === 0 && !net) {
            try{
              const ai=+$('mpai').value,players=2+ai,choices=civilizationSetup.choices();
              if(choices)choices[1]=m.civilizationChoice;
              const cfg = {t:'start',protocol:PROTOCOL_VERSION,rules:wasm.ruleset(),seed:(seedOf()>>>0)||1,ai,visibility:+$('map-visibility').value,seasons:menuSeasonSettings(),
                civilizations:choices?resolveCivilizations(choices,players):null,popLimit:choices?+$('pop-limit').value:null,supplies:choices?+$('supplies').value:null,epoch:choices?+$('start-epoch').value:0};
              checkStart(cfg);ch.send(JSON.stringify(cfg));launch(cfg);
            }catch{return reject('Die Scharmützeleinstellungen konnten nicht übernommen werden.');}
          }
        } else if (m.t === 'start' && local === 1 && !net) {
          try{checkStart(m);launch(m);}catch{return reject('Ungültige Spielversion, Völker- oder Scharmützeleinstellungen.');}
        }
      }
      else if (net) net.ls.receive(d);
    },
    close() { if (net && !net.ls.alone) { net.ls.dropPeer(); toast('The other player disconnected'); } else if (!net && !rejected) mpStatus('Connection closed.'); },
    setPc(p) { pc = p; },
  };
  function reject(message){rejected=true;mpStatus(message);ch.close();}
  function checkStart(cfg){
    if(cfg.protocol!==PROTOCOL_VERSION||cfg.rules!==wasm.ruleset()||![0,1,2].includes(cfg.visibility)||!Number.isInteger(cfg.seed)||cfg.seed<1||cfg.seed>0xffffffff||![0,1,2].includes(cfg.ai))throw Error('Invalid new match settings');
    peerSeasonSettings(cfg.seasons);
    if(civilizationSetup.enabled){
      checkedCivilizations(cfg.civilizations,2+cfg.ai);
      if(![250,500,1000].includes(cfg.popLimit)||![0,1,2].includes(cfg.supplies)||!Number.isInteger(cfg.epoch)||cfg.epoch<0||cfg.epoch>=wasm.epoch_count())throw Error('Invalid skirmish setup');
    }else if(cfg.civilizations!==null||cfg.popLimit!==null||cfg.supplies!==null||cfg.epoch!==0)throw Error('Unsupported civilization setup');
  }
  function launch(cfg) {
    checkStart(cfg);
    wasm.init(cfg.seed >>> 0, cfg.ai, 0, 2, local);
    if(civilizationSetup.enabled)applyMatchSettings(wasm,cfg,2+cfg.ai);
    wasm.skirmish_visibility(cfg.visibility);
    seasonSession.configure(peerSeasonSettings(cfg.seasons));setSeasonControls(seasonSession.settings);
    net = { ch, pc, ls: null };
    net.ls = new Lockstep(wasm, local, buf => { if (ch.readyState === 'open') ch.send(buf); });
    bench = 0; cam.x = wasm.start_x(); cam.y = wasm.start_y(); cam.zoom = 28;
    cam.heading=0;
    if(referenceMode)cam.nativeFraction=.2;
    begin(); toast(local === 0 ? 'You are blue. Good luck!' : 'You are red. Good luck!');
  }
}
$('mpopen').onclick = () => { $('mp').style.display = $('mp').style.display === 'block' ? 'none' : 'block'; };
$('mphost').onclick = async () => {
  mpStatus('Creating invite…');
  try {
    const on = channel(0); pending = await rtcHost(on); on.setPc(pending.pc);
    $('mpout').value = pending.code; $('mpin').value = ''; $('mpin').placeholder = 'Paste the reply code from the other player here'; $('mpgo').textContent = 'Connect';
    $('mpgo').onclick = async () => { try { await pending.accept($('mpin').value); mpStatus('Connecting…'); } catch { mpStatus('That reply code is not valid.'); } };
    mpStatus('1. Send the invite code below to the other player.  2. Paste their reply code and press Connect.');
  } catch (e) { mpStatus('WebRTC is not available: ' + e.message); }
};
$('mpjoin').onclick = () => {
  $('mpout').value = ''; $('mpin').value = ''; $('mpin').placeholder = 'Paste the invite code from the host here'; $('mpgo').textContent = 'Create reply code';
  mpStatus('Paste the host’s invite code, then press “Create reply code”.');
  $('mpgo').onclick = async () => {
    try {
      const on = channel(1); pending = await rtcJoin($('mpin').value, on); on.setPc(pending.pc);
      $('mpout').value = pending.code; mpStatus('Send the reply code below back to the host. The game starts when they connect.');
    } catch { mpStatus('That invite code is not valid.'); }
  };
};
$('mpcopy').onclick = () => { $('mpout').select(); navigator.clipboard?.writeText($('mpout').value).then(() => mpStatus('Copied to the clipboard.'), () => {}); };

// ?bench=2000 or ?seed=42 start immediately (handy for profiling and screenshots)
const qs = new URLSearchParams(location.search);
if (qs.has('bench')) start(1, +qs.get('seed') || 7, +qs.get('bench') || 2000);
else if (qs.has('seed')) start(+qs.get('ai') || 1, +qs.get('seed') || 1, 0);
// ?warm=600 fast-forwards that many ticks; ?zoom= ?x= ?y= place the camera
for (let i = +qs.get('warm') || 0; i > 0 && running; i--) wasm.tick();
for (const k of ['zoom', 'x', 'y']) if (qs.has(k)) cam[k] = +qs.get(k);
window.openepoch = { wasm, rulesProfile, renderNamespace, cam, gl, frame, start, saveGame, loadGame, toWorld, channel, seasonSession, nativeDiplomacy, nativeBitmapFonts, get net() { return net; } };
const loop = now => { requestAnimationFrame(loop); frame(now); };
requestAnimationFrame(loop);
