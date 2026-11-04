"""Regenerate source-cache and current-GDI portable bitmap fonts.

The installed game is read-only. No TTF is copied, installed or distributed.
--verify regenerates into an isolated workspace directory and compares every
runtime font file without changing docs. GDI export needs Windows; playback of
the generated PNGs/JSON has no Windows or installed-font dependency.
"""
from pathlib import Path
import argparse,hashlib,importlib,json,os,runpy,sys,tempfile

PROJECT=Path(__file__).resolve().parents[1];WORKSPACE=PROJECT.parents[1]
PACKAGE=PROJECT/'tools/ee2_fonts';LIVE=PROJECT/'docs/local-original/fonts-native'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def generate(output,reports,resolved,cache_only):
    output.resolve().relative_to(WORKSPACE.resolve())
    os.environ['OPEN_EPOCH_FONT_OUT']=str(output);os.environ['OPEN_EPOCH_FONT_REPORTS']=str(reports);os.environ['OPEN_EPOCH_FONT_RESOLVED']=str(resolved)
    sys.path.insert(0,str(PACKAGE));cache=importlib.import_module('font_cache');cache.main()
    if cache_only:return
    if os.name!='nt':raise SystemExit('Current-GDI source regeneration requires Windows. Generated game assets run on macOS without GDI.')
    runpy.run_path(str(PACKAGE/'font_sources.py'),run_name='__main__');importlib.import_module('font_bitmap').main()

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--verify',action='store_true');parser.add_argument('--cache-only',action='store_true');parser.add_argument('--resolved',type=Path,default=WORKSPACE/'work/up-resolved-sources/effective-db-sources.json');args=parser.parse_args()
    if not args.verify:
        generate(LIVE,PROJECT.parent/'EE2-Research/analysis',args.resolved,args.cache_only);return
    scratch=WORKSPACE/'work';scratch.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=scratch,prefix='native6-font-verify-') as directory:
        generated=Path(directory)/'docs/local-original/fonts-native';generate(generated,Path(directory)/'reports',args.resolved,args.cache_only)
        if args.cache_only:
            data=json.loads((generated/'manifest.json').read_text(encoding='utf8'));files=['manifest.json',*[row['file'] for row in data['fonts'].values()]]
        else:
            cache=json.loads((generated/'manifest.json').read_text(encoding='utf8'));data=json.loads((generated/'bitmap-manifest.json').read_text(encoding='utf8'));files=sorted({'manifest.json','bitmap-manifest.json',*(row['file'] for row in cache['fonts'].values()),*(row['file'] for row in data['images'].values())})
        changed=[name for name in files if not (LIVE/name).is_file() or sha(generated/name)!=sha(LIVE/name)]
        report={'format':'native6-font-regeneration-tests-v1','status':'passed' if not changed else 'failed','filesCompared':len(files),'changed':changed,'liveRuntimeUntouched':True,'systemFontFilesCopied':0,'extractorSha256':sha(Path(__file__)),'scope':'Source ZIP metadata/cache plus source-derived current-system GDI raster regenerated in an isolated directory. Every original and supplemental runtime font file compared byte-for-byte.'}
        target=PROJECT.parent/'EE2-Research/analysis/native6-font-regeneration-tests.json';target.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report,indent=2))
        if changed:raise SystemExit(1)

if __name__=='__main__':main()
