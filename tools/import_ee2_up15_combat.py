"""Generate the explicit BASE EE2 RPS/upgrade matchup slice for the current roster."""
import argparse,csv,hashlib,json,pathlib,re,struct
from import_ee2_units import clean_comments,scalar,sections
from up15_source_database import ActiveDatabase,default_manifest
MANIFEST=default_manifest(pathlib.Path(__file__).resolve().parents[1])
ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
UNKNOWN=65535
normalize=lambda value:re.sub(r'[^a-z0-9]','',value.lower())
sha=lambda raw:hashlib.sha256(raw).hexdigest()
def source(archive,entry,raw,**extra):return {'archive':archive,'entry':entry,'sha256':sha(raw),**extra}

def collect(installation=INSTALL):
 db=ActiveDatabase(MANIFEST)
 try:
  def source(_archive,entry,raw,**extra):
   actual=db.paths.get(entry.lower(),entry);origin=db.reader.origin(actual)
   assert sha(raw)==origin['sha256']
   return {'archive':origin['physical'],'entry':origin.get('member',actual),'virtual_path':actual,
           'sha256':origin['sha256'],'origin':origin,**extra}
  enum_raw=db.reader.read('DbHdrs/DbUnitDefEnums.h');combat_header=db.reader.read('DbHdrs/DbCombatDefs.h')
  enum_text=clean_comments(enum_raw.decode('cp1252'))
  enum_body=re.search(r'\bENUM\s+eUnitRPS\s*\{(.*?)\}',enum_text,re.S)[1]
  enum_names=re.findall(r'\bkUnitRPS_(\w+)',enum_body);enum_names=enum_names[:enum_names.index('NumRPSTypes')]
  assert len(enum_names)==35 and len(set(enum_names))==35
  path='db/Simulation/dbcombat.csv';raw=db.zip.read(path);lines=raw.decode('cp1252').replace('\r','').splitlines()
  header=next(csv.reader([lines[2].removeprefix('//')]))[1:]
  assert [normalize(name) for name in header]==[normalize(name) for name in enum_names]
  matrix=[];matrix_sources=[]
  for line_number,line in enumerate(lines[3:],4):
   if not line.strip() or line.lstrip().startswith('//'):continue
   row=next(csv.reader([line]));assert normalize(row[0])==normalize(enum_names[len(matrix)])
   values=[int(value) for value in row[1:]];assert len(values)==35 and all(0<=value<=65535 for value in values)
   matrix.append(values);matrix_sources.append(source('db.zip',path,raw,line=line_number,defender=row[0]))
  assert len(matrix)==35
  enum_map={normalize(name):index for index,name in enumerate(enum_names)}
  upgrades=db.table('db/TechTree/upgrade_unittypes.csv')
  selected=[row for row in upgrades if row.get('CIV','All')=='All']
  by_class_epoch={};by_source_line={}
  for row in selected:
   key=(row['UNIT'],int(row['EPOCH']));by_class_epoch.setdefault(key,[]).append(row)
   by_source_line[row['_source']['line']]=row
  adjust_rows=db.table('db/Simulation/dbcombat_unittypeadjust.csv')
  referenced={name for row in adjust_rows for name in [row['ATTACKER']]+re.findall(r'\w+',row['DEFENDERS'])}
  upgrade_names=sorted({row['UPGRADE'] for row in upgrades}|referenced);upgrade_ids={name:index for index,name in enumerate(upgrade_names)}
  non_csv_names=sorted(referenced-{row['UPGRADE'] for row in upgrades})
  def non_csv_block(name):return db.block('UnitType',name) or db.block('UpgradeUnitTypeUpgrade',name)
  assert all(non_csv_block(name) for name in non_csv_names),non_csv_names
  adjust_pairs={};duplicate_pairs=[]
  for row in adjust_rows:
   assert row['ATTACKER'] in upgrade_ids
   defenders=re.findall(r'\w+',row['DEFENDERS']);value=int(row['ADJUST']);assert 0<=value<=65535
   for name in defenders:
    assert name in upgrade_ids
    key=(upgrade_ids[row['ATTACKER']],upgrade_ids[name])
    if key in adjust_pairs:duplicate_pairs.append({'key':key,'previous':adjust_pairs[key],'next':value})
    # 0052ffe0 appends in row order; 0052ff40 returns the FIRST equal pair.
    # Preserve later duplicates in audit, never reverse the native priority.
    adjust_pairs.setdefault(key,value)
  unit_audit_blob=(ROOT/'docs/ee2-up15-units.json').read_bytes();unit_audit=json.loads(unit_audit_blob)
  rps_cache={}
  def unit_rps(name,stack=()):
   if name in rps_cache:return rps_cache[name]
   assert name not in stack
   block=db.block('UnitType',name);assert block,'Unknown UnitType '+name
   parent=scalar(block['body'],'parent')
   inherited=unit_rps(parent,stack+(name,)) if parent else None
   own=None;ref=None
   for body,offset in sections(block['body'],'properties'):
    match=re.search(r'\brps\s*=\s*(\w+)',body,re.I)
    if match:own=match[1];ref=db.block_ref(block,offset+match.start())
   result=(enum_map[normalize(own)],own,[ref]) if own else inherited
   assert result,'No native RPS class for '+name
   rps_cache[name]=result;return result
  profiles=[];lookup=[[UNKNOWN]*15 for _ in range(57)]
  for native in unit_audit['class_rows']:
   source_lines={ref['line'] for field in native['fields'].values() for ref in field.get('sources',[]) if ref.get('virtual_path','').lower()=='db/techtree/upgrade_unittype5.csv'}
   candidates=[by_source_line[line] for line in source_lines if line in by_source_line and by_source_line[line]['UNIT']==native['source_unit'] and int(by_source_line[line]['EPOCH'])==native['source_epoch']]
   if not candidates:candidates=by_class_epoch.get((native['source_unit'],native['source_epoch']),[])
   assert len(candidates)<=1,('Ambiguous native source upgrade',native['id'],native['epoch'],source_lines)
   upgrade=candidates[0] if candidates else None
   if upgrade is None and native['source_unit']=='CitizenM':
    name=f"CitizenUpgradeEpoch{native['source_epoch']}"
    declaration=db.block('UpgradeUnitTypeUpgrade',name)
    assert declaration and scalar(declaration['body'],'UnitType')=='CitizenM'
    assert name in upgrade_ids
    upgrade={'UPGRADE':name,'_source':db.block_ref(declaration)}
   rps_index,rps_name,rps_sources=unit_rps(native['source_unit'])
   profile={'index':len(profiles),'id':native['id'],'epoch':native['epoch'],'until':native['until'],
    'sourceUnit':native['source_unit'],'sourceEpoch':native['source_epoch'],'upgrade':upgrade['UPGRADE'] if upgrade else None,'upgradeIndex':upgrade_ids[upgrade['UPGRADE']] if upgrade else UNKNOWN,
    'rps':rps_index,'rpsName':rps_name,'identityVerified':native['identity_verified'],'rpsSources':rps_sources,'upgradeSource':upgrade['_source'] if upgrade else None}
   for epoch in range(profile['epoch'],profile['until']):
    assert lookup[profile['id']][epoch]==UNKNOWN
    lookup[profile['id']][epoch]=profile['index']
   profiles.append(profile)
  return {
   'format':'ee2-native-combat-v1','scope':'Active CFG5/CSV5 resolved UP15 RPS table and upgrade matchup adjustments for the current57 definition mappings. Existing role aliases remain explicit; this does not reconstruct all native attack abilities, armor modifiers, elevation, formations, veterancy or civilization effects.',
   'orientation':'matrix[defenderRPS][attackerRPS]','percentScale':100,'unitAuditSha256':sha(unit_audit_blob),
   'sources':[source('hdrs.zip','DbHdrs/DbUnitDefEnums.h',enum_raw),source('hdrs.zip','DbHdrs/DbCombatDefs.h',combat_header),source('db.zip',path,raw),source('db.zip','db/Simulation/dbcombat_unittypeadjust.csv',db.zip.read('db/Simulation/dbcombat_unittypeadjust.csv')),source('db.zip','db/TechTree/upgrade_unittypes.csv',db.zip.read('db/TechTree/upgrade_unittypes.csv'))],
   'native_source_manifest':db.manifest_audit(),'source_origins':[db.reader.origin(n) for n in ['DbHdrs/DbUnitDefEnums.h','DbHdrs/DbCombatDefs.h','db/simulation/dbcomba5.csv','db/simulation/dbcombat_unittypeadjus5.csv','db/techtree/upgrade_unittype5.csv']],'rpsNames':enum_names,'matrix':matrix,'matrixRows':matrix_sources,'upgradeNames':upgrade_names,
   'adjustmentRows':adjust_rows,'adjustments':[[attacker,defender,value] for (attacker,defender),value in sorted(adjust_pairs.items())],
   'duplicateAdjustmentPairs':duplicate_pairs,'adjustmentPriority':'first matching source row (0052ffe0 append;0052ff40 first hit)',
   'referencedNonCsvUnitTypes':[{'name':name,'source':db.block_ref(non_csv_block(name))} for name in non_csv_names],
   'profiles':profiles,'lookup':lookup,
  }
 finally:db.close()

