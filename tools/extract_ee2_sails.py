"""Isolated source-data extraction of FightingSail's procedural cloth.

This is the native initial plane, not a replacement wind solver. It never adds
a wake or guessed sail to the later ironclad variant.
"""
from pathlib import Path, PureWindowsPath
import argparse,hashlib,json,re,struct,sys
import numpy as np

HERE=Path(__file__).resolve().parent
PROJECT=HERE.parent
ROOT=PROJECT.parents[1]
sys.path.insert(0,str(PROJECT/'tools'))
from up15_source_database import ActiveDatabase,default_manifest
from import_ee2_units import declarations,clean_comments,scalar
from import_ee2_up15_models import MediaSources
from nif_reader import Reader
from extract_ee2_models import local_matrix,AXIS,scene,mesh_pose
from extract_ee2_textures import decode_texcache

OUT=PROJECT/'docs/local-original/sails-native'
F=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]
sha=lambda x:hashlib.sha256(x).hexdigest()

def initial_grid(width,height,nx,ny):
    # 00659E40 substitutes5 only for zero, despite the stale header's6 comment.
    nx=nx or 5;ny=ny or 5
    if not(2<=nx<=255 and 2<=ny<=255 and nx*ny<=65535):raise ValueError('Native uint16 grid bounds')
    dx=F(F(width)/(nx-1));dy=F(F(height)/(ny-1))
    positions=[];uvs=[];indices=[]
    for y in range(ny):
        for x in range(nx):
            positions.extend([F(dx*x),F(dy*y),0.0])
            uvs.extend([F(x/(nx-1)),F(y/(ny-1))])
    for y in range(ny-1):
        for x in range(nx-1):
            a=x+y*nx;b=a+1;c=a+nx
            indices.extend([a,b,c,b,c+1,c])
    return {'positions':positions,'uvs':uvs,'indices':indices,'nx':nx,'ny':ny}

def pins(sail,nx,ny):
    pin=[False]*(nx*ny)
    sides={'Top':list(range(nx)),'Bottom':list(range((ny-1)*nx,ny*nx)),
        'Left':[y*nx for y in range(ny)],'Right':[y*nx+nx-1 for y in range(ny)]}
    corners={'Top':[0,nx-1],'Bottom':[(ny-1)*nx,ny*nx-1],
        'Left':[0,(ny-1)*nx],'Right':[nx-1,ny*nx-1]}
    for side in ['Top','Left','Bottom','Right']:
        if sail.get('block'+side,False):
            for i in sides[side]:pin[i]=True
    # 00A9E830 replaces the specified side's prior pins with its endpoints.
    for side in ['Top','Bottom','Left','Right']:
        if sail.get('blockCorner'+side,False):
            for i in sides[side]:pin[i]=False
            for i in corners[side]:pin[i]=True
    return pin

def matrices(reader):
    world={};active=set()
    def walk(i,parent):
        if i<0:return
        if i in active:raise ValueError('Cyclic native NIF graph')
        active.add(i);obj=reader.objects[i]
        if 'Translation' not in obj:active.remove(i);return
        m=parent@local_matrix(obj);world[i]=m
        for child in obj.get('Children',[]):walk(child,m)
        active.remove(i)
    for root in reader.footer.get('Roots',[0]):walk(root,np.eye(4))
    return world

