"""Reconstruct declared BASE HUD geometry and exact original sprite crops.

Enhanced7 sprites are an explicit quality choice. Widget geometry is read from
the original BASE table; the selected geometry is also shared by UP5/UP7.
No original executable is run.
"""
from pathlib import Path, PurePosixPath, PureWindowsPath
import argparse,csv,hashlib,io,json,sys,zipfile
from PIL import Image

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from extract_ee2_textures import decode_texcache
INSTALL=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
sha=lambda b:hashlib.sha256(b).hexdigest()
WIDGETS='db/UI/dbwidget_ingame.csv'
SPRITES='db/UI/UnpackedSprites/dbsprite_uigame.csv'

ART=[
 'game_top_bkg_1','game_top_bkg_2T','game_top_bkg_2B','game_top_bkg_3',
 'game_playerinfo_epoch','game_bottom_bkg_1','game_bottom_bkg_2B',
 'game_bottom_bkg_2T','game_bottom_bkg_3','game_bottom_bkg_4','game_bottom_bkg_5_nopip',
 'game_top_clock','game_calendar','game_unitinfo',
 'game_bottombuttons_techtree','game_bottombuttons_diplomacy',
 'game_bottombuttons_idlecit','game_bottombuttons_territories',
 'game_button_options','game_button_pause','game_button_chat','game_button_encyclopedia',
]
EXTRA=['spr_game_dash_res_pop',*[f'spr_game_dash_res_{i}' for i in range(1,9)],
       'spr_game_unitinfo_health_bkg','spr_game_unitinfo_meter_health',
       'icon_resource_popcap','icon_resource_tech']

def rows(raw):
    lines=raw.decode('utf-8-sig').replace('\r','').splitlines()
    head=next(csv.reader([lines[0].removeprefix('//').strip()]))
    result={}
    for line_no,line in enumerate(lines[1:],2):
        if not line.strip() or line.lstrip().startswith('//'):continue
        values=next(csv.reader([line]));assert len(values)==len(head),(line_no,len(values),len(head))
        row=dict(zip(head,values));row['sourceLine']=line_no
        assert row['NAME'] not in result,row['NAME']
        result[row['NAME']]=row
    return result

