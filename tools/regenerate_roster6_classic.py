"""Regenerate the isolated61-family numeric slice, optionally checking byte identity.

Models are a separate local-only import: import_roster6_models.py. This tool
does not modify any old UP57 source module or any default/reference WASM.
"""
from pathlib import Path
import argparse,hashlib,json
import generate_roster6_classic as generate
import catalog_roster6_classic as catalogue
ROOT=Path(__file__).resolve().parents[1]
FILES=[*(f'src/ee2_roster6_{name}.rs'for name in ['units','combat','research','attack','profile','economy','oracle_tests']),
       'src/roster6_defs.rs','src/roster6_registry.rs','tools/roster6-combat-oracle.bin',
       *(f'docs/ee2-roster6-{name}.json'for name in ['units','combat','research','catalog']),
       'docs/native-roster6-attack.json','docs/roster6-registry61.json','docs/roster6-classic-audit.json','docs/roster6-classic-catalogue.json','docs/roster6-availability.json']
def digests():return{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in FILES if(ROOT/p).is_file()}
def run(verify=False):
    old=digests();generate.run();catalogue.run();now=digests();assert len(now)==len(FILES)
    if verify:assert old==now,'Non-identical regeneration: '+str([p for p in now if old.get(p)!=now[p]])
    report={'format':'native-classic6-regeneration-v1','verificationRequired':verify,'byteIdentical':old==now,'files':now,'old57ModulesModified':False,'originalExecuted':False}
    (ROOT.parent/'regeneration-report.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(json.dumps({'files':len(now),'byteIdentical':old==now,'verified':verify}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--verify',action='store_true');run(p.parse_args().verify)
