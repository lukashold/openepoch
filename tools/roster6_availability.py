"""Read-only native CFG5 Main/Obsolete/Undo graph and four-family boundaries."""
from pathlib import Path
import hashlib,json,re,sys
ROOT=Path(__file__).resolve().parents[1]
WORK=ROOT/'docs'
sys.path.insert(0,str(ROOT/'tools'))
from up15_source_database import ActiveDatabase,default_manifest
from import_ee2_units import declarations,scalar

def clean(s):
    return re.sub(r'/\*.*?\*/|//[^\n]*',lambda m:''.join('\n'if c=='\n'else' 'for c in m[0]),s,flags=re.S)

def end(s,start):
    stack=[];quoted=False
    for i in range(start,len(s)):
        c=s[i]
        if c=='"'and(i==0 or s[i-1]!='\\'):quoted=not quoted
        if quoted:continue
        if c in '[{':stack.append(c)
        elif c in ']}':
            if not stack or (stack.pop(),c)not in [('[',']'),('{','}')]:raise ValueError('Unbalanced upgrade expression')
            if not stack:return i
    raise ValueError('Unterminated upgrade expression')

def refs(body):
    """Top-level named or typed-inline upgrades, never words from nested fields."""
    result=[]
    for match in re.finditer(r'\b(?:upgrades|UpgradeRefs)\s*=\s*\[',body,re.I):
        start=match.end()-1;stop=end(body,start);text=body[start+1:stop];cursor=0
        while cursor<len(text):
            m=re.search(r'\b([A-Za-z_]\w*)\b',text[cursor:])
            if not m:break
            name=m[1];at=cursor+m.end();next_at=at
            while next_at<len(text)and text[next_at].isspace():next_at+=1
            if next_at<len(text)and text[next_at]=='{':
                last=end(text,next_at);result.append({'inlineKind':name,'body':text[next_at+1:last]});cursor=last+1
            else:result.append({'ref':name});cursor=at
    return result

def run():
    db=ActiveDatabase(default_manifest(ROOT));index={};files={}
    try:
        for path in db.active_ddfs:
            text=clean(db.read(path));origin=db.reader.origin(path)
            kinds=set(re.findall(r'\b(Upgrade\w*)\s+[A-Za-z_]\w*\s*\{',text))
            for kind in sorted(kinds):
                for name,body,body_start,decl_start in declarations(text,kind):
                    if name in index:raise ValueError('Duplicate active upgrade '+name)
                    index[name]={'kind':kind,'name':name,'body':body,'source':{'virtualPath':path,'line':text.count('\n',0,decl_start)+1,'sha256':origin['sha256']}}
            files[path]=origin['sha256']
        csv={r['UPGRADE']:r for r in db.table('db/TechTree/upgrade_unittypes.csv')}
        factors={r['']:r for r in db.table('db/TechTree/upgrade_factorset.csv')}
        tree=db.table('db/TechTree/dbtechtreenode.csv');main=[r for r in tree if r['BRANCH']=='Main'];unknown={}
        def walk(item,stack=()):
            if 'ref'in item:
                name=item['ref']
                if name in stack:raise ValueError('Upgrade cycle '+name)
                if name in index:
                    d=index[name];source=d['source'];kind=d['kind'];body=d['body']
                elif name in csv:
                    row=csv[name];record={'kind':'CSVUnitUpgrade','name':name,'source':row['_source'],'civ':row.get('CIV'),'unit':row['UNIT'],'epoch':row['EPOCH'],'children':[]}
                    for ref in re.findall(r'\b\w+\b',row.get('UPGRADEREFS','')):record['children'].append(walk({'ref':ref},stack+(name,)))
                    return record
                elif name in factors:return{'kind':'CSVFactor','name':name,'source':factors[name]['_source'],'factor':factors[name]['FACTOR']}
                else:unknown.setdefault(name,[]).append(list(stack));return{'kind':'unresolved','name':name}
                stack=stack+(name,)
            else:
                kind=item['inlineKind'];name='inline:'+kind;body=item['body'];source=None
            record={'kind':kind,'name':name,'source':source}
            if kind=='UpgradeObsoleteTech':record['obsoleteNode']=scalar(body,'tech')
            if kind=='UpgradeUndo':record['undoNode']=scalar(body,'techNode')
            if kind=='UpgradeEpoch':record['setsEpoch']=scalar(body,'epoch')
            children=refs(body)if kind in ['UpgradeSet','UpgradeRefSet','UpgradeUnitTypeUpgrade']else[]
            if children:record['children']=[walk(child,stack)for child in children]
            return record
        graphs=[];actions=[]
        def collect(node,root,path=()):
            path=path+(node['name'],)
            for field in ['obsoleteNode','undoNode','setsEpoch']:
                if field in node:actions.append({'mainNode':root['NAME'],'mainEpoch':int(root['EPOCH']),'mainTTCiv':root['TTCIV'],'action':field,'target':node[field],'path':list(path),'source':node['source']})
            for child in node.get('children',[]):collect(child,root,path)
        for row in main:
            graph=walk({'ref':row['UPGRADE']});collect(graph,row)
            graphs.append({'mainNode':row['NAME'],'epoch':int(row['EPOCH']),'TTCiv':row['TTCIV'],'rootUpgrade':row['UPGRADE'],'source':row['_source'],'graph':graph})
        production=[r for r in tree if r.get('PRODUCE')];boundaries=[]
        for ident,name in [(57,'Scout'),(58,'FightingSail'),(59,'AirDefense'),(60,'CoastalDefense')]:
            nodes=[r for r in production if r['PRODUCE']==name];obsolete=[a for a in actions if a['action']=='obsoleteNode'and a['target']in{r['NAME']for r in nodes}];undo=[a for a in actions if a['action']=='undoNode'and a['target']in{r['NAME']for r in nodes}]
            global_end=min([a['mainEpoch']-1 for a in obsolete if not a['mainTTCiv']]or[15])
            boundaries.append({'id':ident,'sourceFamily':name,'productionNodes':nodes,'startRuntimeEpoch':min(int(r['EPOCH'])for r in nodes)-1,'endRuntimeEpochExclusive':global_end,'obsoleteActions':obsolete,'undoActions':undo,'survivingUnitStatsAndModelsMustNotBeTruncated':True})
        result={'format':'native61v2-main-availability-audit-v1','nativeSourceManifestSha256':hashlib.sha256(default_manifest(ROOT).read_bytes()).hexdigest(),'mainNodes':graphs,'actions':actions,'unresolvedUpgradeRefs':unknown,'fourBoundaries':boundaries,'sourceFiles':files,'limits':['This derives source actions and declared Main paths; startup and earned-research shared runtime path requires the accompanying Ghidra exports.','Undo actions are recorded, never interpreted as automatic resurrection of obsolete production without runtime proof.','Only the four new definitions may change in61v2; existing units, audio, models, label and numeric class intervals remain intact.']}
        (WORK/'roster6-availability.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
        print(json.dumps({'mainNodes':len(graphs),'actions':len(actions),'unresolved':sorted(unknown),'bounds':[(b['sourceFamily'],b['startRuntimeEpoch'],b['endRuntimeEpochExclusive'])for b in boundaries]}))
        return result
    finally:db.close()
if __name__=='__main__':run()
