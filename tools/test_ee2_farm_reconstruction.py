"""Independent archive/NIF/PE proof for four-plot native Farm and ground sizes."""
import copy,hashlib,json,os,pathlib,re,struct,unittest,zipfile
from PIL import Image
import numpy as np
import extract_ee2_models as exporter
from extract_ee2_textures import decode_texcache
from import_ee2 import block,without_comments
from test_ee2_city_evidence import PE
ROOT=pathlib.Path(__file__).resolve().parents[3];DOCS=pathlib.Path(os.environ.get('EE2_FARM_DOCS',ROOT/'outputs/OpenEpoch/docs'))
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II/zips');sha=lambda b:hashlib.sha256(b).hexdigest()
f32=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]

class NativeFarmReconstruction(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.db=zipfile.ZipFile(INSTALL/'db.zip');cls.graphics=zipfile.ZipFile(INSTALL/'graphics.zip');cls.textures=zipfile.ZipFile(INSTALL/'textures.zip');cls.headers=zipfile.ZipFile(INSTALL/'hdrs.zip')
        cls.farms=json.loads((DOCS/'local-original/farm-composites.json').read_text());cls.models=json.loads((DOCS/'local-original/models/manifest.json').read_text());cls.ground=json.loads((DOCS/'local-original/ground-decals.json').read_text());cls.seasons=json.loads((DOCS/'local-original/model-seasons.json').read_text());cls.source=without_comments(cls.db.read('db/Units/farm.ddf').decode('cp1252'))
    @classmethod
    def tearDownClass(cls):
        for name in ['db','graphics','textures','headers']:getattr(cls,name).close()
    def test_native_layers_have_documented_zero_default_and_compiled_constants(self):
        header=self.headers.read('DbHdrs/DbUnitDefs.h');self.assertIn(b'will initialize all dataclass data to 0',header)
        self.assertEqual(sha(header),self.farms['source']['zeroDefaultHeaderSha256']);pe=PE((ROOT/'work/EE2-engine.exe').read_bytes())
        conversion=struct.unpack('<f',pe.read(0xb41800,4))[0];building=struct.unpack('<f',pe.read(0x5f6ae3,4))[0]
        expected=f32(f32(building*conversion)*conversion)
        self.assertEqual(expected,self.farms['scale']['world']);self.assertEqual(expected,0.019999999552965164)
        self.assertEqual(f32(25*expected),.5);self.assertEqual(f32(100*expected),2)
        for family in self.farms['families']:
            inner=block(self.source,'CompositeModel',family+'CompositePlots');self.assertNotRegex(inner,r'\b(?:ScaleType|ForceScale)\s*=')
    def test_all_states_keep_every_original_geometry_material_uv_index_and_node(self):
        assets=self.seasons['textures'];by_stem={pathlib.PurePosixPath(v['archive_member']).stem.casefold():v for v in assets.values()}
        previous=exporter.extract_texture
        exporter.extract_texture=lambda source,*args:by_stem[pathlib.PureWindowsPath(source).stem.casefold()]
        try:
            for family,definition in self.farms['families'].items():
                self.assertEqual(definition['defaultState'],'Harvest_3');self.assertEqual(len(definition['plots']),4)
                for state,entry in definition['states'].items():
                    actual=json.loads((DOCS/entry['url']).read_text());raw=self.graphics.read(entry['source']['entry']);self.assertEqual(sha(raw),entry['source']['sha256'])
                    original=exporter.convert(raw,entry['source']['entry'],pathlib.Path('graphics.zip'),self.textures,kind='building',normalization=actual['normalization'])
                    count=len(original['parts']);self.assertEqual(len(actual['parts']),count*4)
                    for plot,offset in enumerate(definition['plots']):
                        shift=np.array([offset[0],offset[2],-offset[1]])*.01
                        for index,part in enumerate(original['parts']):
                            copied=actual['parts'][plot*count+index];self.assertEqual(copied['farmPlot'],plot+1);self.assertEqual(copied['farmPlotOffset'],shift.tolist())
                            positions=np.round(np.asarray(part['positions']).reshape(-1,3)+shift,6).reshape(-1).tolist();self.assertEqual(copied['positions'],positions)
                            for key,value in part.items():
                                if key!='positions':self.assertEqual(copied[key],value,f'{family}/{state}/{plot}/{key}')
                    self.assertEqual(actual['lodSelection'],original['lodSelection'],'No fabricated far LOD or discarded native LOD')
        finally:exporter.extract_texture=previous
    def test_source_default_state_and_four_offsets_match_the_manifest(self):
        for family,entry in self.farms['families'].items():
            for number,offset in enumerate(entry['plots'],1):
                body=block(self.source,'FarmPlotModel',f'{family}Plot_{number}');self.assertEqual(offset,[float(re.search(axis+r'Offset\s*=\s*([-\d.]+)',body)[1])for axis in 'XYZ'])
                state=block(self.source,'StateDatabase',f'{family}PlotState_{number}');self.assertRegex(state,rf'DefaultState\s*=\s*Harvest_3_{number}')
                for phase,model in entry['states'].items():self.assertIn(pathlib.PurePosixPath(model['source']['entry']).stem.replace('.nifcache',''),state)
    def test_original_crop_pixels_and_seasonal_source_bindings(self):
        relevant=set()
        for family in self.farms['families'].values():
            for state in family['states'].values():
                model=json.loads((DOCS/state['url']).read_text());self.assertIn(state['url'],self.seasons['models'])
                for part in model['parts']:relevant.add(part['textureUrl'])
        for asset in self.seasons['textures'].values():
            if asset['url']not in relevant and not asset['url'].startswith('local-original/farm-textures/'):continue
            raw=self.textures.read(asset['archive_member']);self.assertEqual(sha(raw),asset['source_sha256']);expected,_=decode_texcache(raw)
            actual=Image.open(DOCS/asset['url']).convert('RGBA');self.assertEqual(actual.tobytes(),expected.tobytes())
    def test_factory_dimensions_offsets_are_float32_scaled_and_axis_correct(self):
        pe=PE((ROOT/'work/EE2-engine.exe').read_bytes());factor=struct.unpack('<f',pe.read(0xb43910,4))[0];self.assertEqual(factor,1.4142135381698608)
        self.assertEqual(pe.read(0x53b0df,6).hex(),'d80d1039b400');self.assertEqual(pe.read(0x53b0e7,6).hex(),'d80d1039b400');self.assertEqual(pe.read(0x53b0fd,6).hex(),'894d70895574')
        expected={0:(4,3,-.25,0),1:(1.5,1.5,0,0),3:(3,2,-.5,0),4:(2.5,2.5,0,.25),6:(5,3,0,0),47:(3,3,0,0),48:(2.5,2.5,0,.25),49:(3,2,-.5,0),50:(2,2,0,0),51:(2.5,2.5,0,0),52:(2,1,0,0),55:(2.5,2.5,0,.25),56:(3,3,0,0)}
        for entry in self.ground['entries']:
            self.assertEqual(entry['placementProfile'],'native-unit-factory-undertexture')
            if entry['id']==2:self.assertEqual(entry['width'],f32(1.5*factor));self.assertEqual(entry['depth'],entry['width']);continue
            x,y,ox,oy=expected[entry['id']]
            self.assertEqual(entry['width'],f32(y*factor));self.assertEqual(entry['depth'],f32(x*factor));self.assertEqual(entry['offsetX'],f32(-oy*factor));self.assertEqual(entry['offsetY'],f32(-ox*factor))
        self.assertEqual(len(self.ground['entries']),14);self.assertEqual(self.ground['unresolvedBindings'],[])
    def test_old_original_payloads_remain_byte_identical(self):
        baseline=json.loads((ROOT/'work/season-model-baseline/model-hashes.json').read_text())
        live=ROOT/'outputs/OpenEpoch/docs'
        self.assertEqual(len(baseline),191)
        for url,digest in baseline.items():self.assertEqual(sha((live/url).read_bytes()),digest,url)
    def test_farm8_is_absent_from_active_bindings(self):
        farms=[row for row in self.models['models']if row['id']==2];self.assertEqual(len(farms),1);self.assertEqual(farms[0]['epoch'],0)
        self.assertTrue(farms[0]['url'].endswith('native-farmwheat-harvest_3.json'));self.assertEqual(farms[0]['nativeFarmComposite']['defaultState'],'Harvest_3')

if __name__=='__main__':unittest.main()
