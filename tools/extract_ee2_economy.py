"""Extract an explicit BASE db.zip economy profile and auditable PE constants.

Reads installed archives/EXE only. It never modifies the simulation or installation.
Ghidra findings are separate semantic evidence, with function addresses and hashes.
"""
import argparse,csv,hashlib,io,json,pathlib,re,struct,zipfile
from import_ee2 import without_comments,block,assignments

ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
NAMES=['Food','Wood','Stone','Gold','Tin','Iron','Saltpeter','Oil','Uranium']
GERMAN=['Nahrung','Holz','Stein','Gold','Zinn','Eisen','Salpeter','Öl','Uran']
SHA=lambda raw:hashlib.sha256(raw).hexdigest()

class Source:
    def __init__(self,installation):
        self.installation=installation;self.zips={};self.records={};self.archives={}
    def read(self,archive,entry):
        if archive not in self.zips:
            path=self.installation/'zips'/archive;self.zips[archive]=zipfile.ZipFile(path)
            self.archives[archive]={'path':str(path),'sha256':SHA(path.read_bytes())}
        raw=self.zips[archive].read(entry);key=f'{archive}!{entry}'
        self.records[key]={'archive':archive,'entry':entry,'sha256':SHA(raw)}
        return raw.decode('utf-8-sig').replace('\r','')
    def close(self):
        for z in self.zips.values():z.close()

def section(text,kind,name,source):
    body=block(without_comments(text),kind,name)
    if not body:raise ValueError(f'{source}: missing {kind} {name}')
    m=re.search(r'\b'+re.escape(kind)+r'\s+'+re.escape(name)+r'\s*\{',text)
    return body,{'source':source,'line':text.count('\n',0,m.start())+1,'declaration':f'{kind} {name}'}

def ability(body,name):
    m=re.search(r'\b'+re.escape(name)+r'\s*\{([^}]+)\}',body)
    return m[1] if m else ''

def csv_rows(text):
    lines=text.splitlines();header=next(csv.reader([lines[0].removeprefix('//').strip()]))
    for i,line in enumerate(lines[1:],2):
        if line.strip() and not line.startswith('//'):
            values=next(csv.reader([line]));assert len(values)==len(header),(i,len(values),len(header))
            yield dict(zip(header,values)),i

class PE:
    def __init__(self,path):
        self.path=path;self.raw=path.read_bytes();p=struct.unpack_from('<I',self.raw,0x3c)[0]
        assert self.raw[p:p+4]==b'PE\0\0'
        n=struct.unpack_from('<H',self.raw,p+6)[0];optional=struct.unpack_from('<H',self.raw,p+20)[0]
        assert struct.unpack_from('<H',self.raw,p+24)[0]==0x10b,'Expected analyzed PE32'
        self.base=struct.unpack_from('<I',self.raw,p+24+28)[0];self.sections=[]
        for i in range(n):
            at=p+24+optional+i*40;vs,rva,rs,rp=struct.unpack_from('<IIII',self.raw,at+8)
            self.sections.append((rva,vs,rp,rs))
    def raw4(self,address):
        r=address-self.base;s=next(s for s in self.sections if s[0]<=r<s[0]+max(s[1],s[3]))
        at=s[2]+r-s[0];return self.raw[at:at+4]
    def integer(self,address):return struct.unpack('<i',self.raw4(address))[0]
    def floating(self,address):return struct.unpack('<f',self.raw4(address))[0]

