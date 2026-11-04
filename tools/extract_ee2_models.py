"""Convert locally installed EE2 NIF geometry to browser meshes.

No game model is downloaded or recreated. Every vertex, UV and triangle comes
from the named archive entry. Original asset outputs belong in local-original/.
NIF structure facts follow niftools/nifxml; schema attribution is in nif_reader.py.
"""
import argparse,hashlib,json,pathlib,re,zipfile,math
import numpy as np
from nif_reader import Reader
from extract_ee2_textures import extract_texture,source_rows
from import_ee2 import block,without_comments
from native_materials import native_material_fields
ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
OUT=ROOT/'docs/local-original'
CONVERTER_VERSION='ee2-original-mesh-skin-world-v2'
AXIS=np.array([[1,0,0],[0,0,1],[0,-1,0]],dtype=float)

def vec(d,keys='xyz'):
    return np.array([d.get(k,0.0) for k in keys],dtype=float)
def rotation(d):
    return np.array([[d.get('m11',1),d.get('m12',0),d.get('m13',0)],
                     [d.get('m21',0),d.get('m22',1),d.get('m23',0)],
                     [d.get('m31',0),d.get('m32',0),d.get('m33',1)]],dtype=float).T
def local_matrix(o):
    m=np.eye(4);m[:3,:3]=rotation(o.get('Rotation',{}))*o.get('Scale',1);m[:3,3]=vec(o.get('Translation',{}));return m
def quaternion_matrix(q):
    w,x,y,z=np.asarray(q)/max(1e-12,np.linalg.norm(q))
    return np.array([[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],
                     [2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],
                     [2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]])
def interpolate(keys,t,quaternion=False):
    if not keys:return None
    a=keys[0];b=keys[-1]
    for i in range(1,len(keys)):
        if keys[i]['Time']>=t:a=keys[i-1];b=keys[i];break
    if t<=keys[0]['Time']:a=b=keys[0]
    if t>=keys[-1]['Time']:a=b=keys[-1]
    def value(k):
        v=k['Value']
        return vec(v,'wxyz') if quaternion else vec(v) if isinstance(v,dict) else float(v)
    av=value(a);bv=value(b);f=0.0 if a['Time']==b['Time'] else (t-a['Time'])/(b['Time']-a['Time'])
    if quaternion:
        av/=max(1e-12,np.linalg.norm(av));bv/=max(1e-12,np.linalg.norm(bv));dot=float(av@bv)
        if dot<0:bv=-bv;dot=-dot
        if dot>.9995:return (av+(bv-av)*f)/np.linalg.norm(av+(bv-av)*f)
        theta=math.acos(max(-1,min(1,dot)));return (math.sin((1-f)*theta)*av+math.sin(f*theta)*bv)/math.sin(theta)
    return av+(bv-av)*f
def sample_pose(animation,objects,t):
    nodes={o.get('Name'):o for o in objects if 'Translation' in o};pose={};duration=0.0
    for sequence in animation.objects:
        for cb in sequence.get('Controlled Blocks',[]):
            name=cb.get('Target Name');ctrl=animation.objects[cb['Controller']];data=animation.objects[ctrl['Data']]
            duration=max(duration,ctrl.get('Stop Time',0))
            if name not in nodes:continue
            o=nodes[name];m=local_matrix(o);r=None
            if data.get('Rotation Type')==4:
                xyz=[interpolate(g.get('Keys',[]),t) or 0.0 for g in data['XYZ Rotations']]
                a,b,c=xyz;rx=np.array([[1,0,0],[0,math.cos(a),-math.sin(a)],[0,math.sin(a),math.cos(a)]]);ry=np.array([[math.cos(b),0,math.sin(b)],[0,1,0],[-math.sin(b),0,math.cos(b)]]);rz=np.array([[math.cos(c),-math.sin(c),0],[math.sin(c),math.cos(c),0],[0,0,1]])
                r=rz@ry@rx
            elif data.get('Quaternion Keys'):
                r=quaternion_matrix(interpolate(data['Quaternion Keys'],t,True))
            tr=interpolate(data.get('Translations',{}).get('Keys',[]),t)
            sc=interpolate(data.get('Scales',{}).get('Keys',[]),t)
            if r is not None:m[:3,:3]=r*(sc if sc is not None else o.get('Scale',1))
            elif sc is not None:m[:3,:3]=rotation(o.get('Rotation',{}))*sc
            if tr is not None:m[:3,3]=tr
            pose[name]=m
    return pose,duration
def flat(a):
    return np.round(np.asarray(a).reshape(-1),6).tolist()
def triangles(data):
    if 'Triangles' in data:return np.array([[v['v1'],v['v2'],v['v3']] for v in data['Triangles']],dtype=np.uint32)
    result=[]
    for strip in data.get('Points',[]):
        for i in range(2,len(strip)):
            t=[strip[i-2],strip[i-1],strip[i]]
            if i%2:t[0],t[1]=t[1],t[0]
            if len(set(t))==3:result.append(t)
    return np.array(result,dtype=np.uint32)

