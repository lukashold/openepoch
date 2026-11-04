"""Generate only the isolated UP15 unit slice from the proven native source manifest."""
from pathlib import Path
import hashlib,json,shutil
import import_ee2_units as units
from up15_source_database import ActiveDatabase,default_manifest
from up15_unit_upgrade_refs import apply_refs
ROOT=Path(__file__).resolve().parents[1]
MANIFEST=default_manifest(ROOT)
STAGE=ROOT.parent/'stage/OpenEpoch'

def run():
    for directory in ['src','docs/local-original/models']: (STAGE/directory).mkdir(parents=True,exist_ok=True)
    shutil.copyfile(ROOT/'src/defs.rs',STAGE/'src/defs.rs')
    shutil.copyfile(ROOT/'docs/local-original/models/manifest-up15.json',STAGE/'docs/local-original/models/manifest.json')
    research=STAGE.parent/'EE2-Research/analysis';research.mkdir(parents=True,exist_ok=True)
    shutil.copyfile(ROOT.parent/'EE2-Research/analysis/fortress-population-binding.json',research/'fortress-population-binding.json')
    old_root,old_database,old_fields=units.ROOT,units.Database,units.make_fields
    def fields(db,original,epoch,upgrade=None,production=None):
        result,resolved=old_fields(db,original,epoch,upgrade,production)
        abilities,ref_audit,sizes,factors=apply_refs(db,original,upgrade,epoch,resolved['abilities'])
        # Nonzero CSV columns are explicit replacements (DbUpgradeDefs.h).
        # UpgradeAbilities attack structs intentionally contain 0/0/666
        # placeholders; preserve CSV scalar overrides while binding actual refs.
        mapping={'garrison_slots':('garrison','numOfSlots'),'garrison_heal':('garrison','healPoints'),
                 'garrison_attack_bonus':('garrison','attackBonus'),'generation':('garrison','gatherRate'),
                 'attack_missile':('attack','missileName'),'dropoff_bonus':('garrison','resourceBonus')}
        for field,(ability,key) in mapping.items():result[field]=units.value_from(abilities.get(ability,{}),key)
        for field,column,ability,key in [('damage','DAMAGE','attack','damage'),('range','RANGE','attack','range'),
                                        ('reload','RELOAD','attack','reloadTime'),('sight','LOS','los','range')]:
            if not upgrade or not float(upgrade.get(column,'0') or 0):
                candidate=units.value_from(abilities.get(ability,{}),key)
                if candidate['verified']:result[field]=candidate
        resource=abilities.get('garrison',{}).get('gathertype')
        if resource:
            name=str(resource[0]).upper();index=units.RESOURCES.index(name) if name in units.RESOURCES else 9 if name=='TECH' else None
            result['generation_resource']=units.field(index,[resource[1]])
        movement=[abilities[name] for name in ['move','vehiclemove','navalmove','aircraftmove'] if 'speed' in abilities.get(name,{})]
        if len(movement)==1:
            result['raw_speed']=units.value_from(movement[0],'speed')
            result['angular_speed']=units.value_from(movement[0],'angSpeed')
        else:result['angular_speed']=units.field(note='No unique inherited movement ability')
        for size in sizes:result['raw_size']=units.field([size['sizex'][0],size['sizey'][0]],[size['sizex'][1],size['sizey'][1]],'Actual UpgradeSize ref retained; no unproved grid/world conversion')
        chain=[];parent=original
        while parent:
            chain.append(parent);parent=units.scalar(db.block('UnitType',parent)['body'],'parent')
        factor_fields={'HitPoints':'hp','Damage':'damage','Speed':'raw_speed','BuildTime':'buildtime','LOS':'sight','Range':'range',
                       'GarrisonSlots':'garrison_slots','PopCap':'pop_capacity','CityRadius':'city_radius','CityHealRate':'city_heal_rate'}
        for factor in factors:
            target=factor['unit'];matches=(not target and not factor['attribute']) or target==original or (factor['children'] and target in chain)
            if not matches:factor['status']='not targeted at this source class';continue
            if factor['method']=='SetTempMul' and factor['value']==0:
                factor['status']='temporary multiplier reset to zero; explicit no-op because this slice has no native temporary improvement layer';continue
            field=factor_fields.get(factor['factor'])
            if field and factor['method'] in ['Set','Add','AddPermMul']:
                before=result.get(field,units.field())['value'];value=factor['value']
                if factor['method']=='Add':value=(before or 0)+value
                if factor['method']=='AddPermMul':value=(before or 0)*(1+value*0.01)
                result[field]=units.field(value,[factor['source']],'Actual bound UpgradeRefs factor '+factor['method'])
                factor['status']='retained only; original city behavior not integrated' if field in ['city_radius','city_heal_rate'] else 'applied numeric field '+field
            else:factor['status']='unsupported factor/method; no guessed effect'
        # Native 0053C930 calls every +7c/+80 UpgradeRef through 0053CD60
        # BEFORE its nonzero scalar replacement calls (0053CADD..0053CBC1).
        # Reapply explicit CSV values last, including a future factor targeting
        # the same field. Current installed inline factors have no such overlap.
        for field,column in [('hp','HP'),('buildtime','BUILDTIME'),('sight','LOS'),
                             ('damage','DAMAGE'),('range','RANGE'),('reload','RELOAD'),
                             ('pop_capacity','POP CAPACITY')]:
            if upgrade and float(upgrade.get(column,'0') or 0):
                result[field]=units.csv_field(upgrade,column)
                result[field]['note']='Native0053C930: nonzero CSV replacement after actual UpgradeRefs'
        result['declared_upgrade_refs']=units.field(ref_audit,[ref['source'] for ref in ref_audit if 'source' in ref],
            'Complete reachable refs for the actual bound CSV upgrade (or Citizen DDF upgrade), including implemented, no-op and unsupported leaves.')
        return result,resolved
    try:
        units.ROOT=STAGE;units.Database=lambda _:ActiveDatabase(MANIFEST);units.make_fields=fields
        data=units.run()
    finally:units.ROOT,units.Database,units.make_fields=old_root,old_database,old_fields
    db=ActiveDatabase(MANIFEST)
    try:
        db.table('db/techtree/upgrade_unittype5.csv')
        data['format']='ee2-up15-unit-runtime-audit-v1'
        data['scope']='Resolved native UP15 CSV5 and exactly the two native CFG5 DDF lists. Existing 57 gameplay roles only; aliases, unmatched visual identities and unsupported native abilities remain explicit.'
        data['native_source_manifest']=db.manifest_audit()
        data['native_garrison_slots']['scope']='Compiled popCount binding applied to the resolved active CFG5 DDF values, preserving role-alias limitations.'
        for collection in ['rows','class_rows']:
            for row in data[collection]:
                upgrade=row.get('source_upgrade') or {}
                row['display_tokens']={key:upgrade.get(key,'') for key in ['DISPLAYNAME','DISPLAYNAME_PLURAL','UNIT','UPGRADE']}
                row['display_token_source']=upgrade.get('_source')
        for binding in data['bindings']:
            node=binding.get('production_node') or {}
            binding['display_tokens']={key:node.get(key,'') for key in ['DISPLAYNAME','DISPLAYNAME_PLURAL','PRODUCE','UPGRADE']}
            binding['display_token_source']=node.get('_source')
        source_paths=set(data['source_files'])|set(db.files)
        data['source_files']={path:{'sha256':db.reader.origin(path)['sha256'],'origin':db.reader.origin(path)} for path in sorted(source_paths)}
    finally:db.close()
    target=ROOT/'docs/ee2-up15-units.json';target.write_text(json.dumps(data,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
    rust=(STAGE/'src/ee2_units.rs').read_text().replace('tools/import_ee2_units.py','tools/import_ee2_up15_units.py').replace('Base db.zip','Resolved active UP15 CSV5/CFG5').replace('base db.zip','resolved active UP15 CSV5/CFG5')
    (ROOT/'src/ee2_up15_units.rs').write_text(rust)
    print(json.dumps({'bindings':len(data['bindings']),'rows':len(data['rows']),'class_rows':len(data['class_rows']),'manifest':data['native_source_manifest']['manifest_sha256']}))
    return data
if __name__=='__main__':run()
