"""Independent original-file and deterministic import checks for lighting5."""
import contextlib,csv,hashlib,io,json,pathlib,re,tempfile,unittest,zipfile
from import_ee2_lighting import build,INSTALL,ROOT
from nif_reader import Reader

class NativeLightingSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=json.loads((ROOT/'docs/local-original/climate-lighting.json').read_text())

    def test_every_atmosphere_channel_comes_from_exact_source_csv(self):
        for profile,archive,member in [('base','db.zip','db/Terrain/dbclimate_atmosphere.csv'),('native155','db_155.zip','db/Terrain/dbclimate_atmospher5.csv')]:
            with zipfile.ZipFile(INSTALL/'zips'/archive)as z:raw=z.read(member)
            rows=[r for r in csv.reader(raw.decode('cp1252').splitlines())if r and not r[0].lstrip().startswith('//')]
            self.assertEqual(len(rows),113)
            self.assertEqual(set(self.data['profiles'][profile]['atmospheres']),{r[0]for r in rows})
            for row in rows:
                original=self.data['profiles'][profile]['atmospheres'][row[0]]
                for name,cell in zip(['diffuse','ambient','specular','fog'],row[1:5]):
                    with self.subTest(profile=profile,row=row[0],channel=name):
                        self.assertEqual(original['sourceRGBA'][name],[int(v)for v in cell.strip('[] ').split()])
            self.assertEqual(self.data['sources'][archive+'!'+member]['sha256'],hashlib.sha256(raw).hexdigest())

    def test_first_directional_source_light_and_source_shader_are_literal(self):
        with zipfile.ZipFile(INSTALL/'zips/graphics.zip')as z:raw=z.read('cache/nifcache/lights.nifcache')
        reader=Reader(raw).parse();root=reader.objects[reader.footer['Roots'][0]]
        self.assertEqual(root['Children'],[3,4,5])
        light=reader.objects[root['Children'][0]]
        self.assertEqual(light['_type'],'NiDirectionalLight');self.assertEqual(light['Name'],'FDirect01');self.assertEqual(light['Dimmer'],1.5)
        for name in ('IndexPal20_High.vsh','IndexPal20_Low.vsh'):
            source=(INSTALL/'shaders/vsh_psh'/name).read_text()
            actual='\n'.join(line.split(';')[0].strip()for line in source.splitlines())
            self.assertIn('mov     oD0.w,',actual)
            self.assertNotIn('max',actual,'High signed dot is not clamped before adding ambient')
            self.assertNotIn('c4.xyz',actual,'Material RGB does not participate')
        high=(INSTALL/'shaders/vsh_psh/IndexPal20_High.vsh').read_text()
        self.assertRegex(high,r'dp3\s+r4.xyz,\s*r6.xyz,\s*-c6.xyz')
        nsf=(INSTALL/'shaders/IndexPal20.NSF').read_text()
        self.assertGreaterEqual(nsf.count('TOP_Modulate4x'),2)

    def test_import_is_byte_identical_with_explicit_original_install_engine(self):
        active=ROOT/'docs/local-original/climate-lighting.json';before=active.read_bytes()
        with tempfile.TemporaryDirectory(dir=ROOT.parents[1]/'work')as tmp,contextlib.redirect_stdout(io.StringIO()):
            output=pathlib.Path(tmp)
            build(output,INSTALL,INSTALL/'Unofficial Patch Files/EnabledUP15Units/EE2.exe')
            self.assertEqual((output/'local-original/climate-lighting.json').read_bytes(),before)
            # Import does not write the active runtime asset or original archives.
            self.assertEqual(active.read_bytes(),before)
            bogus=output/'invalid-source.exe';bogus.write_bytes(b'Unrecognized source; never execute')
            with self.assertRaises(AssertionError):build(output,INSTALL,bogus)

    def test_native155_is_distinct_and_no_unbound_base_tundra_alias(self):
        base=self.data['profiles']['base'];native=self.data['profiles']['native155']
        self.assertEqual(base['climates']['Tundra'],{})
        self.assertEqual(set(native['climates']['Tundra']),{'Morning','Afternoon','Evening','Night'})
        changes=[name for name,row in base['atmospheres'].items()if row['sourceRGBA']!=native['atmospheres'][name]['sourceRGBA']]
        self.assertEqual(len(changes),49)
        self.assertEqual(self.data['terrain']['diffuseScale'],0.8500000238418579)
        self.assertEqual(self.data['terrain']['ambientScale'],.75);self.assertEqual(self.data['terrain']['shadowScale'],1.)

if __name__=='__main__':unittest.main()
