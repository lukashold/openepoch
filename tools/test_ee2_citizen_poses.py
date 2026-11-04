"""Real original-asset regressions for all five citizen styles and both LODs.

Checks the actual source geometry/skin bind relation as well as exported clips.
It never fits or recenters individual animation frames.
"""
import hashlib,json,pathlib,sys,unittest,zipfile
import numpy as np
from extract_ee2_models import ROOT,OUT,INSTALL,CONVERTER_VERSION,Reader,scene,sample_pose,mesh_pose,local_matrix

def positions(parts,clip=None,frame=0):
    return np.array([v for p in parts for v in (p['animations'][clip]['frames'][frame]['positions'] if clip else p['positions'])]).reshape(-1,3)

class Citizens(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows=[r for r in json.loads((OUT/'models/manifest.json').read_text(encoding='utf8'))['models'] if r['id']==8]
        cls.archive=zipfile.ZipFile(INSTALL/'zips/graphics.zip')
        cls.idle=Reader(cls.archive.read('Graphics/Humans/sh_citizen_idle01.kf')).parse()
        cls.assets=[]
        for row in cls.rows:
            near=json.loads((ROOT/'docs'/row['url']).read_text(encoding='utf8'))
            far=json.loads((ROOT/'docs'/row['lodFarUrl']).read_text(encoding='utf8'))
            raw=cls.archive.read(near['source']['entry']);reader=Reader(raw).parse()
            cls.assets.append((row,near,far,reader,raw))
    @classmethod
    def tearDownClass(cls):cls.archive.close()

    def test_all_five_native_styles_present(self):
        self.assertEqual(sorted(r['epoch'] for r in self.rows),[0,3,6,9,12])

    def test_real_skin_global_cancels_nonidentity_geometry(self):
        nonidentity=0
        for row,near,far,reader,raw in self.assets:
            with self.subTest(epoch=row['epoch']):
                self.assertEqual(hashlib.sha256(raw).hexdigest(),near['source']['sha256'])
                world,_,selected,*_=scene(reader)
                for i in selected:
                    o=reader.objects[i]
                    if o.get('Skin Instance',-1)<0:continue
                    inst=reader.objects[o['Skin Instance']];skin=reader.objects[inst['Data']]
                    self.assertLess(np.max(np.abs(world[inst['Skeleton Root']]-np.eye(4))),1e-4)
                    np.testing.assert_allclose(world[i]@local_matrix(skin['Skin Transform']),np.eye(4),atol=1e-4)
                    if np.max(np.abs(world[i]-np.eye(4)))>.1:nonidentity+=1
        self.assertGreaterEqual(nonidentity,4,'Later source styles exercise nonidentity skin/geometry transforms')

    def test_source_idle_pose_matches_export_and_proportions(self):
        for row,near,far,reader,raw in self.assets:
            with self.subTest(epoch=row['epoch']):
                pose,_=sample_pose(self.idle,reader.objects,0);world,*_=scene(reader,pose)
                parts=[p for p in near['parts'] if p.get('skinned')]
                arrays=[mesh_pose(reader,p['sourceNode'],world)[0] for p in parts];v=np.concatenate(arrays)
                lo=v.min(0);hi=v.max(0);span=hi-lo
                self.assertTrue(5.7<span[1]<6.4,'Original upright source body height')
                self.assertTrue(1.8<span[0]<2.5,'Original idle body width')
                self.assertLess(abs(lo[1]),.03,'Original source feet near the ground')
                n=near['normalization'];self.assertAlmostEqual(n['scale'],.85/span[1],places=9)
                offset=np.array(n['offset'])
                for part,raw_positions in zip(parts,arrays):
                    np.testing.assert_allclose((raw_positions+offset)*n['scale'],np.array(part['positions']).reshape(-1,3),atol=1e-6)

    def test_near_and_far_use_same_space_and_bounded_clips(self):
        audit=[]
        for row,near,far,reader,raw in self.assets:
            self.assertEqual(near['normalization'],far['normalization'])
            for level,model in [('near',near),('far',far)]:
                with self.subTest(epoch=row['epoch'],level=level):
                    self.assertEqual(model['conversion']['version'],CONVERTER_VERSION)
                    body=[p for p in model['parts'] if p.get('skinned')];v=positions(body);span=v.max(0)-v.min(0)
                    self.assertTrue(.80<span[1]<.89);self.assertLess(span[0],.4);self.assertLess(span[2],.3)
                    self.assertLess(abs(v.min(0)[1]),.02)
                    clips={}
                    for clip in ['idle','walk','wood','mine','food','build','combat']:
                        bounds=[]
                        count=len(body[0]['animations'][clip]['frames'])
                        for f in range(count):
                            v=positions(body,clip,f);lo=v.min(0);hi=v.max(0)
                            self.assertTrue(np.isfinite(v).all())
                            self.assertGreater(lo[1],-.12,'Animation feet remain near ground')
                            self.assertLess(hi[1],1.15,'No exploded or oversized animation body')
                            self.assertLess(np.max(np.abs(v[:,[0,2]])),.7,'No transformed body offset')
                            if clip in ('idle','walk'):self.assertTrue(.72<hi[1]-lo[1]<1.0)
                            bounds.append({'min':lo.tolist(),'max':hi.tolist()})
                        clips[clip]=bounds
                    audit.append({'epoch':row['epoch'],'level':level,'source':model['source'],'normalization':model['normalization'],'clips':clips})
        (OUT/'models/citizen-pose-audit.json').write_text(json.dumps({'converterVersion':CONVERTER_VERSION,'models':audit},indent=2)+'\n',encoding='utf8')

if __name__=='__main__':unittest.main()
