"""Import original names into local-only media, separately for BASE and UP.

This binds the existing runtime roles to their audited source classes. It does
not assert that an adapted role has the full original behavior or graphics.
"""
from pathlib import Path
import argparse,csv,hashlib,json,re,sys

HERE=Path(__file__).resolve()
PROJECT=HERE.parents[1] if HERE.parent.name=='tools' else HERE.parents[2]/'outputs/OpenEpoch'
sys.path.insert(0,str(PROJECT/'tools'))
from import_ee2_units import Database
from up15_source_database import ActiveDatabase,default_manifest

TEXTS=['db/Text/dbtext_unittypenames.utf8','db/Text/dbtext_techtreenames.utf8',
       'db/Text/dbtext_unittypetips.utf8']

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()

def text_table(db,path):
    result={}
    for line_number,line in enumerate(db.read(path).splitlines(),1):
        if not line.strip() or line.lstrip().startswith('//'):continue
        row=next(csv.reader([line],skipinitialspace=True))
        if len(row)!=2:raise ValueError(f'{path}:{line_number}: expected token and text')
        key,value=row
        if value.startswith('"') and value.endswith('"'):value=value[1:-1]
        if key in result:raise ValueError('Duplicate localization token: '+key)
        result[key]={'text':value,'source':db.ref(path,line_number)}
    return result

def inherited_tokens(db,name):
    result={};seen=set()
    def visit(unit):
        if unit.casefold() in seen:raise ValueError('Cyclic display-name inheritance: '+unit)
        seen.add(unit.casefold());block=db.block('UnitType',unit)
        if not block:raise ValueError('Missing display-name UnitType: '+unit)
        parent=re.search(r'\bparent\s*=\s*(\w+)',block['body'],re.I)
        if parent:visit(parent[1])
        for field in ['displayName','displayNamePlural','VerboseTooltip']:
            match=re.search(r'\b'+field+r'\s*=\s*(?:"([^"\n]+)"|(\w+))',block['body'],re.I)
            if match:result[field]={'token':match[1] or match[2],
                                    'source':db.block_ref(block,match.start())}
    visit(name);return result

def profile(project,profile_id,db):
    audit_path=project/'docs'/('ee2-up15-units.json' if profile_id=='up15' else 'ee2-units.json')
    audit=json.loads(audit_path.read_text(encoding='utf8'))
    # A role/class audit from another installation must not silently supply
    # declaration tokens for the current source database.
    for path,record in audit['source_files'].items():
        actual=db.paths.get(path.lower())
        if actual is None or hashlib.sha256(db.zip.read(actual)).hexdigest()!=record['sha256']:
            raise ValueError('Unit audit source differs from installation: '+path)
    bindings={b['id']:b for b in audit['bindings']}
    textmap={}
    for path in TEXTS:
        for token,record in text_table(db,path).items():
            if token in textmap:raise ValueError('Ambiguous cross-file localization token: '+token)
            textmap[token]=record
    rows=[];missing=[];unit_tokens={}
    for row in audit['class_rows']:
        unit=row['source_unit']
        if unit not in unit_tokens:unit_tokens[unit]=inherited_tokens(db,unit)
        fields=dict(unit_tokens[unit]);upgrade=row.get('source_upgrade') or {}
        # Native0053C930 clears the prior plural when a new singular is set
        # without a positive plural index. Do not inherit a stale old plural.
        if upgrade.get('DISPLAYNAME','').strip():fields.pop('displayNamePlural',None)
        for native,field in [('DISPLAYNAME','displayName'),('DISPLAYNAME_PLURAL','displayNamePlural'),
                             ('TOOLTIP','VerboseTooltip')]:
            token=upgrade.get(native,'').strip()
            if token:fields[field]={'token':token,'source':upgrade['_source']}
        values={};references={}
        for field,selection in fields.items():
            token=selection['token'];record=textmap.get(token)
            references[field]={'token':token,'declaration':selection['source']}
            if record:
                values[field]=record['text'];references[field]['text']=record['source']
            else:missing.append({'id':row['id'],'epoch':row['epoch'],'field':field,'token':token})
        if not values.get('displayName'):
            missing.append({'id':row['id'],'epoch':row['epoch'],'field':'displayName','unit':unit})
            continue
        rows.append({'id':row['id'],'epoch':row['epoch'],'until':row['until'],
                     'sourceUnit':unit,'sourceEpoch':row['source_epoch'],
                     'roleAdapter':bindings[row['id']].get('role_adapter',not bindings[row['id']].get('identity_verified',False)),
                     'name':values['displayName'],'plural':values.get('displayNamePlural',''),
                     'description':values.get('VerboseTooltip',''),'source':references})
    return {'profile':profile_id,'ruleset':2 if profile_id=='up15' else 1,
            'auditSha256':digest(audit_path),'rows':rows,'missing':missing,
            'scope':'Original text of the currently bound source class, including explicit role adapters; class behavior equivalence is not implied.'}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--project',type=Path,default=PROJECT)
    parser.add_argument('--install',type=Path,default=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II'))
    parser.add_argument('--up-manifest',type=Path)
    parser.add_argument('--out',type=Path)
    args=parser.parse_args();project=args.project.resolve();install=args.install.resolve()
    out=(args.out or project/'docs/local-original/labels-native.json').resolve()
    if out.is_relative_to(install):raise ValueError('Output must stay outside the original installation')
    base=Database(install/'zips/db.zip');up=ActiveDatabase(args.up_manifest or default_manifest(project))
    try:profiles={name:profile(project,name,db)for name,db in [('base',base),('up15',up)]}
    finally:base.close();up.close()
    result={'format':'ee2-native-labels-v1','language':'de','profiles':profiles,
            'scope':'Locally imported original names/descriptions; never required by the independent ruleset0.'}
    out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
    print(json.dumps({'file':str(out),'sha256':digest(out),
                     'profiles':{k:{'rows':len(v['rows']),'missing':len(v['missing'])}for k,v in profiles.items()}},ensure_ascii=False))

if __name__=='__main__':main()
