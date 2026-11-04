"""Bind current visual variants to native class scaling; never edits geometry."""
import collections
import hashlib
import json
import pathlib
import re
import struct
from import_ee2_units import Database, scalar, model_stem, clean_comments, declarations

ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
WORKSPACE=ROOT.parents[1]
ENGINE=WORKSPACE/'work/EE2-engine.exe'
# Exact float literals pushed by the native static initializers005f6a60..b40.
SPECS={
 'kScaleUnit':('g_UnitScale',0x005f6a60,0x3e428f5c),
 'kScaleLandAnimal':('g_LandAnimalScale',0x005f6a80,0x3e428f5c),
 'kScaleVehicle':('g_VehicleScale',0x005f6aa0,0x3dcccccd),
 'kScaleMountedUnit':('g_MountedUnitScale',0x005f6ac0,0x3e4ccccd),
 'kScaleBuilding':('g_BuildingScale',0x005f6ae0,0x3d23d70a),
 'kScaleWall':('g_WallScale',0x005f6b00,0x3d23d70a),
 'kScaleTree':('g_TreeScale',0x005f6b20,0x3d3851ec),
 'kScaleAmmo':('g_AmmoScale',0x005f6b40,0x3e333333),
}

def binary_literal_proofs():
 raw=ENGINE.read_bytes();pe=struct.unpack_from('<I',raw,0x3c)[0]
 base=struct.unpack_from('<I',raw,pe+24+28)[0];sections=[]
 for n in range(struct.unpack_from('<H',raw,pe+6)[0]):
  p=pe+24+struct.unpack_from('<H',raw,pe+20)[0]+40*n
  _,rva,size,off=struct.unpack_from('<IIII',raw,p+8);sections.append((rva,size,off))
 def offset(address):
  for rva,size,off in sections:
   if base+rva<=address<base+rva+size:return off+address-base-rva
  raise ValueError('Initializer outside PE sections')
 proofs={}
 conversion=struct.unpack_from('<f',raw,offset(0x00b41800))[0]
 assert abs(conversion-2**-.5)<1e-7
 for enum,(name,address,bits) in SPECS.items():
  p=offset(address);code=raw[p:p+31]
  # PUSH1; PUSHliteral; PUSHglobaladdress; PUSHsource; PUSHname; CALLregister.
  assert code[:3]==bytes.fromhex('6a0168') and struct.unpack_from('<I',code,3)[0]==bits
  assert code[7]==0x68 and code[12]==0x68 and code[17]==0x68 and code[22]==0xe8
  name_va=struct.unpack_from('<I',code,18)[0]
  assert raw[offset(name_va):].split(b'\0',1)[0].decode('ascii')==name
  proofs[enum]={'global':name,'initializer':f'{address:08x}','literalBits':f'{bits:08x}',
   'compiledDefault':struct.unpack('<f',struct.pack('<I',bits))[0],
   'initializerBytes':code.hex(),'source':'native PE initializer; Ghidra native-scale-defaults.c'}
 return raw,proofs,conversion

