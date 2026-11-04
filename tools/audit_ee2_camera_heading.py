"""Verify the installed native camera-heading evidence; files/read-only only."""
from pathlib import Path
import argparse
import datetime
import hashlib
import json
import struct

ROOT=Path(__file__).resolve().parents[1]
ANALYSIS=ROOT.parent/'EE2-Research/analysis'
parser=argparse.ArgumentParser()
parser.add_argument('--install',type=Path,default=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II'))
args=parser.parse_args()
engine=args.install/'Unofficial Patch Files/EnabledUP15Units/EE2.exe'
data=engine.read_bytes();digest=hashlib.sha256(data).hexdigest()
assert digest=='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f'
pe=struct.unpack_from('<I',data,0x3c)[0]
sections=struct.unpack_from('<H',data,pe+6)[0];optional=struct.unpack_from('<H',data,pe+20)[0]
base=struct.unpack_from('<I',data,pe+24+28)[0]
def read(va,size=4):
    for i in range(sections):
        at=pe+24+optional+i*40
        virtual_size,rva,raw_size,raw=struct.unpack_from('<IIII',data,at+8)
        if rva<=va-base<rva+max(virtual_size,raw_size):
            start=raw+va-base-rva
            return data[start:start+size]
    raise ValueError(hex(va))
constants={}
for va,expected in [(0x00b41414,6.2831854820251465),(0x00b42604,1.5707963705062866),(0x00b42f0c,0.01745329238474369),(0x00b4038c,0.0)]:
    raw=read(va);value=struct.unpack('<f',raw)[0];assert value==expected
    constants[f'{va:08x}']={'bytes':raw.hex(),'f32':value}
config=args.install/'myconfig.cfg';config_bytes=config.read_bytes()
assert 'HEADING 90' in config_bytes.decode('utf8')
names=['native-camera-heading.c','native-camera-heading-bindings.c','native-camera-curve.c','visual-native-camera-runtime.c','native-camera-frustum-final.c']
source={name:(ANALYSIS/name).read_text() for name in names}
for text in source.values():assert digest in text
for address in ['005204b0','00520530','00520600']:assert f'ADDRESS {address}' in source[names[0]]
assert 'param_2 = -param_2' in source[names[0]]
assert 'param_1[0x14] = (int)-(fVar4 * fVar2)' in source[names[0]]
assert 'param_1[0x1b] = (int)fVar2' in source[names[0]]
assert 's_heading__f_00c0f95c' in source['visual-native-camera-runtime.c']
assert '*(float *)(param_1 + 0x20) = param_2 * _DAT_00b42f0c' in source['visual-native-camera-runtime.c']
assert '*(float *)(param_1 + 0x14) - (float)fVar8 * fVar2' in source['native-camera-curve.c']
assert '*param_3 - _DAT_00c985d0' in source['native-camera-heading-bindings.c']
assert 'param_2[2] = uVar1' in source['native-camera-heading-bindings.c']
camera=ROOT/'docs/original-camera.js';camera_sha=hashlib.sha256(camera.read_bytes()).hexdigest()
tests=json.loads((ANALYSIS/'native-camera-heading-tests.json').read_text())
gpu=json.loads((ANALYSIS/'native-camera-heading-gles-tests.json').read_text())
assert tests['passed'] and gpu['passed'] and tests['cameraSha256']==gpu['cameraSha256']==camera_sha
report={
 'format':'ee2-native-camera-heading-evidence-v1','status':'passed','createdUtc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'engineSha256':digest,'config':{'file':str(config),'sha256':hashlib.sha256(config_bytes).hexdigest(),'headingDegrees':90},
 'sourceConstants':constants,
 'bindings':[
  {'function':'0051d090','fact':'HEADING config degrees multiplied by float00b42f0c, stored at config+0x20.'},
  {'function':'005207f0','fact':'Orbit uses heading at camera+0x20: source eyeXY=focusXY-horizontal*[cos(H),sin(H)]. Existing pitch/distance/terrain/fov branches retained.'},
  {'function':'005204b0','fact':'Increment yaw negates its input in orbit mode1, then wraps into[0,float2PI).'},
  {'function':'00520530','fact':'Absolute yaw setter wraps into[0,float2PI) without negation.'},
  {'function':'00520600','fact':'Builds 3x3 source rotation from pitch and(H-PI/2). Columns are native forward/up/right.'},
  {'function':'00520dd0','fact':'World-to-camera subtracts cached eye, applies inverse of the native3x3, then reorders localD/U/R intoR/U/D.'},
  {'function':'0051f9f0','fact':'Camera basis is copied to NiCamera rotation+0x38; unchanged horizontal frustum is +/-tan(FOV/2),vertical divided by aspect.'}],
 'adapter':{
  'heading':'cam.heading is radians relative to native configured90 degrees, normalized[-PI,PI); missing/nonfinite0.',
  'positive':'Camera orbit moves from south toward east; +PI/2 puts eye east of focus and looks west.',
  'coordinateMapping':'SourceXYZ -> browserX/Z/-Y. The previous north-up orientation is calibrated to exactly0 to preserve its numeric output.',
  'eyeHorizontal':'[focusX + horizontal*sin(heading), focusZ + horizontal*cos(heading)]',
  'right':'[cos(heading),0,-sin(heading)]',
  'forward':'[-cos(pitch)*sin(heading),-sin(pitch),-cos(pitch)*cos(heading)]',
  'up':'[-sin(pitch)*sin(heading),cos(pitch),-sin(pitch)*cos(heading)]',
  'groundRight':'[cos(heading),-sin(heading)] in simulationX/Y',
  'groundForward':'[-sin(heading),-cos(heading)] in simulationX/Y; screen-down is its negative.',
  'ray':'direction=forward+right*ndcX*tanX+up*ndcY*tanY. Not normalized: its parameter is forward camera depth, exactly the near/far clip convention.',
  'terrain':'Height sampling uses rotated eyeXY; native minimum clearance/pitch correction preserved.',
  'profileUnchanged':['zoom fraction','distance/pitch knee quirk','horizontal FOV','near1/far300','terrain focus-height blend']},
 'api':{'sampleOriginalCamera':'(fraction,{width,height,heading=0,profile}) includesheading/nativeHeading/right/groundRight/groundForward',
  'updateOriginalCamera':'(cam,view,width,height,heightAt,profile) reads/normalizescam.heading and fills the existing view contract plus rotated basis.',
  'originalCameraRay':'(view,px,py,width,height) returns the unnormalized rotated world direction.',
  'originalCameraRaySegment':'same args; {origin,direction,near,far,nearPoint,farPoint}',
  'normalizeOriginalHeading':'(offsetRadians)->[-PI,PI)',
  'rotateOriginalCameraHeading':'(cam,deltaRadians)->heading; positive increases offset.',
  'applyOriginalCameraNativeTurn':'(cam,nativeDeltaRadians)->heading; orbit005204b0 input sign is negated.',
  'originalCameraHeadingAxes':'(heading=0)->{heading,sin,cos,right,groundRight,groundForward}'},
 'tests':{'mathAndBaseline':tests,'nativeGles':{k:v for k,v in gpu.items() if k!='results'}},
 'sourceFiles':{name:hashlib.sha256((ANALYSIS/name).read_bytes()).hexdigest() for name in names},
 'cameraSha256':camera_sha,
 'limits':['Native CamCtrl token-to-delta, repeat timing and Shift speed multiplier have not been proved; input rate remains a frontend adapter.',
  'The relative-radian API uses JavaScript trigonometry and a calibrated north basis; full x87 instruction-level equality for arbitrary rotations is not claimed.',
  'No original game/app/UI was launched. Evidence is installed config, raw PE constants, bounded read-only/noanalysis Ghidra exports and independent math/GLES checks.']}
(ANALYSIS/'native-camera-heading-evidence.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps({'status':'passed','engineSha256':digest,'cameraSha256':camera_sha,'northCases':tests['northCases'],'mathChecks':tests['checks'],'gpuCases':gpu['cases'],'gpuChecks':gpu['checks']}))
