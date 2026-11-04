"""Explicit native00541940 CSV5 + active CFG DDF database for the UP candidate."""
from pathlib import Path
import hashlib,re
from resolved_archive_reader import ResolvedArchiveReader
from import_ee2_units import Database as BaseDatabase,clean_comments,declarations,sections,numeric_assignments

NATIVE_TABLES={
 'db/techtree/upgrade_unittypes.csv':'db/techtree/upgrade_unittype5.csv',
 'db/techtree/dbtechtreenode.csv':'db/techtree/dbtechtreenod5.csv',
 'db/techtree/upgrade_factorset.csv':'db/techtree/upgrade_factorse5.csv',
 'db/simulation/dbcombat.csv':'db/simulation/dbcomba5.csv',
 'db/simulation/dbcombat_unittypeadjust.csv':'db/simulation/dbcombat_unittypeadjus5.csv',
 'db/simulation/dbresourcelevel.csv':'db/simulation/dbresourcelevel5.csv',
}

def default_manifest(root):
    """Resolve only known local extraction locations; never guess archive order."""
    candidates=[root.parents[1]/'up-resolved-sources/effective-db-sources.json',
                root.parents[1]/'work/up-resolved-sources/effective-db-sources.json']
    found=[path for path in candidates if path.is_file()]
    if len(found)!=1:raise ValueError('Exactly one prepared effective source manifest required: '+str(candidates))
    return found[0]

class ArchiveView:
    def __init__(self,db):self.db=db
    def namelist(self):
        return [p for p in self.db.reader.namelist() if not p.lower().endswith('.ddf')]+self.db.active_ddfs
    def read(self,name):return self.db.raw(name)
    def close(self):self.db.reader.close()
    def __enter__(self):return self
    def __exit__(self,*_):self.close()

class ActiveDatabase(BaseDatabase):
    def __init__(self,manifest):
        self.archive=Path(manifest).resolve();self.reader=ResolvedArchiveReader(self.archive)
        self.paths={p.lower():p for p in self.reader.namelist()}
        for old,new in NATIVE_TABLES.items():
            if new not in self.paths:raise ValueError('Required native CSV5 missing: '+new)
            self.paths[old]=self.paths[new]
        self.files={};self.blocks={};self.resolved={};self.active_ddfs=[];self.active_cfgs=[];self.epoch_rebindings=[]
        self.zip=ArchiveView(self)
        # Both lists are directly named by native00541940. An active DDF may be
        # BASE data when it is explicitly named by CFG5; no inactive BASE unit
        # list or other-version DDF is added as a fallback.
        for cfg in ['db/dbunitfile5.cfg','db/techtree/dbepochupgrade5.cfg']:
            text=clean_comments(self.read(cfg));origin=self.reader.origin(cfg)
            self.active_cfgs.append({'path':cfg,'origin':origin})
            for line_number,line in enumerate(text.splitlines(),1):
                name=line.strip().strip('"').replace('\\','/')
                if not name:continue
                if not name.lower().endswith('.ddf'):raise ValueError((cfg,line_number,name))
                actual=self.paths.get(name.lower())
                if actual is None:raise ValueError('Active CFG DDF unavailable: '+name)
                if actual not in self.active_ddfs:self.active_ddfs.append(actual)
        self.zip=ArchiveView(self)
        for path in self.active_ddfs:
            text=clean_comments(self.read(path))
            for kind in ['UnitType','UnitModel','UpgradeSet','UpgradeAbilities','UpgradeUnitTypeUpgrade','UpgradeRefSet','UpgradeSize']:
                for name,body,body_start,declaration_start in declarations(text,kind):
                    key=(kind.lower(),name.lower())
                    if key in self.blocks:raise ValueError('Duplicate active DDF declaration: '+str((key,path,self.blocks[key]['path'])))
                    self.blocks[key]={'kind':kind,'name':name,'body':body,'path':path,'body_start':body_start,
                                      'line':text.count('\n',0,declaration_start)+1,'text':text}

    def raw(self,name):
        key=name.lower();actual=self.paths.get(key,name)
        return self.reader.read(actual)

    def resolve(self,name,stack=()):
        result=super().resolve(name,stack)
        data=self.block('UnitType',name)
        vehicle=result['abilities'].setdefault('vehiclemove',{})
        for body,offset in sections(data['body'],'VehicleMove'):
            for key,value,position in numeric_assignments(body):
                vehicle[key.lower()]=(value,self.block_ref(data,offset+position))
        if not vehicle:result['abilities'].pop('vehiclemove')
        return result

    def table(self,name):
        rows=super().table(name)
        if self.paths[name.lower()].lower()!='db/techtree/upgrade_unittype5.csv':return rows
        # Native MainN selects an UpgradeRefSet, whose referenced CSV upgrade
        # applies at that event. The CSV EPOCH field is retained as raw metadata;
        # in UP15 CityCenterUpgradeEpoch15 incorrectly carries 13 in that column.
        activation={}
        def visit(ref,epoch,seen):
            key=ref.lower()
            if key in seen:return
            seen.add(key);activation.setdefault(key,set()).add(epoch)
            b=self.block('UpgradeRefSet',ref)
            if b:
                m=re.search(r'\bupgrades\s*=\s*\[([^]]*)\]',b['body'],re.I)
                if m:
                    for child in re.findall(r'\b\w+\b',m[1]):visit(child,epoch,seen)
        nodes=super().table('db/techtree/dbtechtreenod5.csv')
        for node in nodes:
            if re.fullmatch(r'Main\d+',node['NAME']):visit(node['UPGRADE'],int(node['EPOCH']),set())
        for row in rows:
            epochs=activation.get(row['UPGRADE'].lower(),set())
            row['_raw_epoch']=row['EPOCH']
            if len(epochs)==1 and int(row['EPOCH'])!=next(iter(epochs)):
                epoch=next(iter(epochs));row['EPOCH']=str(epoch)
                evidence={'upgrade':row['UPGRADE'],'raw_csv_epoch':int(row['_raw_epoch']),'activation_epoch':epoch,
                          'main_node':next(n for n in nodes if n['NAME']==f'Main{epoch}'),
                          'refset':self.block_ref(self.block('UpgradeRefSet',f'MainEpoch{epoch}')),
                          'csv_source':row['_source']}
                row['_epoch_activation']=evidence
                if evidence not in self.epoch_rebindings:self.epoch_rebindings.append(evidence)
        return rows

    def ref(self,path,line,declaration=None):
        actual=self.paths.get(path.lower(),path);origin=self.reader.origin(actual)
        result={'archive':origin['physical'],'entry':origin.get('member',origin['virtualPath']),
                'virtual_path':origin['virtualPath'],'line':line,'sha256':origin['sha256'],'origin':origin}
        if declaration:result['declaration']=declaration
        return result

    def manifest_audit(self):
        return {'manifest':str(self.archive),'manifest_sha256':hashlib.sha256(self.archive.read_bytes()).hexdigest(),
                'profile':self.reader.manifest['profile'],'active_cfgs':self.active_cfgs,
                'active_ddfs':[{'path':p,'origin':self.reader.origin(p)} for p in self.active_ddfs],
                'compatibility_requests':NATIVE_TABLES,
                'epoch_rebindings':self.epoch_rebindings,
                'scope':'Legacy importer request names are explicitly rebound to native00541940 CSV5 literals. This is a generator API mapping, not an invented game filesystem alias.'}