def run(installation,engine):
    src=Source(installation)
    try:
        h=src.read('hdrs.zip','DbHdrs/DbResourceDefs.h');enum=block(without_comments(h),'ENUM','eResourceType')
        values={};value=-1
        for name,explicit in re.findall(r'\b(k\w+)\s*(?:=\s*(-?\d+))?\s*,',enum):
            value=int(explicit) if explicit else value+1;values[name]=value
        assert [values['kResourceType_'+n] for n in NAMES]==list(range(9))
        resources_path='db.zip!db/Units/resources.ddf';resources=src.read('db.zip','db/Units/resources.ddf')
        citizen_path='db.zip!db/Units/citizen.ddf';citizen=src.read('db.zip','db/Units/citizen.ddf')
        full,ref=section(citizen,'UnitType','FullCitizen',citizen_path);gather=assignments(ability(full,'GatherFarm'))
        farm_path='db.zip!db/Units/farm.ddf';farm_text=src.read('db.zip','db/Units/farm.ddf')
        farm_body,farm_proof=section(farm_text,'UnitType','Farm',farm_path);farm_data=assignments(ability(farm_body,'resourceFarm'))
        farm={'unitType':'Farm','resourceId':0,'configuredAmount':int(farm_data['amount']),
              'maxWorkers':int(farm_data['maxNumOfWorkers']),'gatherPerWorkerPerSecond':gather['farmRate'],
              'sourceComment':'Farm keeps producing resource as long as someone works on it.','evidence':farm_proof}
        unit_header=src.read('hdrs.zip','DbHdrs/DbUnitDefs.h')
        src.read('hdrs.zip','DbHdrs/DbUnitDefEnums.h')
        levels=[dict(row,source='db.zip!db/Simulation/dbresourcelevels.csv',line=line) for row,line in csv_rows(src.read('db.zip','db/Simulation/dbresourcelevels.csv'))]
        for row in levels:
            for k in row:
                if k not in ('NAME','Level','source','line'):row[k]=int(row[k])
        techpath='db.zip!db/TechTree/dbtechtreenode.csv';tech=list(csv_rows(src.read('db.zip','db/TechTree/dbtechtreenode.csv')))
        epoch_sources={e:src.read('db.zip',f'db/TechTree/epoch{e}_upgrades.ddf') for e in (4,7,10,13)}
        reveal_names={5:'ResourceIronUpgradeEpoch04',6:'ResSaltpeterUpgradeEpoch07',7:'ResourceOilUpgradeEpoch10',8:'ResourceUraniumUpgradeEpoch13'}
        pe=PE(engine)
        assert SHA(pe.raw)=='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f','Engine must match existing Ghidra project'
        natural=[]
        for i,name in enumerate(NAMES):
            row={'id':i,'enum':'kResourceType_'+name,'name':name,'displayNameDe':GERMAN[i],
                 'citizenConfiguredRate':gather[name[0].lower()+name[1:]+'Rate'],
                 'revealEpoch':1,'revealEpochZeroBased':0,'special':i>=4,'activeEpochFirst':1,'activeEpochEndExclusive':None}
            if i>=4:
                first=pe.integer(0xb5526c+i*4);middle=pe.integer(0xb55270+i*4) if i-3<5 else 1000
                end=pe.integer(0xb55274+i*4) if i-2<5 else 1000
                row.update(activeEpochFirst=first,activeEpochEndExclusive=end if end!=1000 else None,
                           specialSlotByEpoch=[(1 if i==4 or middle<=e else 2) if first<=e<end else -1 for e in range(1,16)],
                           activeWindowEvidence={'function':'00884710','tableBase':'00b5526c','firstAddress':f'{0xb5526c+i*4:08x}'})
            if i in reveal_names:
                epoch=next(e for e,text in epoch_sources.items() if reveal_names[i] in without_comments(text))
                row.update(revealEpoch=epoch,revealEpochZeroBased=epoch-1,revealUpgrade=reveal_names[i],
                           revealSource=f'db.zip!db/TechTree/epoch{epoch}_upgrades.ddf')
            natural.append(row)
        nodes=[];clean=without_comments(resources)
        for m in re.finditer(r'\bUnitType\s+(\w+)\s*\{',clean):
            body,proof=section(resources,'UnitType',m[1],resources_path);res=ability(body,'Resource')
            if not res:continue
            type_name=re.search(r'\btype\s*=\s*kResourceType_(\w+)',res)[1];numeric=assignments(res)
            nodes.append({'unitType':m[1],'resourceId':NAMES.index(type_name),'resource':type_name,
                          'configuredAmount':int(numeric['amount']),'maxWorkers':int(numeric['maxNumOfWorkers']),
                          'alwaysExhaustible':bool(numeric['alwaysExhaustible']) if 'alwaysExhaustible' in numeric else None,
                          'directWorkerGathering':numeric['maxNumOfWorkers']>0,'evidence':proof})
        structures=[]
        for name in ('OilRig','UraniumMine'):
            body,proof=section(resources,'UnitType',name,resources_path);gar=ability(body,'Garrison');nums=assignments(gar)
            model,_=section(resources,'UnitModel',name,resources_path);tr,line=next((r,n) for r,n in tech if r['NAME']==name)
            structures.append({'unitType':name,'placementType':re.search(r'\bplacementType\s*=\s*(\w+)',body)[1],
              'model':re.search(r'\bDefaultModel\s*=\s*(\S+)',model)[1],
              'gatherResource':re.search(r'\bgatherType\s*=\s*(\w+)',gar)[1],
              'garrisonType':re.search(r'\bgarrisonType\s*=\s*(\w+)',gar)[1],
              'garrisonSlots':int(nums['numOfSlots']),'gatherPerOccupiedSlotPerSecond':nums['gatherRate'],
              'maxBaseGatherPerSecond':nums['numOfSlots']*nums['gatherRate'],'rightClickToGarrison':bool(nums['rightClickToGarrison']),
              'properties':assignments(ability(body,'properties')),'lineOfSight':assignments(ability(body,'LOS'))['range'],
              'build':{'epoch':int(tr['EPOCH']),'seconds':float(tr['TIME']),'host':tr['HOST'],
                       'costByOriginalId':[float(tr[n.upper()]) for n in NAMES],
                       'evidence':{'source':techpath,'line':line}},'evidence':proof})
        dropoffs=[]
        for file,name in [('city_center','CityCenter'),('warehouse','Warehouse'),('dock','Dock')]:
            entry=f'db/Units/{file}.ddf';text=src.read('db.zip',entry);body,proof=section(text,'UnitType',name,'db.zip!'+entry)
            drop=ability(body,'ResourceDropoff');gar=assignments(ability(body,'Garrison'))
            dropoffs.append({'unitType':name,'acceptedType':re.search(r'\btype\s*=\s*(\w+)',drop)[1],
                             'garrisonSlots':int(gar.get('numOfSlots',0)),'bonusPerOccupiedSlot':gar.get('resourceBonus',0),'evidence':proof})
        evidence=[]
        research=ROOT.parent/'EE2-Research/analysis'
        for filename in ('resource-initial.c','resource-gather-runtime.c','resource-special-runtime.c','resource-limits-runtime.c'):
            path=research/filename
            if path.exists():evidence.append({'file':str(path),'sha256':SHA(path.read_bytes()),'functions':re.findall(r'ADDRESS: ([0-9a-f]+)',path.read_text(encoding='utf8'))})
        result={'format':'ee2-base-economy-v1','profile':'Explicit BASE db.zip + hdrs.zip. No patch archive precedence inferred.',
          'sourceArchives':src.archives,'sources':src.records,
          'naturalResources':natural,'nonNaturalTypes':{'TechPoints':values['kResourceType_Tech'],'PopCap':values['kResourceType_Pop']},
          'citizen':{'unitType':'FullCitizen','gatherFarm':gather,'evidence':ref},'farm':farm,'resourceNodes':nodes,'productionStructures':structures,
          'dropoffs':dropoffs,'startingResourceLevels':levels,
          'runtime':{'enginePath':str(engine),'engineSha256':SHA(pe.raw),
             'engineProfile':'Installed Unofficial Patch Files/EnabledUP15Units/EE2.exe, preserved as work/EE2-engine.exe. Not asserted to be a vanilla binary.',
             'ghidraVersion':'12.2','readOnly':True,'analysisFiles':evidence,
             'stock':{'minimum':pe.floating(0xb4038c),'maximum':pe.floating(0xb413a4),'capAddress':'00b413a4','playerArrayOffset':'0x9c','itemBytes':4,'functions':['007ef6a0','007e7cf0']},
             'delivery':{'function':'007ef6a0','resourceBonusesFunction':'00857080','garrisonBonusFunction':'007f8aa0','result':'At dropoff, credit carried resource times type-specific multiplier; clamp stock; clear carry amount. Multiplier includes occupied garrison slots times resourceBonus, added to the base effect modifier.'},
             'resourceNodeInitialization':{'function':'007f86b0','result':'Initialize per-unit reserve as float from the configured Resource amount; worker counters and covered flag begin at zero.'},
             'garrisonGeneration':{'function':'007e7cf0','result':'Rate times occupied slots, with resource factor and fractional accumulation. Credit floor(accumulation) and retain remainder; no citizen haul trip.'},
             'specialResourceEligibility':{'function':'00884710','tableBase':'00b5526c','callers':['007eedd0','007ef6a0'],'result':'Return special UI slot1/2 for active epoch interval, -1 otherwise. Not a resource conversion.'}},
          'limitations':['Database values are base db.zip only; installed patch overlays were deliberately not selected.',
             'Runtime code evidence comes from the already analyzed installed enabled-UP engine; equivalence to every retail/patch version is not asserted.',
             'alwaysExhaustible null means absent in that DDF declaration; engine defaults/global finite-resource settings are not established here.',
             'Special-resource stock cleanup on epoch transition is not established here; active gather/delivery windows are established.',
             'Resource modifiers, taxes, diplomacy, crown/technology effects are not all reconstructed. Rates are base values.',
             'No simulator or renderer code is changed by this extractor.']}
        out=ROOT/'docs/ee2-economy.json';out.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
        print(f'Wrote {out}: {len(natural)} natural resources, {len(nodes)} node classes, {len(structures)} extraction buildings, {len(levels)} starting presets')
        return result
    finally:src.close()

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--install',type=pathlib.Path,default=INSTALL)
    ap.add_argument('--engine',type=pathlib.Path,default=ROOT.parent.parent/'work/EE2-engine.exe')
    args=ap.parse_args();run(args.install,args.engine)
