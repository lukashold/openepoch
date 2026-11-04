"""Compare effective original font-table requests with shipped Western caches."""
from pathlib import Path
import hashlib,json,re,sys,zipfile,os
from font_cache import HERE,INSTALL,OUT,REPORT,PROJECT,WORKSPACE
ROOT=WORKSPACE;sys.path.insert(0,str(PROJECT/'tools'))
from extract_ee2_diplomacy import bracket_csv

resolution=json.loads(Path(os.environ.get('OPEN_EPOCH_FONT_RESOLVED',ROOT/'work/up-resolved-sources/effective-db-sources.json')).read_text());origin=resolution['effectiveByPath']['db/ui/dbfont.csv']['source']
with zipfile.ZipFile(INSTALL/origin['physical']) as archive:raw=archive.read(origin['member'])
assert hashlib.sha256(raw).hexdigest()==origin['sha256']
fontnames=['NAME','FONTNAME','ASIAN','SIZE','S.C.SIZE','FLAGS','FONT SIZE','W_INITSET','W_MINTXSIZE','W_MAXTXSIZE','A_INITSET','A_MINTXSIZE','A_MAXTXSIZE']
manifest=json.loads((OUT/'manifest.json').read_text(encoding='utf8'));caches=[]
for key,row in manifest['fonts'].items():
    if not key.startswith('fv8W'):continue
    match=re.fullmatch(r'fv8W_(.+)_(\d+)_(\d+)_(.+)_(\d+)_(\d+)x(\d+)',key)
    assert match,key
    face,size,small,options,index,width,height=match.groups()
    caches.append({'key':key,'face':face.replace('_',' '),'size':int(size),'smallSize':int(small),'flags':row['headerObjectOffsets']['0x4c'],'initSet':options.split('_')[-1],'minAtlas':[int(width),int(height)]})
flags={'Bold':1,'Italic':2,'Underline':4,'Outline':8,'SmallCaps':16,'AntiAlias':32,'Kern':64}
requests=[]
for line,text in enumerate(raw.decode('utf-8-sig').splitlines(),1):
    if not text.strip() or text.lstrip().startswith('//'):continue
    values=bracket_csv(text);assert len(values)==13,(line,len(values));row=dict(zip(fontnames,values));faces=re.findall(r'"([^"]+)"',row['FONTNAME']);sizes=list(map(int,re.findall(r'\d+',row['SIZE'])));small=list(map(int,re.findall(r'\d+',row['S.C.SIZE'])));mask=sum(value for flag,value in flags.items() if re.search(r'\b'+flag+r'\b',row['FLAGS']));atlas=list(map(int,re.findall(r'\d+',row['W_MINTXSIZE'])));assert len(sizes)==6
    for tier,size in enumerate(sizes):
        smallSize=small[tier] if mask&16 else 0
        matched=[q['key'] for q in caches if q['face'] in faces and q['size']==size and q['smallSize']==smallSize and q['flags']==mask and q['initSet']==row['W_INITSET'].lower() and q['minAtlas']==atlas]
        requests.append({'font':row['NAME'],'sourceLine':line,'resolutionTier':tier,'faces':faces,'size':size,'smallSize':smallSize,'flags':mask,'initSet':row['W_INITSET'],'minAtlas':atlas,'exactShippedCacheMatches':matched})
report={'format':'native6-font-table-cache-bindings-v1','source':origin,'requestedWesternVariants':len(requests),'exactCachedRequests':sum(bool(q['exactShippedCacheMatches']) for q in requests),'uncachedRequests':sum(not q['exactShippedCacheMatches'] for q in requests),'requests':requests,'limits':['Effective installed db_156_autofonts table is used, not an assumed historical BASE font-size table.','No nearest-size substitution: a missing request needs native GDI reconstruction.','Shared source rows repeat the same size in distinct resolution tiers.']}
(REPORT/'native6-font-table-cache-bindings.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(json.dumps({k:v for k,v in report.items() if k not in ('requests','source')},indent=2))
