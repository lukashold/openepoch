"""Verify the four unchanged profiles before the separately enabled paid core."""
from pathlib import Path
import hashlib,json,shutil,subprocess
import build_classic_profiles
ROOT=Path(__file__).resolve().parents[1]
def run():
    build_classic_profiles.run()
    for name,args in [('tests',['cargo','test','--offline','--features','ee2-paidqueue7']),('build',['cargo','build','--offline','--release','--target','wasm32-unknown-unknown','--features','ee2-paidqueue7'])]:
        with(ROOT.parent/f'paid7-{name}.log').open('w',encoding='utf8')as log:subprocess.run(args,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
    target=ROOT.parent/'builds/openepoch-paidqueue7.wasm'
    shutil.copyfile(ROOT/'target/wasm32-unknown-unknown/release/openepoch.wasm',target)
    report=json.loads((ROOT.parent/'build-report.json').read_text(encoding='utf8'))
    report['profiles']['paid7']={'path':str(target),'bytes':target.stat().st_size,'sha256':hashlib.sha256(target.read_bytes()).hexdigest()}
    (ROOT.parent/'build-report.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(json.dumps(report['profiles']['paid7']))
if __name__=='__main__':run()