def scene(reader,overrides=None,lod='near'):
    objects=reader.objects;world={};props={};selected=[];lods=[];culled=[]
    def walk(i,parent,inherited,force_visible=False):
        if i<0 or i>=len(objects):return
        o=objects[i]
        if 'Translation' not in o:return
        if o.get('Flags',0)&1 and not force_visible:culled.append(i);return
        local=local_matrix(o)
        if overrides and o.get('Name') in overrides:local=overrides[o['Name']]
        world[i]=parent@local
        p=dict(inherited)
        for prop in o.get('Properties',[]):p[objects[prop]['_type']]=objects[prop]
        props[i]=p
        if o['_type'] in ('NiTriShape','NiTriStrips'):selected.append(i)
        children=o.get('Children',[])
        if o['_type']=='NiLODNode' and children:
            # The base assets use NiRangeLODData, with the nearest LOD at slot 0.
            ld=objects[o.get('LOD Level Data',-1)] if o.get('LOD Level Data',-1)>=0 else {}
            ranges=ld.get('LOD Levels',[])
            chooser=max if lod=='far' else min
            chosen=chooser(range(len(ranges)),key=lambda n:ranges[n].get('Near Extent',0)) if ranges else (len(children)-1 if lod=='far' else 0)
            lods.append({'node':o.get('Name'),'selected':chosen,'children':len(children),'sourceRanges':ranges,'selectedChildAppCulled':bool(objects[children[chosen]].get('Flags',0)&1)})
            children=[children[chosen]]
        elif o['_type']=='NiSwitchNode' and children:
            chosen=o.get('Index',0)
            children=[children[chosen]] if chosen<len(children) else []
        for child in children:walk(child,world[i],p,force_visible=o['_type']=='NiLODNode')
    for root in reader.footer.get('Roots',[0]):walk(root,np.eye(4),{})
    return world,props,selected,lods,culled

def mesh_pose(reader,i,world):
    o=reader.objects[i];data=reader.objects[o['Data']];vertices=np.array([vec(v) for v in data.get('Vertices',[])])
    normals=np.array([vec(v) for v in data.get('Normals',[])])
    if len(normals)!=len(vertices):normals=np.tile([0,0,1],(len(vertices),1))
    if o.get('Skin Instance',-1)>=0:
        inst=reader.objects[o['Skin Instance']];skin=reader.objects[inst['Data']];positions=np.zeros_like(vertices);outnorm=np.zeros_like(normals);weights=np.zeros(len(vertices))
        root=world[inst['Skeleton Root']];invroot=np.linalg.inv(root);skintr=local_matrix(skin['Skin Transform'])
        for bone,bd in zip(inst['Bones'],skin['Bone List']):
            matrix=world[i]@skintr@(invroot@world[bone])@local_matrix(bd['Skin Transform'])
            normalmatrix=np.linalg.pinv(matrix[:3,:3]).T
            for v in bd.get('Vertex Weights',[]):
                index=v['Index'];weight=v['Weight'];positions[index]+=weight*(matrix[:3,:3]@vertices[index]+matrix[:3,3]);outnorm[index]+=weight*(normalmatrix@normals[index]);weights[index]+=weight
        if np.max(np.abs(weights-1))>.005:raise ValueError('Incomplete skin weights')
        lengths=np.linalg.norm(outnorm,axis=1);outnorm[lengths<1e-12]=[0,0,1]
        outnorm/=np.maximum(1e-12,np.linalg.norm(outnorm,axis=1))[:,None]
        return positions@AXIS.T,outnorm@AXIS.T
    m=world[i];positions=vertices@m[:3,:3].T+m[:3,3];normalmatrix=np.linalg.pinv(m[:3,:3]);normalmatrix=np.eye(3) if np.linalg.norm(normalmatrix)<1e-12 else normalmatrix
    normals=normals@normalmatrix;normals/=np.maximum(1e-12,np.linalg.norm(normals,axis=1))[:,None]
    return positions@AXIS.T,normals@AXIS.T

