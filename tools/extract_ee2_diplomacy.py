"""Export source UI7 diplomacy/proposal geometry and original sprite crops.

Original installation is read-only. Generated original media stays beneath a
local-original directory. Numerical rules and existing controllers are unchanged.
"""
from pathlib import Path,PureWindowsPath,PurePosixPath
import argparse,base64,csv,hashlib,io,json,sys,zipfile
from PIL import Image
HERE=Path(__file__).resolve().parent;PROJECT=HERE.parent;WORKSPACE=PROJECT.parents[1]
sys.path.insert(0,str(PROJECT/'tools'))
from extract_ee2_hud import rows
from extract_ee2_textures import decode_texcache
INSTALL=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
RESOLVED=WORKSPACE/'work/up-resolved-sources/effective-db-sources.json'
OUT=PROJECT/'docs/local-original/diplomacy-native'
sha=lambda b:hashlib.sha256(b).hexdigest()

def bracket_csv(line):
    # Native font CSV contains comma-separated quoted names inside unquoted
    # bracket lists; an ordinary CSV reader would split this source field.
    fields=[];start=0;depth=0;quoted=False
    for i,char in enumerate(line):
        if char=='"':quoted=not quoted
        if not quoted:
            if char=='[':depth+=1
            elif char==']':depth-=1
            elif char==',' and depth==0:fields.append(line[start:i]);start=i+1
    assert depth==0 and not quoted,line
    fields.append(line[start:]);return fields

