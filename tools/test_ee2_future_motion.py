"""Real original Mech gait, Myrmidon wheels and honest future-motion limits."""
import hashlib,json,pathlib,unittest,zipfile
import numpy as np
from extract_ee2_models import ROOT,OUT,INSTALL,Reader,scene,sample_pose,mesh_pose,NATIVE_GROUND_MOTION
from import_ee2 import block,without_comments

class FutureMotion(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.z=zipfile.ZipFile(INSTALL/'zips/graphics.zip');cls.db=zipfile.ZipFile(INSTALL/'zips/db.zip');m=json.loads((OUT/'models/manifest.json').read_text(encoding='utf8'));cls.assets=[]
  for id in [43,46]:
   row=next(r for r in m['models']if r['id']==id and'progressionBinding'not in r);d=json.loads((ROOT/'docs'/row['url']).read_text(encoding='utf8'));raw=cls.z.read(d['source']['entry']);cls.assets.append((id,d,Reader(raw).parse()))
 @classmethod
 def tearDownClass(cls):cls.z.close();cls.db.close()
 def test_declared_model_and_states_not_just_similarly_named_files(self):
  for id,d,r in self.assets:
   b=d['nativeMotionBinding'];unit,epoch,path=NATIVE_GROUND_MOTION[id];self.assertEqual(b['unitType'],unit);self.assertEqual(b['database']['entry'],path);raw=self.db.read(path);self.assertEqual(hashlib.sha256(raw).hexdigest(),b['database']['sha256']);definition=block(without_comments(raw.decode('cp1252')),'UnitModel',b['visualDefinition']);self.assertIn(pathlib.PurePosixPath(d['source']['entry']).stem.casefold()+'.nif',definition.casefold())
   self.assertEqual({a['name']for a in d['animationSources']},{'idle','walk','combat'}if id==43 else{'idle','walk'})
   for a in b['animationStates']:self.assertIn(a['declaredAnimName'].casefold(),definition.casefold());self.assertEqual(hashlib.sha256(self.z.read(a['entry'])).hexdigest(),a['sha256'])
   if id==46:
    self.assertTrue(b['unboundAttack']);self.assertTrue(all(a['unsupportedControllers']==['NiTextureTransformController']for a in b['animationStates']));self.assertTrue(any('UV scrolling'in x for x in d['limitations']))
 def test_every_native_pose_matches_export_with_no_per_frame_fit(self):
  metrics=[]
  for id,d,r in self.assets:
   offset=np.array(d['normalization']['offset']);scale=d['normalization']['scale']
   for a in d['animationSources']:
    self.assertEqual(a['frames'],24 if id==43 else 48);kf=Reader(self.z.read(a['entry'])).parse();bounds=[]
    for f in range(a['frames']):
     pose,duration=sample_pose(kf,r.objects,a['duration']*f/a['frames']);world,*_=scene(r,pose);self.assertEqual(duration,a['duration']);positions=[]
     for p in d['parts']:
      v,n=mesh_pose(r,p['sourceNode'],world);expected=(v+offset)*scale;export=p.get('animations',{}).get(a['name']);frame=export['frames'][0 if export.get('constantPose')else f]if export else p
      np.testing.assert_allclose(np.array(frame['positions']).reshape(-1,3),expected,atol=1.1e-6,rtol=0);np.testing.assert_allclose(np.array(frame['normals']).reshape(-1,3),n,atol=1.1e-6,rtol=0);positions.extend(expected)
     v=np.array(positions);self.assertGreater(v[:,1].min(),-.02);self.assertLess(v[:,1].max(),1.1);self.assertLess(np.max(np.abs(v[:,[0,2]])),.9);bounds.append((v.min(0),v.max(0)))
    metrics.append({'id':id,'clip':a['name'],'source':a['entry'],'source_sha256':a['sha256'],'samples':a['frames'],'duration':a['duration'],'min':np.min([b[0]for b in bounds],axis=0).tolist(),'max':np.max([b[1]for b in bounds],axis=0).tolist()})
  (OUT/'models/future-motion-validation.json').write_text(json.dumps({'scope':'Direct native NIF/KF pose comparison; these aggregate bounds contain no mesh vertices. UV controllers and unused cached Attack files are not simulated.','clips':metrics},indent=2),encoding='utf8')
 def test_mech_and_myrmidon_walks_have_real_geometry_motion(self):
  for id,d,r in self.assets:
   motions=[]
   for p in d['parts']:
    a=p.get('animations',{}).get('walk')
    if not a or a.get('constantPose'):continue
    f=np.array([x['positions']for x in a['frames']]);motions.append(float(np.max(np.abs(f-f[0]))))
   self.assertGreater(max(motions),.4 if id==43 else .02)
   if id==46:self.assertEqual(len(motions),7,'Seven native wheel/piece groups move; static hull/turret are not invented animations')
 def test_future_aircraft_native_clips_are_actually_constant_and_manta_has_no_matching_external_kf(self):
  stem='af15_atmospheric';member=next(n for n in self.z.namelist()if pathlib.PurePosixPath(n).stem.casefold()==stem and n.casefold().endswith('.nifcache'));r=Reader(self.z.read(member)).parse();facts=[]
  for state in ['idle','walk','attack']:
   entry=next(n for n in self.z.namelist()if pathlib.PurePosixPath(n).name.casefold()==f'{stem}_{state}.kf');raw=self.z.read(entry);kf=Reader(raw).parse();pose,duration=sample_pose(kf,r.objects,0);world,_,nodes,*_=scene(r,pose);base={i:mesh_pose(r,i,world)[0]for i in nodes};delta=0.0
   for f in range(16):
    pose,_=sample_pose(kf,r.objects,duration*f/16);w,*_=scene(r,pose)
    for i in nodes:delta=max(delta,float(np.max(np.abs(mesh_pose(r,i,w)[0]-base[i]))))
   self.assertLess(delta,1e-10);facts.append({'id':44,'clip':state,'entry':entry,'sha256':hashlib.sha256(raw).hexdigest(),'duration':duration,'maximumNativeVertexMotion':delta,'action':'No vertex movement fabricated; original constant aircraft pose retained.'})
  self.assertFalse(any(pathlib.PurePosixPath(n).name.casefold().startswith('ns15_manta_')and n.casefold().endswith('.kf')for n in self.z.namelist()))
  facts.append({'id':45,'sourceStem':'ns15_manta','matchingExternalKFCount':0,'action':'Original rigid geometry retained; native procedural/controller motion is not claimed.'})
  (OUT/'models/future-priority-findings.json').write_text(json.dumps(facts,indent=2),encoding='utf8')

if __name__=='__main__':unittest.main()

