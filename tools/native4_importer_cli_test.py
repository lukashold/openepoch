"""Real CLI regeneration in thin isolated output trees; never execute the engine."""
import hashlib,inspect,json,pathlib,shutil,subprocess,sys
PROJECT=pathlib.Path(__file__).resolve().parents[1];ROOT=PROJECT.parents[1];DOCS=PROJECT/'docs';TOOLS=PROJECT/'tools'
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II');ENGINE=INSTALL/'Unofficial Patch Files/EnabledUP15Units/EE2.exe';WORK=ROOT/'work/native4-importer-cli'
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
def run():
    WORK.mkdir(exist_ok=True);reports=[];checks=0
    frozen={'local-original/models/manifest.json':'f681e82a0676d78c05590f3f1d736be665d7cb533346c73cc49dbe317d083d53','local-original/model-seasons.json':'17e4869e547f12eed82ea79a693677f4afebcd804dd1ea9a295d8d3079531e3c'}
    expected=['local-original/models/manifest.json','local-original/model-seasons.json','local-original/model-scales.json','local-original/farm-composites.json','local-original/ground-decals.json']
    farm=json.loads((DOCS/'local-original/farm-composites.json').read_text())
    expected.extend(state['url']for family in farm['families'].values()for state in family['states'].values())
    expected.extend(path.relative_to(DOCS).as_posix()for path in (DOCS/'local-original/farm-textures').glob('*.png'))
    # Include every old model payload and ground PNG in the no-write check.
    active=set(expected)|{'original-renderer.js','original-ground.js','original-material.js','main.js','openepoch-ee2.wasm','openepoch.wasm'}
    old=json.loads((ROOT/'work/season-model-baseline/model-hashes.json').read_text());active.update(old)
    ground_audit=json.loads((DOCS/'local-original/ground-decals-manifest.json').read_text());active.update(ground_audit['textures'])
    before={path:sha(DOCS/path)for path in sorted(active)}
    for path,value in frozen.items():assert before[path]==value;checks+=1
    archives=[INSTALL/'zips'/name for name in ['db.zip','hdrs.zip','graphics.zip','textures.zip']]
    source_before={str(path):{'bytes':path.stat().st_size,'mtimeNs':path.stat().st_mtime_ns}for path in [*archives,ENGINE]}
    historical=ROOT/'outputs/EE2-Research/analysis/native3-farm-ground-integration.json';old_report_sha=sha(historical)
    def call(script,args,success=True):
        result=subprocess.run([sys.executable,str(TOOLS/script),*map(str,args)],capture_output=True,text=True,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        if success and result.returncode:raise AssertionError(result.stdout+'\n'+result.stderr)
        if not success and not result.returncode:raise AssertionError('Expected source validation failure')
        return {'script':script,'arguments':[str(x)for x in args],'exitCode':result.returncode,'stdout':result.stdout.strip(),'stderr':result.stderr.strip()}
    engine_copy=WORK/'read-only-engine-evidence.bin';shutil.copyfile(ENGINE,engine_copy)
    assert sha(engine_copy)=='d41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f';checks+=1
    for label,extra,engine in [('explicit',['--install',INSTALL],['--engine',engine_copy]),('default',[],[])]:
        out=WORK/label/'docs';(out/'local-original/models').mkdir(parents=True,exist_ok=True)
        for name in ['models/manifest.json','model-seasons.json','model-scales.json']:shutil.copyfile(DOCS/'local-original'/name,out/'local-original'/name)
        commands=[call('import_ee2_farm_composites.py',['--docs',out,*extra]),call('import_ee2_ground_profile.py',['--docs',out,'--report',WORK/label/'ground-import-report.json',*extra,*engine])]
        hashes={}
        for name in expected:
            actual=sha(out/name);assert actual==before[name],(label,name,actual,before[name]);hashes[name]=actual;checks+=1
        # Intentional second CLI invocation catches ordering/provenance loss.
        commands.append(call('import_ee2_farm_composites.py',['--docs',out,*extra]))
        for name in expected:
            assert sha(out/name)==hashes[name],('idempotence',label,name);checks+=1
        files=[p for p in out.rglob('*')if p.is_file()]
        reports.append({'label':label,'commands':commands,'outputFiles':len(files),'outputBytes':sum(p.stat().st_size for p in files),'comparedHashes':hashes,'oldModelPayloadsCopied':False,'sourceEngineExecuted':False})
    invalid=WORK/'invalid-source';invalid.mkdir(exist_ok=True)
    failures=[call('import_ee2_farm_composites.py',['--docs',invalid/'farm','--install',WORK/'missing-install'],False),call('import_ee2_ground_profile.py',['--docs',invalid/'ground-install','--install',WORK/'missing-install','--engine',engine_copy,'--report',invalid/'report.json'],False)]
    wrong_engine=invalid/'bad-engine.bin';wrong_engine.write_bytes(b'not a native PE source')
    failures.append(call('import_ee2_ground_profile.py',['--docs',invalid/'ground-engine','--install',INSTALL,'--engine',wrong_engine,'--report',invalid/'report.json'],False));checks+=len(failures)
    after={path:sha(DOCS/path)for path in sorted(active)};assert after==before;checks+=len(before)
    assert sha(historical)==old_report_sha;checks+=1
    for path,expected_stat in source_before.items():
        original=pathlib.Path(path);assert {'bytes':original.stat().st_size,'mtimeNs':original.stat().st_mtime_ns}==expected_stat;checks+=1
    import import_ee2_farm_composites as farm_importer,import_ee2_ground_profile as ground_importer
    assert list(inspect.signature(farm_importer.build).parameters)[:1]==['destination'];assert list(inspect.signature(ground_importer.build).parameters)[:2]==['destination','report_path'];checks+=2
    report={'status':'passed','build':'native4','format':'ee2-farm-ground-importer-cli-tests-v1','checks':checks,'defaultInstall':str(INSTALL),'defaultEngine':str(ENGINE),'originalEngineReadOnlySha256':sha(ENGINE),'engineExecuted':False,'uiLaunched':False,'historicalNative3ReportUnchanged':old_report_sha,'freeze':frozen,'toolHashes':{name:sha(TOOLS/name)for name in ['import_ee2_farm_composites.py','import_ee2_ground_profile.py','native4_importer_cli_test.py']},'cases':reports,'expectedFailures':failures,'activeFilesUnchanged':len(before),'activeSourceHashes':before,'sourceStatsUnchanged':source_before,'scope':'Explicit --install/--engine and default CLIs reproduce complete current Farm/Ground source payloads/manifests byte-for-byte in thin isolated outputs; twice-run Farm CLI preserves original draw order and complete verified scale provenance. No active assets, original installation, engine process or native3 report is modified.'}
    destination=ROOT/'outputs/EE2-Research/analysis/native4-importer-cli-tests.json';destination.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'status':'passed','checks':checks,'activeFilesUnchanged':len(before),'cases':[{k:case[k]for k in ['label','outputFiles','outputBytes']}for case in reports],'report':str(destination)}));return report

if __name__=='__main__':run()
