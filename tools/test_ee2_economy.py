"""Check the auditable economy dataset, including original-source hashes."""
import hashlib,json,pathlib,unittest,zipfile
ROOT=pathlib.Path(__file__).resolve().parents[1]

class Economy(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=json.loads((ROOT/'docs/ee2-economy.json').read_text(encoding='utf8'))

    def test_nine_resources_and_special_epoch_boundaries(self):
        rows=self.data['naturalResources']
        self.assertEqual([r['id'] for r in rows],list(range(9)))
        self.assertEqual([r['name'] for r in rows],['Food','Wood','Stone','Gold','Tin','Iron','Saltpeter','Oil','Uranium'])
        self.assertEqual([r['revealEpoch'] for r in rows],[1,1,1,1,1,4,7,10,13])
        self.assertEqual([r['activeEpochEndExclusive'] for r in rows[4:]],[7,10,13,None,None])
        expected=[[4],[4],[4],[4,5],[4,5],[4,5],[5,6],[5,6],[5,6],[6,7],[6,7],[6,7],[7,8],[7,8],[7,8]]
        for e,active in enumerate(expected):
            actual=[r['id'] for r in rows[4:] if r['specialSlotByEpoch'][e]!=-1]
            self.assertEqual(actual,active)
        self.assertEqual(self.data['nonNaturalTypes'],{'TechPoints':9,'PopCap':10})

    def test_oil_and_uranium_require_garrison_buildings(self):
        nodes={r['resourceId']:r for r in self.data['resourceNodes']}
        for resource,unit,placement,epoch in [(7,'OilRig','OnOil',10),(8,'UraniumMine','OnUranium',13)]:
            self.assertEqual(nodes[resource]['maxWorkers'],0)
            self.assertFalse(nodes[resource]['directWorkerGathering'])
            b=next(r for r in self.data['productionStructures'] if r['unitType']==unit)
            self.assertEqual(b['placementType'],placement)
            self.assertEqual(b['garrisonSlots'],6)
            self.assertEqual(b['gatherPerOccupiedSlotPerSecond'],.75)
            self.assertEqual(b['maxBaseGatherPerSecond'],4.5)
            self.assertEqual(b['build']['epoch'],epoch)
            self.assertEqual(b['build']['seconds'],45)
            self.assertEqual(b['build']['costByOriginalId'],[0,100,0,0,0,0,0,0,0])

    def test_carry_and_dropoff_are_distinct_from_player_stock(self):
        gather=self.data['citizen']['gatherFarm']
        self.assertEqual((gather['carryLimit'],gather['farmCarryLimit']),(15,15))
        self.assertEqual(gather['farmRate'],4.37)
        self.assertEqual((gather['treeCuttingRange'],gather['otherRange'],gather['dropoffRange']),(.9,.8,.7))
        stock=self.data['runtime']['stock'];self.assertEqual((stock['minimum'],stock['maximum']),(0,999999))
        self.assertEqual(stock['capAddress'],'00b413a4')
        drops={r['unitType']:r for r in self.data['dropoffs']}
        for unit in ('CityCenter','Warehouse'):
            self.assertEqual(drops[unit]['acceptedType'],'kResourceType_AllNaturalResource')
            self.assertEqual(drops[unit]['garrisonSlots'],5)
            self.assertEqual(drops[unit]['bonusPerOccupiedSlot'],.02)
        self.assertEqual(drops['Dock']['acceptedType'],'kResourceType_Food')

    def test_starting_presets_keep_all_source_columns(self):
        rows=self.data['startingResourceLevels'];self.assertEqual(len(rows),25)
        row=next(r for r in rows if r['NAME']=='Standard_1')
        self.assertEqual([row[k] for k in ('FOOD','WOOD','STONE','GOLD','TIN','IRON','SALTPETER','OIL','URANIUM')],[400,400,400,400,200,200,200,200,200])
        self.assertEqual(row['TECHPTS'],0)
        self.assertEqual({r['Level'] for r in rows},{'Low','Defensive','Standard','High','DeathMatch'})

    def test_farm_has_one_worker_and_its_own_rate(self):
        farm=self.data['farm']
        self.assertEqual(farm['maxWorkers'],1)
        self.assertEqual(farm['configuredAmount'],100)
        self.assertEqual(farm['gatherPerWorkerPerSecond'],4.37)
        self.assertEqual(farm['evidence']['source'],'db.zip!db/Units/farm.ddf')

    def test_original_source_and_ghidra_output_hashes(self):
        archives={}
        try:
            for name,record in self.data['sourceArchives'].items():
                path=pathlib.Path(record['path'])
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),record['sha256'])
                archives[name]=zipfile.ZipFile(path)
            for record in self.data['sources'].values():
                raw=archives[record['archive']].read(record['entry'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(),record['sha256'])
        finally:
            for archive in archives.values():archive.close()
        runtime=self.data['runtime']
        self.assertTrue(runtime['readOnly'])
        self.assertEqual(hashlib.sha256(pathlib.Path(runtime['enginePath']).read_bytes()).hexdigest(),runtime['engineSha256'])
        functions=set()
        for record in runtime['analysisFiles']:
            raw=pathlib.Path(record['file']).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(),record['sha256'])
            functions.update(record['functions'])
        self.assertTrue({'007ef6a0','007e7cf0','00884710','007ee230'}.issubset(functions))

if __name__=='__main__':unittest.main()
