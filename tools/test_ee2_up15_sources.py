"""Independent raw CSV/DDF oracles for the isolated UP profile (no importer helpers)."""
from pathlib import Path
import csv,hashlib,json,re,struct
ROOT=Path(__file__).resolve().parents[1]
_manifests=sorted({p.resolve() for a in [ROOT,*ROOT.parents] for rel in ['up-resolved-sources/effective-db-sources.json','work/up-resolved-sources/effective-db-sources.json'] if(p:=a/rel).is_file()})
assert len(_manifests)==1,'exactly one prepared source manifest required'
MANIFEST=_manifests[0]
checks=0
def check(condition,message):
    global checks
    assert condition,message
    checks+=1

def run(prefix="ee2-up15"):
    m=json.loads(MANIFEST.read_text());paths=m['effectiveByPath']
    def raw(path):
        key=path.lower();record=paths[key];data=Path(record['extracted']).read_bytes()
        check(hashlib.sha256(data).hexdigest()==record['source']['sha256'],'source SHA '+path)
        return data
    def text(path):
        b=raw(path)
        try:return b.decode('utf-8-sig').replace('\r','')
        except UnicodeDecodeError:return b.decode('cp1252').replace('\r','')
    def table(path,headerline=0):
        lines=text(path).splitlines();header=next(csv.reader([lines[headerline].removeprefix('//').strip()]))
        seen=set()
        for i,h in enumerate(header):
            if h in seen:header[i]+='_PLURAL'
            seen.add(h)
        rows=[]
        for i,l in enumerate(lines[headerline+1:],headerline+2):
            if not l.strip() or l.lstrip().startswith('//'):continue
            values=next(csv.reader([l]));check(len(values)==len(header),f'CSV width {path}:{i}')
            rows.append((dict(zip(header,values)),i))
        return rows
    units=json.loads((ROOT/('docs/'+prefix+'-units.json')).read_text(encoding='utf8'))
    combat=json.loads((ROOT/('docs/'+prefix+'-combat.json')).read_text(encoding='utf8'))
    research=json.loads((ROOT/('docs/'+prefix+'-research.json')).read_text(encoding='utf8'))
    active=[]
    for cfg in ['db/dbunitfile5.cfg','db/techtree/dbepochupgrade5.cfg']:
        for line in text(cfg).splitlines():
            path=line.split('//')[0].strip().strip('"').lower()
            if path and path not in active:active.append(path)
    check(active==[p['path'].lower() for p in units['native_source_manifest']['active_ddfs']],'exact native CFG5 active list')
    check(len(active)==158,'active DDF count')
    upgrades={r['UPGRADE']:(r,line) for r,line in table('db/techtree/upgrade_unittype5.csv')}
    source_units=[];literal_checks=[]
    fieldmap={'hp':'HP','buildtime':'BUILDTIME','sight':'LOS','damage':'DAMAGE','range':'RANGE','reload':'RELOAD'}
    resource=['FOOD','WOOD','IRON','STONE','GOLD','TIN','SALTPETER','OIL','URANIUM']
    for row in units['class_rows']:
        upgrade=row.get('source_upgrade')
        if not upgrade:continue
        native,line=upgrades[upgrade['UPGRADE']]
        for field,column in fieldmap.items():
            numeric=float(native[column]);actual=row['fields'][field]['value']
            if numeric:check(actual==numeric,f'unit {row["id"]} E{row["epoch"]} {field}')
            elif actual:
                refs=row['fields'][field]['sources'];check(bool(refs),'zero CSV fallback retains source')
                found=False
                for ref in refs:
                    key=ref['virtual_path'].lower();lines=text(key).splitlines();source_line=lines[ref['line']-1]
                    keyname={'hp':'hitpoints','sight':'range','damage':'damage','range':'range','reload':'reloadTime','buildtime':'buildTime'}[field]
                    values=re.findall(r'\b'+keyname+r'\s*=\s*([+\-]?[0-9.]+)',source_line,re.I)
                    if any(float(v)==actual for v in values):found=True
                check(found,'raw DDF literal backs zero-CSV override '+field)
            else:check(actual in [None,0],'zero scalar remains absent/zero without ref override')
        costs=[int(native[r]) for r in resource]
        check(row['fields']['cost9']['value']==costs,'nine independent native cost slots')
        check(row['display_tokens']['DISPLAYNAME']==native['DISPLAYNAME'],'singular display token')
        check(row['display_tokens']['DISPLAYNAME_PLURAL']==native['DISPLAYNAME_PLURAL'],'plural display token')
        source_units.append((row['id'],row['epoch'],*[float(row['fields'][f]['value'] or 0) for f in ['hp','sight','damage','range','reload','buildtime']],costs))
        for value in row['fields'].values():
            for ref in value.get('sources',[]):
                key=ref.get('virtual_path','').lower()
                check(key in paths and paths[key]['source']['sha256']==ref['sha256'],'every field origin hash')
                check(not key.endswith('.ddf') or key in active,'no inactive DDF field')
    # Independent literal tests for cross-named actual UpgradeRefs.
    mounted=text('db/units/heavy_mounted1.ddf')
    start=mounted.index('UpgradeAbilities HeavyMounted1Epoch15Move')
    body=mounted[start:mounted.index('}',mounted.index('{',start))+1]
    speed=float(re.search(r'VehicleMove\s*\{[^}]*\bspeed\s*=\s*([0-9.]+)',body,re.S)[1])
    heavy=next(r for r in units['class_rows'] if r['id']==26 and r['epoch']==14)
    check(heavy['source_upgrade']['UNIT']=='HeavyTank','cross-named native parent')
    check('HeavyMounted1Epoch15Move' in heavy['source_upgrade']['UPGRADEREFS'],'actual cross-named ref')
    check(heavy['fields']['raw_speed']['value']==speed==2.3,'actual VehicleMove ref replaces root speed')
    literal_checks.append(f'assert_eq!(u::lookup_class(26,14).unwrap().raw_speed,Some({speed}_f32));')
    citizen=text('db/units/citizen_15.ddf')
    for row in [r for r in units['class_rows'] if r['id']==8]:
        begin=citizen.index('UpgradeSet IntCitizenUpgradeEpoch'+str(row['source_epoch']))
        following=citizen[begin:];body=following.split('\n}',1)[0]
        expected=float(re.search(r'factor\s*=\s*Damage;[^}]*value\s*=\s*([0-9.]+)',body,re.S)[1])
        check(row['fields']['damage']['value']==expected,'native Citizen damage factor')
        literal_checks.append(f'assert_eq!(u::lookup_class(8,{row["epoch"]}).unwrap().damage,Some({expected}_f32));')
    # CityCenter15 has a misleading raw CSV EPOCH13. Verify the active event graph
    # directly rather than trusting the generator's EPOCH normalization.
    check(upgrades['CityCenterUpgradeEpoch15'][0]['EPOCH']=='13','native misleading epoch remains visible')
    tree=dict((r['NAME'],r) for r,_ in table('db/techtree/dbtechtreenod5.csv'))
    check(tree['Main15']['UPGRADE']=='MainEpoch15','native Main15 reference')
    epoch15=text('db/techtree/epoch15_upgrades_15.ddf')
    main15=epoch15.split('UpgradeRefSet MainEpoch15',1)[1].split(']',1)[0]
    check('CityCenterUpgradeEpoch15' in main15,'native event15 activates CityCenter15')
    city=[r for r in units['class_rows'] if r['id']==0]
    check(next(r for r in city if r['epoch']==14)['fields']['hp']['value']==17500,'CityCenter17500 only at runtime14')
    check(next(r for r in city if r['epoch']==12)['until']==14,'CityCenter15000 interval ends14')
    matrix_rows=table('db/simulation/dbcomba5.csv',2)
    matrix=[[int(v) for k,v in r.items() if k!=next(iter(r))] for r,_ in matrix_rows]
    check(len(matrix)==35,'35 native RPS defender rows')
    for d in range(35):
        for a in range(35):check(matrix[d][a]==combat['matrix'][d][a],'RPS orientation/value')
    pairs={}
    for row,_ in table('db/simulation/dbcombat_unittypeadjus5.csv'):
        for defender in re.findall(r'\w+',row['DEFENDERS']):pairs.setdefault((row['ATTACKER'],defender),int(row['ADJUST']))
    check(len(pairs)==516,'native516 adjustments')
    names=combat['upgradeNames']
    decoded={(names[a],names[d]):v for a,d,v in combat['adjustments']}
    check(decoded==pairs,'first-hit adjustment table')
    profiles=combat['profiles'];oracle=bytearray(struct.pack('<I',len(profiles)))
    for a in profiles:
        for d in profiles:
            base=matrix[d['rps']][a['rps']];adjust=pairs.get((a['upgrade'],d['upgrade']),100)
            oracle.extend(struct.pack('<HH',base,adjust))
            check(0<=base<=65535 and 0<=adjust<=65535,'bounded source matchup')
    (ROOT/('tools/'+prefix.removeprefix('ee2-')+'-combat-oracle.bin')).write_bytes(oracle)
    # Assert node cost/time and each factor numeric directly against native CSV5.
    factors={r['']:(r,line) for r,line in table('db/techtree/upgrade_factorse5.csv')}
    check(len(research['nodes'])==180,'180 stable research node slots')
    for node in research['nodes']:
        native=tree[node['id']];rawnode=node['raw_node']
        for key in ['EPOCH','SLOT','TECHPTS','TIME']:check(float(native[key])==float(rawnode[key]),'research native numeric '+key)
        check(node['display_token']==native['DISPLAYNAME'],'research original display token')
        for ef in node['effects']:
            f,_=factors[ef['factor_id']]
            check(ef['value']==float(f['VALUE']),'native factor value')
            check(ef['factor']==f['FACTOR'],'native factor field')
            check(ef['method']=={'AddPermMul':0,'Add':1,'Set':2}[f['METHOD']],'native factor method')
            check(ef['mask']==sum(1<<i for i in ef['targets']),'source selector mask')
    check(sum(len(n['unimplemented_factors']) for n in research['nodes'])==research['summary']['unsupported_factors'],'unsupported source factors disclosed')
    # Independent source literals become Rust tests, exercising compiled arrays.
    out=['use crate::ids::DefId;','// Generated by test_ee2_up15_sources.py directly from raw native CSV5.',
         'use crate::{ee2_combat as c,ee2_units as u,ee2_research as r};',
         '#[test] fn raw_native_matrix_and_adjustment_oracle(){',
         'let bytes=include_bytes!("../tools/up15-combat-oracle.bin");let n=u32::from_le_bytes(bytes[..4].try_into().unwrap())as usize;assert_eq!(n,c::PROFILES.len());',
         'for a in 0..n{for d in 0..n{let o=4+(a*n+d)*4;let b=u16::from_le_bytes(bytes[o..o+2].try_into().unwrap());let adj=u16::from_le_bytes(bytes[o+2..o+4].try_into().unwrap());assert_eq!(c::base_percent(a as u16,d as u16),Some(b));assert_eq!(c::adjustment_percent(a as u16,d as u16),Some(adj));assert!((c::multiplier(a as u16,d as u16).unwrap()-(b as f32*0.01)*(adj as f32*0.01)).abs()<0.000001);}}}',
         '#[test] fn raw_native_unit_statistics_costs_and_reload(){']
    for i,e,hp,los,damage,rng,reload,time,cost in source_units:
        out.append(f'{{let v=u::lookup_class({i},{e}).unwrap();assert_eq!(v.hp,Some({hp}_f32));assert_eq!(v.sight,Some({los}_f32));assert_eq!(v.damage,Some({damage}_f32));assert_eq!(v.range,Some({rng}_f32));assert_eq!(v.reload,Some({reload}_f32));assert_eq!(v.buildtime,Some({time}_f32));assert_eq!(v.cost9,Some({str(cost)}));}}')
    out+=['}','#[test] fn raw_native_research_cost_time_epoch_host(){']
    for i,node in enumerate(research['nodes']):
        native=tree[node['id']]
        out.append(f'assert_eq!(r::NODES[{i}].cost,{float(native["TECHPTS"])}_f32);assert_eq!(r::NODES[{i}].time,{float(native["TIME"])}_f32);assert_eq!(r::NODES[{i}].epoch,{int(native["EPOCH"])-1});')
    out+=['}','#[test] fn raw_native_epoch_advancement_cost(){']
    for i in range(1,16):out.append(f'assert_eq!(r::ADVANCE_COST[{i-1}],{float(tree[f"Main{i}"]["TECHPTS"])}_f32);')
    out+=['}','#[test] fn actual_cross_named_refs_and_citizen_inline_factors(){']+literal_checks+['}']
    out+=['#[test] fn native_popcap_research_effect_executes(){',
      'let mut found=0;for (k,node)in r::NODES.iter().enumerate(){for ef in node.effects{if ef.field!=18{continue;}for id in 0..57{if !crate::ids::mask_has(ef.mask,id as DefId){continue;}let mut game=crate::game::Game::new(7,0,0,1,0);game.players[0].epoch=node.epoch;game.rebuild_modifiers(0,false);let baseline=game.modifiers[0][id][18];game.researched[0][k/32]|=1u32<<(k%32);game.rebuild_modifiers(0,false);let mut expected=baseline;for f in node.effects{if f.field==18&&crate::ids::mask_has(f.mask,id as DefId){match f.method{0=>expected+=baseline*f.value*0.01,1=>expected+=f.value,_=>expected=f.value}}}assert_eq!(game.modifiers[0][id][18],expected);found+=1;}}}assert!(found>0);', '}']
    out=[line.replace('up15-combat-oracle.bin',prefix.removeprefix('ee2-')+'-combat-oracle.bin').replace('0..57','0..crate::defs::DEFS.len()') for line in out]
    (ROOT/('src/'+prefix.replace('-','_')+'_oracle_tests.rs')).write_text('\n'.join(out)+'\n')
    report={'format':'ee2-up15-independent-source-oracles-v1','checks':checks,'source_manifest_sha256':hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),'profiles':len(profiles),'compiled_matchups':len(profiles)**2,'compiled_unit_csv_records':len(source_units),'research_nodes':180,'originals_modified':False,'dll_or_exe_executed':False}
    (ROOT.parent/'source-oracle-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report));return report
if __name__=='__main__':run()
