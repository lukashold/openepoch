"""Independent installed-source, original RGBA, mesh-link and portable proof."""
import csv,hashlib,json,pathlib,re,struct,unittest,zipfile
from types import SimpleNamespace
from PIL import Image
from nif_reader import Reader
from extract_ee2_textures import decode_texcache
from extract_ee2_team_textures import apply_patch
from test_ee2_city_evidence import PE
from import_ee2_model_seasons import inherited_sets
ROOT=pathlib.Path(__file__).resolve().parents[3];DOCS=ROOT/'outputs/OpenEpoch/docs'
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II/zips')
sha=lambda b:hashlib.sha256(b).hexdigest()

class OriginalSeasonSources(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=json.loads((DOCS/'local-original/model-seasons.json').read_text())
        cls.teams=json.loads((DOCS/cls.data['teamManifest']).read_text())
        cls.db=zipfile.ZipFile(INSTALL/'db.zip');cls.graphics=zipfile.ZipFile(INSTALL/'graphics.zip');cls.tex=zipfile.ZipFile(INSTALL/'textures.zip')
    @classmethod
    def tearDownClass(cls):
        cls.db.close();cls.graphics.close();cls.tex.close()
    def test_every_climate_list_matches_original_csv_exactly(self):
        for climate,sets in self.data['sets'].items():
            source=self.data['source']['climates'][climate];raw=self.db.read(source['entry']);self.assertEqual(sha(raw),source['sha256'])
            rows={row[0].strip().casefold():row[1].split()for row in csv.reader(raw.decode('cp1252').splitlines())if len(row)>1 and row[0].strip()and not row[0].lstrip().startswith('//')}
            for key,entry in sets.items():
                self.assertEqual(entry['paths'],rows[key]);self.assertEqual(len(entry['paths']),len(entry['urls']))
        lengths={len(v['paths'])for climate in self.data['sets'].values()for v in climate.values()}
        self.assertEqual(lengths,{1,8,11,12,13})
    def test_all_nif_extra_data_records_and_original_mesh_links(self):
        cache={}
        for url,model in self.data['models'].items():
            source=model['source'];entry=source['entry']
            raw=cache.setdefault(entry,self.graphics.read(entry));self.assertEqual(sha(raw),source['sha256'])
            for binding in model['parts'].values():
                p=binding['extraDataOffset'];self.assertEqual(struct.unpack_from('<I',raw,p)[0],0);p+=4
                n=struct.unpack_from('<I',raw,p)[0];p+=4;self.assertEqual(raw[p:p+n],b'UserPropBuffer');p+=n
                n=struct.unpack_from('<I',raw,p)[0];p+=4;text=raw[p:p+n]
                self.assertEqual(sha(text),binding['userPropSha256'])
                self.assertEqual(re.search(rb'AnimatedTextureSetName\s*=\s*<([^>]+)>',text)[1].decode('cp1252'),binding['key'])
        # Independent structure checks on low/high-epoch buildings and both trees.
        samples=['bld_citycenter1_we.json','bld_citycenter13_we.json','tre_oak1.json','tre_pine.json','bld_farm_wheat.json','bld_oilrig.json']
        selected=[(u,m)for u,m in self.data['models'].items()if pathlib.PurePosixPath(u).name in samples]
        self.assertGreaterEqual(len(selected),3)
        for url,model in selected:
            r=Reader(self.graphics.read(model['source']['entry'])).parse()
            for node,binding in model['parts'].items():
                mesh=r.objects[int(node)];self.assertEqual(mesh['Name'],binding['name'])
                owner=r.objects[binding['bindingNode']];self.assertIn(binding['extraDataNode'],owner['Extra Data List'])
                path=binding['meshSourcePath'];self.assertEqual(path[-1],int(node));self.assertIn(binding['bindingNode'],path)
                for parent,child in zip(path,path[1:]):self.assertIn(child,r.objects[parent].get('Children',[]))
                extra=r.objects[binding['extraDataNode']];self.assertEqual(extra['_start'],binding['extraDataOffset'])
    def test_all_neutral_pngs_are_exact_original_top_mip_pixels(self):
        for key,asset in self.data['textures'].items():
            raw=self.tex.read(asset['archive_member']);self.assertEqual(sha(raw),asset['source_sha256'])
            actual_path=DOCS/asset['url'];self.assertEqual(sha(actual_path.read_bytes()),asset['png_sha256'])
            original,meta=decode_texcache(raw);actual=Image.open(actual_path).convert('RGBA')
            self.assertEqual(actual.size,(meta['width'],meta['height']));self.assertEqual(actual.tobytes(),original.tobytes(),key)
    def test_every_patch_png_hash_and_representative_winter_pixels(self):
        self.assertTrue(self.teams['colour_mapping_verified']);self.assertFalse(self.teams['errors'])
        selected={'bld_citycenter1_we_wi','bld_house13_wi','bld_fortress14_1_wi','bld_citycenter10_we2_wi'};measured=0
        for key,family in self.teams['textures'].items():
            self.assertEqual(len(family['variants']),11)
            raw=self.tex.read(family['base_source']['entry']);self.assertEqual(sha(raw),family['base_source']['sha256'])
            for suffix,variant in family['variants'].items():
                file=DOCS/variant['url'];self.assertEqual(sha(file.read_bytes()),variant['png_sha256'])
                source=variant['patch_source'];patch=self.tex.read(source['entry']);self.assertEqual(sha(patch),source['sha256'])
                if key in selected and suffix in 'ABCG':
                    rebuilt,_=apply_patch(raw,patch);expected,_=decode_texcache(rebuilt)
                    self.assertEqual(Image.open(file).convert('RGBA').tobytes(),expected.tobytes());measured+=1
        self.assertGreaterEqual(measured,12)
    def test_geometry_animation_and_existing_team_manifests_unchanged(self):
        base=ROOT/'work/season-model-baseline';hashes=json.loads((base/'model-hashes.json').read_text())
        self.assertEqual(len(hashes),191)
        for url,digest in hashes.items():self.assertEqual(sha((DOCS/url).read_bytes()),digest,url)
        path='docs/local-original/team-textures/manifest.json'
        self.assertEqual((base/path).read_bytes(),(ROOT/'outputs/OpenEpoch'/path).read_bytes())
        path='docs/local-original/models/manifest.json'
        before=json.loads((base/path).read_bytes());after=json.loads((ROOT/'outputs/OpenEpoch'/path).read_bytes())
        # Native Farm activation intentionally replaces the two old Farm visual
        # adapters. Every other source/LOD/animation/provenance row stays exact.
        rows=lambda data:{(r['id'],r.get('epoch',0)):r for r in data['models']if r['id']!=2}
        self.assertEqual(rows(before),rows(after))
        self.assertEqual([r for r in before['models']if r['id']!=2],[r for r in after['models']if r['id']!=2],'Existing draw order is preserved as well as every binding value')
        self.assertEqual({k:v for k,v in before.items()if k not in ['models','errors']},{k:v for k,v in after.items()if k not in ['models','errors']})
    def test_native_parser_uses_explicit_field_and_current_seasonal_database(self):
        pe=PE((ROOT/'work/EE2-engine.exe').read_bytes())
        self.assertEqual(pe.read(0xc32614,23),b'AnimatedTextureSetName\0')
        self.assertEqual(pe.read(0xc3262c,15),b'UserPropBuffer\0')
        # The parser requests UserPropBuffer, then searches AnimatedTextureSetName.
        self.assertEqual(pe.read(0x5f845b,5),b'\x68'+struct.pack('<I',0xc3262c))
        self.assertEqual(pe.read(0x5f8472,5),b'\x68'+struct.pack('<I',0xc32614))
        # 0052F330 returns active database+0x70, used by00684490 list lookup.
        self.assertEqual(pe.read(0x52f330,9).hex(),'a1448bc9008b4070c3')

class NativeSubtreeBinding(unittest.TestCase):
    def extra(self,key,index):return {'_type':'NiStringExtraData','_start':index*100,'Name':'UserPropBuffer','String Data':f'AnimatedTextureSetName = <{key}>','_id':index}
    def test_annotated_parent_applies_to_every_descendant_and_wins_over_inner_key(self):
        r=SimpleNamespace(objects=[{'Children':[1],'Extra Data List':[3]},{'Children':[2],'Extra Data List':[4]},{'Name':'mesh'},self.extra('outer',3),self.extra('inner',4)],footer={'Roots':[0]})
        bindings=inherited_sets(r);self.assertEqual(bindings[2]['key'],'outer');self.assertEqual(bindings[2]['bindingNode'],0);self.assertEqual(bindings[2]['meshSourcePath'],[0,1,2])
    def test_direct_mesh_binding_without_parent_annotation(self):
        r=SimpleNamespace(objects=[{'Children':[1]},{'Extra Data List':[2]},self.extra('mesh_set',2)],footer={'Roots':[0]})
        value=inherited_sets(r)[1];self.assertEqual(value['bindingNode'],1);self.assertEqual(value['key'],'mesh_set')
    def test_cycle_and_conflicting_multi_parent_binding_are_rejected(self):
        r=SimpleNamespace(objects=[{'Children':[1],'Extra Data List':[3]},{'Children':[0]}, {},self.extra('outer',3)],footer={'Roots':[0]})
        with self.assertRaises(ValueError):inherited_sets(r)
        r=SimpleNamespace(objects=[{'Children':[2],'Extra Data List':[3]},{'Children':[2],'Extra Data List':[4]},{},self.extra('left',3),self.extra('right',4)],footer={'Roots':[0,1]})
        with self.assertRaises(ValueError):inherited_sets(r)

if __name__=='__main__':unittest.main()
