"""Prove preserved glyphs, complete bindings and production-layout PNG output."""
from pathlib import Path,PurePosixPath
import hashlib,json,math
from PIL import Image
PROJECT=Path(__file__).resolve().parents[1]
HERE=PROJECT/'work/native6-fonts';HERE.mkdir(parents=True,exist_ok=True)
OUT=PROJECT/'docs/local-original/fonts-native'

checks=0
def check(ok,label):
    global checks
    if not ok:raise AssertionError(label)
    checks+=1

def main():
    m=json.loads((OUT/'bitmap-manifest.json').read_text(encoding='utf8'));original=json.loads((OUT/'manifest.json').read_text(encoding='utf8'));images={};gold=0;generated=0
    for key,row in m['images'].items():
        path=PurePosixPath(row['file']);check(not path.is_absolute() and '..' not in path.parts and ':' not in row['file'] and '\\' not in row['file'],'Portable exact-relative atlas')
        check((OUT/row['file']).name in {p.name for p in OUT.iterdir()},'Exact file case')
        check(hashlib.sha256((OUT/row['file']).read_bytes()).hexdigest()==row['sha256'],'Atlas hash')
        im=Image.open(OUT/row['file']).convert('RGBA');check(list(im.size)==[row['width'],row['height']],'Atlas dimensions');images[key]=im
        if row['origin']=='current-headless-gdi':check(all(value%17==0 for value in im.tobytes()),'Native4bit channel precision')
    for key,font in m['fonts'].items():
        source=original['fonts'][font['sourceCache']['key']] if font['sourceCache'] else None
        if source:check(font['kerning']=={str(a)+':'+str(b):amount for a,b,amount in source['kerning']},'Complete source kerning remains authoritative')
        areas={}
        for cp,glyph in font['glyphs'].items():
            im=images[glyph['file']];check(glyph['x']>=0 and glyph['y']>=0 and glyph['x']+glyph['width']<=im.width and glyph['y']+glyph['height']<=im.height,'Glyph contained in declared atlas')
            check(glyph['height']==font['lineHeight'],'True native line height')
            if glyph['origin']=='shipped-cache':
                cell=source['glyphs'][cp];check([glyph['file'],glyph['x'],glyph['y'],glyph['width'],glyph['height']]==[source['file'],cell['x'],cell['y'],cell['width'],source['headerObjectOffsets']['0x2c']],'Original glyph image/UV/dimensions preserved');gold+=1
            else:
                check(glyph['origin']=='current-headless-gdi' and glyph['nativeStoredWidth']==glyph['width'],'Supplement has independent provenance/metric');generated+=1
                for y in range(glyph['y'],glyph['y']+glyph['height']):
                    interval=(glyph['x'],glyph['x']+glyph['width'])
                    check(all(interval[1]<=left or right<=interval[0] for left,right in areas.setdefault(y,[])),'Supplemental atlas glyphs never overlap');areas[y].append(interval)
        check(all(str(ord(cp)) in font['glyphs'] for cp in 'ÄÖÜäöüß'),'Complete German glyph coverage')
    for name,rows in m['bindings'].items():check(len(rows)==6 and all(key in m['fonts'] for key in rows),'Every source resolution tier bound')
    for name,style in m['fontStyles'].items():check(style.get('FONT') in m['bindings'],'Every original fontstyle resolves to declared size variants')
    captures=json.loads((HERE/'bitmap-draw-captures.json').read_text(encoding='utf8'));sheet=Image.new('RGBA',(784,86*len(captures)),(29,35,27,255));rasterPixels=0
    for i,row in enumerate(captures):
        target=Image.new('RGBA',(row['width'],row['height']),(29,35,27,255))
        for draw in row['draws']:
            sx,sy,sw,sh,dx,dy,dw,dh=draw['args'];check([sw,sh]==[dw,dh],'Production draw preserves native pixel dimensions')
            glyph=images[draw['file']].crop((sx,sy,sx+sw,sy+sh));raw=glyph.tobytes();tinted=bytearray(raw);color=draw['color']
            for at in range(0,len(raw),4):
                for c in range(3):tinted[at+c]=math.floor(raw[at+c]*color[c]/255+.5)
                tinted[at+3]=math.floor(raw[at+3]*draw.get('alpha',1)+.5)
            glyph=Image.frombytes('RGBA',glyph.size,bytes(tinted));target.alpha_composite(glyph,(dx,dy));rasterPixels+=sum(alpha!=0 for alpha in glyph.getchannel('A').getdata())
        sheet.alpha_composite(target,(0,i*86))
    preview=HERE/'bitmap-fonts-preview.png';sheet.convert('RGB').save(preview)
    report={'format':'native6-bitmap-font-original-preservation-and-images-v1','passed':True,'checks':checks,'originalGlyphBindingsUnchanged':gold,'currentGdiGlyphBindings':generated,'fontVariants':len(m['fonts']),'sourceResolutionBindings':sum(len(rows) for rows in m['bindings'].values()),'productionDrawCaptures':len(captures),'visibleGlyphPixels':rasterPixels,'preview':preview.name,'previewSha256':hashlib.sha256(preview.read_bytes()).hexdigest(),'manifestSha256':hashlib.sha256((OUT/'bitmap-manifest.json').read_bytes()).hexdigest(),'scope':'Every original glyph binding and full cached kerning table preserved; portable image hashes/UV/packing/German coverage; actual activeJS draw placements composed with original and separately provenanced GDI glyphPNGs. Composition preview is not a browser/original executable capture.'}
    (PROJECT.parent/'EE2-Research/analysis/native6-font-images-tests.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report,indent=2))

if __name__=='__main__':main()
