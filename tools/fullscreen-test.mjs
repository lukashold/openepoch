import assert from 'node:assert/strict';
import {createFullscreenController} from '../docs/fullscreen.js';
class Events{
 constructor(){this.listeners=new Map();this.attrs={};}
 addEventListener(name,fn){this.listeners.set(name,fn);}
 removeEventListener(name,fn){if(this.listeners.get(name)===fn)this.listeners.delete(name);}
 setAttribute(key,value){this.attrs[key]=value;}
 fire(name){return this.listeners.get(name)?.();}
}
const doc=new Events(),a=new Events(),b=new Events(),messages=[],changes=[];
let enter=0,exit=0;
doc.documentElement={requestFullscreen:async()=>{enter++;doc.fullscreenElement=doc.documentElement;doc.fire('fullscreenchange');}};
doc.exitFullscreen=async()=>{exit++;doc.fullscreenElement=null;doc.fire('fullscreenchange');};
const control=createFullscreenController({document:doc,buttons:[a,b],notify:m=>messages.push(m),onChange:state=>changes.push(state)});
assert.equal(control.active,false);assert.equal(a.textContent,'Vollbild');assert.equal(b.attrs['aria-pressed'],'false');
assert.equal(await a.fire('click'),true);assert.equal(enter,1);assert.equal(control.active,true);
assert.equal(b.textContent,'Vollbild beenden');assert.equal(a.attrs['aria-pressed'],'true');assert.equal(a.disabled,false);
await b.fire('click');assert.equal(exit,1);assert.equal(control.active,false);
await control.toggle();doc.fullscreenElement=null;doc.fire('fullscreenchange');
assert.equal(a.textContent,'Vollbild','native Escape exit updates both buttons');assert.equal(b.attrs['aria-pressed'],'false');
let finish;doc.documentElement.requestFullscreen=()=>new Promise(resolve=>{enter++;finish=resolve;});
const pending=control.toggle();assert.equal(a.disabled,true);assert.equal(await control.toggle(),false);assert.equal(enter,3,'double click is not a second fullscreen request');
finish();await pending;assert.equal(a.disabled,false);
doc.documentElement.requestFullscreen=async()=>{throw Error('NotAllowedError');};
assert.equal(await control.toggle(),false);assert.equal(messages.length,1);assert.match(messages[0],/blockiert/);assert.equal(control.active,false);
delete doc.documentElement.requestFullscreen;assert.equal(await control.toggle(),false);assert.equal(messages.length,2);
control.dispose();assert.equal(a.listeners.size,0);assert.equal(doc.listeners.size,0);assert.ok(changes.includes(true)&&changes.includes(false));
console.log('PASS fullscreen: real API transitions, both controls, native Escape, concurrent gestures, rejection/unsupported feedback and listener disposal. Physical fullscreen is checked in the browser.');
