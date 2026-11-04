"""Source-derived headless GDI glyph raster. Does not copy any font file.

Original cached pixels remain authoritative. This helper only produces a
separately identified current-GDI candidate; native4/5 runtime is untouched.
"""
import ctypes as C
from ctypes import wintypes as W
from PIL import Image,ImageFilter
from gdi_api import (g,bind,handle,uint,integer,BIH,CreateCompatibleDC,DeleteDC,
    CreateFontA,SelectObject,DeleteObject,CreateDIBitmap,ExtTextOutW,
    GetCharWidthW,GetTextFaceW,GdiFlush,PatBlt,SetTextColor,SetBkColor,SetBkMode)

GetDIBits=bind('GetDIBits',integer,[handle,handle,uint,uint,handle,handle,uint])
GetTextMetricsA=bind('GetTextMetricsA',integer,[handle,handle])
GetKerningPairsA=bind('GetKerningPairsA',uint,[handle,uint,handle])

class TM(C.Structure):
    _fields_=[(name,W.LONG) for name in ('height','ascent','descent','internalLeading','externalLeading','averageWidth','maximumWidth','weight','overhang','digitizedAspectX','digitizedAspectY')]+[(name,W.BYTE) for name in ('firstChar','lastChar','defaultChar','breakChar','italic','underlined','struckOut','pitchAndFamily','charSet')]
class KP(C.Structure):
    _fields_=[('first',W.WORD),('second',W.WORD),('amount',integer)]

SMALL_CAP_RANGES=((0x61,0x7a,1,32),(0xe0,0xf6,1,32),(0xf8,0xff,1,32),
    (0x101,0x137,2,1),(0x13a,0x148,2,1),(0x14b,0x177,2,1),(0x17a,0x17e,2,1),
    (0x1f5,0x1f5,2,1),(0x1fb,0x1ff,2,1),(0x201,0x217,2,1),(0x219,0x21b,2,1),
    (0x1e01,0x1e95,2,1),(0x1ea1,0x1ef9,2,1))
SMALL_CAP_MAP={cp:cp-offset for start,end,step,offset in SMALL_CAP_RANGES for cp in range(start,end+1,step)}