def main(out,enhanced=False):
    out=Path(out);out.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(INSTALL/'zips/db.zip') as db,zipfile.ZipFile(INSTALL/'zips/textures.zip') as tex:
        widget_raw=db.read(WIDGETS);sprite_raw=db.read(SPRITES)
        widgets=rows(widget_raw);sprites=rows(sprite_raw)
        textures={}
        for member in tex.namelist():
            if member.lower().endswith('.texcache'):
                textures.setdefault(PurePosixPath(member).stem.casefold(),[]).append(member)
        chosen={}
        for name in ART:
            assert name in widgets,name
            row=widgets[name];assert row['SPRITE'],name
            chosen[name]=row['SPRITE']
        wanted=set(chosen.values())|set(EXTRA)
        # Adjacent state variants are proved by CSV order, not a filename guess.
        # Only crop variants explicitly following the source base sprite and
        # belonging to the same four-state button sequence.
        names=list(sprites)
        for name in list(wanted):
            if name in sprites:
                index=names.index(name)
                for candidate in names[index+1:index+4]:
                    if candidate.startswith(name+'_'):wanted.add(candidate)
        for name in list(wanted):
            for suffix in ('_e2','_e3'):
                if name+suffix in sprites:wanted.add(name+suffix)
        if enhanced:
            selected_table='db/UI/PackedSprite7/dbsprite_uigame_packed7.csv'
            selected_archive='Unofficial Patch Files/EXEGeneratorData/EE2ExeSourcePart10'
            with zipfile.ZipFile(INSTALL/selected_archive) as packed:
                selected_raw=packed.read(selected_table);selected_sprites=rows(selected_raw)
            selected_texture_archive='zips/Textures_enh_160.zip'
        else:
            selected_table=SPRITES;selected_archive='zips/db.zip'
            selected_raw=sprite_raw;selected_sprites=sprites
            selected_texture_archive='zips/textures.zip'
        with zipfile.ZipFile(INSTALL/selected_texture_archive) as selected_tex:
            texture_paths={}
            for member in selected_tex.namelist():
                if member.lower().endswith('.texcache'):
                    texture_paths.setdefault(PurePosixPath(member).stem.casefold(),[]).append(member)
            # Keep the selected archive open only while decoding its original pixels.
            selected_bytes={}
            for name in wanted:
                stem=PureWindowsPath(selected_sprites[name]['FILENAME']).stem.casefold()
                members=texture_paths.get(stem,[]);assert len(members)==1,(name,stem,len(members))
                if members[0] not in selected_bytes:selected_bytes[members[0]]=selected_tex.read(members[0])
        sources={};output={};images={}
        for name in sorted(wanted):
            row=selected_sprites[name];stem=PureWindowsPath(row['FILENAME']).stem.casefold()
            members=texture_paths.get(stem,[]);assert len(members)==1,(stem,len(members))
            member=members[0]
            if member not in images:
                raw=selected_bytes[member];image,metadata=decode_texcache(raw)
                images[member]=image
                sources[member]={'archive':selected_texture_archive,'member':member,'sha256':sha(raw),**metadata}
            image=images[member];u,v,w,h=[int(row[k]) for k in ('U','V','WIDTH','HEIGHT')]
            assert row['FLAGS'] in ('0','','(Rotate)'),row
            assert u>=0 and v>=0 and w>0 and h>0 and u+w<=image.width and v+h<=image.height,(name,image.size,[u,v,w,h])
            crop=image.crop((u,v,u+w,v+h))
            # Source packed Rotate is restored to the canonical unpacked sprite
            # orientation. Top-left decoded PNG coordinates invert the native
            # texture-UV clockwise convention (GDbSpriteDefs.h flag0x80).
            rotated=row['FLAGS']=='(Rotate)'
            if rotated:crop=crop.transpose(Image.Transpose.ROTATE_90)
            canonical=sprites[name];logical=[int(canonical[k]) for k in ('WIDTH','HEIGHT')]
            expected=[value*(4 if enhanced else 1) for value in logical]
            assert list(crop.size)==expected,(name,crop.size,expected)
            path=out/(name+'.png');crop.save(path,optimize=True)
            output[name]={'file':path.name,'width':crop.width,'height':crop.height,
                'logicalWidth':logical[0],'logicalHeight':logical[1],'sha256':sha(path.read_bytes()),
                'sourceSprite':{'archive':selected_archive,'table':selected_table,'line':row['sourceLine'],'tableSha256':sha(selected_raw),
                    'texture':row['FILENAME'],'rect':[u,v,w,h],'flags':row['FLAGS']},
                'sourceTexture':member,'restoredPackedRotation':rotated,
                'transform':'Decoded original RGBA and exact source crop; packed Rotate restored if declared; no repaint or rescale.'}
        needed=set(ART)|{'game_minimap_container','game_unitbuttons','game_unitinfo',
            'game_resources','game_bottom_bkg_res_cont','game_top_playerinfo','game_playerinfo_name',
            'game_playerinfo_civ','game_unitinfo_health','game_unitinfo_name_cont',
            'game_bottom_bkg_blockCursor1','game_bottom_bkg_blockCursor2','game_bottom_bkg_blockCursor3'}
        def add_dependencies(name):
            row=widgets[name]
            for field in ('PARENT','ANCHORNAME1','ANCHORNAME2'):
                dependency=row[field]
                if dependency and dependency not in needed:
                    needed.add(dependency);add_dependencies(dependency)
        for name in list(needed):add_dependencies(name)
        geometry={name:widgets[name] for name in widgets if name in needed}
        frame_table='db/UI/dbwidgetframestyle.csv';frame_raw=db.read(frame_table)
        all_frames=rows(frame_raw);frame_styles={}
        for row in geometry.values():
            name=row['FRAMESTYLE']
            if not name:continue
            for suffix in ('','_e2','_e3'):
                if name+suffix in all_frames:frame_styles[name+suffix]=all_frames[name+suffix]
        # All fields remain original strings to retain zero vs blank vs -1.
        manifest={'format':'ee2-native-hud-v1','profile':'Declared BASE geometry; explicit enhanced7 source sprites' if enhanced else 'Declared BASE db.zip only',
            'sourceWidgets':{'archive':'zips/db.zip','member':WIDGETS,'sha256':sha(widget_raw)},
            'sourceSprites':{'archive':selected_archive,'member':selected_table,'sha256':sha(selected_raw)},
            'sourceCanonicalSprites':{'archive':'zips/db.zip','member':SPRITES,'sha256':sha(sprite_raw)},
            'sourceResolutionScale':4 if enhanced else 1,
            'sourceFrameStyles':{'archive':'zips/db.zip','member':frame_table,'sha256':sha(frame_raw)},
            'frameStyles':frame_styles,
            'resolutionWidths':[800,1366,1600,1920,2048,2560],
            'coordinateReference':[1600,1200],'metaEpochs':{'stone':[1,5],'parchment':[6,10],'metal':[11,15]},
            'widgets':geometry,'art':chosen,'sprites':output,'textures':sources,
            'limits':['Enhanced7 images explicitly selected from the installed source; the complete native widget lifecycle and bitmap font raster are not reproduced.',
                'Current command menu, resource ordering, game options drawer and player labels are UI adapters.']}
        raw=(json.dumps(manifest,indent=2)+'\n').encode();(out/'manifest.json').write_bytes(raw)
        print(json.dumps({'status':'passed','widgets':len(geometry),'sprites':len(output),
            'sourceTextures':len(sources),'manifestSha256':sha(raw)}))

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--out',default=str(ROOT/'docs/local-original/hud-native'))
    parser.add_argument('--enhanced',action='store_true');parser.add_argument('--install',default=str(INSTALL));args=parser.parse_args();INSTALL=Path(args.install);main(args.out,args.enhanced)
