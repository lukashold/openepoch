"""Render production model shaders into an EGL pbuffer without any app/UI.

This checks actual pixels on the installed ANGLE GPU path. It is not an
original-game screenshot comparison or proof of physical macOS execution.
"""
import argparse
import ctypes as C
import hashlib
import json
import math
import pathlib
from headless_gles import HeadlessGLES, I, U, F, P


def run(payload, report):
    shader=payload['programs'][0]
    checks=[]
    with HeadlessGLES(64) as ctx:
        gl=ctx.gl
        for name,restype,args in [
            ('glUseProgram',None,[U]),('glGenVertexArrays',None,[I,C.POINTER(U)]),('glBindVertexArray',None,[U]),
            ('glGenBuffers',None,[I,C.POINTER(U)]),('glBindBuffer',None,[U,U]),('glBufferData',None,[U,C.c_ssize_t,P,U]),
            ('glEnableVertexAttribArray',None,[U]),('glVertexAttribPointer',None,[U,I,U,U,I,P]),('glVertexAttribDivisor',None,[U,U]),
            ('glGenTextures',None,[I,C.POINTER(U)]),('glActiveTexture',None,[U]),('glBindTexture',None,[U,U]),
            ('glTexImage2D',None,[U,I,I,I,I,I,U,U,P]),('glTexImage3D',None,[U,I,I,I,I,I,I,U,U,P]),('glTexParameteri',None,[U,U,I]),
            ('glGetUniformLocation',I,[U,C.c_char_p]),('glUniform1i',None,[I,I]),('glUniform1f',None,[I,F]),
            ('glUniform3fv',None,[I,I,C.POINTER(F)]),('glUniform4fv',None,[I,I,C.POINTER(F)]),('glUniformMatrix4fv',None,[I,I,U,C.POINTER(F)]),
            ('glViewport',None,[I,I,I,I]),('glClearColor',None,[F,F,F,F]),('glClear',None,[U]),
            ('glDrawArraysInstanced',None,[U,I,I,I]),('glEnable',None,[U]),('glDisable',None,[U]),('glBlendFunc',None,[U,U]),
        ]: ctx.bind(name,restype,*args)
        program=ctx.shader_program(shader['vertex'],shader['fragment']);gl.glUseProgram(program)
        def loc(name):return gl.glGetUniformLocation(program,name.encode())
        def scalar(name,value,integer=False):(gl.glUniform1i if integer else gl.glUniform1f)(loc(name),value)
        def vector(name,values):
            data=(F*len(values))(*values)
            (gl.glUniform3fv if len(values)==3 else gl.glUniform4fv)(loc(name),1,data)
        def matrix(values):gl.glUniformMatrix4fv(loc('uVP'),1,0,(F*16)(*values))
        def create_buffer(values):
            buffer=U();gl.glGenBuffers(1,C.byref(buffer));gl.glBindBuffer(0x8892,buffer)
            data=(F*len(values))(*values);gl.glBufferData(0x8892,C.sizeof(data),data,0x88e4)
            return buffer
        vao=U();gl.glGenVertexArrays(1,C.byref(vao));gl.glBindVertexArray(vao)
        vertex_buffer=create_buffer([0.]*36)
        for index,count,offset in [(0,3,0),(1,3,12),(2,2,24),(6,4,32)]:
            gl.glEnableVertexAttribArray(index);gl.glVertexAttribPointer(index,count,0x1406,0,48,P(offset))
        instance_buffer=create_buffer([0,0,0,0,0,0,0,32,1,0,0,0])
        for index in (3,4,5):
            gl.glEnableVertexAttribArray(index);gl.glVertexAttribPointer(index,4,0x1406,0,48,P((index-3)*16));gl.glVertexAttribDivisor(index,1)
        texture_ids=[]
        def texture(unit,width,height,pixels,*,float32=False,array=False):
            texture_id=U();gl.glGenTextures(1,C.byref(texture_id));texture_ids.append(texture_id.value)
            target=0x8c1a if array else 0x0de1
            gl.glActiveTexture(0x84c0+unit);gl.glBindTexture(target,texture_id)
            data=((F if float32 else C.c_ubyte)*len(pixels))(*pixels)
            if array:gl.glTexImage3D(target,0,0x8814,width,height,1,0,0x1908,0x1406,data)
            else:gl.glTexImage2D(target,0,0x822e if float32 else 0x8058,width,height,0,0x1903 if float32 else 0x1908,0x1406 if float32 else 0x1401,data)
            for parameter,value in [(0x2801,0x2600),(0x2800,0x2600),(0x2802,0x812f),(0x2803,0x812f)]:gl.glTexParameteri(target,parameter,value)
            ctx.check('fixture texture upload')
            return texture_id
        diffuse=texture(0,1,1,[128,192,64,255]);texture(2,1,1,[255,255,255,255]);texture(5,1,1,[0.,0.,0.,0.],float32=True,array=True)
        terrain=texture(6,3,3,[0.]*9,float32=True)
        for name,value in {'uDiffuse':0,'uFog':2,'uFrames':5,'uTerrain':6,'uFrameWidth':1,'uFrameCount':1,'uTerrainSize':3}.items():scalar(name,value,True)
        gl.glViewport(0,0,64,64);gl.glDisable(0xb71);gl.glDisable(0xb44)
        def vertices(positions=None,normal=(0,1,0),color=(1,1,1,1)):
            positions=positions or [(-.9,-.9,0),(.9,-.9,0),(0,.9,0)]
            values=[]
            for position in positions:values.extend([*position,*normal,0,0,*color])
            gl.glBindBuffer(0x8892,vertex_buffer);data=(F*len(values))(*values);gl.glBufferData(0x8892,C.sizeof(data),data,0x88e4)
        def defaults():
            matrix([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);vertices()
            for name,value in {'uShadow':0,'uAnimated':0,'uTerrainEnabled':0,'uSpecularEnabled':0,'uVertexMode':0,'uLightingMode':1,'uHasAlpha':0,'uAlphaFunction':4}.items():scalar(name,value,True)
            for name,value in {'uYOffset':0.,'uCutoff':.5,'uModelHeight':2.,'uGlossiness':16.,'uTextureScale':1.}.items():scalar(name,value)
            for name,value in {'uOriginOffset':[0,0,0],'uEye':[0,1000,0],'uSunDirection':[0,1,0],
                               'uAmbientLight':[.2,.3,.4],'uSunColor':[.5,.6,.7],'uSpecularLight':[.5,.6,.7],'uAmbientMaterial':[1,1,1],
                               'uSpecularMaterial':[0,0,0],'uEmissiveMaterial':[0,0,0],'uColor':[1,1,1,1]}.items():vector(name,value)
            gl.glActiveTexture(0x84c0);gl.glBindTexture(0x0de1,diffuse)
            pixels=(C.c_ubyte*4)(128,192,64,255);gl.glTexImage2D(0x0de1,0,0x8058,1,1,0,0x1908,0x1401,pixels)
            gl.glDisable(0xbe2)
        def render(clear=(0,0,0,1)):
            gl.glClearColor(*clear);gl.glClear(0x4000|0x100)
            ctx.check('fixture clear')
            gl.glDrawArraysInstanced(4,0,3,1);ctx.check('production model draw')
            pixels=ctx.rgba();return tuple(pixels[(32*64+32)*4:(32*64+32)*4+4]),pixels
        def expect(name,pixel,wanted,tolerance=2):
            if any(abs(pixel[i]-wanted[i])>tolerance for i in range(len(wanted))):raise AssertionError(f'{name}: {pixel} expected {wanted}')
            checks.append({'case':name,'pixel':pixel,'expected':wanted,'tolerance':tolerance})
        defaults();pixel,_=render();expect('legacy ambient plus diffuse',pixel,(90,173,70,255))
        defaults();vertices(normal=(0,-1,0));vector('uAmbientMaterial',[.5,.25,1]);pixel,_=render();expect('source ambient independent of diffuse',pixel,(13,14,26,255))
        defaults();vector('uAmbientLight',[0,0,0]);vector('uSunColor',[0,0,0]);vector('uEmissiveMaterial',[.25,.5,.75]);pixel,_=render();expect('source emissive',pixel,(32,96,48,255))
        defaults();vector('uAmbientLight',[0,0,0]);vector('uColor',[0,0,0,1]);vector('uSunColor',[.25,.25,.25]);vector('uSpecularLight',[.25,.25,.25]);vector('uSpecularMaterial',[1,1,1]);pixel,_=render();expect('nonblack specular disabled',pixel,(0,0,0,255))
        scalar('uSpecularEnabled',1,True);pixel,_=render();expect('source specular enable',pixel,(64,64,64,255))
        defaults();scalar('uVertexMode',2,True);vertices(color=(.25,.5,.75,1));pixel,_=render();expect('source vertex ambient-diffuse mode',pixel,(22,86,53,255))
        defaults();scalar('uLightingMode',0,True);vector('uEmissiveMaterial',[.5,.25,1]);pixel,_=render();expect('emissive-only lighting mode',pixel,(64,48,64,255))
        defaults();scalar('uHasAlpha',3,True);scalar('uCutoff',128/255)
        gl.glActiveTexture(0x84c0);gl.glBindTexture(0x0de1,diffuse);rgba=(C.c_ubyte*4)(128,192,64,128);gl.glTexImage2D(0x0de1,0,0x8058,1,1,0,0x1908,0x1401,rgba)
        pixel,_=render((.125,.25,.375,1));expect('GREATER rejects exact alpha threshold',pixel,(32,64,96,255))
        scalar('uCutoff',127/255);pixel,_=render((.125,.25,.375,1));expect('GREATER accepts next alpha byte',pixel,(90,173,70,128))
        # Native source origins move the actual production vertex positions.
        defaults();pixel,base=render();vector('uOriginOffset',[.5,0,0]);pixel,shift=render()
        def centroid(pixels,predicate):
            points=[(i%64,i//64) for i in range(64*64) if predicate(pixels[i*4:i*4+4])]
            if not points:raise AssertionError('No fixture pixels')
            return [sum(point[a] for point in points)/len(points) for a in (0,1)]
        base_center=centroid(base,lambda p:p[0]>0);shift_center=centroid(shift,lambda p:p[0]>0)
        if not 13<shift_center[0]-base_center[0]<17:raise AssertionError(f'origin shift {base_center}->{shift_center}')
        checks.append({'case':'source origin changes actual GPU geometry','centroidBefore':base_center,'centroidAfter':shift_center})
        defaults();scalar('uShadow',1,True);scalar('uTerrainEnabled',1,True)
        vertices(positions=[(.5,1,.5),(1.5,1,.5),(1,1,1.5)])
        matrix([1,0,0,0,0,.5,0,0,0,1,0,0,-1,-1,0,1])
        gl.glEnable(0xbe2);gl.glBlendFunc(0x302,0x303)
        _,flat=render((1,1,1,1));gl.glActiveTexture(0x84c0+6);gl.glBindTexture(0x0de1,terrain)
        hill=(F*9)(0,.25,.5,0,.25,.5,0,.25,.5);gl.glTexImage2D(0x0de1,0,0x822e,3,3,0,0x1903,0x1406,hill)
        _,slope=render((1,1,1,1));flat_center=centroid(flat,lambda p:p[0]<240);slope_center=centroid(slope,lambda p:p[0]<240)
        if not 3<slope_center[1]-flat_center[1]<5:raise AssertionError(f'shadow slope projection {flat_center}->{slope_center}')
        checks.append({'case':'shadow follows sampled sloping terrain','centroidFlat':flat_center,'centroidSlope':slope_center,'algorithm':'own terrain-conforming projector, not native algorithm'})
        # Production IndexPal branch against independent source table arithmetic.
        if payload.get('lighting'):
            source=payload['lighting'];factor=source['nativeColorToFloat']['floatFactor'];brightness=source['sceneLight']['selected']['Dimmer']
            for profile in ('base','native155'):
                for period in ('Afternoon','Night'):
                    table=source['profiles'][profile];tod=table['timeOfDay'][table['climates']['Temperate'][period]]
                    row=table['atmospheres'][tod['atmosphere']['Normal']];source_direction=tod['sunDirectionSource'];length=math.sqrt(sum(x*x for x in source_direction))
                    direction=[-source_direction[0]/length,-source_direction[2]/length,source_direction[1]/length]
                    ambient=[x*factor*brightness for x in row['sourceRGBA']['ambient'][:3]];sun=[x*factor*brightness for x in row['sourceRGBA']['diffuse'][:3]]
                    for low in (False,True):
                        for coefficient in (-1,0,1):
                            defaults();scalar('uNativeLighting',1,True);scalar('uNativeShader',2 if low else 1,True);scalar('uTextureScale',4.)
                            scalar('uVertexMode',2,True);scalar('uSpecularEnabled',1,True);scalar('uHasAlpha',2,True)
                            # Neither RGB material channels, specular nor vertex alpha
                            # participates in the explicit IndexPal source program.
                            vector('uColor',[0,0,0,.25]);vector('uEmissiveMaterial',[1,1,1]);vector('uSpecularMaterial',[1,1,1]);vector('uSpecularLight',[1,1,1]);vector('uAmbientLight',ambient);vector('uSunColor',sun);vector('uSunDirection',direction)
                            normal=direction if coefficient==1 else[-x for x in direction]if coefficient==-1 else[direction[2],0,-direction[0]]
                            vertices(normal=normal,color=(0,0,0,0));pixel,_=render()
                            weight=.5 if low else coefficient
                            wanted=tuple(round(4*texel*min(1,max(0,ambient[i]+weight*sun[i])))for i,texel in enumerate((128,192,64)))+(64,)
                            wanted=tuple(min(255,x)for x in wanted)
                            expect(f'IndexPal {profile}/{period}/{"Low"if low else"High"} dot{coefficient}',pixel,wanted)
            # Static source stage magnifies the texture contribution, preserving
            # a separate enabled specular term with its own source light colour.
            defaults();scalar('uNativeLighting',1,True);scalar('uNativeShader',0,True);scalar('uTextureScale',4.)
            vector('uAmbientLight',[.1,.1,.1]);vector('uSunColor',[0,0,0]);vector('uSpecularLight',[0,0,0]);pixel,_=render();expect('static APPLY_MODULATE4X RGB',pixel,(51,77,26,255))
            scalar('uTextureScale',1.);pixel,_=render();expect('other apply modes keep own scale',pixel,(13,19,6,255))
            scalar('uTextureScale',4.);scalar('uSpecularEnabled',1,True);vector('uSpecularMaterial',[1,1,1]);vector('uSpecularLight',[.1,.2,.3]);pixel,_=render();expect('specular source colour remains outside4x',pixel,(77,128,102,255))
        result={'format':'openepoch-original-material-gpu-test-v1','context':ctx.info,'shaderSha256':{key:hashlib.sha256(shader[key].encode()).hexdigest() for key in ('vertex','fragment')},
                'checks':checks,'passed':len(checks),'uiLaunched':False,'scope':'Production shader pixels on native headless ANGLE. No original-game visual parity or physical macOS execution claim.'}
    report.parent.mkdir(parents=True,exist_ok=True);report.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    print(json.dumps({'passed':len(checks),'context':result['context'],'report':str(report)}))


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('input',type=pathlib.Path);parser.add_argument('--report',type=pathlib.Path,required=True)
    args=parser.parse_args();run(json.loads(args.input.read_text(encoding='utf-8-sig')),args.report)
