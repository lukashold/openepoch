"""Extract the complete declared base technology tree without inventing runtime rules.
Patch archives are compared independently; this does not infer load precedence.
"""
import csv,hashlib,io,json,pathlib,re,sys,zipfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
INSTALL=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else r'C:\Program Files (x86)\GOG.com\Empire Earth II')
TABLE='db/TechTree/dbtechtreenode.csv'
RESOURCES=['FOOD','WOOD','STONE','GOLD','TIN','IRON','SALTPETER','OIL','URANIUM']
def decode(raw):
    try:return raw.decode('utf-8-sig'),'utf-8'
    except UnicodeDecodeError:return raw.decode('cp1252'),'cp1252'
def table(raw,path):
    text,encoding=decode(raw);lines=text.replace('\r','').splitlines()
    head=next(csv.reader([lines[0].removeprefix('//').strip()]))
    rows=[]
    for i,line in enumerate(lines[1:],2):
        if not line.strip() or line.lstrip().startswith('//'):continue
        vals=next(csv.reader([line]))
        if len(vals)!=len(head):raise ValueError(f'{path}:{i}: invalid width {len(vals)}')
        row=dict(zip(head,vals));row['source']=f'db.zip!{path}:{i}';rows.append(row)
    return rows,encoding

def run():
    archive=INSTALL/'zips/db.zip';textmap={};text_sources={};encodings={}
    with zipfile.ZipFile(archive) as z:
        paths={p.lower():p for p in z.namelist()}
        for path in z.namelist():
            if not path.lower().startswith('db/text/'):continue
            text,encoding=decode(z.read(path));encodings[path]=encoding
            for i,line in enumerate(text.replace('\r','').splitlines(),1):
                if not line.strip() or line.lstrip().startswith('//'):continue
                row=next(csv.reader([line]))
                if len(row)==2:
                    textmap[row[0]]=row[1].strip('"');text_sources[row[0]]=f'db.zip!{path}:{i}'
        nodes,encoding=table(z.read(paths[TABLE.lower()]),TABLE);encodings[TABLE]=encoding
        for node in nodes:
            for key in ['EPOCH','SLOT','MENU','ROW','COLUMN','TIME','TECHPTS']+RESOURCES:
                node[key]=float(node[key]) if node[key] else 0
                if node[key].is_integer():node[key]=int(node[key])
            key=node['DISPLAYNAME'];node['label']=textmap.get(key,key or node['NAME'])
            node['label_source']=text_sources.get(key,'')
            node['implemented']=False
        epochs=[]
        for i in range(1,16):
            n=next(n for n in nodes if n['NAME']==f'Main{i}')
            key=f'tx_enum_eTechTreeEpoch_{i}'
            epochs.append({'id':i,'name':textmap[key],'source':text_sources[key],
                'advance':n,'technology_count':sum(n['EPOCH']==i and n['BRANCH'] in ('Economic','Military','Imperial') for n in nodes)})
        definitions=[]
        for path in z.namelist():
            if path.lower().startswith('db/units/') and path.lower().endswith('.ddf'):
                t,_=decode(z.read(path))
                # Match only active declarations; keep source line numbers by blanking comments.
                t=re.sub(r'//[^\n]*|/\*.*?\*/',lambda m:'\n'*m[0].count('\n'),t,flags=re.S)
                for m in re.finditer(r'\bUnitType\s+(\w+)\s*\{',t):
                    definitions.append({'id':m[1],'source':f'db.zip!{path}','implemented':False})
        unit_rows=json.loads((ROOT/'docs/ee2-reference.json').read_text(encoding='utf-8'))['rows']
        for row in unit_rows:
            key=row['DISPLAYNAME'];row['label']=textmap.get(key,key or row['UNIT'])
    base_by_id={n['NAME']:n for n in nodes}
    overlays=[]
    for p in sorted((INSTALL/'zips').glob('db_*.zip')):
        with zipfile.ZipFile(p) as z:
            for path in z.namelist():
                if not re.search(r'/dbtechtreenod[^/]*\.csv$',path,re.I):continue
                raw=z.read(path)
                try:rows,enc=table(raw,path)
                except ValueError as error:
                    overlays.append({'archive':p.name,'path':path,'sha256':hashlib.sha256(raw).hexdigest(),'parse_error':str(error),'effective_precedence':'unverified'})
                    continue
                different=[]
                for row in rows:
                    old=base_by_id.get(row['NAME'])
                    changes=[k for k,v in row.items() if k!='source' and (old is None or str(old.get(k,''))!=v)]
                    if changes:different.append({'id':row['NAME'],'fields':changes})
                overlays.append({'archive':p.name,'path':path,'sha256':hashlib.sha256(raw).hexdigest(),'encoding':enc,'nodes':len(rows),'different':different,'effective_precedence':'unverified'})
    data={'scope':'Declared BASE db.zip data; not a playable implementation. Patch order and runtime dependencies remain unverified.',
          'source':str(archive),'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'resources':RESOURCES,
          'epochs':epochs,'nodes':nodes,'unit_upgrades':unit_rows,'unit_definitions':definitions,
          'text_encodings':encodings,'patch_comparison':overlays}
    (ROOT/'docs/ee2-catalog.json').write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
    print(f'{len(epochs)} epochs, {len(nodes)} nodes, {sum(e["technology_count"] for e in epochs)} technologies, {len(definitions)} declared unit types, {len(unit_rows)} upgrades; {len(overlays)} patch tables compared')
    return data
if __name__=='__main__':run()
