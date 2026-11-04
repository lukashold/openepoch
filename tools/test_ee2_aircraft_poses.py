"""Real-asset checks for native aircraft rotors and per-part constant poses."""
import hashlib,json,pathlib,unittest,zipfile
import numpy as np
from extract_ee2_models import ROOT,OUT,INSTALL,AIRCRAFT_VISUALS,Reader,scene,sample_pose,mesh_pose,AXIS
from import_ee2 import block,without_comments

class AircraftPoses(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.z=zipfile.ZipFile(INSTALL/'zips/graphics.zip');cls.db=zipfile.ZipFile(INSTALL/'zips/db.zip')
  manifest=json.loads((OUT/'models/manifest.json').read_text(encoding='utf8'))
  cls.assets=[]
  for row in manifest['models']:
   if row['id'] not in AIRCRAFT_VISUALS or 'progressionBinding'in row:continue
   for level,url in [('near',row['url']),*([('far',row['lodFarUrl'])]if row.get('lodFarUrl')else[])]:
    data=json.loads((ROOT/'docs'/url).read_text(encoding='utf8'));raw=cls.z.read(data['source']['entry']);reader=Reader(raw).parse();cls.assets.append((row,level,data,reader,raw))
 @classmethod
 def tearDownClass(cls):cls.z.close();cls.db.close()
 def test_exact_visual_and_state_bindings_are_original_database_declarations(self):
  for row,level,data,reader,raw in self.assets:
   with self.subTest(id=row['id'],level=level):
    binding=data['aircraftVisualBinding'];unit,epoch,path=AIRCRAFT_VISUALS[row['id']]
    self.assertEqual(binding['unitType'],unit);self.assertEqual(binding['originalEpoch'],epoch);self.assertEqual(binding['database']['entry'],path)
    source=self.db.read(path);self.assertEqual(hashlib.sha256(source).hexdigest(),binding['database']['sha256']);self.assertEqual(hashlib.sha256(raw).hexdigest(),data['source']['sha256'])
    definition=block(without_comments(source.decode('cp1252')),'UnitModel',binding['visualDefinition']);self.assertTrue(definition)
    self.assertIn(pathlib.PurePosixPath(data['source']['entry']).stem.casefold()+'.nif',definition.casefold())
    self.assertEqual({r['state']for r in binding['animationStates']},{'Idle','Walk','Attack1'})
    for state in binding['animationStates']:
     self.assertIn(state['declaredAnimName'].casefold(),definition.casefold());self.assertEqual(hashlib.sha256(self.z.read(state['entry'])).hexdigest(),state['sha256'])
    self.assertEqual({r['name']for r in data['animationSources']},{'idle','walk','combat'})
    self.assertTrue(all(r['frames']==48 for r in data['animationSources']))
 def test_all_48_source_samples_match_every_export_and_constant_pose_omission(self):
  for row,level,data,reader,raw in self.assets:
   n=data['normalization'];scale=n['scale'];offset=np.array(n['offset']);parts=data['parts']
   for clip in data['animationSources']:
    animation=Reader(self.z.read(clip['entry'])).parse()
    for f in range(48):
     pose,duration=sample_pose(animation,reader.objects,clip['duration']*f/48);world,*_=scene(reader,pose,level)
     self.assertAlmostEqual(duration,clip['duration'])
     for p in parts:
      with self.subTest(id=row['id'],level=level,clip=clip['name'],part=p['name'],frame=f):
       vertices,normals=mesh_pose(reader,p['sourceNode'],world);expected=(vertices+offset)*scale
       exported=p.get('animations',{}).get(clip['name'])
       if exported:
        frame=exported['frames'][0 if exported.get('constantPose')else f]
        if exported.get('constantPose'):self.assertEqual(exported['sampledFrames'],48);self.assertEqual(len(exported['frames']),1)
       else:frame=p
       np.testing.assert_allclose(expected,np.array(frame['positions']).reshape(-1,3),atol=1.1e-6,rtol=0)
       np.testing.assert_allclose(normals,np.array(frame['normals']).reshape(-1,3),atol=1.1e-6,rtol=0)
       self.assertLess(np.max(np.abs(expected)),1.1,'No displaced or exploded native mesh')
 def test_fast_rotor_radius_is_rigid_and_morph_interpolation_does_not_collapse_blades(self):
  checked=0;comparison=[]
  for row,level,data,reader,raw in self.assets:
   clip=next(c for c in data['animationSources']if c['name']=='walk');animation=Reader(self.z.read(clip['entry'])).parse();targets={}
   for sequence in animation.objects:
    for cb in sequence.get('Controlled Blocks',[]):
     ctrl=animation.objects[cb['Controller']];d=animation.objects[ctrl['Data']];axes=d.get('XYZ Rotations',[])
     if not axes:continue
     spans=[np.ptp([k['Value']for k in g.get('Keys',[])])if g.get('Keys')else 0 for g in axes]
     if max(spans)>6:targets[cb['Target Name']]=int(np.argmax(spans))
   parents={child:o['_id']for o in reader.objects for child in o.get('Children',[])};pose,_=sample_pose(animation,reader.objects,0);world,*_=scene(reader,pose,level);found=0
   for p in data['parts']:
    ancestor=p['sourceNode']
    while ancestor in parents and reader.objects[ancestor].get('Name')not in targets:ancestor=parents[ancestor]
    name=reader.objects[ancestor].get('Name')
    if name not in targets:continue
    axis=world[ancestor][:3,:3][:,targets[name]]@AXIS.T;axis/=np.linalg.norm(axis)
    center=(world[ancestor][:3,3]@AXIS.T+np.array(data['normalization']['offset']))*data['normalization']['scale']
    frames=np.array([f['positions']for f in p['animations']['walk']['frames']]).reshape(48,-1,3);r=frames-center
    radial=np.linalg.norm(r-(r@axis)[:,:,None]*axis,axis=2);np.testing.assert_allclose(radial,np.broadcast_to(radial[0],radial.shape),atol=2.5e-6,rtol=0)
    valid=radial[0]>.01;self.assertTrue(np.any(valid));mid=(frames+np.roll(frames,-1,axis=0))*.5-center;midradius=np.linalg.norm(mid-(mid@axis)[:,:,None]*axis,axis=2)
    ratio=float(np.min(midradius[:,valid]/radial[:,valid]));self.assertGreater(ratio,.975,'48 sample interpolation preserves rotor radius')
    old=frames[::6];oldmid=(old+np.roll(old,-1,axis=0))*.5-center;oldradius=np.linalg.norm(oldmid-(oldmid@axis)[:,:,None]*axis,axis=2);oldratio=float(np.min(oldradius[:,valid]/radial[::6,valid]));self.assertLess(oldratio,.8,'Eight-frame sampling would visibly collapse these fast blades')
    comparison.append({'id':row['id'],'lod':level,'node':name,'minimumInterpolatedRadiusRatio48':ratio,'minimumInterpolatedRadiusRatio8':oldratio});checked+=1;found+=1
   self.assertEqual(found,{37:1,38:2,41:2}[row['id']])
  self.assertEqual(checked,7)
  (OUT/'models/aircraft-animation-validation.json').write_text(json.dumps({'scope':'Real native source geometry/KF sampling and rigid-rotation radius; quadratic tangent/event equivalence is not claimed.','rotors':comparison},indent=2),encoding='utf8')
 def test_native_far_model_shares_space_and_sampling(self):
  for row,level,data,reader,raw in self.assets:
   if level!='far':continue
   near=json.loads((ROOT/'docs'/row['url']).read_text(encoding='utf8'));self.assertEqual(data['normalization'],near['normalization']);self.assertEqual(data['animationSources'],near['animationSources']);self.assertEqual(data['aircraftVisualBinding'],near['aircraftVisualBinding'])

if __name__=='__main__':unittest.main()
