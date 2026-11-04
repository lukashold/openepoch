"""Merge exact Civ8 recordings and source-proven live parent sound-cache links.

Only private stage manifests/media are written. Native type names identify sound
cache nodes; a parent's recordings do not create a fake runtime unit definition.
"""
from pathlib import Path
import argparse, copy, hashlib, json, re
from import_ee2_units import scalar
from up15_source_database import ActiveDatabase, default_manifest

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/local-original/audio'
sha = lambda b: hashlib.sha256(b).hexdigest()


def run(plan_path, report_path):
    plan = json.loads(plan_path.read_bytes())
    base_raw = (OUT / 'native-families.json').read_bytes()
    base = json.loads(base_raw)
    child = json.loads((OUT / 'native-families-civ8-raw.json').read_bytes())
    parents = json.loads((OUT / 'native-parent-caches8-raw.json').read_bytes())
    merged = copy.deepcopy(base)
    shared, linked = [], []
    db = ActiveDatabase(default_manifest(ROOT))
    try:
        for incoming in (child, parents):
            assert incoming['sourceManifestSha256'] == base['sourceManifestSha256']
            assert incoming['sourceTables'] == base['sourceTables']
            for name, row in incoming['families'].items():
                if name in merged['families']:
                    prior = merged['families'][name]
                    assert prior['intervals'] == row['intervals'] and prior['role'] == row['role']
                else:
                    merged['families'][name] = copy.deepcopy(row)
            for name, row in incoming['events'].items():
                if name in merged['events']:
                    old = merged['events'][name]
                    assert old['sha256'] == row['sha256']
                    assert {k:v for k,v in old.items() if k not in ('file','source')} == {k:v for k,v in row.items() if k not in ('file','source')}
                    shared.append(name)
                else:
                    merged['events'][name] = copy.deepcopy(row)
            for path, asset in incoming['assets'].items():
                if path in merged['assets']:
                    assert merged['assets'][path] == asset
                else:
                    merged['assets'][path] = copy.deepcopy(asset)
        for family in plan['families']:
            row = merged['families'][family['sourceFamily']]
            if family['initialVoiceStateNeedsNativeBinding']:
                declaration = db.block('UnitType', family['sourceFamily'])
                parent = scalar(declaration['body'], 'parent')
                if parent not in ('Scout', 'LightArtillery1') or parent not in merged['families']:
                    raise ValueError('Unaudited initial sound-cache parent')
                match = re.search(r'\bparent\s*=', declaration['body'], re.I)
                row['parentSoundFamily'] = parent
                row['parentSoundSource'] = db.block_ref(declaration, match.start())
                row['initialVoiceStateUnknown'] = False
                row['parentQueryPolicy'] = 'If the local effective channel is empty, query the current parent cache at the current epoch. Parent sets are not frozen at child creation or copied from a matching visual.'
                linked.append({'family': family['sourceFamily'], 'parent': parent, 'source': row['parentSoundSource']})
            else:
                row['initialVoiceStateUnknown'] = False
        merged['parentSoundEvidence'] = {
            'constructor': '0053B3ED..0053B41E: DbUnitTypeDef.parent resolves through0086C200;007C7E30 stores current type key and literal parent type key;007C7F80 initializes four own DDF channels.',
            'lookup': '0086C200 resolves the case-insensitive name in constructor context and returns the same type-node pointer. No epoch/visual snapshot parameter.',
            'setter': '007C7FD0: count<=0 preserves the previous own channel.',
            'query': '007C80A0: an empty current channel follows stored parent+8 on every query; parent cache updates remain live.',
            'update': '0053C930 updates the same family type sound-cache key through007C7FD0; visual changes do not substitute an unrelated sound identity.'}
        merged['limits'] = [
            'Original recording bytes, DDF parent names and CSV5 response arrays are imported from the active source profile.',
            'Three empty initial child response sets now query their proven native parent cache live at the current epoch.',
            'Application playback gain, scheduling and round-robin response choice remain explicit adapters; native audio RNG and concurrency are not claimed.',
            'Native DDF death channels, child effects and corpse/event scheduling remain separate bindings.',
            'The existing57-role audio manifest is retained separately. No native build-response channel is invented.']
        # Keep the old four families and their recording/event paths unchanged.
        assert all(merged['families'][k] == v for k,v in base['families'].items())
        assert all(merged['events'][k] == v for k,v in base['events'].items())
        assert (OUT / 'native-families.json').read_bytes() == base_raw
        for asset in merged['assets'].values():
            raw = (OUT / asset['file']).read_bytes()
            assert len(raw) == asset['bytes'] and sha(raw) == asset['sha256']
        target = OUT / 'native-families-civ8.json'
        target.write_text(json.dumps(merged, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
        report = {'format':'native-civ8-live-parent-audio-merge-v1','passed':True,
                  'activeRuntimeModified':False,'originalExecuted':False,
                  'familyCount':len(merged['families']),'directNewFamilies':len(child['families']),
                  'parentOnlyFamilies':[k for k,v in merged['families'].items() if not v['definitionIds']],
                  'linkedFamilies':linked,'sharedEventNames':shared,
                  'events':len(merged['events']),'recordings':len(merged['assets']),
                  'addedRecordingFiles':len(merged['assets'])-len(base['assets']),
                  'allRecordingHashesVerified':True,'manifestSha256':sha(target.read_bytes()),
                  'unresolvedInitialFamilies':[],'limits':merged['limits']}
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
        print(json.dumps({k:v for k,v in report.items() if k not in ('linkedFamilies','sharedEventNames','limits')}))
    finally:
        db.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    args = parser.parse_args()
    run(args.plan.resolve(),args.report.resolve())
