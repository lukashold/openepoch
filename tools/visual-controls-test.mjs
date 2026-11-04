// Actual main camera/input/save functions, real WASM, substituted DOM/IndexedDB.
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {createHash} from 'node:crypto';
import {updateOriginalCamera,originalCameraRay,originalFractionForZoom,normalizeOriginalHeading,rotateOriginalCameraHeading} from '../docs/original-camera.js';
import {createSeasonSession,savedSeasonSettings} from '../docs/season-session.js';
import {normalizeSeasonSettings} from '../docs/original-seasons.js';
import {resolveRulesProfile,savedRulesProfileMatches} from '../docs/rules-profile.js';

const main=await fs.readFile(new URL('../docs/main.js',import.meta.url),'utf8');
const bytes=await fs.readFile(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const wasm=(await WebAssembly.instantiate(bytes,{})).instance.exports;
const segment=(start,end)=>{const a=main.indexOf(start),b=main.indexOf(end,a);assert.ok(a>=0&&b>a,start);return main.slice(a,b);};
const camera=segment('function updateCamera()', 'function toast(');
const input=segment("addEventListener('keydown',", "$('idle').onclick");
const mouseEnd=main.indexOf("document.addEventListener('mouseleave'");
const mouseMove=main.slice(main.lastIndexOf("addEventListener('mousemove',",mouseEnd),mouseEnd);
const save=segment('async function saveGame(', 'async function refreshContinue(');
const start=segment('function start(', 'const db =');
const cameraStep=segment('  const pan = 22 / cam.zoom', '  advance(dt, now);');
const controls=segment('function menuSeasonSettings()', "$('season-options').hidden");
const callbacks=new Map(),elements=new Map(),records=new Map();
const $=id=>{if(!elements.has(id))elements.set(id,{value:'0',checked:false,disabled:false,open:false,hidden:true,style:{}});return elements.get(id);};
for(const [id,value]of Object.entries({'pop-limit':'500',supplies:'1','start-epoch':'0','map-visibility':'2','season-start':'6','season-length':'5'}))$(id).value=value;
$('season-cycle').checked=true;
const clone=value=>structuredClone(value);
const db=async()=>({close(){},transaction(){
 const tx={objectStore:()=>({
  get(key){const request={result:clone(records.get(key))};queueMicrotask(()=>{request.onsuccess?.();queueMicrotask(()=>tx.oncomplete?.());});return request;},
  put(value,key){records.set(key,clone(value));},
 })};return tx;
}});
const phaseCalls={terrain:[],ground:[],models:[]};
const renderer=name=>({setSeasonPhase:phase=>phaseCalls[name].push(phase)});
const seasonSession=createSeasonSession(wasm,renderer('terrain'),renderer('ground'),renderer('models'));
let begins=0,selectionJumps=0;
const context=vm.createContext({wasm,$,db,seasonSession,savedSeasonSettings,normalizeSeasonSettings,rulesProfile:resolveRulesProfile('http://localhost/?rules=ee2&profile=base'),savedRulesProfileMatches,
 updateOriginalCamera,originalCameraRay,originalFractionForZoom,normalizeOriginalHeading,rotateOriginalCameraHeading,
 innerWidth:1280,innerHeight:720,referenceMode:true,MAP:256,PITCH:55*Math.PI/180,TANF:Math.tan(35*Math.PI/360),
 groundAt:()=>0,performance,queueMicrotask,
 dbDo:async(mode,action)=>action({get:key=>clone(records.get(key))}),
 toast(){},refreshContinue(){},begin(){begins++;},
 addEventListener:(type,callback)=>callbacks.set(type,callback),
 diplomacy:{isOpen:()=>false,open(){}},fullscreen:{toggle(){}},
 modalOpen:()=>false,cancelMapGesture(){},jumpToSelection(){selectionJumps++;},
 openCheats(){},rotateBuilding(){},setSpeed(){},doAct(){},idleWorker(){},goHome(){},toggleMute(){},togglePause(){},
 SPEEDS:[.5,1,2,4],HOT:[],
});
vm.runInContext(`
 const cam={x:51,y:51,zoom:28,nativeFraction:.2,heading:0};
 const view={eye:[0,0,0],f:[0,-1,-1],u:[0,1,-1],vp:new Float32Array(16)};
 const mouse={x:640,y:360,sx:0,sy:0,inside:false,pan:false,down:false};
 const keys=new Set(),box={style:{}},u8=(ptr,len)=>new Uint8Array(wasm.memory.buffer,ptr,len);
 let running=true,bench=0,net=null,ghost=null,ui=null,saveSession=1,lastUi=0,attackMode=false;
 ${controls}\n${camera}\n${start}\n${save}\n${input}\n${mouseMove}
 globalThis.api={cam,view,mouse,keys,start,saveGame,loadGame,updateCamera,panCameraScreen,rotateView,
  setHeading:value=>{cam.heading=value;updateCamera();},setDimensions:(w,h)=>{innerWidth=w;innerHeight=h;},
  cameraStep:dt=>{${cameraStep}},getRay:ray,toWorld,toPlane};
`,context,{filename:'main.js production camera/input/save extraction'});
const api=context.api;let checks=0;
const check=(value,message)=>{assert.ok(value,message);checks++;};
const close=(actual,expected,message)=>{assert.ok(Math.abs(actual-expected)<1e-8,`${message}: ${actual}/${expected}`);checks++;};
const event=(key,extra={})=>({key,code:'',ctrlKey:false,metaKey:false,shiftKey:false,repeat:false,target:{tagName:'CANVAS',closest:()=>null},preventDefault(){this.prevented=true;},...extra});
api.start(1,7,0);api.updateCamera();const initialHash=wasm.state_hash();
for(const degrees of [0,45,90,135,180,225,270,315]){
 api.setHeading(degrees*Math.PI/180);const before={x:api.cam.x,y:api.cam.y};
 const r=api.view.groundRight,f=api.view.groundForward;api.panCameraScreen(3,2);
 close(api.cam.x-before.x,r[0]*3-f[0]*2,'Pan follows displayed horizontal axis');
 close(api.cam.y-before.y,r[1]*3-f[1]*2,'Pan follows displayed depth axis');
 api.updateCamera();
 const ray=api.getRay(640,360);for(let i=0;i<3;i++)close(ray[i],api.view.f[i],'Center screen ray follows the rotated forward basis');
 const position={x:api.cam.x,y:api.cam.y};api.mouse.pan=true;
 callbacks.get('mousemove')({clientX:650,clientY:370,movementX:12,movementY:7,target:{}});
 close(api.cam.x-position.x,r[0]*(-12/api.cam.zoom)-f[0]*(-7/(api.cam.zoom*Math.sin(api.view.pitch))),'Middle drag follows screen axes');
 close(api.cam.y-position.y,r[1]*(-12/api.cam.zoom)-f[1]*(-7/(api.cam.zoom*Math.sin(api.view.pitch))),'Middle drag depth follows screen axes');
 api.mouse.pan=false;api.mouse.inside=false;
}
check(wasm.state_hash()===initialHash,'Camera movement and native eye updates do not alter deterministic gameplay');
api.setHeading(0);callbacks.get('keydown')(event('Control',{ctrlKey:true}));
const left=event('ArrowLeft',{ctrlKey:true});callbacks.get('keydown')(left);const prior={x:api.cam.x,y:api.cam.y};api.cameraStep(1);
close(api.cam.heading,Math.PI/2,'Ctrl+left rotates continuously');check(left.prevented,'Camera arrow shortcut prevents browser navigation');
close(api.cam.x,prior.x,'Camera rotation does not pan x');close(api.cam.y,prior.y,'Camera rotation does not pan y');
callbacks.get('keyup')(event('ArrowLeft'));callbacks.get('keyup')(event('Control'));
const buttonEvent=event(' ',{target:{tagName:'BUTTON',closest:selector=>selector.includes('#camera-tools')?{}:null}});
callbacks.get('keydown')(buttonEvent);check(selectionJumps===0,'Space on camera buttons keeps native button activation without selection jump');
callbacks.get('keydown')(event('ArrowRight'));api.cameraStep(.05);callbacks.get('keyup')(event('ArrowRight'));
check(api.cam.y<prior.y,'Arrow-right after a quarter turn moves towards displayed right');
api.cam.heading=1.234;api.start(1,7,0);check(api.cam.heading===0,'New skirmish starts facing north');api.updateCamera();
api.setHeading(1.234);const savedHeading=api.cam.heading;
await api.saveGame(true);check(records.has('slot1'),'Actual saveGame stores a complete record');
check(records.get('slot1').cam.heading===savedHeading,'Actual saveGame persists heading');
const savedHash=wasm.state_hash();for(let i=0;i<20;i++)wasm.tick();api.setHeading(4);
await api.loadGame();close(api.cam.heading,savedHeading,'Actual loadGame restores heading');check(wasm.state_hash()===savedHash,'Actual load restores the saved WASM state');
const legacy=clone(records.get('slot1'));delete legacy.cam.heading;records.set('slot1',legacy);api.setHeading(2);
await api.loadGame();check(api.cam.heading===0,'Old saves reset heading instead of retaining the previous view');
const phase=seasonSession.update();for(const name of Object.keys(phaseCalls))check(phaseCalls[name].at(-1)===phase,'Production session forwards the same phase to '+name);
check(begins===4,'Actual start/load functions enter exactly four sessions');
const sha=value=>createHash('sha256').update(value).digest('hex');
const report={format:'openepoch-visual-controls-tests-v1',passed:true,checks,wasmSha256:sha(bytes),mainSha256:sha(main),
 cameraSha256:sha(await fs.readFile(new URL('../docs/original-camera.js',import.meta.url))),
 scope:'Exact production main camera, keyboard, mouse, start, save and load functions in vm; real WASM and serializer. DOM, IndexedDB transaction transport and begin UI hook are substituted. No browser/native app launched.'};
const reportName=process.argv.find(arg=>arg.startsWith('--report='))?.slice(9)||'visual-controls-tests.json';
if(!/^[a-z0-9-]+\.json$/i.test(reportName))throw Error('Invalid report filename');
await fs.writeFile(new URL('../../EE2-Research/analysis/'+reportName,import.meta.url),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report));
