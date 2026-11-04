"""Independent installed-font byte/pixel oracle and malformed-cache checks."""
from pathlib import Path
import hashlib,json,struct,zipfile
from PIL import Image
import sys
PROJECT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(PROJECT/'tools/ee2_fonts'))
from font_cache import parse_info,INSTALL,OUT

checks=0
def check(ok,label):
    global checks
    if not ok:raise AssertionError(label)
    checks+=1

def main():
    manifest=json.loads((OUT/'manifest.json').read_text(encoding='utf8'));pixels=0;glyphs=0;kerns=0;sourcebytes=0
    with zipfile.ZipFile(INSTALL/'zips/fonts.zip') as archive:
        for key,row in manifest['fonts'].items():
            raw=archive.read(row['source']['info']);check(hashlib.sha256(raw).hexdigest()==row['source']['infoSha256'],'Source info digest '+key)
            # Independently derive serialized offsets using fixed native record
            # widths, rather than reusing the importer's variable reader.
            count=struct.unpack_from('<I',raw,49)[0];end=53+count*12
            glyphTable=[struct.unpack_from('<IHHHH',raw,53+12*i) for i in range(count)]
            check({cp for cp,x,y,w,r in glyphTable}==set(map(int,row['glyphs'])),'All cache character codes '+key)
            for cp,x,y,w,r in glyphTable:check(row['glyphs'][str(cp)]=={'x':x,'y':y,'width':w,'reserved':r},'Exact serialized UV/width '+key)
            kerncount=struct.unpack_from('<I',raw,end)[0]
            check([list(struct.unpack_from('<HHi',raw,end+4+i*8)) for i in range(kerncount)]==row['kerning'],'Signed pair records '+key)
            tex=archive.read(row['source']['texture']);image=Image.open(OUT/row['file']).convert('RGBA');expected=bytearray(image.width*image.height*4)
            # Every font uses 16bit 0xARGB (4 bits/channel); the image source's
            # pixel payload immediately precedes the single-object root list.
            bytecount=image.width*image.height*2;payload=tex[-8-bytecount:-8]
            check(row['pixelData']['masks']==[0xf00,0xf0,0xf,0xf000] and row['pixelData']['bpp']==16,'Declared native A4R4G4B4 '+key)
            for i,(pixel,) in enumerate(struct.iter_unpack('<H',payload)):
                expected[i*4:i*4+4]=bytes(((pixel>>8&15)*17,(pixel>>4&15)*17,(pixel&15)*17,(pixel>>12&15)*17))
            check(image.tobytes()==expected,'Every original RGBA pixel '+key);pixels+=image.width*image.height;glyphs+=count;kerns+=kerncount;sourcebytes+=len(raw)+len(tex)
        sample=archive.read('fontcache/fv8W_Palatino_Linotype_14_0_aa_kr_alnum_2_128x128_info.cache')
    rejected=0
    malformed=[sample[:n] for n in range(len(sample))]+[sample+b'\0']
    for offset,value in [(0,9),(49,0xffffffff),(53,0x110000),(57,0xffffffff)]:
        bad=bytearray(sample);struct.pack_into('<I',bad,offset,value);malformed.append(bytes(bad))
    for data in malformed:
        try:parse_info(data)
        except (ValueError,struct.error):rejected+=1
        else:raise AssertionError('Malformed cache accepted')
    check(rejected==len(malformed),'Every partial/invalid cache rejected')
    report={'format':'native6-font-cache-independent-tests-v1','passed':True,'checks':checks,'fontPairs':len(manifest['fonts']),'originalRgbaPixels':pixels,'originalGlyphRecords':glyphs,'originalKerningRecords':kerns,'sourceBytes':sourcebytes,'malformedRejected':rejected,'manifestSha256':hashlib.sha256((OUT/'manifest.json').read_bytes()).hexdigest(),'testSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'scope':'Independent fixed-offset native glyph/kerning record oracle; every original16bit atlas pixel versus exportedPNG; strict full-cache/truncation handling. No active runtime change.'}
    (PROJECT.parent/'EE2-Research/analysis/native6-font-cache-tests.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report,indent=2))

if __name__=='__main__':main()
