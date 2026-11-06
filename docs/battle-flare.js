import {loadOriginalFullmapArt} from './original-fullmap-art.js';
import {createFullmapSpriteCache} from './fullmap-sprites.js';
import {createFlareAnimation} from './player-flare.js?v=native24';

const ANIMATION_SECONDS = .75;
const ANIMATION = {mini:{frames:12, stem:'spr_flare_minimap_'}, fullmap:{frames:14, stem:'spr_flare_attack_'}};

export function createBattleFlare({wasm,originalArt}) {
  const {image} = createFullmapSpriteCache([]);
  const phases = {mini:new Map(),fullmap:new Map()};
  let markers = [], mapOpen = false, art, loading = false;

  function reset(enabled) {
    markers = [];
    phases.mini.clear(); phases.fullmap.clear();
    mapOpen = false;
    if (enabled) wasm.battle_signals_enable();
  }

  function update(delta) {
    const count = wasm.battle_signal_count();
    const pointer = wasm.battle_signal_ptr()>>>0;
    const stride = wasm.battle_signal_stride?.() ?? 16;
    const bytes = new DataView(wasm.memory.buffer,pointer,count*stride);
    markers = [];
    for (let index=0; index<count; index++) {
      const offset = index*stride;
      if (bytes.getFloat32(offset+12,true)>0) markers.push({id:bytes.getUint32(offset,true),
        x:bytes.getFloat32(offset+4,true),y:bytes.getFloat32(offset+8,true),
        kind:stride===20?bytes.getUint32(offset+16,true):2});
    }
    if (markers.length && originalArt && !loading) {
      loading = true;
      loadOriginalFullmapArt(true).then(value=>{art=value;}).catch(error=>console.warn(error));
    }
    for (const surface of ['mini','fullmap']) {
      const current = phases[surface];
      for (const id of current.keys()) if (!markers.some(marker=>marker.id===id)) current.delete(id);
    }
    const surface = mapOpen?'fullmap':'mini';
    for (const {id} of markers) {
      const current = phases[surface];
      const animation = current.get(id);
      if (animation) animation.advance(delta);
      else current.set(id,createFlareAnimation(ANIMATION[surface].frames,ANIMATION_SECONDS));
    }
  }

  function setMapOpen(open) {
    if (open && !mapOpen) phases.fullmap.clear();
    mapOpen = open;
  }

  function draw(ctx,project,size,surface='mini') {
    const {frames} = ANIMATION[surface];
    for (const {id,x,y,kind} of markers) {
      const stem = surface==='fullmap'&&kind===3?'spr_flare_capture_':ANIMATION[surface].stem;
      const position = project(x,y);
      if (!position) continue;
      const [sx,sy] = position;
      const frame = phases[surface].get(id)?.frame() ?? 0;
      const icon = image(art?.sprite(stem+frame));
      if (icon) {
        ctx.drawImage(icon,sx-size/2,sy-size/2,size,size);
        continue;
      }
      ctx.save();
      ctx.strokeStyle='#f04738';
      ctx.lineWidth=Math.max(1,size*.04);
      ctx.beginPath(); ctx.arc(sx,sy,size*(.14+.26*frame/frames),0,Math.PI*2); ctx.stroke();
      ctx.restore();
    }
  }

  function cycle() {
    const pointer = wasm.battle_signal_cycle()>>>0;
    return pointer ? Array.from(new Float32Array(wasm.memory.buffer,pointer,2)) : null;
  }

  return {reset,update,setMapOpen,draw,cycle};
}
