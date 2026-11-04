"""Summarize only current, source-bound production reflection QA."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[1]
RESEARCH = ROOT.parent / 'EE2-Research'
ANALYSIS = RESEARCH / 'analysis'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name):
    return json.loads((ANALYSIS / name).read_text(encoding='utf-8'))


world = read('native-season-world-qa.json')
pixels = read('native-reflection-world-pixel-tests.json')
contract = read('native-reflection-contract-tests.json')
evidence = read('native-reflection-evidence.json')
wasm = sha(ROOT / 'docs/openepoch-ee2.wasm')
assert world['passed'] and world['wasmSha256'] == pixels['wasmSha256'] == wasm
assert len(world['scenes']) == 5 and pixels['passed'] and contract['passed'] and evidence['passed']
for source, digest in world['sourceFiles'].items():
    assert sha(ROOT / 'docs' / source) == digest, f'Stale world proof for {source}'
for source, digest in pixels['sourceHashes']['files'].items():
    assert sha(ROOT / 'docs' / source) == digest, f'Stale reflection proof for {source}'
for source, digest in contract['sourceHashes'].items():
    assert sha(ROOT / 'docs' / source) == digest, f'Stale reflection contract for {source}'
names = ['native-reflection-evidence.json', 'native-reflection-contract-tests.json',
         'native-reflection-world-pixel-tests.json', 'native-reflection-world-coast.json',
         'native-season-world-qa.json', 'native-terrain-contract-tests.json', 'native-terrain-gpu-pixels.json']
report = {
    'format': 'openepoch-native-reflection-implementation-v1', 'date': '2026-11-04',
    'componentBuild': 'native-reflection1', 'status': 'passed', 'wasmSha256': wasm,
    'problem': 'Production water previously used an approximate sky color because no rendered scene texture was supplied. Reflected terrain/objects were missing.',
    'result': 'Production planar RGBA8/depth-stencil scene target now supplies terrain and native objects to the existing source-derived two-normal water distortion path. Above-water terrain/model clipping prevents submerged geometry from contaminating the target. Main and reflected seasonal skins use the same actual saved simulation tick.',
    'api': {
        'module': 'docs/original-reflection.js',
        'factory': 'createOriginalReflection(gl,{resolutionScale:1,maxSize:2048})',
        'capture': 'render({vp,eye,width,height,waterHeight:0,drawScene}) -> {texture,vp,eye,width,height,waterHeight}',
        'callback': 'drawScene({vp,eye,clipPlane:[0,1,0,-waterHeight],capture:true,width,height}); synchronous terrain/model-only callback, no water recursion or sampling this target.',
        'consumer': 'terrain.drawWater({...,reflectionTexture:capture.texture,reflectionVP:capture.vp})',
        'terrainHook': 'drawTerrain({...,clipPlane:null,capture:false}); main default resets the clip uniform. capture:true preserves main terrain counters.',
        'modelHook': 'draw({...,clipPlane:null,capture:false}); Materials renderer owns this hook and preserves picking/timer/main metrics during capture.',
        'lifecycle': 'Reuses same-size attachments, proportionally resizes within limits, retires still-bound previous targets, disposes resources, rejects recursive/async draws. Exceptions and incomplete allocation restore caller state.',
    },
    'stateRestoration': ['separate draw/read framebuffer bindings', 'renderbuffer', 'viewport and scissor',
                         'color/depth/stencil clear values', 'color/depth write masks and separate front/back stencil masks',
                         'depth test/function, blend factors/equations and enabled flags', 'cull/front-face modes',
                         'program, VAO and array buffer', 'all fragment texture slots both2D and2D-array plus active texture'],
    'sourceBacked': ['Native0088A8F0/0088E210 capture-target and hardware clip-plane bracketing.',
                     'Native0088DE20 cloned camera height sign inversion;0088AB30 separate reflected view.',
                     'Native horizontal clip plane Z-up(0,0,1,0), mapped to application Y-up(0,1,0,0).',
                     'Original Waterbump pixels and NSF flow(.01125,.02),(-.01125,.01), averaged signed-normal offset scale0.1.'],
    'explicitAdapters': ['WebGL RGBA8+DEPTH24_STENCIL8 framebuffer, proportional resolution/quality/cap policy and lifecycle.',
                         'VP*mirrorY geometrical transform; fragment-discard clipping rather than native fixed-function hardware clipping.',
                         'Culling disabled inside reflected pass to accommodate handedness; complete native render-state and transparent ordering are not reconstructed.',
                         'Water fallback clear/sky color, base color/layer mixture, Fresnel/specular/opacity and wall-clock animation phase remain adapter behavior.'],
    'validation': {
        'sourceChecks': len(evidence['checks']), 'stateAndMatrixChecks': contract['checks'],
        'combinedWorldChecks': world['checks'], 'combinedWorlds': len(world['scenes']),
        'combinedCases': [scene['name'] for scene in world['scenes']], 'reflectionPixelChecks': pixels['checks'],
        'nativeObjectReflectionPixels': pixels['nativeObjectContribution'],
        'submergedGeometryClipPixels': pixels['submergedClipContribution'],
        'visibleWaterReflectionPixels': pixels['waterReflectionContribution'],
        'method': 'Actual production JS helper/loaders/draw inputs with real WASM worlds and createSeasonSession; real ANGLE EGL/GLES FBO capture and pixel-control replays. Source/profile/shader/manifest hashes are included in reports.',
        'noGuiOrOriginalExecutable': True,
    },
    'remainingLimits': ['Native auxiliary plane0.02 projection/grid/UV construction and reflected-camera near-clamp globals remain incomplete.',
                        'Exact native target format/dimensions/refresh and complete quality mapping are unproven.',
                        'Current production reflection reuses main visible instances. Native inserts a separate view; very tall objects outside the main instance AABB may therefore be absent from our reflection.',
                        'No full native sky/shadow/transparency/cull/caustic/palette/overlay ordering or identical source time phase.',
                        'No controlled same-scene original visual comparison, browser interaction or physical macOS execution in this headless proof.'],
    'sourceFiles': world['sourceFiles'],
    'reports': [{'file': name, 'sha256': sha(ANALYSIS / name)} for name in names],
    'baseline': 'work/reflection-baseline retains pre-change terrain/capture/recorder/replayer files; original installation unchanged.',
}
(ANALYSIS / 'native-reflection-implementation.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'passed': True, 'report': str(ANALYSIS / 'native-reflection-implementation.json'), 'worldChecks': world['checks'], 'pixelChecks': pixels['checks'], 'stateChecks': contract['checks']}))
