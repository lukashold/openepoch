"""Validate original full-size terrain pixels and native binding coverage."""
import hashlib
import io
import json
import unittest
from pathlib import Path
import zipfile
from PIL import Image
from extract_ee2_textures import ROOT, INSTALL, decode_texcache

MANIFEST = ROOT / 'docs/local-original/terrain-native/manifest.json'
REPORT = ROOT.parent / 'EE2-Research/analysis/native-terrain-assets-tests.json'
sha = lambda data: hashlib.sha256(data).hexdigest()

class NativeTerrain(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads(MANIFEST.read_text(encoding='utf8'))
        cls.images = 0

    def test_full_source_tables_are_bound(self):
        m = self.manifest
        self.assertEqual(len(m['terrainTypes']),33)
        self.assertEqual(len(m['textureDefinitions']),32)
        self.assertEqual(len(m['seasonalSets']),32)
        for row in m['terrainTypes']:
            for detail in ('high','medium','low'):
                self.assertIn(row[detail],m['textureDefinitions'])
        for row in m['textureDefinitions'].values():
            self.assertEqual(row['mask'].lower(),'terr_mask.tga')
            self.assertEqual(len(row['tintFractions']),12)
            self.assertIn(row['seasonalSet'],m['seasonalSets'])
        for row in m['seasonalSets'].values():
            self.assertEqual(len(row['urls']),12)
            self.assertEqual(len(row['sourceTextures']),12)
            for url in row['urls']:
                self.assertTrue((ROOT/'docs'/url).is_file(),url)
        by_name={row['name']:row for row in m['terrainTypes']}
        self.assertEqual(by_name['Grass']['low'],'GrassPatchy_Temperate')
        self.assertEqual(by_name['OceanFloor']['high'],'OceanFloor_Temperate')
        self.assertEqual(by_name['OceanFloor']['medium'],'Beach_Temperate')
        self.assertEqual(m['seasonalSets']['OceanFloor_Temperate']['sourceTextures'][6],'terr_coral.tga')
        self.assertEqual(m['adapter']['classSource'],'Map.ter, not minimap RGB')

    def test_every_source_and_png_matches_exact_pixels(self):
        with zipfile.ZipFile(INSTALL/'zips/textures.zip') as archive:
            for source, row in self.manifest['textures'].items():
                with self.subTest(texture=source):
                    raw=archive.read(row['archive_member'])
                    self.assertEqual(sha(raw),row['source_sha256'])
                    png=ROOT/'docs'/row['url']
                    self.assertEqual(sha(png.read_bytes()),row['png_sha256'])
                    original=Image.open(io.BytesIO(raw)).convert('RGBA') if row['archive_member'].lower().endswith('.tga') else decode_texcache(raw)[0]
                    generated=Image.open(png).convert('RGBA')
                    self.assertEqual(generated.size,original.size)
                    self.assertEqual(generated.size,(row['width'],row['height']))
                    self.assertEqual(generated.tobytes(),original.tobytes())
                    type(self).images+=1
            self.assertGreaterEqual(type(self).images,200)

    def test_mask_and_water_roles_keep_native_channels(self):
        m=self.manifest
        mask=Image.open(ROOT/'docs'/m['maskUrl']).convert('RGBA')
        self.assertEqual(mask.size,(256,256))
        self.assertEqual(mask.getextrema()[:3],((0,0),(0,0),(0,0)))
        self.assertEqual(mask.getextrema()[3],(0,255))
        normal=Image.open(ROOT/'docs'/m['water']['normal'])
        self.assertEqual(normal.size,(512,512))
        lut=Image.open(ROOT/'docs'/m['water']['colorLut']).convert('RGB')
        self.assertEqual(lut.size,(4,1))
        self.assertEqual(lut.getpixel((0,0)),(192,192,192))
        self.assertEqual(lut.getpixel((2,0)),(105,105,105))
        palette=Image.open(ROOT/'docs'/m['water']['palette'])
        self.assertEqual(palette.size,(256,1))
        self.assertEqual(len(m['water']['unboundCaustics']),17)

    def test_source_shader_evidence_is_current(self):
        for row in self.manifest['shaderEvidence']:
            self.assertEqual(sha((INSTALL/row['file']).read_bytes()),row['sha256'])
        nsf=(INSTALL/'shaders/WaterReflection_LOD_HIGH.NSF').read_text()
        self.assertIn('0.01125,0.02, -0.01125, 0.01',nsf)
        self.assertIn('0.1, 0.1, 0.1, 0.1',nsf)

if __name__=='__main__':
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(NativeTerrain)
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    report={'format':'openepoch-native-terrain-assets-tests-v1','tests':result.testsRun,
        'passed':result.wasSuccessful(),'verifiedNativeImages':NativeTerrain.images,
        'manifestSha256':sha(MANIFEST.read_bytes()),'nativeFullSize':True,
        'scope':'Read-only original source hashes and exact decoded RGBA pixels, full terrain/detail/seasonal bindings, original mask alpha and water lookup channels. Native shader flow evidence included; no native map generation or complete water shader equivalence claimed.'}
    REPORT.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    raise SystemExit(0 if result.wasSuccessful() else 1)
