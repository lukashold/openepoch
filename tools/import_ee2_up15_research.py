"""Generate research rules from the explicit base database; never from decompiled code.
The unit-role map adapts EE2 factor targets to this engine's smaller roster.
Unsupported factors remain in the audit JSON, never replaced by invented buffs.
"""
import csv, hashlib, json, pathlib, re, zipfile
from import_ee2 import without_comments, block
from up15_source_database import ActiveDatabase,default_manifest
from up15_research_targets import selectors
MANIFEST=default_manifest(pathlib.Path(__file__).resolve().parents[1])
ROOT=pathlib.Path(__file__).resolve().parents[1]
HOST={'CityCenter':0,'Barracks':3,'Dock':5,'Airport':6,'University':47,'Mill':48,'Stable':49,'Market':50,'Temple':51,'Warehouse':52}
# Explicit adapters to the current roster, not an assertion of complete unit equivalence.
ROLES={
 'Citizens':[8], 'TT_CitizenScout':[8], 'TT_CityCenterHouse':[0,1],
 'CityCenters':[0], 'House':[1], 'CityCenterWarehouse':[0,52],
 'LightInfantry':[10,12,15,16], 'HeavyInfantry1':[9,11,13,14],
 'HeavyInfantry2':[17], 'HeavyInfantryAll':[9,11,13,14,17],
 'LightMounted1':[18,20,21], 'HeavyMounted1':[19], 'LightArtillery1':[22,24,27],
 'HeavyArtillery':[24,27,28], 'LightTank':[23,25], 'HeavyTank':[26,46],
 'FortressOutpostWallTower':[7,56], 'TT_Outpost':[7], 'WallTower':[7],
 'TempleUniversityWonder':[47,51], 'TempleUniversity':[47,51],
 'Universities':[47], 'Temples':[51], 'Markets':[50],
 'BarracksStableMill':[3,48,49], 'Naval':list(range(29,37))+[42,45],
 'NavalUnits':list(range(29,37))+[42,45], 'Battleships':[33],
 'NavalUnitsAttack':list(range(29,37)), 'NavalUnitsHP':list(range(29,37))+[42],
 'NavalUnitsEpoch6':[29], 'NavalUnitsAttackEpoch8':[30,31],
 'TT_BasicGathering':[8],
}
UNITS={'CityCenter':[0], 'House':[1], 'Citizen':[8], 'University':[47],
 'Warehouse':[52], 'Barracks':[3], 'Stable':[49], 'Mill':[48], 'Dock':[5],
 'Market':[50], 'Temple':[51], 'Airport':[6], 'Outpost':[7],
 'Fighter':[37,39], 'Bomber':[38,40], 'Destroyer':[32], 'Battleship':[33],
 'Submarine':[34,36], 'LightTank':[23], 'HeavyTank':[26],
 'OilRig':[53], 'UraniumMine':[54], 'Fortress':[56]}
FIELDS={'HitPoints':0,'Damage':1,'Speed':2,'BuildTime':3,'Cost':4,'LOS':5,
 'Range':6,'GatherTech':7,'GarrisonSlots':11,'PopCap':18}
GATHER_FIELDS={'Food':8,'Wood':9,'Iron':10,'Stone':12,'Gold':13,
 'Tin':14,'Saltpeter':15,'Oil':16,'Uranium':17}
for resource,index in GATHER_FIELDS.items():
 FIELDS['Gather'+resource]=index
 FIELDS['Gather'+resource+'_NoTooltip']=index
TOOLTIP_ONLY={'GatherBasic_Tooltip','GatherSpecial_Tooltip'}
METHODS={'AddPermMul':0,'Add':1,'Set':2}
FACTOR_LABELS={'HitPoints':'Lebenspunkte','Damage':'Schaden','Speed':'Tempo','BuildTime':'Bauzeit','Cost':'Kosten','LOS':'Sichtweite','Range':'Reichweite','GatherTech':'Forschungsertrag','GarrisonSlots':'Plätze','PopCap':'Bevölkerungslimit'}
GATHER_LABELS={'Food':'Nahrungsertrag','Wood':'Holzertrag','Iron':'Eisenertrag',
 'Stone':'Steinertrag','Gold':'Goldertrag','Tin':'Zinnertrag','Saltpeter':'Salpeterertrag',
 'Oil':'Ölertrag','Uranium':'Uranertrag'}
for resource,label in GATHER_LABELS.items():
 FACTOR_LABELS['Gather'+resource]=label
 FACTOR_LABELS['Gather'+resource+'_NoTooltip']=label
