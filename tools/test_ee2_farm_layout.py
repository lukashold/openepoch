"""Read-only source proof for the native four-plot farm research draft."""
import hashlib,json,pathlib,re,struct,unittest,zipfile
from import_ee2 import block,without_comments
from test_ee2_city_evidence import PE
ROOT=pathlib.Path(__file__).resolve().parents[3]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II/zips')

class FarmLayoutSource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.db=zipfile.ZipFile(INSTALL/'db.zip');cls.text=without_comments(cls.db.read('db/Units/farm.ddf').decode('cp1252'))
    @classmethod
    def tearDownClass(cls):cls.db.close()
    def test_all_three_families_have_four_exact_model_space_offsets(self):
        for family in ['FarmWheat','FarmSugarCane','FarmYam']:
            composite=block(self.text,'CompositeModel',family)
            self.assertRegex(composite,r'ScaleType\s*=\s*kScaleBuilding')
            self.assertIn('UnderFarm',composite)
            coords=[]
            for index in range(1,5):
                plot=block(self.text,'FarmPlotModel',f'{family}Plot_{index}')
                coords.append(tuple(float(re.search(axis+r'Offset\s*=\s*([-\d.]+)',plot)[1])for axis in 'XYZ'))
                states=block(self.text,'StateDatabase',f'{family}PlotState_{index}')
                self.assertRegex(states,rf'DefaultState\s*=\s*Harvest_3_{index}')
                for state in ['Seeding_'+str(index),*[f'Harvest_{k}_{index}'for k in (1,2,3)],'Death']:self.assertIn('StateName = '+state,states)
            self.assertEqual(coords,[(-25,-25,0),(25,-25,0),(25,25,0),(-25,25,0)])
    def test_farm_footprint_and_underlay_source_are_separate(self):
        farm=block(self.text,'UnitType','Farm')
        for field,value in [('SizeX','1.5'),('SizeY','1.5'),('mass','0')]:self.assertRegex(farm,rf'\b{field}\s*=\s*{re.escape(value)}')
        visual=without_comments(self.db.read('db/Visuals/visual_decals.ddf').decode('cp1252'));decal=block(visual,'Decal','UnderFarm')
        self.assertEqual(decal.count('under_farm'),12);self.assertRegex(decal,r'FadeOutTime\s*=\s*6');self.assertRegex(decal,r'StartFadeDelay\s*=\s*1')
        self.assertNotIn('UndertextureSize',decal)
    def test_later_legacy_farm8_graphic_has_no_base_database_reference(self):
        for name in self.db.namelist():
            if name.lower().endswith(('.ddf','.csv')):self.assertNotRegex(self.db.read(name).lower(),rb'\bbld_farm8\b|\bfarm8\b',name)
    def test_crop_source_bounds_are_unnormalized_half_plot_geometry(self):
        data=json.loads((ROOT/'outputs/EE2-Research/analysis/native-farm-source-bounds.json').read_text())
        for model in data['models'][:4]:
            self.assertFalse(model['normalizationApplied']);width=model['maxYUp'][0]-model['minYUp'][0];depth=model['maxYUp'][2]-model['minYUp'][2]
            self.assertAlmostEqual(width,50,places=3);self.assertLess(abs(depth-50),.05)
    def test_native_farm_plot_vtable_update_samples_ground_and_rewrites_local_position(self):
        pe=PE((ROOT/'work/EE2-engine.exe').read_bytes())
        self.assertEqual(struct.unpack('<I',pe.read(0xb4706c+6*4,4))[0],0x61d4c0)
        self.assertEqual(pe.call(0x61d52d),0x70fb00)
        self.assertEqual(pe.call(0x61d4ff),0x8ee030)
        self.assertEqual(pe.call(0x61d587),0x5de0f0)
        self.assertEqual(pe.read(0x61d5be,2).hex(),'6a08')

if __name__=='__main__':unittest.main()
