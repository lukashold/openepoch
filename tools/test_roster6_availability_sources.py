"""Independent direct-raw source checks for the four61v2 production bounds."""
from pathlib import Path
import hashlib,json,re
from up15_source_database import ActiveDatabase,default_manifest
ROOT=Path(__file__).resolve().parents[1]
def run():
    db=ActiveDatabase(default_manifest(ROOT));checks=0
    try:
        rows=db.table('db/TechTree/dbtechtreenode.csv');raw=db.read('db/TechTree/epoch11_upgrades_15.ddf')
        main=next(r for r in rows if r['NAME']=='Main11');assert main['BRANCH']=='Main'and int(main['EPOCH'])==11 and not main['TTCIV'];checks+=3
        assert re.search(r'UpgradeObsoleteTech\s+ObsoleteFightingSail\s*\{\s*tech\s*=\s*FightingSail\s*\}',raw);checks+=1
        body=re.search(r'UpgradeRefSet\s+MainEpoch11\s*\{(.*?)\n\}',raw,re.S)[1]
        assert re.search(r'\bObsoleteFightingSail\b',body);checks+=1
        expected={57:('Scout',0,15),58:('FightingSail',5,10),59:('AirDefense',9,15),60:('CoastalDefense',1,15)}
        audit=json.loads((ROOT/'docs/roster6-availability.json').read_text());defs=(ROOT/'src/roster6_defs.rs').read_text()
        for ident,(family,start,stop)in expected.items():
            production=[r for r in rows if r['PRODUCE']==family];assert production;checks+=1
            assert min(int(r['EPOCH'])for r in production)-1==start;checks+=1
            assert all(not r['TTCIV']for r in production);checks+=1
            generated=next(r for r in audit['fourBoundaries']if r['id']==ident)
            assert(generated['startRuntimeEpoch'],generated['endRuntimeEpochExclusive'])==(start,stop);checks+=1
            assert re.search(r'd\['+str(ident)+r'\]=Def\{name:"'+family+r'",epoch:'+str(start)+r',until:'+str(stop)+r',',defs);checks+=1
            # Surviving units keep all source numerical classes through the future.
            data=json.loads((ROOT/'docs/ee2-roster6-units.json').read_text());last=max((r for r in data['class_rows']if r['id']==ident),key=lambda r:r['epoch']);assert last['until']==15;checks+=1
        report={'format':'native61v2-independent-raw-availability-v1','status':'PASS','checks':checks,'manifestSha256':hashlib.sha256(default_manifest(ROOT).read_bytes()).hexdigest(),'main11Source':db.reader.origin('db/TechTree/epoch11_upgrades_15.ddf'),'bounds':expected,'unresolvedOtherMainRefs':audit['unresolvedUpgradeRefs'],'scope':'Four new families only; no numerical/model/audio intervals truncated.'}
        (ROOT.parent/'availability-source-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'status':'PASS','checks':checks}))
    finally:db.close()
if __name__=='__main__':run()
