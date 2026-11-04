"""Isolated native family audio6 export; no active project/media changes."""
from pathlib import Path,PurePosixPath
import argparse,csv,hashlib,json,re,sys,time,zipfile
HERE=Path(__file__).resolve().parent
PROJECT=HERE.parent if (HERE.parent/'docs').is_dir() else HERE.parents[1]/'outputs/OpenEpoch'
ROOT=PROJECT.parents[1]
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--resolved',type=Path,default=ROOT/'work/up-resolved-sources/effective-db-sources.json');parser.add_argument('--install',type=Path,default=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II'));parser.add_argument('--family',action='append',help='Repeat ID:SourceFamily to export another explicit typed unit catalog instead of the four defaults');parser.add_argument('--out',type=Path,default=(PROJECT/'docs/local-original/audio' if HERE==PROJECT/'tools' else HERE/'docs/local-original/audio'));parser.add_argument('--cache-family',action='append',default=[],help='Additional exact source sound-cache family without a runtime definition ID');parser.add_argument('--media-subdir',default='family6');parser.add_argument('--manifest-name',default='native-families.json');args=parser.parse_args()
sys.path.insert(0,str(PROJECT/'tools'))
from resolved_archive_reader import ResolvedArchiveReader
from import_ee2 import without_comments,block
RESOLVED=args.resolved;INSTALL=args.install;OUT=args.out;OUT.mkdir(parents=True,exist_ok=True)
FAMILIES={57:'Scout',58:'FightingSail',59:'AirDefense',60:'CoastalDefense'}
if args.family or args.cache_family:
 FAMILIES={}
 for text in args.family or []:
  index,family=text.split(':',1);index=int(index)
  if not 57<=index<=65535 or index in FAMILIES or not re.fullmatch(r'\w+',family):raise ValueError('Invalid explicit unit ID:family binding')
  FAMILIES[index]=family
sha=lambda b:hashlib.sha256(b).hexdigest()

def csv_rows(raw):
 lines=raw.decode('cp1252').replace('\r','').splitlines();header=next(csv.reader([lines[0].removeprefix('//').strip()]));result=[]
 for i,line in enumerate(lines[1:],2):
  if line.strip() and not line.lstrip().startswith('//'):result.append((i,dict(zip(header,next(csv.reader([line]))))))
 return result

def stamp(info):
 y,m,d,h,minute,s=info.date_time
 if m>4 or(m==4 and d>=5):h-=1
 return int(time.mktime((y,m,d,h,minute,s,0,0,0)))&0xffffffff

def resolve_media(names,profile):
 wanted={x.casefold() for x in names};records={};chains={};payloads={};loose={}
 def add(path,source,ts,size):
  key=path.casefold();base=PurePosixPath(path).name.casefold()
  if key in records:
   previous=records[key]
   if previous['initialRecordTime']<=((ts+1)&0xffffffff):previous.update(source=source,path=path)
  else:records[key]={'path':path,'source':source,'initialRecordTime':ts,'initialRecordSize':size};chains.setdefault(base,[]).append(key)
 # Inspect only authored audio filenames, never unrelated user files.
 for p in INSTALL.rglob('*.mp3'):
  if p.name.casefold() not in wanted:continue
  relative=p.relative_to(INSTALL).as_posix()
  if relative.casefold().startswith(('ee2x_db/','zips/ee2x/')):continue
  add(relative,{'kind':'loose','physical':relative},int(p.stat().st_mtime)&0xffffffff,p.stat().st_size);loose.setdefault(p.name.casefold(),[]).append(relative)
 for priority,a in enumerate(profile['archiveOrder']):
  if a['status'].startswith('conditional'):continue
  p=INSTALL/a['payload']
  with p.open('rb') as stream,zipfile.ZipFile(stream) as z:
   for info in z.infolist():
    if info.is_dir() or PurePosixPath(info.filename).name.casefold() not in wanted:continue
    name=info.filename.replace('\\','/');add(name,{'kind':'zip-member','physical':a['payload'],'marker':a['marker'],'member':info.filename,'archivePriority':priority,'memberTime':stamp(info)},stamp(info),info.file_size)
 for name in wanted:
  if len(loose.get(name,[]))>1:raise ValueError('Ambiguous loose basename '+name)
  if name not in chains:raise ValueError('Missing original audio '+name)
  record=records[chains[name][0]];source=record['source'];p=INSTALL/source['physical']
  if source['kind']=='loose':raw=p.read_bytes()
  else:
   with p.open('rb') as stream,zipfile.ZipFile(stream) as z:raw=z.read(source['member'])
   if source['physical'] not in payloads:
    payloads[source['physical']]=sha(p.read_bytes());assert payloads[source['physical']]==profile['sources'][source['physical']]['sha256'],'Archive changed since resolution'
   source['archiveSha256']=payloads[source['physical']]
  source.update(bytes=len(raw),sha256=sha(raw),virtualPath=record['path'],initialRecordTime=record['initialRecordTime'],initialRecordSize=record['initialRecordSize'])
  yield name,raw,source

r=ResolvedArchiveReader(RESOLVED);profile=json.loads(RESOLVED.read_text(encoding='utf8'));upgrade_name='db/TechTree/upgrade_unittype5.csv';sound_name='db/Audio/dbsoun5.csv';upraw=r.read(upgrade_name);soundraw=r.read(sound_name)
upgrades=csv_rows(upraw);sounds={}
for line,row in csv_rows(soundraw):
 key=row['NAME'].casefold();assert key not in sounds,'Duplicate audio event';sounds[key]=(line,row)