def main():
    if 'local-original' not in {part.casefold() for part in OUT.resolve().parts}:raise ValueError('Original media must stay beneath local-original')
    if OUT.resolve().is_relative_to(INSTALL.resolve()):raise ValueError('Original installation is read-only')
    resolution=json.loads(RESOLVED.read_text());source_records={};archives={}
    def read(path):
        record=resolution['effectiveByPath'][path.casefold()];origin=record['source'];physical=INSTALL/origin['physical']
        if origin['kind']=='zip-member':
            if str(physical) not in archives:archives[str(physical)]=zipfile.ZipFile(physical)
            data=archives[str(physical)].read(origin['member'])
        else:data=physical.read_bytes()
        assert sha(data)==origin['sha256'],path
        source_records[path]={'path':record['path'],**origin};return data
    paths=['db/UI/dbwidget_ingame.csv','db/UI/dbwidget_ingam5.csv','db/UI/dbwidget_ingam7.csv']
    tables={path:rows(read(path)) for path in paths};selected=tables[paths[2]]
    widget_names=[name for name in selected if name.startswith(('diplo_','prop_')) or name=='game_modal_overlay']
    widgets={name:selected[name] for name in widget_names}
    differences={name:{key:[tables[paths[1]][name][key],row[key]] for key in row if key!='sourceLine' and tables[paths[1]][name][key]!=row[key]} for name,row in widgets.items()}
    differences={name:value for name,value in differences.items() if value}
    assert all(set(diff)=={'ARGSTR'} for diff in differences.values()),differences
    frames=rows(read('db/UI/dbwidgetframestyle.csv'));frame_styles={}
    for widget in widgets.values():
        key=widget['FRAMESTYLE']
        for suffix in ('','_e2','_e3'):
            if key and key+suffix in frames:frame_styles[key+suffix]=frames[key+suffix]
    all_font_styles=rows(read('db/UI/dbfontstyle.csv'));font_styles={}
    for widget in widgets.values():
        key=widget['FONTSTYLE']
        for suffix in ('','_e2','_e3'):
            if key and key+suffix in all_font_styles:font_styles[key+suffix]=all_font_styles[key+suffix]
    font_text=read('db/UI/dbfont.csv').decode('utf-8-sig').replace('\r','');font_lines=font_text.splitlines();font_header=bracket_csv(font_lines[0].removeprefix('//').strip());fonts={}
    for line_number,line in enumerate(font_lines[1:],2):
        if not line.strip() or line.lstrip().startswith('//'):continue
        fields=bracket_csv(line);assert len(fields)==len(font_header),(line_number,len(fields),len(font_header));font=dict(zip(font_header,fields));font['sourceLine']=line_number
        if font['NAME'] in {row['FONT'] for row in font_styles.values()}:fonts[font['NAME']]=font
    with zipfile.ZipFile(INSTALL/'zips/db.zip') as archive:
        canonical_path='db/UI/UnpackedSprites/dbsprite_uigame.csv';raw_canonical=archive.read(canonical_path);canonical=rows(raw_canonical)
        canonical_shared_path='db/UI/UnpackedSprites/dbsprite_uigeneric.csv';raw_canonical_shared=archive.read(canonical_shared_path)
        for name,row in rows(raw_canonical_shared).items():
            if name not in canonical:canonical[name]=row
    packed_path='db/UI/PackedSprite7/dbsprite_uigame_packed7.csv'
    packed_archive='Unofficial Patch Files/EXEGeneratorData/EE2ExeSourcePart10'
    shared_path='db/UI/PackedSprite7/dbsprite_uigeneric_packed7.csv'
    with zipfile.ZipFile(INSTALL/packed_archive) as archive:
        raw_packed=archive.read(packed_path);sprites=rows(raw_packed)
        raw_shared=archive.read(shared_path);shared_sprites=rows(raw_shared)
    sprite_sources={name:(packed_path,sha(raw_packed)) for name in sprites}
    for name,row in shared_sprites.items():
        if name not in sprites:sprites[name]=row;sprite_sources[name]=(shared_path,sha(raw_shared))
    wanted={r['SPRITE'] for r in widgets.values() if r['SPRITE']}
    # Runtime diplomacy status icons and epoch resource replacements are named
    # source sprites. Keep complete declared states instead of generated tints.
    wanted|={name for name in canonical if name.startswith(('icon_diplo_','spr_dip_'))}
    wanted|={f'icon_resource_{resource}' for resource in ['food','wood','stone','gold','tin','iron','saltpeter','oil','uranium']}
    for frame in frame_styles.values():
        name=frame['SPRITE']
        if name:
            ordered=list(canonical);index=ordered.index(name)
            assert frame['FRAME TYPE']=='FourCornersFourTiledEdges',frame
            wanted.update(ordered[index:index+8])
    for name in list(wanted):wanted.update(candidate for candidate in canonical if candidate.startswith(name+'_'))
    assert not wanted-set(sprites),sorted(wanted-set(sprites))
    OUT.mkdir(parents=True,exist_ok=True);texture_archive='zips/Textures_enh_160.zip'
    textures={};images={};decoded={}
    with zipfile.ZipFile(INSTALL/texture_archive) as archive,zipfile.ZipFile(INSTALL/'zips/textures.zip') as shared_archive:
        texmap={PurePosixPath(name).stem.casefold():name for name in archive.namelist() if name.lower().endswith('.texcache')}
        shared_texmap={PurePosixPath(name).stem.casefold():name for name in shared_archive.namelist() if name.lower().endswith('.texcache')}
        for name in sorted(wanted):
            row=sprites[name];stem=PureWindowsPath(row['FILENAME']).stem.casefold();table,table_sha=sprite_sources[name]
            # Shared white-pixel sprites remain the source1x1 base texture;
            # UI7 declares them in its generic table, without enhanced repaint.
            shared=stem not in texmap
            if shared:assert not stem.startswith('enh_') and stem in shared_texmap,(name,stem)
            member=(shared_texmap if shared else texmap)[stem];origin_archive='zips/textures.zip' if shared else texture_archive
            if member not in decoded:
                raw=(shared_archive if shared else archive).read(member);image,meta=decode_texcache(raw);decoded[member]=image
                textures[member]={'archive':origin_archive,'member':member,'sha256':sha(raw),**meta}
            source=decoded[member];x,y,w,h=[int(row[k]) for k in ('U','V','WIDTH','HEIGHT')]
            assert x>=0 and y>=0 and x+w<=source.width and y+h<=source.height,(name,[x,y,w,h],source.size)
            crop=source.crop((x,y,x+w,y+h));rotated=row['FLAGS']=='(Rotate)'
            assert row['FLAGS'] in ('','0','(Rotate)'),row
            if rotated:crop=crop.transpose(Image.Transpose.ROTATE_90)
            logical=[int(canonical[name][k]) for k in ('WIDTH','HEIGHT')]
            if canonical[name]['FLAGS']=='(Rotate)':logical.reverse()
            ratios=[crop.width/logical[0],crop.height/logical[1]]
            scale=ratios[0] if ratios[0]==ratios[1] else ratios
            if not shared:assert scale==4,(name,crop.size,logical)
            file=OUT/(name+'.png');crop.save(file,optimize=True)
            images[name]={'file':file.name,'sha256':sha(file.read_bytes()),'width':crop.width,'height':crop.height,'logicalWidth':logical[0],'logicalHeight':logical[1],'sourceResolutionScale':scale,'sourceTexture':member,'restoredPackedRotation':rotated,'canonicalFlags':canonical[name]['FLAGS'],
                'sourceSprite':{'archive':packed_archive,'table':table,'tableSha256':table_sha,'line':row['sourceLine'],'rect':[x,y,w,h],'flags':row['FLAGS']}}
    labels={};label_sources={}
    wanted_labels={w['LABEL'] for w in widgets.values() if w['LABEL']}
    for path in ['db/Text/dbtext_ingame_widgets.utf8','db/Text/dbtext_tooltips.utf8']:
        if path.casefold() not in resolution['effectiveByPath']:continue
        raw=read(path);text=raw.decode('utf-8-sig');parsed=csv.reader(io.StringIO(text))
        for row in parsed:
            if len(row)>=2 and row[0] in wanted_labels:
                labels[row[0]]=row[1].strip('"').replace('\\n','\n');label_sources[row[0]]={'path':path,'line':parsed.line_num,'sha256':sha(raw)}
    strips={}
    for name in ['spr_dip_pcolor_strip_glow','spr_dip_pcolor_strip_glow_on','spr_dip_pcolor_strip_2state','spr_dip_pcolor_strip_2state_on']:
        image=Image.open(OUT/images[name]['file']).convert('RGBA');raw=image.tobytes()
        strips[name]={'width':image.width,'height':image.height,'pngFile':images[name]['file'],'rgbaSha256':sha(raw),'rgba':base64.b64encode(raw).decode('ascii')}
    manifest={'format':'ee2-native-diplomacy-v1','sourceResolutionManifestSha256':sha(RESOLVED.read_bytes()),'coordinateReference':[1600,1200],'sourceResolutionScale':4,
        'profile':'Source UI7 geometry, enhanced atlases and the shared original sprites explicitly named by UI7. UI5 geometry is equal; four background tiling ARGSTR values differ and remain audited.',
        'sourceWidgets':source_records[paths[2]],'ui5To7Differences':differences,'sources':source_records,
        'sourceCanonicalSprites':{'archive':'zips/db.zip','member':canonical_path,'sha256':sha(raw_canonical)},'sourceCanonicalSharedSprites':{'archive':'zips/db.zip','member':canonical_shared_path,'sha256':sha(raw_canonical_shared)},'sourceSprites':{'archive':packed_archive,'member':packed_path,'sha256':sha(raw_packed)},'sourceSharedSprites':{'archive':packed_archive,'member':shared_path,'sha256':sha(raw_shared)},
        'widgets':widgets,'frameStyles':frame_styles,'fontStyles':font_styles,'fonts':fonts,'resolutionWidths':[800,1366,1600,1920,2048,2560],
        'sprites':images,'playerStripPixels':strips,'textures':textures,'labels':labels,'labelSources':label_sources,
        'limits':['Source UI7 layout and sprites, with source-backed frame/ancestor/center behavior. Original executable and GUI were never launched; controller/render adapters remain explicit.','Source visual templates do not implement native diplomacy simulation, unit tribute, tariffs, border rights or treaty durations. Existing application commands and validations remain authoritative.','Text uses source strings where declared and browser font rendering; native bitmap font raster is not reconstructed.']}
    (OUT/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    for archive in archives.values():archive.close()
    print(json.dumps({'widgets':len(widgets),'sprites':len(images),'textures':len(textures),'labels':len(labels),'manifestSha256':sha((OUT/'manifest.json').read_bytes())}))
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--out',type=Path,default=OUT);parser.add_argument('--install',type=Path,default=INSTALL);parser.add_argument('--sources',type=Path,default=RESOLVED);args=parser.parse_args();OUT=args.out;INSTALL=args.install;RESOLVED=args.sources;main()