def generate(data):
 lines=['// Generated by tools/import_ee2_up15_combat.py. Resolved UP15/CSV5 profile; numerical data only.',
  '// Rows are defenders, columns are attackers. No full native combat equivalence claim.',
  'pub const UNKNOWN_PROFILE:u16=u16::MAX;',
  '#[derive(Clone,Copy,Debug)] pub struct CombatProfile { pub rps:u8,pub upgrade:u16 }',
  'pub const RPS_PERCENT:[[u16;35];35]=[']
 lines += [' ['+','.join(map(str,row))+'],' for row in data['matrix']];lines+= ['];',f"pub const PROFILES:[CombatProfile;{len(data['profiles'])}]=["]
 lines += [f" CombatProfile{{rps:{row['rps']},upgrade:{row['upgradeIndex']}}}, // {row['id']} E{row['epoch']+1}..{row['until']} {row['sourceUnit']} {row['upgrade']}" for row in data['profiles']]
 lines += ['];','pub const PROFILE_BY_DEF_EPOCH:[[u16;15];57]=[']
 lines += [' ['+','.join('UNKNOWN_PROFILE' if value==UNKNOWN else str(value) for value in row)+'],' for row in data['lookup']]
 lines += ['];',f"const ADJUSTMENTS:[(u32,u16);{len(data['adjustments'])}]=["]
 lines += [f' ({(attacker<<16)|defender},{value}),' for attacker,defender,value in data['adjustments']]
 lines += ['''];
#[inline] pub fn profile(id:u8,epoch:u8)->u16 {
 PROFILE_BY_DEF_EPOCH.get(id as usize).and_then(|row|row.get(epoch as usize)).copied().unwrap_or(UNKNOWN_PROFILE)
}
#[inline] pub fn base_percent(attacker_profile:u16,defender_profile:u16)->Option<u16> {
 let attacker=PROFILES.get(attacker_profile as usize)?;let defender=PROFILES.get(defender_profile as usize)?;
 Some(RPS_PERCENT[defender.rps as usize][attacker.rps as usize])
}
pub fn adjustment_percent(attacker_profile:u16,defender_profile:u16)->Option<u16> {
 let attacker=PROFILES.get(attacker_profile as usize)?;let defender=PROFILES.get(defender_profile as usize)?;
 let key=((attacker.upgrade as u32)<<16)|defender.upgrade as u32;
 Some(ADJUSTMENTS.binary_search_by_key(&key,|entry|entry.0).map(|index|ADJUSTMENTS[index].1).unwrap_or(100))
}
#[inline] pub fn multiplier(attacker_profile:u16,defender_profile:u16)->Option<f32> {
 let base=base_percent(attacker_profile,defender_profile)? as f32*0.01;
 Some(base*(adjustment_percent(attacker_profile,defender_profile)? as f32*0.01))
}
#[inline] pub fn multiplier_from_profile(attacker_profile:u16,defender_id:u8,defender_epoch:u8)->Option<f32> {
 multiplier(attacker_profile,profile(defender_id,defender_epoch))
}
#[inline] pub fn can_target(attacker_id:u8,attacker_epoch:u8,defender_id:u8,defender_epoch:u8)->Option<bool> {
 multiplier(profile(attacker_id,attacker_epoch),profile(defender_id,defender_epoch)).map(|value|value>0.0)
}
///007dfdf0: primary target retains full damage; other centers use squared falloff.
///The caller performs radius and domain/hostility eligibility first.
#[inline] pub fn splash_factor(distance:f32,radius:f32,primary:bool)->f32 {
 if !distance.is_finite()||!radius.is_finite()||radius<=0.0{return 0.0;}
 if primary{return 1.0;}
 let f=(1.0-distance.max(0.0)/radius).clamp(0.0,1.0);f*f
}

''']
 return '\n'.join(lines)