def convert(raw,entry,archive,texture_archive,kind='building',pose=None,animations=None,lod='near',normalization=None,omit_static_animations=False):
    reader=Reader(raw).parse();world,props,selected,lods,culled=scene(reader,pose,lod)
    if animations:
        pose,_=sample_pose(animations['idle']['reader'],reader.objects,0)
        world,props,selected,lods,culled=scene(reader,pose,lod)
    parents={child:o['_id'] for o in reader.objects for child in o.get('Children',[])}
    def path(i):
        p=[]
        while i>=0:p.append(reader.objects[i].get('Name',''));i=parents.get(i,-1)
        return '/'.join(reversed(p)).casefold()
    parts=[];allpos=[];sources=[]
    for i in selected:
        o=reader.objects[i];data=reader.objects[o['Data']];vertices=np.array([vec(v) for v in data.get('Vertices',[])])
        if len(vertices)==0:continue
        p=props[i];positions,normals=mesh_pose(reader,i,world)
        tris=triangles(data)
        if tris.size and int(tris.max())>=len(vertices):raise ValueError('Invalid triangle index')
        texture=p.get('NiTexturingProperty',{});desc=texture.get('Base Texture',{});source='';url=None;tm=None
        if desc.get('Source',-1)>=0:
            source=reader.objects[desc['Source']].get('File Name','')
            if source:
                tm=extract_texture(source,OUT/'model-textures',texture_archive);url=tm['url'];sources.append(tm)
        uvset=desc.get('UV Set',0);uvdata=data.get('UV Sets',[])
        uv=np.array([[v['u'],v['v']] for v in uvdata[uvset]]) if uvset<len(uvdata) else np.zeros((len(vertices),2))
        material=p.get('NiMaterialProperty',{});color=vec(material.get('Diffuse Color',dict(r=1,g=1,b=1)),'rgb').tolist()+[material.get('Alpha',1.0)]
        alpha=p.get('NiAlphaProperty',{})
        part={'name':o.get('Name',''),'sourceNode':i,'positions':positions,'normals':flat(normals),'uvs':flat(uv),'indices':flat(tris),'textureSource':source,'textureUrl':url,'color':color,
              'alpha':{'flags':alpha.get('Flags',0),'threshold':alpha.get('Threshold',0)},'textureApplyMode':texture.get('Apply Mode',2),'vertexColorMode':p.get('NiVertexColorProperty',{}),
              'skinned':o.get('Skin Instance',-1)>=0}
        part.update(native_material_fields(p))
        if kind=='human' and 'tool_' in path(i):
            matches={'tool_wood_axe':[2,6],'tool_mine_pick':[3],'tool_farm_bag':[4],'tool_farm_bowl':[4],'tool_build_hammer':[5]}
            part['activities']=next((v for k,v in matches.items() if k in path(i)),[])
        if 'Vertex Colors' in data:part['colors']=flat([vec(v,'rgba') for v in data['Vertex Colors']])
        parts.append(part)
        if kind!='human' or part['skinned']:allpos.extend(positions)
    if not allpos:raise ValueError('No visible triangle geometry')
    allpos=np.array(allpos);lo=allpos.min(axis=0);hi=allpos.max(axis=0)
    # Preserve the real local proportions; only unit conversion and ground centering.
    scale=(.85/(hi[1]-lo[1])) if kind=='human' else (1.0/max(hi[0]-lo[0],hi[2]-lo[2]))
    offset=np.array([-(lo[0]+hi[0])*.5,-lo[1],-(lo[2]+hi[2])*.5])
    if normalization:
        scale=normalization['scale'];offset=np.array(normalization['offset'])
    for p in parts:
        p['positions']=flat((p['positions']+offset)*scale);p['indices']=[int(v) for v in p['indices']]
        if animations:p['animations']={}
    animation_sources=[]
    for name,animation in (animations or {}).items():
        poses,duration=sample_pose(animation['reader'],reader.objects,0);frames=1 if duration<=1e-8 else int(animation.get('samples',8))
        if not 1<=frames<=64:raise ValueError('Animation sample count must fit the bounded morph exporter')
        animation_sources.append({'name':name,'entry':animation['entry'],'sha256':animation['sha256'],'duration':duration,'static':duration<=1e-8,'frames':frames})
        for p in parts:p['animations'][name]={'duration':duration,'static':duration<=1e-8,'frames':[]}
        for frame in range(frames):
            poses,_=sample_pose(animation['reader'],reader.objects,duration*frame/frames);w,*_=scene(reader,poses,lod)
            for p in parts:
                positions,normals=mesh_pose(reader,p['sourceNode'],w)
                p['animations'][name]['frames'].append({'positions':flat((positions+offset)*scale),'normals':flat(normals)})
    # Rigid aircraft hulls need no duplicate vertex-texture layers. Omit a
    # part only when every exported clip/sample equals its base pose exactly;
    # parts with a different constant state pose still retain those states.
    omitted=[]
    if omit_static_animations:
        for part in parts:
            for clip in part.get('animations',{}).values():
                samples=clip['frames']
                if len(samples)>1 and all(frame==samples[0] for frame in samples[1:]):
                    clip['constantPose']=True;clip['sampledFrames']=len(samples);clip['frames']=samples[:1]
            if part.get('animations') and all(frame['positions']==part['positions'] and frame['normals']==part['normals'] for clip in part['animations'].values() for frame in clip['frames']):
                part.pop('animations');omitted.append(part['sourceNode'])
    result={'format':'ee2-original-mesh-v1','conversion':{'version':CONVERTER_VERSION,'skinning':'geometryWorld * skinGlobal * inverse(skeletonRootWorld) * boneWorld * boneBindOffset'},'source':{'archive':archive.name,'entry':entry,'sha256':hashlib.sha256(raw).hexdigest()},'sourceBounds':{'min':flat(lo),'max':flat(hi)},
      'normalization':dict(normalization) if normalization else {'scale':scale,'offset':flat(offset),'axis':'x,z,-y','kind':kind},'parts':parts,'lodSelection':lods,'lodPolicy':'furthest-native-range' if lod=='far' else 'nearest-native-range','culledNodes':culled,
      'sourceTextures':list({x['url']:x for x in sources}.values()),'animationSources':animation_sources,'limitations':['Original geometry/UVs retained. Morph animation samples, not every source interpolation mode or animation event.'] if animations else ['Static original scene pose; source geometry and UVs retained.']}
    if omit_static_animations:
        result['animationSampling']={'omitStaticParts':True,'omittedSourceNodes':omitted,'policy':'48 uniform original-duration samples for fast native propellers/rotors; unchanged complete parts remain static.','interpolationLimit':'Existing source-key interpolation is linear/slerp; native quadratic tangents and animation events are not reproduced.'}
    return result

INITIAL=[(0,0,'bld_citycenter1_we','building'),(1,0,'bld_house1_we','building'),(3,0,'bld_barracks1','building'),
 (4,0,'bld_mill1','building'),(5,0,'bld_dock1','building'),(7,0,'bld_outpost1','building'),(47,0,'bld_university1','building'),(48,0,'bld_mill1','building'),
 (49,0,'bld_stable2','building'),(50,0,'bld_market2','building'),(51,0,'bld_temple1_we','building'),(52,0,'bld_warehouse1','building'),
 (100,0,'tre_oak1','prop'),(101,0,'tre_pine','prop'),(102,0,'res_iron','prop'),(103,0,'res_foragepatch','prop'),(8,0,'LSF1_citizen_m_we','human'),
 (53,0,'bld_oilrig','building'),(54,0,'res_uranium_mine','building'),(55,13,'bld_herc_factory','building'),(56,0,'bld_fort2_we','building'),
 (104,0,'res_stone','prop'),(105,0,'res_gold','prop'),(106,0,'res_tin','prop'),(107,0,'res_saltpeter','prop'),
 (108,0,'res_oil_1','prop'),(109,0,'res_uranium_1','prop'),(110,0,'res_rock_1','prop')]
