"""Import explicit classic-family original model progressions, isolated and typed.
Original installation read-only. No game/DLL execution. Local original media remain excluded from public source.
"""
from pathlib import Path,PurePosixPath,PureWindowsPath
import hashlib,json,math,re,struct,sys,time,zipfile
ROOT=Path(__file__).resolve().parents[1]
from up15_source_database import ActiveDatabase,default_manifest
from import_ee2_units import scalar,sections
from nif_reader import Reader
from extract_ee2_textures import decode_texcache
import extract_ee2_models as meshes
MANIFEST=default_manifest(ROOT);WORKSPACE=MANIFEST.parents[2]
DOCS=ROOT/'docs';OUT=DOCS/'local-original';BASE_DOCS=WORKSPACE/'outputs/OpenEpoch/docs'
INVENTORY=MANIFEST.parent/'protected-payload-inventory.json'
sha=lambda b:hashlib.sha256(b).hexdigest()
def member_time(d):
    y,m,day,h,minute,sec=d
    if m>4 or(m==4 and day>=5):h-=1
    return int(time.mktime((y,m,day,h,minute,sec,0,0,0)))&0xffffffff
class MediaSources:
    def __init__(self,extensions=None):
        m=json.loads(MANIFEST.read_text());inventory=json.loads(INVENTORY.read_text());self.install=Path(m['profile']['physicalRoot']);self.records={};self.basenames={};self.opened={};self.events=[]
        wanted={'.nifcache','.texcache','.kf'} if extensions is None else set(extensions)
        loose=sorted((p for p in self.install.rglob('*') if p.is_file() and p.suffix.lower() in wanted),key=lambda p:str(p).casefold())
        for p in loose:
            name=p.relative_to(self.install).as_posix();record={'path':name,'physical':str(p),'member':None,'initialTime':int(p.stat().st_mtime)&0xffffffff,'priority':-1}
            self.records[name.lower()]=record;self.basenames.setdefault(p.name.lower(),record)
        archives={a['marker']:a for a in inventory['archives']}
        for priority,order in enumerate(m['archiveOrder']):
            a=archives[order['marker']]
            for entry in a.get('members',[]):
                name=entry['path'].replace('\\','/');key=name.lower()
                if PurePosixPath(name).suffix.lower() not in wanted:continue
                stamp=member_time(entry['dateTime']);origin={'physical':str(self.install/a['payload']),'marker':a['marker'],'member':name,'priority':priority,'memberTime':stamp,'bytes':entry['bytes'],'crc32':entry['crc32']}
                if key not in self.records:
                    record={'path':name,'initialTime':stamp,**origin};self.records[key]=record;self.basenames.setdefault(PurePosixPath(name).name.lower(),record)
                elif self.records[key]['initialTime']<=stamp+1:
                    record=self.records[key];previous=dict(record);record.update(origin);self.events.append({'path':name,'previous':previous,'selected':dict(record)})
    def origin(self,name):
        key=name.replace('\\','/').lower();record=self.records.get(key) if '/'in key else self.basenames.get(key)
        if record is None:raise ValueError('Native media namespace has no '+name)
        return dict(record)
    def read(self,name):
        o=self.origin(name)
        if o['member'] is None:return Path(o['physical']).read_bytes()
        if o['physical'] not in self.opened:self.opened[o['physical']]=zipfile.ZipFile(o['physical'])
        data=self.opened[o['physical']].read(o['member']);assert len(data)==o['bytes'];return data
    def close(self):
        for z in self.opened.values():z.close()
def specs(db):
    selected=json.loads((ROOT/'docs/roster6-classic-selection.json').read_text())['bindings']
    upgrades=db.table('db/TechTree/upgrade_unittypes.csv');out=[]
    for binding in selected:
        rows=[r for r in upgrades if r['UNIT']==binding['source_unit'] and r['CIV']=='All']
        assert rows,'No exact CSV progression for '+binding['source_unit']
        seen=set()
        for row in rows:
            identity=(int(row['EPOCH']),row['VISUAL'])
            if identity in seen:continue
            seen.add(identity)
            name=row['VISUAL'];chain=[]
            while name:
                d=db.block('UnitModel',name);assert d,'Missing UnitModel '+name
                assert name not in [z['name'] for z in chain],'Visual parent cycle'
                chain.append(d);name=scalar(d['body'],'Parent')
            def prop(key):
                return next((v for z in chain if (v:=scalar(z['body'],key))is not None),None)
            model=prop('DefaultModel');assert model,'No inherited scalar DefaultModel'
            out.append((binding,int(row['EPOCH'])-1,row['VISUAL'],PureWindowsPath(model).stem,chain,model,prop('ScaleType')or'kScaleIdentity',prop('ForceScale')))
    return out

