"""Apply original EE2 .pcpatch sparse DXT replacements to original texture mips.

This reconstructs a local asset container, not a procedural team-colour tint.
Patch letters are identified against db/UI/dbcolor.csv using actual resulting
pixels. Unverified mappings are not marked ready for rendering. Source files
remain untouched; generated PNGs belong only in local-original/.
"""
from __future__ import annotations
import argparse
import copy
import csv
import hashlib
import json
import pathlib
import struct
import zipfile
import numpy as np
from PIL import Image,ImageDraw
from extract_ee2_textures import decode_texcache

ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
OUTPUT=ROOT/'docs/local-original/team-textures'
LETTERS='ABCDEFGHIJK'

def sha(data):return hashlib.sha256(data).hexdigest()

def pixel_cache(raw):
    """Read the bounded single-NiPixelData cache without expanding bytes to ints."""
    if not raw.startswith(b'Gamebryo File Format, Version 10.1.0.0\n'):raise ValueError('unsupported cache header')
    cursor=raw.index(b'\n')+1
    def take(fmt):
        nonlocal cursor
        size=struct.calcsize('<'+fmt)
        if cursor+size>len(raw):raise ValueError('truncated pixel cache')
        values=struct.unpack_from('<'+fmt,raw,cursor);cursor+=size
        return values[0] if len(values)==1 else values
    version,user,blocks=take('III');types=take('H')
    if (version,user,blocks,types)!=(0x0a010000,0,1,1):raise ValueError('unsupported pixel cache object layout')
    length=take('I');name=raw[cursor:cursor+length];cursor+=length
    if name!=b'NiPixelData' or take('H')!=0:raise ValueError('expected NiPixelData')
    groups=take('I')
    if groups>64:raise ValueError('invalid cache group count')
    for _ in range(groups):take('I')
    if take('I')!=0:raise ValueError('unexpected object record marker')
    fmt,*masks,bpp=take('6I');fast=raw[cursor:cursor+8];cursor+=8
    tiling=take('I');palette,mip_count,bytes_per_pixel=take('iII')
    if tiling!=0 or palette!=-1 or not 1<=mip_count<=16:raise ValueError('unsupported tiled/palette layout')
    levels=[dict(zip(('width','height','offset'),take('III'))) for _ in range(mip_count)]
    count=take('I');start=cursor
    if count>128*1024*1024 or start+count>len(raw):raise ValueError('invalid pixel payload size')
    return {'format':fmt,'bits_per_pixel':bpp,'bytes_per_pixel':bytes_per_pixel,'masks':masks,
            'fast_compare':fast.hex(),'mips':levels,'payload':raw[start:start+count],
            'payload_offset':start,'payload_bytes':count}

