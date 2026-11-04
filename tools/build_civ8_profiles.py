"""Build/test every isolated feature profile without touching active artifacts."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess
ROOT=Path(__file__).resolve().parents[1]
PROFILES=[('own0',''),('reference1','ee2-reference'),('up5','ee2-up15'),
          ('roster6','ee2-roster6'),('joint7','ee2-paidqueue7,ee2-render-meta7'),('civ8','ee2-civ8')]
def run():
    out=ROOT.parent/'builds';out.mkdir(exist_ok=True);env=os.environ.copy();env['RUST_MIN_STACK']='67108864'
    rows={}
    for name,features in PROFILES:
        option=['--no-default-features']+(['--features',features]if features else[])
        with(ROOT.parent/(name+'-rust-tests.log')).open('w',encoding='utf8')as log:
            subprocess.run(['cargo','test','--offline',*option,'--','--test-threads=1'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
        with(ROOT.parent/(name+'-build.log')).open('w',encoding='utf8')as log:
            subprocess.run(['cargo','build','--offline','--release','--target','wasm32-unknown-unknown',*option],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
        dest=out/('openepoch-'+name+'.wasm');shutil.copyfile(ROOT/'target/wasm32-unknown-unknown/release/openepoch.wasm',dest)
        rows[name]={'path':str(dest),'features':features,'bytes':dest.stat().st_size,'sha256':hashlib.sha256(dest.read_bytes()).hexdigest(),
                    'testLogSha256':hashlib.sha256((ROOT.parent/(name+'-rust-tests.log')).read_bytes()).hexdigest()}
        report={'format':'native-civ8-isolated-profile-build-v1','status':'IN_PROGRESS'if len(rows)<len(PROFILES)else'PASS','profiles':rows,'activeWrites':False}
        (ROOT.parent/'profile-build-report.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
        print(json.dumps({'profile':name,**rows[name]}),flush=True)
if __name__=='__main__':run()
