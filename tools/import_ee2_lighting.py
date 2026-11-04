"""Import source atmosphere/light values without executing an original program.

The static Afternoon/Normal selection is an application choice. BASE and the
engine's explicitly named 155 tables remain separately selectable source data.
"""
import argparse, csv, hashlib, json, pathlib, re, struct, zipfile
from import_ee2 import block, without_comments
from nif_reader import Reader
from test_ee2_city_evidence import PE

ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
ENGINE_SHA='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f'
sha=lambda raw:hashlib.sha256(raw).hexdigest()

def build(docs=ROOT/'docs',installation=INSTALL,engine_path=None,report_path=None):
    installation=pathlib.Path(installation);docs=pathlib.Path(docs)
    engine_path=pathlib.Path(engine_path)if engine_path else installation/'Unofficial Patch Files/EnabledUP15Units/EE2.exe'
    engine=engine_path.read_bytes();assert sha(engine)==ENGINE_SHA,'Unrecognized source engine'
    pe=PE(engine);sources={};profiles={}
    def member(archive,name):
        with zipfile.ZipFile(installation/'zips'/archive)as z:raw=z.read(name)
        sources[archive+'!'+name]={'archive':archive,'entry':name,'sha256':sha(raw),'bytes':len(raw)}
        return raw
    member('hdrs.zip','DbHdrs/DbClimateDefs.h')
    for key,archive,ddf,atmos in [('base','db.zip','db/Terrain/dbclimate.ddf','db/Terrain/dbclimate_atmosphere.csv'),('native155','db_155.zip','db/Terrain/dbclimat5.ddf','db/Terrain/dbclimate_atmospher5.csv')]:
        text=without_comments(member(archive,ddf).decode('cp1252'));raw=member(archive,atmos);rows={}
        for index,row in enumerate(csv.reader(raw.decode('cp1252').splitlines()),1):
            if not row or row[0].lstrip().startswith('//'):continue
            assert len(row)==8
            colours=[]
            for cell in row[1:5]:
                values=[int(v)for v in re.findall(r'\d+',cell)]
                assert len(values)==4 and all(0<=v<=255 for v in values)
                colours.append(values)
            rows[row[0]]={'line':index,'sourceRGBA':dict(zip(['diffuse','ambient','specular','fog'],colours)),
                         'fogStart':float(row[5]),'fogOn':row[6].upper()=='TRUE','fogIntensity':float(row[7])}
        climates={};tod={}
        for climate in ['Temperate','Arid','Tropical','Tundra']:
            body=block(text,'DbClimateInfo',climate);climates[climate]={}
            if not body.strip():continue
            for period in ['Morning','Afternoon','Evening','Night']:
                name=re.search(r'\bToDSettings'+period+r'\s*=\s*(\w+)',body)[1];climates[climate][period]=name
                if name in tod:continue
                settings=block(text,'DbClimateToDInfo',name)
                direction=[float(v)for v in re.search(r'SunDirectionVec3f\s*=\s*\[([^]]+)',settings)[1].split()]
                assert len(direction)==3
                weather={}
                for state,field in [('Normal','Normal'),('RainLt','RainLight'),('SnowLt','SnowLight'),('DustLt','DustLight'),('RainHv','RainHeavy'),('SnowHv','SnowHeavy'),('DustHv','DustHeavy')]:
                    target=re.search(r'\bAtmosphericSettings'+field+r'\s*=\s*(\w+)',settings)[1]
                    assert target in rows;weather[state]=target
                tod[name]={'sunDirectionSource':direction,'atmosphere':weather}
        profiles[key]={'sources':[archive+'!'+ddf,archive+'!'+atmos],'climates':climates,'timeOfDay':tod,'atmospheres':rows}
    lights=Reader(member('graphics.zip','cache/nifcache/lights.nifcache')).parse()
    def select(index):
        obj=lights.objects[index]
        if obj['_type']=='NiDirectionalLight':return index
        if obj['_type']!='NiNode':return None
        for child in obj.get('Children',[]):
            if child>=0:
                selected=select(child)
                if selected is not None:return selected
        return None
    selected=select(lights.footer['Roots'][0]);assert selected==3
    obj=lights.objects[selected]
    selected_light={'sourceNode':selected,**{key:obj[key]for key in ['Name','Dimmer','Translation','Rotation','Ambient Color','Diffuse Color','Specular Color']}}
    assert selected_light['Dimmer']==1.5
    # Independent PE guards bind the actual CSV ordering, source light selector,
    # compile-time terrain defaults and native byte→float conversion constant.
    assert pe.call(0x604bdc)==0x5f81c0 and pe.read(0x604be3,3)==bytes.fromhex('894708')
    assert pe.read(0x5f81d9,5)==b'\x3d'+struct.pack('<I',0xd02c50)
    assert pe.read(0x8ef370,15)==bytes.fromhex('68482cd00068644cc700b9502cd000')
    assert pe.call(0xa5cf94)==0xa5c370
    assert pe.read(0xa5cfa2,8)==bytes.fromhex('8a068a4e02884602')
    assert pe.read(0xa5cfb1,2)==bytes.fromhex('880e')
    assert pe.read(0xa5cfd1,4)==bytes.fromhex('c64603ff')
    factor=struct.unpack('<f',pe.read(0xb46fe0,4))[0]
    assert factor==struct.unpack('<f',struct.pack('<f',1/255))[0]
    terrain={}
    for name,address,cell,value in [('diffuseScale',0x6f3520,0xc430c8,.85),('ambientScale',0x6f3540,0xc430cc,.75),('shadowScale',0x6f3560,0xc430d0,1.)]:
        expected=b'\x6a\x01\x68'+struct.pack('<f',value)+b'\x68'+struct.pack('<I',cell)
        assert pe.read(address,len(expected))==expected
        terrain[name]=struct.unpack('<f',pe.read(address+3,4))[0]
    shaders={}
    for name in ['IndexPal20.NSF','vsh_psh/IndexPal20_High.vsh','vsh_psh/IndexPal20_Low.vsh']:
        raw=(installation/'shaders'/name).read_bytes();shaders[name]={'sha256':sha(raw),'bytes':len(raw)}
    result={'format':'ee2-native-lighting-v1','profiles':profiles,'sources':sources,'shaders':shaders,
            'sourceEngineSha256':ENGINE_SHA,'sceneLight':{'source':'graphics.zip!cache/nifcache/lights.nifcache','selected':selected_light,
            'binding':'00604BA0→005F81C0: first NiDirectionalLight in child order, stored Scene+8;006296B0 multiplies diffuse/ambient by Dimmer.'},
            'nativeColorToFloat':{'reader':'00A5CF80','sourceOrder':'RGBA','packedOrder':'BGRA','helper':'00A30390','packedByteOffsetsRGB':[2,1,0],'floatFactor':factor,'factorAddress':'00B46FE0'},
            'terrain':terrain,'evidence':{'sunDirection':'006CCA00→008EF400 normalized source vector to world-direction column0.',
            'indexPal':'005F9100 selects IndexPal20_High/Low for skinned geometry; original NSF base stage is TOP_Modulate4x and alpha TOP_Modulate.',
            'staticTextureStage':'00957B70 APPLY_MODULATE4X4→COLOROP6 via0094BFB0; source material/specular state remains distinct.',
            'terrainScalars':'006F3520/3540/3560 compiled defaults;006F41D0 uses diffuse*dimmer*.85 and ambient*dimmer*2*.75 before byte packing.'},
            'limits':['Application period/weather selection is explicit, not an asserted original default or reconstructed day/weather scheduler.',
                      'Ordinary global lights.nif binding; scenario/editor light overrides and full native shadow pass graph are not reconstructed.']}
    target=docs/'local-original/climate-lighting.json';target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    report={'status':'passed','format':result['format'],'outputSha256':sha(target.read_bytes()),'bytes':target.stat().st_size,'sourceEngineSha256':ENGINE_SHA,
            'atmospheresPerProfile':{key:len(value['atmospheres'])for key,value in profiles.items()},'selectedLight':selected_light,'sources':sources}
    if report_path:pathlib.Path(report_path).write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(json.dumps({'status':'passed','outputSha256':report['outputSha256'],'atmospheresPerProfile':report['atmospheresPerProfile']}))
    return result

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--docs',type=pathlib.Path,default=ROOT/'docs')
    parser.add_argument('--install',type=pathlib.Path,default=INSTALL);parser.add_argument('--engine',type=pathlib.Path);parser.add_argument('--report',type=pathlib.Path)
    args=parser.parse_args();build(args.docs,args.install,args.engine,args.report)
