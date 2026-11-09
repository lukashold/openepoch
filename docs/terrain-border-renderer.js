import {createTerritoryBorderGeometryRenderer} from './territory-border-geometry.js?v=terrain-presentation227';
import {createTerritoryCameraMode} from './territory-camera-mode.js?v=territory-valley226';
import {loadOriginalTerritoryBorders, uploadOriginalTerritoryBorders} from './original-territory-borders.js?v=territory-borders218';
import {packTerritoryBorderDraws} from './territory-border-compositor.js?v=territory-borders218';
import {createTerritoryBorderTopology, territoryBorderDrawRecords,
  TERRITORY_BORDER_DRAW_STRIDE} from './territory-border-topology.js?v=territory-valley226';
import {freshTerritoryBorderFlush} from './territory-border-order.js?v=territory-borders218';

// Original world-territory colors in db/UI/dbcolor.csv. Portable seats use
// the existing blue/red/green/yellow assignment, independent of minimap colors.
const palette = new Float32Array(17 * 3);
for (const [index, rgb] of [[25,25,25],[0,0,200],[250,9,9],[0,200,0],[237,252,5]].entries())
  palette.set(rgb.map(value => value / 255), index * 3);

function sameField(a, b, step) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += step) if (a[i] !== b[i]) return false;
  return true;
}

function drawInBufferOrder(draws, vertexDemand) {
  const stride = TERRITORY_BORDER_DRAW_STRIDE;
  if (!vertexDemand) return draws;
  const counts = vertexDemand(draws);
  if (!counts || counts.length !== draws.length / stride) throw Error('Wrong territory decal vertex demand');
  const ordered = new Float32Array(draws.length);
  for (const [offset, {index}] of freshTerritoryBorderFlush(counts).drawOrder.entries())
    ordered.set(draws.subarray(index * stride, (index + 1) * stride), offset * stride);
  return ordered;
}

export async function loadTerrainBorderRenderer(gl, enabled, {vertexDemand = null, geometryRequested = false, ...fetchOptions} = {}) {
  const atlas = await loadOriginalTerritoryBorders({enabled, ...fetchOptions});
  if (!atlas) return null;
  const texture = uploadOriginalTerritoryBorders(gl, atlas, {
    minFilter:gl.LINEAR_MIPMAP_NEAREST, magFilter:gl.LINEAR,
  });
  const cells = gl.createTexture(), records = gl.createTexture();
  if (!cells || !records) {
    gl.deleteTexture(texture);
    if (cells) gl.deleteTexture(cells);
    if (records) gl.deleteTexture(records);
    return null;
  }
  const geometry = geometryRequested
    ? createTerritoryCameraMode(()=>createTerritoryBorderGeometryRenderer(gl,texture),palette) : null;
  let topology, last = new Uint8Array(), lastClimate, lastWidth, lastHeight, highSampling = false;
  const metrics = {records:0, topologyBuilds:0, uploads:0, order:vertexDemand?'native-fresh-buffers':'registration'};

  function upload(data) {
    const target = gl.TEXTURE_2D;
    const previous = gl.getParameter(gl.TEXTURE_BINDING_2D);
    try {
      for (const [handle, width, height, internal, format, type, pixels] of [
        [cells,data.width,data.height,gl.RG32UI,gl.RG_INTEGER,gl.UNSIGNED_INT,data.cells],
        [records,data.recordWidth,data.recordHeight,gl.RGBA32F,gl.RGBA,gl.FLOAT,data.records],
      ]) {
        gl.bindTexture(target,handle);
        gl.texParameteri(target,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
        gl.texParameteri(target,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
        gl.texImage2D(target,0,internal,width,height,0,format,type,pixels);
      }
    } finally {
      gl.bindTexture(target,previous);
    }
  }
  upload(packTerritoryBorderDraws(new Float32Array(),1,1));

  function setTerrain(heights,width,height,sourceYForward=false) {
    if (!geometry) return;
    geometry.setTerrain(heights,width,height,sourceYForward);
    lastWidth = undefined;
  }

  function prepareGeometry(cameras) {
    geometry?.prepare(cameras);
  }

  function drawGeometry(options) {
    geometry?.draw({...options,highQuality:highSampling});
  }

  function update(field, {width, height, stride, climate, authored = false, warmup = false}) {
    const slots = atlas.climateSlots[climate];
    if (!slots || !(field instanceof Uint8Array) || !Number.isInteger(width) || !Number.isInteger(height)
        || !Number.isInteger(stride) || width < 1 || height < 1 || stride < width
        || field.length < height * stride * 4) throw Error('Invalid territory border world input');
    let compact = field.subarray(0,width*height*4);
    if (stride !== width) {
      compact = new Uint8Array(width*height*4);
      for (let row=0; row<height; row++) compact.set(field.subarray(row*stride*4,(row*stride+width)*4),row*width*4);
    }
    const changedSize = width !== lastWidth || height !== lastHeight;
    if (!changedSize && climate === lastClimate && sameField(last,compact,1)) return false;
    const changedTopology = changedSize || !sameField(last,compact,4);
    if (changedTopology) {
      topology = createTerritoryBorderTopology(compact,width,height,!!geometry&&authored);
      metrics.topologyBuilds++;
    }
    const native = territoryBorderDrawRecords(topology,compact,slots);
    if (geometry) {
      geometry.update(native,changedTopology,warmup);
      metrics.order = authored?'authored-stock-block-geometry':'stock-block-geometry';
      metrics.geometry = geometry.metrics;
      metrics.records = native.length / TERRITORY_BORDER_DRAW_STRIDE;
    } else {
      const data = packTerritoryBorderDraws(drawInBufferOrder(native,vertexDemand),width,height);
      upload(data);
      metrics.order = vertexDemand?'native-fresh-buffers':'registration';
      delete metrics.geometry;
      metrics.records = data.recordCount;
    }
    metrics.uploads++;
    last = compact.slice();
    lastWidth = width;
    lastHeight = height;
    lastClimate = climate;
    return true;
  }

  function setQuality(high) {
    if (highSampling === high) return;
    highSampling = high;
    const target = gl.TEXTURE_2D_ARRAY;
    const previous = gl.getParameter(gl.TEXTURE_BINDING_2D_ARRAY);
    gl.bindTexture(target,texture);
    gl.texParameteri(target,gl.TEXTURE_MIN_FILTER,high ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR_MIPMAP_NEAREST);
    gl.bindTexture(target,previous);
  }

  function bind(uniform, [cellsUnit, recordsUnit, alphaUnit]) {
    for (const [name, unit, target, handle] of [
      ['uTerritoryBorderCells',cellsUnit,gl.TEXTURE_2D,cells],
      ['uTerritoryBorderRecords',recordsUnit,gl.TEXTURE_2D,records],
      ['uTerritoryBorderMips',alphaUnit,gl.TEXTURE_2D_ARRAY,texture],
    ]) {
      gl.activeTexture(gl.TEXTURE0+unit);
      gl.bindTexture(target,handle);
      gl.uniform1i(uniform(name),unit);
    }
    gl.uniform3fv(uniform('uTerritoryBorderTint[0]'),palette);
    gl.uniform1f(uniform('uTerritoryBorderOpacity'),1);
  }

  return {update, setQuality, bind, metrics, geometry, setTerrain, prepareGeometry, drawGeometry};
}
