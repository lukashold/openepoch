"""Regenerate the isolated Civ8 numerical slice and its independent source oracle.

--verify requires every generated Rust/oracle byte to match the reviewed build.
Audit JSON may be refreshed separately; no game installation is written.
"""
from pathlib import Path
import argparse, hashlib, json
import generate_civ8_numerics, generate_civ8_main, test_ee2_up15_sources

ROOT = Path(__file__).resolve().parents[1]

def hashed_outputs():
    paths = list((ROOT / 'src').glob('ee2_civ8_*.rs'))
    paths += [ROOT / ('src/' + n) for n in ['civ8_defs.rs', 'civ8_registry.rs', 'civ8_main.rs']]
    paths += [ROOT / 'tools/civ8-combat-oracle.bin', ROOT / 'docs/civ8-registry80.json']
    return {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(paths) if p.is_file()}

def run(verify=False):
    before = hashed_outputs()
    generate_civ8_numerics.run()
    generate_civ8_main.run()
    test_ee2_up15_sources.run(prefix='ee2-civ8')
    after = hashed_outputs()
    changed = [k for k in sorted(set(before) | set(after)) if before.get(k) != after.get(k)]
    if verify and changed:
        raise ValueError('Reviewed generated outputs changed: ' + ', '.join(changed))
    report = {'format': 'native-civ8-regeneration-v1', 'status': 'PASS',
              'verify': verify, 'generated': after, 'changed': changed,
              'scope': 'Isolated candidate only; no original media or active runtime writes.'}
    (ROOT.parent / 'regeneration-report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print(json.dumps({'status': 'PASS', 'generated': len(after), 'changed': changed}))

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify', action='store_true')
    run(parser.parse_args().verify)
