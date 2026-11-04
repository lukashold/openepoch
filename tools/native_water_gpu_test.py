"""Real GLES candidate pixels against an independent native-formula CPU oracle."""
from pathlib import Path
import hashlib
import json
import math
import struct
import sys
from PIL import Image

PROJECT = Path(__file__).resolve().parents[1]
HERE = PROJECT.parents[1] / 'work/native-water3-qa'
ASSETS = PROJECT / 'docs/local-original/water-native'
sys.path.insert(0, str(PROJECT / 'tools'))
from replay_world_gles import replay

manifest = json.loads((ASSETS / 'manifest.json').read_text())
contract = json.loads((HERE / 'contract-and-fields.json').read_text())
f32 = lambda n: struct.unpack('<f', struct.pack('<f', n))[0]
inv255 = f32(1/255)
checks = 0
errors = []


def require(value, message):
    global checks
    if not value:
        raise AssertionError(message)
    checks += 1


def source_fields(case):
    size = case['size'];width = size+1;hs = case['heights'];movement = case['movement']
    palette = manifest['paletteAlphaBytes'];lookup = manifest['lookupRGBBytes']
    raw = []
    for h, kind in zip(hs, movement):
        source = lookup[2 if h >= 0 or kind == 5 else 0 if kind == 7 else 1]
        index = max(0, min(255, int((0-h)*127.5)))
        raw.append([f32(source[0]*inv255), f32(palette[index]*inv255), 1 if kind == 7 else .5 if kind == 5 else 0, .5 if h >= 0 else f32(.35) if kind == 5 else 0])
    def get(x,z):
        return raw[max(0,min(size,z))*width+max(0,min(size,x))]
    out = []
    for z in range(width):
        for x in range(width):
            # Derive the stencils by shape/radius rather than sharing JS offsets.
            color = [get(x+dx,z+dz) for dz in range(-2,3) for dx in range(-2,3) if abs(dx)+abs(dz)<=2]
            require(len(color)==13,'Independent native13-point diamond')
            green = [get(x,z)[2]]
            blue = get(x,z)[3]*5
            for radius in range(1,5):
                ring = [get(x+dx*radius,z+dz*radius) for dz in (-1,0,1) for dx in (-1,0,1) if dx or dz]
                green.extend(p[2] for p in ring)
                if radius<=2:
                    blue += sum(p[3] for p in ring)*(2 if radius==1 else 1)
            require(len(green)==33,'Independent native33-point ocean/coast mean')
            out.append([f32(sum(p[0] for p in color)*f32(1/13)),f32(sum(green)*f32(1/33)),f32(blue*f32(1/29)),f32(sum(p[1] for p in color)*f32(1/13))])
    return out


for case in contract['fields']:
    expected = source_fields(case)
    for i, color in enumerate(expected):
        actual = case['vertices'][i*8+2:i*8+6]
        require(max(abs(a-b) for a,b in zip(actual,color)) < 0.000001,'JS native vertex channels match independent source stencil/palette oracle')

image_cache = {}


def image(key):
    if key not in image_cache:
        image_cache[key] = Image.open(ASSETS/manifest['images'][key]['url']).convert('RGBA')
    return image_cache[key]


def linear(im, u, v, repeat=True):
    width,height = im.size;x=u*width-.5;y=v*height-.5;ix=math.floor(x);iy=math.floor(y);fx=x-ix;fy=y-iy
    def px(x,y):
        if repeat:x%=width;y%=height
        else:x=max(0,min(width-1,x));y=max(0,min(height-1,y))
        return im.getpixel((x,y))
    a,b,c,d=px(ix,iy),px(ix+1,iy),px(ix,iy+1),px(ix+1,iy+1)
    return [((a[k]*(1-fx)+b[k]*fx)*(1-fy)+(c[k]*(1-fx)+d[k]*fx)*fy)/255 for k in range(4)]


def interpolate(fixture, x, z):
    size=fixture['size'];w=size+1;ix=int(x);iz=int(z);fx=x-ix;fz=z-iz;a=iz*w+ix;b=a+1;c=a+w;d=c+1
    if fx>=fz:indices=[a,b,d];weights=[1-fx,fx-fz,fz]
    else:indices=[a,d,c];weights=[1-fz,fx,fz-fx]
    visible=any(fixture['heights'][i]<0 for i in indices)
    values=[sum(fixture['vertices'][i*8+k]*t for i,t in zip(indices,weights)) for k in range(2,8)]
    return visible,values


