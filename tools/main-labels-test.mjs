// Evaluate the production HUD update and its actual handlers with DOM surfaces substituted.
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {originalLabelCatalog,escapeLabelHtml} from '../docs/original-labels.js';
const mainFile=process.argv[2]?new URL(process.argv[2],'file:///'+process.cwd().replaceAll('\\','/')+'/'):new URL('../docs/main.js',import.meta.url);
const bytes=await readFile(mainFile),source=bytes.toString();
const section=source.slice(source.indexOf("const HOT ="),source.indexOf('\nfunction doAct(i)'));
assert.ok(section.includes('originalLabels.name')&&section.includes('escapeLabelHtml'));
const data=JSON.parse(await readFile(new URL('../docs/local-original/labels-native.json',import.meta.url)));
let checks=0;const check=(value,note)=>{assert.ok(value,note);checks++;};
class Node{
 constructor(tag='div'){this.tagName=tag;this.children=[];this.style={};this.dataset={};this.textContent='';this.classList={toggle(){}};}
 set innerHTML(value){this.html=value;this.children=[];}get innerHTML(){return this.html||'';}
 append(...nodes){this.children.push(...nodes);}getContext(){return {drawImage(){}};}
}
for(const profile of [{id:'base',ruleset:1,reference:true},{id:'up15',ruleset:2,reference:true},{id:'own',ruleset:0,reference:false}]){
 const nodes=new Map(['r0','r1','r2','pop','epoch','science','terr','idle','perf','info','acts','tip','diplomacy-button','end','endtext'].map(id=>[id,new Node()]));
 let payload,ordered=[],cancels=0;
 const context=vm.createContext({document:{createElement:tag=>new Node(tag)},$:id=>nodes.get(id),originalLabelCatalog,escapeLabelHtml,
  originalLabels:originalLabelCatalog(data,profile),originalAssets:{},originalIcon:()=>null,iconAtlas:{},CELL:64,
  wasm:{ui:()=>0,ui_len:()=>0,cmd_cancel(){cancels++;}},u8:()=>new TextEncoder().encode(JSON.stringify(payload)),dec:new TextDecoder(),
  referenceMode:profile.reference,RESOURCE_NAMES:['Nahrung','Holz','Eisen'],frameMs:16,worstMs:17,simMs:0.1,bench:0,net:null,
  economyBar:{update(){}},sciencePanel:{refresh(){}},diplomacy:{refresh(){},close(){}},nativeHud:{update(){}},seasonSession:{settings:{}},
  doAct:index=>ordered.push(index),sfx:{play(){}},console});
 vm.runInContext('let ui=null,actSig="";'+section+'\nglobalThis.refresh=()=>updateUi(0,0);',context);
 function update(epoch,id,count=1,types=[[id,count,'Musketeer']]){
  payload={res:[12,34,56],pop:3,cap:10,ename:'Epoche',terr:2,idle:0,epoch,n:15,research:{points:50,rate:0,count:0},techs:[],win:0,
   sel:{n:count,name:'Musketeer',def:id,hp:10,max:20,built:1,q:[],types,activity:0},
   act:[{k:'t',id,name:'Musketeer',cost:[0,1,0],ok:true,tip:'120 HP',size:0},{k:'r',id:64,name:'Forschung',cost:[0,0,0],tp:5,ok:true,tip:'Forschungswerte'}]};
  context.refresh();return nodes.get('info').innerHTML;
 }
 for(const epoch of [0,2,6,10,14]){
  const html=update(epoch,15),catalog=originalLabelCatalog(data,profile),name=catalog.name(15,epoch,'Musketeer');
  check(html.includes('<h3>'+escapeLabelHtml(name)+'</h3>'),profile.id+' actual selection name at epoch'+epoch);
  const buttons=nodes.get('acts').children;
  check(buttons[0].children[1].textContent===name,'actual production-button name');
  check(buttons[1].children[1].textContent==='Forschung','research ID never receives unit text');
  buttons[0].onclick();check(ordered.at(-1)===0,'production handler kept original action index');
  buttons[0].onmouseenter();check(nodes.get('tip').innerHTML.startsWith('<b>'+escapeLabelHtml(name)+'</b>'),'actual hover title');
  check(nodes.get('tip').innerHTML.endsWith('120 HP'),'current runtime stats remain available');
  if(profile.reference){const description=catalog.description(15,epoch);check(!!description&&nodes.get('tip').innerHTML.includes(escapeLabelHtml(description)+'<br>120 HP'),'original source description reaches hover');}
  buttons[0].onmouseleave();check(nodes.get('tip').style.display==='none','mouseleave preserved');
  let prevented=false;buttons[0].oncontextmenu({preventDefault(){prevented=true;}});check(prevented&&cancels>0,'cancel listener preserved');
 }
 const catalog=originalLabelCatalog(data,profile);
 check(update(10,15,3).includes(escapeLabelHtml(catalog.name(15,10,'Musketeer',3))+' ×3'),'selection plural uses source class');
 const mixed=update(10,15,3,[[15,2,'Musketeer'],[8,1,'Worker']]);
 check(mixed.includes('<h3>3 Einheiten</h3>'),'mixed-selection heading retained');
 check(mixed.includes(escapeLabelHtml(catalog.name(15,10,'Musketeer',2))+' ×2'),'mixed-selection type resolves by ID');
 payload.sel.n=0;payload.techs=[];context.refresh();check(nodes.get('info').innerHTML.includes('Keine Einheit ausgewählt'),'empty selection preserved');
}
const report={format:'openepoch-native5-label-main-tests-v1',status:'passed',checks,mainSha256:createHash('sha256').update(bytes).digest('hex'),
 scope:'Actual Main HUD updater and button/hover/cancel handlers executed in vm; DOM, WASM UI transport and rendering surfaces substituted. No browser or original executable launched.'};
await writeFile(new URL('../../EE2-Research/analysis/native5-label-main-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