BUILDING_STAGES={
 0:[(4,'bld_citycenter4_we'),(7,'bld_citycenter7_we'),(10,'bld_citycenter10_we'),(13,'bld_citycenter13_we')],
 1:[(3,'bld_house3_we'),(5,'bld_house5_we'),(7,'bld_house7_we'),(9,'bld_house9_we'),(11,'bld_house11_we'),(13,'bld_house13')],
 3:[(e,f'bld_barracks{e}') for e in [4,7,10,13]],
 4:[(e,f'bld_mill{e}') for e in [5,9,12,15]],5:[(e,f'bld_dock{e}') for e in [4,7,10,13]],
 6:[(11,'bld_airport11'),(13,'bld_airport13')],7:[(e,f'bld_outpost{e}') for e in [4,7,10,13]],
 47:[(e,f'bld_university{e}') for e in [4,7,10,13]],48:[(e,f'bld_mill{e}') for e in [5,9,12,15]],
 49:[(e,f'bld_stable{e}') for e in [5,8,11,14]],50:[(e,f'bld_market{e}') for e in [5,8,11,14]],
 51:[(e,f'bld_temple{e}_we' if e!=13 else 'bld_temple13') for e in [4,7,10,13]],52:[(e,f'bld_warehouse{e}') for e in [4,7,10,13]],
 56:[(e,f'bld_fort{e}_we' if e!=14 else 'bld_fort14') for e in [4,6,8,10,12,14]],
}
INITIAL.extend((id,epoch-1,name,'building') for id,stages in BUILDING_STAGES.items() for epoch,name in stages)
INITIAL.extend([(8,3,'lsf4_citizen_m_we','human'),(8,6,'lsf7_citizen_m_we','human'),(8,9,'lsf10_citizen_m_we','human'),(8,12,'lsf13_citizen_m','human')])
INITIAL.extend((id,0,name,'human') for id,name in [(9,'lhi1_maceman_we'),(10,'lli1_bowman_we'),(11,'lhi2_spearman_we'),(12,'lli3_compositebowman_we'),(13,'lhi3_swordsman_we'),(14,'lli5_crossbowman_we'),(15,'lhi7_musketeer_we'),(16,'lhi11_rifleman_we'),(17,'lhi13_assaultrifleman_we')])
INITIAL.extend((id,0,name,'vehicle') for id,name in [(23,'llm11_guntank'),(24,'LHA10_siegegun'),(25,'lla11_aahalftrack'),(26,'lhm13_mainbattletank'),(27,'lla13_mrls'),(28,'lla15_paladin_defense'),(30,'NG8_Carrack'),(31,'NF10_Frigate'),(32,'nd11_destroyer'),(33,'NB11_Battleship'),(34,'ns11_submarine'),(35,'nb13_missilecruiser'),(36,'ns13_nuclearsubmarine'),(37,'af11_biplane'),(38,'ab11_TwinEngineBomber'),(39,'AF13_JetFighter'),(40,'ab14_stealthbomber'),(41,'lhm13_gunhelicopter'),(43,'llm14_mech'),(44,'af15_Atmospheric'),(45,'ns15_manta'),(46,'lhm15_myrmidontank')])
INITIAL.extend((id,0,name,'vehicle') for id,name in [(18,'lhm2_mountedinfantry_we'),(19,'lhm4_armoredcavalry_we'),(20,'lha6_trebuchet'),(21,'llm10_dragoon_we'),(22,'lla9_fieldgun'),(29,'nwg1_galley'),(42,'nnc6_cargoboat')])
HUMAN_UNITS={8:'citizen',9:'maceman',10:'bowman',11:'spear',12:'compositebowman',13:'sword',14:'crossbowman',15:'musketeer',16:'rifleman',17:'assaultrifleman',18:'mountedinfantry',19:'armoredcavalry',21:'dragoon'}
ROLE_SOURCES={14:('light_infantry','LightInfantry_05'),15:('heavy_infantry1','HeavyInfantry1_07'),16:('heavy_infantry1','HeavyInfantry1_11'),17:('heavy_infantry1','HeavyInfantry1_13'),18:('heavy_mounted1','HeavyMounted1_02'),19:('heavy_mounted1','HeavyMounted1_04'),20:('heavy_artillery','HeavyArtillery_06'),21:('light_mounted1','LightMounted1_10'),22:('light_artillery1','LightArtillery1_09'),23:('light_mounted1','LightMounted1_11'),29:('war_galley','WarGalley_01'),42:('cargo_ship','CargoShip')}
DEF_NAMES=re.findall(r'^\s+(?:unit|bld)\("([^"]+)"',(ROOT/'src/defs.rs').read_text(encoding='utf8'),re.M)

# Explicit base-database visual links, not guesses from similar asset names.
RESOURCE_VISUALS={
 53:('OilRig','UnitModel','OilRig',None,10),
 54:('UraniumMine','UnitModel','UraniumMine',None,13),
 102:('ResourceIron','StateDatabase','ResourceIronStatesRevealed','Iron',4),
 103:('ResourceForagePatch','StateDatabase','ForagePatchStates','Food',1),
 104:('ResourceStone','StateDatabase','StoneResourceStates','Stone',1),
 105:('ResourceGold','StateDatabase','GoldResourceStates','Gold',1),
 106:('ResourceTin','StateDatabase','TinResourceStates','Tin',1),
 107:('ResourceSaltPeter','StateDatabase','ResourceSaltPeterStatesRevealed','Saltpeter',7),
 108:('ResourceOil','UnitModel','ResourceOilRevealed','Oil',10),
 109:('ResourceUranium','UnitModel','ResourceUraniumRevealed','Uranium',13),
 110:('ResourceDummy','UnitModel','ResourceDummy',None,None),
}

