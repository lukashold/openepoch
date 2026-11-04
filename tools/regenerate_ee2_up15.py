"""Regenerate the isolated/static UP15 numeric modules from proven local sources.
Never runs installed DLL/EXE, never extracts media, never opens Ghidra or contacts GitHub.
"""
from pathlib import Path
import argparse,hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[1]
FILES=['src/ee2_up15_units.rs','src/ee2_up15_combat.rs','src/ee2_up15_research.rs','src/ee2_up15_profile.rs','src/ee2_up15_economy.rs','src/ee2_up15_oracle_tests.rs','tools/up15-combat-oracle.bin','docs/ee2-up15-units.json','docs/ee2-up15-combat.json','docs/ee2-up15-research.json','docs/ee2-up15-catalog.json','docs/ee2-up15-unit-refs.json']
SCRIPTS=['import_ee2_up15_units.py','import_ee2_up15_combat.py','import_ee2_up15_research.py','import_ee2_up15_profile.py','test_ee2_up15_sources.py','audit_ee2_up15_upgrade_refs.py']
def digest():return {p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in FILES if (ROOT/p).is_file()}
def run(verify=False):
    before=digest()
    for script in SCRIPTS:subprocess.run([sys.executable,str(ROOT/'tools'/script)],check=True)
    after=digest();assert len(after)==len(FILES)
    if verify:assert before==after,'Regeneration was not byte-identical: '+str([p for p in after if before.get(p)!=after[p]])
    report={'format':'ee2-up15-regeneration-v1','byte_identical':before==after,'verification_required':verify,'files':after,'source_scope':'Native00541940 CSV5 literals and two active CFG5 lists only; native VFS resolution already encoded in the prepared effective manifest.'}
    (ROOT.parent/'regeneration-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'regenerated':len(after),'byte_identical':before==after,'verification_required':verify}))
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--verify',action='store_true');run(parser.parse_args().verify)