ROLE_LABELS={'Citizens':'Bürger','TT_CitizenScout':'Bürger','TT_CityCenterHouse':'Stadtzentrum und Häuser','LightInfantry':'Leichte Infanterie','HeavyInfantry1':'Schwere Infanterie','HeavyInfantry2':'Schwere Infanterie','HeavyInfantryAll':'Schwere Infanterie','LightMounted1':'Leichte Reiterei','HeavyMounted1':'Schwere Reiterei','LightArtillery1':'Leichte Artillerie','HeavyArtillery':'Schwere Artillerie','LightTank':'Leichte Panzer','HeavyTank':'Schwere Panzer','Universities':'Universitäten','TT_BasicGathering':'Bürger','Naval':'Schiffe','NavalUnits':'Schiffe','TempleUniversityWonder':'Tempel und Universitäten','TempleUniversity':'Tempel und Universitäten','CityCenterWarehouse':'Stadtzentren und Lagerhäuser','BarracksStableMill':'Kaserne, Stall und Mühle','FortressOutpostWallTower':'Wachtürme','TT_Outpost':'Wachtürme','NavalUnitsAttack':'Kriegsschiffe','NavalUnitsHP':'Schiffe'}
ROLE_LABELS.update({'House':'Häuser','OilRig':'Ölbohrturm','UraniumMine':'Uranmine',
 'Fortress':'Festungen','FortressOutpostWallTower':'Festungen und Wachtürme'})

