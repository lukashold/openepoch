"""Regenerate the isolated80-family numerical candidate, without importing media."""
from pathlib import Path
import hashlib, json
import import_ee2_up15_units as units
import import_ee2_up15_combat as combat
import import_ee2_up15_research as research
import generate_native_attack as attack
import import_ee2_up15_profile as profile

ROOT = Path(__file__).resolve().parents[1]
PREFIX = 'ee2-civ8'


def run():
    selection = json.loads((ROOT / 'docs/civ8-selection.json').read_text(encoding='utf8'))['bindings']
    gates = json.loads((ROOT.parent / 'civ8-sources.json').read_text(encoding='utf8'))
    units.run(selection, PREFIX, 'manifest-roster6.json')
    combat.run(prefix=PREFIX); research.run(prefix=PREFIX)
    attacks = attack.run(prefix=PREFIX)['rows']; profile.run(prefix=PREFIX)
    data = json.loads((ROOT / 'docs/ee2-civ8-units.json').read_text(encoding='utf8'))
    code = ['// Generated80-definition isolated Civ8 candidate; exact source families.',
            'use crate::defs::*;', 'pub const fn initial(input:[Def;57])->[Def;80]{',
            'let old=crate::roster6_defs::initial(input);let mut d=[old[0];80];',
            'let mut i=0;while i<61{d[i]=old[i];i+=1;}']
    source_rows = []
    for gate in gates['candidateFamilies']:
        ident = gate['proposedDefId']; binding = next(b for b in selection if b['id'] == ident)
        row = min((r for r in data['class_rows'] if r['id'] == ident), key=lambda r: r['epoch'])
        fields = row['fields']
        def value(key):
            field = fields[key]
            if not field['verified'] or field['value'] is None:
                raise ValueError('Unverified required source field: ' + gate['sourceFamily'] + ':' + key)
            return field['value']
        hp, sight, damage, rng, reload, build, speed = [value(k) for k in
            ['hp', 'sight', 'damage', 'range', 'reload', 'buildtime', 'raw_speed']]
        geometry = value('raw_size'); size = 2 * max(geometry)
        cost = value('cost9'); pop = value('pop')
        source_attack = next(r for r in attacks if r['id'] == ident and r['epoch'] == row['epoch'])
        if not source_attack['present'] or source_attack['latent']:
            raise ValueError('Candidate ordinary Attack is not present/active: ' + gate['sourceFamily'])
        host = {'Barracks': 'BARRACKS', 'Stable': 'STABLE', 'Mill': 'MILL'}[gate['host']]
        code.append('d[%d]=Def{name:%s,epoch:%d,until:%d,dom:LAND,building:false,size:%s_f32,hp:%s_f32,speed:%s_f32,sight:%s_f32,armor:A_INF,cost:%s,time:%s_f32,pop:%d,made_at:%s,flags:0,w:[Wpn{range:%s_f32,dmg:%s_f32,cd:%s_f32,kind:%d,targets:LAND|SEA|AIR|SUB,splash:%s_f32},NOW]};' % (
            ident, json.dumps(binding['definition_name']), row['epoch'], gate['endRuntimeEpochExclusive'],
            float(size), float(hp), float(speed), float(sight), cost[:3], float(build), int(pop), host,
            float(rng), float(damage), float(reload), source_attack['visualKind'], source_attack['splashRadius']))
        source_rows.append({'id': ident, 'sourceFamily': gate['sourceFamily'], 'sourceCiv': binding['source_civ'],
            'firstClassRow': row, 'production': gate,
            'geometryAdapter': {'source': geometry, 'scalarCollisionSize': size},
            'modelVerified': False, 'completeAbilities': False})
    code += ['d', '}', 'pub const fn production(mut d:[Def;80])->[Def;80]{']
    # Class upgrades may continue to15 after a family stops production. Reapply
    # source production windows only after economy/profile class generation.
    old = json.loads((ROOT / 'docs/roster6-availability.json').read_text(encoding='utf8'))
    for r in old['fourBoundaries']:
        code.append(f'd[{r["id"]}].epoch={r["startRuntimeEpoch"]};d[{r["id"]}].until={r["endRuntimeEpochExclusive"]};')
    for r in gates['candidateFamilies']:
        code.append(f'd[{r["proposedDefId"]}].epoch={r["startRuntimeEpoch"]};d[{r["proposedDefId"]}].until={r["endRuntimeEpochExclusive"]};')
    code += ['d', '}']
    (ROOT / 'src/civ8_defs.rs').write_text('\n'.join(code) + '\n', encoding='utf8')
    records = [{'id': b['id'], 'legacy_role': b['definition_name'], 'role_adapter': b['role_adapter'],
                'source_family': b['source_unit']} for b in data['bindings']]
    encoded = json.dumps(records, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n'
    fingerprint = hashlib.sha256(encoded.encode('utf8')).digest()
    (ROOT / 'docs/civ8-registry80.json').write_text(encoded, encoding='utf8')
    (ROOT / 'src/civ8_registry.rs').write_text('// Generated explicit80-family registry.\n' +
        'pub const FINGERPRINT:[u8;32]=[' + ','.join(map(str, fingerprint)) + '];\n' +
        'pub const PREVIOUS_61:[u8;32]=[' + ','.join(map(str, bytes.fromhex('d5790a14cedaef0d8c08c3399851eccdab34d2e302c9f68b1442a2d2284eed40'))) + '];\n', encoding='utf8')
    report = {'format': 'native-civ8-isolated-numeric-candidate-v1', 'definitions': 80,
        'classRows': len(data['class_rows']), 'registrySha256': fingerprint.hex(), 'newFamilies': source_rows,
        'activeRuntimeModified': False, 'readyForActivation': False,
        'limits': ['CIV-specific numerical rows and actual production gates are distinct from a completed ability implementation.',
                   'Root models are inventoried but not imported into this candidate yet; no placeholder models are supplied.',
                   'Pack/unpack is a separate source-backed candidate and is not active in this numerical core.',
                   'Capture, conversion, wonder events and live FarEast breakthrough are not wired; nine trait fields require additional mechanics.',
                   'This candidate has actual production, civilization gates, modern/legacy save and multiplayer oracles, documented in the phase1 freeze.',
                   'Regional building/Citizen graphics and host-specific original details remain separate adapters.']}
    (ROOT / 'docs/civ8-numeric-audit.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print(json.dumps({'definitions': 80, 'classRows': len(data['class_rows']), 'registrySha256': fingerprint.hex()}))


if __name__ == '__main__': run()
