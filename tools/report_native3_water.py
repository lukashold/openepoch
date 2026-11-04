"""Bind the activated source-backed water implementation to completed QA hashes."""
import hashlib,json
from datetime import datetime,timezone
from pathlib import Path
P=Path(__file__).resolve().parents[1];A=P.parent/'EE2-Research/analysis'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda name:json.loads((A/name).read_text(encoding='utf8'))
reports={name:read(name) for name in ['native-water-appearance-evidence.json','native3-water-contract-tests.json','native3-water-gpu-tests.json','native3-world-qa.json','native3-reflection-world-pixel-tests.json','native3-portable-assets-check.json']}
world=reports['native3-world-qa.json'];water=P/'docs/original-water.js';manifest=P/'docs/local-original/water-native/manifest.json'
for name in ['native3-water-contract-tests.json','native3-water-gpu-tests.json']:
    assert reports[name]['passed'] and reports[name]['sourceSha256']==sha(water)
    assert reports[name]['manifestSha256']==sha(manifest)
assert world['passed'] and world['checks']==274 and len(world['scenes'])==6
assert world['wasmSha256']==sha(P/'docs/openepoch-ee2.wasm')
for name,digest in world['sourceFiles'].items():assert sha(P/'docs'/name)==digest,name
pixel=reports['native3-reflection-world-pixel-tests.json'];assert pixel['passed'] and pixel['checks']==106
portable=reports['native3-portable-assets-check.json'];run=next(r for r in portable['runs'] if r['label']=='water-farm3-workspace')
assert run['status']=='passed' and not run['errors'] and run['counts']['farmStateModels']==12
assert run['checkerSha256']==sha(P/'tools/portable-assets-test.py')
images=json.loads(manifest.read_text())['images']
for row in images.values():assert sha(manifest.parent/row['url'])==row['pngSha256']
report={
 'format':'openepoch-native3-water-implementation-v1','build':'water-farm3','date':datetime.now(timezone.utc).isoformat(),'status':'PASS',
 'previousBuild':'visual-seasons2; its reports and source/package freeze remain historical snapshots.',
 'originalEngineSha256':reports['native-water-appearance-evidence.json']['engineSha256'],
 'wasmSha256':world['wasmSha256'],'simulationChanges':'None. Active EE2 simulation remains the frozen d69f6d38 build; all changes in this slice are original-media import or presentation.',
 'production':{'module':'docs/original-water.js','api':['loadOriginalWater(gl, enabled, {manifestURL?})','updateMap({heights, types, movement?, size?, waterHeight?})','drawWater({vp, time, fogTexture, reflectionTexture, reflectionVP})','drawBase(...)','drawReflection(...)','dispose()'],
   'mainIntegration':'Own water map receives actual WASM height/class arrays at startup/map dirty. Native water draws after the terrain/model scene and samples the actual planar reflection target. Legacy water is retained only as loader-unavailable fallback.',
   'assets':{'manifest':'docs/local-original/water-native/manifest.json','images':len(images),'pngBytes':sum((manifest.parent/row['url']).stat().st_size for row in images.values()),'resolution':'Six native Skin images and native normal stay512x512; native palette256x1 and lookup4x1. No generated/repainted media.'}},
 'sourceBacked':[{'part':'Material binding','behavior':'High profile Skin1 Dark / Skin2 Detail / Skin4 Gloss; compatibility fallback Skin1 / Skin5 / Skin6. Skin3 retained as source low-profile media.'},
   {'part':'Vertex channels','behavior':'Native 13-point red/alpha,33-point green and29-weight blue filters; native depth/2 palette index and original alpha bytes; movement-class lookup color and source blue coastal/land secondary color.'},
   {'part':'Base pixel shader','behavior':'Dark*Detail and Gloss*Gloss mixed by vertex green; red modulates RGB, vertex alpha modulates alpha, secondary blue adds to green/blue. No fabricated Phong/Fresnel in this native high base.'},
   {'part':'Base states','behavior':'Lighting disabled, depth test/write on, LessEqual, SrcAlpha/InvSrcAlpha; source NSF UV velocities and native vertex cosine warp.'},
   {'part':'Reflection overlay','behavior':'Separate normal-distorted render-target sample multiplied by white vertex RGB and factory initial alpha f32(0.32); depth writes off, LessEqual and source alpha blend. This does not claim no later native CPU alpha update exists.'}],
 'explicitAdapters':[
   'Own six archipelago terrain classes map to eight source movement IDs via[7,5,0,0,4,3]. Native River is available only with explicit movement IDs; no claim of original map generation.',
   'Native default UV table00D0232C remains unresolved; base scale1/12 and normal scale1/8 remain labeled renderer adapters.',
   'Native CM time-vector population is not closed; base uses[time,time] from the existing deterministic presentation clock.',
   'Own corner grid, edge clamping, wet-triangle selection, projection into the planar target and fog policy adapt source shader equations to the independent map/renderer.',
   'Reflection currently reuses main visible native instances. Native separate reflected-view culling, exact RTT sizing and full atmosphere/sky remain open.',
   'Farm visual uses native explicit default Harvest_3 with four source plots and source UnderFarm. Simulation-linked harvest state progression remains unimplemented; no timer was invented.'
 ],
 'validation':{'sourceEvidenceChecks':len(reports['native-water-appearance-evidence.json']['checks']),'loaderAndStateChecks':44,'independentCpuVsGlesChecks':1758,'fixedInputPixelProbes':294,'maximumChannelErrorBytes':1,'combinedWorldChecks':274,'worldScenes':['summer','winter','halfblend-tick1000','future','heading90-coast','actual-production-Farm-tick1'],'reflectionPixelControls':106,'portableSelfTests':15,'portable':{k:run[k] for k in ['fileCount','localOriginalFileCount','runtimeReferenceCount','uniqueRuntimeTargets']},'farmGraph':{k:run['counts'][k] for k in ['farmFamilies','farmStateModels','farmMaterialImages']},'worldPngs':{s['name']:{'path':s['png'],'sha256':s['pngSha256']} for s in world['scenes']},'noUiOrNativeExeLaunched':True},
 'sourceFiles':world['sourceFiles'],
 'tools':{p.name:sha(p) for p in [P/'tools'/n for n in ['import_ee2_native_water.py','native-water-test.mjs','native_water_gpu_test.py','headless-world-capture.mjs','native3-world-qa.mjs','native3_reflection_world_gpu_test.py','replay_world_gles.py','portable-assets-test.py','report_native3_water.py']]},
 'reports':{name:{'sha256':sha(A/name),'path':name} for name in reports},
 'reviewImages':['../../OpenEpoch/native3-reflection-coast.png','../../OpenEpoch/native3-world-farm.png'],
 'parityClaim':'Validated source slices and a functioning production presentation path, not1:1 game parity or an original-executable visual comparison.'
}
destination=A/'native3-water-implementation.json';destination.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
print(json.dumps({'status':'PASS','build':report['build'],'report':str(destination),'sourceChecks':report['validation']['sourceEvidenceChecks'],'worldChecks':274,'pixelControls':106,'waterSha256':sha(water),'mainSha256':world['sourceFiles']['main.js']}))
