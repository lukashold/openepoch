"""Read-only source proof for native water appearance; no runtime asset writes."""
from pathlib import Path
import csv
import hashlib
import io
import json
import re
import struct
import zipfile
from PIL import Image, ImageStat
from extract_ee2_textures import decode_texcache

APP = Path(__file__).resolve().parents[1]
ANALYSIS = APP.parent / 'EE2-Research/analysis'
INSTALL = Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
ENGINE = INSTALL / 'Unofficial Patch Files/EnabledUP15Units/EE2.exe'
pe_bytes = ENGINE.read_bytes()
header = struct.unpack_from('<I', pe_bytes, 60)[0]
image_base = struct.unpack_from('<I', pe_bytes, header + 52)[0]
section_count = struct.unpack_from('<H', pe_bytes, header + 6)[0]
optional_size = struct.unpack_from('<H', pe_bytes, header + 20)[0]
sections = [struct.unpack_from('<8sIIII', pe_bytes, header + 24 + optional_size + 40 * i) for i in range(section_count)]


def va(offset):
    return next(image_base + rva + offset - start for _, _, rva, length, start in sections if start <= offset < start + length)


def at(address, size=4):
    return next(pe_bytes[start + address - image_base - rva:start + address - image_base - rva + size]
                for _, _, rva, length, start in sections if image_base + rva <= address and address + size <= image_base + rva + length)


def sha(value):
    return hashlib.sha256(value).hexdigest()


def file_hash(path):
    return sha(path.read_bytes())


checks = []


def require(value, name):
    if not value:
        raise AssertionError(name)
    checks.append(name)


exports = ['native-water-appearance-colors.c', 'native-water-appearance-bindings.c', 'native-water-appearance-profile.c']
texts = {name: (ANALYSIS / name).read_text(encoding='utf-8') for name in exports}
old = (ANALYSIS / 'visual-terrain-water-source.c').read_text(encoding='utf-8')
require(sha(pe_bytes) == 'd41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f', 'Exact analyzed engine SHA')
constant_expectations = {0x00b41938: 2.0, 0x00b552e4: -255.0, 0x00b416ac: 5.0,
                         0x00b47158: struct.unpack('<f', bytes.fromhex('3333b33e'))[0],
                         0x00b5302c: struct.unpack('<f', bytes.fromhex('8fc2f53c'))[0], 0x00b41928: .25,
                         0x00b552e0: struct.unpack('<f', struct.pack('<f', 1 / 13))[0],
                         0x00b552e8: struct.unpack('<f', struct.pack('<f', 1 / 33))[0],
                         0x00b552ec: struct.unpack('<f', struct.pack('<f', 1 / 29))[0]}
constants = {}
for address, expected in constant_expectations.items():
    value = struct.unpack('<f', at(address))[0]
    require(value == expected, f'PE float{address:08X}={value}')
    constants[f'{address:08X}'] = {'bytes': at(address).hex(), 'f32': value}

# Decode real PUSH c-string; MOV ECX,std::string-global; CALL constructor/assignment.
# These direct PE operands independently close the previously unbound filename table.
assignments = []
for name in [f'Skin_{i}.tga' for i in range(1, 7)] + ['Water_LOD_LOW', 'Water_LOD_MEDIUM', 'Water_LOD_HIGH']:
    address = va(pe_bytes.index(name.encode() + b'\0'))
    pattern = re.escape(b'\x68' + struct.pack('<I', address)) + rb'\xb9....\xe8....'
    for match in re.finditer(pattern, pe_bytes, re.S):
        offset = match.start()
        call = va(offset + 15) + struct.unpack_from('<i', pe_bytes, offset + 11)[0]
        assignments.append({'file': name, 'instruction': f'{va(offset):08X}',
                            'global': f'{struct.unpack_from("<I", pe_bytes, offset + 6)[0]:08X}',
                            'method': f'{call:08X}', 'bytes': pe_bytes[offset:offset + 15].hex()})