def resource_visual_bindings(installation):
    database=installation/'zips/db.zip';headers=installation/'zips/hdrs.zip'
    with zipfile.ZipFile(database) as z:raw=z.read('db/Units/resources.ddf')
    text=raw.decode('cp1252').replace('\r','');clean=without_comments(text)
    with zipfile.ZipFile(headers) as z:header=z.read('DbHdrs/DbResourceDefs.h')
    enums={};value=-1
    for name,explicit in re.findall(r'\b(kResourceType_\w+)\s*(?:=\s*(-?\d+))?\s*,',without_comments(header.decode('cp1252'))):
        value=int(explicit) if explicit else value+1;enums[name]=value
    result={}
    for id,(unit,kind,visual,resource,revealed) in RESOURCE_VISUALS.items():
        definition=block(clean,kind,visual);unit_definition=block(clean,'UnitType',unit)
        if not definition or not unit_definition:raise ValueError(f'Missing original resource definition {unit}/{visual}')
        models=re.findall(r'\b(res_[\w]+|bld_[\w]+)\.nif\b',definition,re.I)
        if not models:raise ValueError(f'No original model declared by {visual}')
        if resource and not re.search(r'\btype\s*=\s*kResourceType_'+resource+r'\b',unit_definition):raise ValueError(f'Resource ability mismatch {unit}')
        match=re.search(r'\b'+re.escape(kind)+r'\s+'+re.escape(visual)+r'\s*\{',text)
        workers=re.search(r'\bmaxNumOfWorkers\s*=\s*(\d+)',unit_definition)
        result[id]={
          'bindingType':'source-database-visual',
          'database':{'archive':'db.zip','entry':'db/Units/resources.ddf','sha256':hashlib.sha256(raw).hexdigest()},
          'unitType':unit,'visualDefinition':visual,'visualDefinitionKind':kind,
          'visualDefinitionLine':text.count('\n',0,match.start())+1,
          'declaredModelChoices':[name+'.nif' for name in models],
          'selection':'First declared original model; alternate native appearances are not synthesized.',
          'revealedOriginalEpoch':revealed,
          'scope':'Source visual and resource identity only; simulation availability and harvesting rules are implemented separately.'}
        if resource:
            enum='kResourceType_'+resource
            result[id].update(resourceType=resource,originalResourceId=enums[enum],
              resourceEnumSource={'archive':'hdrs.zip','entry':'DbHdrs/DbResourceDefs.h','sha256':hashlib.sha256(header).hexdigest(),'symbol':enum},
              maxDirectWorkers=int(workers[1]) if workers else None)
        elif id==110:result[id]['scope']='Actual unknown-resource rock appearance, with no resource identity or invented material.'
    return result

def building_profile_binding(installation,unit,epoch,definition_path):
    with zipfile.ZipFile(installation/'zips/db.zip') as z:
        csv_path='db/TechTree/upgrade_unittypes.csv';csv_raw=z.read(csv_path)
        row=next(row for row in source_rows(z,csv_path) if row['UNIT']==unit and int(row['EPOCH'])==epoch)
        raw=z.read(definition_path)
    text=raw.decode('cp1252').replace('\r','');visual=row['VISUAL'];definition=block(without_comments(text),'UnitModel',visual)
    models=re.findall(r'\b([\w]+)\.nif\b',definition,re.I)
    if not models:raise ValueError(f'No original model declared by {unit}/{visual}')
    declaration=re.search(r'\bUnitModel\s+'+re.escape(visual)+r'\s*\{',text)
    return {'bindingType':'source-database-visual','unitType':unit,'originalEpoch':epoch,
      'database':{'archive':'db.zip','entry':definition_path,'sha256':hashlib.sha256(raw).hexdigest()},
      'upgradeSource':{'archive':'db.zip','entry':csv_path,'sha256':hashlib.sha256(csv_raw).hexdigest(),'line':int(row['_source'].rsplit(':',1)[1])},
      'visualDefinition':visual,'visualDefinitionLine':text.count('\n',0,declaration.start())+1,
      'declaredModelChoices':[name+'.nif' for name in models],
      'scope':'Exact original building visual and epoch identity. No additional factory family or intermediate appearance was invented.'}

