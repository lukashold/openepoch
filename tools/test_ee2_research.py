"""Research field ABI, source factors and nine-resource regression checks."""
import collections
import csv
import hashlib
import json
import pathlib
import re
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
GATHER = {'Food':8,'Wood':9,'Iron':10,'Stone':12,'Gold':13,
          'Tin':14,'Saltpeter':15,'Oil':16,'Uranium':17}
TOOLTIP = {'GatherBasic_Tooltip', 'GatherSpecial_Tooltip'}


class ResearchData(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.audit = json.loads((ROOT/'docs/research-audit.json').read_text(encoding='utf-8'))
        cls.catalog = json.loads((ROOT/'docs/ee2-catalog.json').read_text(encoding='utf-8'))
        cls.nodes = cls.audit['nodes']
        cls.by_id = {node['id']:node for node in cls.nodes}

    def test_field_abi_has_distinct_nine_resources_and_population(self):
        fields = self.audit['fields']
        for resource, index in GATHER.items():
            self.assertEqual(fields['Gather'+resource], index)
            self.assertEqual(fields['Gather'+resource+'_NoTooltip'], index)
        self.assertEqual(fields['PopCap'], 18)
        self.assertEqual(fields['GarrisonSlots'], 11)
        self.assertEqual(fields['GatherTech'], 7)
        self.assertEqual(set(fields.values()), set(range(19)))
        for name in TOOLTIP:
            self.assertNotIn(name, fields, 'Tooltip groups are not extra gather effects')

    def test_basic_gather_stone_and_gold_do_not_collapse_into_iron(self):
        for key in ['Economic1_2','Economic6_2','Economic11_2']:
            node = self.by_id[key]
            effects = node['effects']
            self.assertEqual(len(effects), 4)
            self.assertEqual({effect['field'] for effect in effects}, {8,9,12,13})
            self.assertEqual({effect['factor'] for effect in effects},
                {'GatherFood_NoTooltip','GatherWood_NoTooltip','GatherStone_NoTooltip','GatherGold_NoTooltip'})
            for effect in effects:
                self.assertEqual(effect['targets'], [8])
                self.assertEqual(effect['mask'], 1<<8)
                self.assertEqual((effect['method'],effect['value']), (0,10))
            self.assertEqual([factor['FACTOR'] for factor in node['tooltip_only_factors']],
                             ['GatherBasic_Tooltip'])

    def test_three_special_gather_groups_reach_correct_extractors(self):
        expected = {10:8, 14:8, 15:8, 16:53, 17:54}
        for key in ['Economic4_2','Economic9_2','Economic14_2']:
            effects = self.by_id[key]['effects']
            self.assertEqual(len(effects), 5)
            self.assertEqual({effect['field'] for effect in effects}, set(expected))
            for effect in effects:
                target = expected[effect['field']]
                self.assertEqual(effect['targets'], [target])
                self.assertEqual(effect['mask'], 1<<target)
                self.assertEqual((effect['method'],effect['value']), (0,10))
                self.assertTrue(effect['factor'].endswith('_NoTooltip'))
            self.assertEqual([factor['FACTOR'] for factor in self.by_id[key]['tooltip_only_factors']],
                             ['GatherSpecial_Tooltip'])

    def test_three_native_house_population_capacity_factors(self):
        effects = [(node['id'],effect) for node in self.nodes
                   for effect in node['effects'] if effect['field'] == 18]
        self.assertEqual([key for key,_ in effects], ['Imperial3_1','Imperial8_1','Imperial13_1'])
        for _, effect in effects:
            self.assertEqual(effect['factor'], 'PopCap')
            self.assertEqual(effect['targets'], [1])
            self.assertEqual(effect['mask'], 2)
            self.assertEqual((effect['method'],effect['value']), (1,2))

    def test_tooltip_only_factors_are_audited_without_runtime_bonus(self):
        tooltips = [row for node in self.nodes for row in node['tooltip_only_factors']]
        self.assertEqual(len(tooltips), 6)
        self.assertEqual(collections.Counter(row['FACTOR'] for row in tooltips),
                         {'GatherBasic_Tooltip':3,'GatherSpecial_Tooltip':3})
        for node in self.nodes:
            self.assertFalse(any(effect['factor'] in TOOLTIP for effect in node['effects']))
            self.assertFalse(any(row['FACTOR'] in TOOLTIP for row in node['unimplemented_factors']))
            self.assertEqual(len(node['effects']), node['adapted_effects'])
            keys = [(effect['mask'], effect['field'], effect['method'], effect['value'])
                    for effect in node['effects']]
            self.assertEqual(len(keys), len(set(keys)), 'Same effect cannot stack through aliases')
        self.assertEqual(self.audit['summary']['runtime_effects'], 187)
        self.assertEqual(self.audit['summary']['unsupported_factors'], 82)
        self.assertEqual(sum(node['adapted_effects'] for node in self.nodes), 187)

    def test_all_180_ids_and_keys_match_original_catalog_order(self):
        source = sorted([node for node in self.catalog['nodes']
                         if node['BRANCH'] in ('Economic','Military','Imperial')],
                        key=lambda node:(node['EPOCH'],
                            ['Economic','Military','Imperial'].index(node['BRANCH']),node['SLOT']))
        self.assertEqual(len(self.nodes), 180)
        self.assertEqual(len(self.by_id), 180)
        self.assertEqual([node['id'] for node in self.nodes], [node['NAME'] for node in source])
        for actual, original in zip(self.nodes, source):
            self.assertEqual(actual['host'], original['HOST'])
            self.assertEqual(actual['label'], original['label'])
            self.assertEqual(actual['source'], original['source'])
        rust = (ROOT/'src/ee2_research.rs').read_text(encoding='utf-8')
        self.assertEqual(len(re.findall(r'^Node\{name:',rust,re.M)), 180)
        self.assertNotIn('Metallertrag', rust)
        self.assertIn('Steinertrag', rust)
        self.assertIn('Goldertrag', rust)
        self.assertIn('Eisenertrag', rust)
        self.assertIn('Bevölkerungslimit', rust)

    def test_runtime_values_and_tooltip_metadata_match_original_csv(self):
        metadata = self.audit['factor_source']
        with zipfile.ZipFile(self.catalog['source']) as archive:
            raw = archive.read(metadata['entry'])
        self.assertEqual(hashlib.sha256(raw).hexdigest(), metadata['sha256'])
        lines = raw.decode('cp1252').replace('\r','').splitlines()
        header = next(csv.reader([lines[0].removeprefix('//').strip()]))
        source = {}
        for line_number,line in enumerate(lines[1:],2):
            if line.strip() and not line.lstrip().startswith('//'):
                row = dict(zip(header,next(csv.reader([line]))))
                source[row['']] = row, line_number
        for node in self.nodes:
            for effect in node['effects']:
                original, line = source[effect['factor_id']]
                self.assertEqual(effect['factor'], original['FACTOR'])
                self.assertEqual(effect['value'], float(original['VALUE']))
                self.assertEqual(effect['method'], {'AddPermMul':0,'Add':1,'Set':2}[original['METHOD']])
                self.assertEqual(effect['source'], f'db.zip!{metadata["entry"]}:{line}')
                self.assertEqual(effect['source_sha256'], metadata['sha256'])
                self.assertTrue(0 <= effect['field'] < 19)
            for tooltip in node['tooltip_only_factors']:
                original, _ = source[tooltip['']]
                self.assertEqual(original['FACTOR'], tooltip['FACTOR'])
                self.assertEqual(original['METHOD'], '', 'Tooltip group has no numerical operation')

    def test_fortress_targets_extend_only_source_named_fortress_groups(self):
        with zipfile.ZipFile(self.catalog['source']) as archive:
            raw=archive.read(self.audit['factor_source']['entry']).decode('cp1252')
        lines=raw.replace('\r','').splitlines()
        header=next(csv.reader([lines[0].removeprefix('//').strip()]))
        source={r['']:r for line in lines[1:] if line.strip() and not line.lstrip().startswith('//')
                for r in [dict(zip(header,next(csv.reader([line]))))]}
        hits=[]
        for node in self.nodes:
            for effect in node['effects']:
                row=source[effect['factor_id']]
                if row['UNIT TYPE ATTRIBUTE']=='FortressOutpostWallTower':
                    self.assertEqual(effect['targets'],[7,56])
                    self.assertEqual(effect['mask'],(1<<7)|(1<<56))
                    hits.append(node['id'])
                elif row['UNIT']=='Fortress':
                    self.assertEqual(effect['targets'],[56])
                    self.assertEqual(effect['mask'],1<<56)
                    hits.append(node['id'])
                else:
                    self.assertNotIn(56,effect['targets'],'No unrelated role acquires Fortress')
        self.assertGreater(len(hits),0)
        rust=(ROOT/'src/ee2_research.rs').read_text(encoding='utf8')
        self.assertIn('Festungen und Wachtürme',rust)


if __name__ == '__main__':
    unittest.main()
