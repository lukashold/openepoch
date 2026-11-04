"""Import native four-plot Farm composites from exact BASE declarations/NIFs.

Only id2 is activated; all other model payloads and bindings are retained.
Source states are explicit choices, never a render-time harvest simulation."""
import argparse,copy,hashlib,json,pathlib,re,struct,sys,zipfile
import numpy as np
PROJECT=pathlib.Path(__file__).resolve().parents[1];ROOT=PROJECT.parents[1]
from import_ee2 import block,without_comments
import extract_ee2_models as models
from nif_reader import Reader
from extract_ee2_textures import decode_texcache
from import_ee2_model_seasons import inherited_sets,seasonal_rows

INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
SHA=lambda b:hashlib.sha256(b).hexdigest()
F32=lambda f:struct.unpack('<f',struct.pack('<f',f))[0]
DEST=PROJECT/'docs'

def states(body):
    found={}
    for m in re.finditer(r'\{\s*StateName\s*=\s*(\w+)',body):
        start=m.start();depth=0
        for end in range(start,len(body)):
            depth+=(body[end]=='{')-(body[end]=='}')
            if depth==0:break
        text=body[start:end+1];model=re.search(r'\bModelName\s*=\s*([^\s}]+)',text)
        animation=re.search(r'\bAnimName\s*=\s*([^\s}]+)',text)
        found[m[1]]={'model':model[1]if model else None,'animation':animation[1]if animation else None}
    return found

