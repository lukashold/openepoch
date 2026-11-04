"""Resolve each bound upgrade's actual native UPGRADEREFS with complete leaf audit."""
import copy,re
from import_ee2_units import sections,scalar,numeric_assignments

def apply_refs(db,original,upgrade,epoch,base_abilities):
    abilities=copy.deepcopy(base_abilities);records=[];sizes=[];factors=[]
    if not hasattr(db,'_native_factor_rows'):
        db._native_factor_rows={r['']:r for r in db.table('db/techtree/upgrade_factorse5.csv')}
    roots=re.findall(r'\w+',upgrade.get('UPGRADEREFS','')) if upgrade else ([f'CitizenUpgradeEpoch{epoch}'] if original=='CitizenM' else [])
    def visit(name,stack):
        if name.lower() in stack:raise ValueError('Cyclic UpgradeRefs '+name)
        path=stack+(name.lower(),)
        if name in db._native_factor_rows:
            raw=db._native_factor_rows[name]
            record={'name':name,'kind':'CSVFactorSet','factor':raw['FACTOR'],'method':raw['METHOD'],'value':float(raw['VALUE']),'unit':raw['UNIT'],'attribute':raw['UNIT TYPE ATTRIBUTE'],'children':int(raw['CHILDREN'] or 0),'source':raw['_source']}
            factors.append(record);records.append(record);return
        declaration=next((db.block(kind,name) for kind in ['UpgradeAbilities','UpgradeRefSet','UpgradeSet','UpgradeUnitTypeUpgrade','UpgradeSize'] if db.block(kind,name)),None)
        if declaration is None:
            records.append({'name':name,'kind':'unresolved','status':'unsupported unresolved declaration'});return
        record={'name':name,'kind':declaration['kind'],'source':db.block_ref(declaration)};records.append(record)
        if declaration['kind']=='UpgradeAbilities':
            changes=[]
            for match in re.finditer(r'\b(\w+)\s*\{',declaration['body']):
                ability=match[1]
                for body,offset in sections(declaration['body'],ability):
                    values=abilities.setdefault(ability.lower(),{})
                    fields={}
                    for key,value,position in numeric_assignments(body):
                        ref=db.block_ref(declaration,offset+position);values[key.lower()]=(value,ref);fields[key]=value
                    for key in re.findall(r'\b(\w+)\s*=',body):
                        value=scalar(body,key)
                        if value is not None and not re.fullmatch(r'[+-]?[\d.]+',value):
                            values[key.lower()]=(value,db.block_ref(declaration,offset));fields.setdefault(key,value)
                    changes.append({'ability':ability,'fields':fields})
            record['abilities']=changes;record['status']='applied ability fields; runtime omissions itemized separately';return
        if declaration['kind']=='UpgradeSize':
            values={key.lower():(value,db.block_ref(declaration,position)) for key,value,position in numeric_assignments(declaration['body'])}
            if 'sizex' in values and 'sizey' in values:sizes.append(values)
            record['values']={key:value[0] for key,value in values.items()};record['status']='retained raw size only; grid/world conversion unsupported';return
        for body,offset in sections(declaration['body'],'UpgradeUnitTypeFactor'):
            factor={'name':name,'kind':'InlineFactor','unit':scalar(body,'UnitType') or '',
                    'factor':scalar(body,'factor'),'method':scalar(body,'method'),'value':float(scalar(body,'value')),
                    'children':int(scalar(body,'upgradeChildren') or 0),'attribute':'','source':db.block_ref(declaration,offset)}
            factors.append(factor);records.append(factor)
        for field in ['upgrades','UpgradeRefs']:
            match=re.search(r'\b'+field+r'\s*=\s*\[([^]]*)\]',declaration['body'],re.I)
            if match and '{' not in match[1]:
                for child in re.findall(r'\w+',match[1]):visit(child,path)
    for name in roots:visit(name,())
    return abilities,records,sizes,factors
