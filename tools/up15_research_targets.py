"""Source-driven selectors projected onto the explicitly aliased 57 source roles."""
import re
from import_ee2_units import declarations,clean_comments,scalar,sections

def selectors(db,bindings):
    origin=db.reader.origin('dbunittypeattributes.ddf')
    raw=db.reader.read('dbunittypeattributes.ddf');text=clean_comments(raw.decode('cp1252').replace('\r',''))
    attrs={n:(b,text.count('\n',0,s)+1) for n,b,_,s in declarations(text,'UnitTypeAttribute')}
    classes={b['id']:b['source_unit'] for b in bindings if b.get('records')}
    def traits(name):
        chain=[];props={};seen=set()
        while name:
            if name.lower() in seen:raise ValueError('UnitType cycle')
            seen.add(name.lower());b=db.block('UnitType',name)
            if not b:raise ValueError(name)
            chain.append(name)
            for body,_ in sections(b['body'],'properties'):
                for field in ['rps','moveType']:
                    value=scalar(body,field)
                    if value is not None:props.setdefault(field,value)
            name=scalar(b['body'],'parent')
        return chain,props
    info={i:traits(name) for i,name in classes.items()}
    units={name:[i for i,(chain,_) in info.items() if name in chain] for name in sorted({n for chain,_ in info.values() for n in chain})}
    roles={};audit={}
    for name,(body,line) in attrs.items():
        fields={key:re.findall(r'"?([A-Za-z_]\w*)"?',content) for key,content in re.findall(r'\b(\w+)\s*=\s*\[([^]]*)\]',body)}
        supported=set(fields)<= {'UnitTypeFamilies','UnitTypeInstances','RPSMask','MovementMask'}
        # A single selector dimension has no unresolved Any/All interaction.
        dimensions=sum(bool(fields.get(key)) for key in ['UnitTypeFamilies','UnitTypeInstances','RPSMask','MovementMask'])
        supported=supported and dimensions==1
        ids=[]
        if supported:
            for i,(chain,props) in info.items():
                match=(any(n in chain for n in fields.get('UnitTypeFamilies',[])) or
                       classes[i] in fields.get('UnitTypeInstances',[]) or
                       props.get('rps','').removeprefix('kUnitRPS_') in fields.get('RPSMask',[]) or
                       props.get('moveType','').removeprefix('kUnitMoveType_') in fields.get('MovementMask',[]))
                if match:ids.append(i)
        roles[name]=ids
        audit[name]={'targets':ids,'supported_selector':supported,'fields':fields,'line':line,'origin':origin,
                     'scope':'Native family/RPS/movement selector projected onto existing source-class aliases; not full original roster or per-instance UnitType identity.'}
    return roles,units,audit