def over(color, background):
    alpha=max(0,min(1,color[3]));return [color[k]*alpha+background[k]*(1-alpha) for k in range(3)]


reports=[]
for name in ['ocean','river','coast','shore','fallback','reflection']:
    path=HERE/f'capture-{name}.json';data=json.loads(path.read_text());fixture=data['fixture']
    gpu=replay(path,HERE/f'gpu-{name}.png',HERE/f'gpu-{name}.json')
    require(gpu['passed'] and all(p['compiled'] and p['linked'] for p in gpu['compiledPrograms']),'Actual candidate programs compile/link/render on GLES')
    output=Image.open(HERE/f'gpu-{name}.png').convert('RGB');width,height=output.size;profile=manifest['profiles'][fixture['profile']]
    reflect=Image.frombytes('RGBA',(4,4),bytes(fixture['reflectionPixels']))
    maximum=0;probes=0
    for py in [9,23,39,63,81,103,119]:
        for px in [7,19,37,59,77,99,121]:
            x=(px+.5)/width*fixture['size'];z=(py+.5)/height*fixture['size'];visible,vertex=interpolate(fixture,x,z)
            expected=fixture['background'][:3]
            if visible:
                red,green,blue,alpha,u,v=vertex;t=fixture['time']
                dark=linear(image(profile['dark']),u+t*.0155,v+t*.02)
                detail=linear(image(profile['detail']),u-t*.015,v+t*.01)
                gloss0=linear(image(profile['gloss']),u+t*.0155,v+t*.02)
                gloss1=linear(image(profile['gloss']),u-t*.015,v+t*.01)
                mixed=[dark[k]*detail[k]*green+gloss0[k]*gloss1[k]*(1-green) for k in range(4)]
                source=[mixed[0]*red,mixed[1]*red+blue,mixed[2]*red+blue,mixed[3]*alpha]
                expected=over(source,expected)
                if fixture['reflection']:
                    u0=x*manifest['adapter']['normalUvScale'][0]+(t*.01125)%1;v0=z*manifest['adapter']['normalUvScale'][1]+(t*.02)%1
                    u1=z*manifest['adapter']['normalUvScale'][1]+(t*.01)%1;v1=x*manifest['adapter']['normalUvScale'][0]+(-t*.01125)%1
                    first=linear(image('normal'),u0,v0);second=linear(image('normal'),u1,v1)
                    ru=max(.001,min(.999,x/fixture['size']+(first[0]+second[0]-1)*.1));rv=max(.001,min(.999,z/fixture['size']+(first[1]+second[1]-1)*.1))
                    sample=linear(reflect,ru,rv,False);sample[3]*=manifest['native']['reflectionAlpha'];expected=over(sample,expected)
            actual=output.getpixel((px,height-1-py));delta=max(abs(actual[k]-round(max(0,min(1,expected[k]))*255)) for k in range(3))
            maximum=max(maximum,delta);probes+=1
            require(delta<=3,f'Native CPU PS/palette/overlay oracle{name} pixel({px},{py}) expected{expected} actual{actual} delta{delta}')
    reports.append({'case':name,'probes':probes,'maximumChannelError':maximum,'gpuReport':f'gpu-{name}.json','png':f'gpu-{name}.png'})
report={'format':'native3-water-source-gpu-tests-v1','passed':True,'checks':checks,'jsContractChecks':contract['checks'],'cases':reports,
        'sourceSha256':hashlib.sha256((PROJECT/'docs/original-water.js').read_bytes()).hexdigest(),
        'manifestSha256':hashlib.sha256((ASSETS/'manifest.json').read_bytes()).hexdigest(),
        'oracle':'Independent Python diamond/ring filters and source active high PS multiply/lerp/channel formula, original PNG bilinear sampler, source palette depth/2 and source reflection32percent overlay.',
        'scope':'Production water-native3. Real ANGLE EGL/GLES images from actual JS loader/passes; fixed native movement/height input. No original executable/browser/UI and no image-equality claim.',
        'adapters':['Own six-class mapping when explicit movement is absent','UV/normal scale until native defaults are recovered','Current fog-of-war adapter and reflected projection/clamp behavior','Float32 output with source constants, not asserted byte-identical native x87 intermediate rounding.']}
(PROJECT.parent/'EE2-Research/analysis/native3-water-gpu-tests.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'checks':checks,'contractChecks':contract['checks'],'cases':reports}))