def run():
    sources=MediaSources();db=ActiveDatabase(MANIFEST);model_rows=[];scale_rows={};assets=[]
    original_out,original_texture=meshes.OUT,meshes.extract_texture
    def texture(source,output_dir,_archive=None):
        stem=PureWindowsPath(source.replace('/','\\')).stem;name=stem+'.texcache';origin=sources.origin(name);data=sources.read(name);image,meta=decode_texcache(data)
        output_dir=Path(output_dir);assert output_dir.resolve().is_relative_to(DOCS.resolve());output_dir.mkdir(parents=True,exist_ok=True);dest=output_dir/(stem.lower()+'.png');image.save(dest,optimize=True)
        info={'url':dest.relative_to(DOCS).as_posix(),'original_texture_path':source,'archive_member':origin['member'],'origin':origin,'source_sha256':sha(data),'png_sha256':sha(dest.read_bytes()),**meta};assets.append(info);return info
    meshes.OUT=OUT;meshes.extract_texture=texture
    try:
        for binding,epoch,visual,stem,chain,native_model,scale_type,force in specs(db):
            identifier=binding['id'];unit=binding['source_unit'];declaration=chain[0]
            name=stem+'.nifcache';origin=sources.origin(name);raw=sources.read(name);reader=Reader(raw).parse();anims={}
            # Only states explicitly declared in the actual active UP UnitModel.
            for body,_ in [(body,offset)for d in chain for body,offset in sections(d['body'],'States')]:
                for state in re.finditer(r'\{([^{}]*)\}',body):
                    state_name=scalar(state[1],'StateName');animation=scalar(state[1],'AnimName')
                    key={'Idle':'idle','Walk':'walk','Attack1':'combat'}.get(state_name)
                    if key and animation and key not in anims:
                        ar=sources.read(animation);anims[key]={'reader':Reader(ar).parse(),'entry':sources.origin(animation)['member'],'sha256':sha(ar),'samples':24}
            # States uses square brackets; match each StateName record without inventing filenames.
            for state_name,key in [('Idle','idle'),('Walk','walk'),('Attack1','combat')]:
                match=next((match for d in chain if (match:=re.search(r'StateName\s*=\s*'+state_name+r'\b[^}]*?AnimName\s*=\s*("[^"]*"|[\w.]+)',d['body'],re.I|re.S))),None)
                if match and key not in anims:
                    animation=match[1].strip('"');ar=sources.read(animation);anims[key]={'reader':Reader(ar).parse(),'entry':sources.origin(animation)['member'],'sha256':sha(ar),'samples':24}
            data=meshes.convert(raw,origin['member'],Path(origin['physical']),sources,binding['modelKind'],animations=anims or None)
            data['source']['origin']=origin;data['databaseVisualBinding']={'visualDefinition':visual,'sourceUnit':unit,'declaredModel':native_model,'source':db.block_ref(declaration),'scope':'Exact active UP default model and explicit KF states; no old BASE-model numeric alias.'}
            data['nativeSourceProfile']='ee2-roster6';data['sourceLookupScope']='Model cache and KF/texcache namespace uses proven archiveOrder/first-basename/fullpath timestamp policy; native cache-extension translation follows existing importer adapter.'
            dest=OUT/'models'/('roster6_'+stem.lower()+'.json');dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps(data,separators=(',',':'))+'\n')
            row={'assetKind':0,'id':identifier,'epoch':epoch,'sourceFamily':unit,'sourceEpoch':epoch+1,'sourceVisual':visual,'verified':True,'yawOffset':-math.pi/2,'url':dest.relative_to(DOCS).as_posix(),'source':data['source'],'triangles':sum(len(p['indices'])//3 for p in data['parts']),'databaseVisualBinding':data['databaseVisualBinding']};model_rows.append(row)
            defaults=json.loads((BASE_DOCS/'local-original/model-scales.json').read_text())
            f32=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]
            assert scale_type in defaults['classDefaults'];selected=defaults['classDefaults'][scale_type]['selectedValue'];conversion=defaults['unitConversion']['value'];scale=f32(f32(float(force) if force and float(force)>0 else selected)*conversion)
            scale_rows[f'0:{identifier}:{epoch}']={'url':row['url'],'sourceSha256':sha(raw),'sourceEntry':origin['member'],'verified':True,'visual':visual,'scaleType':scale_type,'forceScale':float(force) if force else None,'sourceScale':scale,'inheritance':[db.block_ref(d)for d in chain],'sourceBounds':data['sourceBounds'],'normalization':data['normalization'],'sourceOrigin':origin}
        base=json.loads((ROOT/'docs/local-original/models/manifest-up15.json').read_text(encoding='utf8'));base['models']=[r for r in base['models'] if r.get('assetKind')!=0 or r['id']<57]+model_rows;base['profile']='ee2-up15';base['nativeSourceManifestSha256']=sha(MANIFEST.read_bytes())
        (OUT/'models/manifest-roster6.json').write_text(json.dumps(base,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
        scales=json.loads((ROOT/'docs/local-original/model-scales-up15.json').read_text());scales['variants'].update(scale_rows);scales['profile']='ee2-up15';scales['scope']+=' Four additional classic source families use explicit typed identities and inherited active DDF ScaleType/ForceScale under the same native class conversion.'
        (OUT/'model-scales-roster6.json').write_text(json.dumps(scales,indent=2)+'\n')
        report={'format':'native-roster6-model-candidate-v1','original_install_readonly':True,'base_profile_manifests_modified':False,'source_manifest_sha256':sha(MANIFEST.read_bytes()),'models':model_rows,'scales':scale_rows,'textures':list({a['url']:a for a in assets}.values()),'overrideEvents':sources.events,'limitations':['Four exact classical families add local-only assets. This does not claim complete native abilities, production UI conditions or the full132family roster.','NIF child-name attachment/effect models and native animation event callbacks remain separate renderer work.']}
        (ROOT.parent/'EE2-Research/analysis/native-roster6-models.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'models':[(r['id'],r['source']['entry'],r['triangles']) for r in model_rows],'textures':len(report['textures']),'base_profile_manifests_modified':False}))
    finally:meshes.OUT=original_out;meshes.extract_texture=original_texture;sources.close();db.close()
if __name__=='__main__':run()
