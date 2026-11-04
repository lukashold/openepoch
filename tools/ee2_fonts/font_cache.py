"""Isolated source cache inventory. No active runtime or original file changes.

Cache layout follows native00A9AFD0 (read)/00A9B4E0 (write). Header field
semantics are retained as raw object offsets until the GDI constructor is proved.
"""
from pathlib import Path
import hashlib,json,re,struct,zipfile,os
from PIL import Image

HERE=Path(__file__).resolve().parent
PROJECT=HERE.parents[1]
WORKSPACE=PROJECT.parents[1]
REPORT=Path(os.environ.get('OPEN_EPOCH_FONT_REPORTS',PROJECT.parent/'EE2-Research/analysis'))
REPORT.mkdir(parents=True,exist_ok=True)
INSTALL=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
OUT=Path(os.environ.get('OPEN_EPOCH_FONT_OUT',PROJECT/'docs/local-original/fonts-native'))
sha=lambda data:hashlib.sha256(data).hexdigest()

def parse_info(data):
    pos=0
    def take(fmt):
        nonlocal pos
        size=struct.calcsize('<'+fmt)
        if pos+size>len(data):raise ValueError('Truncated font information')
        row=struct.unpack_from('<'+fmt,data,pos);pos+=size
        return row[0] if len(row)==1 else list(row)
    version=take('I')
    if version!=8:raise ValueError('Unsupported cache version')
    header=take('11i');asian=take('B');count=take('I')
    if count>65536:raise ValueError('Unreasonable glyph count')
    glyphs={}
    for _ in range(count):
        cp,x,y,width,reserved=take('Ihhhh')
        if cp in glyphs or cp>0x10ffff or min(x,y,width)<0 or reserved!=0:raise ValueError('Invalid glyph record')
        glyphs[cp]={'x':x,'y':y,'width':width,'reserved':reserved}
    count=take('I')
    if count>65536:raise ValueError('Unreasonable kerning count')
    kerning=[]
    for _ in range(count):
        left,right,amount=take('HHi');kerning.append([left,right,amount])
    count=take('I')
    if count>65536:raise ValueError('Unreasonable shelf count')
    shelves=[]
    for _ in range(count):
        y,used,members=take('III')
        if members>65536:raise ValueError('Unreasonable shelf membership')
        cells=[take('II') for _ in range(members)]
        shelves.append({'y':y,'used':used,'cells':cells})
    if pos!=len(data):raise ValueError('Unconsumed cache bytes')
    cells=[(cp,x,shelf['y']) for shelf in shelves for cp,x in shelf['cells']]
    if len(cells)!=len(glyphs) or {cp for cp,x,y in cells}!=set(glyphs):raise ValueError('Shelf/glyph set mismatch')
    for cp,x,y in cells:
        if [glyphs[cp]['x'],glyphs[cp]['y']]!=[x,y]:raise ValueError('Shelf coordinate mismatch')
    return {'version':version,'headerObjectOffsets':{hex(0x2c+i*4):value for i,value in enumerate(header)},'asianByte':asian,'glyphs':glyphs,'kerning':kerning,'shelves':shelves}

