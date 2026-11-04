"""Produce a complete status inventory of every bound row's actual native refs."""
from pathlib import Path
import collections,hashlib,json
ROOT=Path(__file__).resolve().parents[1]
def run():
    units=json.loads((ROOT/'docs/ee2-up15-units.json').read_text(encoding='utf8'))
    ability_policy={
      'Attack':{'damage':'CSV nonzero override; zero falls back to actual ref. Live damage modifier consumes effective field.',
                'range':'CSV nonzero override; zero falls back to actual ref. Live range modifier consumes effective field.',
                'reloadTime':'CSV nonzero override; zero falls back to actual ref. Live fire cooldown consumes effective field.',
                'missileName':'Retained in effective attack_missile. Only Fortress Cannonball/TankShell mapping is integrated; other projectile types remain adapters.'},
      'LOS':{'range':'CSV nonzero override; zero falls back to actual ref. Live LOS modifier consumes effective field.'},
      'Garrison':{'numOfSlots':'Effective source slots consumed; existing CARGO_MAX10 capacity/role adapter remains.',
                   'healPoints':'Retained; runtime healing consumes this only for Fortress.',
                   'attackBonus':'Retained; runtime attack bonus consumes this only for Fortress.',
                   'gatherRate':'Effective University/extractor generation rate consumed.',
                   'gatherType':'Effective University/extractor resource selector consumed.',
                   'resourceBonus':'Effective dropoff resource bonus consumed.'},
    }
    for movement in ['Move','VehicleMove','NavalMove','AircraftMove']:
        ability_policy[movement]={'speed':'Effective source movement value consumed using explicit engine distance-unit identity adapter.',
                                  'angSpeed':'Effective source degrees/sec multiplied by compiled radians constant; native accel/turn path remains adapter.'}
    rows=[];counts=collections.Counter();field_counts=collections.Counter();unresolved=[];unsupported=[]
    for row in units['class_rows']:
        leaves=row['fields']['declared_upgrade_refs']['value'];evaluated=[]
        for record in leaves:
            leaf=dict(record);counts[leaf['kind']]+=1
            if leaf['kind']=='unresolved':unresolved.append({'id':row['id'],'epoch':row['epoch'],**leaf})
            for change in leaf.get('abilities',[]):
                change=dict(change);statuses={}
                for field,value in change['fields'].items():
                    status=ability_policy.get(change['ability'],{}).get(field)
                    if status is None:
                        status='Unsupported native ability behavior; retained verbatim as numeric/token metadata, no guessed simulation effect.'
                        unsupported.append({'id':row['id'],'epoch':row['epoch'],'ref':leaf['name'],'ability':change['ability'],'field':field,'value':value,'source':leaf['source']})
                    statuses[field]=status;field_counts[(change['ability'],field)]+=1
                change['field_status']=statuses;evaluated.append({'ref':leaf['name'],**change})
            if 'status' in leaf and 'unsupported' in leaf['status'] and leaf['kind'] not in ['UpgradeSize','unresolved']:
                unsupported.append({'id':row['id'],'epoch':row['epoch'],**leaf})
        rows.append({'id':row['id'],'epoch':row['epoch'],'until':row['until'],'source_unit':row['source_unit'],
                     'source_upgrade':(row.get('source_upgrade')or{}).get('UPGRADE'),
                     'roots':(row.get('source_upgrade')or{}).get('UPGRADEREFS'),
                     'leaves':leaves,'ability_field_status':evaluated})
    header=ROOT.parents[1]/'up-resolved-sources/files/dbhdrs/DbUpgradeDefs.h'
    if not header.exists():header=ROOT.parents[1]/'work/up-resolved-sources/files/dbhdrs/DbUpgradeDefs.h'
    report={'format':'ee2-up15-complete-unit-upgrade-refs-v1','scope':'All275 bound class rows; actual CSV UPGRADEREFS or Citizen DDF UpgradeRefs. Explicit no-op temporary reset when no native temp improvement layer exists. Missing, unsupported and merely retained fields are disclosed.',
      'header_evidence':{'path':'DbHdrs/DbUpgradeDefs.h','sha256':hashlib.sha256(header.read_bytes()).hexdigest(),'line58':'DbUpgradeUnitTypeUpgrade','line61':'m_userEpoch IGNORED','line62to66':'nonzero scalar replacements or0 placeholders'},
      'class_rows':len(rows),'counts':dict(counts),'ability_field_counts':[{'ability':a,'field':f,'occurrences':n} for (a,f),n in sorted(field_counts.items())],
      'unresolved_refs':unresolved,'unsupported_occurrences':unsupported,'rows':rows,
      'remaining_nonref_scope':['Native target/weapon domain matrices and secondary attacks unbound beyond RPS gates.',
         'Native temporary veteran/elite/area/civilization layers are absent; resetting their multiplier to zero has no current simulation effect.',
         'CityRadius/CityHealRate retained; completed-TC radius adapter happens to match7..11 source values, native CityManager/heal-area behavior remains unbound.',
         'UpgradeSize retained in raw_size; simulation footprint/pathing/world conversion unchanged.']}
    (ROOT/'docs/ee2-up15-unit-refs.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
    assert not unresolved,unresolved
    print(json.dumps({'class_rows':len(rows),'leaves':sum(counts.values()),'kinds':dict(counts),'unresolved':len(unresolved),'unsupported_occurrences':len(unsupported)}))
    return report
if __name__=='__main__':run()