# Follow exactly CFG5 declarations; unrelated UnitTypes with similar names are not used.
cfg='db/dbunitfile5.cfg';cfgraw=r.read(cfg);paths=re.findall(r'(?m)^\s*"?([^"\s]+\.ddf)"?\s*$',without_comments(cfgraw.decode('cp1252').replace('\r','')));decls={}
for path in paths:
 raw=r.read(path);text=without_comments(raw.decode('cp1252').replace('\r',''))
 for match in re.finditer(r'(?m)^\s*UnitType\s+(\w+)\s*\{',text):
  family=match[1];body=block(text,'UnitType',family)
  if body:assert family not in decls;decls[family]={'source':r.origin(path),'body':body}
def building_family(family,seen=None):
 seen=set() if seen is None else seen
 if family in seen:raise ValueError('Cyclic native UnitType parents')
 seen.add(family)
 if family in ['BareBoneBuilding','NoProduceBuilding','Building']:return True
 body=decls.get(family,{}).get('body','');parent=re.search(r'\bparent\s*=\s*(\w+)',body)
 return bool(parent and building_family(parent[1],seen))
selected={};states={}
for family in dict.fromkeys([*FAMILIES.values(),*args.cache_family]):
 if not re.fullmatch(r'\w+',family):raise ValueError('Invalid cache family')
 rows=[(line,row) for line,row in upgrades if row['UNIT']==family];rows.sort(key=lambda x:int(x[1]['EPOCH']));prior={};intervals=[]
 for index,(line,row) in enumerate(rows):
  epoch=int(row['EPOCH'])-1;until=int(rows[index+1][1]['EPOCH'])-1 if index+1<len(rows) else 15;native={};effective={};event_source={}
  for action,column in [('select','SNDSEL'),('order','SNDMOVE'),('attack','SNDATTACK')]:
   names=re.findall(r'\b\w+\b',row[column]);native[action]=names
   # 007C7FD0 only writes if arrayCount>0 and at least one event resolves.
   if names:prior[action]={'events':names,'line':line,'upgrade':row['UPGRADE']}
   previous=prior.get(action,{'events':[],'line':None,'upgrade':None});effective[action]=previous['events'];event_source[action]={'line':previous['line'],'upgrade':previous['upgrade'],'carriedForwardFromPriorUpgrade':not bool(names)}
   for event in previous['events']:
    assert event.casefold() in sounds,event
    sl,sound=sounds[event.casefold()];selected[event]={'file':sound['FILENAME'],'volume':float(sound['VOLUME']),'loop':sound['LOOP']=='1','maxInstances':int(sound['MAX_INSTANCES']),'pitchVariation':float(sound['PITCH_VARIATION']),'sourceEvent':event,'sourceLine':sl}
  intervals.append({'epoch':epoch,'until':until,'sourceEpoch':epoch+1,'sourceVisual':row['VISUAL'],'sourceUpgrade':row['UPGRADE'],'sourceLine':line,'rawEvents':native,'effectiveEvents':effective,'effectiveEventSources':event_source})
 states[family]={'definitionIds':[key for key,value in FAMILIES.items() if value==family],'role':'selection-effect' if building_family(family) else 'voice','declarationSource':decls[family]['source'],'intervals':intervals,'buildResponse':'No native build command channel in DbUnitTypeSounds; no build recording invented.'}
media={name:(raw,source) for name,raw,source in resolve_media([x['file'] for x in selected.values()],profile)};assets={}
for event,record in selected.items():
 raw,source=media[record['file'].casefold()];nativeName=PurePosixPath(source.get('member',source['physical'])).name;file=args.media_subdir+'/'+nativeName;target=OUT/file;target.parent.mkdir(exist_ok=True)
 if target.exists():assert sha(target.read_bytes())==sha(raw)
 else:target.write_bytes(raw)
 record['file']=file;record['sha256']=sha(raw);record['source']={**source,'eventTable':sound_name,'eventTableSha256':sha(soundraw)};assets[file]={'file':file,'sha256':sha(raw),'bytes':len(raw),'source':source,'transform':'byte-for-byte original recording copy'}
manifest={'format':'ee2-native-family-audio-v1','profile':'up15-csv5','sourceManifestSha256':sha(RESOLVED.read_bytes()),'sourceTables':{'upgrade':r.origin(upgrade_name),'sound':r.origin(sound_name),'cfg':r.origin(cfg)},'setterEvidence':{'function':'007C7FD0','caller':'0053C930','emptyArrays':'Preserve prior set: count<=0 performs no write.','maxEvents':8,'nativeChannels':['selection','move','attack','death']},'families':states,'events':selected,'assets':assets,'limits':['Existing57-role audio manifest is not changed.','This profile explicitly selects CSV5; no CSV6 or BASE event table is merged.','Native event arrays/carry-forward and recording bytes are source-backed. Playback scheduling/round-robin response selection remain application adapters.','These families have no native build-response channel; own fallback speech is kept separate.']}
(OUT/args.manifest_name).write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(json.dumps({'families':len(states),'intervals':sum(len(v['intervals']) for v in states.values()),'events':len(selected),'files':len(assets),'bytes':sum(x['bytes'] for x in assets.values()),'manifestSha256':sha((OUT/args.manifest_name).read_bytes())},indent=2))
