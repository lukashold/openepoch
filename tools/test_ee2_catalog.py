import json,unittest
from pathlib import Path
import import_ee2_catalog as importer
class CatalogTests(unittest.TestCase):
 def test_complete_base_catalog(self):
  d=json.loads((Path(__file__).resolve().parents[1]/'docs/ee2-catalog.json').read_text(encoding='utf-8'))
  self.assertEqual([e['id'] for e in d['epochs']],list(range(1,16)))
  self.assertEqual(len(d['nodes']),641);self.assertEqual(len({n['NAME'] for n in d['nodes']}),641)
  self.assertEqual([e['technology_count'] for e in d['epochs']],[12]*15)
  self.assertEqual(d['epochs'][14]['name'],'Synthetikzeitalter')
  self.assertEqual(d['epochs'][8]['name'],'Zeitalter der Aufklärung')
  self.assertEqual(d['epochs'][1]['advance']['TECHPTS'],32)
  self.assertEqual(d['epochs'][14]['advance']['TECHPTS'],104)
  self.assertTrue(all(e['advance']['TIME']==15 for e in d['epochs']))
  self.assertEqual(len(d['resources']),9);self.assertEqual(len(d['unit_upgrades']),396)
  self.assertTrue(all(n['source'] and not n['implemented'] for n in d['nodes']))
  self.assertTrue(any(p.get('parse_error') for p in d['patch_comparison']))
 def test_encoding(self):
  self.assertEqual(importer.decode('Aufklärung'.encode('cp1252'))[0],'Aufklärung')
  self.assertEqual(importer.decode('Aufklärung'.encode('utf-8'))[0],'Aufklärung')
 def test_corrupt_row_is_not_silently_accepted(self):
  with self.assertRaises(ValueError):importer.table(b'// NAME,EPOCH\na,1,extra','fixture.csv')
if __name__=='__main__':unittest.main()
