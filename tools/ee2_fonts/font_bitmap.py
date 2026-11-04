"""Build isolated portable bitmap-font candidate with per-glyph provenance."""
from pathlib import Path
import hashlib,json,math,sys,zipfile,os
from PIL import Image
from font_cache import HERE,OUT,INSTALL,REPORT,PROJECT,WORKSPACE
from font_raster import Raster

sha=lambda data:hashlib.sha256(data).hexdigest()

def main():
    cache=json.loads((OUT/'manifest.json').read_text(encoding='utf8'));table=json.loads((REPORT/'native6-font-table-cache-bindings.json').read_text(encoding='utf8'));bindings={};fonts={};signatures={};images={}
    root=WORKSPACE;resolution=json.loads(Path(os.environ.get('OPEN_EPOCH_FONT_RESOLVED',root/'work/up-resolved-sources/effective-db-sources.json')).read_text());styleSource=resolution['effectiveByPath']['db/ui/dbfontstyle.csv']['source']
    sys.path.insert(0,str(root/'outputs/OpenEpoch/tools'))
    from extract_ee2_hud import rows
    with zipfile.ZipFile(INSTALL/styleSource['physical']) as archive:styleRaw=archive.read(styleSource['member'])
    assert sha(styleRaw)==styleSource['sha256'];fontStyles=rows(styleRaw)
    points=[*range(32,127),*range(160,256)]
    for request in table['requests']:
        signature=(request['faces'][0],request['size'],request['smallSize'],request['flags'],request['initSet'],*request['minAtlas'])
        if signature in signatures:key=signatures[signature]
        else:
            key='font'+str(len(fonts));signatures[signature]=key;sourcekey=next(iter(request['exactShippedCacheMatches']),None);source=cache['fonts'][sourcekey] if sourcekey else None
            glyphs={};originals=0;supplements=[]
            with Raster(signature[0],request['size'],request['smallSize'],request['flags']) as raster:
                if source:
                    images[source['file']]={'file':source['file'],'sha256':source['pngSha256'],'width':source['width'],'height':source['height'],'origin':'shipped-cache'}
                    for cp,cell in source['glyphs'].items():
                        glyphs[cp]={'file':source['file'],'x':cell['x'],'y':cell['y'],'width':cell['width'],'height':source['headerObjectOffsets']['0x2c'],'origin':'shipped-cache'};originals+=1
                    kerning={str(a)+':'+str(b):amount for a,b,amount in source['kerning']}
                else:kerning=raster.kerning()
                for cp in points:
                    if str(cp) in glyphs:continue
                    value=raster.raster(cp)
                    if value is None:continue
                    image,metadata=value;supplements.append((cp,image,metadata))
                file=key+'-current-gdi.png';positions={};x=y=0;rowHeight=raster.lineHeight;atlasWidth=512
                for cp,image,metadata in supplements:
                    if x+image.width>atlasWidth:x=0;y+=rowHeight
                    positions[cp]=(x,y);x+=image.width
                atlasHeight=2**math.ceil(math.log2(max(1,y+rowHeight)));atlas=Image.new('RGBA',(atlasWidth,atlasHeight))
                for cp,image,metadata in supplements:
                    x,y=positions[cp];atlas.paste(image,(x,y));glyphs[str(cp)]={'file':file,'x':x,'y':y,'width':image.width,'height':image.height,'origin':'current-headless-gdi',**metadata}
                atlas.save(OUT/file);images[file]={'file':file,'sha256':sha((OUT/file).read_bytes()),'width':atlasWidth,'height':atlasHeight,'origin':'current-headless-gdi'}
                spacing=source['headerObjectOffsets']['0x48'] if source else -1 if request['flags']&8 else 0
                fonts[key]={'face':signature[0],'nominalSize':request['size'],'smallSize':request['smallSize'],'flags':request['flags'],'lineHeight':raster.lineHeight,'spacing':spacing,'ascent':raster.metrics.ascent,'baselineOffset':raster.baseline,'glyphs':glyphs,'kerning':kerning,'sourceCache':{'key':sourcekey,**source['source']} if source else None,'gdiSource':{'kind':'current-headless-gdi','createFontQuality':2,'charSet':0,'textAPI':'ExtTextOutW','scratch':'native256x256DDB','antiAlias':'native4x4 coverage to4bit alpha','algorithm':['00ABED10','00ABF6D0','00ABF550','00ABEDB0','00ABEE80','00ABF020','00ABF210','00A9AD30'],'fontFileCopied':False},'originalGlyphs':originals,'supplementalGlyphs':len(supplements)}
            print(json.dumps({'font':key,'size':request['size'],'flags':request['flags'],'originalGlyphs':originals,'supplementalGlyphs':len(supplements)}),flush=True)
        bindings.setdefault(request['font'],[None]*6)[request['resolutionTier']]=key
    result={'format':'ee2-native-bitmap-font-v1','fonts':fonts,'bindings':bindings,'images':images,'fontStyles':fontStyles,'resolutionWidths':[800,1366,1600,1920,2048,2560],'sourceFontStyles':styleSource,'sourceFontTable':table['source'],'sourceCacheManifestSha256':sha((OUT/'manifest.json').read_bytes()),'limits':['Original cached glyphs and complete cached kerning tables are unchanged; current-GDI supplements are identified separately.','Supplemental glyphs are current-system headless GDI products of source-derived rules; historical font raster equality is not claimed.','Generated atlas packing and Canvas display are application adapters; native text metrics/kerning/outline spacing are kept separately.','Currently covers printableASCII and Latin1/German; original alternate-face selection for codepoints aboveU+052F remains to be bound.','Cache comparison records4Palatino14 alpha-nibble differences andDEL127, plus the complete70font comparison; no source glyph is replaced by that probe.']}
    target=OUT/'bitmap-manifest.json';target.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    report={'format':'native6-font-bitmap-import-v1','status':'passed','fonts':len(fonts),'bindings':len(bindings),'resolutionBindings':sum(len(row) for row in bindings.values()),'originalGlyphBindings':sum(row['originalGlyphs'] for row in fonts.values()),'currentGdiGlyphBindings':sum(row['supplementalGlyphs'] for row in fonts.values()),'images':len(images),'manifestSha256':sha(target.read_bytes()),'generatorSha256':sha(Path(__file__).read_bytes()),'rasterSha256':sha((HERE/'font_raster.py').read_bytes()),'fontFilesCopied':0,'activeRuntimeModified':False}
    (REPORT/'native6-font-bitmap-import.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report,indent=2))

if __name__=='__main__':main()
