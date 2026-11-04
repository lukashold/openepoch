"""Independent byte/source-oracle checks for the offline VFS source reader."""
from pathlib import Path
import csv,hashlib,io,json,sys,zipfile
from contextlib import ExitStack
from resolved_archive_reader import ResolvedArchiveReader

ROOT=Path(__file__).resolve().parents[1]
WORK=ROOT.parents[1]/'work'
path=WORK/'up-resolved-sources/effective-db-sources.json'
manifest=json.loads(path.read_text())
install=Path(manifest['profile']['physicalRoot'])
sha=lambda b:hashlib.sha256(b).hexdigest()
checks=0
def check(value,label):
    global checks
    assert value,label
    checks+=1

with ResolvedArchiveReader(path) as reader,ExitStack() as stack:
    archives={}
    def original(source):
        p=install/source['physical']
        if source['kind']=='loose':return p.read_bytes()
        if str(p) not in archives:archives[str(p)]=stack.enter_context(zipfile.ZipFile(p))
        return archives[str(p)].read(source['member'])
    check(len(reader.namelist())==manifest['counts']['fullPaths'],'complete virtual file list')
    for name in reader.namelist():
        origin=reader.origin(name)
        check(reader.read(name)==original(origin),'native query bytes equal independent original source: '+name)
        check(reader.read(name.swapcase())==reader.read(name),'case-insensitive source lookup: '+name)
        check(sha(reader.read(name))==origin['sha256'],'source SHA: '+name)
    for key,record in manifest['effectiveByPath'].items():
        data=Path(record['extracted']).read_bytes()
        check(data==original(record['source']),'every staged full-path record matches original, including shadowed records: '+key)
        check(sha(data)==record['source']['sha256'],'every full-path record hash: '+key)
    for name,choice in manifest['effectiveByBasename'].items():
        check(choice['status']=='resolved-first-basename-record','unambiguous basename: '+name)
        check(reader.read(name)==original(choice['source']),'basename chooses actual source: '+name)
    for name in ['dbunittypeattributes.ddf','territory.ddf','turret.ddf']:
        source=reader.origin(name);check(source['kind']=='loose' and source['physical'].startswith('zips/'),'loose basename precedence: '+name)
        packed=[r for r in manifest['effectiveByPath'].values() if Path(r['path']).name.casefold()==name and r['source']['kind']=='zip-member']
        check(bool(packed),'full-path ZIP alternative retained: '+name)
        check(reader.read(name)!=Path(packed[0]['extracted']).read_bytes(),'loose source differs from retained packed alternative: '+name)
    for name in ['DbFormationDefs.h','DbFrontEndDefs.h','DbResourceLevelDefs.h','GDbDisplayDefs.h']:
        check(reader.origin(name)['physical'].startswith('zips_ee2x/'),'native recursive loose header: '+name)
    comparisons=[]
    for item in manifest['version5SourceComparison']:
        a,b=reader.read(item['current']),reader.read(item['legacy'])
        check(a!=b,'UP5 source actually differs from BASE: '+item['current'])
        comparisons.append({'name':item['current'],'sha256':sha(a),'legacySha256':sha(b),
                            'rows':len(list(csv.reader(io.StringIO(a.decode('cp1252'))))) if item['current'].endswith('.csv') else len(a.splitlines()),
                            'origin':reader.origin_ref(item['current'])})
    for invalid in ['does-not-exist.ddf','../dbunitfile5.cfg','/db/dbunitfile5.cfg']:
        try:reader.read(invalid)
        except KeyError:checks+=1
        else:raise AssertionError(invalid)
try:reader.read('dbunitfile5.cfg')
except ValueError:checks+=1
else:raise AssertionError('closed reader accepted a read')

# Separate malformed fixture, never mutate a staged game source used by importers.
fixture=WORK/'up-reader-test-fixture';(fixture/'files').mkdir(parents=True,exist_ok=True)
(fixture/'files/one.ddf').write_bytes(b'changed')
bad={'format':'ee2-up-static-source-resolution-v1','profile':{},'effectiveByPath':{'one.ddf':{'path':'one.ddf','extracted':str((fixture/'files/one.ddf').resolve()),'source':{'bytes':8,'sha256':sha(b'original')}}},'effectiveByBasename':{'one.ddf':{'status':'resolved-first-basename-record','path':'one.ddf'}}}
(fixture/'manifest.json').write_text(json.dumps(bad))
with ResolvedArchiveReader(fixture/'manifest.json') as test:
    try:test.read('one.ddf')
    except ValueError:checks+=1
    else:raise AssertionError('altered source accepted')
report={'format':'native-up-source-reader-tests-v1','status':'passed','checks':checks,'manifestSha256':sha(path.read_bytes()),'counts':manifest['counts'],
        'version5SourceComparison':comparisons,'scope':'All effective native queries and all staged full-path bytes compared with independent original ZIP/member or loose files. Case, basename/full-path distinction, recursive loose headers, missing/closed/tampered-source handling. No original files modified; no game code executed.'}
(ROOT.parent/'EE2-Research/analysis/native-up-source-reader-tests.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps({'status':'passed','checks':checks,'counts':manifest['counts'],'manifestSha256':report['manifestSha256']},indent=2))