def run():
 raw,defaults,conversion=binary_literal_proofs()
 f32=lambda value:struct.unpack('<f',struct.pack('<f',value))[0]
 config_refs=[]
 for name in ['config.cfg','bldconfig.cfg','myconfig_sim.cfg','myconfig.cfg']:
  path=INSTALL/name
  if not path.exists():continue
  data=path.read_bytes();text=clean_comments(data.decode('cp1252'))
  for enum,spec in SPECS.items():
   matches=list(re.finditer(r'\b'+re.escape(spec[0])+r'\s*=\s*([-+]?\d*\.?\d+(?:[eE][-+]?\d+)?)',text,re.I))
   for match in matches:
    value=float(match[1]);assert value>0
    ref={'file':name,'sha256':hashlib.sha256(data).hexdigest(),'line':text.count('\n',0,match.start())+1,'global':spec[0],'value':value}
    config_refs.append(ref);defaults[enum]['configOverride']=ref
 for definition in defaults.values():definition['selectedValue']=definition.get('configOverride',{}).get('value',definition['compiledDefault'])
 db=Database(INSTALL/'zips/db.zip')
 try:
  # Resources use CompositeModel→StateDatabase and explicit model lists.
  # Farms' multi-child plot placement is deliberately not flattened here.
  for path in db.zip.namelist():
   if not(path.lower().startswith('db/units/') and path.lower().endswith('.ddf')):continue
   text=clean_comments(db.read(path))
   for kind in ['CompositeModel','StateDatabase']:
    for name,body,body_start,start in declarations(text,kind):
     key=(kind.lower(),name.lower());assert key not in db.blocks
     db.blocks[key]={'kind':kind,'name':name,'body':body,'path':path,'body_start':body_start,'line':text.count('\n',0,start)+1,'text':text}
  resolved={}
  def resolve(name,stack=()):
   key=name.lower()
   if key in resolved:return resolved[key]
   assert key not in stack,'Cyclic UnitModel inheritance'
   block=db.block('UnitModel',name) or db.block('CompositeModel',name)
   if not block:raise ValueError('Unknown UnitModel '+name)
   parent=scalar(block['body'],'Parent') if block['kind']=='UnitModel' else None
   result=dict(resolve(parent,stack+(key,))) if parent else {'scaleType':'kScaleIdentity','forceScale':None,'defaultModel':None,'defaultModels':[],'chain':[]}
   result['chain']=[*result['chain'],db.block_ref(block)]
   #00602210 takes the current UnitModel's ForceScale(+0xc) directly;
   #only ScaleType is resolved through the native parent-search helper.
   result['forceScale']=None
   for field,target in [('ScaleType','scaleType'),('ForceScale','forceScale'),('DefaultModel','defaultModel')]:
    own=scalar(block['body'],field)
    if own is not None:
     if field=='ScaleType' and own=='kScaleIdentity' and parent:continue
     result[target]=float(own) if field=='ForceScale' else own
   model_list=re.search(r'\bDefaultModelList\s*=\s*\[([^]]*)\]',block['body'],re.I)
   if scalar(block['body'],'DefaultModel'):result['defaultModels']=[scalar(block['body'],'DefaultModel')]
   elif model_list:result['defaultModels']=re.findall(r'[\w.-]+\.nif',model_list[1],re.I)
   state_name=scalar(block['body'],'StateDatabaseName')
   if state_name:
    state=db.block('StateDatabase',state_name)
    if state and scalar(state['body'],'DefaultModel'):
     result['defaultModels']=[scalar(state['body'],'DefaultModel')]
     result['chain']=[*result['chain'],db.block_ref(state)]
   resolved[key]=result;return result
  candidates=collections.defaultdict(list)
  for block in db.blocks.values():
   if block['kind'] not in ['UnitModel','CompositeModel']:continue
   value=resolve(block['name'])
   for model in value['defaultModels']:
    if model and model.lower()!='null.nif':candidates[model_stem(model)].append(block['name'])
  manifest=json.loads((ROOT/'docs/local-original/models/manifest.json').read_text(encoding='utf8'))
  unit_audit=json.loads((ROOT/'docs/ee2-units.json').read_text(encoding='utf8'))
  variants={};counts=collections.Counter();used_sources={}
  for row in manifest['models']:
   model=json.loads((ROOT/'docs'/row['url']).read_text(encoding='utf8'));stem=model_stem(model['source']['entry']);names=candidates.get(stem,[])
   named=[]
   for binding in [row.get(k,{}) for k in ['databaseVisualBinding','aircraftVisualBinding','resourceVisualBinding','nativeMotionBinding','progressionBinding']]:
    name=binding.get('visualDefinition') or binding.get('sourceVisual')
    if name and name in names:named.append(name)
   for native in unit_audit['rows']:
    if native['id']==row['id'] and native['epoch']<=row.get('epoch',0)<native.get('until',15) and native['source_visual'] in names:named.append(native['source_visual'])
   if not named:
    # Old manifest base rows have epoch0 even when the role starts in E11+.
    # A matching audited native visual remains the source of that exact mesh.
    named=[native['source_visual'] for native in unit_audit['rows'] if native['id']==row['id'] and native['source_visual'] in names]
   selection=sorted(set(named or names),key=str.lower)
   options=[]
   for name in selection:
    value=resolve(name);scale_type=value['scaleType'];force=value['forceScale']
    if force is not None and force>0:scale=f32(f32(force)*conversion)
    elif scale_type in defaults:scale=f32(f32(defaults[scale_type]['selectedValue'])*conversion)
    else:continue
    options.append((name,value,scale))
   key=f"{row['id']}:{row.get('epoch',0)}"
   binding={'url':row['url'],'sourceSha256':model['source']['sha256'],'sourceEntry':model['source']['entry'],'verified':False}
   if options and len({(value['scaleType'],scale) for _,value,scale in options})==1:
    name,value,scale=options[0]
    binding.update(verified=True,visual=name,compatibleVisuals=[option[0] for option in options],scaleType=value['scaleType'],forceScale=value['forceScale'],sourceScale=scale,
     selection='Exact current visual source' if named else 'All matching declared model choices agree on class and scale',
     inheritance=value['chain'],sourceBounds=model['sourceBounds'],normalization=model['normalization'])
    counts[value['scaleType']]+=1
    for ref in value['chain']:used_sources[ref['entry']]=ref['sha256']
   else:
    binding['reason']='No unambiguous declared UnitModel scale binding; retain current adapter'
    counts['unresolved']+=1
   variants[key]=binding
  output={'format':'ee2-native-visual-scale-v1','engineSha256':hashlib.sha256(raw).hexdigest(),
   'scope':'Native compiled category defaults, common renderer conversion005f8040/00602210/00602860, active assignments in the four startup config files, BASE UnitModel ScaleType inheritance, and CompositeModel/StateDatabase or DefaultModelList bindings. ForceScale is the current visual declaration only. Later launcher hooks, per-state scale and procedural nonuniform variation remain outside this profile.',
   'unitConversion':{'address':'00b41800','value':conversion,'classGetter':'005f8040','overrideApplication':'00602210'},
   'classDefaults':defaults,'configAssignments':config_refs,'databaseSources':used_sources,'counts':dict(counts),'variants':variants}
  path=ROOT/'docs/local-original/model-scales.json';temporary=path.with_suffix('.json.tmp')
  temporary.write_text(json.dumps(output,indent=2)+'\n',encoding='utf8');temporary.replace(path)
  print(json.dumps({'path':str(path),'counts':dict(counts),'variants':len(variants),'configAssignments':len(config_refs)},indent=2))
 finally:db.close()

if __name__=='__main__':run()
