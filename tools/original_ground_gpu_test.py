"""Production ground-fragment sampling checks using native PNGs and no UI."""
import argparse,ctypes as C,hashlib,json,math,pathlib
from PIL import Image
from headless_gles import HeadlessGLES,I,U,F,P
ROOT=pathlib.Path(__file__).resolve().parents[1]


def run(payload,report):
    shaders=payload['programs'][0];checks=[];images={}
    with HeadlessGLES(64) as ctx:
        gl=ctx.gl
        for name,result,args in [
          ('glUseProgram',None,[U]),('glGetUniformLocation',I,[U,C.c_char_p]),('glUniform1i',None,[I,I]),('glUniform1f',None,[I,F]),('glUniformMatrix4fv',None,[I,I,C.c_ubyte,C.POINTER(F)]),
          ('glGenVertexArrays',None,[I,C.POINTER(U)]),('glBindVertexArray',None,[U]),('glGenBuffers',None,[I,C.POINTER(U)]),('glBindBuffer',None,[U,U]),
          ('glBufferData',None,[U,C.c_ssize_t,P,U]),('glEnableVertexAttribArray',None,[U]),('glVertexAttribPointer',None,[U,I,U,C.c_ubyte,I,P]),('glVertexAttribDivisor',None,[U,U]),
          ('glGenTextures',None,[I,C.POINTER(U)]),('glActiveTexture',None,[U]),('glBindTexture',None,[U,U]),('glTexImage2D',None,[U,I,I,I,I,I,U,U,P]),('glTexParameteri',None,[U,U,I]),
          ('glViewport',None,[I,I,I,I]),('glClearColor',None,[F,F,F,F]),('glClear',None,[U]),('glDrawArraysInstanced',None,[U,I,I,I]),('glDisable',None,[U]),('glEnable',None,[U]),('glBlendFunc',None,[U,U])]:ctx.bind(name,result,*args)
        program=ctx.shader_program(shaders['vertex'],shaders['fragment']);gl.glUseProgram(program)
        loc=lambda name:gl.glGetUniformLocation(program,name.encode())
        vao=U();gl.glGenVertexArrays(1,C.byref(vao));gl.glBindVertexArray(vao)
        buffers=[]
        def buffer(values):
            b=U();gl.glGenBuffers(1,C.byref(b));buffers.append(b);gl.glBindBuffer(0x8892,b)
            data=(F*len(values))(*values);gl.glBufferData(0x8892,C.sizeof(data),data,0x88e4);return b
        buffer([0,0,1,0,0,1,1,1]);gl.glEnableVertexAttribArray(0);gl.glVertexAttribPointer(0,2,0x1406,0,8,P())
        instances=buffer([0,0,0,0,0])
        for index,size,offset in [(1,4,0),(2,1,16)]:
            gl.glEnableVertexAttribArray(index);gl.glVertexAttribPointer(index,size,0x1406,0,20,P(offset));gl.glVertexAttribDivisor(index,1)
        textures=[]
        def texture(unit,width,height,data,float32=False):
            t=U();gl.glGenTextures(1,C.byref(t));textures.append(t)
            gl.glActiveTexture(0x84c0+unit);gl.glBindTexture(0x0de1,t)
            values=((F if float32 else C.c_ubyte)*len(data))(*data)
            gl.glTexImage2D(0x0de1,0,0x822e if float32 else 0x8058,width,height,0,0x1903 if float32 else 0x1908,0x1406 if float32 else 0x1401,values)
            for key,value in [(0x2801,0x2600 if float32 else 0x2601),(0x2800,0x2600 if float32 else 0x2601),(0x2802,0x812f),(0x2803,0x812f)]:gl.glTexParameteri(0x0de1,key,value)
            ctx.check('native ground texture upload');return t
        texture(2,1,1,[255,255,255,255]);texture(6,257,257,[0.]*(257*257),True)
        for name,unit in [('uDiffuse',0),('uDiffuseB',7),('uFog',2),('uHeight',6)]:gl.glUniform1i(loc(name),unit)
        gl.glViewport(0,0,64,64);gl.glDisable(0xb71);gl.glDisable(0xb44);gl.glEnable(0xbe2);gl.glBlendFunc(1,0x303)
        light=.55+.6*.82/math.sqrt(.42**2+.82**2+.39**2)
        probes=[(x,z) for z in (16,32,48) for x in (16,32,48)]
        def sample(image,x,z):
            sx=(x+.5)/64*image.width-.5;sz=(z+.5)/64*image.height-.5
            ix,iz=math.floor(sx),math.floor(sz);fx,fz=sx-ix,sz-iz
            values=[]
            for axis in range(4):
                a=image.getpixel((ix,iz))[axis];b=image.getpixel((ix+1,iz))[axis];c=image.getpixel((ix,iz+1))[axis];d=image.getpixel((ix+1,iz+1))[axis]
                values.append((a+(b-a)*fx)*(1-fz)+(c+(d-c)*fx)*fz)
            return values
        def image_data(url):
            if url not in images:
                path=ROOT/'docs'/url;image=Image.open(path).convert('RGBA')
                images[url]={'image':image,'texture':texture(0,image.width,image.height,list(image.tobytes())),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
            return images[url]
        for fixture in payload['fixtures']:
            source=image_data(fixture['sprite']);other=image_data(fixture.get('nextSprite',fixture['sprite']));blend=fixture.get('blend',0)
            cx,cz,angle,width,depth=fixture['placement']
            if angle!=0:raise AssertionError('Axis-aligned UV fixture required')
            gl.glActiveTexture(0x84c0);gl.glBindTexture(0x0de1,source['texture'])
            gl.glActiveTexture(0x84c0+7);gl.glBindTexture(0x0de1,other['texture']);gl.glUniform1f(loc('uSeasonMix'),blend)
            gl.glBindBuffer(0x8892,instances);values=(F*5)(cx,cz,angle,width,depth);gl.glBufferData(0x8892,C.sizeof(values),values,0x88e4)
            matrix=(F*16)(2/width,0,0,0,0,0,0,0,0,2/depth,0,0,-2*cx/width,-2*cz/depth,0,1)
            gl.glUniformMatrix4fv(loc('uVP'),1,0,matrix);gl.glClearColor(0,0,0,1);gl.glClear(0x4000);gl.glDrawArraysInstanced(5,0,4,1);ctx.check('native ground sprite draw')
            pixels=ctx.rgba();samples=[]
            for x,z in probes:
                observed=list(pixels[(z*64+x)*4:(z*64+x)*4+4]);a=sample(source['image'],x,z);b=sample(other['image'],x,z)
                values=[a[i]*(1-blend)+b[i]*blend for i in range(4)]
                expected=[0,0,0,255] if values[3]/255<.004 else [min(255,round(values[i]*light*values[3]/255)) for i in range(3)]+[255]
                if any(abs(observed[i]-expected[i])>3 for i in range(4)):raise AssertionError(f'{fixture}: probe{x,z} {observed} expected{expected}')
                samples.append({'at':[x,z],'observed':observed,'expected':expected})
            checks.append({**fixture,'pngSha256':source['sha256'],'nextPngSha256':other['sha256'],'probes':samples})
        result={'format':'openepoch-original-ground-gpu-tests-v1','context':ctx.info,'uiLaunched':False,'fixtureCount':len(checks),'pixelProbes':len(checks)*len(probes),
                'uniquePhysicalPngs':len(images),'shaderSha256':{key:hashlib.sha256(shaders[key].encode()).hexdigest() for key in ('vertex','fragment')},
                'scope':'Original source slots, seasonal frame pairs and unchanged native PNGs through the exact production ground shaders; independent bilinear-top-mip/alpha/lighting/interpolation pixel expectations. Native epoch/context predicates are covered separately, not inferred from these images.',
                'checks':checks}
    report.parent.mkdir(parents=True,exist_ok=True);report.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    print(json.dumps({key:result[key] for key in ('fixtureCount','pixelProbes','uniquePhysicalPngs','uiLaunched')}))


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('input',type=pathlib.Path);parser.add_argument('--report',type=pathlib.Path,required=True)
    args=parser.parse_args();run(json.loads(args.input.read_text(encoding='utf-8-sig')),args.report)
