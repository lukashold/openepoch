"""Rebuild the independent own visual WASM and update its browser integrity pin."""
from pathlib import Path
import hashlib,json,re,shutil,subprocess

ROOT=Path(__file__).resolve().parents[1]

def main():
    crate=ROOT/'visual-cloth7'
    subprocess.run(['cargo','build','--offline','--release','--target','wasm32-unknown-unknown'],cwd=crate,check=True)
    # Cargo honors CARGO_TARGET_DIR; ask it for the effective directory rather
    # than assuming that the build product lives under the source crate.
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--no-deps','--format-version','1'],cwd=crate,text=True))
    binary=Path(metadata['target_directory'])/'wasm32-unknown-unknown/release/cloth_fast7.wasm'
    digest=hashlib.sha256(binary.read_bytes()).hexdigest()
    destination=ROOT/'docs/openepoch-cloth7.wasm'
    module=ROOT/'docs/source-sail-runtime.js';text=module.read_text(encoding='utf8')
    expression=r"(export const SOURCE_CLOTH_MODULE=Object\.freeze\(\{\s*url:new URL\([^\n]+\),bytes:)\d+(,\s*sha256:')[a-f0-9]{64}(')"
    updated,count=re.subn(expression,lambda m:m[1]+str(binary.stat().st_size)+m[2]+digest+m[3],text)
    if count!=1:raise SystemExit('Cannot update own visual WASM integrity descriptor')
    shutil.copyfile(binary,destination)
    module.write_text(updated,encoding='utf8',newline='\n')
    print(json.dumps({'file':'docs/openepoch-cloth7.wasm','bytes':destination.stat().st_size,'sha256':digest,'scope':'independent own visual module'}))

if __name__=='__main__':main()