def build(destination=DEST,installation=INSTALL):
    destination=pathlib.Path(destination).resolve();destination.relative_to(ROOT)
    installation=pathlib.Path(installation);archives=installation if installation.name.casefold()=='zips'else installation/'zips'
    local=destination/'local-original';(local/'models').mkdir(parents=True,exist_ok=True)
    live=PROJECT/'docs';native_scales=json.loads((live/'local-original/model-scales.json').read_text())
    conversion=F32(native_scales['unitConversion']['value']);building=F32(native_scales['classDefaults']['kScaleBuilding']['selectedValue'])
    root_scale=F32(building*conversion);child_scale=conversion;world_scale=F32(root_scale*child_scale)
    manifest={'format':'ee2-original-model-manifest-v1','models':[],'errors':[]};season={'format':'ee2-original-model-seasonal-v1','defaultClimate':'Temperate','models':{},'sets':{},'textures':{},'source':{}}
    farm={'format':'ee2-native-farm-composites-v1','scope':'Explicit source StateDatabase choices. No fabricated simulation or render-time harvest cycle.','families':{},'scale':{'building':building,'conversion':conversion,'rootLocal':root_scale,'plotsCompositeLocal':child_scale,'plotLocal':1,'world':world_scale}}
    with zipfile.ZipFile(archives/'db.zip')as db,zipfile.ZipFile(archives/'hdrs.zip')as hdrs,zipfile.ZipFile(archives/'graphics.zip')as graphics,zipfile.ZipFile(archives/'textures.zip')as tex:
        source=db.read('db/Units/farm.ddf');text=without_comments(source.decode('cp1252'));header=hdrs.read('DbHdrs/DbUnitDefs.h')
        assert b'will initialize all dataclass data to 0' in header
        farm['source']={'archive':'db.zip','entry':'db/Units/farm.ddf','sha256':SHA(source),'zeroDefaultHeaderSha256':SHA(header)}
        caches={pathlib.PurePosixPath(n).name.casefold():n for n in graphics.namelist()}
        textures={pathlib.PurePosixPath(n).stem.casefold():n for n in tex.namelist()if n.lower().endswith('.texcache')}
        def texture(source_path,*_):
            stem=pathlib.PureWindowsPath(source_path).stem.casefold()
            if stem not in season['textures']:
                member=textures[stem];raw=tex.read(member);image,meta=decode_texcache(raw)
                path=local/'farm-textures'/f'{stem}.png';path.parent.mkdir(exist_ok=True);image.save(path,optimize=True)
                season['textures'][stem]={'url':path.relative_to(destination).as_posix(),'original_texture_path':source_path,'archive_member':member,'source_sha256':SHA(raw),'png_sha256':SHA(path.read_bytes()),**meta}
            return season['textures'][stem]
        prior=models.extract_texture;models.extract_texture=texture
        all_keys=set()
        try:
            for family in ['FarmWheat','FarmSugarCane','FarmYam']:
                root=block(text,'CompositeModel',family);assert re.search(r'\bScaleType\s*=\s*kScaleBuilding',root)
                inner=block(text,'CompositeModel',family+'CompositePlots');assert not re.search(r'\b(?:ScaleType|ForceScale)\s*=',inner)
                offsets=[];tables=[]
                for number in range(1,5):
                    p=block(text,'FarmPlotModel',f'{family}Plot_{number}')
                    offsets.append([float(re.search(axis+r'Offset\s*=\s*([-\d.]+)',p)[1])for axis in 'XYZ'])
                    database=re.search(r'StateDatabaseName\s*=\s*(\w+)',p)[1];tables.append(states(block(text,'StateDatabase',database)))
                definition={'scale':farm['scale'],'plots':offsets,'states':{},'defaultState':'Harvest_3','rootDefinition':family,'innerDefinition':family+'CompositePlots'}
                for phase in ['Seeding','Harvest_1','Harvest_2','Harvest_3']:
                    entries=[tables[k][f'{phase}_{k+1}']for k in range(4)];assert all(e==entries[0]for e in entries)
                    filename=entries[0]['model'];member=caches[pathlib.PureWindowsPath(filename).name.lower().replace('.nif','.nifcache')];raw=graphics.read(member)
                    reader=Reader(raw).parse();bindings=inherited_sets(reader)
                    data=models.convert(raw,member,pathlib.Path('graphics.zip'),tex,kind='building',normalization={'scale':.01,'offset':[0,0,0],'axis':'x,z,-y','kind':'native-farm-composite'})
                    # Start from the exact single-plot native scene, then apply the
                    # four DDF model-space offsets in the existing Y-up basis.
                    parts=[];season_parts={}
                    for plot,offset in enumerate(offsets):
                        shift=np.array([offset[0],offset[2],-offset[1]])*.01
                        for part in data['parts']:
                            p=copy.deepcopy(part);positions=np.asarray(p['positions']).reshape(-1,3)+shift
                            p['positions']=np.round(positions,6).reshape(-1).tolist();p['farmPlot']=plot+1;p['farmPlotOffset']=shift.tolist();parts.append(p)
                            value=bindings.get(p['sourceNode'])
                            if value:season_parts[str(p['sourceNode'])]={**value,'name':p['name'],'baseTextureSource':p['textureSource'],'baseTextureUrl':p['textureUrl']};all_keys.add(value['key'].casefold())
                    data['parts']=parts
                    vertices=np.concatenate([np.asarray(p['positions']).reshape(-1,3)for p in parts]);data['compositeBounds']={'min':vertices.min(axis=0).tolist(),'max':vertices.max(axis=0).tolist(),'basis':'normalized Y-up four-plot local geometry; sourceBounds retains the original single-plot NIF bounds'}
                    data['nativeFarmComposite']={'family':family,'state':phase,'source':farm['source'],'plotOffsets':offsets,'scale':farm['scale'],'sourceAnimation':entries[0]['animation'],'animationPolicy':'Original KF recorded; no invented idle-loop binding or harvest state progression.'}
                    model_url=f'local-original/models/native-{family.lower()}-{phase.lower()}.json';(destination/model_url).write_text(json.dumps(data,separators=(',',':'))+'\n')
                    definition['states'][phase]={'url':model_url,'source':data['source'],'animation':entries[0]['animation'],'triangles':sum(len(p['indices'])//3 for p in parts)}
                    season['models'][model_url]={'source':data['source'],'parts':season_parts}
                farm['families'][family]=definition
        finally:models.extract_texture=prior
        for climate in ['Temperate','Arid','Tropical']:
            member=f'db/Terrain/DbSeasonalTextureSets_{climate}.csv';raw=db.read(member);rows=seasonal_rows(raw);season['sets'][climate]={}
            for key in all_keys:
                row=rows[key];season['sets'][climate][key]={**row,'urls':[texture(p)['url']for p in row['paths']]}
    active=farm['families']['FarmWheat']['states']['Harvest_3'];manifest['models']=[{'id':2,'epoch':0,'verified':True,**active,'nativeFarmComposite':{'family':'FarmWheat','defaultState':'Harvest_3','source':farm['source'],'scale':farm['scale']}}]
    if (local/'models/manifest.json').exists():
        previous=json.loads((local/'models/manifest.json').read_text());rows=[];inserted=False
        for row in previous['models']:
            if row['id']==2:
                if not inserted:rows.extend(manifest['models']);inserted=True
            else:rows.append(row)
        if not inserted:rows.extend(manifest['models'])
        previous['models']=rows;previous['errors']=[row for row in previous.get('errors',[])if row.get('id')!=2];manifest=previous
    (local/'models/manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');season['source']['modelManifestSha256']=SHA((local/'models/manifest.json').read_bytes())
    if (local/'model-seasons.json').exists():
        previous=json.loads((local/'model-seasons.json').read_text());reused={}
        # A complete seasonal import may already own winter PNGs in its shared
        # cache. Keep that verified URL instead of silently rebinding the same
        # pixels to the Farm-local duplicate during a later targeted import.
        for stem,asset in season['textures'].items():
            prior=previous.get('textures',{}).get(stem,{})
            if prior.get('source_sha256')==asset['source_sha256']and prior.get('png_sha256')==asset['png_sha256']:
                path=live/prior['url']
                if path.is_file()and SHA(path.read_bytes())==prior['png_sha256']:reused[asset['url']]=prior['url'];season['textures'][stem]=prior
        for sets in season['sets'].values():
            for value in sets.values():value['urls']=[reused.get(url,url)for url in value['urls']]
        previous['models'].update(season['models']);previous['textures'].update(season['textures'])
        for climate,sets in season['sets'].items():previous['sets'].setdefault(climate,{}).update(sets)
        previous.setdefault('source',{})['modelManifestSha256']=SHA((local/'models/manifest.json').read_bytes());season=previous
    (local/'model-seasons.json').write_text(json.dumps(season,indent=2)+'\n');(local/'farm-composites.json').write_text(json.dumps(farm,indent=2)+'\n')
    scales=copy.deepcopy(native_scales);prior=scales['variants'].get('2:0',{})
    if prior.get('verified')and prior.get('scaleType')=='nativeFarmComposite'and prior.get('url')==active['url']and prior.get('sourceSha256')==active['source']['sha256']and prior.get('sourceScale')==world_scale:
        farm_binding=prior # Preserve complete verified provenance and key order.
    else:
        model=json.loads((destination/active['url']).read_text())
        farm_binding={'url':active['url'],'sourceSha256':active['source']['sha256'],'sourceEntry':active['source']['entry'],'verified':True,'scaleType':'nativeFarmComposite','sourceScale':world_scale,'scaleLayers':farm['scale'],'compositeProof':farm['source'],'normalization':model['normalization'],'sourceBounds':model['sourceBounds'],'selection':'Exact source four-plot composite with native parent-scale multiplication and documented zero dataclass default'}
    scales['variants']={key:(farm_binding if key=='2:0'else value)for key,value in scales['variants'].items()if not key.startswith('2:')or key=='2:0'}
    scales['variants'].setdefault('2:0',farm_binding)
    (local/'model-scales.json').write_text(json.dumps(scales,indent=2)+'\n')
    print(json.dumps({'candidate':str(destination),'families':len(farm['families']),'states':sum(len(f['states'])for f in farm['families'].values()),'defaultWorldScale':world_scale,'fieldWidth':F32(100*world_scale),'plotCenters':F32(25*world_scale),'textures':len(season['textures'])}))
    return farm

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--docs',type=pathlib.Path,default=DEST);ap.add_argument('--install',type=pathlib.Path,default=INSTALL,help='Original installation containing zips/; archives are read only');args=ap.parse_args();build(args.docs,args.install)
