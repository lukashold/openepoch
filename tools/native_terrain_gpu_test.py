"""Render the production terrain/water shaders on an EGL pbuffer, without UI."""
from __future__ import annotations
import ctypes as C
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw
from headless_gles import HeadlessGLES, I, U, F, P

ROOT = Path(__file__).resolve().parents[1]
RESEARCH = ROOT.parent / 'EE2-Research/analysis'
manifest = json.loads((ROOT / 'docs/local-original/terrain-native/manifest.json').read_text())
programs = json.loads((RESEARCH / 'native-terrain-shaders.json').read_text())['programs']
checks = []
images = []

def require(value, name):
    if not value:
        raise AssertionError(name)
    checks.append(name)

with HeadlessGLES(size=192) as ctx:
    gl = ctx.gl
    gen_textures = ctx.bind('glGenTextures', None, I, C.POINTER(U))
    active = ctx.bind('glActiveTexture', None, U)
    bind_texture = ctx.bind('glBindTexture', None, U, U)
    tex_image = ctx.bind('glTexImage2D', None, U, I, I, I, I, I, U, U, P)
    tex_image3 = ctx.bind('glTexImage3D', None, U, I, I, I, I, I, I, U, U, P)
    tex_param = ctx.bind('glTexParameteri', None, U, U, I)
    mipmaps = ctx.bind('glGenerateMipmap', None, U)
    pixel_store = ctx.bind('glPixelStorei', None, U, I)
    pixel_store(0x0cf5, 1)
    targets={}
    def texture(image=None, pixels=None, width=1, height=1, floating=False, repeat=True, image_array=None, array_depth=0):
        value = U()
        target=0x8c1a if image_array or array_depth else 0x0de1
        gen_textures(1, C.byref(value)); active(0x84c0); bind_texture(target, value)
        if image_array:
            frames=[Image.open(ROOT/'docs'/url).convert('RGBA') for url in image_array]
            width,height=frames[0].size;array_depth=len(frames)
            if any(frame.size!=(width,height) for frame in frames):
                raise ValueError('Monthly fixture dimensions do not match')
            payload=C.create_string_buffer(b''.join(frame.tobytes() for frame in frames))
        elif image:
            source = Image.open(ROOT / 'docs' / image).convert('RGBA')
            width, height = source.size
            payload = C.create_string_buffer(source.tobytes())
        elif floating:
            payload = (F * len(pixels))(*pixels)
        else:
            payload = (C.c_ubyte * len(pixels))(*pixels)
        if target==0x8c1a:
            tex_image3(target,0,0x8058,width,height,array_depth,0,0x1908,0x1401,payload)
        else:
            tex_image(target, 0, 0x822e if floating else 0x8058, width, height, 0, 0x1903 if floating else 0x1908, 0x1406 if floating else 0x1401, payload)
        tex_param(target, 0x2801, 0x2600 if floating else 0x2703)
        tex_param(target, 0x2800, 0x2600 if floating else 0x2601)
        for key in (0x2802, 0x2803):
            tex_param(target, key, 0x2901 if repeat else 0x812f)
        if not floating:
            mipmaps(target)
        ctx.check('texture upload')
        targets[value.value]=target
        return value.value
    def bind(unit, value):
        active(0x84c0 + unit); bind_texture(targets[value], value)
    uniform_location = ctx.bind('glGetUniformLocation', I, U, C.c_char_p)
    uniform_i = ctx.bind('glUniform1i', None, I, I)
    uniform_f = ctx.bind('glUniform1f', None, I, F)
    uniform_3 = ctx.bind('glUniform3fv', None, I, I, C.POINTER(F))
    uniform_2 = ctx.bind('glUniform2fv', None, I, I, C.POINTER(F))
    uniform_matrix = ctx.bind('glUniformMatrix4fv', None, I, I, C.c_ubyte, C.POINTER(F))
    use = ctx.bind('glUseProgram', None, U)
    bindings = {}
    def location(p, name):
        return uniform_location(p, name.encode())
    def sampler(p, name, unit, value):
        bindings.setdefault(p,{})[unit]=value
        bind(unit, value); uniform_i(location(p, name), unit)
    matrix = (F * 16)(1/128,0,0,0, 0,0,0,0, 0,1/128,0,0, -1,-1,0,1)
    vao = U()
    ctx.bind('glGenVertexArrays', None, I, C.POINTER(U))(1, C.byref(vao))
    ctx.bind('glBindVertexArray', None, U)(vao)
    buffer = U()
    ctx.bind('glGenBuffers', None, I, C.POINTER(U))(1, C.byref(buffer))
    ctx.bind('glBindBuffer', None, U, U)(0x8892, buffer)
    upload = ctx.bind('glBufferData', None, U, C.c_ssize_t, P, U)
    enable = ctx.bind('glEnableVertexAttribArray', None, U)
    pointer = ctx.bind('glVertexAttribPointer', None, U, I, U, C.c_ubyte, I, P)
    draw = ctx.bind('glDrawArrays', None, U, I, I)
    viewport = ctx.bind('glViewport', None, I, I, I, I)
    viewport(0, 0, ctx.size, ctx.size)
    clear_color = ctx.bind('glClearColor', None, F, F, F, F)
    clear = ctx.bind('glClear', None, U)
    ctx.bind('glDisable', None, U)(0x0be2)
    ctx.bind('glDisable', None, U)(0x0b71)
    def render(p, water=False):
        use(p); ctx.bind('glBindVertexArray', None, U)(vao)
        for unit,value in bindings[p].items():
            bind(unit,value)
        vertices = [0,0,0,0,1,0, 256,0,0,0,1,0, 0,0,256,0,1,0, 256,0,256,0,1,0]
        data = (F * len(vertices))(*vertices)
        ctx.bind('glBindBuffer', None, U, U)(0x8892, buffer)
        upload(0x8892, C.sizeof(data), data, 0x88e4)
        enable(0); pointer(0, 2 if water else 3, 0x1406, 0, 24, P())
        if water:
            # Production water aP is x,z rather than the terrain x,y,z stream.
            points = (F * 8)(0,0,256,0,0,256,256,256)
            upload(0x8892, C.sizeof(points), points, 0x88e4); pointer(0,2,0x1406,0,0,P())
        else:
            enable(1); pointer(1,3,0x1406,0,24,P(12))
        uniform_matrix(location(p,'uVP'),1,0,matrix)
        clear_color(0,0,0,0); clear(0x4000)
        draw(5,0,4); ctx.check('production shader draw')
        return Image.frombytes('RGBA',(ctx.size,ctx.size),ctx.rgba()).transpose(Image.Transpose.FLIP_TOP_BOTTOM)
    def mean_difference(a,b):
        return sum(abs(x-y) for x,y in zip(a.tobytes(),b.tobytes()))/(ctx.size*ctx.size*4)
    terrain = ctx.shader_program(programs[0]['vertex'], programs[0]['fragment'])
    use(terrain)
    roles = {}
    for role, unit in [('grass',0),('patch',4),('forest',5),('rock',6),('beach',7),('coral',8),('rocky',9),('roots',10)]:
        definition = manifest['textureDefinitions'][manifest['runtimeRoles'][role]['textureDefinition']]
        urls = manifest['seasonalSets'][definition['seasonalSet']]['urls']
        roles[role] = texture(image_array=urls)
        sampler(terrain, 'u'+role.capitalize(), unit, roles[role])
    mask = texture(image=manifest['maskUrl'])
    sampler(terrain,'uMask',11,mask)
    fog = texture(pixels=[255,255,255,255],repeat=False)
    terr = texture(pixels=[0,0,0,0],repeat=False)
    sampler(terrain,'uFog',2,fog);sampler(terrain,'uTerr',3,terr)
    uniform_3(location(terrain,'uTeam'),8,(F*24)(*([1,1,1]*8)))
    uniform_f(location(terrain,'uPx'),1/28)
    uniform_2(location(terrain,'uSeasonMonths'),1,(F*2)(6,7));uniform_f(location(terrain,'uSeasonBlend'),0)
    materials = []
    for name, weights in [('Grass',[255,0,0,0]),('Forest',[0,255,0,0]),('Rock',[0,0,255,0]),('Beach',[0,0,0,255])]:
        sampler(terrain,'uWeights',12,texture(pixels=weights,repeat=False))
        image = render(terrain);materials.append(image);images.append((name,image))
        require(max(image.convert('RGB').getextrema()[1])>15,name+' produces native textured pixels')
    require(mean_difference(materials[0],materials[1])>3,'Grass and forest differ from terrain class IDs')
    require(mean_difference(materials[2],materials[3])>3,'Rock and beach differ from terrain class IDs')
    sampler(terrain,'uWeights',12,texture(pixels=[255,0,0,0],repeat=False))
    uniform_2(location(terrain,'uSeasonMonths'),1,(F*2)(2,3))
    uniform_f(location(terrain,'uSeasonBlend'),0);winter=render(terrain)
    uniform_f(location(terrain,'uSeasonBlend'),1);spring=render(terrain)
    uniform_f(location(terrain,'uSeasonBlend'),.5);half_season=render(terrain)
    require(mean_difference(winter,spring)>3,'Native adjacent spring snowmelt images differ')
    require(mean_difference(winter,half_season)>1 and mean_difference(spring,half_season)>1,'Fractional month renders an intermediate native season')
    require(all(min(a,b)<=h+1 and h<=max(a,b)+1 for a,b,h in zip(winter.tobytes(),spring.tobytes(),half_season.tobytes())),'Monthly native blend remains between both endpoints')
    images.extend([('Snowmelt month2',winter),('Half monthly blend',half_season),('Spring month3',spring)])
    synthetic=[]
    for month in range(12):synthetic.extend([40,80,120,255] if month==2 else [120,40,80,255])
    fixture=texture(pixels=synthetic,array_depth=12)
    sampler(terrain,'uGrass',0,fixture);sampler(terrain,'uPatch',4,fixture)
    uniform_f(location(terrain,'uSeasonBlend'),0);a=render(terrain)
    uniform_f(location(terrain,'uSeasonBlend'),1);b=render(terrain)
    uniform_f(location(terrain,'uSeasonBlend'),.5);middle=render(terrain)
    require(max(abs(h-(x+y)/2) for x,y,h in zip(a.tobytes(),b.tobytes(),middle.tobytes()))<=1,'GPU monthly interpolation is linear on known RGBA input')
    sampler(terrain,'uGrass',0,roles['grass']);sampler(terrain,'uPatch',4,roles['patch'])
    uniform_2(location(terrain,'uSeasonMonths'),1,(F*2)(6,7));uniform_f(location(terrain,'uSeasonBlend'),0)
    sampler(terrain,'uWeights',12,texture(pixels=[191,64,0,0],repeat=False))
    sampler(terrain,'uMask',11,texture(pixels=[0,0,0,0]))
    alpha_zero = render(terrain)
    sampler(terrain,'uMask',11,texture(pixels=[0,0,0,255]))
    alpha_one = render(terrain)
    require(mean_difference(alpha_zero,alpha_one)>.2,'Mask alpha changes the blend while RGB remains exactly black')
    sampler(terrain,'uMask',11,mask)
    sampler(terrain,'uFog',2,texture(pixels=[0,0,0,255],repeat=False))
    hidden = render(terrain)
    require(all(hi==0 for _,hi in hidden.convert('RGB').getextrema()),'Unexplored terrain remains black')
    sampler(terrain,'uFog',2,fog)
    gradient = bytearray()
    for y in range(256):
        for x in range(256):
            gradient.extend((255-x,x,0,0))
    sampler(terrain,'uWeights',12,texture(pixels=gradient,width=256,height=256,repeat=False))
    images.append(('Original mask transition',render(terrain)))
    water = ctx.shader_program(programs[1]['vertex'],programs[1]['fragment']);use(water)
    for name,unit,key in [('uNormal',0,'normal'),('uLayer1',4,'layer1'),('uLayer2',5,'layer2'),('uColorLut',6,'colorLut')]:
        sampler(water,name,unit,texture(image=manifest['water'][key],repeat=key!='colorLut'))
    sampler(water,'uFog',2,fog)
    sampler(water,'uReflection',7,texture(pixels=[80,100,120,255]))
    uniform_i(location(water,'uHasReflection'),0)
    uniform_3(location(water,'uEye'),1,(F*3)(128,50,160))
    uniform_f(location(water,'uTime'),0)
    sampler(water,'uHeights',13,texture(pixels=[-.8]*(257**2),width=257,height=257,floating=True,repeat=False))
    moving0=render(water,True);images.append(('Water, time0',moving0))
    require(moving0.getchannel('A').getextrema()[0]>100,'Deep water has opacity from independent height field')
    uniform_f(location(water,'uTime'),12)
    moving12=render(water,True);images.append(('Water, time12',moving12))
    require(mean_difference(moving0,moving12)>.1,'Source flow constants animate native images')
    sampler(water,'uHeights',13,texture(pixels=[.1]*(257**2),width=257,height=257,floating=True,repeat=False))
    dry=render(water,True)
    require(dry.getchannel('A').getextrema()==(0,0),'Water is discarded above land')
    coast_height=[-.8+x/160 for y in range(257) for x in range(257)]
    sampler(water,'uHeights',13,texture(pixels=coast_height,width=257,height=257,floating=True,repeat=False))
    coast=render(water,True);images.append(('Height-driven coastline',coast))
    alphas=list(coast.getchannel('A').getdata())
    require(any(a==0 for a in alphas) and any(0<a<150 for a in alphas) and any(a>150 for a in alphas),'Heightfield produces dry, shallow and deep regions')
    sampler(water,'uHeights',13,texture(pixels=[-.8]*(257**2),width=257,height=257,floating=True,repeat=False))
    uniform_i(location(water,'uHasReflection'),1);uniform_matrix(location(water,'uReflectionVP'),1,0,matrix)
    sampler(water,'uReflection',7,texture(pixels=[255,0,0,255]))
    red=render(water,True)
    sampler(water,'uReflection',7,texture(pixels=[0,0,255,255]))
    blue=render(water,True)
    require(mean_difference(red,blue)>.5,'Optional reflection texture is sampled through source distortion path')
    sheet=Image.new('RGB',(ctx.size*4,(ctx.size+24)*((len(images)+3)//4)),(20,23,25));labels=ImageDraw.Draw(sheet)
    for index,(title,image) in enumerate(images):
        x=(index%4)*ctx.size;y=(index//4)*(ctx.size+24)
        sheet.paste(image.convert('RGB'),(x,y+24));labels.text((x+4,y+6),title,fill=(235,235,235))
    preview=ROOT/'native-terrain-gpu-preview.png';sheet.save(preview)
    report={'format':'openepoch-native-terrain-gpu-pixels-v1','context':ctx.info,'checks':checks,'passed':True,
        'productionShaders':[{'name':p['name'],'vertexSha256':hashlib.sha256(p['vertex'].encode()).hexdigest(),'fragmentSha256':hashlib.sha256(p['fragment'].encode()).hexdigest()} for p in programs],
        'preview':preview.name,'scope':'Production shader programs with original PNG textures and synthetic terrain/height/fog/reflection fields. Actual pbuffer rendering and RGBA readback; no browser/UI. This is not an original-game screenshot or a full browser compatibility test.'}
    (RESEARCH/'native-terrain-gpu-pixels.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(json.dumps(report,indent=2))
