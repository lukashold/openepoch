"""Generate effective native attack fields from active UP source inheritance/refs.
Runtime binder quirks are proved in native-attack-ability-phase3.c (007da1c0).
"""
from pathlib import Path
import hashlib,json,re,struct
from roster6_codegen import rust_identities
from up15_source_database import ActiveDatabase,default_manifest
from up15_unit_upgrade_refs import apply_refs
from import_ee2_units import scalar,sections,numeric_assignments
ROOT=Path(__file__).resolve().parents[1]
F32=lambda v:struct.unpack('<f',struct.pack('<f',float(v)))[0]
def run(prefix="ee2-up15"):
    db=ActiveDatabase(default_manifest(ROOT));units=json.loads((ROOT/('docs/'+prefix+'-units.json')).read_text(encoding='utf8'));rows=[]
    def raw_ability(name,ability):
        declaration=db.block('UnitType',name)
        if not declaration:return {}
        parent=scalar(declaration['body'],'parent');values=raw_ability(parent,ability) if parent else {}
        for body,offset in sections(declaration['body'],ability):
            for key,value,position in numeric_assignments(body):values[key.lower()]=(value,db.block_ref(declaration,offset+position))
            for key in re.findall(r'\b(\w+)\s*=',body):
                value=scalar(body,key)
                if value is not None and key.lower() not in {k.lower() for k,_,_ in numeric_assignments(body)}:values[key.lower()]=(value,db.block_ref(declaration,offset))
        return values
    def prop(name,key):
        while name:
            d=db.block('UnitType',name)
            for body,_ in sections(d['body'],'properties'):
                value=scalar(body,key)
                if value is not None:return value
            name=scalar(d['body'],'parent')
        return None
    for row in units['class_rows']:
        source=row['source_unit'];base=db.resolve(source)['abilities'];base={k:dict(v) for k,v in base.items()};full=raw_ability(source,'Attack')
        if full:base['attack']=full
        ab,refs,_sizes,_factors=apply_refs(db,source,row.get('source_upgrade'),row['source_epoch'],base);attack=ab.get('attack',{})
        value=lambda key,default=0:attack.get(key.lower(),(default,None))[0]
        boolean=lambda key: str(value(key,False)).lower() in ['true','1','1.0']
        present=bool(attack)
        missile=str(value('missileName','')).strip()
        if missile.lower() in ['null','none','0']:missile=''
        weapon=str(value('WeaponType','None')).removeprefix('kWeaponType_')
        # Existing renderer categories are an explicit presentation adapter.
        # A rocket is W_MISSILE5; W_TORP4 is drawn below the water surface.
        # Named shells retain the existing ballistic shell path, not bullets.
        kind=(4 if 'torpedo' in missile.lower() else 6 if 'bomb' in missile.lower() else 1 if 'arrow'in missile.lower() else 5 if re.search('missile|rocket',missile,re.I) else 3) if missile else {'Melee':0,'Beam':2,'LightProjectile':2,'MediumProjectile':3,'Missile':5,'Nuke':6}.get(weapon,0)
        speed=None;ammo_source=[]
        if missile and db.block('UnitType',missile):
            ammo=raw_ability(missile,'ProjectileMissileMove')
            if 'speed' in ammo:
                speed=float(ammo['speed'][0]);ammo_source=[ammo['speed'][1]]
        pack=F32(value('packTime'));unpack=F32(value('unpackTime'));angle=F32(F32(value('angularRange'))*F32(0.01745329238474369))
        friendly=F32(value('areaDamageFriendlyRatio'))
        effective={'id':row['id'],'epoch':row['epoch'],'until':row['until'],'present':present,'sourceUnit':source,
          'sourceUpgrade':(row.get('source_upgrade')or{}).get('UPGRADE'),'missile':missile,'weaponType':weapon,'visualKind':kind,
          'minMobile':F32(value('minRangeMobile')),'minImmobile':F32(value('minRangeImmobile')),
          'angularRange':angle if angle!=0 else F32(0.5199999809265137),'applyTime':F32(value('applyDamageTime')),
          'packTime':pack if pack!=0 else F32(1),'unpackTime':unpack if unpack!=0 else F32(1),
          'splashRadius':F32(value('areaDamageRadius')),'friendlyRatio':friendly if friendly==1 else F32(0),
          'ignoreBlocking':boolean('ignoreBlocking'),'attackOnMove':boolean('attackOnTheMove'),'latent':boolean('latent'),
          'disableAutoTarget':boolean('disableAutoTarget'),'ordnance':int(float(value('ordnance'))),
          'startWithNoOrdnance':boolean('startWithNoOrdnance'),'turret':str(value('turretControllerName','')),
          'air':str(prop(source,'moveType')).removeprefix('kUnitMoveType_')=='Air',
          'mobile':row['fields']['raw_speed']['value'] is not None and row['fields']['raw_speed']['value']>0,
          'projectileSpeed':speed,'ammoSources':ammo_source,'rawFields':{k:{'value':v,'source':s} for k,(v,s)in attack.items()},
          'effectiveStats':{k:row['fields'][k] for k in ['damage','range','reload']}}
        rows.append(effective)
    out=['// Generated from actual active UP Attack inheritance/UPGRADEREFS; candidate only.',
         '#[derive(Clone,Copy,Debug)] pub struct Attack{pub id:u8,pub epoch:u8,pub until:u8,pub present:bool,pub missile:bool,pub visual_kind:u8,pub min_mobile:f32,pub min_immobile:f32,pub angle:f32,pub apply_time:f32,pub pack_time:f32,pub unpack_time:f32,pub splash:f32,pub friendly:f32,pub latent:bool,pub auto_target:bool,pub attack_on_move:bool,pub ordnance:u16,pub air:bool,pub mobile:bool,pub projectile_speed:Option<f32>}',
         'pub const ROWS:&[Attack]=&[']
    for r in rows:
        boolean=lambda key:str(bool(r[key])).lower();speed='None'if r['projectileSpeed'] is None else f'Some({r["projectileSpeed"]}_f32)'
        out.append('Attack{'+f'id:{r["id"]},epoch:{r["epoch"]},until:{r["until"]},present:{boolean("present")},missile:{str(bool(r["missile"])).lower()},visual_kind:{r["visualKind"]},min_mobile:{r["minMobile"]}_f32,min_immobile:{r["minImmobile"]}_f32,angle:{r["angularRange"]}_f32,apply_time:{r["applyTime"]}_f32,pack_time:{r["packTime"]}_f32,unpack_time:{r["unpackTime"]}_f32,splash:{r["splashRadius"]}_f32,friendly:{r["friendlyRatio"]}_f32,latent:{boolean("latent")},auto_target:{str(not r["disableAutoTarget"]).lower()},attack_on_move:{boolean("attackOnMove")},ordnance:{r["ordnance"]},air:{boolean("air")},mobile:{boolean("mobile")},projectile_speed:{speed}'+'},')
    out+= ['];',"pub fn lookup(id:u8,epoch:u8)->Option<&'static Attack>{ROWS.iter().find(|r|r.id==id&&epoch>=r.epoch&&epoch<r.until)}",
       'pub fn min_range(a:&Attack,target_mobile:bool)->f32{let r=if target_mobile{a.min_mobile}else{a.min_immobile};if r==0.0&&a.splash>0.0&&!a.air{a.splash}else{r}}']
    (ROOT/('src/'+prefix.replace('-','_')+'_attack.rs')).write_text(rust_identities('\n'.join(out)+'\n'))
    audit={'format':'native-up15-attack-fields-candidate-v1','rows':rows,'nativeBinder':'007da1c0: 18dword copy; degrees->radians; angular0->.52; pack/unpack0->1; friendlyRatio kept only when exactly1.0.','runtimeSource':'native-attack-ability-phase1/2/3.c','limits':['Latent gate modifier-default polarity not yet resolved; never enable houses merely from positive damage.','DB WeaponType is visual/death explosion type, not weapon/armor damage multiplier.','Pack/unpack application state, firing banks and ordnance/rearm are pending runtime proof.']}
    (ROOT/('docs/native-'+prefix.removeprefix('ee2-')+'-attack.json')).write_text(json.dumps(audit,indent=2)+'\n')
    print(json.dumps({'rows':len(rows),'attacking':sum(r['present'] for r in rows),'latent_ids':sorted({r['id'] for r in rows if r['latent']}),'splash_rows':sum(r['splashRadius']>0 for r in rows),'projectile_speeds':sum(r['projectileSpeed'] is not None for r in rows)}))
    db.close();return audit
if __name__=='__main__':run()