for name, address in [('Water_LOD_LOW', '00D02348'), ('Water_LOD_MEDIUM', '00D02350'), ('Water_LOD_HIGH', '00D02358'),
                      ('Skin_1.tga', '00D0230C'), ('Skin_2.tga', '00D022F4'), ('Skin_4.tga', '00D022DC')]:
    require(any(row['file'] == name and row['global'] == address and row['method'] == '00A18440' for row in assignments), f'Static native table{name}->{address}')
for name, address in [('Skin_5.tga', '00D02304'), ('Skin_5.tga', '00D022F4'), ('Skin_6.tga', '00D022DC')]:
    require(any(row['file'] == name and row['global'] == address and row['method'] == '00A184A0' for row in assignments), f'Native compatibility replacement{name}->{address}')

shader_names = ['shaders/Water_LOD_HIGH.NSF', 'shaders/vsh_psh/Water_LOD_HIGH.vsh',
                'shaders/vsh_psh/Water_LOD_HIGH.psh', 'shaders/WaterReflection_LOD_HIGH.NSF',
                'shaders/vsh_psh/WaterReflection_LOD_HIGH.psh']
shaders = {name: (INSTALL / name).read_text() for name in shader_names}
ps = [re.sub(r'\s+', ' ', line.split('//')[0].strip()) for line in shaders[shader_names[2]].splitlines()]
ps = [line for line in ps if line]
require(ps == ['ps.1.4', 'texld r2, t0', 'texld r4, t0', 'texld r3, t1', 'texld r5, t1',
               'mul r2, r2, r3', 'mul r3, r4, r5', 'lrp r2, v0.g, r2, r3',
               'mul r0.rgb, r2, v0.r', '+mul r0.a, r2, v0.a', 'add r0.gb, r0, v1'], 'Exact active native high-water PS instruction sequence; commented alternative excluded')
nsf = shaders[shader_names[0]]
require('Lighting            = false' in nsf and 'ZWriteEnable        = true' in nsf, 'Base water disables fixed-function lighting and writes depth')
require('ZWriteEnable        = false' in shaders[shader_names[3]], 'Reflection overlay keeps depth writes off')
require('v5.zzzz' in shaders[shader_names[1]] and 'oD0, v5' in shaders[shader_names[1]], 'Native vertex blue becomes secondary color, full color becomes primary')
require('FUN_0088cc80' in texts[exports[0]] and '_DAT_00b552e0' in texts[exports[0]], '13-tap full-color filter is exported')
require('_DAT_00b552e8' in texts[exports[0]] and '_DAT_00b552ec' in texts[exports[0]], '33- and29-weight filter divisors are exported')
require('0088cdf8 FMUL float ptr [0x00b552e4]' in texts[exports[1]], 'Alpha index uses native negative255 factor before signed pointer subtraction')
require('param_1[10] + 0xc + iVar5) = param_7' in texts[exports[2]], 'Reflection mesh factory initializes white RGB and supplied alpha')
require('0088dc80 PUSH 0x3ea3d70a' in old and '0088dd1a PUSH 0x3ea3d70a' in old, 'Both reflection factory mode callers supply native0.32 alpha operand')

assets = []
with zipfile.ZipFile(INSTALL / 'zips/textures.zip') as textures:
    for i in range(1, 7):
        name = f'cache/texcache/Skin_{i}.texcache'
        raw = textures.read(name);image, metadata = decode_texcache(raw)
        require(image.size == (512, 512), f'Native Skin_{i} stays512-square')
        assets.append({'member': name, 'sha256': sha(raw), **metadata,
                       'meanRGBA': ImageStat.Stat(image).mean, 'extremaRGBA': image.getextrema()})
    palette_raw = textures.read('Graphics/Fx/NoConvert/WaterLayer1Palette.tga')
    palette = Image.open(io.BytesIO(palette_raw)).convert('RGBA')
    lookup_raw = textures.read('Graphics/Fx/NoConvert/WaterColor_OceanRiverCoast.tga')
    lookup = Image.open(io.BytesIO(lookup_raw)).convert('RGBA')
require(palette.size == (256, 1), 'Exact256-entry native palette')
depth_examples = []
for depth in [0, .1, .25, .5, 1, 1.5, 2, 3]:
    index = min(255, max(0, int(255 * depth / 2)))
    depth_examples.append({'depth': depth, 'paletteIndex': index, 'alphaByteBefore13TapAverage': palette.getpixel((index, 0))[3]})