def apply_patch(base_raw,patch_raw):
    base=pixel_cache(base_raw);patch=pixel_cache(patch_raw)
    if base['format'] not in (4,5,6):raise ValueError('base is not DXT1/DXT3/DXT5')
    if patch['format']!=2 or patch['bits_per_pixel']!=8 or len(patch['mips'])!=1:
        raise ValueError('not the verified PAL8-shaped custom patch stream')
    shape=patch['mips'][0]
    if shape!={'width':patch['payload_bytes'],'height':1,'offset':0}:raise ValueError('unexpected patch stream dimensions')
    stream=patch['payload'];block_size=8 if base['format']==4 else 16
    if len(stream)<4:raise ValueError('missing sparse record count')
    count=struct.unpack_from('<I',stream)[0];data_start=4+count*8
    if count>1000000 or data_start>len(stream):raise ValueError('invalid sparse record count')
    pairs=[struct.unpack_from('<II',stream,4+i*8) for i in range(count)]
    mip_count=len(base['mips']);mip=0;consumed=data_start;target=bytearray(base['payload']);stats=[]
    previous_end=0;blocks_in_mip=0;runs_in_mip=0
    for offset,length in pairs:
        if offset==0xffffffff:
            if length!=0 or mip>=mip_count:raise ValueError('invalid mip terminator')
            stats.append({'mip':mip,'runs':runs_in_mip,'blocks':blocks_in_mip})
            mip+=1;previous_end=0;blocks_in_mip=0;runs_in_mip=0;continue
        if mip>=mip_count or length==0:raise ValueError('run outside mip or empty sparse run')
        level=base['mips'][mip];capacity=((level['width']+3)//4)*((level['height']+3)//4)
        if offset<previous_end or offset+length>capacity:raise ValueError('overlapping/out-of-bounds DXT block run')
        size=length*block_size;write=level['offset']+offset*block_size
        if write+size>len(target) or consumed+size>len(stream):raise ValueError('sparse run payload outside buffer')
        target[write:write+size]=stream[consumed:consumed+size];consumed+=size
        previous_end=offset+length;blocks_in_mip+=length;runs_in_mip+=1
    if mip!=mip_count or consumed!=len(stream):raise ValueError('incomplete mip markers or trailing sparse bytes')
    start=base['payload_offset'];patched=base_raw[:start]+bytes(target)+base_raw[start+len(target):]
    return patched,{'record_count':count,'mip_terminators':mip,'dxt_block_bytes':block_size,
        'replaced_blocks':sum(s['blocks'] for s in stats),'replacement_bytes':len(stream)-data_start,
        'mips':stats,'patched_pixel_payload_sha256':sha(target),
        'transformation':'Original sparse DXT replacement blocks copied to original per-mip block indices; original top mip decoded to RGBA PNG. No recolouring, resizing or repainting.'}

def source_palette(archive):
    path='db/UI/dbcolor.csv';raw=archive.read(path);palette={}
    for row in csv.reader(raw.decode('cp1252').replace('\r','').splitlines()):
        if len(row)>=2 and row[0].startswith('playerColor_'):
            palette[row[0].removeprefix('playerColor_')]=[int(v) for v in row[1].split()[:3]]
    if len(palette)!=11:raise ValueError('unexpected source player colour palette')
    return palette,{'archive':'db.zip','entry':path,'sha256':sha(raw)}

def classify(images,palette):
    arrays=np.stack([np.asarray(images[letter],dtype=np.uint8) for letter in LETTERS])
    rgb=arrays[...,:3].astype(np.float64);spread=rgb.max(axis=0)-rgb.min(axis=0)
    mask=(spread.max(axis=2)>=24)&(arrays[0,...,3]>=128)
    if not mask.any():return {'verified':False,'reason':'no measurable team-colour pixel set','mask_pixels':0,'letters':{}}
    weights=spread.max(axis=2)[mask];weights/=weights.sum()
    # Fixed dirt/trim and antialiased edge colours survive inside some replaced
    # blocks. Remove the common per-pixel RGB floor before measuring team chroma.
    rgb=np.maximum(0,rgb-rgb.min(axis=0,keepdims=True))
    names=list(palette);colours=np.asarray([palette[name] for name in names],dtype=float)
    chroma=colours/np.maximum(1,colours.sum(axis=1,keepdims=True))
    result={}
    for index,letter in enumerate(LETTERS):
        mean=(rgb[index][mask]*weights[:,None]).sum(axis=0);normal=mean/max(1,mean.sum())
        distances=np.linalg.norm(chroma-normal[None,:],axis=1);order=np.argsort(distances)
        result[letter]={'colour':names[order[0]],'mean_rgb':[round(float(v),4) for v in mean],
            'chromaticity':[round(float(v),6) for v in normal],
            'distance':round(float(distances[order[0]]),6),'runner_up':names[order[1]],
            'margin':round(float(distances[order[1]]-distances[order[0]]),6)}
    verified=len({v['colour'] for v in result.values()})==11 and max(v['distance'] for v in result.values())<.22 and min(v['margin'] for v in result.values())>.015
    return {'verified':verified,'mask_pixels':int(mask.sum()),'letters':result,
        'method':'Weighted mean chromaticity over pixels that differ across the eleven original variants, after subtracting their common per-pixel RGB floor, compared with the original dbcolor playerColor palette. Letters are not treated as numeric player slots.'}

def run(installation=INSTALL,textures=None,all_models=False):
    installation=pathlib.Path(installation);archive_path=installation/'zips/textures.zip'
    OUTPUT.mkdir(parents=True,exist_ok=True)
    try:previous=json.loads((OUTPUT/'manifest.json').read_text(encoding='utf8'))
    except (FileNotFoundError,ValueError):previous={}
    selected={}
    if all_models:
        model_manifest=json.loads((ROOT/'docs/local-original/models/manifest.json').read_text(encoding='utf8'))
        for row in model_manifest['models']:
            model=json.loads((ROOT/'docs'/row['url']).read_text(encoding='utf8'))
            for texture in model.get('sourceTextures',[]):
                stem=pathlib.PurePosixPath(texture['archive_member']).stem.casefold()
                selected.setdefault(stem,set()).add(texture['url'])
    for stem in textures or ['lsf1_citizen_m_we','bld_citycenter1_we']:
        selected.setdefault(stem.casefold(),set()).add('local-original/model-textures/'+stem.casefold()+'.png')
    output={'format':'ee2-original-team-texture-manifest-v1','pipeline_revision':1,'archive':str(archive_path),
        'scope':'Local original PC-patch derivatives; renderer must use only verified colour mappings. Source suffix letters are not player-slot numbers.',
        'textures':{},'texture_bindings':{},'errors':[],'without_patch':[]}
    with zipfile.ZipFile(archive_path) as z,zipfile.ZipFile(installation/'zips/db.zip') as db:
        palette,palette_source=source_palette(db);output['palette']=palette;output['palette_source']=palette_source
        names={pathlib.PurePosixPath(name).name.casefold():name for name in z.namelist()}
        for stem,urls in sorted(selected.items()):
            if (stem+'a.pcpatch') not in names:output['without_patch'].append(stem);continue
            try:
                base_entry=names[stem+'.texcache'];base_raw=z.read(base_entry);images={};variants={}
                prior=previous.get('textures',{}).get(stem)
                reusable=bool(prior and previous.get('pipeline_revision',1)==1 and prior['base_source']['sha256']==sha(base_raw))
                if reusable:
                    for letter in LETTERS:
                        variant=prior.get('variants',{}).get(letter);patch_entry=names.get(stem+letter.lower()+'.pcpatch')
                        if not variant or not patch_entry or variant['patch_source']['sha256']!=sha(z.read(patch_entry)):
                            reusable=False;break
                        path=(ROOT/'docs'/variant['url']).resolve()
                        if not path.is_relative_to(OUTPUT.resolve()) or not path.exists() or sha(path.read_bytes())!=variant['png_sha256']:
                            reusable=False;break
                if reusable:
                    entry=copy.deepcopy(prior);entry['base_texture_urls']=sorted(urls);output['textures'][stem]=entry
                    for url in urls:output['texture_bindings'][url]=stem
                    print('CACHE',stem,flush=True);continue
                for letter in LETTERS:
                    patch_entry=names[stem+letter.lower()+'.pcpatch'];patch_raw=z.read(patch_entry)
                    result,metadata=apply_patch(base_raw,patch_raw);image,texture_metadata=decode_texcache(result)
                    name=stem+'-'+letter+'.png';path=OUTPUT/name;image.save(path,optimize=True);images[letter]=image
                    variants[letter]={'url':path.relative_to(ROOT/'docs').as_posix(),'png_sha256':sha(path.read_bytes()),
                        'patch_source':{'archive':'textures.zip','entry':patch_entry,'sha256':sha(patch_raw)},**metadata}
                identification=classify(images,palette)
                entry={'base_texture_urls':sorted(urls),'base_source':{'archive':'textures.zip','entry':base_entry,'sha256':sha(base_raw)},
                       'width':texture_metadata['width'],'height':texture_metadata['height'],'format':texture_metadata['format'],
                       'colour_identification':identification,'variants':variants}
                for letter,variant in variants.items():
                    measurement=identification['letters'].get(letter,{})
                    variant['colour']=measurement.get('colour')
                    variant['verified']=bool(measurement.get('distance',1)<.22 and measurement.get('margin',0)>.015)
                entry['verified']=any(v['verified'] for v in variants.values())
                entry['all_variants_verified']=all(v['verified'] for v in variants.values())
                output['textures'][stem]=entry
                for url in urls:output['texture_bindings'][url]=stem
                print('OK',stem,len(variants),'variants',sum(v['verified'] for v in variants.values()),'verified',identification.get('mask_pixels'))
                if stem=='lsf1_citizen_m_we':
                    tile=160;sheet=Image.new('RGB',(6*tile,2*(tile+36)),(24,34,39));draw=ImageDraw.Draw(sheet)
                    for index,letter in enumerate(LETTERS):
                        x=(index%6)*tile;y=(index//6)*(tile+36);thumbnail=images[letter].copy();thumbnail.thumbnail((tile,tile));sheet.paste(thumbnail,(x,y),thumbnail)
                        info=identification['letters'].get(letter,{});draw.text((x+5,y+tile+4),letter+' '+str(info.get('colour','unverified')),fill='white')
                    sheet.save(OUTPUT/'citizen-colour-identification.png')
            except Exception as exc:output['errors'].append({'texture':stem,'error':str(exc)});print('FAIL',stem,type(exc).__name__,exc)
    reference=output['textures'].get('lsf1_citizen_m_we')
    if reference and reference['all_variants_verified']:
        output['colour_mapping_verified']=True
        output['suffix_by_colour']={variant['colour']:letter for letter,variant in reference['variants'].items()}
        output['mapping_evidence']={'reference_texture':'lsf1_citizen_m_we','source_palette':palette_source,
            'method':reference['colour_identification']['method'],
            'contact_sheet':'local-original/team-textures/citizen-colour-identification.png'}
        for entry in output['textures'].values():
            for letter,variant in entry['variants'].items():
                measurement=entry['colour_identification']['letters'].get(letter,{})
                expected=reference['variants'][letter]['colour'];variant['measured_colour']=measurement.get('colour');variant['colour']=expected
                variant['verified']=bool(variant['measured_colour']==expected and measurement.get('distance',1)<.22 and measurement.get('margin',0)>.015)
            entry['verified']=any(v['verified'] for v in entry['variants'].values())
            entry['all_variants_verified']=all(v['verified'] for v in entry['variants'].values())
    else:output['colour_mapping_verified']=False
    output['verified_colour_maps']=sum(entry['verified'] for entry in output['textures'].values())
    output['colour_gate_revision']=2
    temporary=OUTPUT/'manifest.json.tmp';temporary.write_text(json.dumps(output,ensure_ascii=False,indent=2),encoding='utf8');temporary.replace(OUTPUT/'manifest.json')
    print('Team texture maps:',len(output['textures']),'verified:',output['verified_colour_maps'],'errors:',len(output['errors']))
    return output

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--install',type=pathlib.Path,default=INSTALL)
    parser.add_argument('--texture',action='append');parser.add_argument('--all-model-textures',action='store_true')
    args=parser.parse_args();run(args.install,args.texture,args.all_model_textures)
