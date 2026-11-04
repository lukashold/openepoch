"""Decode local EE2 Gamebryo NiPixelData caches into browser PNGs.

No game installation files are modified. The output is local-original/ and must
not be added to the distributable source archive. Pixel layouts are validated
against the NIFTools format description, then decoded by Pillow's DDS reader.
https://github.com/niftools/nifxml/blob/develop/nif.xml
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import io
import json
import re
from pathlib import Path, PurePosixPath, PureWindowsPath
from import_ee2 import block,without_comments
import struct
import zipfile
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
INSTALL = Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
FORMAT_SOURCE = 'https://github.com/niftools/nifxml/blob/develop/nif.xml'

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def decode_texcache(data: bytes) -> tuple[Image.Image, dict]:
    """Read the single NiPixelData object emitted in the installed 10.1 caches.

    Unsupported versions/classes, palette formats, tiling, invalid sizes and
    inconsistent payload lengths fail explicitly rather than guessing offsets.
    The returned image is the original top mip, without rescaling/repainting.
    """
    off = data.find(b'\n') + 1
    if off <= 0 or not data.startswith(b'Gamebryo File Format, Version 10.1.0.0\n'):
        raise ValueError('not a supported Gamebryo 10.1.0.0 texture cache')
    def take(fmt):
        nonlocal off
        size = struct.calcsize('<'+fmt)
        if off+size > len(data): raise ValueError('truncated texture structure')
        values = struct.unpack_from('<'+fmt, data, off); off += size
        return values[0] if len(values) == 1 else values
    version, user, blocks = take('III')
    types = take('H')
    if (version,user,blocks,types) != (0x0a010000,0,1,1):
        raise ValueError('unsupported cache version or block layout')
    length = take('I'); typename = data[off:off+length]; off += length
    if typename != b'NiPixelData' or take('H') != 0: raise ValueError('expected single NiPixelData object')
    groups = take('I')
    if groups > 64: raise ValueError('unreasonable group count')
    for _ in range(groups): take('I')
    record = take('I')
    if record != 0: raise ValueError('unexpected single-object record index')
    fmt, red, green, blue, alpha, bpp = take('IIIIII')
    fast_compare = data[off:off+8]; off += 8
    tiling = take('I')
    palette, mip_count, bytes_per_pixel = take('iII')
    if tiling != 0 or palette != -1 or not (1 <= mip_count <= 16):
        raise ValueError('unsupported tiled/palette texture or mip count')
    mips = [take('III') for _ in range(mip_count)]
    count = take('I')
    payload = data[off:off+count]
    if len(payload) != count: raise ValueError('truncated pixel payload')
    width,height,start = mips[0]
    if not (1 <= width <= 8192 and 1 <= height <= 8192): raise ValueError('invalid texture dimensions')
    if fmt in (4,5,6):
        fourcc = {4:b'DXT1',5:b'DXT3',6:b'DXT5'}[fmt]
        size = ((width+3)//4)*((height+3)//4)*(8 if fmt == 4 else 16)
        flags,pitch = 0x81007,size
        pf = struct.pack('<II4s5I',32,4,fourcc,0,0,0,0,0)
    elif fmt in (0,1) and bpp in (24,32):
        size = width*height*(bpp//8)
        flags,pitch = 0x100f,width*(bpp//8)
        pf = struct.pack('<II4s5I',32,0x40 | (1 if alpha else 0),b'\0'*4,bpp,red,green,blue,alpha)
    else: raise ValueError(f'unsupported NiPixelData format {fmt}, bpp {bpp}')
    if start+size > count: raise ValueError('top mip extends beyond payload')
    header = b'DDS '+struct.pack('<7I',124,flags,height,width,pitch,0,1)+bytes(44)+pf+struct.pack('<5I',0x1000,0,0,0,0)
    image = Image.open(io.BytesIO(header+payload[start:start+size])).convert('RGBA')
    image.load()
    meta = {'format':{0:'RGB24',1:'RGBA32',4:'DXT1',5:'DXT3',6:'DXT5'}[fmt],
            'width':width,'height':height,'mipmaps_in_source':mip_count,
            'pixel_payload_bytes':count,'top_mip_bytes':size,
            'transformation':'Original top mip decoded to RGBA PNG; no rescaling or repainting'}
    return image,meta

def extract_texture(source_path: str, output_dir: Path, archive=None) -> dict:
    """Resolve an original .tga/.dds path to its flat texcache and write a PNG.

    `archive` may be an already-open ZipFile (recommended for a model batch) or
    a Path to textures.zip. Output must be inside this project's docs/ directory.
    The returned metadata includes `url`, source-member hash and output PNG hash.
    """
    output_dir=Path(output_dir).resolve()
    output_dir.relative_to((ROOT/'docs').resolve())
    stem=PureWindowsPath(source_path.replace('/','\\')).stem
    owned=not isinstance(archive,zipfile.ZipFile)
    z=zipfile.ZipFile(archive or INSTALL/'zips/textures.zip') if owned else archive
    try:
        candidates=[n for n in z.namelist() if n.lower().endswith('.texcache') and PurePosixPath(n).stem.casefold()==stem.casefold()]
        if len(candidates)!=1:raise ValueError(f'{source_path}: expected one texcache, found {len(candidates)}')
        member=candidates[0];raw=z.read(member);image,meta=decode_texcache(raw)
        output_dir.mkdir(parents=True,exist_ok=True)
        out=output_dir/(PurePosixPath(member).stem.lower()+'.png');image.save(out,optimize=True)
        return {'url':out.relative_to((ROOT/'docs').resolve()).as_posix(),
                'original_texture_path':source_path,'archive_member':member,
                'source_sha256':sha256(raw),'png_sha256':sha256(out.read_bytes()),**meta}
    finally:
        if owned:z.close()

TERRAIN = {
    'grass':'terr_grass_su', 'dirt':'Terr_Dirt', 'sand':'terr_desert',
    'rock':'terr_cliff01', 'waves':'terr_water_01',
    'grass_patchy':'terr_grass_patchy_su', 'coastal_grass':'terr_coastal_grass01_su',
    'gravel':'Terr_Gravel', 'oceanfloor':'terr_beach_oceanfloor',
    'foundation':'terr_underbuilding_1', 'shore_wave':'fx_waves',
}

# Explicit early western visual correspondences. Workshop is deliberately absent:
# the current game's workshop is an adapted production building, not an EE2 name.
ICONS = {0:'icon_unit_citycenter_1_we',1:'icon_unit_house_1_we',2:'icon_unit_farm',
         3:'icon_unit_barracks_1',5:'icon_unit_dock_1',6:'icon_unit_airport_11',
         7:'icon_unit_outpost_1',8:'icon_unit_citizen_m_we_1',
         47:'icon_unit_university_1',48:'icon_unit_mill_1',49:'icon_unit_stable_2',
         50:'icon_unit_market_2',51:'icon_unit_temple_1',52:'icon_unit_warehouse_1'}
ICONS[56]='icon_unit_fortress_2_we'
# These variants already use imported EE2 gameplay-profile definitions.
EXACT_PROFILE_ICONS = {
    32:('Destroyer',11),33:('Battleship',11),34:('Submarine',11),36:('Submarine',13),
    37:('Fighter',11),38:('Bomber',11),39:('Fighter',13),40:('Bomber',15),
    55:('HercFacility',14),
    56:('Fortress',1),
}
DDF_BUILDING_ICONS = {53:'OilRig',54:'UraniumMine'}
# Explicit weapon/role visual adapters for the smaller OpenEpoch roster. These
# select source pictures only; they do not assert equality of unit definitions.
ROLE_ICONS = {
    9:('HeavyInfantry1',1,'Clubman uses the original maceman image: early blunt melee infantry.'),
    10:('LightArtillery1',1,'Slinger uses the original stone-thrower/sling image: early foot ranged stone weapon.'),
    11:('HeavyInfantry2',2,'Spearman uses the original spearman image; statistics and availability remain adapted.'),
    12:('LightInfantry',3,'Archer uses the original composite bowman image: foot bow infantry.'),
    13:('HeavyInfantry1',3,'Swordsman uses the original swordsman image; availability is adapted.'),
    14:('LightInfantry',5,'Crossbowman uses the original crossbowman image; availability is adapted.'),
    15:('HeavyInfantry1',7,'Musketeer uses the original musketeer image; availability is adapted.'),
    16:('HeavyInfantry1',11,'Rifleman uses the original rifleman image; statistics remain adapted.'),
    17:('HeavyInfantry1',13,'Marine uses the original assault-rifleman image; Marine is not an exact EE2 unit.'),
    18:('HeavyMounted1',2,'Horseman uses the original mounted-infantry image: early melee cavalry.'),
    19:('HeavyMounted1',4,'Knight uses the original armored-cavalry image; Knight is an adapted role.'),
    20:('HeavyArtillery',6,'Trebuchet uses the original trebuchet image; statistics remain adapted.'),
    21:('LightMounted1',10,'Dragoon uses the original dragoon image; statistics remain adapted.'),
    22:('LightArtillery1',9,'Field Cannon uses the original field-gun image: towed gun artillery.'),
    23:('Tank',11,'Tank uses the original gun-tank image; statistics remain adapted.'),
    24:('LightArtillery1',11,'Howitzer uses the original field-artillery image: conventional towed artillery.'),
    25:('AirDefense',10,'Flak Halftrack uses the original early air-defense image; its vehicle chassis and rules are adapted.'),
    26:('HeavyTank',13,'Battle Tank uses the original main-battle-tank image; availability is adapted.'),
    27:('LightArtillery1',13,'Rocket Artillery uses the original mobile-rocket-launcher image; availability is adapted.'),
    28:('AirDefense',13,'SAM Launcher uses the original missile air-defense image; availability is adapted.'),
    29:('WarGalley',1,'Galley uses the original war-galley image; statistics and availability remain adapted.'),
    30:('FightingSail',6,'Carrack uses the original carrack image; statistics remain adapted.'),
    31:('Frigate',10,'Frigate uses the original frigate image; statistics remain adapted.'),
    35:('Battleship',13,'Missile Cruiser uses the original missile-cruiser visual of Battleship epoch 13.'),
    41:('HeavyMounted2',13,'Gunship uses the original gun-helicopter image; its anti-submarine capability is an adapted rule.'),
    42:('CargoShip',6,'Transport Ship uses the original cargo-ship image; transport rules and availability remain adapted.'),
    44:('Fighter',15,'Aurora Interceptor is an own rule archetype using the already-bound original atmospheric-fighter visual and its exact source sprite.'),
    45:('Submarine',15,'Leviathan Sub is an own rule archetype using the already-bound original Manta visual and its exact source sprite.'),
    46:('HeavyTank',15,'Aegis Hover Tank is an own rule archetype using the already-bound original Myrmidon-tank visual and its exact source sprite.'),
}
# Additional native source sprites for verified later visual transitions. These
# are keyed by their exact original sprite identity, not aliased to a guessed
# simulation definition. The model importer independently matches the same
# source class/visual/CSV row before attaching an epoch icon.
NATIVE_EPOCH_ICON_SOURCES = (
    ('HeavyArtillery',10,'heavy_artillery','HeavyArtillery_10'),
    ('LightArtillery2',15,'light_artillery2','LightArtillery2_15'),
    ('HeavyTank',15,'heavy_mounted1','HeavyMounted1_15'),
    ('Submarine',15,'submarine','Submarine_15'),
    ('Fighter',15,'fighter','Fighter_15'),
    ('Bomber',14,'bomber','Bomber_14'),
)
ICON_FAMILIES = ('icon_unit_citycenter_','icon_unit_house_','icon_unit_barracks_',
                 'icon_unit_dock_','icon_unit_airport_','icon_unit_outpost_',
                 'icon_unit_citizen_m_we_','icon_unit_citizen_f_we_',
                 'icon_unit_university_','icon_unit_mill_','icon_unit_stable_',
                 'icon_unit_market_','icon_unit_temple_','icon_unit_warehouse_','icon_unit_fortress_')
HUD = ('ui_ingame_bottom_left_stone','ui_ingame_bottom_center_stone',
       'ui_ingame_bottom_res_stone','ui_ingame_bottom_right_nopip_stone',
       'ui_ingame_bottom_clock_panel_stone','ui_ingame_button_small_stone',
       'ui_ingame_button_large_stone','ui_ingame_black_glass',
       'ui_ingame_top_bkg_stone','ui_unitinfo_bkg_stone',
       'ui_fe_bkg_main','ui_fe_bkg_sp','EE2_logo','ui_fe_frame_panel',
       'ui_fe_bkg_parch_top_left','ui_fe_bkg_parch_top_mid','ui_fe_bkg_parch_top_right',
       'ui_fe_bkg_parch_bottom_left','ui_fe_bkg_parch_bottom_mid','ui_fe_bkg_parch_bottom_right')
HUD += tuple('icon_resource_'+name for name in ('food','wood','stone','gold','tin','iron','saltpeter','oil','uranium','tech','territories'))

def source_rows(z, member):
    lines=z.read(member).decode('cp1252').replace('\r','').splitlines()
    header=next(csv.reader([lines[0].removeprefix('//').strip()]))
    result=[]
    for index,line in enumerate(lines[1:],2):
        if not line.strip() or line.lstrip().startswith('//'):continue
        fields=next(csv.reader([line]))
        if len(fields)!=len(header):raise ValueError(f'{member}:{index}: unexpected column count')
        row=dict(zip(header,fields));row['_source']=f'db.zip!{member}:{index}';result.append(row)
    return result

# OpenEpoch ids identify existing visual adapters, not additional original units.
# A binding is emitted only when the named original model actually owns the
# Decal child and its UnitType explicitly declares both undertexture dimensions.
GROUND_BINDINGS = (
    (0,'city_center','CityCenter','UnderCityCenter'),
    (1,'house','House','UnderHouse'),
    (2,'farm','FarmWheat','UnderFarm'),
    (3,'barracks','Barracks','UnderBarracks'),
    (4,'mill','Mill','UnderMill'),
    (6,'airport','Airport','UnderAirport'),
    (47,'university','University','UnderUniversity'),
    (48,'mill','Mill','UnderMill'),
    (49,'stable','Stable','UnderStable'),
    (50,'market','Market','UnderMarket'),
    (51,'temple','Temple','UnderTemple'),
    (52,'warehouse','Warehouse','UnderWarehouse'),
    (55,'HERC_Facility','HercFacility','UnderMill'),
    (56,'fortress','Fortress','UnderFortress'),
)

def extract_ground_decals(installation, extract, output, texture_archive_hash=None):
    """Resolve original DDF -> sprite -> temperate table -> cached pixels.

    Sprite-array slots remain source slots: their epoch thresholds have not
    been established. The most frequent physical filename is a deterministic
    preview adapter; every source seasonal alternative is retained in the audit.
    Missing offset properties are explicitly labelled zero adapter defaults.
    Missing physical undertexture dimensions never create a guessed quad.
    """
    db_path=Path(installation)/'zips/db.zip'
    headers_path=Path(installation)/'zips/hdrs.zip'
    texture_path=Path(installation)/'zips/textures.zip'
    decal_member='db/Visuals/visual_decals.ddf'
    sprite_member='db/Terrain/dbsprite_overlay.csv'
    seasonal_member='db/Terrain/DbSeasonalTextureSets_Temperate.csv'
    flag_member='GDbHdrs/GDbSpriteDefs.h'
    (output/'textures/ground').mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(db_path) as db, zipfile.ZipFile(headers_path) as headers:
        sources={}
        def read(member):
            raw=db.read(member)
            sources[member]={'archive':'db.zip','member':member,'sha256':sha256(raw)}
            return raw.decode('cp1252').replace('\r','')
        decal_text=read(decal_member)
        sprite_text=read(sprite_member)
        seasonal_text=read(seasonal_member)
        sprites={row['NAME'].casefold():row for row in source_rows(db,sprite_member)}
        seasonal={}
        for line_no,line in enumerate(seasonal_text.splitlines(),1):
            if not line.strip() or line.lstrip().startswith('//'):continue
            row=next(csv.reader([line]))
            if len(row)!=2:raise ValueError(f'{seasonal_member}:{line_no}: unexpected columns')
            if row[0].casefold() in seasonal:raise ValueError('duplicate seasonal texture set')
            seasonal[row[0].casefold()]={'paths':row[1].split(),'line':line_no}
        flag_raw=headers.read(flag_member)
        selection={'climate':'Temperate','epochSlotPolicy':'unresolved',
                   'previewSourceSlot':0,
                   'physicalTexturePolicy':'Most frequent path in the source seasonal array; ties use first appearance.',
                   'seasonalArrayIndexMeaning':'Unresolved; source order preserved without invented season labels.',
                   'spriteUVPolicy':'Full original image; source rows have zero crop fields and MipMap only.',
                   'pixelPolicy':'Straight RGBA, original top mip; no crop, scale, flip or repaint.',
                   'offsetDefaultPolicy':'Absent UndertextureOffsetX/Y use zero adapter offsets, recorded per binding.',
                   'scope':'Explicit base db.zip, hdrs.zip and textures.zip only; no VFS precedence claim.'}
        audit={'format':'ee2-ground-decals-audit-v1','selection':selection,
               'archives':{'db.zip':{'path':str(db_path),'sha256':sha256(db_path.read_bytes())},
                           'hdrs.zip':{'path':str(headers_path),'sha256':sha256(headers_path.read_bytes())},
                           'textures.zip':{'path':str(texture_path),'sha256':texture_archive_hash or sha256(texture_path.read_bytes())}},
               'spriteFlagSource':{'archive':'hdrs.zip','member':flag_member,'sha256':sha256(flag_raw)},
               'entries':[],'unresolvedBindings':[],'sources':sources,'textures':{}}
        compact={'format':'ee2-ground-decals-v1','selection':selection,'entries':[],'unresolvedBindings':[]}
        def proof(text,kind,name,member):
            declaration=re.search(r'\b'+re.escape(kind)+r'\s+'+re.escape(name)+r'\s*\{',text,re.I)
            if declaration is None:raise ValueError(f'no {kind} {name} in {member}')
            return {'source':f'db.zip!{member}:{text.count(chr(10),0,declaration.start())+1}',
                    'sha256':sources[member]['sha256']}
        def property_value(body,name):
            match=re.search(r'\b'+name+r'\s*=\s*([-+]?\d+(?:\.\d+)?)\b',body,re.I)
            return float(match[1]) if match else None
        def sprite_asset(name,slot):
            sprite=sprites[name.casefold()]
            rect=[float(sprite[k]) for k in ('U','V','WIDTH','HEIGHT')]
            if rect!=[0,0,0,0]:raise ValueError(f'{name}: only full-image ground sprites supported')
            flags=re.findall(r'\w+',sprite['FLAGS'])
            if flags!=['MipMap']:raise ValueError(f'{name}: unsupported source flags {flags}')
            seasonal_entry=seasonal.get(sprite['FILENAME'].casefold())
            paths=seasonal_entry['paths'] if seasonal_entry else [sprite['FILENAME']]
            if not paths:raise ValueError(f'{name}: empty seasonal table')
            # dict preserves source order, making equal-frequency ties explicit.
            choices=list(dict.fromkeys(paths));chosen=max(choices,key=paths.count)
            variants=[]
            for original_path in choices:
                texture=extract(PureWindowsPath(original_path).stem,'textures/ground')
                image=Image.open(ROOT/'docs'/texture['url']).convert('RGBA')
                alpha=list(image.getchannel('A').getextrema())
                item={**texture,'alphaRange':alpha,'alphaBBox':list(image.getchannel('A').getbbox() or (0,0,0,0))}
                audit['textures'][texture['url']]=item
                variants.append({'originalPath':original_path,'sourceIndices':[i for i,p in enumerate(paths) if p==original_path],
                                 'url':texture['url']})
            selected=next(v for v in variants if v['originalPath']==chosen)
            texture=audit['textures'][selected['url']]
            result={'url':texture['url'],'sprite':name,'sourceSlot':slot,
                    'imageWidth':texture['width'],'imageHeight':texture['height'],
                    'uv':[0,0,1,1],'flipU':False,'flipV':False,'rotate90':False,
                    'alphaRange':texture['alphaRange']}
            detailed={**result,'spriteSource':sprite['_source'],'spriteSourceSha256':sources[sprite_member]['sha256'],
                      'spriteFilename':sprite['FILENAME'],'sourceSpriteRect':rect,'sourceFlags':flags,
                      'seasonalSource':f'db.zip!{seasonal_member}:{seasonal_entry["line"]}' if seasonal_entry else None,
                      'seasonalSourceSha256':sources[seasonal_member]['sha256'] if seasonal_entry else None,
                      'seasonalPaths':paths,'selectedPhysicalPath':chosen,'physicalVariants':variants}
            return result,detailed
        for id,file_stem,unit,decal in GROUND_BINDINGS:
            member=f'db/Units/{file_stem}.ddf';text=read(member);clean=without_comments(text)
            model_kind='CompositeModel' if unit=='FarmWheat' else 'UnitModel'
            model=block(clean,model_kind,unit)
            children=re.search(r'\bChildNames\s*=\s*\[([^]]*)\]',model,re.I)
            if children is None or decal not in children[1].split():raise ValueError(f'{unit}: missing {decal} child')
            unit_body=block(clean,'UnitType',unit)
            # FarmWheat inherits Size but does not declare undertexture size.
            parent=re.search(r'\bparent\s*=\s*(\w+)',unit_body,re.I)
            inherited=block(clean,'UnitType',parent[1]) if unit=='FarmWheat' and parent else ''
            properties={name:property_value(unit_body,name) for name in
                        ('SizeX','SizeY','UndertextureSizeX','UndertextureSizeY','UndertextureOffsetX','UndertextureOffsetY')}
            for name in ('SizeX','SizeY'):
                if properties[name] is None:properties[name]=property_value(inherited,name)
            native_size=[properties['SizeX'],properties['SizeY']]
            body=block(without_comments(decal_text),'Decal',decal)
            states={state:state_body for state,state_body in re.findall(r'\{\s*StateName\s*=\s*(\w+)(.*?)\}',body,re.S)}
            entry={'id':id,'decal':decal,'nativeSize':native_size,
                   'width':properties['UndertextureSizeX'],'depth':properties['UndertextureSizeY'],
                   'offsetX':properties['UndertextureOffsetX'] or 0,'offsetY':properties['UndertextureOffsetY'] or 0,
                   'offsetDefaults':[axis for axis in ('x','y') if properties['UndertextureOffset'+axis.upper()] is None]}
            detailed={**entry,'originalUnit':unit,'bindingType':'role-adapter' if id==4 else 'original-visual',
                      'unitType':proof(text,'UnitType',unit,member),'unitModel':proof(text,model_kind,unit,member),
                      'decalSource':proof(decal_text,'Decal',decal,decal_member),'declaredProperties':properties,'states':{}}
            for state in ('Idle','Death'):
                state_body=states[state];state_compact={};state_audit={}
                for category in ('Rural','Urban'):
                    names_match=re.search(r'\b'+category+r'SpriteNames\s*=\s*\[([^]]*)\]',state_body,re.I)
                    names=names_match[1].split() if names_match else []
                    if len(names)!=3:raise ValueError(f'{decal}/{state}: expected three {category} source slots')
                    values=[sprite_asset(name,slot) for slot,name in enumerate(names)]
                    state_compact[category.lower()]=[v[0] for v in values]
                    state_audit[category.lower()]=[v[1] for v in values]
                state_audit['fadeOutTime']=property_value(state_body,'FadeOutTime')
                state_audit['startFadeDelay']=property_value(state_body,'StartFadeDelay')
                detailed['states'][state]=state_audit
                if state=='Idle':entry.update(state_compact)
                else:entry['death']=state_compact
            if entry['width'] is None or entry['depth'] is None:
                reason='Original UnitType does not explicitly declare both UndertextureSizeX/Y; no quad dimensions invented.'
                compact['unresolvedBindings'].append({**entry,'reason':reason})
                audit['unresolvedBindings'].append({**detailed,'reason':reason})
            else:
                if any(v is None or v<=0 for v in native_size):raise ValueError(f'{unit}: invalid native size')
                compact['entries'].append(entry);audit['entries'].append(detailed)
        (output/'ground-decals.json').write_text(json.dumps(compact,ensure_ascii=False,indent=2),encoding='utf8')
        (output/'ground-decals-manifest.json').write_text(json.dumps(audit,ensure_ascii=False,indent=2),encoding='utf8')
        return compact,audit

def run(installation=INSTALL):
    archive = Path(installation)/'zips/textures.zip'
    archive_hash = sha256(archive.read_bytes())
    output = ROOT/'docs/local-original'
    for folder in ['textures','ui']: (output/folder).mkdir(parents=True,exist_ok=True)
    manifest = {'scope':'Actual local base textures.zip assets for this installed copy; not included in source distribution.',
                'archive':str(archive),'archive_sha256':archive_hash,'format_reference':FORMAT_SOURCE,
                'terrain':{},'icons':{},'iconBindings':{},'iconVariants':{},'variantCropWarnings':[],
                'nativeSpriteIcons':{},
                'hud':{},'hudCropped':{},'files':[],'errors':[]}
    database=Path(installation)/'zips/db.zip'
    manifest['iconBindingDatabase']={'archive':str(database),'sha256':sha256(database.read_bytes())}
    with zipfile.ZipFile(database) as db:
        units=source_rows(db,'db/TechTree/upgrade_unittypes.csv')
        sprites={row['NAME'].casefold():row for row in source_rows(db,'db/UI/UnpackedSprites/dbsprite_unitIcons.csv')}
        upgrade_sha=sha256(db.read('db/TechTree/upgrade_unittypes.csv'))
        sprite_sha=sha256(db.read('db/UI/UnpackedSprites/dbsprite_unitIcons.csv'))
        native_icons=[]
        for unit,epoch,file_stem,expected_visual in NATIVE_EPOCH_ICON_SOURCES:
            row=next(r for r in units if r['UNIT']==unit and int(float(r['EPOCH']))==epoch)
            if row['VISUAL']!=expected_visual:raise ValueError(f'{unit}/E{epoch}: unexpected native visual {row["VISUAL"]}')
            member=f'db/Units/{file_stem}.ddf';raw=db.read(member);text=raw.decode('cp1252').replace('\r','')
            body=block(without_comments(text),'UnitModel',expected_visual)
            model=re.search(r'\bDefaultModel\s*=\s*"?([\w.\\]+)',body,re.I)
            declaration=re.search(r'\bUnitModel\s+'+re.escape(expected_visual)+r'\s*\{',text,re.I)
            if not model or not declaration:raise ValueError(f'No declared native visual for {unit}/E{epoch}')
            native_icons.append({'sprite':row['ICON'],'bindingType':'exact-native-sprite',
                'sourceUnit':unit,'sourceVisual':expected_visual,'sourceEpoch':epoch,
                'unitUpgradeSource':row['_source'],'unitUpgradeSha256':upgrade_sha,
                'visualSource':f'db.zip!{member}:{text.count(chr(10),0,declaration.start())+1}',
                'visualSourceSha256':sha256(raw),'declaredDefaultModel':model[1],
                'scope':'Exact original class/epoch CSV icon and declared DDF visual; no simulation-ID or rule equivalence inferred.'})
        resource_raw=db.read('db/Units/resources.ddf')
        resource_text=resource_raw.decode('cp1252').replace('\r','')
        ddf_bindings={}
        for id,unit in DDF_BUILDING_ICONS.items():
            body=block(without_comments(resource_text),'UnitType',unit)
            icon=re.search(r'\bicon\s*=\s*(\w+)',body,re.I)
            declaration=re.search(r'\bUnitType\s+'+re.escape(unit)+r'\s*\{',resource_text)
            if not icon or not declaration:raise ValueError(f'No source icon for {unit}')
            ddf_bindings[id]={'sprite':icon[1],'bindingType':'exact',
                'bindingReason':'Named original resource-harvesting building icon; source visual identity only.',
                'originalUnit':unit,
                'unitTypeSource':f'db.zip!db/Units/resources.ddf:{resource_text.count(chr(10),0,declaration.start())+1}',
                'unitTypeSourceSha256':sha256(resource_raw)}
    current_names=re.findall(r'^\s+(?:unit|bld)\("([^"]+)"',(ROOT/'src/defs.rs').read_text(encoding='utf8'),re.M)
    with zipfile.ZipFile(archive) as z:
        names = {PurePosixPath(n).stem.lower():n for n in z.namelist() if n.lower().endswith('.texcache')}
        written = {}
        def extract(stem,folder):
            if (stem.lower(),folder) in written: return written[stem.lower(),folder]
            source = names.get(stem.lower())
            if source is None: raise ValueError(f'missing named source {stem}')
            raw = z.read(source); image,meta = decode_texcache(raw)
            filename = f'{folder}/{PurePosixPath(source).stem}.png'
            path = output/filename; image.save(path,optimize=True)
            entry = {'url':f'local-original/{filename}','archive_member':source,
                     'source_sha256':sha256(raw),'png_sha256':sha256(path.read_bytes()),**meta}
            manifest['files'].append(entry); written[stem.lower(),folder] = entry
            return entry
        for key,stem in TERRAIN.items():
            try:manifest['terrain'][key]=extract(stem,'textures')
            except ValueError as e:manifest['errors'].append(str(e))
        for id,stem in ICONS.items():
            try:manifest['icons'][str(id)]=extract(stem,'ui')
            except ValueError as e:manifest['errors'].append(str(e))
        bindings={id:{'sprite':stem,'bindingType':'role-adapter','bindingReason':'Same functional building/citizen role; current rules, regional roster and statistics remain adapted.'} for id,stem in ICONS.items()}
        bindings.update(ddf_bindings)
        for id,(unit,epoch) in EXACT_PROFILE_ICONS.items():
            row=next(r for r in units if r['UNIT']==unit and int(float(r['EPOCH']))==epoch)
            bindings[id]={'sprite':row['ICON'],'bindingType':'exact','bindingReason':'Corresponds to the EE2 unit/epoch variant already selected by the imported gameplay profile.','originalUnit':unit,'originalEpoch':epoch,'unitUpgradeSource':row['_source']}
        for id,(unit,epoch,reason) in ROLE_ICONS.items():
            row=next(r for r in units if r['UNIT']==unit and int(float(r['EPOCH']))==epoch)
            bindings[id]={'sprite':row['ICON'],'bindingType':'role-adapter','bindingReason':reason,'originalUnit':unit,'originalEpoch':epoch,'unitUpgradeSource':row['_source']}
        for id,binding in bindings.items():
            try:
                sprite_name=binding['sprite']
                sprite=sprites.get(sprite_name.casefold()) or sprites.get((sprite_name+'_we').casefold())
                if sprite is None and id in ICONS:
                    matches=[s for s in sprites.values() if PureWindowsPath(s['FILENAME']).stem.casefold()==sprite_name.casefold()]
                    if len(matches)==1:sprite=matches[0]
                if sprite is None:raise ValueError(f'icon binding {id}: no source sprite {sprite_name}')
                base=extract(PureWindowsPath(sprite['FILENAME']).stem,'ui')
                u,v,w,h=(int(float(sprite[key])) for key in ('U','V','WIDTH','HEIGHT'))
                image=Image.open(ROOT/'docs'/base['url']).convert('RGBA')
                if not (u>=0 and v>=0 and w>0 and h>0 and u+w<=image.width and v+h<=image.height):raise ValueError(f'icon binding {id}: sprite bounds exceed texture')
                path=output/'ui'/f'{PurePosixPath(base["archive_member"]).stem}-sprite.png'
                image.crop((u,v,u+w,v+h)).save(path,optimize=True)
                audit={**binding,'currentDef':id,'currentName':current_names[id] if id<len(current_names) else binding.get('originalUnit',str(id)),
                       'resolvedSprite':sprite['NAME'],'spriteSource':sprite['_source'],
                       'spriteRect':[u,v,w,h],'sourceTextureWidth':image.width,'sourceTextureHeight':image.height,
                       'cultureVariant':'western' if sprite['NAME'].lower().endswith('_we') else 'neutral'}
                entry={**base,**audit,'url':path.relative_to(ROOT/'docs').as_posix(),
                       'width':w,'height':h,'png_sha256':sha256(path.read_bytes()),
                       'transformation':'Original top mip decoded to RGBA PNG and cropped exactly to the source sprite rectangle; no rescaling/repainting'}
                manifest['icons'][str(id)]=entry;manifest['iconBindings'][str(id)]=audit
            except (ValueError,StopIteration) as e:manifest['errors'].append(str(e))
        for binding in native_icons:
            try:
                name=binding['sprite'];sprite=sprites.get(name.casefold()) or sprites.get((name+'_we').casefold())
                if sprite is None:raise ValueError(f'native epoch sprite {name}: no exact source sprite')
                base=extract(PureWindowsPath(sprite['FILENAME']).stem,'ui')
                u,v,w,h=(int(float(sprite[key])) for key in ('U','V','WIDTH','HEIGHT'))
                image=Image.open(ROOT/'docs'/base['url']).convert('RGBA')
                if not (u>=0 and v>=0 and w>0 and h>0 and u+w<=image.width and v+h<=image.height):
                    raise ValueError(f'{name}: native source sprite bounds exceed texture')
                path=output/'ui'/f'{PurePosixPath(base["archive_member"]).stem}-sprite.png'
                image.crop((u,v,u+w,v+h)).save(path,optimize=True)
                entry={**base,**binding,'resolvedSprite':sprite['NAME'],
                       'spriteSource':sprite['_source'],'spriteSourceSha256':sprite_sha,
                       'spriteRect':[u,v,w,h],'sourceTextureWidth':image.width,'sourceTextureHeight':image.height,
                       'url':path.relative_to(ROOT/'docs').as_posix(),'width':w,'height':h,'png_sha256':sha256(path.read_bytes()),
                       'transformation':'Original top mip decoded to RGBA PNG and cropped exactly to the source sprite rectangle; no rescaling/repainting'}
                manifest['nativeSpriteIcons'][name.casefold()]=entry
            except (ValueError,StopIteration) as e:manifest['errors'].append(str(e))
        for stem in names:
            if stem.startswith(ICON_FAMILIES):
                # Western variants or culturally neutral names; avoid arbitrary culture mixing.
                if re_has_other_culture(stem): continue
                try:
                    base=extract(stem,'ui')
                    matches=[s for s in sprites.values() if PureWindowsPath(s['FILENAME']).stem.casefold()==stem.casefold()]
                    rectangles={tuple(int(float(s[key])) for key in ('U','V','WIDTH','HEIGHT')) for s in matches}
                    if len(rectangles)!=1:
                        manifest['iconVariants'][stem]={**base,'spriteCropStatus':'unresolved'}
                        manifest['variantCropWarnings'].append(f'{stem}: no unique source sprite rectangle; original full texture retained')
                        continue
                    u,v,w,h=next(iter(rectangles))
                    image=Image.open(ROOT/'docs'/base['url']).convert('RGBA')
                    if not (u>=0 and v>=0 and w>0 and h>0 and u+w<=image.width and v+h<=image.height):
                        raise ValueError(f'{stem}: source sprite rectangle exceeds texture dimensions')
                    path=output/'ui'/f'{PurePosixPath(base["archive_member"]).stem}-sprite.png'
                    image.crop((u,v,u+w,v+h)).save(path,optimize=True)
                    manifest['iconVariants'][stem]={**base,'url':path.relative_to(ROOT/'docs').as_posix(),
                        'width':w,'height':h,'png_sha256':sha256(path.read_bytes()),'spriteCropStatus':'source-verified',
                        'spriteSources':[s['_source'] for s in matches],'resolvedSprites':[s['NAME'] for s in matches],
                        'spriteRect':[u,v,w,h],'sourceTextureWidth':image.width,'sourceTextureHeight':image.height,
                        'transformation':'Original top mip decoded to RGBA PNG and cropped exactly to its unambiguous source sprite rectangle; no rescaling/repainting'}
                except ValueError as e:manifest['errors'].append(f'{stem}: {e}')
        for stem in HUD:
            try:
                original=extract(stem,'ui');manifest['hud'][stem]=original
                image=Image.open(ROOT/'docs'/original['url']).convert('RGBA')
                bbox=image.getchannel('A').getbbox()
                if bbox is None: raise ValueError('entire HUD texture is transparent')
                cropped=image.crop(bbox)
                name=f'ui/{PurePosixPath(original["archive_member"]).stem}-cropped.png'
                path=output/name;cropped.save(path,optimize=True)
                entry={**original,'url':'local-original/'+name,'png_sha256':sha256(path.read_bytes()),
                       'source_width':image.width,'source_height':image.height,
                       'width':cropped.width,'height':cropped.height,'cropRect':list(bbox),
                       'transformation':'Original top mip decoded to RGBA PNG; fully transparent outer margins removed; no rescaling/repainting'}
                manifest['hudCropped'][stem]=entry
            except ValueError as e:manifest['errors'].append(f'{stem}: {e}')
        ground,ground_audit=extract_ground_decals(installation,extract,output,archive_hash)
        manifest['groundDecals']={'url':'local-original/ground-decals.json',
                                 'auditUrl':'local-original/ground-decals-manifest.json',
                                 'bindings':len(ground['entries']),'textures':len(ground_audit['textures']),
                                 'unresolvedBindings':len(ground['unresolvedBindings'])}
    (output/'textures-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf8')
    (output/'ui/icon-bindings.json').write_text(json.dumps(manifest['iconBindings'],ensure_ascii=False,indent=2),encoding='utf8')
    # Compact front-end mapping keeps caller code independent of the audit detail.
    compact={k:{name:item['url'] for name,item in manifest[k].items()} for k in ['terrain','icons','iconVariants','nativeSpriteIcons','hud','hudCropped']}
    (output/'textures.json').write_text(json.dumps(compact,ensure_ascii=False,indent=2),encoding='utf8')
    print(f"Decoded {len(manifest['files'])} original textures; {len(manifest['terrain'])} terrain maps, {len(manifest['icons'])} icon mappings, {len(manifest['iconVariants'])} variants, {len(manifest['hud'])} HUD pieces; {len(manifest['errors'])} errors")
    if manifest['errors']: print('\n'.join(manifest['errors']))
    return manifest

def re_has_other_culture(stem):
    import re
    return bool(re.search(r'_(?:am|fe|me)(?:_|$)',stem))

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('installation',nargs='?',type=Path,default=INSTALL)
    run(parser.parse_args().installation)
