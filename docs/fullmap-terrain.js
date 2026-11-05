// The installed Fullmap builds a 512×512 terrain image before drawing overlays.
const SIDE = 512;
const CELLS = SIDE * SIDE;
const SHORE = Math.fround(-0.2);
const SHORE_STEP = Math.fround(0.1);
const SHORE_RGB = [90, 70, 30];

export async function loadFullmapTerrainArt(base = new URL('./local-original/fullmap-native/terrain/', import.meta.url)) {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = SIDE;
  const ctx = canvas.getContext('2d', {willReadFrequently:true});
  return Promise.all(['water','ground','mountain','cliff'].map(async name => {
    const response = await fetch(new URL(`${name}.png`, base));
    if (!response.ok) throw new Error(`Fullmap texture unavailable: ${name}`);
    const bitmap = await createImageBitmap(await response.blob());
    try {
      if (bitmap.width !== SIDE || bitmap.height !== SIDE) throw new Error(`Unexpected Fullmap texture size: ${name}`);
      ctx.drawImage(bitmap, 0, 0);
      return ctx.getImageData(0, 0, SIDE, SIDE).data;
    } finally {
      bitmap.close();
    }
  }));
}

// Integer offsets match the native flat raster search, including row crossings.
function shoreDistance(types, pixel) {
  for (let d=1; d<=9; d++) {
    if (types[pixel-d] === 0 || types[pixel+d] === 0
      || types[pixel-SIDE*d] === 0 || types[pixel+SIDE*d] === 0) return d;
  }
  return 0;
}

export function composeFullmapTerrain(art, sample) {
  const output = new Uint8ClampedArray(CELLS * 4);
  const types = new Uint8Array(CELLS);
  for (let y=0; y<SIDE; y++) for (let x=0; x<SIDE; x++) {
    const pixel = y * SIDE + x;
    const offset = pixel * 4;
    const {height, mountain, cliff} = sample(x,y);
    const type = height < 0 ? 0 : mountain ? 2 : cliff ? 3 : 1;
    types[pixel] = type;
    const tex = art[type];
    const shade = type === 0 && height > SHORE ? Math.trunc((1 + 5 * height) * 60) : 0;
    for (let c=0; c<3; c++) output[offset+c] = Math.min(255, tex[offset+c] + shade);
    output[offset+3] = 255;
  }
  for (let i=0; i<CELLS; i++) {
    if (types[i] === 0) continue;
    const distance = shoreDistance(types,i);
    if (!distance) continue;
    const land = (distance-1)*SHORE_STEP;
    for (let c=0; c<3; c++) {
      const index = i*4+c;
      output[index] = Math.trunc(output[index]*land + SHORE_RGB[c]*(1-land));
    }
  }
  return output;
}

export function sampleFullmapTiles(heights, terrain, width, height) {
  const stride = width + 1;
  const dx = Math.fround((width-1)/SIDE), dy = Math.fround((height-1)/SIDE);
  // OpenEpoch's ground plane increases southwards, corresponding to screen Y.
  return (column,row) => {
    const x = Math.fround(column*dx), y = Math.fround(row*dy);
    const ix = Math.floor(x), iy = Math.floor(y);
    const fx = x-ix, fy = y-iy;
    const index = iy*stride+ix;
    const north = heights[index] + (heights[index+1]-heights[index])*fx;
    const south = heights[index+stride] + (heights[index+stride+1]-heights[index+stride])*fx;
    return {height:Math.fround(north+(south-north)*fy), mountain:terrain[iy*width+ix] === 5, cliff:false};
  };
}
