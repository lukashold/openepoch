"""Offline isolated profile verification; never replaces active/default WASMs."""
from pathlib import Path
import hashlib,json,subprocess,shutil
ROOT=Path(__file__).resolve().parents[1]
WORK=ROOT.parent
OUT=WORK/'builds'

def run():
    OUT.mkdir(parents=True,exist_ok=True)
    entries={}
    for name,feature,filename in [('own','','rebuilt-own.wasm'),('base','ee2-reference','rebuilt-base.wasm'),('up5','ee2-up15','rebuilt-up5.wasm'),('classic','ee2-roster6','openepoch-classic6.wasm')]:
        args=['--features',feature]if feature else[]
        for mode,command in [('tests',['cargo','test','--offline',*args]),('build',['cargo','build','--offline','--release','--target','wasm32-unknown-unknown',*args])]:
            with (WORK/f'{name}-{mode}.log').open('w',encoding='utf8')as log:
                subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
        source=ROOT/'target/wasm32-unknown-unknown/release/openepoch.wasm'
        target=OUT/filename;shutil.copyfile(source,target)
        entries[name]={'path':str(target),'sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'bytes':target.stat().st_size}
        print(json.dumps({name:entries[name]}),flush=True)
    (WORK/'build-report.json').write_text(json.dumps({'format':'native-classic6-isolated-builds-v1','profiles':entries,'originalExecuted':False,'activeRuntimeModified':False},indent=2)+'\n',encoding='utf8')

if __name__=='__main__':run()
