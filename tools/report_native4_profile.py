"""Bind three-profile UI/save/network QA and current UP world rendering evidence."""
import hashlib,json
from datetime import datetime,timezone
from pathlib import Path
P=Path(__file__).resolve().parents[1];A=P.parent/'EE2-Research/analysis'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
load=lambda name:json.loads((A/name).read_text(encoding='utf8'))
names=['native4-rules-profile-tests.json','native4-season-session-tests.json','native4-visual-controls-tests.json','native4-world-qa.json','native4-reflection-world-pixel-tests.json','native4-portable-assets-check.json','up15-independent-source-review.json','up15-independent-execution-review.json','native4-portable-server-tests.json']
reports={name:load(name) for name in names};profiles=reports[names[0]];world=reports['native4-world-qa.json']
assert profiles['passed'] and profiles['productionUp15Enabled'] and profiles['checks']==1364
for name,digest in profiles['sourceHashes'].items():assert sha(P/'docs'/name)==digest,name
assert world['passed'] and world['checks']==324 and len(world['scenes'])==7
for name,digest in world['sourceFiles'].items():assert sha(P/'docs'/name)==digest,name
for row in profiles['binaries'].values():assert sha(P/'docs'/row['file'])==row['sha256']
assert profiles['binaries']['up15']['sha256']=='f9ce07f4aa1105fb0bb9472be6e88fc070cc06046eb8db17a01b4956920fe2c3'
assert profiles['binaries']['up15']['sha256']==world['wasmHashes']['up15']
assert profiles['binaries']['base']['sha256']==world['wasmHashes']['base']
portable=reports['native4-portable-assets-check.json'];run=next(r for r in portable['runs'] if r['label']==portable['latestLabel'])
assert run['status']=='passed' and not run['errors'] and run['counts']['numericalRuleProfileBinaries']==3
assert run['checkerSha256']==sha(P/'tools/portable-assets-test.py')
assets={row['path']:row for row in run['assets']}
for name in ['main.js','index.html','rules-profile.js','local-original/models/manifest.json','local-original/model-seasons.json','local-original/hud-native/manifest.json']:
    assert assets[name]['sha256']==sha(P/'docs'/name),name
for name in ['native4-season-session-tests.json','native4-visual-controls-tests.json']:
    row=reports[name];assert row['passed'];assert row.get('mainSha256',row.get('sources',{}).get('docs/main.js'))==profiles['sourceHashes']['main.js']
