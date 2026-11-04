// Complete persisted-state projection across the explicitly different5/6 DTOs.
import {saveLayout} from './save-fixture.mjs';
import{unwrapPackSave}from'./pack8-save-envelope.mjs';
export const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
export const saveCore=w=>unwrapPackSave(save(w)).core;
export const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const building=id=>id<8||(id>=47&&id<57)||id===59||id===60;
function oldJob(q){if(q===255)return[3,0];if(q>=64&&q<244)return[1,q-64];if(q>=200)return[2,q-200];return[0,q];}
function newJob(q){if(q===0x30000)return[3,0];if(q>=0x10000&&q<0x100b4)return[1,q-0x10000];if(q>=0x20000)return[2,q-0x20000];return[0,q];}
export function persistedState(b,{ignoreFlowCache=true}={}){
 b=unwrapPackSave(b).core;
 const l=saveLayout(b),d=l.d,wide=l.roster6,stride=d.getUint32(4,true),pstride=d.getUint32(8,true),high=d.getUint32(32,true),entities=36+4*pstride,rows=[];
 for(let i=0;i<high;i++){
  const at=entities+i*stride,def=wide?d.getUint16(at+2,true):b[at+1],row={alive:b[at],def,team:b[at+(wide?4:2)],gen:d.getUint16(at+(wide?6:4),true)};
  const floats={x:8,y:12,px:16,py:20,vx:24,vy:28,rot:32,prot:36,hp:40,cd:44,ox:52,oy:56,built:80,qt:wide?108:92,rx:wide?112:96,ry:wide?116:100,reveal_t:wide?124:108,carry:wide?132:116,flash:wide?136:120};
  for(const[k,o]of Object.entries(floats))row[k]=d.getUint32(at+o,true); // IEEE bits preserve negativezero and exact source arithmetic.
  for(const[k,o]of Object.entries({otgt:60,otile:64,tgt:68,fkey:72}))row[k]=d.getUint32(at+o,true);
  Object.assign(row,{order:b[at+48],sel:b[at+78],qn:b[at+(wide?104:89)],revealed:b[at+(wide?120:104)],stuck:b[at+(wide?128:112)],prev:b[at+(wide?140:124)],ncargo:b[at+(wide?172:145)]});
  if(!ignoreFlowCache)row.fslot=d.getUint16(at+76,true);
  row.q=Array.from({length:5},(_,k)=>{const q=wide?d.getUint32(at+84+4*k,true):b[at+84+k];return building(def)?(wide?newJob(q):oldJob(q)):q;});
  row.cargo=Array.from({length:10},(_,k)=>wide?d.getUint16(at+142+2*k,true):b[at+125+k]);row.cargo_hp=Array.from(b.subarray(at+(wide?162:135),at+(wide?172:145)));rows.push(row);
 }
 const players=b.slice(36,entities);for(let i=0;i<4;i++)players.fill(0,i*pstride+34,i*pstride+36); // Only reprC padding, never a field.
 return {head:Array.from({length:6},(_,k)=>d.getUint32(12+4*k,true)),players:Array.from(players),entities:rows,world:Array.from(b.subarray(entities+stride*high,l.dataEnd))};
}
