"""Native state/export checks; every mesh/animation field is guarded by a hash."""
import hashlib
import json
import pathlib
import unittest
import zipfile
from extract_ee2_models import Reader, scene, ROOT, INSTALL
from native_materials import channels, native_material_fields, PROPERTY_TYPES
from upgrade_ee2_materials import digest


class NativeMaterialExport(unittest.TestCase):
    def test_distinct_channels_and_source_properties_are_not_collapsed(self):
        native = {'NiMaterialProperty': {'Ambient Color': dict(r=.1,g=.2,b=.3),
                  'Diffuse Color': dict(r=.4,g=.5,b=.6), 'Specular Color': dict(r=.7,g=.8,b=.9),
                  'Emissive Color': dict(r=0,g=.01,b=.02), 'Glossiness': 32., 'Alpha': 0.},
                  'NiSpecularProperty': {'Flags': 0}, 'NiZBufferProperty': {'Flags': 1, 'Function': 4}}
        result = native_material_fields(native)
        self.assertEqual(result['material'], {'ambient':[.1,.2,.3], 'diffuse':[.4,.5,.6],
                         'specular':[.7,.8,.9], 'emissive':[0.,.01,.02], 'glossiness':32., 'alpha':0.})
        self.assertEqual(result['nativeProperties']['NiSpecularProperty']['Flags'], 0)
        native['NiZBufferProperty']['Flags'] = 99
        self.assertEqual(result['nativeProperties']['NiZBufferProperty']['Flags'], 1)
        self.assertIsNone(result['nativeProperties']['NiAlphaProperty'])

    def test_absent_material_stays_distinct_from_explicit_native_defaults(self):
        absent = native_material_fields({})
        self.assertIsNone(absent['material'])
        self.assertTrue(all(absent['nativeProperties'][name] is None for name in PROPERTY_TYPES))
        explicit = native_material_fields({'NiMaterialProperty':{}})
        self.assertEqual(explicit['material']['diffuse'], [1.,1.,1.])
        self.assertEqual(explicit['material']['specular'], [0.,0.,0.])

    def test_all_current_near_far_materials_and_nonmaterial_hashes(self):
        report = ROOT.parent/'EE2-Research/analysis/visual-material-upgrade.json'
        if not report.exists(): self.skipTest('Native upgrade report unavailable')
        records=json.loads(report.read_text(encoding='utf8'))['records']
        self.assertEqual(len(records),191)
        for record in records:
            with self.subTest(file=record['url']):
                raw=(ROOT/'docs'/record['url']).read_bytes();model=json.loads(raw)
                self.assertEqual(hashlib.sha256(raw).hexdigest(),record['afterSha256'])
                self.assertEqual(model['source']['sha256'],record['sourceSha256'])
                data=[{k:v for k,v in p.items() if k not in ('material','nativeProperties','materialStateVersion')} for p in model['parts']]
                self.assertEqual(digest(data),record['nonMaterialDataSha256'])
                for part in model['parts']:
                    self.assertEqual(part['materialStateVersion'],1)
                    self.assertEqual(part['material'],channels(part['nativeProperties']['NiMaterialProperty']))
                    self.assertEqual(part['nativeProperties']['NiZBufferProperty']['Flags'],3)

    def test_independent_native_source_properties_match_both_lod_levels(self):
        source=INSTALL/'zips/graphics.zip'
        if not source.exists(): self.skipTest('Original installation unavailable')
        names=['bld_citycenter1_we','bld_citycenter1_we_lodfar','res_iron','AF13_JetFighter']
        with zipfile.ZipFile(source) as archive:
            for name in names:
                model=json.loads((ROOT/f'docs/local-original/models/{name}.json').read_text(encoding='utf8'))
                raw=archive.read(model['source']['entry'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(),model['source']['sha256'])
                _,props,*_=scene(Reader(raw).parse(),lod='far' if name.endswith('_lodfar') else 'near')
                for part in model['parts']:
                    for prop in PROPERTY_TYPES:
                        self.assertEqual(part['nativeProperties'][prop],props[part['sourceNode']].get(prop))


if __name__=='__main__': unittest.main()
