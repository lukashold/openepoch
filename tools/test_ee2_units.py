"""Independent known-value, provenance and boundary checks of the unit import."""
import hashlib
import json
import math
import pathlib
import re
import struct
import unittest
import zipfile
from import_ee2_units import ROOT, RESOURCES, clean_comments, declarations, brace_end


class UnitImport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.audit = json.loads((ROOT/'docs/ee2-units.json').read_text(encoding='utf-8'))
        cls.rows = cls.audit['rows']

    def lookup(self, identifier, epoch):
        rows = [row for row in self.rows if row['id'] == identifier and row['epoch'] <= epoch < row['until']]
        self.assertLessEqual(len(rows), 1, 'No overlapping runtime stat variants')
        return rows[0] if rows else None

    def val(self, identifier, epoch, name):
        return self.lookup(identifier, epoch)['fields'][name]['value']

    def test_comment_and_brace_boundaries_preserve_source_lines(self):
        text = '// UnitType Fake { bad }\nUnitType Real {\n name="quoted } // {"\n /* } bogus */ properties { HP=7 }\n}\n'
        cleaned = clean_comments(text)
        self.assertEqual(cleaned.count('\n'), text.count('\n'))
        self.assertEqual(len(cleaned), len(text))
        rows = list(declarations(cleaned, 'UnitType'))
        self.assertEqual([row[0] for row in rows], ['Real'])
        self.assertIn('HP=7', rows[0][1])
        with self.assertRaises(ValueError):
            brace_end('properties {', len('properties {'))

    def test_explicit_roster_and_no_unverified_zero_defaults(self):
        self.assertEqual(len(self.audit['bindings']), 57)
        self.assertEqual({row['id'] for row in self.audit['bindings']}, set(range(57)))
        self.assertLess(len(self.rows), 438, 'Declared helper UnitTypes are not all playable roster entries')
        for row in self.rows + self.audit['class_rows']:
            self.assertTrue(0 <= row['epoch'] < row['until'] <= 15)
            self.assertLessEqual(row['source_epoch'], row['epoch'] + 1,
                                 'A source initialization can predate its actual production gate')
            for name, item in row['fields'].items():
                self.assertEqual(item['verified'], item['value'] is not None, name)
                if item['verified']:
                    self.assertTrue(item['sources'], name + ' must have source provenance')
                    values = item['value'] if isinstance(item['value'], list) else [item['value']]
                    if name == 'attack_missile':
                        self.assertTrue(isinstance(item['value'], str) and item['value'])
                    else:
                        self.assertTrue(all(isinstance(x, (int, float)) and math.isfinite(x) for x in values))
        self.assertIsNone(self.val(0, 0, 'pop'), 'Unspecified building population usage is not invented')
        self.assertIsNone(self.val(1, 0, 'gather'), 'A house does not acquire worker gather rates')

    def test_nine_slot_costs_known_building_values(self):
        self.assertEqual(RESOURCES, ['FOOD','WOOD','IRON','STONE','GOLD','TIN','SALTPETER','OIL','URANIUM'])
        self.assertEqual(self.val(0, 0, 'cost9'), [0,300,0,300,0,0,0,0,0])
        self.assertEqual(self.val(0, 0, 'hp'), 8000)
        self.assertEqual(self.val(0, 0, 'buildtime'), 50)
        self.assertEqual(self.val(1, 0, 'cost9'), [0,50,0,50,0,0,0,0,0])
        self.assertEqual(self.val(48, 0, 'cost9'), [0,0,0,250,0,0,0,0,0])
        self.assertEqual(self.val(50, 1, 'cost9'), [0,125,0,0,125,0,0,0,0])
        self.assertEqual(self.val(16, 10, 'cost9'), [50,0,0,0,31,0,26,0,0])
        for row in self.rows:
            value = row['fields']['cost9']['value']
            if value is not None:
                self.assertEqual(len(value), 9)
                self.assertTrue(all(isinstance(x, int) and 0 <= x <= 65535 for x in value))

    def test_original_epoch_boundaries_and_static_variant_gaps(self):
        self.assertIsNone(self.lookup(2, 0), 'Farm production starts in original epoch 2')
        self.assertEqual(self.val(2, 1, 'buildtime'), 25)
        self.assertIsNone(self.lookup(49, 0), 'Stable starts in original epoch 2')
        self.assertIsNone(self.lookup(6, 9), 'Airport starts in original epoch 11')
        self.assertIsNotNone(self.lookup(6, 10))
        self.assertIsNone(self.lookup(53, 8))
        self.assertEqual(self.val(53, 9, 'hp'), 1000)
        self.assertIsNone(self.lookup(54, 11))
        self.assertEqual(self.val(54, 12, 'buildtime'), 45)
        self.assertIsNone(self.lookup(14, 3), 'Crossbowman is not the old project epoch 3 mapping')
        self.assertIsNotNone(self.lookup(14, 4))
        self.assertIsNone(self.lookup(14, 6), 'The next native visual variant is not silently this Crossbowman')
        self.assertEqual(self.lookup(40, 13)['source_epoch'], 14, 'Stealth Bomber uses native epoch 14')
        for identifier in range(57):
            for epoch in range(15):
                self.lookup(identifier, epoch)

    def test_citizen_ancestor_upgrade_and_natural_gather(self):
        self.assertEqual([self.val(8, epoch, 'hp') for epoch in [0,3,6,9,12]], [30,30,40,50,60])
        self.assertEqual(self.val(8, 0, 'cost9'), [50,0,0,0,0,0,0,0,0])
        self.assertEqual(self.val(8, 0, 'buildtime'), 25)
        self.assertEqual(self.val(8, 0, 'pop'), 1)
        self.assertEqual(self.val(8, 0, 'gather'), [.85,.85,.85,.85,.85,.85,.85,3.4,3.4])
        self.assertEqual(self.val(8, 0, 'farm_rate'), 4.37)
        self.assertEqual(self.val(8, 0, 'carry'), 15)
        self.assertEqual(self.val(8, 0, 'repair_rate'), 12)
        self.assertEqual(self.val(1, 0, 'pop_capacity'), 5)
        self.assertEqual(self.val(0, 0, 'pop_capacity'), 40)

    def test_special_extraction_garrisons_and_university_override(self):
        for identifier, epoch, slot in [(53,9,7),(54,12,8)]:
            self.assertEqual(self.val(identifier, epoch, 'garrison_slots'), 6)
            self.assertEqual(self.val(identifier, epoch, 'generation'), .75)
            self.assertEqual(self.val(identifier, epoch, 'generation_resource'), slot)
        self.assertEqual(self.val(47, 0, 'garrison_slots'), 3)
        self.assertEqual(self.val(47, 0, 'generation'), .03, 'Epoch override wins over base .08')
        self.assertEqual(self.val(47, 0, 'generation_resource'), 9, 'Technology is separate from natural resource slots')

    def test_adapter_identity_is_separate_from_verified_source_values(self):
        by_id = {item['id']: item for item in self.audit['bindings']}
        for identifier in [4,7,9,10,12,17,18,19,24,28,43,44,45,46]:
            self.assertFalse(by_id[identifier]['identity_verified'])
            self.assertTrue(by_id[identifier]['role_adapter'])
            self.assertTrue(by_id[identifier]['reason'])
        self.assertTrue(by_id[16]['identity_verified'])
        self.assertEqual(self.lookup(16,10)['source_visual'], 'HeavyInfantry1_11')
        self.assertEqual(self.lookup(17,12)['source_visual'], 'HeavyInfantry1_13')
        self.assertEqual(self.lookup(23,10)['source_visual'], 'LightMounted1_11')
        self.assertTrue(self.lookup(23,10)['model_verified'], 'Tank must match actual LLM11 gun-tank source')
        self.assertEqual(self.val(33, 10, 'pop'), 4)
        self.assertEqual(self.val(18, 1, 'pop'), 2)

    def test_conflicting_same_visual_csv_rows_follow_explicit_production_upgrade(self):
        # CSV line32 has HP170 but its obsolete UpgradeEpoch1 key is not what
        # the original production node at epoch11 actually requests.
        halftrack = self.lookup(25, 10)
        self.assertEqual(halftrack['source_upgrade']['UPGRADE'], 'LightArtillery2UpgradeEpoch11')
        self.assertEqual(halftrack['fields']['hp']['value'], 225)
        self.assertEqual(halftrack['fields']['damage']['value'], 27)
        self.assertEqual(halftrack['fields']['reload']['value'], .5)
        binding = next(item for item in self.audit['bindings'] if item['id'] == 25)
        self.assertEqual(len(binding['model_matching_upgrade_rows']), 2)
        self.assertEqual(binding['production_node']['UPGRADE'], 'LightArtillery2UpgradeEpoch11')

    def test_complete_source_class_progression_fills_visual_roster_gaps(self):
        def class_lookup(identifier, epoch):
            found = [row for row in self.audit['class_rows']
                     if row['id'] == identifier and row['epoch'] <= epoch < row['until']]
            self.assertLessEqual(len(found), 1, 'Source class records must not overlap')
            return found[0] if found else None
        # Strict rendered-model records remain unchanged; native numerical class
        # records can still progress where that fixed graphic has no bound ID.
        self.assertIsNone(self.lookup(31, 5))
        self.assertEqual(class_lookup(31, 5)['source_epoch'], 6)
        self.assertEqual(class_lookup(31, 5)['source_unit'], 'Frigate')
        self.assertFalse(class_lookup(31, 5)['model_verified'])
        self.assertFalse(class_lookup(31, 5)['identity_verified'])
        self.assertTrue(class_lookup(31, 5)['fields']['cost9']['verified'])
        self.assertIsNone(self.lookup(37, 11))
        self.assertEqual(class_lookup(37, 11)['source_epoch'], 12)
        self.assertFalse(class_lookup(37, 11)['model_verified'])
        self.assertTrue(class_lookup(37, 11)['fields']['hp']['verified'])
        for binding in self.audit['bindings']:
            identifier = binding['id']
            start = int(self.audit['production_class_from'][str(identifier)])
            for epoch in range(15):
                row = class_lookup(identifier, epoch)
                if epoch < start:
                    self.assertIsNone(row, 'No values before native production-node availability')
                else:
                    self.assertIsNotNone(row, f'Complete native progression: id{identifier} epoch{epoch}')
                    for name in ['cost9', 'hp', 'buildtime']:
                        self.assertTrue(row['fields'][name]['verified'], name)

    def test_native_production_class_epoch_is_independent_of_visual_epoch(self):
        starts = self.audit['production_class_from']
        self.assertEqual(starts['31'], 5, 'Frigate class native production epoch6')
        self.assertEqual(self.lookup(31, 9)['source_epoch'], 10, 'Currently bound Frigate visual variant epoch10')
        self.assertEqual(starts['49'], 1, 'Stable native epoch2')
        self.assertEqual(starts['53'], 9, 'OilRig native epoch10')
        self.assertEqual(starts['54'], 12, 'UraniumMine native epoch13')

    def test_herc_facility_is_native_epoch14_and_not_an_invented_tank_factory(self):
        self.assertIsNone(self.lookup(55,12))
        facility = self.lookup(55,13)
        self.assertEqual(facility['source_unit'], 'HercFacility')
        self.assertEqual(facility['source_epoch'], 14)
        self.assertEqual(facility['fields']['hp']['value'], 3000)
        self.assertEqual(facility['fields']['buildtime']['value'], 45)
        self.assertEqual(facility['fields']['cost9']['value'], [0,0,0,250,0,0,0,0,0])
        self.assertEqual(self.audit['production_class_from']['55'], 13)
        self.assertEqual(self.lookup(23,10)['source_host'], 'Stable')
        self.assertEqual(self.lookup(26,12)['source_host'], 'Stable')
        self.assertEqual(self.lookup(20,5)['source_host'], 'Mill')
        self.assertEqual(self.lookup(43,13)['source_host'], 'HercFacility')

    def test_fortress_all_seven_native_stages_and_garrison_abilities(self):
        epochs=[0,3,5,7,9,11,13]
        costs=[300,370,440,510,580,640,700]
        self.assertEqual([r['epoch'] for r in self.rows if r['id']==56],epochs)
        self.assertEqual([self.val(56,e,'hp') for e in epochs],[4200,4800,5800,6700,7600,8250,9000])
        self.assertEqual([self.val(56,e,'damage') for e in epochs],[75,100,115,145,170,200,225])
        self.assertEqual([self.val(56,e,'range') for e in epochs],[7,9,9,10,10,11,11])
        self.assertEqual([self.val(56,e,'garrison_heal') for e in epochs],[2,4,6,8,10,12,14])
        self.assertEqual([self.val(56,e,'attack_missile') for e in epochs],
                         ['FireArrow']*3+['Cannonball']*2+['TankShell']*2)
        for e,cost in zip(epochs,costs):
            row=self.lookup(56,e)
            self.assertEqual((row['source_unit'],row['source_host']),('Fortress','Citizen'))
            self.assertTrue(row['model_verified'])
            self.assertEqual(self.val(56,e,'cost9'),[0,cost,0,cost,0,0,0,0,0])
            self.assertEqual(self.val(56,e,'buildtime'),120)
            self.assertEqual(self.val(56,e,'reload'),3)
            self.assertEqual(self.val(56,e,'garrison_slots'),10)
            self.assertEqual(self.val(56,e,'garrison_attack_bonus'),.03)
        self.assertEqual(self.audit['production_class_from']['56'],0)
        self.assertTrue(self.val(8,0,'fortress_garrison'),'Citizen is explicitly eligible')
        self.assertFalse(self.val(37,10,'fortress_garrison'),'Aircraft has no Fortress eligibility')

    def test_native_garrison_slots_are_verified_inherited_popcount(self):
        for row in self.rows+self.audit['class_rows']:
            slots=row['fields']['garrison_size'];pop=row['fields']['pop']
            if row['fields']['fortress_garrison']['value']:
                self.assertTrue(slots['verified'])
                self.assertEqual(slots['value'],pop['value'])
                self.assertEqual(slots['sources'],pop['sources'])
                self.assertEqual(slots['semanticEvidence'],'native_garrison_slots')
            else:
                self.assertIsNone(slots['value'],'No guessed slot cost on an ineligible type')
        for identifier,epoch,expected in [(8,0,1),(18,1,2),(19,3,2),(22,8,3),(23,10,2),(26,12,2),(43,13,2)]:
            self.assertEqual(self.val(identifier,epoch,'garrison_size'),expected)
        # No bound class or ancestor currently declares the Leaders exception.
        with zipfile.ZipFile(self.audit['source_archive']) as source:
            for row in self.rows+self.audit['class_rows']:
                for origin in row['inheritance']:
                    text=clean_comments(source.read(origin['entry']).decode('cp1252'))
                    name=origin['declaration'].split(' ',1)[1]
                    body=next(body for n,body,_,_ in declarations(text,'UnitType') if n==name)
                    for attrs in re.findall(r'\battributes\s*=\s*\[([^]]*)\]',body,re.I):
                        self.assertNotIn('Leaders',re.findall(r'\w+',attrs))

    def test_native_slot_property_binder_and_getter_against_raw_pe(self):
        metadata=self.audit['native_garrison_slots']
        report_path=ROOT.parent/'EE2-Research/analysis/fortress-population-binding.json'
        raw=report_path.read_bytes();self.assertEqual(hashlib.sha256(raw).hexdigest(),metadata['report_sha256'])
        evidence=json.loads(raw);self.assertEqual(evidence,metadata['evidence'])
        for item in evidence['exports']:
            self.assertEqual(hashlib.sha256((report_path.parent/item['file']).read_bytes()).hexdigest(),item['sha256'])
        executable=pathlib.Path(self.audit['source_archive']).parents[1]/'Unofficial Patch Files/EnabledUP15Units/EE2.exe'
        binary=executable.read_bytes();self.assertEqual(hashlib.sha256(binary).hexdigest(),evidence['executableSha256'])
        pe=struct.unpack_from('<I',binary,0x3c)[0];coff=pe+4;optional=coff+20
        count=struct.unpack_from('<H',binary,coff+2)[0];optional_size=struct.unpack_from('<H',binary,coff+16)[0]
        image_base=struct.unpack_from('<I',binary,optional+28)[0];sections=[]
        for i in range(count):
            virtual_size,rva,file_size,file_offset=struct.unpack_from('<IIII',binary,optional+optional_size+i*40+8)
            sections.append((rva,max(virtual_size,file_size),file_offset))
        def at(va,size):
            target=va-image_base
            for rva,length,file_offset in sections:
                if rva<=target<rva+length:return binary[file_offset+target-rva:file_offset+target-rva+size]
            self.fail(f'Unmapped native address {va:x}')
        self.assertEqual(struct.unpack('<I',at(0xc14970,4))[0],0xc149dc)
        self.assertEqual(at(0xc149dc,9),b'popCount\0')
        self.assertEqual(at(0xc47ce4,8),b'Leaders\0')
        instructions={0x53a209:'bebc48c100',0x53a20e:'8b4efc',0x53a21e:'8916',
                      0x53a220:'83c608',0x53acdc:'8b357449c100',0x53acec:'8b0430',
                      0x53acfe:'8985a0000000',0x7e63c5:'b801000000',0x7e63d2:'8b80a0000000'}
        for va,expected in instructions.items():
            value=bytes.fromhex(expected);self.assertEqual(at(va,len(value)),value,f'Native binder/getter opcode {va:x}')

    def test_source_hashes_and_record_line_provenance(self):
        archive = pathlib.Path(self.audit['source_archive'])
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), self.audit['source_sha256'])
        with zipfile.ZipFile(archive) as source:
            for path, metadata in self.audit['source_files'].items():
                self.assertEqual(hashlib.sha256(source.read(path)).hexdigest(), metadata['sha256'])
            for row in self.rows + self.audit['class_rows']:
                for item in row['fields'].values():
                    for origin in item['sources']:
                        self.assertEqual(origin['archive'], 'db.zip')
                        self.assertEqual(origin['sha256'], self.audit['source_files'][origin['entry']]['sha256'])
                        self.assertGreater(origin['line'], 0)
                        self.assertLessEqual(origin['line'], source.read(origin['entry']).count(b'\n') + 1)
        rust = (ROOT/'src/ee2_units.rs').read_text(encoding='utf-8')
        self.assertEqual(len(re.findall(r' UnitRow\{id:', rust)), len(self.rows) + len(self.audit['class_rows']))
        self.assertIn('pub fn lookup_class(', rust)
        self.assertIn('pub fn production_class_from(', rust)
        self.assertNotIn('NaN', rust)


if __name__ == '__main__':
    unittest.main()
