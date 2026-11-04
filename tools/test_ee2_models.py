"""Regression checks for the independent NIF reader and coordinate conversion."""
import json,pathlib,struct,types,unittest
import numpy as np
from nif_reader import Reader
from extract_ee2_models import ROOT,rotation,mesh_pose,scene

class Models(unittest.TestCase):
    def test_bounded_header_and_complete_consumption(self):
        data=b'Gamebryo File Format, Version 10.1.0.0\n'+struct.pack('<IIIHII',0x0a010000,0,0,0,0,0)
        self.assertEqual(Reader(data).parse().objects,[])
        with self.assertRaises(AssertionError):Reader(data+b'x').parse()
        with self.assertRaises((AssertionError,ValueError,struct.error)):Reader(data[:-3]).parse()

    def test_nif_row_rotation_becomes_column_rotation(self):
        stored={'m11':0,'m12':-1,'m13':0,'m21':1,'m22':0,'m23':0,'m31':0,'m32':0,'m33':1}
        np.testing.assert_allclose(rotation(stored)@np.array([1,0,0]),[0,-1,0],atol=1e-8)

    def test_skin_offset_order_is_not_commuted(self):
        data={'Vertices':[dict(x=1,y=0,z=0)],'Normals':[dict(x=1,y=0,z=0)]}
        objs=[{}, {}, {'Data':3,'Skin Instance':4},data,{'Data':5,'Skeleton Root':0,'Bones':[1]},
              {'Skin Transform':{'Translation':dict(x=0,y=2,z=0)},'Bone List':[{'Skin Transform':{'Translation':dict(x=-1,y=0,z=0)},'Vertex Weights':[dict(Index=0,Weight=1)]}]}]
        bone=np.eye(4);bone[:3,:3]=[[0,-1,0],[1,0,0],[0,0,1]]
        geom=np.eye(4);geom[0,3]=10
        v,n=mesh_pose(types.SimpleNamespace(objects=objs),2,{0:np.eye(4),1:bone,2:geom})
        np.testing.assert_allclose(v,[[10,0,-2]],atol=1e-8)
        np.testing.assert_allclose(n,[[0,0,-1]],atol=1e-8)

    def test_lod_nearest_branch_only(self):
        identity=dict(Translation={},Rotation={},Scale=1,Properties=[],Flags=0)
        objs=[dict(identity,_id=0,_type='NiLODNode',Name='LOD',Children=[1,2,3],**{'LOD Level Data':4}),
              *[dict(identity,_id=i,_type='NiTriShape',Name=f'level{i}',Children=[]) for i in [1,2,3]],
              {'LOD Levels':[{'Near Extent':0},{'Near Extent':10},{'Near Extent':100}]}]
        _,_,selected,lods,_=scene(types.SimpleNamespace(objects=objs,footer={'Roots':[0]}))
        self.assertEqual(selected,[1]);self.assertEqual(lods[0]['selected'],0)
        objs[3]['Flags']=1 # Application-cull flag on an inactive LOD must not hide the chosen far branch.
        _,_,selected,lods,_=scene(types.SimpleNamespace(objects=objs,footer={'Roots':[0]}),lod='far')
        self.assertEqual(selected,[3]);self.assertEqual(lods[0]['selected'],2)
        self.assertTrue(lods[0]['selectedChildAppCulled'])

    def test_zero_scale_visibility_pose_stays_finite(self):
        objs=[{'Data':1,'Skin Instance':-1},{'Vertices':[dict(x=1,y=0,z=0)],'Normals':[dict(x=0,y=0,z=1)]}]
        v,n=mesh_pose(types.SimpleNamespace(objects=objs),0,{0:np.diag([0,0,0,1])})
        self.assertTrue(np.isfinite(v).all() and np.isfinite(n).all())
        np.testing.assert_allclose(v,[[0,0,0]]);self.assertAlmostEqual(float(np.linalg.norm(n[0])),1)

    def test_exported_native_far_lods_keep_near_space(self):
        """A renderer can switch LOD without changing the entity's transform."""
        manifest=ROOT/'docs/local-original/models/manifest.json'
        if not manifest.exists():self.skipTest('Local original asset export unavailable')
        rows=json.loads(manifest.read_text(encoding='utf8'))['models'];checked=set()
        for row in rows:
            if not row.get('lodFarUrl') or row['url'] in checked:continue
            checked.add(row['url'])
            with self.subTest(model=row['url']):
                near=json.loads((ROOT/'docs'/row['url']).read_text(encoding='utf8'))
                far=json.loads((ROOT/'docs'/row['lodFarUrl']).read_text(encoding='utf8'))
                self.assertEqual(far['normalization'],near['normalization'])
                self.assertEqual(far['source'],near['source'])
                self.assertEqual(far['nearModelUrl'],row['url'])
                self.assertEqual(far['lodPolicy'],'furthest-native-range')
                # Earlier valid near exports omit the optional false static flag.
                canon=lambda sources:[dict(s,static=bool(s.get('static',False))) for s in sources]
                self.assertEqual(canon(far['animationSources']),canon(near['animationSources']))
                lowtris=sum(len(p['indices'])//3 for p in far['parts'])
                hightris=sum(len(p['indices'])//3 for p in near['parts'])
                self.assertGreater(lowtris,0);self.assertLess(lowtris,hightris)
                self.assertEqual(lowtris,row['lodFarTriangles']);self.assertEqual(hightris,row['lodNearTriangles'])
                self.assertTrue(far['lodSelection'])
                for selection in far['lodSelection']:
                    ranges=selection['sourceRanges'];self.assertTrue(ranges)
                    choice=max(range(len(ranges)),key=lambda i:ranges[i]['Near Extent'])
                    self.assertEqual(selection['selected'],choice)
                for part in far['parts']:
                    self.assertTrue(np.isfinite(part['positions']).all())
                    self.assertEqual(len(part['normals']),len(part['positions']))
                    self.assertEqual(len(part['uvs'])*3,len(part['positions'])*2)
                    self.assertLess(max(part['indices']),len(part['positions'])//3)
                    for clip in part.get('animations',{}).values():
                        for frame in clip['frames']:
                            self.assertEqual(len(frame['positions']),len(part['positions']))
                            self.assertEqual(len(frame['normals']),len(part['normals']))
                            self.assertTrue(np.isfinite(frame['positions']).all())
                            self.assertTrue(np.isfinite(frame['normals']).all())
        self.assertGreater(len(checked),0,'Native far export must contain at least one reduced original mesh')

if __name__=='__main__':unittest.main()
