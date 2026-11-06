"""Preserve native Main reference order across80 exact family bindings."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
def run():
    units=json.loads((ROOT/'docs/ee2-civ8-units.json').read_text(encoding='utf8'))
    audit=json.loads((ROOT/'docs/roster6-availability.json').read_text(encoding='utf8'))
    civs=json.loads((ROOT.parent/'civ8-sources.json').read_text(encoding='utf8'))['civs']
    families={}
    for b in units['bindings']:families.setdefault(b['source_unit'],[]).append(b['id'])
    production={}
    for b in units['bindings']:
        if b['id']>=57 and b.get('production_node'):production.setdefault(b['production_node']['NAME'],[]).append(b['id'])
    roster_retirements={'WarGalley','HeavyMounted1','LightMounted1','Frigate','Galleon'}
    rows=[]
    def walk(node,epoch,path=()):
        path+= (node['name'],)
        for child in node.get('children',[]):walk(child,epoch,path)
        if node['kind']=='CSVUnitUpgrade':
            civ=node.get('civ')or'All';tt=0 if civ=='All'else civs.index(civ)+1
            for ident in families.get(node['unit'],[]):rows.append({'epoch':epoch,'kind':'Reprice','def':ident,'ttciv':tt,'path':path,'source':node['source']})
        if node.get('obsoleteNode'):
            for ident in production.get(node['obsoleteNode'],[]):rows.append({'epoch':epoch,'kind':'Obsolete','def':ident,'ttciv':0,'path':path,'source':node['source']})
            if node['obsoleteNode'] in roster_retirements:
                for ident in families[node['obsoleteNode']]:rows.append({'epoch':epoch,'kind':'RosterObsolete','def':ident,'ttciv':0,'path':path,'source':node['source']})
    for main in audit['mainNodes']:
        if not main['TTCiv']:walk(main['graph'],main['epoch']-1)
    out=['// Generated source-ordered80-family Main actions.',
        '#[derive(Clone,Copy)] pub enum Kind{Reprice,Obsolete,RosterObsolete}',
        'pub struct Action{pub epoch:u8,pub kind:Kind,pub def:crate::ids::DefId,pub ttciv:u8}',
        'pub const ACTIONS:&[Action]=&[']
    for r in rows:out.append('Action{epoch:%d,kind:Kind::%s,def:%d,ttciv:%d},'%(r['epoch'],r['kind'],r['def'],r['ttciv']))
    out.append('];')
    (ROOT/'src/civ8_main.rs').write_text('\n'.join(out)+'\n',encoding='utf8')
    (ROOT/'docs/civ8-main-actions.json').write_text(json.dumps({'format':'native-civ8-main-order-v1','actions':rows,'limits':['RosterObsolete actions apply only when the source production roster is active.']},indent=2)+'\n',encoding='utf8')
    print(json.dumps({'actions':len(rows),'obsolete':[(r['epoch'],r['def'])for r in rows if r['kind']=='Obsolete']}))
if __name__=='__main__':run()
