"""Independent source-file checks for generated native combat tables and bindings."""
import csv,hashlib,json,pathlib,re,unittest,zipfile
from import_ee2_combat import ROOT,INSTALL,generate

class NativeCombatImport(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.data=json.loads((ROOT/'docs/ee2-combat.json').read_text(encoding='utf8'))
  cls.archive=zipfile.ZipFile(INSTALL/'zips/db.zip')
  cls.headers=zipfile.ZipFile(INSTALL/'zips/hdrs.zip')
 @classmethod
 def tearDownClass(cls):cls.archive.close();cls.headers.close()
 def test_every_matrix_cell_matches_raw_csv_with_defender_rows(self):
  lines=self.archive.read('db/Simulation/dbcombat.csv').decode('cp1252').splitlines()
  rows=list(csv.reader(lines[3:]));self.assertEqual(len(rows),35)
  for defender,row in enumerate(rows):
   self.assertEqual(len(row),36)
   for attacker,value in enumerate(row[1:]):self.assertEqual(self.data['matrix'][defender][attacker],int(value))
  self.assertEqual(self.data['matrix'][2][1],150);self.assertEqual(self.data['matrix'][1][2],100)
  self.assertEqual(self.data['matrix'][1][7],250);self.assertEqual(self.data['matrix'][7][1],25)
 def test_original_hashes_are_current(self):
  for ref in self.data['sources']:
   archive=self.archive if ref['archive']=='db.zip' else self.headers
   self.assertEqual(hashlib.sha256(archive.read(ref['entry'])).hexdigest(),ref['sha256'])
  self.assertEqual(hashlib.sha256((ROOT/'docs/ee2-units.json').read_bytes()).hexdigest(),self.data['unitAuditSha256'])
 def test_explicit_adjustments_match_raw_named_upgrade_pairs(self):
  raw=self.archive.read('db/Simulation/dbcombat_unittypeadjust.csv').decode('cp1252').splitlines()
  names=self.data['upgradeNames'];actual={(names[a],names[d]):value for a,d,value in self.data['adjustments']};expected={}
  for line in raw:
   if not line.strip() or line.lstrip().startswith('//'):continue
   _,attacker,defenders,value=next(csv.reader([line]))
   for defender in re.findall(r'\w+',defenders):expected[(attacker,defender)]=int(value)
  self.assertEqual(actual,expected);self.assertEqual(len(actual),221)
  self.assertEqual(actual[('HeavyMounted1UpgradeEpoch11','LightInfantryUpgradeEpoch11')],60)
 def test_profiles_match_current_class_progression_and_exact_upgrade_line(self):
  classes=json.loads((ROOT/'docs/ee2-units.json').read_text(encoding='utf8'))['class_rows']
  self.assertEqual(len(classes),len(self.data['profiles']))
  upgrade_lines=self.archive.read('db/TechTree/upgrade_unittypes.csv').decode('cp1252').splitlines()
  for native,profile in zip(classes,self.data['profiles']):
   for native_key,key in [('id','id'),('epoch','epoch'),('until','until'),('source_unit','sourceUnit'),('source_epoch','sourceEpoch'),('identity_verified','identityVerified')]:self.assertEqual(native[native_key],profile[key])
   if profile['upgradeSource']:
    source_row=next(csv.reader([upgrade_lines[profile['upgradeSource']['line']-1]]))
    self.assertEqual(source_row[0],profile['upgrade']);self.assertEqual(source_row[1],profile['sourceUnit']);self.assertEqual(int(source_row[3]),profile['sourceEpoch'])
   else:
    self.assertIn(profile['sourceUnit'],['Farm','CitizenM','OilRig','UraniumMine']);self.assertEqual(profile['upgradeIndex'],65535)
  spear=next(p for p in self.data['profiles'] if p['id']==11 and p['epoch']==1)
  self.assertEqual(spear['upgrade'],'HeavyInfantry2UpgradeEpoch2')
 def test_epoch_slots_have_only_their_own_valid_profiles(self):
  for id,row in enumerate(self.data['lookup']):
   self.assertEqual(len(row),15)
   for epoch,index in enumerate(row):
    if index==65535:continue
    profile=self.data['profiles'][index];self.assertEqual(profile['id'],id);self.assertLessEqual(profile['epoch'],epoch);self.assertLess(epoch,profile['until'])
 def test_generated_rust_is_reproducible(self):
  self.assertEqual((ROOT/'src/ee2_combat.rs').read_text(encoding='utf8'),generate(self.data))

if __name__=='__main__':unittest.main()
