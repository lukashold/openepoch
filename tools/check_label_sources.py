"""Independently verify every imported text against raw source lines and columns."""
from pathlib import Path
import csv,hashlib,json,re,sys,zipfile

PROJECT=Path(__file__).resolve().parents[1];ROOT=PROJECT.parents[1]
sys.path.insert(0,str(PROJECT/'tools'))
from resolved_archive_reader import ResolvedArchiveReader
data_path=PROJECT/'docs/local-original/labels-native.json'
data=json.loads(data_path.read_text(encoding='utf8'))
install=Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
base=zipfile.ZipFile(install/'zips/db.zip')
base_names={p.lower():p for p in base.namelist()}
up=ResolvedArchiveReader(ROOT/'work/up-resolved-sources/effective-db-sources.json')
checks=0;source_files={};counts={}

def check(value,message):
    global checks
    if not value:raise AssertionError(message)
    checks+=1

def source_bytes(profile,source):
    name=source.get('virtual_path') or source['entry']
    raw=up.read(name) if profile=='up15' else base.read(base_names[name.lower()])
    actual=hashlib.sha256(raw).hexdigest()
    check(actual==source['sha256'],'source digest '+name)
    source_files[profile+':'+name]=actual
    return raw

def text_source(profile,ref,token,expected):
    raw=source_bytes(profile,ref);line=raw.decode('utf-8-sig').replace('\r','').splitlines()[ref['line']-1]
    # Separate parser from the csv-based importer: these native rows use the
    # triple-quoted text representation. Preserve inner quotes exactly.
    match=re.fullmatch(r'([^,]+),[ \t]*"""(.*)"""[ \t]*',line)
    check(match is not None,'native triple-quoted text row '+token+' '+repr(line[:180]))
    check(match[1]==token,'native text token')
    check(match[2].replace('""','"')==expected,'native original text bytes')

for profile,record in data['profiles'].items():
    path=PROJECT/'docs'/('ee2-up15-units.json' if profile=='up15' else 'ee2-units.json')
    audit=json.loads(path.read_text(encoding='utf8'))
    check(record['auditSha256']==hashlib.sha256(path.read_bytes()).hexdigest(),'bound audit digest')
    classes={(r['id'],r['epoch'],r['until']):r for r in audit['class_rows']}
    text_count=0
    for row in record['rows']:
        native=classes.get((row['id'],row['epoch'],row['until']))
        check(native is not None,'class interval from native numerical audit')
        check(native['source_unit']==row['sourceUnit'],'actual bound source class')
        check(native['source_epoch']==row['sourceEpoch'],'actual source epoch event')
        for field,target,column in [('displayName','name','DISPLAYNAME'),('displayNamePlural','plural','DISPLAYNAME_PLURAL'),('VerboseTooltip','description','TOOLTIP')]:
            reference=row['source'].get(field)
            if not reference or 'text' not in reference:
                check(not row[target],'no invented text for missing token');continue
            text_count+=1;token=reference['token'];declaration=reference['declaration']
            raw=source_bytes(profile,declaration)
            text=raw.decode('utf-8-sig').replace('\r','');lines=text.splitlines();line=lines[declaration['line']-1]
            if declaration['entry'].lower().endswith('.csv'):
                header=next(csv.reader([lines[0].removeprefix('//').strip()]));columns=next(csv.reader([line]))
                index=header.index(column) if column!='DISPLAYNAME_PLURAL' else [i for i,v in enumerate(header)if v=='DISPLAYNAME'][1]
                check(columns[index]==token,'actual singular/plural/tooltip CSV column')
                check(columns[header.index('UPGRADE')]==native['source_upgrade']['UPGRADE'],'actual activated CSV upgrade')
            else:
                match=re.search(r'\b'+field+r'\s*=\s*(?:"([^"\n]+)"|(\w+))',line,re.I)
                check(match is not None and (match[1]or match[2])==token,'actual inherited DDF field')
                parents={item.get('declaration') for item in native['inheritance']}
                check(declaration.get('declaration') in parents,'field belongs to actual UnitType ancestry')
            text_source(profile,reference['text'],token,row[target])
    counts[profile]={'classIntervals':len(record['rows']),'resolvedFields':text_count,'missing':record['missing']}
base.close();up.close()
report={'format':'openepoch-native5-label-source-checks-v1','status':'passed','checks':checks,
        'dataSha256':hashlib.sha256(data_path.read_bytes()).hexdigest(),'profiles':counts,'sourceFileSha256':source_files,
        'originalExeOrDllExecuted':False,'originalFilesModified':False,
        'scope':'All text values independently decoded from raw original rows; actual CSV columns or inherited DDF source chains and audited class intervals.'}
out=ROOT/'outputs/EE2-Research/analysis/native5-label-source-tests.json';out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps({k:v for k,v in report.items()if k not in ['sourceFileSha256','profiles']}))