with zipfile.ZipFile(INSTALL / 'zips/db.zip') as database, zipfile.ZipFile(INSTALL / 'zips/hdrs.zip') as headers:
    database_names = ['db/Terrain/dbclimate.ddf', 'db/Terrain/dbclimate_atmosphere.csv']
    data = {name: database.read(name) for name in database_names}
    terrain_header = headers.read('DbHdrs/DbTerrainDefs.h')
    climate_header = headers.read('DbHdrs/DbClimateDefs.h')
    enums = re.search(r'ENUM eTerrainMoveType\s*\{(.*?)\}', terrain_header.decode('cp1252'), re.S).group(1)
    enum_names = re.findall(r'\b(kTerrainMoveType_\w+)\s*,', enums)
    require(enum_names[5:8] == ['kTerrainMoveType_Coast', 'kTerrainMoveType_River', 'kTerrainMoveType_Ocean'], 'Native terrain movement types5coast/6river/7ocean from HDR')
    rows = list(csv.reader(io.StringIO(data[database_names[1]].decode('cp1252'))))
    normal_atmosphere = [row for row in rows if row and row[0].startswith('Atm_Temperate_') and row[0].endswith('_normal')]

static = ANALYSIS / 'native-water-static-profile.c'
if static.exists():
    exports.append(static.name)