def run(output=None):
    global OUT
    OUT=Path(output).resolve() if output is not None else OUT
    OUT.mkdir(parents=True,exist_ok=True)
    db=ActiveDatabase(default_manifest(PROJECT))
    media=MediaSources();assets={};effects={};models=[]
    def texture(name):
        key=PureWindowsPath(name).stem
        origin=media.origin(key+'.texcache');raw=media.read(key+'.texcache')
        image,meta=decode_texcache(raw);target=OUT/'textures'/(key.lower()+'.png');target.parent.mkdir(exist_ok=True);image.save(target,optimize=True)
        url='local-original/sails-native/textures/'+target.name
        assets[url]={'url':url,'originalPath':name,'origin':origin,'sourceSha256':sha(raw),'pngSha256':sha(target.read_bytes()),'rgbaSha256':sha(image.tobytes()),**meta}
        return url
    for path in db.active_ddfs:
        text=clean_comments(db.read(path))
        for name,body,_,at in declarations(text,'SailEffect'):
            if name not in ['CaravelSail','ShipOfTheLineSail']:continue
            sails=[]
            for match in re.finditer(r'\{([^{}]+)\}',body):
                record={}
                for key,value in re.findall(r'\b(\w+)\s*=\s*([^\s;]+)',match[1]):
                    record[key]=value=='true' if value in ['true','false'] else float(value) if re.fullmatch(r'[-+\d.]+',value) else value
                if 'TextureName' not in record:continue
                record['textureUrl']=texture(record['TextureName']);record['grid']=initial_grid(record['FlagWidth'],record['FlagHeight'],int(record.get('NbrVertexWidth',0)),int(record.get('NbrVertexHeight',0)))
                record['pinned']=pins(record,record['grid']['nx'],record['grid']['ny']);sails.append(record)
            effects[name]={'name':name,'source':db.ref(path,text.count('\n',0,at)+1),'sails':sails}
    damage=[texture('fx_flag_damage'+str(i).zfill(2)+'.tga')for i in range(9)]
    specs=[(5,7,'FightingSail_06','CaravelSail'),(7,9,'FightingSail_08','ShipOfTheLineSail'),(9,15,'FightingSail_10',None)]
    for epoch,until,visual,effect in specs:
        decl=db.block('UnitModel',visual);model_name=scalar(decl['body'],'DefaultModel');cache=PureWindowsPath(model_name).stem+'.nifcache';raw=media.read(cache);reader=Reader(raw).parse();world=matrices(reader)
        tags=[]
        # 0066E000/0066E0E0 append matches before recursively visiting each
        # NiNode child in its authored list order. Caravel is4,5,6,1,2,3.
        for i in world:
            obj=reader.objects[i]
            name=obj.get('Name','')
            if re.fullmatch(r'tag_sail_\d+',name):tags.append({'name':name,'node':i,'local':local_matrix(obj).tolist(),'world':world[i].tolist()})
        expected=len(effects[effect]['sails']) if effect else 0
        if len(tags)!=expected:raise ValueError((visual,'Exact sequential sail tag cardinality differs',len(tags),expected))
        # Match the parent mesh's existing export normalization; attachments are
        # source-unit matrices first, then the common browser basis/normalization.
        w,_,selected,_,_=scene(reader);allpos=np.concatenate([mesh_pose(reader,i,w)[0]for i in selected]);lo=allpos.min(axis=0);hi=allpos.max(axis=0)
        normal={'scale':1/float(max(hi[0]-lo[0],hi[2]-lo[2])),'offset':[-float((lo[0]+hi[0])*.5),-float(lo[1]),-float((lo[2]+hi[2])*.5)],'axis':'x,z,-y'}
        parts=[]
        if effect:
            children=re.search(r'\bChildNames\s*=\s*\[([^]]*)\]',decl['body'])[1].split()
            if effect not in children:raise ValueError('Source model does not declare effect')
            for tag,sail in zip(tags,effects[effect]['sails']):
                p=np.array(sail['grid']['positions']).reshape(-1,3);m=np.array(tag['world']);native=p@m[:3,:3].T+m[:3,3];converted=native@AXIS.T;norm=(converted+np.array(normal['offset']))*normal['scale']
                unit_normal=np.linalg.inv(m[:3,:3]).T@np.array([0.,0.,1.]);unit_normal=AXIS@unit_normal;unit_normal/=np.linalg.norm(unit_normal)
                parts.append({'name':effect+':'+tag['name'],'sourceTag':tag,'positions':norm.reshape(-1).tolist(),'normals':np.tile(unit_normal,(len(norm),1)).reshape(-1).tolist(),'sourcePlanePositions':sail['grid']['positions'],'uvs':sail['grid']['uvs'],'indices':sail['grid']['indices'],'textureUrl':sail['textureUrl'],'damageMaskUrls':damage,'pinned':sail['pinned'],'status':'native-initial-plane-only'})
        model={'assetKind':0,'id':58,'epoch':epoch,'until':until,'sourceFamily':'FightingSail','sourceVisual':visual,'effect':effect,'source':{'origin':media.origin(cache),'sha256':sha(raw),'db':db.block_ref(decl)},'sourceBounds':{'min':lo.tolist(),'max':hi.tolist()},'normalization':normal,'tags':tags,'parts':parts,'windSolver':'not implemented; no invented animation'}
        dest=OUT/('fighting-sail-'+str(epoch)+'-initial.json');dest.write_text(json.dumps(model,separators=(',',':'))+'\n');models.append({'url':'local-original/sails-native/'+dest.name,'sha256':sha(dest.read_bytes()),'epoch':epoch,'until':until,'effect':effect,'sails':len(parts),'source':model['source']})
    result={'format':'ee2-native-sail-initial-composites-v1','sourceManifestSha256':sha(db.reader.manifest_path.read_bytes()),'engineSha256':db.reader.manifest['engineSha256'],'models':models,'effects':effects,'assets':assets,
        'sourceBindings':{'effectFactory':'00658810 maps SailEffect→VisualSail via00659BA0','tagCollection':'00659FF0→0066E000/0066E0E0: depth-first authored NiNode child order, match via strstr(tag_sail_). Original node world transform copied in00659160. Caravel order4,5,6,1,2,3 is retained.','initialGrid':'006195D0 row-major XY-plane with original dimensions, UV and two triangles per cell','gridDefault':'00659E40 uses5 for omitted/zero count; DBVisualDefs.h6 comment is stale','edgePins':'00A9E740 then00A9E830, exact edge/corner replacement','planeOn':'Present as DDF bool but00659E40 ends with00A9E730, a bareRET in this engine. No invented plane deformation.','shader':'00659E40 passes FlagShader, not ClothShader. Source NSF uses Base*Diffuse*4 and Dark multipliers, source alpha; damage mask selected in00659A60.'},
        'limits':['Only original initial cloth planes and source attachments are emitted. Native cloth mass/spring/random-wind dynamics are not activated or approximated.','Tag matches retain native depth-first child traversal; importer rejects missing/cardinality mismatches. The existing parent model bounding volume must be extended before a future effect activation.','Native boat wakes, muzzle smoke and explosion children remain separate source classes; no generic wake or particle is added.','Parent hull/masts and integral meshes are unchanged; only additive candidate data exist outside active docs.']}
    (OUT/'manifest.json').write_text(json.dumps(result,indent=2)+'\n');media.close();db.close()
    print(json.dumps({'models':len(models),'sails':sum(row['sails']for row in models),'textures':len(assets),'vertices':sum(len(p['grid']['positions'])//3 for e in effects.values()for p in e['sails']),'manifestSha256':sha((OUT/'manifest.json').read_bytes())}))
    return result

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path,default=OUT,help='Output directory for initial composites and decoded PNGs; prepared effective source manifest is required.')
    args=parser.parse_args()
    run(args.out)
