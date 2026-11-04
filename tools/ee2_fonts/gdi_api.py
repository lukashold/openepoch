"""Headless memory-DC probe of the source-proved GDI/4x AA glyph path.

No original EXE, window, font installation, screen capture, or TTF packaging.
This is an isolated comparison experiment, not an active glyph importer.
"""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import hashlib,json
from PIL import Image

from font_cache import HERE,OUT,REPORT
ASSETS=OUT
g=C.WinDLL('gdi32',use_last_error=True)
def bind(name,restype,args):
    f=getattr(g,name);f.restype=restype;f.argtypes=args;return f
handle=C.c_void_p;uint=C.c_uint;integer=C.c_int
CreateCompatibleDC=bind('CreateCompatibleDC',handle,[handle]);DeleteDC=bind('DeleteDC',integer,[handle])
CreateFontA=bind('CreateFontA',handle,[integer]*5+[uint]*8+[C.c_char_p])
SelectObject=bind('SelectObject',handle,[handle,handle]);DeleteObject=bind('DeleteObject',integer,[handle])
CreateDIBSection=bind('CreateDIBSection',handle,[handle,handle,uint,C.POINTER(handle),handle,uint])
ExtTextOutW=bind('ExtTextOutW',integer,[handle,integer,integer,uint,handle,C.c_wchar_p,uint,handle])
GetCharWidthW=bind('GetCharWidthW',integer,[handle,uint,uint,C.POINTER(integer)])
GetTextFaceW=bind('GetTextFaceW',integer,[handle,integer,C.c_wchar_p]);GdiFlush=bind('GdiFlush',integer,[])
SetTextColor=bind('SetTextColor',uint,[handle,uint]);SetBkColor=bind('SetBkColor',uint,[handle,uint])
SetBkMode=bind('SetBkMode',integer,[handle,integer]);SetTextAlign=bind('SetTextAlign',uint,[handle,uint])
CreateDIBitmap=bind('CreateDIBitmap',handle,[handle,handle,uint,handle,handle,uint]);GetPixel=bind('GetPixel',uint,[handle,integer,integer])
PatBlt=bind('PatBlt',integer,[handle,integer,integer,integer,integer,uint])

class BIH(C.Structure):
    _fields_=[('size',W.DWORD),('width',W.LONG),('height',W.LONG),('planes',W.WORD),('bits',W.WORD),('compression',W.DWORD),('imageBytes',W.DWORD),('xppm',W.LONG),('yppm',W.LONG),('used',W.DWORD),('important',W.DWORD)]