report = {
    'format': 'ee2-native-water-appearance-evidence-v1', 'date': '2026-11-04',
    'status': 'source-proof-complete-for-listed-slices-no-runtime-change',
    'engineSha256': sha(pe_bytes), 'checks': checks,
    'scope': 'Read-only installed PE operands, targeted Ghidra exports, original NSF/VSH/PSH and BASE texture/DB/HDR inspection. No GUI, native executable, browser, simulation, runtime modules or portable assets changed.',
    'baseShader': {
        'activeInstructionSha256': sha('\n'.join(ps).encode()),
        'formula': 'A=sampleStage2(uv0)*sampleStage3(uv1); B=sampleStage4(uv0)*sampleStage5(uv1); M=lerp(B,A,vertex.g); rgb=M.rgb*vertex.r; alpha=M.a*vertex.a; rgb.gb+=vertex.b.',
        'stageRoles': {'2': 'NTM_Dark', '3': 'NTM_Detail', '4': 'NTM_Gloss', '5': 'NTM_Gloss'},
        'stageCoordinates': 'uv0=baseUV+(c13.x*.0155,c13.y*.02); uv1=baseUV+(c13.x*-.015,c13.y*.01). Full native CM_Defined time-vector population remains separate; reflection VS explicitly broadcasts c13.x.',
        'states': 'Lighting=false; depth testLessEqual; depth writes=true; SrcAlpha/InvSrcAlpha.',
        'qualification': 'No Fresnel/Phong expression occurs in this active PS. This does not prove that no CPU-side/native alternate path ever changes reflection alpha.'
    },
    'nativeTextureProfile': {
        'tableStrideBytes': 8, 'qualityOrder': ['Water_LOD_LOW', 'Water_LOD_MEDIUM', 'Water_LOD_HIGH'],
        'high': {'darkSlot1': 'Skin_1.tga', 'detailSlot2': 'Skin_2.tga', 'glossSlot3': 'Skin_4.tga'},
        'fallbackReplacements': {'mediumDark': 'Skin_5.tga', 'highDetail': 'Skin_5.tga', 'highGloss': 'Skin_6.tga'},
        'proof': 'Shader/material global arrays in008895D0 feed0088B2D0 four material slots; direct original PE PUSH/MOV ECX/CALL constructor operands prove string assignments independently of labels.',
        'assignments': assignments,
        'currentMismatch': 'The current adapter imports waterLayer1/2 and mixes them. They are different256-square images; the actual proven high-water inputs are the512-square Skin1/2/4 profiles above.'
    },
    'vertexChannels': {
        'redIntensity': '0088CC80 selects Ocean/River/Coast RGB lookup triplets; nonnegative ground overrides to Coast.0088C8C0 averages13 points: center, eight radius1 neighbors, four axis radius2 neighbors.0088B2D0 subsequently overwrites G/B, so R carries averaged native intensity.',
        'alpha': '0088CC80 uses palette alpha indexed by ground depth relative to water height over2 units. Above water reads entry0; below2 uses last entry. Within range, signed trunc(-255*depth/2) and pointer subtraction address the nonnegative palette index.0088C8C0 averages alpha over the same13 points.',
        'green': '0088CE40 maps native movement type7Ocean->1,5Coast->.5,else->0.0088CED0 uniformly averages center and eight axial/diagonal points at each radius1..4:33 samples.',
        'blue': '0088D6D0 returns.5 for nonnegative ground,.35 for submerged Coast,0 otherwise.0088D780 averages center weight5, eight radius1 neighbors weight2, eight radius2 neighbors weight1, divided by29. The active PS adds this channel to G/B; the old VS comment saying subtract is superseded by its active ADD instruction.',
        'lookupPixelsRGBA': list(lookup.getdata()), 'paletteSourceSha256': sha(palette_raw), 'lookupSourceSha256': sha(lookup_raw),
        'alphaExamples': depth_examples,
        'mapAdapterCaution': 'Our map has six own terrain classes, not native terrain33/movement8 classes. A future rendering-only mapping must remain explicit; shallow ocean/coast versus river cannot be inferred solely from a generic minimap color.'
    },
    'reflectionOverlay': {
        'formula': 'sample mirrored scene at source averaged normal offset, multiply by vertexRGBA; alpha blend over base water, no depth write.',
        'factoryEvidence': '00AB2B00 initializes every reflection-mesh vertex RGB to1 and alpha to param7;0088DBE0 supplies float0.32 in both material mode branches.',
        'qualification': 'Initial alpha0.32 is proven. Complete later CPU update/quality/overlay-order behavior is not exhausted here; do not claim a universal Fresnel absence.'
    },
    'atmosphere': {
        'facts': 'Climate DDF binds time-of-day sun vectors, skybox and general atmospheric Sun/Ambient/Specular/Fog records. Those records declare no independent WaterColor/Fresnel field. Base high-water shader has Lighting=false, so adding a general atmosphere multiplier to its pixel formula is unsupported by this evidence. Reflected scene appearance can still depend on its lighting/sky.',
        'temperateNormalRows': normal_atmosphere,
        'sources': [{'file': name, 'sha256': sha(raw)} for name, raw in data.items()] + [{'file': 'DbHdrs/DbTerrainDefs.h', 'sha256': sha(terrain_header)}, {'file': 'DbHdrs/DbClimateDefs.h', 'sha256': sha(climate_header)}],
        'unresolved': 'Exact selected time of day/weather, source atmosphere parser normalization, skybox draw and interaction with other water/material quality paths.'
    },
    'constants': constants, 'sourceAssets': assets,
    'shaderFiles': [{'file': name, 'sha256': file_hash(INSTALL / name)} for name in shader_names],
    'exports': [{'file': name, 'sha256': file_hash(ANALYSIS / name)} for name in exports],
    'implementationReadiness': {
        'closed': ['Actual native high-water texture families instead of filename guesses', 'Native high PS multiply/lerp/channel formula', 'Depth-to-palette alpha and13/33/29 vertex filters', 'Native reflection initial whiteRGB/alpha0.32', 'Native base-vs-reflection depth-write states'],
        'stillOpen': ['Default texture UV scales in00D0232C table and native grid density/plane relation to our generated map', 'Complete mode-setting lifecycle and quality/UI selection', 'Any later reflection vertex-alpha updates and complete pass ordering', 'Native skybox/atmospheric rendering and exact original-map classes'],
        'gate': 'Research only. Runtime color/alpha/texture changes wait for Root release-freeze handoff; do not repaint/recolor textures or choose constants by visual taste.'
    }
}
(ANALYSIS / 'native-water-appearance-evidence.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'passed': True, 'checks': len(checks), 'report': str(ANALYSIS / 'native-water-appearance-evidence.json'), 'runtimeChanged': False}))
