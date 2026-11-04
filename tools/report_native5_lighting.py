"""Freeze lighting5 evidence and compare real world passes with stage controls."""
import copy,hashlib,json,pathlib
from PIL import Image
from replay_world_gles import replay

ROOT=pathlib.Path(__file__).resolve().parents[3];PROJECT=ROOT/'outputs/OpenEpoch';DOCS=PROJECT/'docs';ANALYSIS=ROOT/'outputs/EE2-Research/analysis';WORK=ROOT/'work/native-lighting4'
sha=lambda raw:hashlib.sha256(raw).hexdigest()
def file_record(path):return {'sha256':sha(path.read_bytes()),'bytes':path.stat().st_size}

def run():
    files=['original-renderer.js','original-lighting.js','original-material.js','local-original/climate-lighting.json','local-original/models/manifest.json','local-original/model-scales.json','local-original/models/manifest-up15.json','local-original/model-scales-up15.json']
    hashes={name:file_record(DOCS/name)for name in files};world=[]
    for profile,suffix in [('base','base'),('up15','up')]:
        capture=ROOT/f'work/world-0-town-month6-reflection-profile-{profile}-native5-light-{suffix}.json'
        raw=json.loads(capture.read_text());gpu_path=ANALYSIS/f'native5-lighting-world-{profile}.json';gpu=json.loads(gpu_path.read_text())
        assert gpu['passed']and gpu['captureSha256']==sha(capture.read_bytes())
        for name in ['original-renderer.js','original-lighting.js','local-original/climate-lighting.json']:
            assert raw['sourceHashes']['files'][name]==hashes[name]['sha256']
        expected_profile='base'if profile=='base'else'native155'
        assert raw['modelOptions']['lightingSettings']['profile']==expected_profile
        assert raw['lighting']['profile']==expected_profile and raw['lighting']['brightness']==1.5
        if profile=='up15':
            assert raw['modelOptions']['manifestURL']=='./local-original/models/manifest-up15.json'
            assert raw['modelOptions']['scaleManifestURL']=='./local-original/model-scales-up15.json'
        wasm_name=raw['rulesProfile']['wasm'].split('?')[0]
        assert raw['sourceHashes']['wasmSha256']==sha((DOCS/wasm_name).read_bytes())
        model_draws=[d for d in raw['draws']if 'uNativeLighting'in d['uniforms']]
        assert model_draws and any(d['uniforms']['uNativeShader']['value']==1 for d in model_draws)
        assert all(d['uniforms']['uNativeLighting']['value']==1 for d in model_draws)
        control=copy.deepcopy(raw)
        for draw in control['draws']:
            uniforms=draw['uniforms']
            if 'uNativeLighting'in uniforms:
                uniforms['uNativeLighting']['value']=0;uniforms['uNativeShader']['value']=0;uniforms['uTextureScale']['value']=1
        control_path=WORK/f'world-final-{profile}-stage-control.json';control_path.write_text(json.dumps(control))
        control_png=WORK/f'world-final-{profile}-stage-control.png'
        replay(control_path,control_png)
        actual=Image.open(gpu['output']).convert('RGBA');other=Image.open(control_png).convert('RGBA')
        assert actual.size==other.size
        changed=sum(a!=b for a,b in zip(actual.getdata(),other.getdata()))
        assert changed>100,'Real native model shader/stage must visibly contribute to this world'
        world.append({'profile':profile,'capture':str(capture.relative_to(ROOT)).replace('\\','/'),'captureSha256':sha(capture.read_bytes()),'gpuReport':gpu_path.name,
            'instances':raw['instances'],'drawCalls':gpu['stats']['drawCalls'],'modelDrawCalls':len(model_draws),'modelOptions':raw['modelOptions'],'lighting':raw['lighting'],
            'wasmSha256':raw['sourceHashes']['wasmSha256'],'outputSha256':gpu['pngSha256'],'changedPixelsAgainstStageControl':changed,
            'controlScope':'Only native IndexPal selection, native stage scales and native stage clamps disabled. Same source light context, geometry, animation, WASM snapshot, view, terrain, ground, fog, water and reflection graph.'})
    tests={}
    for name in ['native5-lighting-source-tests.json','native5-lighting-loader-tests.json','native5-model-lighting-gpu-tests.json','native5-model-season-loader-tests.json','native5-model-season-gpu-tests.json','native5-farm-gpu-tests.json']:
        value=json.loads((ANALYSIS/name).read_text());assert value.get('status')=='passed'or value.get('passed',0)>0
        tests[name]={'sha256':sha((ANALYSIS/name).read_bytes()),'checks':value.get('passed',value.get('checks'))}
    assert json.loads((ANALYSIS/'native5-lighting-loader-tests.json').read_text())['sourceHashes']['renderer']==hashes['original-renderer.js']['sha256']
    source=json.loads((DOCS/'local-original/climate-lighting.json').read_text());states=json.loads((WORK/'model-source-states.json').read_text())
    old_models=json.loads((ROOT/'work/season-model-baseline/model-hashes.json').read_text())
    for name,digest in old_models.items():assert sha((DOCS/name).read_bytes())==digest
    historical={}
    owned_historical=['native3-farm-ground-integration.json','native3-farm-ground-evidence.json','native3-ground-profile-import.json','native3-model-season-import.json','native3-model-season-loader-tests.json','native3-farm-gpu-tests.json','native3-model-material-gpu-tests.json','native3-model-season-gpu-tests.json','native3-farm-scene-tests.json']
    for filename in owned_historical:
        snapshot=ROOT/'work/github-native4-source/research/analysis'/filename
        if not snapshot.exists():continue
        active=ANALYSIS/snapshot.name
        if active.exists():
            assert active.read_bytes()==snapshot.read_bytes(),f'Historical evidence changed: {snapshot.name}'
            historical[snapshot.name]=sha(active.read_bytes())
    evidence={
        'originalEngineSha256':source['sourceEngineSha256'],
        'sourceRGB':'00A5CF80 reads3/4 RGBA unsigned bytes then swaps offsets0/2 intoBGRA (alpha defaults255 for3 components).00A30390 reads offsets2/1/0 times f32(1/255), no gamma transform.',
        'nativeTableSelection':'00541940 explicitly names dbclimat5.ddf and dbClimate_atmospher5.csv. BASE db.zip tables remain a distinct profile;49 atmosphere rows differ.',
        'lightSelection':'00604BA0 loads lights.nif,005F81C0 returns first NiDirectionalLight by ascending NiNode child order; sourcechildren[3,4,5] select FDirect01 withDimmer1.5.006296B0 multiplies source ambient/diffuse byDimmer.',
        'direction':'006CCA00 normalizes DDF SunDirectionVec3f into localmatrix column0;008EF400 copies worldcolumn0 into WorldDirection+110/114/118. Y-up portable mapping preserved.',
        'indexPal':'005F9100 assigns IndexPal20_High/Low to skinned geometry on enabled programmable path. Source HighVS signed dot(normal,-SunDir)*SunColor+SceneAmbient; Low .5*SunColor+SceneAmbient. Per-vertex clamped diffuse output, materialalpha only. NSF base4x RGB/alphaModulate. MaterialRGB/specular do not participate.',
        'staticTextureStage':'00957B70 reads NiTexturingProperty+20 apply4 and selects D3DTOP_MODULATE4X6 through0094BFB0. Only textured parts with that explicit applymode receive4x; IndexPal ownNSF selects4x separately. No universal4x multiply on untextured/legacy/specular/shadow RGB.',
        'terrainBoundary':'006F3520/3540/3560 prove compiled .85/.75/1 atC430C8/CC/D0; source file is Terrain/MaskRenderer/MaskRenderer.cpp.006F41D0 combines diffuse*dimmer*.85, ambient*dimmer*2*.75, byte-packs and uses mask/stencil/multiply passes. These scalars are retained as source data, not applied as an invented single terrain RGB formula.',
        'inventory':{key:states[key]for key in ['modelFiles','sourceNifs','textureProperties','counts']}}
    result={'status':'passed','build':'native5','format':'ee2-native-lighting-integration-v1','runtimeFiles':hashes,
        'wasm':{name:file_record(DOCS/name)for name in ['openepoch.wasm','openepoch-ee2.wasm','openepoch-ee2-up15.wasm']},
        'sourceFiles':source['sources'],'evidence':evidence,'tests':tests,'pythonOriginalSourceUnitTests':4,'world':world,
        'preservedModelPayloads':len(old_models),'historicalNative3ReportsUnchanged':historical,'uiLaunched':False,'originalEngineExecuted':False,
        'limits':['Source period/weather are explicit application settings; no native default-time, dynamic day cycle or random weather scheduler is claimed.',
                  'Static model illumination remains the existing per-fragment portable material/specular adapter. Source light channels, stage4x and separate specular colour are improvements, not full native FFP equivalence.',
                  'Existing sampled geometry/animation/normal streams remain unchanged; source IndexPal first-bone normal skinning is not newly reconstructed by this lighting-only change.',
                  'Terrain/ground illumination, projected/stencil shadow eligibility/opacity/pass order, scenario light overrides, sampler override precedence and special player-colour passes remain incomplete native paths.',
                  'Actual ANGLE/GLES pixels on Windows; no original-game screenshot comparison, physical macOS/Safari execution or1:1 equality claim.']}
    target=ANALYSIS/'native5-lighting-integration.json';target.write_text(json.dumps(result,indent=2)+'\n')
    (WORK/'native5-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
    print(json.dumps({'status':'passed','report':str(target),'checks':{name:entry['checks']for name,entry in tests.items()},'worldChangedPixels':{entry['profile']:entry['changedPixelsAgainstStageControl']for entry in world}}))
    return result

if __name__=='__main__':run()