def run(installation=INSTALL):
 data=collect(installation)
 # A UnitType's inherited RPS exists before its production upgrade. Captured,
 # scenario or cheat-spawned future units must not lose known class eligibility
 # merely because their new owner's epoch precedes production availability.
 for identifier,lookup in enumerate(data['lookup']):
  absent=[epoch for epoch,value in enumerate(lookup) if value==UNKNOWN]
  if not absent:continue
  members=[p for p in data['profiles'] if p['id']==identifier]
  assert members and len({p['rps'] for p in members})==1
  original=members[0];index=len(data['profiles'])
  base={**original,'index':index,'epoch':min(absent),'until':max(absent)+1,'sourceEpoch':0,
    'upgrade':None,'upgradeIndex':UNKNOWN,'upgradeSource':None,'baseTypeBeforeProduction':True,
    'scope':'Inherited DDF UnitType RPS only; no unverified CSV upgrade identity before availability.'}
  data['profiles'].append(base)
  for epoch in absent:lookup[epoch]=index
 data['baseTypeScope']='Every bound source UnitType retains its proven inherited RPS before production availability; missing upgrade identity uses native default adjustment100.'
 targets=[(ROOT/'src/ee2_up15_combat.rs',generate(data)),(ROOT/'docs/ee2-up15-combat.json',json.dumps(data,indent=2)+'\n')]
 for path,text in targets:
  temporary=path.with_suffix(path.suffix+'.tmp');temporary.write_text(text,encoding='utf8');temporary.replace(path)
 print(json.dumps({'profiles':len(data['profiles']),'rpsTypes':len(data['rpsNames']),'matchups':35*35,'upgradeAdjustments':len(data['adjustments']),'duplicatePairs':len(data['duplicateAdjustmentPairs'])}))
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--install',type=pathlib.Path,default=INSTALL);run(parser.parse_args().install)