AIRCRAFT_VISUALS={
 37:('Fighter',11,'db/Units/fighter.ddf'),
 38:('Bomber',11,'db/Units/bomber.ddf'),
 41:('HeavyMounted2',13,'db/Units/heavy_mounted2.ddf'),
}
def aircraft_animation_binding(installation,id,model_name,graphics,specification=None,samples=48,clip_states=None):
    unit,epoch,path=specification or AIRCRAFT_VISUALS[id]
    binding=building_profile_binding(installation,unit,epoch,path)
    binding['scope']='Exact original aircraft model and declared Idle/Walk/Attack1 KF states, using the explicit base db.zip profile. No alternate unit or patch precedence inferred.'
    if model_name.casefold()+'.nif' not in [n.casefold() for n in binding['declaredModelChoices']]:raise ValueError('Aircraft model not declared by its source visual')
    with zipfile.ZipFile(installation/'zips/db.zip') as z:raw=z.read(path)
    definition=block(without_comments(raw.decode('cp1252').replace('\r','')),'UnitModel',binding['visualDefinition'])
    declarations={state.casefold():name for state,name in re.findall(r'\bStateName\s*=\s*(\w+)\s+AnimName\s*=\s*([\w.]+)',definition,re.I)}
    names={pathlib.PurePosixPath(n).name.casefold():n for n in graphics.namelist() if n.casefold().endswith('.kf')}
    animations={};states=[]
    for clip,state in (clip_states or [('idle','Idle'),('walk','Walk'),('combat','Attack1')]):
        declared=declarations.get(state.casefold())
        if not declared or declared.casefold() not in names:raise ValueError(f'Missing declared aircraft animation {state}/{declared}')
        entry=names[declared.casefold()];data=graphics.read(entry);digest=hashlib.sha256(data).hexdigest()
        animations[clip]={'reader':Reader(data).parse(),'entry':entry,'sha256':digest,'samples':samples}
        unsupported=sorted({animation.objects[cb['Controller']]['_type'] for sequence in animations[clip]['reader'].objects for cb in sequence.get('Controlled Blocks',[]) for animation in [animations[clip]['reader']] if animation.objects[cb['Controller']]['_type']!='NiKeyframeController'})
        states.append({'clip':clip,'state':state,'declaredAnimName':declared,'entry':entry,'sha256':digest,'unsupportedControllers':unsupported})
    binding['animationStates']=states
    return animations,binding

NATIVE_GROUND_MOTION={43:('LightMounted3',14,'db/Units/light_mounted3.ddf'),46:('HeavyTank',15,'db/Units/heavy_mounted1.ddf')}

