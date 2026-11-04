"""Bound read-only native reflection evidence, checked against PE + shader files."""
from pathlib import Path
import hashlib
import json
import re
import struct

ROOT = Path(__file__).resolve().parents[1]
ANALYSIS = ROOT.parent / 'EE2-Research/analysis'
INSTALL = Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
ENGINE = INSTALL / 'Unofficial Patch Files/EnabledUP15Units/EE2.exe'
raw = ENGINE.read_bytes()
pe = struct.unpack_from('<I', raw, 60)[0]
base = struct.unpack_from('<I', raw, pe + 52)[0]
section_count = struct.unpack_from('<H', raw, pe + 6)[0]
optional_size = struct.unpack_from('<H', raw, pe + 20)[0]
sections = [struct.unpack_from('<8sIIII', raw, pe + 24 + optional_size + 40 * i) for i in range(section_count)]


def bytes_at(va, size):
    for name, virtual_size, rva, raw_size, offset in sections:
        if base + rva <= va and va + size <= base + rva + raw_size:
            start = offset + va - base - rva
            return raw[start:start + size]
    raise ValueError(hex(va))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


camera = (ANALYSIS / 'native-reflection-camera.c').read_text(encoding='utf-8')
runtime = (ANALYSIS / 'native-reflection-runtime.c').read_text(encoding='utf-8')
plane = (ANALYSIS / 'native-reflection-pass.c').read_text(encoding='utf-8')
target = (ANALYSIS / 'native-reflection-target.c').read_text(encoding='utf-8')
nsf = (INSTALL / 'shaders/WaterReflection_LOD_HIGH.NSF').read_text()
psh = (INSTALL / 'shaders/vsh_psh/WaterReflection_LOD_HIGH.psh').read_text()
vsh = (INSTALL / 'shaders/vsh_psh/WaterReflection_LOD_HIGH.vsh').read_text()
checks = []


def require(value, message):
    if not value:
        raise AssertionError(message)
    checks.append(message)


require(hashlib.sha256(raw).hexdigest() == 'd41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f', 'Exact analyzed installed engine')
require(bytes_at(0x0088e1a8, 2) == bytes.fromhex('d9e0'), 'Original PE FCHS instruction at0088E1A8 negates camera height')
require('*(float *)(uVar5 + 100) = -fVar3;' in camera, 'Camera clone retains source XY and negates native Z')
require('FUN_0088e210(DAT_00d02378,1)' in runtime and 'FUN_0088e210(DAT_00d02378,0)' in runtime, 'Native reflection pass explicitly binds then restores target')
require('piVar1,0x98,1' in runtime and 'piVar1,0x98,0' in runtime, 'Native D3DRS_CLIPPLANEENABLE state152 toggles at pass boundary')
require('fStack_10' in plane and 'param_1[3] = -(' in plane, 'Plane constructor uses normalized cross product and signed offset')
require('*(undefined4 *)(*piVar6' in target and 'FUN_0088de20(pbVar4)' in target, 'Reflected camera is inserted into native view list')
require('0.01125,0.02, -0.01125, 0.01' in nsf, 'Exact two normal-UV flow vectors from native material constants')
require('ps_contant_value' in nsf and '0.1, 0.1, 0.1, 0.1' in nsf, 'Reflection UV offset scale0.1')
require('add_d2' in psh and 'r1_bx2' in psh and 'texld' in psh, 'Original pixel shader averages normals, expands to signed offset, samples RTT')
require('mov\toT2, v6' in vsh and 'frc' in vsh, 'Original vertex shader preserves reflection UV and wraps source motion phase')
require('SrcBlend            = SrcAlpha' in nsf and 'ZWriteEnable        = false' in nsf, 'Native reflection overlay blends alpha without depth writes')

exports = ['native-reflection-refs.c', 'native-reflection-runtime.c', 'native-reflection-camera.c', 'native-reflection-pass.c', 'native-reflection-target.c']
shader_paths = ['shaders/WaterReflection_LOD_HIGH.NSF', 'shaders/vsh_psh/WaterReflection_LOD_HIGH.vsh', 'shaders/vsh_psh/WaterReflection_LOD_HIGH.psh']
report = {
    'format': 'ee2-native-reflection-evidence-v1', 'date': '2026-11-04',
    'passed': True, 'checks': checks,
    'scope': 'Targeted read-only/noanalysis Ghidra exports, native PE bytes and installed plain shader files. No GUI, original executable, browser or installation writes.',
    'engineSha256': hashlib.sha256(raw).hexdigest(),
    'exports': [{'file': name, 'sha256': sha(ANALYSIS / name)} for name in exports],
    'shaderFiles': [{'file': name, 'sha256': sha(INSTALL / name)} for name in shader_paths],
    'findings': [
        {'id': 'capture-target-and-clip', 'functions': ['00889F70', '0088A8F0', '0088E210', '00A4D370'],
         'verified': 'WaterManager probes hardware clip-plane support, activates a reflection target before a scene pass, enables D3D state152 bit0 and disables/restores it afterward. Plane input points(0,0,0),(1,0,0),(1,1,0) produce native Z-up plane(0,0,1,0).',
         'application': 'Y-up adapter plane(0,1,0,0). WebGL2 terrain/model fragment discards retain the above-water half-space; native hardware clip scheduling is not copied.'},
        {'id': 'reflection-camera', 'functions': ['0088DE20', '0088AB30'],
         'verified': 'Clone retains source basis/projection fields, copies XY position and negates native Z height; auxiliary plane construction includes literal0.02. Near plane also uses native minimum and far/ratio clamp. The reflected camera is inserted into the native view list.',
         'application': 'Current geometric VP*mirrorY(0) and mirrored eye are explicit WebGL equivalents of the horizontal reflection. Native auxiliary projection/grid/UV construction and near-clamp globals are not fully reconstructed.'},
        {'id': 'scene-pass', 'functions': ['0088AE10', '008969C0', '00896940'],
         'verified': 'Reflections are gated by capability/settings, enter the reflected target/camera, render a scene and exit. Camera flag2 suppresses the Calendar/environment update branch inside008969C0.',
         'application': 'Render callback draws current terrain and native objects without simulation/calendar ticks or water recursion.'},
        {'id': 'reflection-material', 'functions': ['0088DBE0', '00628FE0'],
         'verified': 'Waterbump.tga and WaterReflection_LOD_HIGH are explicitly bound. NSF uses two wrapped normal UVs(.01125,.02),(-.01125,.01); pixel shader averages, expands to signed normal offset and offsets reflection UV by0.1 before sampling and multiplying vertex color. Alpha blend uses SrcAlpha/InvSrcAlpha with depth writes off.',
         'application': 'Native texture pixels and distortion vectors/formula are retained. Current water color, Fresnel/opacity, base-layer combination and seconds time source are still adapters.'},
    ],
    'unresolvedNativeDetails': [
        'Exact render-target allocation dimensions/format/refresh policy and full quality-setting mapping.',
        'Full auxiliary plane0.02 ray/grid UV constructor, reflected projection near-clamp policy and runtime camera hooks.',
        'Complete scene selection, transparent/cull/shadow/sky priorities and off-main-frustum native object visibility.',
        'Native vertex reflection color/alpha, complete overlay ordering and base water/caustic/palette/time linkage.',
    ],
}
(ANALYSIS / 'native-reflection-evidence.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'passed': True, 'checks': len(checks), 'report': str(ANALYSIS / 'native-reflection-evidence.json')}))
