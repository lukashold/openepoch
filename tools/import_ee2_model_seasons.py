"""Import seasonal model skins from explicit original NIF UserPropBuffer keys.

Geometry/animations/LODs are never regenerated. The separate binding manifest
uses original source-node indices and source hashes. Original installed archives
are read only; newly decoded original pixels and sparse PC patches stay local.
"""
import argparse,csv,hashlib,json,pathlib,re,zipfile
from nif_reader import Reader
from extract_ee2_textures import decode_texcache,extract_texture
import extract_ee2_team_textures as team

ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
CLIMATES=('Temperate','Arid','Tropical')
sha=lambda b:hashlib.sha256(b).hexdigest()

def seasonal_rows(raw):
    rows={}
    for line,row in enumerate(csv.reader(raw.decode('cp1252').splitlines()),1):
        if len(row)<2 or not row[0].strip() or row[0].lstrip().startswith('//'):continue
        key=row[0].strip();paths=row[1].split()
        if not paths:raise ValueError(f'Empty source seasonal list: {key}')
        if key.casefold() in rows:raise ValueError(f'Duplicate source seasonal set: {key}')
        rows[key.casefold()]={'key':key,'line':line,'paths':paths}
    return rows

def animated_set(reader,node):
    found=[]
    for i in node.get('Extra Data List',[]):
        if i<0:continue
        extra=reader.objects[i]
        if extra['_type']!='NiStringExtraData' or extra.get('Name')!='UserPropBuffer':continue
        text=extra.get('String Data','')
        match=re.search(r'AnimatedTextureSetName\s*=\s*<([^<>]+)>',text)
        if match and len(match[1])>1:
            found.append({'key':match[1],'extraDataNode':i,'extraDataOffset':extra['_start'],
                'userPropSha256':sha(text.encode('cp1252'))})
    if len(found)>1:raise ValueError('Ambiguous AnimatedTextureSetName on source mesh')
    return found[0] if found else None

def inherited_sets(reader):
    """Native005F8260/005F8560: an annotated subtree keeps its outer set.

    Once a set is found the native binder attaches it recursively to children,
    without searching their UserPropBuffer again. Keep the exact NIF path as
    evidence rather than selecting an ancestor from names or texture stems.
    """
    result={}
    def visit(index,inherited,path):
        if index<0:return
        if index in path:raise ValueError('Cyclic native scene hierarchy')
        node=reader.objects[index];own=animated_set(reader,node) if inherited is None else None
        value=inherited if inherited is not None else ({**own,'bindingNode':index}if own else None)
        path=[*path,index]
        if value:
            existing=result.get(index)
            if existing and (existing['key'],existing['bindingNode'])!=(value['key'],value['bindingNode']):raise ValueError('Conflicting seasonal parents')
            result[index]={**value,'meshSourcePath':path}
        for child in node.get('Children',[]):visit(child,value,path)
    for root in reader.footer.get('Roots',[0]):visit(root,None,[])
    return result

