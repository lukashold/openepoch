"""Real production shader pixels with original seasonal RGBA and world clipping."""
import argparse,ctypes as C,hashlib,json,pathlib
from PIL import Image
from headless_gles import HeadlessGLES,I,U,F,P
ROOT=pathlib.Path(__file__).resolve().parents[1]
SHA=lambda b:hashlib.sha256(b).hexdigest()

def run(shaders,report):
    manifest=json.loads((ROOT/'docs/local-original/model-seasons.json').read_text())
    checks=[];images={}
    def image(url):
        if url not in images:images[url]=Image.open(ROOT/'docs'/url).convert('RGBA')
        return images[url]
    with HeadlessGLES(32)as ctx:
        gl=ctx.gl
        specs=[('glUseProgram',None,[U]),('glGenVertexArrays',None,[I,C.POINTER(U)]),('glBindVertexArray',None,[U]),('glGenBuffers',None,[I,C.POINTER(U)]),('glBindBuffer',None,[U,U]),('glBufferData',None,[U,C.c_ssize_t,P,U]),('glEnableVertexAttribArray',None,[U]),('glVertexAttribPointer',None,[U,I,U,U,I,P]),('glVertexAttribDivisor',None,[U,U]),('glGenTextures',None,[I,C.POINTER(U)]),('glActiveTexture',None,[U]),('glBindTexture',None,[U,U]),('glTexImage2D',None,[U,I,I,I,I,I,U,U,P]),('glTexImage3D',None,[U,I,I,I,I,I,I,U,U,P]),('glTexParameteri',None,[U,U,I]),('glGetUniformLocation',I,[U,C.c_char_p]),('glUniform1i',None,[I,I]),('glUniform1f',None,[I,F]),('glUniform3fv',None,[I,I,C.POINTER(F)]),('glUniform4fv',None,[I,I,C.POINTER(F)]),('glUniformMatrix4fv',None,[I,I,U,C.POINTER(F)]),('glViewport',None,[I,I,I,I]),('glClearColor',None,[F,F,F,F]),('glClear',None,[U]),('glDrawArraysInstanced',None,[U,I,I,I]),('glDisable',None,[U])]
        for name,restype,args in specs:ctx.bind(name,restype,*args)
        program=ctx.shader_program(shaders['vertex'],shaders['fragment']);gl.glUseProgram(program)
        loc=lambda name:gl.glGetUniformLocation(program,name.encode())
        def scalar(name,v,integer=False):(gl.glUniform1i if integer else gl.glUniform1f)(loc(name),v)
        def vector(name,v):(gl.glUniform3fv if len(v)==3 else gl.glUniform4fv)(loc(name),1,(F*len(v))(*v))
        def matrix(v):gl.glUniformMatrix4fv(loc('uVP'),1,0,(F*16)(*v))
        def buffer(values):
            b=U();gl.glGenBuffers(1,C.byref(b));gl.glBindBuffer(0x8892,b);data=(F*len(values))(*values);gl.glBufferData(0x8892,C.sizeof(data),data,0x88e4);return b
        vao=U();gl.glGenVertexArrays(1,C.byref(vao));gl.glBindVertexArray(vao)
        vb=buffer([0.]*36)
        for index,count,offset in [(0,3,0),(1,3,12),(2,2,24),(6,4,32)]:gl.glEnableVertexAttribArray(index);gl.glVertexAttribPointer(index,count,0x1406,0,48,P(offset))
        ib=buffer([0,0,0,0,0,0,0,32,1,0,0,0])
        for index in (3,4,5):gl.glEnableVertexAttribArray(index);gl.glVertexAttribPointer(index,4,0x1406,0,48,P((index-3)*16));gl.glVertexAttribDivisor(index,1)
        textures={}
        def texture(unit,im=None,array=False):
            t=U();gl.glGenTextures(1,C.byref(t));textures[unit]=t
            target=0x8c1a if array else 0x0de1;gl.glActiveTexture(0x84c0+unit);gl.glBindTexture(target,t)
            if array:gl.glTexImage3D(target,0,0x8814,1,1,1,0,0x1908,0x1406,(F*4)(0,0,0,0))
            else:
                im=im or Image.new('RGBA',(1,1),(255,255,255,255));data=(C.c_ubyte*(im.width*im.height*4)).from_buffer_copy(im.tobytes());gl.glTexImage2D(target,0,0x8058,im.width,im.height,0,0x1908,0x1401,data)
            for p,v in [(0x2801,0x2600),(0x2800,0x2600),(0x2802,0x812f),(0x2803,0x812f)]:gl.glTexParameteri(target,p,v)
        texture(0);texture(7);texture(2);texture(6);texture(5,array=True)
        for name,v in {'uDiffuse':0,'uDiffuseNext':7,'uFog':2,'uTerrain':6,'uFrames':5,'uTerrainSize':1,'uFrameCount':1,'uFrameWidth':1,'uShadow':0,'uAnimated':0,'uTerrainEnabled':0,'uSpecularEnabled':0,'uVertexMode':0,'uLightingMode':0,'uHasAlpha':0,'uAlphaFunction':4,'uClipEnabled':0}.items():scalar(name,v,True)
        for name,v in {'uModelSeasonBlend':0,'uYOffset':0,'uCutoff':180/255,'uModelHeight':2,'uGlossiness':10,'uTextureScale':1}.items():scalar(name,v)
        for name,v in {'uColor':[1,1,1,1],'uOriginOffset':[0,0,0],'uEmissiveMaterial':[1,1,1],'uAmbientMaterial':[0,0,0],'uSpecularMaterial':[0,0,0],'uEye':[0,10,0],'uSunDirection':[0,1,0],'uSunColor':[0,0,0],'uAmbientLight':[0,0,0],'uClipPlane':[0,0,0,0]}.items():vector(name,v)
        identity=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];matrix(identity)
        gl.glViewport(0,0,32,32)
        for cap in (0xb71,0xb44,0xbe2):gl.glDisable(cap)
        def draw(uv):
            vertices=[]
            for pos in [(-1,-1,0),(3,-1,0),(-1,3,0)]:vertices.extend([*pos,0,1,0,*uv,1,1,1,1])
            gl.glBindBuffer(0x8892,vb);data=(F*len(vertices))(*vertices);gl.glBufferData(0x8892,C.sizeof(data),data,0x88e4)
            gl.glClearColor(17/255,31/255,47/255,1);gl.glClear(0x4000|0x100);gl.glDrawArraysInstanced(4,0,3,1);ctx.check('season source draw');data=ctx.rgba();return list(data[(16*32+16)*4:(16*32+16)*4+4])
        def expect(name,got,wanted):
            if any(abs(a-b)>2 for a,b in zip(got,wanted)):raise AssertionError(f'{name}: {got},expected{wanted}')
            checks.append({'case':name,'pixel':got,'expected':wanted,'tolerance':2})
        # Actual winter/summer foliage, opaque building skin, and original player patch.
        pairs=[('oak',manifest['sets']['Temperate']['tre_oak01']['urls'][0],manifest['sets']['Temperate']['tre_oak01']['urls'][3]),
            ('pine',manifest['sets']['Temperate']['tre_pine']['urls'][0],manifest['sets']['Temperate']['tre_pine']['urls'][3]),
            ('city',manifest['sets']['Temperate']['bld_citycenter1_we']['urls'][0],manifest['sets']['Temperate']['bld_citycenter1_we']['urls'][3])]
        farm_path=ROOT/'docs/local-original/farm-composites.json'
        if farm_path.is_file():
            # Follow the exact model/node annotation to its source seasonal list;
            # crop names and seasonal filenames are never matched heuristically.
            farms=json.loads(farm_path.read_text())
            for family,definition in farms['families'].items():
                url=definition['states']['Harvest_3']['url'];nodes=manifest['models'][url]['parts']
                keys={node['key'].casefold()for node in nodes.values()};assert len(keys)==1
                sequence=manifest['sets']['Temperate'][list(keys)[0]]['urls']
                pairs.append((family+' native seasonal skin',sequence[0],sequence[len(sequence)//2]))
        patches=json.loads((ROOT/'docs'/manifest['teamManifest']).read_text())['textures']['bld_citycenter1_we_wi']['variants']
        pairs.append(('winter team A/B',patches['A']['url'],patches['B']['url']))
        for label,first,next in pairs:
            a,b=image(first),image(next);assert a.size==b.size
            candidates=[(x,y)for y in range(0,a.height,3)for x in range(0,a.width,3)if a.getpixel((x,y))[3]>200 and b.getpixel((x,y))[3]>200]
            xy=max(candidates,key=lambda p:sum(abs(a.getpixel(p)[i]-b.getpixel(p)[i])for i in range(3)))
            ca,cb=a.getpixel(xy),b.getpixel(xy);uv=((xy[0]+.5)/a.width,(xy[1]+.5)/a.height)
            texture(0,a);texture(7,b)
            for mix in (0,.25,.5,.75,1):
                scalar('uModelSeasonBlend',mix);expect(f'{label} source RGBA mix{mix}',draw(uv),[round(ca[i]*(1-mix)+cb[i]*mix)for i in range(3)]+[255])
        a,b=image(pairs[0][1]),image(pairs[0][2]);xy=max(((x,y)for y in range(a.height)for x in range(a.width)),key=lambda p:abs(a.getpixel(p)[3]-b.getpixel(p)[3]))
        uv=((xy[0]+.5)/a.width,(xy[1]+.5)/a.height);texture(0,a);texture(7,b);scalar('uHasAlpha',3,True)
        for mix in (0,.25,.5,.75,1):
            scalar('uModelSeasonBlend',mix);color=[round(a.getpixel(xy)[i]*(1-mix)+b.getpixel(xy)[i]*mix)for i in range(4)]
            expect(f'foliage alpha cutout mix{mix}',draw(uv),color if color[3]>180 else [17,31,47,255])
        scalar('uHasAlpha',0,True);scalar('uModelSeasonBlend',0);scalar('uClipEnabled',1,True);vector('uClipPlane',[0,1,0,-.5])
        expect('world plane clips below reflection water height',draw(uv),[17,31,47,255])
        gl.glBindBuffer(0x8892,ib);instance=(F*12)(0,0,1,0,0,0,0,32,1,0,0,0);gl.glBufferData(0x8892,C.sizeof(instance),instance,0x88e4)
        moved=identity.copy();moved[13]=-1;matrix(moved)
        expect('world clip follows instance height after camera compensation',draw(uv),list(a.getpixel(xy))[:3]+[255])
        scalar('uClipEnabled',0,True);vector('uClipPlane',[0,1,0,-100]);expect('default draw disables stale reflection clipping',draw(uv),list(a.getpixel(xy))[:3]+[255])
        result={'status':'passed','format':'ee2-model-season-gpu-v1','passed':len(checks),'checks':checks,'context':ctx.info,'uiLaunched':False,'productionShaders':{k:SHA(shaders[k].encode())for k in ('vertex','fragment')},'sourceManifestSha256':SHA((ROOT/'docs/local-original/model-seasons.json').read_bytes()),'sourceImages':{url:SHA((ROOT/'docs'/url).read_bytes())for url in images},'scope':'Actual original texture pixels through production GLES shaders: seasonal RGBA interpolation, foliage alpha test and world-space clipping. No original-game screenshot or complete visual parity claim.'}
    report.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8');print(json.dumps({'status':'passed','passed':len(checks),'renderer':result['context']['renderer'],'report':str(report)}))
if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('input',type=pathlib.Path);ap.add_argument('--report',type=pathlib.Path,required=True);args=ap.parse_args();run(json.loads(args.input.read_text())['programs'][0],args.report)