def decode_font_texture(data):
    # Read the original NiPixelData channel masks, including the font-specific
    # 16-bit A4R4G4B4 format that the active general-media importer does not use.
    signature=data[:data.find(b'\n')+1]
    if signature not in (b'Gamebryo File Format, Version 10.1.0.0\n',b'Gamebryo File Format, Version 10.2.0.0\n'):raise ValueError('Unsupported texture signature')
    pos=len(signature)
    def take(fmt):
        nonlocal pos
        size=struct.calcsize('<'+fmt)
        if pos+size>len(data):raise ValueError('Truncated pixel record')
        row=struct.unpack_from('<'+fmt,data,pos);pos+=size
        return row[0] if len(row)==1 else row
    version,user,blocks,types=take('IIIH')
    if version not in (0x0a010000,0x0a020000) or (user,blocks,types)!=(0,1,1):raise ValueError('Unexpected Gamebryo object header')
    length=take('I');name=data[pos:pos+length];pos+=length
    if name!=b'NiPixelData' or take('H')!=0 or take('I')!=0 or take('I')!=0:raise ValueError('Expected ungrouped NiPixelData')
    fmt,red,green,blue,alpha,bpp=take('6I');fast=data[pos:pos+8];pos+=8
    tiling,palette,mips,bytespp=take('IiII')
    if fmt not in (0,1) or bpp not in (16,32) or tiling or palette!=-1 or mips!=1 or bytespp!=bpp//8:raise ValueError('Unsupported font pixel format')
    width,height,start=take('III');count=take('I');payload=data[pos:pos+count];pos+=count
    if take('II')!=(1,0) or pos!=len(data) or start!=0 or count!=width*height*bytespp:raise ValueError('Invalid font pixel payload/root list')
    channels=[]
    for mask in (red,green,blue,alpha):
        if not mask:channels.append((0,0));continue
        shift=(mask&-mask).bit_length()-1;maximum=mask>>shift
        if maximum&(maximum+1):raise ValueError('Noncontiguous color mask')
        channels.append((shift,maximum))
    rgba=bytearray(width*height*4)
    for i in range(width*height):
        pixel=int.from_bytes(payload[i*bytespp:(i+1)*bytespp],'little')
        for c,(shift,maximum) in enumerate(channels):rgba[i*4+c]=((pixel>>shift)&maximum)*255//maximum if maximum else (255 if c==3 else 0)
    return Image.frombytes('RGBA',(width,height),bytes(rgba)),{'nifVersion':version,'format':fmt,'bpp':bpp,'masks':[red,green,blue,alpha],'fastCompareBytes':fast.hex(),'sourcePayloadSha256':sha(payload),'rgbaSha256':sha(rgba)}

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    if 'local-original' not in OUT.parts:raise ValueError('Original images must stay in local-original')
    fonts={};summaries=[]
    with zipfile.ZipFile(INSTALL/'zips/fonts.zip') as archive:
        for member in archive.namelist():
            if not member.endswith('_info.cache'):continue
            raw=archive.read(member);info=parse_info(raw)
            texture=member.replace('_info.cache','_texture.texcache');texraw=archive.read(texture)
            image,meta=decode_font_texture(texraw);key=Path(member).name.removesuffix('_info.cache');file=key+'.png';image.save(OUT/file)
            for row in info['glyphs'].values():
                if row['x']+row['width']>image.width or row['y']+info['headerObjectOffsets']['0x2c']>image.height:raise ValueError('Glyph outside atlas')
            fonts[key]={**info,'file':file,'width':image.width,'height':image.height,'pngSha256':sha((OUT/file).read_bytes()),'source':{'archive':'zips/fonts.zip','info':member,'infoSha256':sha(raw),'texture':texture,'textureSha256':sha(texraw)},'pixelData':meta}
            chars=set(info['glyphs']);summaries.append({'key':key,'glyphCount':len(chars),'minCodepoint':min(chars),'maxCodepoint':max(chars),'umlauts':{c:ord(c) in chars for c in 'ÄÖÜäöüß'},'punctuation':{c:ord(c) in chars for c in '.,:;!?-()[]/\\%&@€„“–'},'kerningPairs':len(info['kerning']),'atlas':[image.width,image.height]})
    manifest={'format':'ee2-original-font-cache-v1','fonts':fonts,'limits':['Original source-cache export. The runtime preserves shipped glyphs; supplemental current-GDI glyphs have separate provenance.','Header semantics beyond serialized offsets remain under native constructor review.','Missing glyphs are not fabricated; a source-proved dynamic raster import is separate.']}
    (OUT/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    expansion=[]
    with zipfile.ZipFile(INSTALL/'zips_ee2x/EE2X_fonts.zip') as archive:
        for member in archive.namelist():
            if not member.endswith('_info.cache'):continue
            raw=archive.read(member);info=parse_info(raw);texture=member.replace('_info.cache','_texture.texcache');image,meta=decode_font_texture(archive.read(texture));key=Path(member).name.removesuffix('_info.cache')
            expansion.append({'member':member,'infoSha256':sha(raw),'glyphCount':len(info['glyphs']),'codepoints':sorted(info['glyphs']),'sameInfoAsEE2':sha(raw)==fonts[key]['source']['infoSha256'],'sameAtlasPixelsAsEE2':meta['rgbaSha256']==fonts[key]['pixelData']['rgbaSha256']})
    report={'format':'native6-font-cache-inventory-v1','status':'passed','entries':len(fonts)*2,'fonts':len(fonts),'western':sum(key.startswith('fv8W') for key in fonts),'asian':sum(key.startswith('fv8A') for key in fonts),'summary':summaries,'expansion':{'archive':'zips_ee2x/EE2X_fonts.zip','entries':len(expansion)*2,'fonts':expansion},'manifestSha256':sha((OUT/'manifest.json').read_bytes()),'importerSha256':sha(Path(__file__).read_bytes()),'allInfoBytesConsumed':True,'allGlyphShelfCoordinatesAgree':True,'scope':'All installed fonts.zip and EE2X_fonts.zip info/cache pairs, original mask-based texture decode, exact source hashes. No OS font or original process started.'}
    (REPORT/'native6-font-cache-inventory.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    print(json.dumps({k:v for k,v in report.items() if k not in ('summary','expansion')},indent=2))

if __name__=='__main__':main()
