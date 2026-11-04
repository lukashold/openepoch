"""Export existing native far LODs, never generate replacement geometry.

Sources without a native LOD group stay explicitly unavailable; the audit
records that fact instead of creating reduced geometry.
"""
import argparse,hashlib,json,pathlib,zipfile
from extract_ee2_models import ROOT,OUT,INSTALL,Reader,convert

def atomic_json(path,data):
    tmp=path.with_suffix('.tmp');tmp.write_text(json.dumps(data,separators=(',',':')),encoding='utf8');tmp.replace(path)

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--install',type=pathlib.Path,default=INSTALL);ap.add_argument('--only',help='Comma-separated simulation ids');args=ap.parse_args()
    only=set(map(int,args.only.split(','))) if args.only else None
    path=OUT/'models/manifest.json';manifest=json.loads(path.read_text(encoding='utf8'));memo={};clips={};audit=[]
    audit_path=OUT/'models/lod-audit.json'
    if only and audit_path.exists():
        selected_urls={row['url'] for row in manifest['models'] if row['id'] in only}
        previous=json.loads(audit_path.read_text(encoding='utf8'))
        audit=[row for row in previous['models'] if row['model'] not in selected_urls]
    audit_path=OUT/'models/lod-audit.json'
    if only and audit_path.exists():
        audit=[row for row in json.loads(audit_path.read_text(encoding='utf8'))['models'] if row['id'] not in only]
    archive=args.install/'zips/graphics.zip';total=0
    with zipfile.ZipFile(archive) as z,zipfile.ZipFile(args.install/'zips/textures.zip') as textures:
        for row in manifest['models']:
            if only and row['id'] not in only:continue
            url=row['url']
            for key in ('lodFarUrl','lodNearTriangles','lodFarTriangles'):
                row.pop(key,None)
            if url in memo:
                if memo[url]:row.update(memo[url])
                continue
            high=json.loads((ROOT/'docs'/url).read_text(encoding='utf8'));source=high['source'];raw=z.read(source['entry']);reader=Reader(raw).parse()
            lods=[o for o in reader.objects if o['_type']=='NiLODNode']
            record={'id':row['id'],'epoch':row['epoch'],'model':url,'source':source,'normalization':high['normalization'],'nearTriangles':sum(len(p['indices'])//3 for p in high['parts'])}
            if not lods:
                record.update(nativeFarAvailable=False,reason='Source contains no NiLODNode. No reduced geometry was invented.')
                row.pop('lodFarUrl',None);memo[url]=None;audit.append(record);continue
            try:
                animations={}
                for clip in high.get('animationSources',[]):
                    entry=clip['entry']
                    if entry not in clips:
                        kr=z.read(entry);clips[entry]={'reader':Reader(kr).parse(),'entry':entry,'sha256':hashlib.sha256(kr).hexdigest()}
                    animations[clip['name']]={**clips[entry],'samples':clip['frames']}
                far=convert(raw,source['entry'],archive,textures,high['normalization']['kind'],animations=animations or None,lod='far',normalization=high['normalization'],omit_static_animations=bool(high.get('animationSampling',{}).get('omitStaticParts')))
                count=sum(len(p['indices'])//3 for p in far['parts'])
                if count>=record['nearTriangles']:
                    record.update(nativeFarAvailable=False,reason='Furthest native branch does not reduce triangle count.',farTriangles=count,lodSelection=far['lodSelection']);memo[url]=None;audit.append(record);continue
                far['nearModelUrl']=url
                if 'resourceVisualBinding' in high:far['resourceVisualBinding']=high['resourceVisualBinding']
                if 'databaseVisualBinding' in high:far['databaseVisualBinding']=high['databaseVisualBinding']
                if 'aircraftVisualBinding' in high:far['aircraftVisualBinding']=high['aircraftVisualBinding']
                if 'nativeMotionBinding' in high:far['nativeMotionBinding']=high['nativeMotionBinding']
                far['billboardNodes']=[{'name':o.get('Name'),'mode':o.get('Billboard Mode')} for o in reader.objects if o['_type']=='NiBillboardNode']
                farurl=url.removesuffix('.json')+'_lodfar.json';atomic_json(ROOT/'docs'/farurl,far)
                meta={'lodFarUrl':farurl,'lodNearTriangles':record['nearTriangles'],'lodFarTriangles':count};row.update(meta);memo[url]=meta
                record.update(nativeFarAvailable=True,url=farurl,farTriangles=count,lodSelection=far['lodSelection'],billboardNodes=far['billboardNodes']);audit.append(record);total+=1
                print('OK',row['id'],pathlib.Path(url).stem,record['nearTriangles'],'->',count,flush=True)
            except Exception as exc:
                record.update(nativeFarAvailable=False,error=f'{type(exc).__name__}: {exc}');audit.append(record);memo[url]=None
                print('FAIL',url,record['error'],flush=True)
    atomic_json(path,manifest)
    atomic_json(audit_path,{'scope':'Existing native LOD selection, preserving near-model scale/offset exactly. No procedural geometry.','models':audit})
    print('Exported',total,'unique native far meshes;',sum(not x['nativeFarAvailable'] for x in audit),'sources have no useful native far LOD')
if __name__=='__main__':main()
