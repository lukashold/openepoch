"""Composite production component sprite and bitmap draw inputs; no UI launch."""
from pathlib import Path
import base64,hashlib,json,math,re
from urllib.parse import urlsplit
from urllib.request import url2pathname
from PIL import Image,ImageOps
ROOT=Path(__file__).resolve().parents[3];PROJECT=ROOT/'outputs/OpenEpoch';DOCS=PROJECT/'docs';HERE=PROJECT/'work/native6-fonts';OUT=DOCS/'local-original/fonts-native';SIZE=(1280,720)
ALLOWED=[(DOCS/'local-original').resolve()]
def number(value,default=0):
    try:return float(str(value).removesuffix('px'))
    except ValueError:return default
def integer(value):return math.floor(number(value)+.5)
def source_sprite(style,width,height):
    match=re.search(r'url\("([^"]+)"\)',style.get('backgroundImage',''))
    if not match:return None
    if match[1].startswith('data:image/x-test-rgba;'):
        head,data=match[1].split(',',1);w,h=[int(re.search(r';'+key+r'=(\d+)',head)[1]) for key in ('w','h')];source=Image.frombytes('RGBA',(w,h),base64.b64decode(data))
    else:
        path=Path(url2pathname(urlsplit(match[1]).path)).resolve()
        if not any(path.is_relative_to(root) for root in ALLOWED):raise ValueError('Source sprite outside declared roots')
        source=Image.open(path).convert('RGBA')
    repeat=style.get('backgroundRepeat','')
    if repeat not in ('repeat-x','repeat-y'):return source.resize((width,height),Image.Resampling.BILINEAR)
    values=style.get('backgroundSize','').split();tw=max(1,integer(values[0] if values else width));th=max(1,integer(values[1] if len(values)>1 else height));tile=source.resize((tw,th),Image.Resampling.BILINEAR);out=Image.new('RGBA',(width,height))
    for y in range(0,height,th):
        for x in range(0,width,tw):out.alpha_composite(tile,(x,y))
    return out
def main():
    atlas={};reports=[];background=ROOT/'outputs/OpenEpoch/native5-world-summer.png'
    for name in ('hud','relations','resources','proposal','territories'):
        rows=json.loads((HERE/('ui-'+name+'.json')).read_text(encoding='utf8'));image=Image.open(background).convert('RGBA').resize(SIZE);labels=0;glyphs=0;sprites=0
        for row in rows:
            if row['hidden']:continue
            style=row['style']
            if not all(key in style for key in ('left','top','width','height')):continue
            x,y,w,h=[integer(style[key]) for key in ('left','top','width','height')]
            if w<=0 or h<=0:continue
            sprite=source_sprite(style,w,h)
            if sprite:
                if row['disabled']:sprite.putalpha(sprite.getchannel('A').point(lambda a:a//2))
                image.alpha_composite(sprite,(x,y));sprites+=1
            if row.get('src'):
                path=Path(url2pathname(urlsplit(row['src']).path)).resolve()
                if not any(path.is_relative_to(root) for root in ALLOWED):raise ValueError('Source image outside local-original')
                source=Image.open(path).convert('RGBA');contained=ImageOps.contain(source,(w,h),Image.Resampling.BILINEAR);image.alpha_composite(contained,(x+(w-contained.width)//2,y+(h-contained.height)//2))
            if row.get('pixels'):
                data=row['pixels'];pixelImage=Image.frombytes('RGBA',(data['width'],data['height']),base64.b64decode(data['rgba']));image.alpha_composite(pixelImage.resize((w,h),Image.Resampling.NEAREST),(x,y))
            if row.get('bitmapDraws'):
                canvas=Image.new('RGBA',(max(w,row['canvasWidth']),max(h,row['canvasHeight'])))
                for draw in row['bitmapDraws']:
                    sx,sy,sw,sh,dx,dy,dw,dh=draw['args']
                    if [sw,sh]!=[dw,dh]:raise ValueError('Unexpected glyph rescale')
                    if draw['file'] not in atlas:atlas[draw['file']]=Image.open(OUT/draw['file']).convert('RGBA')
                    glyph=atlas[draw['file']].crop((sx,sy,sx+sw,sy+sh));pixels=bytearray(glyph.tobytes());color=draw['color']
                    for i in range(0,len(pixels),4):
                        for c in range(3):pixels[i+c]=math.floor(pixels[i+c]*color[c]/255+.5)
                        pixels[i+3]=math.floor(pixels[i+3]*draw.get('alpha',1)+.5)
                    glyph=Image.frombytes('RGBA',glyph.size,bytes(pixels));canvas.alpha_composite(glyph,(integer(dx),integer(dy)));glyphs+=1
                image.alpha_composite(canvas.crop((0,0,w,h)),(x,y));labels+=1
        target=HERE/('native6-'+name+'-bitmap.png');image.convert('RGB').save(target);reports.append({'scene':name,'file':target.name,'sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'sourceSpriteDraws':sprites,'bitmapLabels':labels,'glyphDraws':glyphs})
    sourceHashes={str(path.relative_to(PROJECT)):hashlib.sha256(path.read_bytes()).hexdigest() for path in [DOCS/'original-fonts.js',DOCS/'original-font-ui.js',OUT/'bitmap-manifest.json']}
    report={'format':'native6-bitmap-live-ui-images-v1','sourceHashes':sourceHashes,'status':'passed','images':reports,'background':{'path':str(background.relative_to(ROOT)),'sha256':hashlib.sha256(background.read_bytes()).hexdigest()},'scope':'Actual production component DOM/art and active font coordinator glyph draw calls, original PNG crops and actual WASM minimap over a separately captured Native5 world background. No browser/original EXE; this is not a same-frame original screenshot.','limits':['Positioned source widgets and native bitmap overlays are rasterized. Legacy flowing treaty/territory forms are outside this non-browser layout transport.','HUD resource typography and calendar label binding are explicit application adapters. Native font-style shadow and complete native text wrapping are still unbound.']}
    (ROOT/'outputs/EE2-Research/analysis/native6-font-ui-images.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
