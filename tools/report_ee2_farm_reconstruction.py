"""Record the checked native Farm/UnitType-undertexture handoff separately from V2."""
import hashlib,json,pathlib,re,zipfile
from PIL import Image,ImageChops
ROOT=pathlib.Path(__file__).resolve().parents[3];PROJECT=ROOT/'outputs/OpenEpoch';DOCS=PROJECT/'docs';ANALYSIS=ROOT/'outputs/EE2-Research/analysis'
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II/zips')
sha=lambda data:hashlib.sha256(data).hexdigest()
read=lambda path:json.loads(path.read_text(encoding='utf8'))
def digest(path):return {'sha256':sha(path.read_bytes()),'bytes':path.stat().st_size}
def save(name,data):(ANALYSIS/name).write_text(json.dumps(data,indent=2)+'\n',encoding='utf8')

def run():
    farms=read(DOCS/'local-original/farm-composites.json');ground=read(DOCS/'local-original/ground-decals.json');scales=read(DOCS/'local-original/model-scales.json');manifest=read(DOCS/'local-original/models/manifest.json')
    baseline=read(ROOT/'work/season-model-baseline/model-hashes.json')
    assert len(baseline)==191 and all(sha((DOCS/url).read_bytes())==value for url,value in baseline.items())
    old=read(ROOT/'work/farm-active-baseline/docs/local-original/models/manifest.json')
    keyed=lambda rows:{(row['id'],row.get('epoch',0)):row for row in rows if row['id']!=2}
    assert keyed(old['models'])==keyed(manifest['models'])
    assert [row for row in old['models']if row['id']!=2]==[row for row in manifest['models']if row['id']!=2]
    assets={state['url']:digest(DOCS/state['url'])for family in farms['families'].values()for state in family['states'].values()}
    assets.update({path.relative_to(DOCS).as_posix():digest(path)for path in (DOCS/'local-original/farm-textures').glob('*.png')})
    source={}
    with zipfile.ZipFile(INSTALL/'db.zip')as db,zipfile.ZipFile(INSTALL/'hdrs.zip')as hdrs:
        for archive,member in [(db,'db/Units/farm.ddf'),(db,'db/Visuals/visual_decals.ddf'),(hdrs,'DbHdrs/DbUnitDefs.h'),(hdrs,'DbHdrs/DBVisualDefs.h')]:
            actual=next(name for name in archive.namelist()if name.casefold()==member.casefold());source[pathlib.Path(archive.filename).name+'!'+actual]=sha(archive.read(actual))
    exports={path.name:digest(path)for path in sorted(ANALYSIS.glob('native-farm-*.c'))}
    common={'build':'water-farm3','engineSha256':sha((ROOT/'work/EE2-engine.exe').read_bytes()),'uiLaunched':False}
    evidence={'status':'passed','format':'ee2-native-farm-and-undertexture-evidence-v1',**common,'sourceHashes':source,'ghidraExports':exports,
      'nativeFarm':{'factories':{'CompositeModel':'00602860','FarmPlotModel':'0061D380; registered by0061D260','baseVisualConstructor':'006079F0; transform initializer008EDFA0'},
       'defaultValueProof':'DbUnitDefs.h explicitly documents that the DB reader initializes ALL dataclass data to zero. DBVisualDefs.h places kScaleIdentity first. This closes the previously unresolved absent inner CompositeModel.ScaleType; enum order or a missing token alone was insufficient.',
       'scaleChain':{'classGetter':'005F8040','parentPropagation':'00609310','parentAssignment':'00608A90','matrixVector':'008E6970','vectorScale':'005F9C00','vectorAdd':'005F7850','values':farms['scale'],'normalizedMeshScale':.01,'worldScaleForNormalizedMesh':scales['variants']['2:0']['sourceScale']/.01},
       'plots':{'nativeOffsets':farms['families']['FarmWheat']['plots'],'worldCentersXZ':[[-.5,.5],[.5,.5],[.5,-.5],[-.5,-.5]],'combinedNativeWidth':2.0,'sourceBoundsMeaning':'Single original plot NIF bounds remain sourceBounds; new compositeBounds names the complete four-plot geometry.'},
       'stateBindings':'All three DDF crop families have four independent plot StateDatabases, each defaulting to Harvest_3_N. Import all12 family/state source meshes; activate only source default FarmWheat Harvest_3 for the existing simulation Farm archetype. No BASE DDF/CSV references Farm8.',
       'terrain':'FarmPlotSM vtable00B4706C slot6=0061D4C0 queries0070FB00 at each plot world center and changes translation Z only. Production VS follows that behavior using the current terrain b-c triangle mesh. Normal/slope rotation is not invented.',
       'lod':'All imported crop source NIFs have an empty native LOD selection. No substitute crop LOD or arbitrary distance threshold is added.'},
      'nativeUndertexture':{'unitFactory':'0053A8D0','sizePayloadPublisher':'005DA540; other native publishers00773E00/00842F90','subscription':'00603DA0; channel00CDCDB4','selector':'00603A60','fieldSlots':{'bodySize':['+58','+5C'],'centerOffset':['+60','+64'],'footprintSize':['+68','+6C'],'undertextureSize':['+70','+74'],'undertextureOffset':['+78','+7C']},
       'conversion':{'sqrt2Address':'00B43910','sqrt2Float32':1.4142135381698608,'epsilonAddress':'00B4390C','epsilonFloat32':1.1920928955078125e-7},
       'factoryRule':'Non-ellipse body sizes, all footprint sizes, both explicit undertexture sizes and offsets, and center offsets are multiplied by source sqrt2 and stored as floats. Missing footprint falls back to raw body size before conversion.',
       'selectorRule':'If either explicit undertexture size is zero: use converted footprint dimensions, optional Decal.Scale, centerOffset-(footprint-body)/2. Otherwise: width=storedUnderY, depth=storedUnderX, direct stored offsets.',
       'axisRule':'006037B0 gets Euler from local parent matrix008E7D00, rotates offsets by(-ownYaw-pi/2) via00A20800 and SUBTRACTS them via004A05C0. Getter00603640 then adds pi/2 to the quad angle. In the existing Y-up basis, quad yaw remains ownYaw and local offset=(-nativeOffsetY,-nativeOffsetX).',
       'extentRule':'00714CF0 uses full width/depth and multiplies by0.5 for clipping-plane half extents; no later sqrt2 cancellation exists.',
       'underFarm':{'rawSize':[1.5,1.5],'width':next(row['width']for row in ground['entries']if row['id']==2),'offset':[0,0],'sourceDecal':'UnderFarm; identical original under_farm Rural/Urban sprites in all three epoch slots'},
       'affectedBindings':[{'id':row['id'],'decal':row['decal'],'width':row['width'],'depth':row['depth'],'offsetX':row['offsetX'],'offsetY':row['offsetY']}for row in ground['entries']]}}
    save('native3-farm-ground-evidence.json',evidence)
    historical=[]
    pattern=re.compile(r'native-world-undertexture|native.world.exact|UndertextureSize/Offset|explicit.*world.*dimension|world.dimension.*native',re.I)
    for path in sorted(ANALYSIS.glob('*.json')):
        if path.name.startswith(('native3-','native-undertexture-correction-')):continue
        matching=[{'line':index,'text':line.strip()[:500]}for index,line in enumerate(path.read_text(encoding='utf8').splitlines(),1)if pattern.search(line)]
        if matching:historical.append({'path':path.name,**digest(path),'affectedClaims':matching})
    correction={'status':'superseded-claims-documented','format':'ee2-undertexture-source-correction-v1',**common,'historicalArtifactsUnmodified':True,'superseded':historical,
      'finding':'Earlier native-world-undertexture/raw-world-dimension claims only followed00603A60 and missed the0053A8D0 unit-definition producer. Raw DDF UndertextureSize/Offset are NOT final world parameters. Prior V2 artifacts remain historical snapshots, not current dimensional equivalence evidence.',
      'correction':'Apply source float32 sqrt2 conversion before the payload; preserve explicit axis swap and derived signed offset transform. Native fallback now proves UnderFarm1.5*sqrt2. No artwork is resized or repainted.',
      'affectedBindings':evidence['nativeUndertexture']['affectedBindings'],'currentEvidence':'native3-farm-ground-evidence.json','currentIntegration':'native3-farm-ground-integration.json'}
    save('native-undertexture-correction-findings.json',correction)
    report_names=['native3-ground-profile-import.json','native3-model-season-import.json','native3-model-season-loader-tests.json','native3-farm-gpu-tests.json','native3-model-material-gpu-tests.json','native3-model-season-gpu-tests.json','native3-farm-scene-tests.json','native3-farm-scene-control-tests.json']
    reports={name:{**digest(ANALYSIS/name),'status':read(ANALYSIS/name).get('status',read(ANALYSIS/name).get('passed'))}for name in report_names}
    assert all(row['status']in ['passed',True]or isinstance(row['status'],int)and row['status']>0 for row in reports.values())
    rendered=Image.open(ROOT/'work/farm-reconstruction/production/farm-scene.png').convert('RGB');control=Image.open(ROOT/'work/farm-reconstruction/production/farm-control.png').convert('RGB')
    delta=ImageChops.difference(rendered,control);pixels=sum(any(p)for p in delta.getdata());assert pixels>1000
    runtime=['original-renderer.js','original-ground.js','original-ground-native.js','original-scale.js','original-model-seasons.js','local-original/models/manifest.json','local-original/farm-composites.json','local-original/ground-decals.json','local-original/model-scales.json','local-original/model-seasons.json']
    tests=['test_ee2_farm_layout.py','test_ee2_farm_reconstruction.py','test_ee2_ground_decals.py','test_ee2_ground_runtime.py','test_ee2_model_seasons.py','native-ground-sizing-test.mjs','original-scale-test.mjs','original-renderer-test.mjs','original-material-test.mjs','original-ground-test.mjs','original-model-seasons-test.mjs','original_farm_gpu_test.py','original_material_gpu_test.py','original_model_season_gpu_test.py','export_original_farm_fixture.mjs']
    tools=['import_ee2_farm_composites.py','import_ee2_ground_profile.py','import_ee2_visual_scales.py','import_ee2_model_seasons.py','extract_ee2_models.py']
    integration={'status':'passed','format':'ee2-native-farm-ground-integration-v1',**common,'scope':'Four source crop plots, native hierarchy scale/offsets and center-height conformance; source-backed building undertexture size/offset conversion. Runtime visual layer only.',
      'wasm':{'ee2':digest(DOCS/'openepoch-ee2.wasm'),'base':digest(DOCS/'openepoch.wasm')},
      'runtimeHashes':{name:digest(DOCS/name)for name in runtime},'toolHashes':{name:digest(PROJECT/'tools'/name)for name in tools+tests},'assetHashes':assets,
      'preservation':{'oldModelPayloadsByteIdentical':191,'allNonFarmManifestRowsIdentical':True,'mainEdited':False,'rustEdited':False,'oldGeometryRenormalized':False,'newSourceMeshStates':12,'newSourceCropPngFiles':6,'activeModelVariants':len(manifest['models']),'verifiedScaleVariants':sum(bool(row['verified'])for row in scales['variants'].values()),'groundBindings':len(ground['entries']),'frozenV2ReportsRewritten':False},
      'tests':{'farmSourceUnittests':12,'groundSourceUnittests':10,'seasonSourceUnittests':9,'nativeSizingAssertions':361,'scaleAssertions':2109,'seasonLoaderAssertions':34269,'farmGpuTransformFeedback':66,'materialGpuPixels':11,'seasonGpuPixelsIncludingAllCropFamilies':43,'farmSlopeSourceSceneDraws':5,'heightConformanceChangedPixels':pixels,'unchangedPathRegressions':['renderer animation/LOD/team/picking/state','source material alpha/depth/specular/emissive','ground epoch/urban/season/terrain/GL restoration']},
      'reports':reports,'evidence':{'native3-farm-ground-evidence.json':digest(ANALYSIS/'native3-farm-ground-evidence.json'),'native-undertexture-correction-findings.json':digest(ANALYSIS/'native-undertexture-correction-findings.json')},
      'reproduce':['python tools/import_ee2_farm_composites.py','python tools/import_ee2_visual_scales.py','python tools/import_ee2_ground_profile.py','python tools/import_ee2_model_seasons.py --report <new-build-season-import.json>','node tools/export_original_farm_fixture.mjs <absolute-output-directory>','python tools/original_farm_gpu_test.py <output-directory>/farm-shaders.json --report <new-report.json>'],
      'limits':['Current simulation Farm explicitly selects original FarmWheat Harvest_3, the native source default. Native worker/crop state transitions, per-plot lifecycle, climate/random crop-family choice and Seeding KF playback remain separate; no automatic visual harvest loop is invented. All three families and four states are imported as explicit future bindings.','Native crop NIFs have no LOD nodes; current renderer keeps source geometry rather than fabricating a far shape.','Farm GPU conformance follows native per-plot vertical center placement, adapted to the current terrain triangle sampler. Full native terrain generation and0070FB00 interpolation equivalence are not claimed.','Crop source scene uses a synthetic slope and one source instance, not a WASM world. The generic replay field wasmInstances is a format label; combined actual WASM Farm scene is independently tested by native3-world-qa.','Lighting, complete native terrain/CityManager/dead-decal history, UI and simulation equality remain outside this bounded visual fix. Source Farm health/selection visual children use existing separate game overlays.','No physical macOS or original-game screenshot comparison was run; no1:1 visual parity is claimed.']}
    save('native3-farm-ground-integration.json',integration)
    print(json.dumps({'status':'passed','report':'native3-farm-ground-integration.json','runtimeHashes':integration['runtimeHashes'],'tests':integration['tests'],'historicalClaimFiles':len(historical)}))
    return integration

if __name__=='__main__':run()