class Raster:
    def __init__(self,face,size,small,flags):
        self.face=face;self.size=size;self.small=small;self.flags=flags;self.handles={};self.dc=CreateCompatibleDC(None)
        if not self.dc:raise C.WinError(C.get_last_error())
        self.originalBitmap=None;self.originalFont=None;self.bitmap=None
        try:
            info=BIH(C.sizeof(BIH),256,256,1,24,0,0x30000,0,0,0,0)
            self.bitmap=CreateDIBitmap(self.dc,C.byref(info),0,None,None,0)
            if not self.bitmap:raise C.WinError(C.get_last_error())
            self.originalBitmap=SelectObject(self.dc,self.bitmap)
            SetTextColor(self.dc,0);SetBkColor(self.dc,0xffffff);SetBkMode(self.dc,2)
            self.metrics=self.get_metrics(size)
            self.smallMetrics=self.get_metrics(small) if flags&16 else None
            self.baseline=self.metrics.ascent-self.smallMetrics.ascent if self.smallMetrics else 0
            self.lineHeight=size+(2 if flags&8 else 0)
        except BaseException:self.close();raise
    def font(self,size):
        if size not in self.handles:
            value=CreateFontA(size,0,0,0,700 if self.flags&1 else 400,int(bool(self.flags&2)),int(bool(self.flags&4)),0,0,0,0,2,0,self.face.encode('ascii'))
            if not value:raise C.WinError(C.get_last_error())
            self.handles[size]=value
        old=SelectObject(self.dc,self.handles[size])
        if self.originalFont is None:self.originalFont=old
        face=C.create_unicode_buffer(128);GetTextFaceW(self.dc,128,face)
        if face.value!=self.face:raise ValueError('Source face unavailable: '+self.face+' resolved to '+face.value)
    def get_metrics(self,size):
        self.font(size);metrics=TM()
        if not GetTextMetricsA(self.dc,C.byref(metrics)):raise C.WinError(C.get_last_error())
        return metrics
    def pairs(self,size):
        self.font(size);rows=(KP*4096)();count=GetKerningPairsA(self.dc,4096,rows)
        return {(row.first,row.second):row.amount for row in rows[:count]}
    def kerning(self):
        if not self.flags&64:return {}
        result=self.pairs(self.size)
        if self.flags&16:
            inverse={upper:lower for lower,upper in SMALL_CAP_MAP.items()}
            for (left,right),amount in self.pairs(self.small).items():
                if left in inverse and right in inverse:result[inverse[left],inverse[right]]=amount
        return {str(left)+':'+str(right):amount for (left,right),amount in result.items()}
    def raster(self,cp):
        if not (0x20<=cp<0x530):raise ValueError('NonWestern alternate face is a separate source path')
        scale=4 if self.flags&32 else 1;outline=bool(self.flags&8)
        rendered=SMALL_CAP_MAP.get(cp,cp) if self.flags&16 else cp
        small=rendered!=cp;baseline=self.baseline if small else 0
        self.font((self.small if small else self.size)*scale)
        measured=integer()
        if not GetCharWidthW(self.dc,rendered,rendered,C.byref(measured)):raise C.WinError(C.get_last_error())
        natural=(measured.value+1)//4 if scale==4 else measured.value
        if natural<=0:return None
        width=natural+(2 if outline else 1);drawWidth=natural if outline else width
        drawHeight=self.size-baseline;widthHi=(drawWidth+(2 if outline else 0))*scale;heightHi=(drawHeight+(2 if outline else 0))*scale
        if max(widthHi,heightHi)>256:raise ValueError('Native256px scratch surface exceeded')
        PatBlt(self.dc,0,0,256,256,0x00ff0062)
        if not ExtTextOutW(self.dc,scale if outline else 0,scale if outline else 0,2,None,chr(rendered),1,None):raise C.WinError(C.get_last_error())
        GdiFlush()
        # Read the same monochrome DDB in bulk, rather than making thousands
        # of Python/GetPixel calls. The independent probe retains GetPixel.
        info=BIH(C.sizeof(BIH),256,-256,1,1,0,8192,0,0,2,0);bi=C.create_string_buffer(bytes(info)+bytes.fromhex('00000000ffffff00'));pixels=C.create_string_buffer(8192)
        SelectObject(self.dc,self.originalBitmap)
        try:lines=GetDIBits(self.dc,self.bitmap,0,256,pixels,bi,0)
        finally:SelectObject(self.dc,self.bitmap)
        if lines!=256:raise C.WinError(C.get_last_error())
        bits=Image.frombytes('1',(256,256),pixels.raw).convert('L').crop((0,0,widthHi,heightHi))
        # 255 means original ink; white background becomes0.
        ink=bits.point(lambda value:255-value)
        union=ink.filter(ImageFilter.MaxFilter(7 if scale==4 else 3)) if outline else ink
        a=ink.tobytes();u=union.tobytes();output=bytearray(width*self.lineHeight*4)
        for y in range(drawHeight+(2 if outline else 0)):
            for x in range(width):
                covered=0;body=0
                for dy in range(scale):
                    at=(y*scale+dy)*widthHi+x*scale
                    for dx in range(scale):covered+=bool(u[at+dx]);body+=bool(a[at+dx])
                if not covered:continue
                alpha=(min(covered*16,255)//16*17) if scale==4 else 255
                gray=(min(body*256//covered,255)//16*17) if scale==4 and outline else 255 if body else 0
                at=((y+baseline)*width+x)*4;output[at:at+4]=bytes((gray,gray,gray,alpha))
        return Image.frombytes('RGBA',(width,self.lineHeight),bytes(output)),{'renderedCodepoint':rendered,'currentGdiMeasuredWidth':measured.value,'baselineOffset':baseline,'nativeStoredWidth':width}
    def close(self):
        if self.originalFont:SelectObject(self.dc,self.originalFont)
        for value in self.handles.values():DeleteObject(value)
        if self.originalBitmap:SelectObject(self.dc,self.originalBitmap)
        if self.bitmap:DeleteObject(self.bitmap)
        if self.dc:DeleteDC(self.dc)
        self.dc=None
    def __enter__(self):return self
    def __exit__(self,*args):self.close()
