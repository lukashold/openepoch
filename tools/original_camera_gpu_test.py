"""Check production camera matrices/picking rays on native GLES, without a UI."""
from pathlib import Path
import ctypes as C
import hashlib
import json
from headless_gles import HeadlessGLES, I, U, F, P

ROOT=Path(__file__).resolve().parents[1]
ANALYSIS=ROOT.parent/'EE2-Research/analysis'
fixture_path=ANALYSIS/'native-camera-heading-fixtures.json'
fixture=json.loads(fixture_path.read_text())
camera_sha=hashlib.sha256((ROOT/'docs/original-camera.js').read_bytes()).hexdigest()
assert fixture['cameraSha256']==camera_sha, 'Regenerate fixtures after changing the production camera'
VERTEX='''#version 300 es
precision highp float;
layout(location=0) in vec3 aPosition;
uniform mat4 uVP;
void main(){gl_Position=uVP*vec4(aPosition,1.);gl_PointSize=5.;}
'''
FRAGMENT='''#version 300 es
precision highp float;
out vec4 outColor;
void main(){outColor=vec4(1.,.4,.1,1.);}
'''
checks=0
results=[]
with HeadlessGLES(size=256) as ctx:
    program=ctx.shader_program(VERTEX,FRAGMENT)
    ctx.bind('glUseProgram',None,U)(program)
    vao=U();ctx.bind('glGenVertexArrays',None,I,C.POINTER(U))(1,C.byref(vao))
    ctx.bind('glBindVertexArray',None,U)(vao)
    buffer=U();ctx.bind('glGenBuffers',None,I,C.POINTER(U))(1,C.byref(buffer))
    ctx.bind('glBindBuffer',None,U,U)(0x8892,buffer)
    upload=ctx.bind('glBufferData',None,U,C.c_ssize_t,P,U)
    ctx.bind('glEnableVertexAttribArray',None,U)(0)
    ctx.bind('glVertexAttribPointer',None,U,I,U,C.c_ubyte,I,P)(0,3,0x1406,0,12,None)
    location=ctx.bind('glGetUniformLocation',I,U,C.c_char_p)(program,b'uVP')
    matrix=ctx.bind('glUniformMatrix4fv',None,I,I,C.c_ubyte,C.POINTER(F))
    clear_color=ctx.bind('glClearColor',None,F,F,F,F);clear=ctx.bind('glClear',None,U)
    draw=ctx.bind('glDrawArrays',None,U,I,I)
    ctx.bind('glViewport',None,I,I,I,I)(0,0,256,256)
    ctx.bind('glDisable',None,U)(0x0b71)
    ctx.bind('glDisable',None,U)(0x0be2)
    for case in fixture['cases']:
        assert case['width']==case['height']==256
        matrix(location,1,0,(F*16)(*case['vp']))
        probes=[]
        for sample in case['samples']:
            clear_color(0,0,0,1);clear(0x4000|0x0100)
            point=(F*3)(*sample['world']);upload(0x8892,C.sizeof(point),point,0x88e0)
            draw(0,0,1)
            pixels=ctx.rgba()
            lit=[(i//4%256,i//4//256) for i in range(0,len(pixels),4) if pixels[i]>200 and pixels[i+1]>60]
            assert bool(lit)==sample['visible'], (case['heading'],case['fraction'],sample,len(lit))
            checks+=1
            measured=None
            if lit:
                measured=[sum(x for x,y in lit)/len(lit),255-sum(y for x,y in lit)/len(lit)]
                assert abs(measured[0]-sample['pixel'][0])<=1, (sample,measured)
                assert abs(measured[1]-sample['pixel'][1])<=1, (sample,measured)
                checks+=2
            probes.append({'pixel':sample['pixel'],'forwardDepth':sample['depth'],'expectedVisible':sample['visible'],
                           'paintedPixels':len(lit),'measuredTopLeftPixel':measured})
        ctx.check('heading camera projection and clip probes')
        results.append({'headingRadians':case['heading'],'fraction':case['fraction'],'probes':probes})
    info=ctx.info
report={'format':'native-camera-heading-gles-tests-v1','passed':True,'checks':checks,'cases':len(results),
        'probes':sum(len(c['probes']) for c in results),'cameraSha256':camera_sha,
        'fixtureSha256':hashlib.sha256(fixture_path.read_bytes()).hexdigest(),'context':info,
        'scope':'Native GLES point draw/readback consumes production camera matrices and points from public picking rays. Tests 0/45/90/180 degrees, four zooms, expected screen pixels, and near/far/behind-eye clipping. No original runtime or UI launched.',
        'results':results}
(ANALYSIS/'native-camera-heading-gles-tests.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps({k:v for k,v in report.items() if k!='results'},indent=2))
