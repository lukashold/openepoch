"""Upgrade native ground bindings using compiled UnitType field conversions.

The source audit is retained unchanged. Native floats, axis conventions and
original decoded image bytes are preserved; no model-size adapter is applied."""
import argparse,copy,hashlib,json,pathlib,re,shutil,struct,sys,zipfile
PROJECT=pathlib.Path(__file__).resolve().parents[1];ROOT=PROJECT.parents[1]
from import_ee2 import block,without_comments
F=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]
SQRT2=F(1.4142135381698608);EPS=F(1.1920928955078125e-7)
SHA=lambda b:hashlib.sha256(b).hexdigest()
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
ENGINE_RELATIVE=pathlib.Path('Unofficial Patch Files/EnabledUP15Units/EE2.exe')
DEST=PROJECT/'docs'

def sizing(raw):
    def scaled(a):return[F(F(x)*SQRT2)for x in a]
    size=list(map(F,raw['size']));fp=list(map(F,raw['footprint']));body=size if raw['footprintType']==1 else scaled(size)
    footprint=scaled(size if (sum(x*x for x in fp)**.5)<=EPS else fp)
    under=scaled(raw['underSize']);offset=scaled(raw['underOffset']);center=scaled(raw['centerOffset'])
    if 0 in under:
        mode='footprint';dims=footprint;native_offset=[F(center[k]-(footprint[k]-body[k])*.5)for k in range(2)]
    else:mode='explicit';dims=under[::-1];native_offset=offset
    return {'mode':mode,'width':dims[0],'depth':dims[1],'offsetX':F(-native_offset[1]),'offsetY':F(-native_offset[0]),'bodySize':body,'footprintSize':footprint,'underSize':under,'nativeOffset':native_offset}

