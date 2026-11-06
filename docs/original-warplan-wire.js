const PATH = 3;
const STATUS_COUNT = 11;
const HEADER = 12 + STATUS_COUNT*4;
const ARRAYS = 8;
const ELEMENT = 12;
export const WARPLAN_MAX_BYTES = 1250;

const seat = value => Number.isInteger(value) && value>=1 && value<=2;
const color = value => value===-1 || Number.isInteger(value) && value>=1 && value<=10;
const clock = value => Number.isInteger(value) && value>=0 && value<=0x7fffffff;
const coord = (value,end) => Number.isFinite(value) && value>=0 && value<=end;

// Native 00768340 rounds its Float32 game time to whole seconds. The x87
// mantissa bias first rounds the positive value to 16 fractional bits.
export function originalWarplanTimestamp(seconds) {
  return Math.floor(Math.round((Math.fround(seconds)+.5)*65536)/65536);
}

// 00768E20 and 007692F0 with the 0076C0F0 Path subtype. The original
// GameMsg envelope lives outside this path-only plan payload.
export function encodeWarplanPath({sender,recipient,timestamp,paths}, end) {
  if (!seat(sender) || !seat(recipient) || sender===recipient || !clock(timestamp) || !Array.isArray(paths) || !paths.length) return null;
  let length=HEADER+ARRAYS;
  for (const path of paths) {
    if (path?.type!==PATH || !color(path.colorPlayer) || !Array.isArray(path.points) || !path.points.length) return null;
    length+=ELEMENT+8*path.points.length;
    if (length>WARPLAN_MAX_BYTES) return null;
    for (const point of path.points) {
      if (point?.length!==2 || !coord(point[0],end) || !coord(point[1],end)) return null;
    }
  }
  const bytes=new Uint8Array(length), data=new DataView(bytes.buffer);
  let offset=0;
  const i32=value=>{data.setInt32(offset,value,true);offset+=4;};
  const u16=value=>{data.setUint16(offset,value,true);offset+=2;};
  const f32=value=>{data.setFloat32(offset,value,true);offset+=4;};
  i32(sender); i32(timestamp); i32(1);
  for (let player=0;player<STATUS_COUNT;player++) i32(player===sender ? 2 : player===recipient ? 1 : 0);
  i32(paths.length);
  for (const path of paths) {
    i32(PATH); u16(path.points.length);
    for (const [x,y] of path.points) { f32(x); f32(y); }
    u16(0xffff); i32(path.colorPlayer);
  }
  i32(0);
  return bytes;
}

export function decodeWarplanPath(bytes,{sender,recipient,end}) {
  if (!(bytes instanceof Uint8Array) || bytes.length<HEADER+ARRAYS+ELEMENT+8 || bytes.length>WARPLAN_MAX_BYTES ||
    !seat(sender) || !seat(recipient) || sender===recipient) return null;
  const data=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  let offset=0;
  const i32=()=>{const value=data.getInt32(offset,true);offset+=4;return value;};
  const u16=()=>{const value=data.getUint16(offset,true);offset+=2;return value;};
  const f32=()=>{const value=data.getFloat32(offset,true);offset+=4;return value;};
  if (i32()!==sender) return null;
  const timestamp=i32();
  if (!clock(timestamp) || i32()!==1) return null;
  for (let player=0;player<STATUS_COUNT;player++) {
    if (i32() !== (player===sender ? 2 : player===recipient ? 1 : 0)) return null;
  }
  const count=i32(), paths=[];
  if (count<1 || count>Math.floor((bytes.length-offset-4)/(ELEMENT+8))) return null;
  for (let index=0;index<count;index++) {
    if (offset+ELEMENT+4>bytes.length || i32()!==PATH) return null;
    const length=u16(), points=[];
    if (!length || offset+8*length+6+4>bytes.length) return null;
    for (let point=0;point<length;point++) {
      const x=f32(), y=f32();
      if (!coord(x,end) || !coord(y,end)) return null;
      points.push([x,y]);
    }
    if (u16()!==0xffff) return null;
    const colorPlayer=i32();
    if (!color(colorPlayer)) return null;
    paths.push({type:PATH,colorPlayer,points});
  }
  if (offset+4!==bytes.length || i32()!==0) return null;
  return {sender,recipient,timestamp,status:1,paths};
}