def run():
 db=ActiveDatabase(MANIFEST)
 base=json.loads((ROOT/'docs/ee2-catalog.json').read_text(encoding='utf8'))
 base_nodes={n['NAME']:n for n in base['nodes']}
 nodes=db.table('db/techtree/dbtechtreenod5.csv')
 for n in nodes:
  for key in ['EPOCH','SLOT','MENU','ROW','COLUMN','TIME','TECHPTS','FOOD','WOOD','IRON','STONE','GOLD','TIN','SALTPETER','OIL','URANIUM']:
   value=float(n[key] or 0);n[key]=int(value) if value.is_integer() else value
  old=base_nodes.get(n['NAME'],{})
  same_token=old.get('DISPLAYNAME')==n['DISPLAYNAME']
  n['label']=old.get('label',n['DISPLAYNAME'] or n['NAME']) if same_token else n['DISPLAYNAME'] or n['NAME']
  n['label_scope']='Existing BASE label only when native display token is unchanged; otherwise token retained for separate localization resolver.'
  n['source']=n['_source']
 c={'nodes':nodes,'epochs':[{'id':i,'name':base['epochs'][i-1]['name'],'advance':next(n for n in nodes if n['NAME']==f'Main{i}')} for i in range(1,16)]}
 (ROOT/'docs/ee2-up15-catalog.json').write_text(json.dumps(c,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
 p='db/techtree/upgrade_factorse5.csv';raw=db.raw(p);factor_sha256=hashlib.sha256(raw).hexdigest()
 rows=db.table(p)
 for row in rows:row['source']=row['_source'];row['source_sha256']=factor_sha256
 factors={r['']:r for r in rows}
 unit_audit=json.loads((ROOT/'docs/ee2-up15-units.json').read_text(encoding='utf8'))
 role_targets,unit_targets,target_audit=selectors(db,unit_audit['bindings'])
 texts='\n'.join(without_comments(db.read(n)) for n in db.active_ddfs if '/techtree/' in n.lower())
 nodes=sorted([n for n in c['nodes'] if n['BRANCH'] in ('Economic','Military','Imperial')],key=lambda n:(n['EPOCH'],['Economic','Military','Imperial'].index(n['BRANCH']),n['SLOT']))
 quote=lambda x:json.dumps(x,ensure_ascii=False)
 out=['// Generated by tools/import_ee2_up15_research.py. Resolved active UP15 CSV5/CFG5.',
 'pub struct Node { pub name: &\'static str, pub note: &\'static str, pub epoch: u8, pub branch: u8, pub host: u8, pub cost: f32, pub time: f32, pub effects: &\'static [Effect] }',
 '#[derive(Clone, Copy)] pub struct Effect { pub mask: u64, pub field: u8, pub method: u8, pub value: f32 }',
 'pub const NODES: [Node; 180] = [']
 audit=[]
 for n in nodes:
  refs=[]
  def visit(name,seen):
   if name in seen:return
   seen.add(name)
   if name in factors:refs.append(factors[name]);return
   b=block(texts,'UpgradeRefSet',name)
   m=re.search(r'upgrades\s*=\s*\[([^\]]*)\]',b)
   if m:
    for ref in re.findall(r'\b\w+\b',m[1]):visit(ref,seen)
  visit(n['UPGRADE'],set())
  effects=[]; adapted=[]; skipped=[];notes=[];effect_audit=[];tooltip_only=[];deduplicated=[]
  for r in refs:
   if r['FACTOR'] in TOOLTIP_ONLY:
    tooltip_only.append(r);continue
   targets=unit_targets.get(r['UNIT'],[]) if r['UNIT'] else role_targets.get(r['UNIT TYPE ATTRIBUTE'].strip(),[])
   if targets and r['FACTOR'] in FIELDS and r['METHOD'] in METHODS:
    # Hidden-tooltip variants still change gameplay; only identical target/
    # field/method/value effects deduplicate. Distinct resources never collapse.
    key=(tuple(targets),FIELDS[r['FACTOR']],METHODS[r['METHOD']],float(r['VALUE']))
    if key in adapted:deduplicated.append(r);continue
    adapted.append(key);mask=sum(1<<i for i in targets)
    effects.append(f'Effect{{mask:{mask},field:{key[1]},method:{key[2]},value:{key[3]}_f32}}')
    effect_audit.append({'factor_id':r[''],'factor':r['FACTOR'],'targets':targets,'mask':mask,
     'field':key[1],'method':key[2],'value':key[3],'source':r['source'],'source_sha256':factor_sha256})
    target=r['UNIT'] or r['UNIT TYPE ATTRIBUTE']
    notes.append(f'{ROLE_LABELS.get(target,target)}: {FACTOR_LABELS[r["FACTOR"]]} {float(r["VALUE"]):+g}{"%" if r["METHOD"]=="AddPermMul" else ""}')
   else:skipped.append(r)
  note=' · '.join(notes) if notes else 'Zählt für den Epochenwechsel. Zugehörige Spezialfähigkeit noch offen.'
  if notes and skipped:note+=' · Weitere Spezialeffekte noch offen.'
  out.append(f'Node{{name:{quote(n["label"])},note:{quote(note)},epoch:{n["EPOCH"]-1},branch:{["Economic","Military","Imperial"].index(n["BRANCH"])},host:{HOST[n["HOST"]]},cost:{float(n["TECHPTS"])}_f32,time:{float(n["TIME"])}_f32,effects:&[{",".join(effects)}]}},')
  audit.append({'id':n['NAME'],'label':n['label'],'host':n['HOST'],'source':n['source'],'display_token':n['DISPLAYNAME'],'upgrade_ref':n['UPGRADE'],'raw_node':n,
   'adapted_effects':len(effects),'effects':effect_audit,'tooltip_only_factors':tooltip_only,
   'deduplicated_factors':deduplicated,'unimplemented_factors':skipped})
 out+= ['];','pub const EPOCH_NAMES: [&str;15] = ['+','.join(quote(e['name']) for e in c['epochs'])+'];',
 'pub const ADVANCE_COST: [f32;15] = ['+','.join(str(float(e['advance']['TECHPTS'])) for e in c['epochs'])+'];']
 runtime_count=sum(x['adapted_effects'] for x in audit)
 unsupported_count=sum(len(x['unimplemented_factors']) for x in audit)
 tooltip_count=sum(len(x['tooltip_only_factors']) for x in audit)
 summary={'runtime_effects':runtime_count,'unsupported_factors':unsupported_count,
  'tooltip_only_factors':tooltip_count,'deduplicated_factors':sum(len(x['deduplicated_factors']) for x in audit)}
 data={'scope':'Resolved active UP15 CSV5/CFG5, 180 stable research nodes. Nine-resource fields and population capacity are distinct; role adapters and unsupported abilities remain explicit. Tooltip-only factors never create runtime effects.',
  'factor_source':db.reader.origin(p),'native_source_manifest':db.manifest_audit(),
  'fields':FIELDS,'target_selectors':target_audit,'unit_targets':unit_targets,'summary':summary,'nodes':audit}
 for filename,content in [(ROOT/'src/ee2_up15_research.rs','\n'.join(out)+'\n'),
                          (ROOT/'docs/ee2-up15-research.json',json.dumps(data,ensure_ascii=False,indent=2))]:
  temporary=filename.with_name(filename.name+'.tmp');temporary.write_text(content,encoding='utf8');temporary.replace(filename)
 db.close()
 print(f'{len(nodes)} nodes; {runtime_count} runtime effects; {unsupported_count} unsupported factors; {tooltip_count} tooltip-only factors, no extra effect')
if __name__=='__main__':run()