def run(installation=INSTALL,report_path=None):
    installation=pathlib.Path(installation);docs=ROOT/'docs';output=docs/'local-original/season-model-textures'
    manifest_path=docs/'local-original/models/manifest.json';manifest=json.loads(manifest_path.read_text(encoding='utf8'))
    urls={r['url']for r in manifest['models']}|{r['lodFarUrl']for r in manifest['models']if r.get('lodFarUrl')}
    farm_path=docs/'local-original/farm-composites.json'
    if farm_path.is_file():
        farms=json.loads(farm_path.read_text(encoding='utf8'))
        urls.update(state['url']for family in farms['families'].values()for state in family['states'].values())
    models={};source_groups={};before={};base_textures={}
    # Retain metadata only, not huge vertex/animation arrays, between source files.
    for url in sorted(urls):
        raw=(docs/url).read_bytes();before[url]=sha(raw);model=json.loads(raw)
        slim={'source':model['source'],'parts':[{k:p.get(k)for k in ('sourceNode','name','textureSource','textureUrl')}for p in model['parts']]}
        models[url]=slim;source_groups.setdefault(model['source']['entry'],[]).append(url)
        for source in model.get('sourceTextures',[]):
            stem=pathlib.PurePosixPath(source['archive_member']).stem.casefold()
            base_textures[stem]=source
    bindings={};sets={};assets={};needed=set();source_hashes={};missing={};unresolved=[]
    with zipfile.ZipFile(installation/'zips/graphics.zip')as graphics:
        for member,model_urls in source_groups.items():
            raw=graphics.read(member);digest=sha(raw)
            if any(models[url]['source']['sha256']!=digest for url in model_urls):raise ValueError(f'Source NIF changed: {member}')
            # Plain nonseasonal models need no full geometry parse.
            if b'AnimatedTextureSetName' not in raw:continue
            reader=Reader(raw).parse();native_sets=inherited_sets(reader)
            for url in model_urls:
                parts={}
                for part in models[url]['parts']:
                    node=reader.objects[part['sourceNode']]
                    if node.get('Name')!=part['name']:raise ValueError(f'Source node mismatch: {url}')
                    value=native_sets.get(part['sourceNode'])
                    if value:
                        parts[str(part['sourceNode'])]={**value,'name':part['name'],'baseTextureSource':part['textureSource'],'baseTextureUrl':part['textureUrl']}
                        needed.add(value['key'].casefold())
                if parts:bindings[url]={'source':models[url]['source'],'parts':parts}
    with zipfile.ZipFile(installation/'zips/db.zip')as db,zipfile.ZipFile(installation/'zips/textures.zip')as textures:
        names={pathlib.PurePosixPath(n).stem.casefold():n for n in textures.namelist()if n.casefold().endswith('.texcache')}
        for climate in CLIMATES:
            member=f'db/Terrain/DbSeasonalTextureSets_{climate}.csv';raw=db.read(member);source_hashes[climate]={'archive':'db.zip','entry':member,'sha256':sha(raw)}
            rows=seasonal_rows(raw);sets[climate]={}
            for key in sorted(needed):
                if key not in rows:
                    missing.setdefault(key,[]).append(climate);continue
                row=rows[key];sequence=[]
                for path in row['paths']:
                    stem=pathlib.PureWindowsPath(path).stem.casefold()
                    if stem not in assets:
                        member=names[stem];cache=textures.read(member);old=base_textures.get(stem)
                        if old and old['source_sha256']==sha(cache) and (docs/old['url']).is_file() and sha((docs/old['url']).read_bytes())==old['png_sha256']:
                            meta=dict(old)
                        else:meta=extract_texture(path,output,textures)
                        assets[stem]=meta
                    sequence.append(assets[stem]['url'])
                sets[climate][key]={**row,'urls':sequence}
    # One installed door node names bld_stable11_01, absent from all three CSVs.
    # Preserve the source defect and base image rather than silently guessing
    # the similarly named bld_stable11_1 family. Native terrain fallback differs.
    for url in list(bindings):
        entry=bindings[url]
        for node,binding in list(entry['parts'].items()):
            if binding['key'].casefold() in missing:
                unresolved.append({'modelUrl':url,'sourceNode':int(node),'source':entry['source'],**binding,
                    'missingClimates':missing[binding['key'].casefold()],
                    'fallback':'Existing original base material retained; native terrain-derived fallback is not emulated.'})
                del entry['parts'][node]
        if not entry['parts']:del bindings[url]
    needed={part['key'].casefold()for entry in bindings.values()for part in entry['parts'].values()}
    # Existing material/team manifests remain untouched. A separate patch cache
    # includes the original citizen reference needed for measured colour gating.
    winter_stems=sorted(stem for stem,a in assets.items()if a['url'].startswith('local-original/season-model-textures/'))
    team.OUTPUT=output/'team'
    team_manifest=team.run(installation,textures=winter_stems+['lsf1_citizen_m_we'])
    team_manifest['texture_bindings']={}
    for stem,entry in team_manifest['textures'].items():
        urls=[assets[stem]['url']]if stem in assets else entry['base_texture_urls']
        entry['base_texture_urls']=urls
        for url in urls:team_manifest['texture_bindings'][url]=stem
    team_path=team.OUTPUT/'manifest.json';team_path.write_text(json.dumps(team_manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    header_member='DbHdrs/DbTerrainDefs.h'
    with zipfile.ZipFile(installation/'zips/hdrs.zip')as z:header_hash=sha(z.read(header_member))
    result={'format':'ee2-original-model-seasonal-v1','defaultClimate':'Temperate',
        'bindingRule':'Native005F8450 reads NiStringExtraData UserPropBuffer/AnimatedTextureSetName=<key>. Native005F8260/005F8560 apply an explicit outer node set throughout its subtree; exact meshSourcePath and bindingNode preserve this inheritance. No texture-filename inference.',
        'interpolationRule':'Preserve full source order, duplicate entries and actual list length; sample uniformly over the year as DbTextureSetinfo documents.',
        'models':bindings,'unresolvedBindings':unresolved,'sets':sets,'textures':assets,'teamManifest':team_path.relative_to(docs).as_posix(),
        'source':{'modelManifestSha256':sha(manifest_path.read_bytes()),'climates':source_hashes,'header':{'archive':'hdrs.zip','entry':header_member,'sha256':header_hash}}}
    target=docs/'local-original/model-seasons.json';target.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    for url,digest in before.items():
        if sha((docs/url).read_bytes())!=digest:raise AssertionError(f'Geometry payload changed: {url}')
    report={'status':'passed','format':'ee2-model-season-import-v1','modelsUnchanged':len(before),'seasonalModels':len(bindings),
        'seasonalParts':sum(len(v['parts'])for v in bindings.values()),'inheritedParts':sum(int(node)!=part['bindingNode']for v in bindings.values()for node,part in v['parts'].items()),'unresolvedBindings':unresolved,'setsPerClimate':len(needed),'climates':list(CLIMATES),
        'listLengths':sorted({len(s['paths'])for climate in sets.values()for s in climate.values()}),
        'physicalTextureCount':len(assets),'newNeutralTextures':len(winter_stems),'newTeamTextureSets':len(team_manifest['textures']),
        'teamErrors':team_manifest['errors'],'manifestSha256':sha(target.read_bytes()),'teamManifestSha256':sha(team_path.read_bytes()),
        'unchangedModelHashes':before,'source':result['source']}
    dest=pathlib.Path(report_path)if report_path else ROOT.parent/'EE2-Research/analysis/native-model-season-import.json';dest.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(json.dumps({k:v for k,v in report.items()if k not in ('unchangedModelHashes','source')}),flush=True)
    return result

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--install',type=pathlib.Path,default=INSTALL);ap.add_argument('--report',type=pathlib.Path);args=ap.parse_args();run(args.install,args.report)
