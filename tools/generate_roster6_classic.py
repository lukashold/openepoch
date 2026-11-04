"""Build the isolated four-family classic slice from active UP sources.

No original executable/DLL is run. The old57 modules and original media are
never overwritten. All numerical fields follow the same native-bound UP loader.
"""
from pathlib import Path
import hashlib
import json
import import_ee2_up15_units as units
import bind_candidate_visual_progression as visual_progression
import import_ee2_up15_combat as combat
import import_ee2_up15_research as research
import generate_native_attack as attack
import import_ee2_up15_profile as profile
import test_ee2_up15_sources as oracle
import roster6_availability as availability

ROOT = Path(__file__).resolve().parents[1]
PREFIX = 'ee2-roster6'


def run():
    availability_data=availability.run()
    availability_by_id={b['id']:b for b in availability_data['fourBoundaries']}
    selected = json.loads((ROOT/'docs/roster6-classic-selection.json').read_text())['bindings']
    units.run(selected, PREFIX, 'manifest-roster6.json')
    visual_progression.run(PREFIX,'manifest-roster6.json')
    data=json.loads((ROOT/('docs/'+PREFIX+'-units.json')).read_text(encoding='utf8'))
    combat.run(prefix=PREFIX)
    research.run(prefix=PREFIX)
    attacks = attack.run(prefix=PREFIX)['rows']
    profile.run(prefix=PREFIX)
    oracle.run(prefix=PREFIX)
    out = ['// Generated exact source-bound classic families; no placeholder definitions.',
           'use crate::defs::*;', f'pub const DEF_COUNT:usize={len(data["bindings"])};',
           f'pub const fn initial(base:[Def;57])->[Def;{len(data["bindings"])}]{{',
           f'let mut d=[base[0];{len(data["bindings"])}];let mut i=0;while i<57{{d[i]=base[i];i+=1;}}']
    audit = []
    for b in selected:
        row = min((r for r in data['class_rows'] if r['id']==b['id']), key=lambda r:r['epoch'])
        fields = row['fields']
        def value(key):
            field = fields[key]
            assert field['verified'] and field['value'] is not None, (b['source_unit'],key)
            return field['value']
        hp, sight, damage, rng, reload, build = [value(k) for k in ['hp','sight','damage','range','reload','buildtime']]
        speed = 0.0 if b['building'] else value('raw_speed')
        if b['building']:
            speed = 0.0
            geometry = value('raw_footprint')
            size = max(geometry)
            host = 'BUILT_BY_WORKER'
            armor = 'A_BLD'
        else:
            geometry = value('raw_size')
            size = 2*max(geometry)
            host = {'CityCenter':'TC','Dock':'DOCK'}[b['host']]
            armor = 'A_SHIP' if b['domain']=='SEA' else 'A_INF'
        production=availability_by_id[b['id']]
        assert production['startRuntimeEpoch']==row['epoch']
        until=production['endRuntimeEpochExclusive']
        cost = value('cost9')
        pop = 0 if b['building'] else value('pop') # Building queue adapters never consume unit population.
        source_attack = next(r for r in attacks if r['id']==b['id'] and r['epoch']==row['epoch'])
        targets = 'LAND|SEA|AIR|SUB' if source_attack['present'] and not source_attack['latent'] else '0'
        out.append(f'd[{b["id"]}]=Def{{name:{json.dumps(b["definition_name"])},epoch:{row["epoch"]},until:{until},dom:{b["domain"]},building:{str(b["building"]).lower()},size:{float(size)}_f32,hp:{float(hp)}_f32,speed:{float(speed)}_f32,sight:{float(sight)}_f32,armor:{armor},cost:{cost[:3]},time:{float(build)}_f32,pop:{int(pop)},made_at:{host},flags:0,w:[Wpn{{range:{float(rng)}_f32,dmg:{float(damage)}_f32,cd:{float(reload)}_f32,kind:{source_attack["visualKind"]},targets:{targets},splash:{source_attack["splashRadius"]}_f32}},NOW]}};')
        audit.append({'id':b['id'],'sourceFamily':b['source_unit'],'firstClassRow':row,
                      'geometryProjection':{'sourceField':'raw_footprint' if b['building'] else 'raw_size',
                        'sourceValue':geometry,'coreSize':size,
                        'scope':'Existing square building grid or scalar mobile collision adapter; no full native oriented collision/formation claim.'},
                      'productionAvailability':production,'limits':b['limits']})
    out += ['d','}',f'pub const fn production(mut d:[Def;{len(data["bindings"])}])->[Def;{len(data["bindings"])}]{{']
    for ident,bound in availability_by_id.items():
        out.append(f'd[{ident}].epoch={bound["startRuntimeEpoch"]};d[{ident}].until={bound["endRuntimeEpochExclusive"]};')
    out += ['d','}']
    (ROOT/'src/roster6_defs.rs').write_text('\n'.join(out)+'\n',encoding='utf8')
    records = [{'id':b['id'],'legacy_role':b['definition_name'],'role_adapter':b['role_adapter'],
                'source_family':b['source_unit']} for b in data['bindings']]
    encoded = json.dumps(records,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n'
    fingerprint = hashlib.sha256(encoded.encode('utf8')).digest()
    (ROOT/'docs/roster6-registry61.json').write_text(encoded,encoding='utf8')
    text = '// Generated explicit61-family namespace fingerprint.\n'
    text += 'pub const FINGERPRINT:[u8;32]=['+','.join(map(str,fingerprint))+'];\n'
    text += 'pub const PREVIOUS_57:[u8;32]=['+','.join(map(str,bytes.fromhex('9d62e9fb0ed9ba38c483510f2c5ac3da28752129b5c6ab3b39b60ed58225ac00')))+'];\n'
    (ROOT/'src/roster6_registry.rs').write_text(text,encoding='utf8')
    report = {'format':'native-classic6-source-slice-v1','definitions':len(data['bindings']),
              'classRows':len(data['class_rows']),'registrySha256':fingerprint.hex(),'newFamilies':audit,
              'scope':'Four additional classical production families with all active UP numeric progressions and exact original meshes. Existing57 IDs remain fixed.',
              'limits':['Native procedural child effects, including sails, are not supplied by a parent NIF mesh alone.',
                        'The four new production bounds follow active Main/Obsolete paths; existing57 availability remains unchanged. Undo targets are audited and none restores these four production nodes.',
                        'Native MainObsolete also cancels production and refunds stored paid job prices (008167F0). Existing core has no paid-price job state. This availability-only fix therefore leaves existing queues intact; exact cancellation is deferred to a saved-price implementation.',
                        'Per-family improvements, conversion, formations and native collision are not newly claimed.',
                        'Permanent percent-field selectors and additive state are source-bound, but 180/204 are implementation arithmetic regression values. Native x87 CHOP53 versus nearest context at the actual setter remains unresolved; no general native IEEE-bit guarantee is made.'],
              'activeRuntimeModified':False}
    (ROOT/'docs/roster6-classic-audit.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(json.dumps({'definitions':len(data['bindings']),'classRows':len(data['class_rows']),'registrySha256':fingerprint.hex()}))


if __name__=='__main__':
    run()
