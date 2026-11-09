// Native world axes. Convert the portable Y direction at the geometry boundary.
const TERRITORY_RING = Object.freeze([
  [-1,0],[-1,1],[0,1],[1,1],[1,0],[1,-1],[0,-1],[-1,-1],
].map(offset=>Object.freeze(offset)));

// Native 00724B90 treats out-of-grid cells exactly like the -1 sentinel.
// The callback supplies neighbors in TERRITORY_RING order.
function nativeTerritoryBorderMask(territory, neighbor) {
  let same=0, outside=0;
  for(let bit=0;bit<8;bit++){
    const id=neighbor(bit), flag=1<<bit;
    if(id===territory||id===-1)same|=flag;
    if(id===-1)outside|=flag;
  }
  if(same===255||outside===0)return same;
  let result=same;
  for(let bit=0;bit<8;bit++){
    if(same&(1<<bit))continue;
    const adjacent=(1<<((bit+7)&7))|(1<<((bit+1)&7));
    result&=~(adjacent&outside);
  }
  return result;
}

// Native 00718700 calls 00718670 once for each cyclic run of foreign cells.
// This returns the packed visible slots, omitting the first zero terminator.
function nativeTerritoryBorderShapes(mask) {
  if(mask===0||mask===255)return [];
  let inHome=Boolean(mask&1);
  let firstHome=inHome&&!(mask&128)?0:-1;
  let previousHome=!inHome&&(mask&128)?7:-1;
  const segments=[];
  function append(start,end){
    const span=end>start?end-start-1:end-start+7;
    segments.push({
      shape:span+(end%2===0?7:0),
      quarterTurns:(6-Math.floor((end+1)/2))%4,
      cornerExtension:span===1?(end+7)%8:8,
    });
  }
  for(let bit=0;bit<8;bit++){
    const home=Boolean(mask&(1<<bit));
    if(home&&!inHome){
      if(previousHome>=0){append(previousHome,bit);previousHome=-1;}
      else firstHome=bit;
    }else if(!home&&inHome)previousHome=bit-1;
    inHome=home;
  }
  if(!inHome)append(previousHome,firstHome);
  return segments;
}

const CORNER_PROBES = [
  [[2,1],[1,2]],[[-2,1],[-1,2]],[[-2,-1],[-1,-2]],[[2,-1],[1,-2]],
];
const HALF_PI = Math.fround(Math.PI/2);

function duplicateBorderStrip(x,y,territory,shape,turn,territoryAt) {
  if(shape!==8)return false;
  return CORNER_PROBES[turn].some(([dx,dy])=>territoryAt(x+dx,y+dy)===territory);
}

// X and Y here are original map axes, with extent and orientation taken from
// the matching one-cell terrain raster. Keep the portable world-axis mapping
// at the consumer boundary.
function nativeTerritoryDecalPlan(x,y,territory,mask,state,territoryAt) {
  return nativeTerritoryBorderShapes(mask).flatMap(({shape,quarterTurns,cornerExtension})=>{
    if(duplicateBorderStrip(x,y,territory,shape,quarterTurns,territoryAt))return [];
    const [dx,dy]=TERRITORY_RING[cornerExtension]??[0,0];
    return [{
      sprite:3*shape+state,shape,quarterTurns,center:[x+.5+dx,y+.5+dy],
      angle:Math.fround(quarterTurns*HALF_PI),textureSize:[1,1],quadSize:[1,1],mode:2,
    }];
  });
}

export const TERRITORY_BORDER_DRAW_STRIDE=8;

function assertRaster(rgba,width,height){
  if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<1||height<1||
      !(rgba instanceof Uint8Array||rgba instanceof Uint8ClampedArray)||rgba.length!==width*height*4)
    throw new RangeError('Invalid territory raster');
}

// R is immutable map geometry. Region 0 remains a valid portable region;
// R=255 and out-of-grid positions both represent the native invalid sentinel.
export function createTerritoryBorderTopology(rgba,width,height,sourceYForward=false){
  assertRaster(rgba,width,height);
  const worldRow=y=>sourceYForward?y:height-1-y;
  const at=(x,y)=>{
    if(x<0||x>=width||y<0||y>=height)return -1;
    const id=rgba[(worldRow(y)*width+x)*4];
    return id===255?-1:id;
  };
  const records=[];
  for(let y=0;y<height;y++)for(let x=0;x<width;x++){
    const id=at(x,y);
    if(id<0)continue;
    const mask=nativeTerritoryBorderMask(id,bit=>{
      const [dx,dy]=TERRITORY_RING[bit];
      return at(x+dx,y+dy);
    });
    if(mask===0||mask===255)continue;
    const origin=worldRow(y)*width+x;
    for(const decal of nativeTerritoryDecalPlan(x,y,id,mask,0,at)){
      const [centerX,centerY]=decal.center;
      records.push({origin,shape:decal.shape,centerX,centerZ:sourceYForward?centerY:height-centerY,
        cos:Math.fround(Math.cos(decal.angle)),sin:Math.fround(Math.sin(decal.angle))});
    }
  }
  return {width,height,records};
}

// G is the packed incumbent, B=2 is a fog-authorized disputed state.
// The source-cell state controls the complete translated corner piece.
export function territoryBorderDrawRecords(topology,rgba,atlasByShape){
  const {width,height,records}=topology;
  assertRaster(rgba,width,height);
  if(!Array.isArray(atlasByShape)||atlasByShape.length!==15||
      atlasByShape.some(row=>!Array.isArray(row)||row.length!==3||
        row.some(index=>!Number.isInteger(index)||index<0)))
    throw new RangeError('Invalid territory border atlas');
  const result=new Float32Array(records.length*TERRITORY_BORDER_DRAW_STRIDE);
  let at=0;
  for(const record of records){
    const base=record.origin*4,owner=rgba[base+1],state=owner===0?0:rgba[base+2]===2?2:1;
    result[at++]=record.centerX;
    result[at++]=record.centerZ;
    result[at++]=record.cos;
    result[at++]=record.sin;
    result[at++]=atlasByShape[record.shape][state];
    result[at++]=owner;
    result[at++]=state;
    result[at++]=record.origin;
  }
  return result;
}
