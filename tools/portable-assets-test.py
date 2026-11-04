"""Audit the portable game's runtime URLs using exact, case-sensitive paths.

Uses only Python's standard library. No original installation, ZIP, EXE, Pillow,
Node or Windows-only tool is required. Source/provenance metadata is not treated
as a runtime path. After unpacking, run with --docs <unpacked docs directory>
--compare <original docs directory> to also prove complete media-copy identity.
"""
from __future__ import annotations
import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import posixpath
import re
import tempfile
import unicodedata
import unittest
from urllib.parse import unquote, urlsplit

PROJECT=Path(__file__).resolve().parents[1]
DEFAULT_REPORT=PROJECT.parent/'EE2-Research/analysis/portable-assets-check.json'
DRIVE=re.compile(r'^[A-Za-z]:')
RAW_ORIGINAL={'.exe','.dll','.zip','.texcache','.pcpatch','.nifcache','.nif','.kf'}

def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for data in iter(lambda:stream.read(1024*1024),b''):h.update(data)
    return h.hexdigest()

def inventory(root):
    return {p.relative_to(root).as_posix():p for p in root.rglob('*') if p.is_file()}

class AssetAudit:
    def __init__(self,docs):
        self.docs=Path(docs).resolve()
        if not self.docs.is_dir():raise ValueError(f'No docs directory: {self.docs}')
        self.files=inventory(self.docs)
        self.directories={p.relative_to(self.docs).as_posix() for p in self.docs.rglob('*') if p.is_dir()}
        self.fold=defaultdict(list)
        for path in [*self.files,*self.directories]:self.fold[path.casefold()].append(path)
        self.errors=[];self.warnings=[];self.refs=[];self.json_cache={};self.hash_cache={}
        self.metadata_absolute=[];self.network_endpoints=[];self.stats=Counter()
        for names in self.fold.values():
            if len(names)>1:self.error('case-collision',names)
        normalized=defaultdict(list)
        for name in self.files:normalized[unicodedata.normalize('NFC',name).casefold()].append(name)
        for names in normalized.values():
            if len(names)>1 and len(set(names))>1:self.error('unicode/case-collision',names)
        for name,path in self.files.items():
            try:path.resolve().relative_to(self.docs)
            except ValueError:self.error('symlink-leaves-docs',name)

    def error(self,kind,detail,source=None):
        record={'kind':kind,'detail':detail}
        if source is not None:record['source']=source
        self.errors.append(record)

    def ref(self,url,source,group,base='',directory=False):
        if not isinstance(url,str) or not url:
            self.error('empty/non-string-runtime-url',repr(url),source);return None
        self.stats[group]+=1
        if '\\' in url or DRIVE.match(url) or url.startswith(('/', '//')):
            self.error('absolute/windows-runtime-url',url,source);return None
        parsed=urlsplit(url)
        if parsed.scheme or parsed.netloc:
            self.error('external/nonrelative-runtime-url',url,source);return None
        decoded=unquote(parsed.path)
        if '\\' in decoded or '\x00' in decoded or DRIVE.match(decoded) or decoded.startswith('/'):
            self.error('unsafe-decoded-runtime-url',url,source);return None
        name=posixpath.normpath(posixpath.join(base,decoded))
        if name in ('','.') and directory:return ''
        if name=='..' or name.startswith('../'):
            self.error('runtime-path-leaves-docs',url,source);return None
        expected=self.directories if directory else self.files
        self.refs.append({'url':url,'resolved':name,'source':source,'group':group,'directory':directory})
        if name not in expected:
            candidates=self.fold.get(name.casefold(),[])
            self.error('case-mismatch' if candidates else 'missing-runtime-file',
                       {'url':url,'resolved':name,'caseCandidates':candidates},source)
            return None
        if not directory and Path(name).suffix.lower() in RAW_ORIGINAL:
            self.error('raw-original-runtime-dependency',name,source)
        return name

    def read_json(self,name):
        if name in self.json_cache:return self.json_cache[name]
        if name not in self.files:
            self.error('missing-required-manifest',name);return {}
        data=json.loads(self.files[name].read_text(encoding='utf8'))
        self.json_cache[name]=data
        # This is informational evidence that provenance was kept separate from
        # bindings, not an instruction to rewrite historical GOG/source paths.
        def metadata(value,label):
            if isinstance(value,dict):
                for key,item in value.items():metadata(item,label+'.'+key)
            elif isinstance(value,list):
                if value and isinstance(value[0],(int,float)):return
                for i,item in enumerate(value):metadata(item,label+f'[{i}]')
            elif isinstance(value,str) and (DRIVE.match(value) or value.startswith('file:')):
                self.metadata_absolute.append({'source':label,'value':value})
        if isinstance(data,dict) and data.get('format')=='ee2-original-mesh-v1':
            for key in ('source','sourceTextures','animationSources','limitations'):metadata(data.get(key),name+'.'+key)
        else:metadata(data,name)
        return data

    def digest(self,name):
        if name not in self.hash_cache:self.hash_cache[name]=sha(self.files[name])
        return self.hash_cache[name]

    def hash_ref(self,record,source,group='asset-audit',base=''):
        if not isinstance(record,dict) or not record.get('url'):return
        name=self.ref(record['url'],source,group,base)
        expected=record.get('png_sha256')
        if expected and name and self.digest(name)!=expected:self.error('png-hash-mismatch',name,source)

    def local_urls(self,value,source,group):
        """Additional exported media references, without guessing source paths."""
        if isinstance(value,dict):
            if isinstance(value.get('url'),str) and value['url'].startswith('local-original/'):
                self.hash_ref(value,source,group)
            for key,item in value.items():
                if key=='url':continue
                if isinstance(key,str) and key.startswith('local-original/'):self.ref(key,source+'.<key>',group)
                self.local_urls(item,source+'.'+key,group)
        elif isinstance(value,list):
            if value and isinstance(value[0],(int,float)):return
            for i,item in enumerate(value):self.local_urls(item,source+f'[{i}]',group)
        elif isinstance(value,str) and value.startswith('local-original/'):
            self.ref(value,source,group)

    def check_models(self):
        manifest_name='local-original/models/manifest.json'
        self.ref(manifest_name,'original-renderer.js','model-manifest')
        manifest=self.read_json(manifest_name)
        models=manifest.get('models',[])
        self.stats['modelBindings']=len(models)
        loaded=set()
        for i,row in enumerate(models):
            if row.get('verified') is False:continue
            for field in ('url','lodFarUrl'):
                if field=='lodFarUrl' and not row.get(field):continue
                name=self.ref(row.get(field),f'{manifest_name}.models[{i}].{field}','near-model' if field=='url' else 'far-model')
                if not name or name in loaded:continue
                loaded.add(name);model=self.read_json(name)
                for j,part in enumerate(model.get('parts',[])):
                    # Identical precedence to the renderer's material URL choice.
                    url=part.get('textureUrl') or part.get('texture') or manifest.get('textures',{}).get(str(part.get('textureSource','')).lower())
                    if url:self.ref(url,f'{name}.parts[{j}].material','model-material')
                    elif part.get('textureSource'):
                        self.warnings.append({'kind':'source-texture-with-white-material-fallback','source':f'{name}.parts[{j}]','textureSource':part['textureSource']})
                self.local_urls(model.get('sourceTextures',[]),name+'.sourceTextures','model-texture-audit')
                # Animation arrays are irrelevant to URL resolution and can be
                # hundreds of MB. Retain only the metadata needed by the later
                # auxiliary scan, so this audit also fits smaller Mac machines.
                self.json_cache[name]={'format':model.get('format'),'source':model.get('source'),
                                       'sourceTextures':model.get('sourceTextures',[])}
        self.stats['uniqueBoundModelJSON']=len(loaded)
        for i,row in enumerate(manifest.get('epochIconBindings',[])):
            self.hash_ref(row,f'{manifest_name}.epochIconBindings[{i}]','epoch-icon')
        self.stats['epochIconBindings']=len(manifest.get('epochIconBindings',[]))
        self.local_urls(manifest.get('textures',{}),manifest_name+'.textures','model-material-map')

    def check_textures_and_ground(self):
        name='local-original/textures.json';self.ref(name,'original-assets.js','texture-manifest');data=self.read_json(name)
        for group,mapping in data.items():
            if isinstance(mapping,dict):
                for key,url in mapping.items():self.ref(url,f'{name}.{group}.{key}','texture-'+group)
        for name in ('local-original/textures-manifest.json','local-original/ground-decals.json','local-original/ground-decals-manifest.json'):
            self.ref(name,'exported-local-asset-map','asset-manifest');self.local_urls(self.read_json(name),name,'texture/ground-audit')
        ground=self.read_json('local-original/ground-decals.json')
        self.stats['groundBindings']=len(ground.get('entries',[]))
        self.stats['groundUnresolvedBindings']=len(ground.get('unresolvedBindings',[]))
        for section in ('entries','unresolvedBindings'):
            for i,row in enumerate(ground.get(section,[])):
                for state,states in [('idle',row),('death',row.get('death',{}))]:
                    for group in ('rural','urban'):
                        for j,sprite in enumerate(states.get(group,[])):
                            self.ref(sprite.get('url'),f'ground.{section}[{i}].{state}.{group}[{j}]','ground-'+state)

    def check_teams(self):
        name='local-original/team-textures/manifest.json';self.ref(name,'original-renderer.js','team-manifest');data=self.read_json(name)
        bindings=data.get('texture_bindings',{})
        for url,key in bindings.items():
            self.ref(url,name+'.texture_bindings.<key>','team-base-binding')
            if key not in data.get('textures',{}):self.error('missing-team-family',key,name)
        for family,record in data.get('textures',{}).items():
            for url in record.get('base_texture_urls',[]):self.ref(url,f'{name}.textures.{family}.base_texture_urls','team-base')
            for letter,variant in record.get('variants',{}).items():
                self.hash_ref(variant,f'{name}.textures.{family}.variants.{letter}','team-variant')
        self.stats['teamFamilies']=len(data.get('textures',{}))
        self.stats['teamVariants']=sum(len(t.get('variants',{})) for t in data.get('textures',{}).values())

    def check_audio(self):
        name='local-original/audio/manifest.json';base='local-original/audio'
        self.ref(name,'original-audio.js','audio-manifest');data=self.read_json(name)
        assets=data.get('assets',{})
        for key,asset in assets.items():
            path=self.ref(asset.get('file'),f'{name}.assets.{key}.file','audio-asset',base)
            if path:
                if self.files[path].stat().st_size!=asset.get('bytes'):self.error('audio-size-mismatch',path,name)
                if self.digest(path)!=asset.get('sha256'):self.error('audio-hash-mismatch',path,name)
        def clip(file,label):
            self.ref(file,label,'audio-playback',base)
            if not isinstance(file,str) or file.lower() not in assets:self.error('playback-clip-missing-from-assets',file,label)
        for kind,voices in [('voices',data.get('voices',{}).values()),('voiceVariants',data.get('voiceVariants',[]))]:
            for i,voice in enumerate(voices):
                for action in ('select','order','build','attack'):
                    for j,file in enumerate(voice.get(action,[])):clip(file,f'audio.{kind}[{i}].{action}[{j}]')
        for group in ('effects','buildingSelections'):
            for key,record in data.get(group,{}).items():clip(record if isinstance(record,str) else record.get('file'),f'audio.{group}.{key}')
        for group,files in data.get('music',{}).items():
            for i,file in enumerate(files):clip(file,f'audio.music.{group}[{i}]')
        ambient=data.get('ambient',{})
        if ambient.get('loop'):clip(ambient['loop'],'audio.ambient.loop')
        for i,file in enumerate(ambient.get('birds',[])):clip(file,f'audio.ambient.birds[{i}]')
        self.stats['audioAssets']=len(assets)

    def check_static_runtime(self):
        audit=self
        class Markup(HTMLParser):
            def __init__(self,name):super().__init__();self.name=name
            def handle_starttag(self,tag,attrs):
                attrs=dict(attrs)
                for key in ('src','poster'):
                    if attrs.get(key):static_ref(attrs[key],self.name,f'html-{key}')
                if tag=='link' and attrs.get('href'):static_ref(attrs['href'],self.name,'html-link')
        def static_ref(url,source,group):
            if '${' in url or url.startswith(('#','data:','blob:')):return
            self.ref(url,source,group,posixpath.dirname(source),directory=url.endswith('/'))
        for name,path in self.files.items():
            if Path(name).suffix not in ('.js','.html','.css') or name.startswith('local-original/'):continue
            text=path.read_text(encoding='utf8')
            if name.endswith('.html'):Markup(name).feed(text)
            # Literal module imports, including version querystrings.
            for match in re.finditer(r'\b(?:import|export)\s+(?:[^;\n]*?\s+from\s*)?[\'"]([^\'"\n]+)[\'"]',text):
                static_ref(match[1],name,'module-import')
            for match in re.finditer(r'new\s+URL\s*\(([^;\n]*?),\s*import\.meta\.url\s*\)',text):
                for quoted in re.finditer(r'[\'"]([^\'"\n]+)[\'"]',match[1]):static_ref(quoted[1],name,'module-url')
            for match in re.finditer(r'\b(?:fetch|getJSON|json)\s*\(\s*[\'"]([^\'"\n]+)[\'"]',text):
                static_ref(match[1],name,'literal-fetch')
            for match in re.finditer(r'url\(\s*[\'"]?([^\)\'"\n]+)',text):
                url=match[1].strip()
                if '\\' not in url:static_ref(url,name,'css-url')
            # A literal drive path in executable source is not a JSON provenance
            # field. Nothing should fetch or assign a Windows installation path.
            for match in re.finditer(r'[\'"]([A-Za-z]:[\\/][^\'"\n]*)[\'"]',text):
                self.error('windows-drive-literal-in-runtime-source',match[1],name)
            self.network_endpoints.extend({'source':name,'endpoint':m[0]} for m in re.finditer(r'stun:[A-Za-z0-9.:-]+',text))
        for name in ('index.html','main.js','openepoch-ee2.wasm','openepoch.wasm'):
            self.ref(name,'portable-startup-contract','startup')

    def compare_original_tree(self,other):
        other=Path(other).resolve();reference=inventory(other)
        wanted={p for p in reference if p.startswith('local-original/')}
        actual={p for p in self.files if p.startswith('local-original/')}
        comparison={'referenceDocs':str(other),'referenceFileCount':len(wanted),'packageFileCount':len(actual),'missing':sorted(wanted-actual),'extra':sorted(actual-wanted),'changed':[]}
        for name in sorted(wanted&actual):
            if self.digest(name)!=sha(reference[name]):comparison['changed'].append(name)
        if comparison['missing'] or comparison['extra'] or comparison['changed']:self.error('local-original-tree-not-identical',comparison)
        return comparison

    def run(self,compare=None):
        self.check_models();self.check_textures_and_ground();self.check_teams();self.check_audio();self.check_static_runtime()
        # All local-original JSONs carry auxiliary provenance/preview references;
        # validate those too without interpreting source .tga/.nif/cache paths.
        for name in self.files:
            if name.startswith('local-original/') and name.endswith('.json'):
                data=self.read_json(name)
                if isinstance(data,dict) and data.get('format')=='ee2-original-mesh-v1':continue
                self.local_urls(data,name,'auxiliary-exported-map')
        comparison=self.compare_original_tree(compare) if compare else None
        targets={ref['resolved'] for ref in self.refs if not ref['directory'] and ref['resolved'] in self.files}
        asset_inventory=[]
        for name in sorted(targets):asset_inventory.append({'path':name,'bytes':self.files[name].stat().st_size,'sha256':self.digest(name)})
        raw=[name for name in self.files if name.startswith('local-original/') and Path(name).suffix.lower() in RAW_ORIGINAL]
        if raw:self.error('raw-original-files-in-runtime-tree',raw)
        return {'status':'passed' if not self.errors else 'failed','docs':str(self.docs),'createdUTC':datetime.now(timezone.utc).isoformat(),
                'method':'Exact relative-path inventory, including every directory/file spelling; Windows case-insensitive exists() cannot satisfy a mismatched URL.',
                'fileCount':len(self.files),'localOriginalFileCount':sum(n.startswith('local-original/') for n in self.files),
                'runtimeReferenceCount':len(self.refs),'uniqueRuntimeTargets':len(targets),'counts':dict(self.stats),
                'errors':self.errors,'warnings':self.warnings,'sourceMetadataAbsolutePathsIgnored':len(self.metadata_absolute),
                'metadataPolicy':'Original GOG/Windows paths inside provenance fields remain unchanged; they are not runtime URL bindings.',
                'networkServices':self.network_endpoints,'rawOriginalRuntimeDependencies':raw,'comparison':comparison,
                'assets':asset_inventory,'references':self.refs}