def probe(key,monochrome=False,nativeDc=False):
    manifest=json.loads((ASSETS/'manifest.json').read_text(encoding='utf8'));font=manifest['fonts'][key];header=font['headerObjectOffsets'];flags=header['0x4c']
    if flags&0x18:raise ValueError('Small caps/outline are separate native branches')
    nominal=header['0x30'];scale=4 if flags&32 else 1
    family='Palatino Linotype' if 'Palatino_Linotype' in key else 'Arial'
    dc=CreateCompatibleDC(None)
    if not dc:raise C.WinError(C.get_last_error())
    bitmap=None;hfont=None;oldBitmap=None;oldFont=None
    try:
        size=256 if nativeDc else 512;stride=size//8 if monochrome else size*4
        bi=BIH(C.sizeof(BIH),size,-size,1,1 if monochrome else 32,0,stride*size,0,0,2 if monochrome else 0,0);bits=handle()
        info=C.create_string_buffer(bytes(bi)+(bytes.fromhex('00000000ffffff00') if monochrome else b''))
        if nativeDc:
            # Source00ABED10: DDB from the default compatible memory DC and a
            # 256x256,24bit BI_RGB descriptor, no initial bits/color table.
            nativeBi=BIH(C.sizeof(BIH),256,256,1,24,0,0x30000,0,0,0,0)
            bitmap=CreateDIBitmap(dc,C.byref(nativeBi),0,None,None,0)
        else:bitmap=CreateDIBSection(dc,info,0,C.byref(bits),None,0)
        if not bitmap:raise C.WinError(C.get_last_error())
        oldBitmap=SelectObject(dc,bitmap)
        # Native00ABF6D0: positive cell height, width/angles0, weight400/700,
        # italic/underline from flags, strike0, quality2, pitch/family0.
        hfont=CreateFontA(nominal*scale,0,0,0,700 if flags&1 else 400,int(bool(flags&2)),int(bool(flags&4)),0,0,0,0,2,0,family.encode('ascii'))
        if not hfont:raise C.WinError(C.get_last_error())
        oldFont=SelectObject(dc,hfont);face=C.create_unicode_buffer(128);GetTextFaceW(dc,128,face)
        if face.value!=family:raise ValueError('GDI substituted font: '+face.value)
        SetTextColor(dc,0);SetBkColor(dc,0xffffff);SetBkMode(dc,2);SetTextAlign(dc,0)
        source=Image.open(ASSETS/font['file']).convert('RGBA');reference=source.tobytes();result=bytearray(len(reference));metrics=[];differentPixels=0;glyphMismatches=[]
        for cp,record in font['glyphs'].items():
            cp=int(cp);width=record['width'];height=header['0x2c'];advance=integer()
            if not GetCharWidthW(dc,cp,cp,C.byref(advance)):raise C.WinError(C.get_last_error())
            # Source constructor width downsample rounds (width+1)/4 toward0.
            expectedWidth=((advance.value+1)//4 if scale==4 else advance.value)+1
            metrics.append({'cp':cp,'nativeWidth':width,'gdiWidth':expectedWidth,'equal':width==expectedWidth})
            GdiFlush()
            if nativeDc:PatBlt(dc,0,0,size,size,0x00ff0062)
            else:C.memset(bits,255,stride*size)
            if not ExtTextOutW(dc,0,0,2,None,chr(cp),1,None):raise C.WinError(C.get_last_error())
            GdiFlush();data=b'' if nativeDc else C.string_at(bits,stride*size);glyphDiff=0
            for y in range(height):
                for x in range(width):
                    def black(px,py):
                        if nativeDc:return GetPixel(dc,px,py)!=0xffffff
                        if monochrome:return not (data[py*stride+px//8]>>(7-px%8)&1)
                        at=(py*size+px)*4;return data[at:at+3]!=b'\xff\xff\xff'
                    coverage=sum(black(x*scale+dx,y*scale+dy) for dy in range(scale) for dx in range(scale))
                    if scale==4:
                        # Source00ABEE80 uses coverage*(1/16)*256, clamps255,
                        # masksF0 and stores alpha in A4R4G4B4, RGB=FFF.
                        alpha=min(coverage*16,255)//16*17
                    else:alpha=255 if coverage else 0
                    rgba=bytes((255,255,255,alpha)) if coverage else bytes(4)
                    at=((record['y']+y)*source.width+record['x']+x)*4;result[at:at+4]=rgba
                    if reference[at:at+4]!=rgba:glyphDiff+=1
            if glyphDiff:glyphMismatches.append({'cp':cp,'char':chr(cp),'pixels':glyphDiff})
            differentPixels+=glyphDiff
        target=ASSETS/(key+('-native' if nativeDc else '-mono' if monochrome else '')+'-gdi-probe.png');Image.frombytes('RGBA',source.size,bytes(result)).save(target)
        return {'font':key,'probeDcBpp':'native-DDB' if nativeDc else 1 if monochrome else 32,'face':face.value,'glyphs':len(metrics),'metricEqual':sum(row['equal'] for row in metrics),'metricDifferences':[row for row in metrics if not row['equal']],'differentGlyphs':len(glyphMismatches),'differentPixels':differentPixels,'glyphDifferences':glyphMismatches,'sourceRgbaSha256':hashlib.sha256(reference).hexdigest(),'probeRgbaSha256':hashlib.sha256(result).hexdigest(),'sourceExactPixels':differentPixels==0,'limits':['Current installed GDI font raster may differ from historical cache raster; equality is measured, never assumed.','Probe initializes background via memset/WHITENESS rather than the source FillRect call; text/bitmap/quality/AA/ExtTextOut operations are source-proved.']}
    finally:
        if oldFont:SelectObject(dc,oldFont)
        if oldBitmap:SelectObject(dc,oldBitmap)
        if hfont:DeleteObject(hfont)
        if bitmap:DeleteObject(bitmap)
        DeleteDC(dc)

if __name__=='__main__':
    keys=('fv8W_Palatino_Linotype_14_0_aa_kr_alnum_2_128x128','fv8W_Arial_12_0_kr_ascii_0_64x64')
    rows=[probe(key,mono) for mono in (False,True) for key in keys]+[probe(key,nativeDc=True) for key in keys]
    report={'format':'native6-font-gdi-probe-v1','status':'measured','probeSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'scope':'Memory-only GDI drawing with original proved CreateFont/ExtTextOut/4x downsample; independently compared with original atlas pixels. No UI or original process.'}
    (REPORT/'native6-font-gdi-probe.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(json.dumps([{k:r[k] for k in ('font','probeDcBpp','glyphs','metricEqual','differentGlyphs','differentPixels','sourceExactPixels')} for r in rows],indent=2))