def build(destination=DEST,report_path=None,installation=INSTALL,engine_path=None):
    destination=pathlib.Path(destination).resolve();destination.relative_to(ROOT);local=destination/'local-original';local.mkdir(parents=True,exist_ok=True)
    installation=pathlib.Path(installation);archives=installation if installation.name.casefold()=='zips'else installation/'zips'
    install_root=installation.parent if installation.name.casefold()=='zips'else installation
    engine=pathlib.Path(engine_path if engine_path is not None else install_root/ENGINE_RELATIVE).read_bytes();assert SHA(engine)=='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f'
    pe=struct.unpack_from('<I',engine,0x3c)[0];base=struct.unpack_from('<I',engine,pe+52)[0];table=pe+24+struct.unpack_from('<H',engine,pe+20)[0]
    sections=[struct.unpack_from('<IIII',engine,table+40*i+8)for i in range(struct.unpack_from('<H',engine,pe+6)[0])]
    def binary(address,count):
        for _,rva,size,offset in sections:
            if base+rva<=address<base+rva+size:return engine[offset+address-base-rva:offset+address-base-rva+count]
        raise ValueError('Native address outside raw PE')
    assert struct.unpack('<f',binary(0xb43910,4))[0]==SQRT2
    assert struct.unpack('<f',binary(0xb4390c,4))[0]==EPS
    assert binary(0x53b0fd,6)==bytes.fromhex('894d70895574')
    assert binary(0x604164,3)==bytes.fromhex('83f806')and binary(0x60416b,6)==bytes.fromhex('83f80b0f9dc2')
    live=PROJECT/'docs';data=json.loads((live/'local-original/ground-decals.json').read_text());audit=json.loads((live/'local-original/ground-decals-manifest.json').read_text())
    records={r['id']:r for r in audit['entries']+audit.get('unresolvedBindings',[])};corrected=[];all_entries=data['entries']+data['unresolvedBindings']
    with zipfile.ZipFile(archives/'hdrs.zip')as headers:
        header=headers.read('DbHdrs/DbUnitDefs.h');enum=re.search(r'ENUM\s+eFootprintType\s*\{([^}]+)',header.decode('cp1252'))[1]
        footprint_types={name:index for index,name in enumerate(re.findall(r'\bkFootprintType_\w+',enum))}
        assert footprint_types=={'kFootprintType_None':0,'kFootprintType_Ellipse':1,'kFootprintType_Rectangle':2}
    with zipfile.ZipFile(archives/'db.zip')as z:
        units={};sources={}
        for member in {r['unitType']['source'].split('!',1)[1].rsplit(':',1)[0]for r in records.values()}|{'db/Units/building.ddf'}:
            raw=z.read(member);text=without_comments(raw.decode('cp1252'))
            for m in re.finditer(r'\bUnitType\s+(\w+)\s*\{',text):units[m[1]]=block(text,'UnitType',m[1]);sources[m[1]]={'member':member,'sha256':SHA(raw)}
        def prop(unit,key):
            seen=set()
            while unit in units and unit not in seen:
                seen.add(unit);body=units[unit];m=re.search(r'\b'+key+r'\s*=\s*([-+\d.]+)',body,re.I)
                if m and float(m[1])!=0:return float(m[1])
                parent=re.search(r'\bparent\s*=\s*(\w+)',body)
                if not parent:break
                unit=parent[1]
            return 0
        for entry in all_entries:
            proof=records[entry['id']];unit=proof['originalUnit'];raw={key:[prop(unit,prefix+axis)for axis in 'XY']for key,prefix in [('size','Size'),('footprint','FootprintSize'),('centerOffset','CenterOffset'),('underSize','UndertextureSize'),('underOffset','UndertextureOffset')]}
            current=unit;visited=set();footprint_type=None
            while current in units and current not in visited:
                visited.add(current);body=units[current];token=re.search(r'\bFootprintType\s*=\s*(\w+)',body)
                if token:footprint_type=token[1];break
                parent=re.search(r'\bparent\s*=\s*(\w+)',body)
                if not parent:break
                current=parent[1]
            assert footprint_type in footprint_types,(unit,footprint_type)
            raw['footprintType']=footprint_types[footprint_type]
            result=sizing(raw);new=copy.deepcopy(entry)
            new.pop('reason',None)
            for key in ['width','depth','offsetX','offsetY']:new[key]=result[key]
            new['placementProfile']='native-unit-factory-undertexture';new['nativeUnitSizing']={'raw':raw,'stored':result,'source':sources[unit],'footprintTypeSource':{'name':footprint_type,'declaredBy':current,'headerSha256':SHA(header)}}
            corrected.append(new)
    data['entries']=sorted(corrected,key=lambda r:r['id']);data['unresolvedBindings']=[]
    data['selection'].update(epochSlotPolicy='Native player epochs1..5/6..10/11..15 select three source Rural/Urban slots.',seasonalArrayIndexMeaning='Uniform annual interpolation over the full declared seasonal list; duplicates and original order retained.',offsetDefaultPolicy='Native UnitType inheritance and0053A8D0 float field conversion;00603A60 fallback computes center offsets.')
    data.setdefault('runtimeSelection',{}).update(format='ee2-ground-selection-v1',engineSha256=SHA(engine),nativeEpochFirst=[1,6,11],nativeSelector='006040D0',epochGetter='005E0070',cityRadiusFactor=SQRT2,urbanQuery='WASM completed Town Center lifetime-max city registration adapter; native CityManager00814340 strict L1 comparison, no owner filter.',placement='Unit factory0053A8D0 stores Size/Footprint/Undertexture fields with native sqrt2;00603A60 selects fallback or swapped explicit dimensions. Native006037B0/A20800/004A05C0 yield local(-offsetY,-offsetX) in the existing Y-up renderer basis.',remaining='Full native CityManager priorities/history/other emitters and dead-unit decal lifetime are separate.')
    (local/'ground-decals.json').write_text(json.dumps(data,indent=2)+'\n')
    # Keep original exact decoded pixels; all image references remain portable.
    def paths(value):
        if isinstance(value,dict):
            for v in value.values():yield from paths(v)
        elif isinstance(value,list):
            for v in value:yield from paths(v)
        elif isinstance(value,str)and value.startswith('local-original/')and value.endswith('.png'):yield value
    for url in set(paths(data)):
        target=destination/url;target.parent.mkdir(parents=True,exist_ok=True);
        if target.resolve()!=(live/url).resolve():shutil.copyfile(live/url,target)
    report={'status':'passed','build':'water-farm3','format':'ee2-native-undertexture-profile-v1','entries':len(corrected),'sourceFactor':SQRT2,'bindings':[{'id':r['id'],'decal':r['decal'],**r['nativeUnitSizing']}for r in corrected],'manifestSha256':SHA((local/'ground-decals.json').read_bytes()),'runtimeDocsEdited':destination==live.resolve()}
    (pathlib.Path(report_path)if report_path else PROJECT.parent/'EE2-Research/analysis/native3-ground-profile-import.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'candidate':str(destination),'entries':len(corrected),'farm':next(r['nativeUnitSizing']['stored']for r in corrected if r['id']==2)}))
    return report

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--docs',type=pathlib.Path,default=DEST);ap.add_argument('--report',type=pathlib.Path);ap.add_argument('--install',type=pathlib.Path,default=INSTALL,help='Original installation containing zips/; archives are read only');ap.add_argument('--engine',type=pathlib.Path,help='Read-only native engine; default: <install>/Unofficial Patch Files/EnabledUP15Units/EE2.exe');args=ap.parse_args();build(args.docs,args.report,args.install,args.engine)
