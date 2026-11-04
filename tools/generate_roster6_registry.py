"""Generate the portable save namespace fingerprint from explicit source bindings."""
from pathlib import Path
import argparse
import hashlib
import json

ROOT = Path(__file__).resolve().parents[1]


def generate(verify=False):
    audit = json.loads((ROOT / 'docs/ee2-up15-units.json').read_text(encoding='utf8'))
    records = [{'id': b['id'], 'legacy_role': b['definition_name'],
                'role_adapter': b['role_adapter'], 'source_family': b['source_unit']}
               for b in sorted(audit['bindings'], key=lambda b: b['id'])]
    assert [r['id'] for r in records] == list(range(len(records)))
    encoded = json.dumps(records, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n'
    digest = hashlib.sha256(encoded.encode('utf8')).digest()
    # Keep stable rustfmt output; this is namespace identity, not native indices.
    nums = ', '.join(str(v) for v in digest)
    content = '// Generated from roster6-registry57.json; namespace identity, not a fabricated original index.\n'
    content += 'pub const FINGERPRINT: [u8; 32] = [' + nums + '];\n'
    if not verify and (ROOT/'docs/roster6-registry61.json').exists():
        raise ValueError('The61 registry requires generate_roster6_classic.py; refusing a silent57 namespace reset')
    targets = {ROOT / 'docs/roster6-registry57.json': encoded,
               ROOT / 'src/roster6_registry.rs': content}
    for path, text in targets.items():
        if verify:
            if path.suffix == '.rs':
                import re
                got = bytes(map(int, re.findall(r'\b\d+\b', path.read_text().split('= [', 1)[1])))
                assert got == digest, 'Registry fingerprint changed'
            else:
                assert path.read_text(encoding='utf8') == text, 'Registry identity changed'
        else:
            path.write_text(text, encoding='utf8')
    return {'definitions': len(records), 'sha256': digest.hex(), 'verified': verify}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify', action='store_true')
    print(json.dumps(generate(parser.parse_args().verify)))
