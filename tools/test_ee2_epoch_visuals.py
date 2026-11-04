"""Audit exact native class/visual aliases without changing game rules."""
import hashlib,json,unittest
from extract_ee2_models import ROOT,OUT,INSTALL,bind_epoch_visuals
from import_ee2_units import Database,model_stem,scalar

class EpochVisualBindings(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.manifest=json.loads((OUT/'models/manifest.json').read_text(encoding='utf8'));cls.unit=json.loads((ROOT/'docs/ee2-units.json').read_text(encoding='utf8'));cls.db=Database(INSTALL/'zips/db.zip');cls.added=[r for r in cls.manifest['models']if 'progressionBinding'in r];cls.icons=json.loads((OUT/'textures-manifest.json').read_text(encoding='utf8'))
 @classmethod
 def tearDownClass(cls):cls.db.close()
 def test_each_alias_is_the_same_native_unit_class_and_visual(self):
  self.assertGreaterEqual(len(self.added),26);keys=[(r['id'],r['epoch'])for r in self.manifest['models']];self.assertEqual(len(keys),len(set(keys)))
  for row in self.added:
   with self.subTest(id=row['id'],epoch=row['epoch']):
    proof=row['progressionBinding'];native=next(r for r in self.unit['class_rows']if r['id']==row['id']and r['epoch']==row['epoch']);strict=next(r for r in self.unit['rows']if r['id']==proof['reusedFrom']['id']and r['source_visual']==proof['sourceVisual'])
    self.assertEqual((native['source_unit'],native['source_visual']),(strict['source_unit'],strict['source_visual']));self.assertEqual(proof['sourceUnit'],native['source_unit']);self.assertEqual(proof['sourceVisual'],native['source_visual']);self.assertEqual(row['nativeUntil'],native['until']);self.assertGreater(row['epoch'],min(r['epoch']for r in self.unit['rows']if r['id']==row['id']))
    visual=self.db.block('UnitModel',proof['sourceVisual']);self.assertEqual(scalar(visual['body'],'DefaultModel'),proof['declaredDefaultModel']);self.assertEqual(model_stem(proof['declaredDefaultModel']),model_stem(row['source']['entry']))
    for origin in [proof['visualSource'],proof['upgradeSource']]:self.assertEqual(hashlib.sha256(self.db.zip.read(origin['entry'])).hexdigest(),origin['sha256'])
    self.assertEqual(proof['upgradeSource'],native['upgrade_source']);self.assertEqual(native['source_upgrade']['UNIT'],proof['sourceUnit']);self.assertEqual(native['source_upgrade']['VISUAL'],proof['sourceVisual'])
    source=next(r for r in self.manifest['models']if 'progressionBinding'not in r and r['id']==proof['reusedFrom']['id']and r['url']==proof['reusedFrom']['url'])
    for key in ['url','source','triangles','yawOffset']:self.assertEqual(row[key],source[key])
    self.assertEqual(row.get('lodFarUrl'),source.get('lodFarUrl'))
 def test_known_later_progression_switches_use_existing_real_geometry(self):
  expected={(9,2):13,(9,6):15,(9,10):16,(9,12):17,(10,2):12,(10,4):14,(33,12):35,(37,12):39,(38,13):40}
  for key,source in expected.items():self.assertEqual(next(r for r in self.added if(r['id'],r['epoch'])==key)['progressionBinding']['reusedFrom']['id'],source)
 def test_existing_matching_icons_are_not_repainted_or_role_substituted(self):
  self.assertGreaterEqual(len(self.manifest['epochIconBindings']),18)
  for entry in self.manifest['epochIconBindings']:
   row=next(r for r in self.unit['class_rows']if r['id']==entry['id']and r['epoch']==entry['epoch']);self.assertEqual(entry['sprite'].casefold(),row['source_upgrade']['ICON'].casefold())
   if entry.get('sourceKind')=='native-epoch-sprite':
    src=self.icons['nativeSpriteIcons'][entry['sprite'].casefold()];self.assertEqual(src['sourceUnit'],row['source_unit']);self.assertEqual(src['sourceVisual'],row['source_visual']);self.assertEqual(src['sourceEpoch'],row['source_epoch'])
    for key,sha_key in [('unitUpgradeSource','unitUpgradeSha256'),('visualSource','visualSourceSha256')]:
     archive,reference=src[key].split('!',1);source_entry,source_line=reference.rsplit(':',1);self.assertEqual(archive,'db.zip');self.assertGreater(int(source_line),0);self.assertEqual(hashlib.sha256(self.db.zip.read(source_entry)).hexdigest(),src[sha_key])
   else:
    src=self.icons['icons'][str(entry['reusedFromDef'])];binding=self.icons['iconBindings'][str(entry['reusedFromDef'])];self.assertEqual(binding['sprite'].casefold(),entry['sprite'].casefold())
   self.assertEqual(src['url'],entry['url']);self.assertEqual(hashlib.sha256((ROOT/'docs'/entry['url']).read_bytes()).hexdigest(),entry['png_sha256'])

 def test_alias_generation_is_idempotent_and_mesh_count_does_not_expand(self):
  before=json.loads(json.dumps(self.manifest));after=bind_epoch_visuals(before,INSTALL);canonical=lambda x:hashlib.sha256(json.dumps(x,sort_keys=True).encode()).hexdigest();self.assertEqual(canonical(after),canonical(self.manifest));self.assertEqual(len({r['url']for r in after['models']}),118)
 def test_unit_import_refresh_retained_rust_bytes_and_all_numeric_values(self):
  proof=json.loads((OUT/'models/unit-import-validation.json').read_text(encoding='utf8'));self.assertTrue(proof['rustByteIdentical']);self.assertEqual(hashlib.sha256((ROOT/'src/ee2_units.rs').read_bytes()).hexdigest(),proof['rustBeforeSha256']);self.assertEqual(proof['numericProjectionBeforeSha256'],proof['numericProjectionAfterSha256'])

if __name__=='__main__':unittest.main()