class PathPolicyTests(unittest.TestCase):
    def setUp(self):
        self.scratch=(Path.cwd()/'work').resolve();self.scratch.mkdir(parents=True,exist_ok=True)
        self.temp=tempfile.TemporaryDirectory(dir=self.scratch,prefix='portable-path-test-');self.root=Path(self.temp.name)
        self.root.resolve().relative_to(self.scratch)
        (self.root/'Icons').mkdir();(self.root/'Icons/Tree.png').write_bytes(b'pixel-fixture')
    def tearDown(self):
        self.root.resolve().relative_to(self.scratch) # verify cleanup stays in the explicit scratch root
        self.temp.cleanup()
    def test_exact_path_and_query(self):
        a=AssetAudit(self.root);self.assertEqual(a.ref('Icons/Tree.png?v=abc#x','test','fixture'),'Icons/Tree.png');self.assertFalse(a.errors)
    def test_windows_cannot_hide_wrong_case(self):
        a=AssetAudit(self.root);self.assertIsNone(a.ref('icons/tree.png','test','fixture'));self.assertEqual(a.errors[0]['kind'],'case-mismatch')
    def test_reject_drive_file_external_and_backslash(self):
        for value in ('C:/Game/Tree.png',r'C:\Game\Tree.png','file:///C:/Tree.png','https://example.com/Tree.png',r'Icons\Tree.png'):
            a=AssetAudit(self.root);self.assertIsNone(a.ref(value,'test','fixture'));self.assertTrue(a.errors)
    def test_reject_root_escape_and_encoded_escape(self):
        for value in ('../Tree.png','%2e%2e/Tree.png','/%2fTree.png','Icons/%00.png'):
            a=AssetAudit(self.root);self.assertIsNone(a.ref(value,'test','fixture'));self.assertTrue(a.errors)
    def test_audio_base_and_safe_relative_module_parent(self):
        a=AssetAudit(self.root);self.assertEqual(a.ref('../Icons/Tree.png','test','fixture','module'),'Icons/Tree.png');self.assertFalse(a.errors)
    def test_missing_file_is_not_a_case_mismatch(self):
        a=AssetAudit(self.root);a.ref('Icons/Missing.png','test','fixture');self.assertEqual(a.errors[0]['kind'],'missing-runtime-file')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--docs',type=Path,default=PROJECT/'docs');parser.add_argument('--compare',type=Path)
    parser.add_argument('--report',type=Path,default=DEFAULT_REPORT);parser.add_argument('--label',default='workspace')
    parser.add_argument('--self-test',action='store_true');args=parser.parse_args()
    if args.self_test:
        result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(PathPolicyTests))
        if not result.wasSuccessful():raise SystemExit(1)
    audit=AssetAudit(args.docs);run=audit.run(args.compare);run['label']=args.label
    run['checkerSha256']=sha(Path(__file__).resolve())
    report={'format':'portable-assets-check-v1','scope':'Portable original media/runtime URL audit only; no original archives/EXE or original installation needed. Does not claim a physical Mac/Safari or GPU execution test.','runs':[]}
    if args.report.is_file():
        previous=json.loads(args.report.read_text(encoding='utf8'))
        if previous.get('format')==report['format']:report['runs']=[r for r in previous.get('runs',[]) if r.get('label')!=args.label]
    report['runs'].append(run);report['status']='passed' if all(r['status']=='passed' for r in report['runs']) else 'failed'
    report['latestLabel']=args.label;args.report.parent.mkdir(parents=True,exist_ok=True);args.report.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    print(f"{run['status'].upper()}: {run['fileCount']} files, {run['runtimeReferenceCount']} references, {run['uniqueRuntimeTargets']} exact-case targets, {len(run['errors'])} errors; report {args.report}")
    for error in run['errors'][:25]:print(json.dumps(error,ensure_ascii=False))
    if run['errors']:raise SystemExit(1)

if __name__=='__main__':main()