assert reports['native4-reflection-world-pixel-tests.json']['passed']
server=reports['native4-portable-server-tests.json']
assert server['passed'] and server['serverVersion']==2 and server['testsRun']==12
for name,digest in server['sourceHashes'].items():assert sha(P/name)==digest,name
for row in profiles['binaries'].values():assert server['wasmSha256ByFile'][row['file']]==row['sha256']
report={'format':'native4-three-profile-integration-v1','build':'native-ui4','status':'PASS','createdUTC':datetime.now(timezone.utc).isoformat(),
 'behavior':{'defaultEE2':'?rules=ee2 -> installed UP profile2','explicitBASE':'?rules=ee2&profile=base -> retained EE2 reference1','independent':'Without rules=ee2 -> independent profile0','selector':'Visible Regelversion menu reloads the explicit chosen URL and preserves QA/seed/epoch/hash parameters.','loader':'One explicitly selected file, strict exported wasm.ruleset ID. Missing/corrupt/mismatched UP file displays a launch error; never falls back to BASE. MIME decoding fallback reuses only the same response bytes.','saveIsolation':'Three database names, each with a distinct -qa namespace. New records carry exact rules/profile metadata; the WASM magic additionally rejects relabeled foreign bytes. Legacy own/BASE records stay compatible in their original namespaces. No legacy implicit UP import.','multiplayer':'Production protocol9 hello/start rejects other numerical rulesets. Same-profile real player commands remain deterministic through the actual production Lockstep.'},
 'profiles':[
  {'id':'own','ruleset':0,'wasm':profiles['binaries']['own'],'database':'openepoch-v2'},
  {'id':'base','ruleset':1,'wasm':profiles['binaries']['base'],'database':'openepoch-ee2-reference-v2'},
  {'id':'up15','ruleset':2,'wasm':profiles['binaries']['up15'],'database':'openepoch-ee2-up15-v1'}],
 'validation':{'profileLoaderSelectorIndexedDBSaveContinueNetworkChecks':1364,'existingSeasonSessionChecks':425,'existingCameraControlSaveChecks':73,'independentRawSourceChecks':reports['up15-independent-source-review.json']['checks'],'independentCandidateWasmChecks':reports['up15-independent-execution-review.json']['checks'],'gpuWorldChecks':324,'gpuWorldScenes':7,'gpuReflectionPixelControls':106,'portableSelfTests':20,'standaloneServerHttpCases':12,'portable':{k:run[k] for k in ['fileCount','localOriginalFileCount','runtimeReferenceCount','uniqueRuntimeTargets']},'actualNamespaces':profiles['openedDatabases'],'network':profiles['network']},
 'sourceFiles':{**world['sourceFiles'],**{name:sha(P/'docs'/name) for name in ['reference.html','catalog.html']}},'wasmHashes':{key:row['sha256'] for key,row in profiles['binaries'].items()},
 'portableServer':{'version':2,'statusField':'wasmSha256ByFile includes all three numerical profiles; legacy wasmSha256 remains BASE-reference1.','reuse':'Exact application/version/rootToken/legacy hash/three-profile hashes; equal current server reuses, old or changed-UP server does not.','startup':'UP binary is mandatory because installed UP is the default EE2 profile. Build URL uses native-ui4.','report':'native4-portable-server-tests.json'},
 'reports':{name:{'path':name,'sha256':sha(A/name)} for name in reports},
 'toolHashes':{name:sha(P/'tools'/name) for name in ['rules-profile-test.mjs','headless-world-capture.mjs','native4-world-qa.mjs','native4_reflection_world_gpu_test.py','replay_world_gles.py','portable-assets-test.py','serve_game.py','portable-server-test.py','report_native4_profile.py']},
 'scope':'Production JS profile loading and actual main startup selector/start/IndexedDB/save/load/Continue/channel execution in VM; real0/1/2 WASMs and six Lockstep peers. Actual ANGLE GLES FBO world rendering for UP2, BASE1 control and reflection pixel controls. DOM/IndexedDB event transport and queued network are substituted; no GUI, browser, WebRTC socket or original executable is launched.',
 'explicitLimits':['UP CSV5/CFG5 numerical imports project onto the existing57 role aliases; this is not the complete original roster or native simulation.','Original graphics retain the declared visual adapters; numerical UP activation does not imply a newly reconstructed UP visual roster.','Original map generator, native separate reflection culling, UV defaults/time-vector population, atmosphere/weather and complete special abilities remain outside this change.','In-memory IndexedDB event adapter proves production namespace/transaction selection; physical browser disk persistence and Mac/Safari execution require separate device checks.','Static reference/catalog pages retain the BASE source dataset; they do not become a UP data catalog through this runtime profile switch.'],
 'parityClaim':'Functioning isolated numerical rule profiles and validated source slices; no1:1 or finished-original claim.'}
(A/'native4-profile-integration.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
# Refresh the HUD portability summary against the now-current checker/three-profile
# graph, preserving older V2/V3 reports entirely.
hud=load('native4-hud-portability.json');hud['selfTests']=20
hud['files']={name:sha(P/name) for name in hud['files']}
hud['portableReport']={'path':'native4-portable-assets-check.json','sha256':sha(A/'native4-portable-assets-check.json'),**{key:run[key] for key in ['fileCount','runtimeReferenceCount','uniqueRuntimeTargets']},'errors':0}
hud['numericalRuleProfileBinaries']=3
(A/'native4-hud-portability.json').write_text(json.dumps(hud,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(json.dumps({'status':'PASS','report':str(A/'native4-profile-integration.json'),'checks':1364,'worldChecks':324,'checkerSha256':run['checkerSha256'],'mainSha256':profiles['sourceHashes']['main.js']}))
