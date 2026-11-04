"""Bind native building-ground epoch/world-dimension rules to audited inputs."""
import hashlib,json,pathlib,re,struct,zipfile
from import_ee2 import block,without_comments
ROOT=pathlib.Path(__file__).resolve().parents[1];WORKSPACE=ROOT.parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
ENGINE_SHA='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f'


def run():
    raw=(WORKSPACE/'work/EE2-engine.exe').read_bytes();assert hashlib.sha256(raw).hexdigest()==ENGINE_SHA
    pe=struct.unpack_from('<I',raw,0x3c)[0];base=struct.unpack_from('<I',raw,pe+52)[0];opt=struct.unpack_from('<H',raw,pe+20)[0];sections=[]
    for i in range(struct.unpack_from('<H',raw,pe+6)[0]):
        _,va,size,start=struct.unpack_from('<IIII',raw,pe+24+opt+i*40+8);sections.append((base+va,size,start))
    def at(va,count):
        for start,size,offset in sections:
            if start<=va<start+size:return raw[offset+va-start:offset+va-start+count]
        raise ValueError('VA outside binary')
    assert at(0x00604164,3)==bytes.fromhex('83f806')
    assert at(0x0060416b,6)==bytes.fromhex('83f80b0f9dc2')
    factor=struct.unpack('<f',at(0x00b43910,4))[0];assert factor==1.4142135381698608
    output=ROOT/'docs/local-original';path=output/'ground-decals.json';data=json.loads(path.read_text(encoding='utf8'))
    audit=json.loads((output/'ground-decals-manifest.json').read_text(encoding='utf8'))
    proof={row['id']:row for row in audit['entries']}
    with zipfile.ZipFile(INSTALL/'zips/db.zip') as db:
        for entry in data['entries']:
            source=proof[entry['id']];member=source['unitType']['source'].split('!',1)[1].rsplit(':',1)[0]
            unit_raw=db.read(member);assert hashlib.sha256(unit_raw).hexdigest()==source['unitType']['sha256']
            body=block(without_comments(unit_raw.decode('cp1252')),'UnitType',source['originalUnit'])
            for field,key in [('UndertextureSizeX','width'),('UndertextureSizeY','depth')]:
                match=re.search(r'\b'+field+r'\s*=\s*([-+\d.]+)',body,re.I);assert match and float(match[1])==entry[key]
            entry['placementProfile']='native-world-undertexture'
    with zipfile.ZipFile(INSTALL/'zips/hdrs.zip') as hdrs:
        epoch=hdrs.read('DbHdrs/DbTechTreeDefs.h');text=epoch.decode('cp1252')
        assert re.search(r'kTechTreeEpoch_Stone\s*=\s*1',text) and re.search(r'kTechTreeEpoch_Synthetic\s*=\s*15',text)
    data['runtimeSelection']={'format':'ee2-ground-selection-v1','engineSha256':ENGINE_SHA,'nativeEpochFirst':[1,6,11],
        'nativeSelector':'006040D0','epochGetter':'005E0070','epochHeaderSha256':hashlib.sha256(epoch).hexdigest(),
        'urbanQuery':'00814340; global CityManager; no owner filter; strict float L1 distance < stored city-band radius times native sqrt2 constant.',
        'cityRadiusFactor':factor,'placement':'00603A60 supplies explicit UndertextureSize/Offset to00714CF0 without model-size normalization.',
        'remaining':'Urban callback must supply registered native-equivalent city bands. Original map generation, full CityManager priorities, road decals and dead-unit lifetime integration are separate.'}
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    report={**data['runtimeSelection'],'bindings':len(data['entries']),'runtimeManifestSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'mediaChanged':False}
    target=ROOT.parent/'EE2-Research/analysis/ground-selection-profile.json';target.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report))


if __name__=='__main__':run()
