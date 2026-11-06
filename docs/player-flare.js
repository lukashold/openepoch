import {loadOriginalFullmapArt} from './original-fullmap-art.js';
import {createFullmapSpriteCache} from './fullmap-sprites.js';

const PLAYER_FRAME_COUNT = 12;
const PLAYER_ANIMATION_SECONDS = 0.75;
const PLAYER_FLARE_SECONDS = 15;

// GWAnimatedSprite advances at most one frame per UI update and discards overshoot.
function createAnimation() {
  const slice = Math.fround(PLAYER_ANIMATION_SECONDS/PLAYER_FRAME_COUNT);
  let elapsed=0, frame=0;
  return {
    advance(dt) {
      const total=elapsed+Math.fround(dt);
      if (total>=slice) {
        frame=(frame+1)%PLAYER_FRAME_COUNT;
        elapsed=0;
      } else elapsed=Math.fround(total);
    },
    frame:()=>frame,
    reset(){elapsed=0; frame=0;},
  };
}

export function createPlayerFlare({mapSize,tickSeconds,originalArt,local,teams,onMode}) {
  let marker, tick = 0, armed = false, art, loading = false;
  let mapOpen = false;
  const miniAnimation=createAnimation(), mapAnimation=createAnimation();
  const {image,colorize} = createFullmapSpriteCache(teams);

  function arm() {
    if (armed) return;
    armed = true;
    onMode(true);
    if (originalArt && !loading) {
      loading = true;
      loadOriginalFullmapArt(true).then(value=>{art=value;}).catch(error=>console.warn(error));
    }
  }

  function cancel() {
    if (!armed) return false;
    armed = false;
    onMode(false);
    return true;
  }

  function release(button,point) {
    if (!armed) return false;
    if (button===2) return cancel();
    if (button!==0) return false;
    if (!Array.isArray(point) || point.length!==2 || !point.every(value=>Number.isFinite(value)&&value>=0&&value<mapSize)) return true;
    const owner = local();
    if (owner===undefined) return true;
    const [x,y] = point;
    marker = {x:Math.fround(x),y:Math.fround(y),owner,remaining:PLAYER_FLARE_SECONDS};
    miniAnimation.reset(); mapAnimation.reset();
    return cancel();
  }

  function update(now,uiDelta=0) {
    const delta=Math.fround(tickSeconds);
    for (; marker && tick<now; tick++) {
      if (marker.remaining>0) marker.remaining=Math.fround(marker.remaining-delta);
      else marker=undefined;
    }
    tick = now;
    (mapOpen?mapAnimation:miniAnimation).advance(uiDelta);
  }

  function reset(now) {
    cancel();
    tick = now;
    marker = undefined;
    miniAnimation.reset(); mapAnimation.reset();
    mapOpen = false;
  }

  function setMapOpen(open) {
    if (open && !mapOpen) mapAnimation.reset();
    mapOpen = open;
  }

  function draw(ctx,project,size,surface) {
    const viewer = local();
    if (!marker || marker.owner!==viewer || marker.remaining<=0) return;
    const position = project(marker.x,marker.y);
    if (!position) return;
    const [x,y] = position;
    const index = (surface==='fullmap'?mapAnimation:miniAnimation).frame();
    const sprite = art?.sprite(`spr_flare_minimap_${index}`);
    const icon = image(sprite);
    if (icon) {
      ctx.drawImage(colorize(sprite,icon,viewer),x-size/2,y-size/2,size,size);
    } else {
      const radius = size*(.12+.24*index/PLAYER_FRAME_COUNT);
      ctx.save();
      ctx.strokeStyle = `rgb(${teams[viewer].map(value=>Math.round(value*255)).join(',')})`;
      ctx.lineWidth = Math.max(1,size*.035);
      ctx.beginPath(); ctx.arc(x,y,radius,0,Math.PI*2); ctx.stroke();
      ctx.restore();
    }
  }

  return {arm,cancel,release,update,reset,setMapOpen,draw,isArmed:()=>armed};
}
