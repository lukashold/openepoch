"""Offline static EE2 UP source resolver; never loads game code or changes originals.

Only DB/HDR text is extracted by default. Protected-part media are inventoried,
including the already present NTFS ADS ZIP; no decryption/key guessing is used.
The UP1.6 flag/game type are declared profile inputs, not observed runtime state.
"""
from pathlib import Path, PurePosixPath
import argparse, collections, datetime, hashlib, json, time, zipfile
import os

ROOT=Path(__file__).resolve().parents[1]
WORK=ROOT.parents[1]/'work'
parser=argparse.ArgumentParser()
parser.add_argument('--install',type=Path,default=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II'))
parser.add_argument('--out',type=Path,default=WORK/'up-resolved-sources')
parser.add_argument('--game',choices=['ee2','ee2x'],default='ee2')
parser.add_argument('--enhanced-terrain',action='store_true',help='Explicit conditional Part13 candidate; graphics-mode gates are recorded.')
args=parser.parse_args()
install=args.install.resolve();destination=args.out.resolve()
assert destination.is_relative_to(WORK.resolve()), 'Extract only inside workspace work/'
destination.mkdir(parents=True,exist_ok=True)
analysis=ROOT.parent/'EE2-Research/analysis'
engine_sha='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f'
helper_sha='1315fa25077004e2c35cf0e731f3dd33757e8ed054a76dc35e4b0c6eb5179e8e'
assert helper_sha in (analysis/'native-uphelper-protected-callback.c').read_text()
assert engine_sha in (analysis/'native-vfs-protected-hook.c').read_text()
assert '00a29af1' in (analysis/'native-vfs-protected-hook.c').read_text().lower()
assert 'UP15_Load_Protected_Zip' in (analysis/'native-uphelper-protected-callback.c').read_text()
source_suffixes={'.ddf','.csv','.cfg','.h','.hdr','.utf8'}

def digest_file(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(4*1024*1024),b''):h.update(chunk)
    return h.hexdigest()

def safe_name(name):
    name=name.replace('\\','/');p=PurePosixPath(name)
    if p.is_absolute() or any(x in ('..','.') or ':' in x or x.endswith((' ','.')) for x in p.parts):
        raise ValueError('Unsafe member path: '+name)
    return '/'.join(p.parts)

def selected(name):
    p=PurePosixPath(name);low=name.lower()
    return p.suffix.lower() in source_suffixes and (low.startswith(('db/','ee2x_db/','hdrs/')) or p.suffix.lower() in {'.h','.hdr'})

def member_time(info):
    # Exact DOS field-to-tm transformation from engine00A4E260; mktime's local
    # timezone is explicit in the report. No UTC interpretation of ZIP dates.
    y,m,d,h,minute,sec=info.date_time
    if m>4 or (m==4 and d>=5):h-=1
    return int(time.mktime((y,m,d,h,minute,sec,0,0,0))) & 0xffffffff

def extract(name,data):
    path=(destination/'files'/safe_name(name)).resolve()
    assert path.is_relative_to(destination/'files')
    path.parent.mkdir(parents=True,exist_ok=True)
    expected=hashlib.sha256(data).hexdigest()
    if not path.exists() or digest_file(path)!=expected:path.write_bytes(data)
    assert digest_file(path)==expected
    return str(path)

records={};chains=collections.defaultdict(list);overrides=[];sources={};unknown=[]
archive_inventory=[]
def add(name,source,stamp,size,data=None):
    name=safe_name(name);key=name.casefold();base=PurePosixPath(name).name.casefold()
    if key in records:
        record=records[key]
        wins=record['initialRecordTime']<=((stamp+1)&0xffffffff)
        overrides.append({'path':name,'earlierSource':record['source'],'candidateSource':source,'initialRecordTime':record['initialRecordTime'],'candidateTime':stamp,'selected':wins})
        if wins:record['source']=source;record['path']=name;record['selectedData']=data
    else:
        record={'path':name,'initialRecordTime':stamp,'initialRecordSize':size,'source':source,'selectedData':data,'insertionPriority':len(records)}
        records[key]=record;chains[base].append(key)

# CompleteScan precedes every ZIP. Only authored DB source folders are read:
# user config/license/network files outside those namespaces are not inspected.
loose=[]
for directory in ['zips','zips_ee2x','db','hdrs']:
    folder=install/directory
    if not folder.exists():continue
    for path in folder.rglob('*'):
        if not path.is_file() or path.suffix.lower() not in source_suffixes:continue
        relative=path.relative_to(install).as_posix()
        if relative.casefold().startswith(('ee2x_db/','zips/ee2x/')):continue
        data=path.read_bytes();stamp=int(path.stat().st_mtime)&0xffffffff
        source={'kind':'loose','physical':relative,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}
        add(relative,source,stamp,len(data),data);loose.append(relative)

part_root=install/'Unofficial Patch Files/EXEGeneratorData'
part10=part_root/('EE2XExeSourcePart10' if args.game=='ee2x' else 'EE2ExeSourcePart10')
redirects={
    'db_160_protected.zip':part10,
    'db_160_protected2.zip':part_root/'EE2EE2XExeSourcePart12',
    'db_158_protected.zip':part_root/'EE2EE2XExeSourcePart9:protected',
    'db_160_ultrawide_ui.zip':part_root/('EE2XExeSourcePart14' if args.game=='ee2x' else 'EE2ExeSourcePart14'),
}
if args.enhanced_terrain:redirects['textures_terrain_160.zip']=part_root/'EE2EE2XExeSourcePart13'
zip_paths=sorted((install/'zips').glob('*.zip'),key=lambda p:p.name.casefold())
for archive_priority,marker in enumerate(zip_paths):
    payload=redirects.get(marker.name.casefold(),marker)
    status='marker-redirect' if payload!=marker else 'physical-archive'
    try:
        # ADS existence must use open(), not GetFileAttributes/Path.exists().
        payload_handle=payload.open('rb');payload_handle.close()
    except FileNotFoundError:
        unknown.append({'marker':marker.name,'payload':str(payload),'status':'not-present-no-payload-assumption'})
        payload=marker;status='missing-redirect-original-marker'
    relative=str(payload.relative_to(install)).replace('\\','/')
    with payload.open('rb') as f:f.seek(0,2);payload_size=f.tell()
    sources[relative]={'bytes':payload_size,'sha256':digest_file(payload)}
    inventory={'marker':marker.relative_to(install).as_posix(),'payload':relative,'status':status,'members':[]}
    with payload.open('rb') as stream,zipfile.ZipFile(stream) as z:
        for info in z.infolist():
            name=safe_name(info.filename)
            item={'path':name,'bytes':info.file_size,'crc32':f'{info.CRC:08x}','dateTime':info.date_time,'encrypted':bool(info.flag_bits&1)}
            inventory['members'].append(item)
            if info.is_dir() or not selected(name):continue
            data=z.read(info);sha=hashlib.sha256(data).hexdigest();item['sha256']=sha
            source={'kind':'zip-member','physical':relative,'marker':marker.relative_to(install).as_posix(),'member':info.filename,'bytes':len(data),'sha256':sha,
                    'archivePriority':archive_priority,'hookSubstitution':payload!=marker,'memberTime':member_time(info)}
            add(name,source,member_time(info),len(data),data)
    inventory['memberCount']=len(inventory['members']);inventory['uncompressedBytes']=sum(r['bytes'] for r in inventory['members'])
    archive_inventory.append(inventory)

effective={}
for key,r in records.items():
    data=r.pop('selectedData');r['extracted']=extract(r['path'],data)
    r['staticNamespace']='case-insensitive-full-path';effective[key]=r
basenames={}
for name,keys in chains.items():
    loose_keys=[k for k in keys if effective[k]['source']['kind']=='loose']
    if len(loose_keys)>1:
        basenames[name]={'status':'ambiguous-physical-enumeration-order','candidates':keys}
    else:
        basenames[name]={'status':'resolved-first-basename-record','path':keys[0],'source':effective[keys[0]]['source'],'extracted':effective[keys[0]]['extracted']}
outside_loose=[];outside_zip=[]
for path in install.rglob('*'):
    if path.is_file() and path.name.casefold() in basenames:
        relative=path.relative_to(install).as_posix()
        if relative.casefold().startswith(('ee2x_db/','zips/ee2x/')):continue
        if relative.casefold() not in effective:outside_loose.append(relative)
for archive in archive_inventory:
    if archive['status'].startswith('conditional'):continue
    for entry in archive['members']:
        if PurePosixPath(entry['path']).name.casefold() in basenames and entry['path'].casefold() not in effective:
            outside_zip.append({'archive':archive['payload'],'path':entry['path']})
assert not outside_loose and not outside_zip, ('Unmodelled native basename collision',outside_loose,outside_zip)

comparisons=[]
for current,legacy in [('dbcomba5.csv','dbcombat.csv'),('dbcombat_unittypeadjus5.csv','dbcombat_unittypeadjust.csv'),('upgrade_unittype5.csv','upgrade_unittypes.csv'),('dbunitfile5.cfg','dbunitfiles.cfg')]:
    a,b=basenames.get(current),basenames.get(legacy)
    if a and b and 'source' in a and 'source' in b:comparisons.append({'current':current,'legacy':legacy,'currentSource':a['source'],'legacySource':b['source'],'identical':a['source']['sha256']==b['source']['sha256']})
# Inventory optional source parts and precisely named fallback files only; never
# enumerate unrelated user temp files. The automation sandbox TEMP changes per
# tool call and is not an original-game source; audit the known user-local path.
optional_inventory=[]
candidate_paths=[part_root/n for n in ['EE2EE2XExeSourcePart8','EE2EE2XExeSourcePart9:protected','EE2ExeSourcePart10','EE2XExeSourcePart10','EE2EE2XExeSourcePart12','EE2EE2XExeSourcePart13','EE2ExeSourcePart14','EE2ExeSourcePart14:protected','EE2XExeSourcePart14','EE2XExeSourcePart14:protected']]
temp_roots={Path(os.environ['USERPROFILE'])/'AppData/Local/Temp'}
for folder in sorted(temp_roots):
    candidate_paths.extend(folder/name for name in ['EE2up161.tmp','EE2up161.tmp:protected','EE2Xup161.tmp','EE2Xup161.tmp:protected'])
for path in candidate_paths:
    item={'path':str(path)}
    try:
        with path.open('rb') as f:f.seek(0,2);item['bytes']=f.tell();f.seek(0);item['header32']=f.read(32).hex()
    except FileNotFoundError:
        item['status']='not-present';optional_inventory.append(item);continue
    item['sha256']=digest_file(path)
    try:
        with path.open('rb') as f,zipfile.ZipFile(f) as z:
            es=z.infolist();item['status']='ordinary-zip';item['memberCount']=len(es);item['encryptedMembers']=sum(bool(e.flag_bits&1) for e in es)
            item['sourceTextMembers']=[e.filename for e in es if not e.is_dir() and selected(safe_name(e.filename))]
            item['prefixCounts']=dict(collections.Counter(e.filename.replace('\\','/').split('/')[0] for e in es))
            # Protected cache/audio source namespaces are explicit even when the
            # current text-only profile does not select their media contents.
            if path.name=='EE2EE2XExeSourcePart13' and not args.enhanced_terrain:
                archive_inventory.append({'marker':'conditional:textures_terrain_160.zip','payload':str(path.relative_to(install)).replace('\\','/'),'status':'conditional-available-not-selected',
                    'memberCount':len(es),'uncompressedBytes':sum(e.file_size for e in es),'members':[{'path':e.filename,'bytes':e.file_size,'crc32':f'{e.CRC:08x}','dateTime':e.date_time,'encrypted':bool(e.flag_bits&1)} for e in es]})
    except zipfile.BadZipFile:item['status']='opaque-non-zip-no-decryption-attempted'
    optional_inventory.append(item)
report={'format':'ee2-up-static-source-resolution-v1',
 'profile':{'game':args.game,'up16Enabled':True,'enhancedTerrainRequested':args.enhanced_terrain,'physicalRoot':str(install),'auditedDefaultUserTemp':str(next(iter(temp_roots))),
 'scope':'Explicit static UP1.6 profile. Engine/helper are never executed. Config/license/CDKey/network data are outside extraction scope.'},
 'engineSha256':engine_sha,'helperSha256':helper_sha,'timezone':{'names':time.tzname,'secondsWestUTC':time.timezone},
 'priorityEvidence':'native-vfs-order-evidence.json + native-vfs-protected-hook.c + native-uphelper-protected-callback.c',
 'conditions':['Initial physical scan root is installation cwd; namespaces below that root are used.','UP1.6-enabled profile flag and selected EE2/EE2X game type are explicit inputs, not observed process state.','Part13 is only included when --enhanced-terrain is explicitly selected; native graphics-mode gating remains a declared condition.','Part14 sources/ADS and specifically named161 temporary files in the default user-local Windows temp are audited; an unobserved original-process custom TEMP is not asserted absent.','Initial helper memory patch tables are not used as invented file namespace aliases.','For duplicate full paths initial size/time are retained exactly as engine00A29BD0 shows; selected member bytes are read offline, without reproducing native allocation behavior.'],
 'looseFiles':loose,'archiveOrder':[{k:r[k] for k in ['marker','payload','status','memberCount','uncompressedBytes']} for r in archive_inventory if not r['status'].startswith('conditional')],
 'sources':sources,'effectiveByPath':effective,'effectiveByBasename':basenames,'overrides':overrides,'optionalSourceInventory':optional_inventory,
 'version5SourceComparison':comparisons,'optionalPayloadsNotPresent':unknown,
 'counts':{'fullPaths':len(effective),'basenames':len(basenames),'ambiguousBasenames':sum(v['status'].startswith('ambiguous') for v in basenames.values()),'archives':len(zip_paths),'overridesConsidered':len(overrides),'unmodelledLooseCollisions':len(outside_loose),'unmodelledZipCollisions':len(outside_zip)},
 'originalsReadOnly':True,'dllOrExeExecuted':False,'mediaExtracted':False}
serialized=json.dumps(report,indent=2)+'\n'
(destination/'manifest.json').write_text(serialized,encoding='utf8')
(destination/'effective-db-sources.json').write_text(serialized,encoding='utf8')
(destination/'protected-payload-inventory.json').write_text(json.dumps({'sources':sources,'archives':archive_inventory},indent=2)+'\n',encoding='utf8')
print(json.dumps({'status':'resolved-declared-static-profile','manifest':str(destination/'manifest.json'),'counts':report['counts'],'version5Comparisons':comparisons,'optionalPayloadsNotPresent':unknown},indent=2))
