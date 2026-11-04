import hashlib,json,pathlib,struct,unittest
from import_ee2_ground_seasons import seasonal_urls
ROOT=pathlib.Path(__file__).resolve().parents[1]


class GroundRuntime(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        output=ROOT/'docs/local-original'
        cls.compact=json.loads((output/'ground-decals.json').read_text(encoding='utf8'))
        cls.audit=json.loads((output/'ground-decals-manifest.json').read_text(encoding='utf8'))

    def test_variable_source_lists_and_physical_aliases_are_preserved(self):
        sources={row['id']:row for row in self.audit['entries']+self.audit['unresolvedBindings']};lengths=set();count=0
        for entry in self.compact['entries']+self.compact['unresolvedBindings']:
            for state,groups in [('Idle',entry),('Death',entry['death'])]:
                for category in ('rural','urban'):
                    for row,source in zip(groups[category],sources[entry['id']]['states'][state][category]):
                        self.assertEqual(row['seasonalUrls'],seasonal_urls(source));lengths.add(len(row['seasonalUrls']));count+=1
        self.assertEqual(lengths,{1,4,5,12});self.assertEqual(count,168)

    def test_native_selection_and_world_dimension_profile(self):
        selection=self.compact['runtimeSelection'];self.assertEqual(selection['nativeEpochFirst'],[1,6,11])
        self.assertEqual(selection['cityRadiusFactor'],struct.unpack('<f',struct.pack('<f',2**.5))[0])
        self.assertEqual({e['id'] for e in self.compact['entries'] if e.get('placementProfile')=='native-unit-factory-undertexture'},
                         {0,1,2,3,4,6,47,48,49,50,51,52,55,56})
        self.assertEqual(self.compact['unresolvedBindings'],[],'Native size payload now proves Farm fallback dimensions')
        farm=next(e for e in self.compact['entries']if e['id']==2)
        self.assertEqual(farm['width'],struct.unpack('<f',struct.pack('<f',1.5*selection['cityRadiusFactor']))[0])

    def test_metadata_upgrade_changes_no_physical_original_png(self):
        for url,row in self.audit['textures'].items():
            self.assertEqual(hashlib.sha256((ROOT/'docs'/url).read_bytes()).hexdigest(),row['png_sha256'])

    def test_repeated_sources_remain_repeated_instead_of_resampling_to_twelve(self):
        fixture={'physicalVariants':[{'originalPath':'a','url':'a.png'},{'originalPath':'b','url':'b.png'}],
                 'seasonalPaths':['a','b','b','b','b']}
        self.assertEqual(seasonal_urls(fixture),['a.png','b.png','b.png','b.png','b.png'])
        with self.assertRaises(KeyError):seasonal_urls({**fixture,'seasonalPaths':['missing']})


if __name__=='__main__':unittest.main()