def bind_epoch_visuals(manifest,installation):
    """Reuse geometry only for an identical audited native class/visual pair.

    This adds renderer dates, never gameplay numbers or backward era bindings.
    Re-running the geometry exporter deterministically rebuilds these aliases.
    """
    import copy
    from import_ee2_units import Database,model_stem,scalar
    audit_path=ROOT/'docs/ee2-units.json'
    if not audit_path.exists():return manifest
    unit_audit=json.loads(audit_path.read_text(encoding='utf8'))
    bases=[r for r in manifest['models'] if 'progressionBinding' not in r]
    manifest['models']=bases;native_rows=unit_audit['rows'];anchors={};available={}
    icon_path=OUT/'textures-manifest.json';icons=json.loads(icon_path.read_text(encoding='utf8')) if icon_path.exists() else {}
    db=Database(installation/'zips/db.zip')
    try:
        for native in native_rows:
            identifier=native['id'];anchors[identifier]=min(anchors.get(identifier,99),native['epoch'])
            if not native['model_verified']:continue
            visual=db.block('UnitModel',native['source_visual']);filename=scalar(visual['body'],'DefaultModel') if visual else None
            if not filename:continue
            candidates=[r for r in bases if r['id']==identifier and r.get('verified') and model_stem(r['source']['entry'])==model_stem(filename)]
            if not candidates:continue
            row=min(candidates,key=lambda r:abs(r['epoch']-native['epoch']))
            key=(native['source_unit'],native['source_visual'])
            # Prefer the first strict existing role; metadata always identifies it.
            available.setdefault(key,(row,native,filename,db.block_ref(visual)))
        generated=[];icon_bindings=[]
        for native in unit_audit['class_rows']:
            identifier=native['id'];key=(native['source_unit'],native['source_visual'])
            if native['model_verified'] or native['epoch']<=anchors.get(identifier,99) or key not in available:continue
            if any(r['id']==identifier and r['epoch']==native['epoch'] for r in bases):continue
            reference,strict,filename,visual_source=available[key]
            # Freshly verify the class row's CSV and DDF provenance before reuse.
            upgrade=native['upgrade_source'];raw=db.zip.read(upgrade['entry'])
            if hashlib.sha256(raw).hexdigest()!=upgrade['sha256']:raise ValueError('Native upgrade audit source changed')
            visual=db.block('UnitModel',native['source_visual']);declared=scalar(visual['body'],'DefaultModel') if visual else None
            if not declared or model_stem(declared)!=model_stem(reference['source']['entry']):raise ValueError('Native visual does not declare the reused geometry')
            row=copy.deepcopy(reference);row['id']=identifier;row['epoch']=native['epoch'];row['nativeUntil']=native['until'];row.pop('roleBinding',None)
            original=next(r for r in bases if r['id']==identifier);row['definitionName']=original.get('definitionName',str(identifier))
            proof={'type':'identical-native-class-visual','sourceUnit':native['source_unit'],'sourceVisual':native['source_visual'],'epoch':native['epoch'],'until':native['until'],
                   'upgradeSource':upgrade,'visualSource':visual_source,'declaredDefaultModel':declared,'reusedFrom':{'id':reference['id'],'epoch':reference['epoch'],'url':reference['url']},
                   'scope':'Existing geometry reused only for this exact source class and visual. No unit rules changed. Missing intermediate visuals remain explicit renderer fallbacks; no backward-era alias is added.'}
            sprite=native.get('source_upgrade',{}).get('ICON','').strip('"')
            icon=icons.get('icons',{}).get(str(reference['id']));icon_binding=icons.get('iconBindings',{}).get(str(reference['id']),{})
            if sprite and icon and icon_binding.get('sprite','').casefold()==sprite.casefold() and (ROOT/'docs'/icon['url']).is_file():
                proof['icon']={'sprite':sprite,'resolvedSprite':icon_binding.get('resolvedSprite'),'url':icon['url'],'source_sha256':icon['source_sha256'],'png_sha256':icon['png_sha256'],'reusedFromDef':reference['id']}
                icon_bindings.append({'id':identifier,'epoch':native['epoch'],'until':native['until'],'sourceUnit':native['source_unit'],'sourceVisual':native['source_visual'],**proof['icon']})
            else:
                native_icon=icons.get('nativeSpriteIcons',{}).get(sprite.casefold())
                if native_icon and native_icon.get('sourceUnit')==native['source_unit'] and native_icon.get('sourceVisual')==native['source_visual'] and native_icon.get('sourceEpoch')==native['source_epoch'] and (ROOT/'docs'/native_icon['url']).is_file():
                    proof['icon']={k:native_icon[k] for k in ['sprite','resolvedSprite','url','source_sha256','png_sha256','sourceUnit','sourceVisual','sourceEpoch','unitUpgradeSource','visualSource']}
                    for key in ['archive_member','unitUpgradeSha256','visualSourceSha256','spriteSource','spriteSourceSha256','spriteRect','width','height','sourceTextureWidth','sourceTextureHeight']:
                        if key in native_icon:proof['icon'][key]=native_icon[key]
                    proof['icon']['sourceKind']='native-epoch-sprite'
                    icon_bindings.append({'id':identifier,'epoch':native['epoch'],'until':native['until'],**proof['icon']})
                else:proof['iconLimitation']='No exported icon with the identical native source class, visual and sprite was reused.'
            row['progressionBinding']=proof;generated.append(row)
        manifest['models'].extend(generated);manifest['epochIconBindings']=icon_bindings
        manifest['progressionScope']='Only later native class/visual pairs already present as verified geometry; no new gameplay, source stats or backward-era replacements.'
        print('Native epoch aliases:',len(generated),'with',len(icon_bindings),'existing source-matched icons')
        return manifest
    finally:db.close()

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--install',type=pathlib.Path,default=INSTALL);ap.add_argument('--only',help='Comma-separated simulation ids to update');ap.add_argument('--defer-farm',action='store_true',help='Import Farm composites after the native visual scale profile is generated');args=ap.parse_args();archive=args.install/'zips/graphics.zip'
    only=set(map(int,args.only.split(','))) if args.only else None
    resource_bindings=resource_visual_bindings(args.install)
    building_bindings={55:building_profile_binding(args.install,'HercFacility',14,'db/Units/HERC_Facility.ddf')}
    (OUT/'models').mkdir(parents=True,exist_ok=True);manifest={'format':'ee2-original-model-manifest-v1','models':[],'errors':[]}
    if only and (OUT/'models/manifest.json').exists():
        old=json.loads((OUT/'models/manifest.json').read_text(encoding='utf8'))
        manifest['models']=[row for row in old['models'] if row['id'] not in only]
        manifest['errors']=[row for row in old['errors'] if row['id'] not in only]
    with zipfile.ZipFile(archive) as z,zipfile.ZipFile(args.install/'zips/textures.zip') as tex:
        names={pathlib.PurePosixPath(n).stem.casefold():n for n in z.namelist() if n.lower().endswith('.nifcache')}
        for id,epoch,name,kind in INITIAL:
            if only and id not in only:continue
            try:
                member=names[name.casefold()];anims={};aircraft_binding=None;native_motion_binding=None
                if id in AIRCRAFT_VISUALS:anims,aircraft_binding=aircraft_animation_binding(args.install,id,name,z)
                if id in NATIVE_GROUND_MOTION:
                    states=[('idle','Idle'),('walk','Walk')]+([('combat','Attack1')] if id==43 else [])
                    anims,native_motion_binding=aircraft_animation_binding(args.install,id,name,z,NATIVE_GROUND_MOTION[id],24 if id==43 else 48,states)
                    native_motion_binding['scope']='Exact native model and explicitly declared KF states. No unbound attack file or UV-animation behavior inferred.'
                    if id==46:native_motion_binding['unboundAttack']='Native Attack1 and its parent have no AnimName; the similarly named cached attack KF was not used.'
                if id in HUMAN_UNITS:
                    actions={'idle':'idle01','walk':'walk01','wood':'tool_wood_axe','mine':'tool_mine_pickaxe','food':'tool_farm_forage','build':'tool_build_hammer','combat':'attack01'} if id==8 else {'idle':'idle01','walk':'walk01','combat':'attack01'}
                    if id in (13,16):actions['combat']='attack02'
                    for action,kfname in actions.items():
                        kf=next(n for n in z.namelist() if n.casefold()==f'graphics/humans/sh_{HUMAN_UNITS[id]}_{kfname}.kf');kfraw=z.read(kf)
                        anims[action]={'reader':Reader(kfraw).parse(),'entry':kf,'sha256':hashlib.sha256(kfraw).hexdigest()}
                if id in (20,22,24):
                    suffixes={'idle':'idle01','walk':'walk' if id==24 else 'walk01','combat':'attack01'}
                    if id in (20,22):suffixes={'idle':'idle','walk':'walk','combat':'attack'}
                    for action,suffix in suffixes.items():
                        kf=next(n for n in z.namelist() if n.casefold()==f'graphics/vehicles/{name}_{suffix}.kf'.casefold());kfraw=z.read(kf)
                        anims[action]={'reader':Reader(kfraw).parse(),'entry':kf,'sha256':hashlib.sha256(kfraw).hexdigest()}
                if id==29:
                    for action in ['idle','walk']:
                        kf=next(n for n in z.namelist() if pathlib.PurePosixPath(n).name.casefold()==f'nwg1_galley_{action}.kf');kfraw=z.read(kf)
                        anims[action]={'reader':Reader(kfraw).parse(),'entry':kf,'sha256':hashlib.sha256(kfraw).hexdigest()}
                if id==23:
                    for action in ['idle','walk']:
                        kf=next(n for n in z.namelist() if pathlib.PurePosixPath(n).name.casefold()==f'llm11_guntank_{action}.kf');kfraw=z.read(kf)
                        anims[action]={'reader':Reader(kfraw).parse(),'entry':kf,'sha256':hashlib.sha256(kfraw).hexdigest()}
                if id==53:
                    for action,suffix in [('idle','idle'),('mine','walk')]:
                        kf=next(n for n in z.namelist() if pathlib.PurePosixPath(n).name.casefold()==f'bld_oilrig_{suffix}.kf');kfraw=z.read(kf)
                        anims[action]={'reader':Reader(kfraw).parse(),'entry':kf,'sha256':hashlib.sha256(kfraw).hexdigest()}
                data=convert(z.read(member),member,archive,tex,kind,animations=anims or None,omit_static_animations=id in AIRCRAFT_VISUALS or id in NATIVE_GROUND_MOTION)
                if aircraft_binding:data['aircraftVisualBinding']=aircraft_binding
                if native_motion_binding:
                    data['nativeMotionBinding']=native_motion_binding
                    data['animationSampling']['policy']='Native-duration key samples with exact constant-pose/static-part deduplication.'
                    data['animationSampling']['sampleCounts']={a['name']:a['frames'] for a in data['animationSources']}
                    if id==46:data['limitations'].append('Original track UV scrolling uses NiTextureTransformController and is not exported by this geometry-only animation path.')
                binding=resource_bindings.get(id)
                if binding:
                    if name.casefold()+'.nif' not in [choice.casefold() for choice in binding['declaredModelChoices']]:raise ValueError(f'{name} is not declared by {binding["visualDefinition"]}')
                    data['resourceVisualBinding']=binding
                building_binding=building_profile_binding(args.install,'Fortress',epoch+1,'db/Units/fortress.ddf') if id==56 else building_bindings.get(id)
                if building_binding:
                    canonical=name.casefold().removesuffix('_we') if id==56 else name.casefold()
                    if canonical+'.nif' not in [choice.casefold() for choice in building_binding['declaredModelChoices']]:raise ValueError(f'{name} is not declared by {building_binding["visualDefinition"]}')
                    if id==56:
                        building_binding['resolvedCacheModel']=name+'.nifcache'
                        building_binding['cultureVariant']='western' if name.casefold().endswith('_we') else 'neutral'
                        building_binding['scope']='Declared original Fortress canonical model/epoch, using the explicit western cache variant where present. Canonical stem and regional suffix retained; native culture-selection/VFS ordering is not inferred.'
                        if epoch==13:
                            building_binding['unboundAnimationSources']=[{'archive':'graphics.zip','entry':n,'sha256':hashlib.sha256(z.read(n)).hexdigest(),
                                'reason':'Original KF exists, but neither this visual nor its inherited BuildingWithTurrets declaration supplies a matching AnimName; no state assignment guessed.'}
                                for n in z.namelist() if pathlib.PurePosixPath(n).name.casefold().startswith('bld_fort14_') and n.casefold().endswith('.kf')]
                    data['databaseVisualBinding']=building_binding
                url=f'local-original/models/{name}.json';dest=ROOT/'docs'/url;tmp=dest.with_suffix('.tmp');tmp.write_text(json.dumps(data,separators=(',',':')),encoding='utf8');tmp.replace(dest)
                row={'id':id,'epoch':epoch,'verified':True,'yawOffset':-math.pi/2 if kind in ('human','vehicle') else 0,'url':url,'source':data['source'],'triangles':sum(len(p['indices'])//3 for p in data['parts'])};manifest['models'].append(row)
                if binding:row['resourceVisualBinding']=binding
                if aircraft_binding:row['aircraftVisualBinding']=aircraft_binding
                if native_motion_binding:row['nativeMotionBinding']=native_motion_binding
                if building_binding:row['databaseVisualBinding']=building_binding
                if id in ROLE_SOURCES:
                    file,unitmodel=ROLE_SOURCES[id];row['roleBinding']={'source':f'db.zip!db/Units/{file}.ddf','unitModel':unitmodel,'scope':'Visual archetype binding to existing simulation definition; not a claim of identical rules.'}
                print('OK',id,name,len(data['parts']),'parts',row['triangles'],'triangles')
            except Exception as ex:
                manifest['errors'].append({'id':id,'name':name,'error':str(ex)});print('FAIL',name,type(ex).__name__,ex)
            dest=OUT/'models/manifest.json';tmp=dest.with_suffix('.tmp');tmp.write_text(json.dumps(manifest,indent=2),encoding='utf8');tmp.replace(dest)
    for row in manifest['models']:
        row.setdefault('yawOffset',-math.pi/2 if 8<=row['id']<=46 else 0)
        if row['id']<len(DEF_NAMES):row['definitionName']=DEF_NAMES[row['id']]
    manifest=bind_epoch_visuals(manifest,args.install)
    dest=OUT/'models/manifest.json';tmp=dest.with_suffix('.tmp');tmp.write_text(json.dumps(manifest,indent=2),encoding='utf8');tmp.replace(dest)
    if not args.defer_farm and (only is None or 2 in only):
        # Farm is a source CompositeModel of four independent FarmPlotModels,
        # not the old single seeding mesh or an unbound Farm8 epoch alias.
        from import_ee2_farm_composites import build
        build(ROOT/'docs',args.install)
        manifest=json.loads(dest.read_text(encoding='utf8'))
    print(len(manifest['models']),'models',len(manifest['errors']),'errors')
if __name__=='__main__':main()
