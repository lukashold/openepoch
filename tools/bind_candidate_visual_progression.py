from pathlib import Path
import json
import import_ee2_units as units
ROOT=Path(__file__).resolve().parents[1]
def run():
    p=ROOT/'docs/ee2-up15-units.json';data=json.loads(p.read_text(encoding='utf8'));models=json.loads((ROOT/'docs/local-original/models/manifest-up15.json').read_text(encoding='utf8'))['models']
    model=next(r for r in models if r['id']==25 and r['epoch']==12)
    native=next(r for r in data['class_rows'] if r['id']==25 and r['epoch']==12)
    assert native['source_visual']=='LightArtillery2_13'
    assert model['databaseVisualBinding']['visualDefinition']==native['source_visual']
    if not any(r['id']==25 and r['epoch']==12 for r in data['rows']):
        row=dict(native);row['model_verified']=True;row['identity_verified']=data['bindings'][25]['identity_verified'];row['visual_adapter']=False
        row['candidate_model_binding']=model;data['bindings'][25]['records'].append(len(data['rows']));data['rows'].append(row)
    data['visual_scope']='Exact UP replacements28/43 and explicit AA13 HumveeStriker progression25; separate UP model manifest preserves other profiles.'
    p.write_text(json.dumps(data,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
    units.write_rust(data['rows'],data['class_rows'],{int(k):v for k,v in data['production_class_from'].items()},ROOT/'src/ee2_up15_units.rs')
    q=ROOT/'src/ee2_up15_units.rs';q.write_text(q.read_text().replace('tools/import_ee2_units.py','tools/bind_candidate_visual_progression.py').replace('base db.zip','active UP CSV5/CFG5'))
    print('Exact AA visual progression:',len(data['rows']),'visual rows,',len(data['class_rows']),'class rows')
if __name__=='__main__':run()
