"""Audit the portable game's runtime URLs using exact, case-sensitive paths.

Uses only Python's standard library. No original installation, ZIP, EXE, Pillow,
Node or Windows-only tool is required. Source/provenance metadata is not treated
as a runtime path. After unpacking, run with --docs <unpacked docs directory>
--compare <original docs directory> to also prove complete media-copy identity.
"""
from __future__ import annotations
import argparse
import base64
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
from html.parser import HTMLParser
import json
import math
from pathlib import Path, PurePosixPath
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
    def __init__(self,docs,overlay_root=None,overlay_names=()):
        self.docs=Path(docs).resolve()
        if not self.docs.is_dir():raise ValueError(f'No docs directory: {self.docs}')
        self.files=inventory(self.docs)
        self.directories={p.relative_to(self.docs).as_posix() for p in self.docs.rglob('*') if p.is_dir()}
        overlay_root=Path(overlay_root).resolve() if overlay_root is not None else self.docs
        for name in overlay_names:
            relative=PurePosixPath(name)
            if (not relative.parts or relative.is_absolute() or '..' in relative.parts or
                    ':' in name or '\\' in name or relative.as_posix()!=name):
                raise ValueError(f'Invalid docs overlay name: {name}')
            path=overlay_root/relative
            if not path.is_file() or path.is_symlink() or path.resolve()!=path:
                raise ValueError(f'Invalid docs overlay file: {path}')
            self.files[name]=path
            self.directories.update(parent.as_posix() for parent in relative.parents if parent.parts)
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
            resolved=path.resolve()
            if resolved==overlay_root/name:continue
            try:resolved.relative_to(self.docs)
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

    def check_models(self,manifest_name='local-original/models/manifest.json',stat_prefix=''):
        self.ref(manifest_name,'original-renderer.js','model-manifest')
        manifest=self.read_json(manifest_name)
        models=manifest.get('models',[])
        self.stats[stat_prefix+'modelBindings']=len(models)
        loaded=set()
        for i,row in enumerate(models):
            if row.get('verified') is False:continue
            for field, kind in [('url', 'near-model'), ('lodMiddleUrl', 'middle-model'), ('lodFarUrl', 'far-model')]:
                if field != 'url' and not row.get(field):
                    continue
                name=self.ref(row.get(field),f'{manifest_name}.models[{i}].{field}',kind)
                if not name or name in loaded:continue
                loaded.add(name);model=self.read_json(name)
                for j,part in enumerate(model.get('parts',[])):
                    # Identical precedence to the renderer's material URL choice.
                    url=part.get('textureUrl') or part.get('texture') or manifest.get('textures',{}).get(str(part.get('textureSource','')).lower())
                    if url:self.ref(url,f'{name}.parts[{j}].material','model-material')
                    elif part.get('textureSource'):
                        self.warnings.append({'kind':'source-texture-with-white-material-fallback','source':f'{name}.parts[{j}]','textureSource':part['textureSource']})
                self.local_urls(model.get('sourceTextures',[]),name+'.sourceTextures','model-texture-audit')
                payload = model.get('animationPayload')
                if payload:
                    sidecar = self.ref(payload.get('url'), name + '.animationPayload', 'sampled-animation')
                    if sidecar and (self.files[sidecar].stat().st_size != payload.get('bytes')
                                    or self.digest(sidecar) != payload.get('sha256')):
                        self.error('animation-payload-byte-mismatch', sidecar, name)
                # Animation arrays are irrelevant to URL resolution and can be
                # hundreds of MB. Retain only the metadata needed by the later
                # auxiliary scan, so this audit also fits smaller Mac machines.
                self.json_cache[name]={'format':model.get('format'),'source':model.get('source'),
                                       'sourceTextures':model.get('sourceTextures',[]),
                                       'nativeFarmComposite':model.get('nativeFarmComposite'),
                                       'parts':[{key:part.get(key) for key in ('sourceNode','name','textureSource','textureUrl','farmPlot','farmPlotOffset')} for part in model.get('parts',[])]}
        self.stats[stat_prefix+'uniqueBoundModelJSON']=len(loaded)
        for i,row in enumerate(manifest.get('epochIconBindings',[])):
            self.hash_ref(row,f'{manifest_name}.epochIconBindings[{i}]','epoch-icon')
        self.stats[stat_prefix+'epochIconBindings']=len(manifest.get('epochIconBindings',[]))
        self.local_urls(manifest.get('textures',{}),manifest_name+'.textures','model-material-map')

    def check_citizens(self):
        base = 'local-original/citizens9/'
        self.check_models(base + 'manifest.json', 'citizen-')
        self.check_teams(base + 'team/manifest.json', 'citizen-')
        manifest = base + 'presentation.json'
        self.ref(manifest, 'original-citizen-presentation.js', 'citizen-presentation')
        data = self.read_json(manifest)
        for name, icon in data.get('icons', {}).items():
            self.hash_ref(icon, manifest + '.icons.' + name, 'citizen-icon')
        for name, event in data.get('events', {}).items():
            file = self.ref(event.get('recording'), manifest + '.events.' + name, 'citizen-voice')
            if file and self.digest(file) != event.get('sha256'):
                self.error('citizen-voice-byte-mismatch', file, manifest)

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

    def check_teams(self,name='local-original/team-textures/manifest.json',stat_prefix=''):
        self.ref(name,'original-renderer.js',stat_prefix+'team-manifest');data=self.read_json(name)
        bindings=data.get('texture_bindings',{})
        for url,key in bindings.items():
            self.ref(url,name+'.texture_bindings.<key>','team-base-binding')
            if key not in data.get('textures',{}):self.error('missing-team-family',key,name)
        for family,record in data.get('textures',{}).items():
            for url in record.get('base_texture_urls',[]):self.ref(url,f'{name}.textures.{family}.base_texture_urls','team-base')
            for letter,variant in record.get('variants',{}).items():
                self.hash_ref(variant,f'{name}.textures.{family}.variants.{letter}','team-variant')
        self.stats[stat_prefix+'teamFamilies']=len(data.get('textures',{}))
        self.stats[stat_prefix+'teamVariants']=sum(len(t.get('variants',{})) for t in data.get('textures',{}).values())

    def check_model_seasons(self):
        name='local-original/model-seasons.json'
        self.ref(name,'original-renderer.js','model-season-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-original-model-seasonal-v1':self.error('unknown-model-season-schema',data.get('format'),name)
        source_manifest='local-original/models/manifest.json'
        if source_manifest in self.files and data.get('source',{}).get('modelManifestSha256')!=self.digest(source_manifest):
            self.error('season-model-manifest-hash-mismatch',source_manifest,name)
        climates=data.get('sets',{});needed=set();texture_urls=set()
        if data.get('defaultClimate') not in climates:self.error('missing-default-model-season-climate',data.get('defaultClimate'),name)
        for key,record in data.get('textures',{}).items():
            self.hash_ref(record,f'{name}.textures.{key}','model-season-neutral-texture')
            texture_urls.add(record.get('url'))
        for climate,sets in climates.items():
            for key,record in sets.items():
                urls=record.get('urls',[]);paths=record.get('paths',[])
                if not urls or len(urls)!=len(paths):self.error('season-source-list-length-mismatch',f'{climate}/{key}',name)
                for i,url in enumerate(urls):
                    self.ref(url,f'{name}.sets.{climate}.{key}.urls[{i}]','model-season-ordered-image')
                    if url not in texture_urls:self.error('season-image-without-decoded-source-record',url,name)
        for url,record in data.get('models',{}).items():
            target=self.ref(url,f'{name}.models.<key>','model-season-model-binding')
            if not target:continue
            model=self.read_json(target)
            if record.get('source')!=model.get('source'):self.error('season-nif-source-mismatch',url,name)
            parts={str(p.get('sourceNode')):p for p in model.get('parts',[])}
            for node,binding in record.get('parts',{}).items():
                source=parts.get(node)
                if not source:self.error('season-node-not-in-model',{'model':url,'node':node},name);continue
                if any(binding.get(a)!=source.get(b) for a,b in [('name','name'),('baseTextureSource','textureSource'),('baseTextureUrl','textureUrl')]):
                    self.error('season-mesh-source-mismatch',{'model':url,'node':node},name)
                if binding.get('baseTextureUrl'):self.ref(binding['baseTextureUrl'],f'{name}.models.{url}.parts.{node}.baseTextureUrl','model-season-base-image')
                key=str(binding.get('key','')).casefold();needed.add(key)
                for climate,sets in climates.items():
                    if key not in sets:self.error('missing-explicit-model-season-set',{'model':url,'node':node,'climate':climate,'key':key},name)
                self.stats['modelSeasonMeshBindings']+=1
        team_name=self.ref(data.get('teamManifest'),name+'.teamManifest','model-season-team-manifest')
        if team_name:self.check_teams(team_name,'modelSeason-')
        for i,binding in enumerate(data.get('unresolvedBindings',[])):
            target=self.ref(binding.get('modelUrl'),f'{name}.unresolvedBindings[{i}].modelUrl','model-season-unresolved-model')
            if binding.get('baseTextureUrl'):self.ref(binding['baseTextureUrl'],f'{name}.unresolvedBindings[{i}].baseTextureUrl','model-season-unresolved-base-image')
            if target:
                model=self.read_json(target);parts={p.get('sourceNode'):p for p in model.get('parts',[])}
                if binding.get('sourceNode') not in parts:self.error('unresolved-season-node-not-in-model',binding.get('sourceNode'),target)
                if binding.get('source')!=model.get('source'):self.error('unresolved-season-source-mismatch',target,name)
        self.stats['modelSeasonModelBindings']=len(data.get('models',{}))
        self.stats['modelSeasonSetsPerClimate']=len(needed)
        self.stats['modelSeasonClimates']=len(climates)
        self.stats['modelSeasonPhysicalTextures']=len(texture_urls)
        self.stats['modelSeasonUnresolvedBindings']=len(data.get('unresolvedBindings',[]))

    def check_water(self):
        name='local-original/water-native/manifest.json';base='local-original/water-native'
        self.ref(name,'original-water.js','native-water-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-native-water-v1':self.error('water-manifest-format',name);return
        images=data.get('images',{});required={*(f'skin{i}' for i in range(1,7)),'normal','palette','lookup'}
        if not required.issubset(images):self.error('water-missing-native-images',sorted(required-set(images)),name)
        for key,record in images.items():
            target=self.ref(record.get('url'),f'{name}.images.{key}','native-water-image',base)
            if target and record.get('pngSha256')!=self.digest(target):self.error('water-png-hash-mismatch',target,name)
        for profile in ('high','fallback'):
            row=data.get('profiles',{}).get(profile,{})
            for role in ('dark','detail','gloss'):
                if row.get(role) not in images:self.error('water-invalid-material-binding',f'{profile}/{role}',name)
        alpha=data.get('paletteAlphaBytes',[])
        if len(alpha)!=256 or any(type(value)!=int or not 0<=value<=255 for value in alpha):self.error('water-invalid-palette-alpha',name)
        lookup=data.get('lookupRGBBytes',[])
        if len(lookup)<3 or any(not isinstance(rgb,list) or len(rgb)!=3 or any(type(value)!=int or not 0<=value<=255 for value in rgb) for rgb in lookup):self.error('water-invalid-lookup-rgb',name)
        self.stats['nativeWaterImages']=len(images)
        self.stats['nativeWaterProfiles']=len(data.get('profiles',{}))

    def check_farms(self):
        name='local-original/farm-composites.json'
        self.ref(name,'original-renderer.js','farm-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-native-farm-composites-v1':self.error('farm-manifest-format',name);return
        families=data.get('families',{});expected={'FarmWheat','FarmSugarCane','FarmYam'}
        if set(families)!=expected:self.error('farm-family-list',sorted(families),name)
        loaded=set();images=set()
        for family,row in families.items():
            states=row.get('states',{})
            if set(states)!={'Seeding','Harvest_1','Harvest_2','Harvest_3'}:self.error('farm-state-list',family,name)
            if row.get('defaultState') not in states:self.error('farm-default-state',family,name)
            for state,record in states.items():
                target=self.ref(record.get('url'),f'{name}.{family}.{state}','farm-state-model')
                if not target:continue
                model=self.read_json(target);loaded.add(target)
                if model.get('source')!=record.get('source'):self.error('farm-model-source-mismatch',target,name)
                native=model.get('nativeFarmComposite',{})
                if native.get('family')!=family or native.get('state')!=state:self.error('farm-model-state-mismatch',target,name)
                parts=model.get('parts',[])
                if {part.get('farmPlot') for part in parts}!={1,2,3,4}:self.error('farm-plot-list',target,name)
                for i,part in enumerate(parts):
                    offset=part.get('farmPlotOffset',[])
                    if not isinstance(offset,list) or len(offset)!=3 or any(type(v) not in (int,float) or not math.isfinite(v) for v in offset):self.error('farm-invalid-plot-offset',f'{target}.parts[{i}]',name)
                    url=part.get('textureUrl') or part.get('texture')
                    if url:
                        image=self.ref(url,f'{target}.parts[{i}].material','farm-material')
                        if image:images.add(image)
                    elif part.get('textureSource'):self.error('farm-unbound-native-material',f'{target}.parts[{i}]',name)
                self.local_urls(model.get('sourceTextures',[]),target+'.sourceTextures','farm-texture-audit')
                # Retain source/mesh metadata for the seasonal graph, not large
                # animation/geometry arrays irrelevant to portable URL checks.
                self.json_cache[target]={'format':model.get('format'),'source':model.get('source'),
                    'sourceTextures':model.get('sourceTextures',[]),'parts':[{key:part.get(key) for key in ('sourceNode','name','textureSource','textureUrl')} for part in parts]}
        self.stats['farmFamilies']=len(families);self.stats['farmStateModels']=len(loaded);self.stats['farmMaterialImages']=len(images)

    def check_hud(self):
        name='local-original/hud-native/manifest.json';base='local-original/hud-native'
        self.ref(name,'original-hud.js','hud-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-native-hud-v1':self.error('hud-manifest-format',name);return
        scale=data.get('sourceResolutionScale')
        if scale!=4:self.error('hud-source-resolution-scale',scale,name)
        if data.get('coordinateReference')!=[1600,1200]:self.error('hud-coordinate-reference',data.get('coordinateReference'),name)
        sprites=data.get('sprites',{});textures=data.get('textures',{});widgets=data.get('widgets',{})
        for key,row in sprites.items():
            target=self.ref(row.get('file'),f'{name}.sprites.{key}.file','hud-sprite-png',base)
            if target:
                if self.digest(target)!=row.get('sha256'):self.error('hud-png-hash-mismatch',target,name)
                header=self.files[target].read_bytes()[:24]
                if len(header)!=24 or header[:8]!=b'\x89PNG\r\n\x1a\n' or header[8:16]!=b'\x00\x00\x00\rIHDR':self.error('hud-invalid-png-header',target,name)
                elif [int.from_bytes(header[16:20],'big'),int.from_bytes(header[20:24],'big')]!=[row.get('width'),row.get('height')]:self.error('hud-png-dimension-mismatch',target,name)
            for physical,logical in [('width','logicalWidth'),('height','logicalHeight')]:
                value=row.get(logical);dimension=row.get(physical)
                if type(value) not in (int,float) or not math.isfinite(value) or value<=0 or type(dimension)!=int or dimension<=0 or value*4!=dimension:self.error('hud-logical-scale-mismatch',f'{key}/{logical}',name)
            texture=textures.get(row.get('sourceTexture'))
            rect=row.get('sourceSprite',{}).get('rect',[])
            if not texture:self.error('hud-missing-source-texture',row.get('sourceTexture'),name)
            elif not isinstance(rect,list) or len(rect)!=4 or any(type(v)!=int or v<0 for v in rect) or rect[2]<=0 or rect[3]<=0 or rect[0]+rect[2]>texture.get('width',0) or rect[1]+rect[3]>texture.get('height',0):self.error('hud-source-crop-outside-texture',key,name)
            else:
                expected=[rect[3],rect[2]] if row.get('restoredPackedRotation') else rect[2:4]
                if expected!=[row.get('width'),row.get('height')]:self.error('hud-restored-crop-dimension-mismatch',key,name)
        for key,row in widgets.items():
            if row.get('NAME')!=key:self.error('hud-widget-name-mismatch',key,name)
            for field in ('PARENT','ANCHORNAME1','ANCHORNAME2'):
                if row.get(field) and row[field] not in widgets:self.error('hud-missing-widget-anchor',f'{key}/{field}/{row[field]}',name)
            for field in ('ANCHORTYPE1','ANCHORTYPE2'):
                value=row.get(field,'')
                if value not in ('','NoAnchor') and not re.fullmatch(r'Source(?:UL|LL|UR|LR)Target(?:UL|LL|UR|LR)',value):self.error('hud-invalid-anchor-type',f'{key}/{field}/{value}',name)
        done=set();visiting=set()
        def visit(key):
            if key in visiting:self.error('hud-widget-anchor-cycle',key,name);return
            if key in done:return
            visiting.add(key)
            for field in ('PARENT','ANCHORNAME1','ANCHORNAME2'):
                parent=widgets[key].get(field)
                if parent in widgets:visit(parent)
            visiting.remove(key);done.add(key)
        for key in widgets:visit(key)
        for widget,sprite in data.get('art',{}).items():
            if widget not in widgets:self.error('hud-art-missing-widget',widget,name)
            elif widgets[widget].get('SPRITE')!=sprite:self.error('hud-art-widget-sprite-mismatch',widget,name)
            if sprite not in sprites:self.error('hud-art-missing-sprite',sprite,name)
            # Match the runtime's explicit meta variant -> base fallback.
            for meta in (2,3):
                chosen=f'{sprite}_e{meta}' if f'{sprite}_e{meta}' in sprites else sprite
                if chosen not in sprites:self.error('hud-art-missing-meta-sprite',f'{widget}/e{meta}',name)
        self.stats['hudWidgets']=len(widgets);self.stats['hudSprites']=len(sprites);self.stats['hudSourceTextures']=len(textures);self.stats['hudArtBindings']=len(data.get('art',{}))

    def check_diplomacy(self):
        name='local-original/diplomacy-native/manifest.json';base='local-original/diplomacy-native'
        self.ref(name,'original-diplomacy.js','diplomacy-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-native-diplomacy-v1':self.error('diplomacy-manifest-format',name);return
        if data.get('sourceResolutionScale')!=4:self.error('diplomacy-source-resolution-scale',data.get('sourceResolutionScale'),name)
        if data.get('coordinateReference')!=[1600,1200]:self.error('diplomacy-coordinate-reference',data.get('coordinateReference'),name)
        sprites=data.get('sprites',{});textures=data.get('textures',{});widgets=data.get('widgets',{})
        frames=data.get('frameStyles',{});styles=data.get('fontStyles',{});fonts=data.get('fonts',{})
        for key,row in sprites.items():
            target=self.ref(row.get('file'),f'{name}.sprites.{key}.file','diplomacy-sprite-png',base)
            if target:
                if self.digest(target)!=row.get('sha256'):self.error('diplomacy-png-hash-mismatch',target,name)
                header=self.files[target].read_bytes()[:24]
                if len(header)!=24 or header[:16]!=b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR':self.error('diplomacy-invalid-png-header',target,name)
                elif [int.from_bytes(header[16:20],'big'),int.from_bytes(header[20:24],'big')]!=[row.get('width'),row.get('height')]:self.error('diplomacy-png-dimension-mismatch',target,name)
            # Enhanced UI7 atlases are 4x; unchanged shared sprites are native 1x.
            scale=row.get('sourceResolutionScale');scales=scale if isinstance(scale,list) else [scale,scale]
            for axis,(physical,logical) in enumerate([('width','logicalWidth'),('height','logicalHeight')]):
                value=row.get(logical);dimension=row.get(physical);factor=scales[axis] if len(scales)==2 else None
                if type(factor) not in (int,float) or not math.isfinite(factor) or factor<=0 or type(value) not in (int,float) or not math.isfinite(value) or value<=0 or type(dimension)!=int or dimension<=0 or value*factor!=dimension:self.error('diplomacy-logical-scale-mismatch',f'{key}/{logical}',name)
            texture=textures.get(row.get('sourceTexture'));rect=row.get('sourceSprite',{}).get('rect',[])
            if not texture:self.error('diplomacy-missing-source-texture',row.get('sourceTexture'),name)
            elif not isinstance(rect,list) or len(rect)!=4 or any(type(v)!=int or v<0 for v in rect) or rect[2]<=0 or rect[3]<=0 or rect[0]+rect[2]>texture.get('width',0) or rect[1]+rect[3]>texture.get('height',0):self.error('diplomacy-source-crop-outside-texture',key,name)
            elif ([rect[3],rect[2]] if row.get('restoredPackedRotation') else rect[2:4])!=[row.get('width'),row.get('height')]:self.error('diplomacy-restored-crop-dimension-mismatch',key,name)
        for key,row in widgets.items():
            if row.get('NAME')!=key:self.error('diplomacy-widget-name-mismatch',key,name)
            for field in ('PARENT','ANCHORNAME1','ANCHORNAME2'):
                if row.get(field) and row[field] not in widgets:self.error('diplomacy-missing-widget-anchor',f'{key}/{field}/{row[field]}',name)
            for field in ('ANCHORTYPE1','ANCHORTYPE2'):
                value=row.get(field,'')
                if value not in ('','NoAnchor') and not re.fullmatch(r'Source(?:UL|LL|UR|LR)Target(?:UL|LL|UR|LR)',value):self.error('diplomacy-invalid-anchor-type',f'{key}/{field}/{value}',name)
            for field,table in [('SPRITE',sprites),('FRAMESTYLE',frames),('FONTSTYLE',styles)]:
                if row.get(field) and row[field] not in table:self.error('diplomacy-missing-widget-binding',f'{key}/{field}/{row[field]}',name)
        done=set();visiting=set()
        def visit(key):
            if key in visiting:self.error('diplomacy-widget-anchor-cycle',key,name);return
            if key in done:return
            visiting.add(key)
            for field in ('PARENT','ANCHORNAME1','ANCHORNAME2'):
                parent=widgets[key].get(field)
                if parent in widgets:visit(parent)
            visiting.remove(key);done.add(key)
        for key in widgets:visit(key)
        for key,row in frames.items():
            # Native glass/inset frames are color-only and declare no sprite.
            if row.get('SPRITE') and row['SPRITE'] not in sprites:self.error('diplomacy-frame-missing-sprite',key,name)
        for key,row in styles.items():
            if row.get('FONT') not in fonts:self.error('diplomacy-style-missing-font',key,name)
        strips=data.get('playerStripPixels',{})
        for key in ('spr_dip_pcolor_strip_glow','spr_dip_pcolor_strip_glow_on','spr_dip_pcolor_strip_2state','spr_dip_pcolor_strip_2state_on'):
            row=strips.get(key,{})
            try:rgba=base64.b64decode(row.get('rgba',''),validate=True)
            except (ValueError,TypeError):rgba=b''
            sprite=sprites.get(key,{})
            if not rgba or [row.get('width'),row.get('height')]!=[sprite.get('width'),sprite.get('height')] or len(rgba)!=row.get('width',0)*row.get('height',0)*4 or hashlib.sha256(rgba).hexdigest()!=row.get('rgbaSha256'):self.error('diplomacy-invalid-strip-rgba',key,name)
            if row.get('pngFile')!=sprite.get('file'):self.error('diplomacy-strip-sprite-file-mismatch',key,name)
        self.stats['diplomacyWidgets']=len(widgets);self.stats['diplomacySprites']=len(sprites);self.stats['diplomacySourceTextures']=len(textures);self.stats['diplomacyPlayerStrips']=len(strips)

    def check_fonts(self):
        base='local-original/fonts-native';name=base+'/bitmap-manifest.json';cache_name=base+'/manifest.json'
        self.ref(name,'original-fonts.js','font-manifest');self.ref(cache_name,name,'font-cache-manifest')
        data=self.read_json(name);cache=self.read_json(cache_name)
        if data.get('format')!='ee2-native-bitmap-font-v1' or cache.get('format')!='ee2-original-font-cache-v1':self.error('font-manifest-format',name);return
        if data.get('sourceCacheManifestSha256')!=self.digest(cache_name):self.error('font-cache-manifest-hash-mismatch',name)
        def image(row,label,group):
            target=self.ref(row.get('file'),label,group,base)
            if not target:return
            if self.digest(target)!=(row.get('sha256') or row.get('pngSha256')):self.error('font-png-hash-mismatch',target,label)
            h=self.files[target].read_bytes()[:24]
            if len(h)!=24 or h[:16]!=b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR':self.error('font-invalid-png',target,label)
            elif [int.from_bytes(h[16:20],'big'),int.from_bytes(h[20:24],'big')]!=[row.get('width'),row.get('height')]:self.error('font-png-dimensions',target,label)
        for key,row in cache.get('fonts',{}).items():image(row,f'{cache_name}.fonts.{key}','font-original-cache-png')
        images=data.get('images',{});fonts=data.get('fonts',{});bindings=data.get('bindings',{})
        for key,row in images.items():
            if row.get('file')!=key:self.error('font-image-key-mismatch',key,name)
            image(row,f'{name}.images.{key}','font-atlas-png')
        glyph_count=0;original_count=0
        for key,font in fonts.items():
            glyphs=font.get('glyphs',{});glyph_count+=len(glyphs);source=font.get('sourceCache');original=cache.get('fonts',{}).get(source.get('key')) if isinstance(source,dict) else None
            if source and not original:self.error('font-unknown-source-cache',key,name)
            nominal=font.get('nominalSize');height=font.get('lineHeight');flags=font.get('flags',0)
            if type(nominal)!=int or nominal<=0 or type(height)!=int or type(flags)!=int or height!=nominal+(2 if flags&8 else 0) or font.get('spacing')!=(-1 if flags&8 else 0):self.error('font-invalid-native-metrics',key,name)
            if original and font.get('kerning')!={f'{a}:{b}':amount for a,b,amount in original.get('kerning',[])}:self.error('font-original-kerning-changed',key,name)
            for pair,amount in font.get('kerning',{}).items():
                if not re.fullmatch(r'\d+:\d+',pair) or type(amount)!=int:self.error('font-invalid-kerning',f'{key}/{pair}',name)
            for cp,glyph in glyphs.items():
                atlas=images.get(glyph.get('file'));rect=[glyph.get(k) for k in ('x','y','width','height')]
                if not atlas:self.error('font-glyph-missing-atlas',f'{key}/{cp}',name);continue
                if not cp.isdecimal() or int(cp)>0x10ffff or any(type(v)!=int for v in rect) or min(rect)<0 or rect[2]<=0 or rect[3]!=height or rect[0]+rect[2]>atlas.get('width',0) or rect[1]+rect[3]>atlas.get('height',0):self.error('font-glyph-outside-atlas',f'{key}/{cp}',name)
                if glyph.get('origin')=='shipped-cache':
                    original_count+=1;cell=original.get('glyphs',{}).get(cp) if original else None
                    if not cell or [glyph['file'],*rect]!=[original['file'],cell['x'],cell['y'],cell['width'],original['headerObjectOffsets']['0x2c']]:self.error('font-shipped-glyph-changed',f'{key}/{cp}',name)
                elif glyph.get('origin')!='current-headless-gdi':self.error('font-unknown-glyph-origin',f'{key}/{cp}',name)
            if not all(str(ord(cp)) in glyphs for cp in 'ÄÖÜäöüß'):self.error('font-missing-german-glyph',key,name)
        for key,tiers in bindings.items():
            if not isinstance(tiers,list) or len(tiers)!=6 or any(tier not in fonts for tier in tiers):self.error('font-invalid-resolution-binding',key,name)
        for key,row in data.get('fontStyles',{}).items():
            if row.get('FONT') not in bindings:self.error('font-style-missing-font',key,name)
        self.stats['nativeFontVariants']=len(fonts);self.stats['nativeFontSourceAtlases']=len(cache.get('fonts',{}));self.stats['nativeFontGlyphBindings']=glyph_count;self.stats['nativeFontOriginalGlyphBindings']=original_count;self.stats['nativeFontStyleBindings']=len(data.get('fontStyles',{}))

    def check_audio(self,name='local-original/audio/manifest.json',stat_prefix=''):
        base='local-original/audio'
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
        self.stats[stat_prefix+'audioAssets']=len(assets)
        family_name=base+'/native-families.json'
        module=self.files.get('original-audio.js')
        if not stat_prefix and (family_name in self.files or (module and 'native-families.json' in module.read_text(encoding='utf8'))):
            self.check_audio_families()

    def check_audio_families(self):
        name='local-original/audio/native-families.json';base='local-original/audio'
        self.ref(name,'original-audio.js','native-family-audio-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-native-family-audio-v1':self.error('native-family-audio-format',name);return
        assets=data.get('assets',{});events=data.get('events',{});families=data.get('families',{})
        if not all(isinstance(value,dict) for value in (assets,events,families)):
            self.error('native-family-audio-tables',name);return
        for key,record in assets.items():
            if not isinstance(record,dict):self.error('native-family-audio-asset',key,name);continue
            target=self.ref(record.get('file'),f'{name}.assets.{key}','native-family-audio-asset',base)
            if record.get('file')!=key:self.error('native-family-audio-asset-key',key,name)
            if target:
                if not target.startswith(base+'/'):self.error('native-family-audio-base',target,name)
                if self.files[target].stat().st_size!=record.get('bytes'):self.error('native-family-audio-size',target,name)
                if self.digest(target)!=record.get('sha256'):self.error('native-family-audio-hash',target,name)
        for event,record in events.items():
            if not isinstance(record,dict):self.error('native-family-audio-event',event,name);continue
            file=record.get('file');self.ref(file,f'{name}.events.{event}','native-family-audio-event',base)
            if not isinstance(file,str) or file not in assets:self.error('native-family-audio-unbound-clip',event,name)
            volume=record.get('volume')
            if type(volume) not in (int,float) or not math.isfinite(volume) or not 0<=volume<=1:self.error('native-family-audio-volume',event,name)
        interval_count=0
        for family,record in families.items():
            if not isinstance(record,dict) or record.get('role') not in ('voice','selection-effect') or not isinstance(record.get('intervals'),list):
                self.error('native-family-audio-role',family,name);continue
            previous=0
            for row in record['intervals']:
                if not isinstance(row,dict):self.error('native-family-audio-interval',family,name);continue
                epoch,until=row.get('epoch'),row.get('until')
                if type(epoch)!=int or type(until)!=int or not previous<=epoch<until<=15:
                    self.error('native-family-audio-interval',family,name)
                else:previous=until
                channels=row.get('effectiveEvents',{})
                for action in ('select','order','attack'):
                    names=channels.get(action) if isinstance(channels,dict) else None
                    if not isinstance(names,list) or len(names)>8 or any(not isinstance(event,str) or event not in events for event in names):
                        self.error('native-family-audio-channel',family+':'+action,name)
                interval_count+=1
        self.stats['nativeFamilyAudioAssets']=len(assets);self.stats['nativeFamilyAudioEvents']=len(events);self.stats['nativeFamilyAudioFamilies']=len(families);self.stats['nativeFamilyAudioIntervals']=interval_count

    def check_sails(self):
        name='local-original/sails-native/manifest.json'
        self.ref(name,'original-renderer.js','native-sail-manifest');data=self.read_json(name)
        if data.get('format')!='ee2-native-sail-initial-composites-v1':self.error('native-sail-format',name);return
        assets=data.get('assets',{})
        if not isinstance(assets,dict) or not isinstance(data.get('models'),list):self.error('native-sail-tables',name);return
        for key,row in assets.items():
            target=self.ref(row.get('url'),name+'.assets.'+key,'native-sail-texture')
            if row.get('url')!=key:self.error('native-sail-texture-key',key,name)
            if target and self.digest(target)!=row.get('pngSha256'):self.error('native-sail-texture-hash',target,name)
        parts=0
        for row in data['models']:
            target=self.ref(row.get('url'),name+'.models','native-sail-composite')
            if not target:continue
            if self.digest(target)!=row.get('sha256'):self.error('native-sail-composite-hash',target,name)
            composite=self.read_json(target)
            if composite.get('assetKind')!=0 or composite.get('id')!=58 or composite.get('sourceFamily')!='FightingSail':self.error('native-sail-typed-binding',target,name)
            if composite.get('source',{}).get('sha256')!=row.get('source',{}).get('sha256'):self.error('native-sail-parent-hash',target,name)
            if composite.get('effect')!=row.get('effect') or composite.get('epoch')!=row.get('epoch') or composite.get('until')!=row.get('until'):self.error('native-sail-model-interval',target,name)
            planes=composite.get('parts',[])
            if not isinstance(planes,list) or len(planes)!=row.get('sails'):self.error('native-sail-plane-count',target,name);continue
            for part in planes:
                positions=part.get('positions',[]);vertices=len(positions)//3
                if len(positions)%3 or not vertices or len(part.get('normals',[]))!=3*vertices or len(part.get('uvs',[]))!=2*vertices or len(part.get('pinned',[]))!=vertices or any(type(i)!=int or i<0 or i>=vertices for i in part.get('indices',[])):
                    self.error('native-sail-mesh-layout',target,name)
                urls=[part.get('textureUrl'),*part.get('damageMaskUrls',[])]
                if len(part.get('damageMaskUrls',[]))!=9:self.error('native-sail-mask-count',target,name)
                for url in urls:
                    self.ref(url,target,'native-sail-part-texture')
                    if url not in assets:self.error('native-sail-unbound-texture',url,target)
            parts+=len(planes)
        self.stats['nativeSailComposites']=len(data['models']);self.stats['nativeSailPlanes']=parts;self.stats['nativeSailTextures']=len(assets)

    def check_rule_profiles(self):
        name='rules-profile.js';source=self.files[name].read_text(encoding='utf8')
        enabled=re.search(r'^export\s+const\s+UP15_PROFILE_ENABLED\s*=\s*(true|false)\s*;',source,re.M)
        if not enabled:self.error('missing-explicit-runtime-profile-switch',name);return
        files=['openepoch.wasm','openepoch-ee2.wasm']
        if enabled[1]=='true':files.append('openepoch-ee2-up15.wasm')
        for file in files:self.ref(file,name,'numerical-profile-wasm')
        self.stats['numericalRuleProfileBinaries']=len(files)

    def check_source_cloth(self):
        name='source-sail-runtime.js';text=self.files[name].read_text(encoding='utf8')
        expression=r"SOURCE_CLOTH_MODULE=Object\.freeze\(\{\s*url:new URL\('([^']+)',\s*import\.meta\.url\),bytes:(\d+),\s*sha256:'([a-f0-9]{64})'"
        descriptor=re.search(expression,text)
        if not descriptor:self.error('own-cloth-missing-integrity-descriptor',name);return
        target=self.ref(descriptor[1],name,'own-source-cloth-wasm')
        if not target:return
        binary=self.files[target]
        if binary.stat().st_size!=int(descriptor[2]):self.error('own-cloth-size-mismatch',target,name)
        if self.digest(target)!=descriptor[3]:self.error('own-cloth-sha256-mismatch',target,name)
        with binary.open('rb')as stream:
            if stream.read(8)!=b'\0asm\x01\0\0\0':self.error('own-cloth-invalid-wasm-header',target,name)
        self.stats['ownSourceClothWasmBytes']=binary.stat().st_size

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
        self.check_models()
        if 'local-original/models/manifest-up15.json' in self.files:
            self.check_models('local-original/models/manifest-up15.json','up15-')
        if 'local-original/models/manifest-roster6.json' in self.files:
            self.check_models('local-original/models/manifest-roster6.json','roster6-')
        if 'original-citizen-presentation.js' in self.files:
            self.check_citizens()
            self.check_models('local-original/regional9/manifest.json', 'regional-')
        if 'local-original/regional-citycenters/manifest.json' in self.files:
            self.check_models('local-original/regional-citycenters/manifest.json', 'town-centers-')
            self.check_teams('local-original/regional-citycenters/team/manifest.json', 'town-centers-')
        if 'local-original/regional-houses/manifest.json' in self.files:
            self.check_models('local-original/regional-houses/manifest.json', 'regional-buildings-')
            self.check_teams('local-original/regional-houses/team/manifest.json', 'regional-buildings-')
        if 'local-original/regional-docks/manifest.json' in self.files:
            self.check_models('local-original/regional-docks/manifest.json', 'regional-docks-')
            self.check_teams('local-original/regional-docks/team/manifest.json', 'regional-docks-')
        if 'local-original/regional-barracks/manifest.json' in self.files:
            self.check_models('local-original/regional-barracks/manifest.json', 'regional-barracks-')
            self.check_teams('local-original/regional-barracks/team/manifest.json', 'regional-barracks-')
        for family in ('mill','temple'):
            base=f'local-original/regional-{family}/'
            if base+'manifest.json' in self.files:
                self.check_models(base+'manifest.json', f'regional-{family}-')
                self.check_teams(base+'team/manifest.json', f'regional-{family}-')
        if 'local-original/stone-thrower/regional-models.json' in self.files:
            self.check_models('local-original/stone-thrower/regional-models.json','stone-thrower-')
        if 'local-original/stone-thrower/regional-models-native15.json' in self.files:
            self.check_models('local-original/stone-thrower/regional-models-native15.json','stone-thrower-clocked-')
        self.check_textures_and_ground();self.check_teams();self.check_model_seasons();self.check_audio();self.check_static_runtime()
        if 'local-original/audio/manifest-stone-thrower-up.json' in self.files:
            self.check_audio('local-original/audio/manifest-stone-thrower-up.json','stone-thrower-')
        if 'original-water.js' in self.files:self.check_water()
        if 'local-original/farm-composites.json' in self.files:self.check_farms()
        if 'original-hud.js' in self.files:self.check_hud()
        if 'original-diplomacy.js' in self.files:self.check_diplomacy()
        if 'original-fonts.js' in self.files:self.check_fonts()
        if 'local-original/sails-native/manifest.json' in self.files:self.check_sails()
        if 'local-original/labels-native.json' in self.files:
            self.ref('local-original/labels-native.json','original-labels.js','original-labels-manifest');self.read_json('local-original/labels-native.json')
        if 'rules-profile.js' in self.files:self.check_rule_profiles()
        if 'source-sail-runtime.js' in self.files:self.check_source_cloth()
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
    def test_own_cloth_pin_rejects_corruption_and_missing_or_wrong_case_files(self):
        original=b'\0asm\x01\0\0\0own-visual-fixture'
        binary=self.root/'Cloth.wasm';binary.write_bytes(original)
        def audit(url='Cloth.wasm?v=test',size=len(original),digest=None):
            text="export const SOURCE_CLOTH_MODULE=Object.freeze({\n url:new URL('"+url+"',import.meta.url),bytes:"+str(size)+",\n sha256:'"+(digest or hashlib.sha256(original).hexdigest())+"',\n});"
            (self.root/'source-sail-runtime.js').write_text(text,encoding='utf8');value=AssetAudit(self.root);value.check_source_cloth();return value
        self.assertFalse(audit().errors)
        binary.write_bytes(original[:-1]+b'X');self.assertIn('own-cloth-sha256-mismatch',[e['kind']for e in audit().errors])
        binary.write_bytes(original);self.assertIn('own-cloth-size-mismatch',[e['kind']for e in audit(size=1).errors])
        self.assertIn('case-mismatch',[e['kind']for e in audit(url='cloth.wasm').errors])
        self.assertIn('missing-runtime-file',[e['kind']for e in audit(url='Missing.wasm').errors])
        binary.write_bytes(b'MZ'+original[2:]);self.assertIn('own-cloth-invalid-wasm-header',[e['kind']for e in audit(digest=hashlib.sha256(binary.read_bytes()).hexdigest()).errors])
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
    def family_audio_fixture(self):
        folder=self.root/'local-original/audio/family6';folder.mkdir(parents=True,exist_ok=True)
        clip=folder/'Voice.mp3';clip.write_bytes(b'original-audio-byte-fixture')
        file='family6/Voice.mp3'
        return {'format':'ee2-native-family-audio-v1','assets':{file:{'file':file,'bytes':clip.stat().st_size,'sha256':sha(clip)}},
                'events':{'voice_select':{'file':file,'volume':.5}},'families':{'Scout':{'role':'voice','intervals':[{'epoch':0,'until':15,'effectiveEvents':{'select':['voice_select'],'order':[],'attack':[]}}]}}}
    def family_audio_audit(self,data):
        (self.root/'local-original/audio/native-families.json').write_text(json.dumps(data),encoding='utf8')
        audit=AssetAudit(self.root);audit.check_audio_families();return audit
    def test_family_audio_graph_binds_exact_case_media_and_events(self):
        audit=self.family_audio_audit(self.family_audio_fixture());self.assertFalse(audit.errors);self.assertEqual(audit.stats['nativeFamilyAudioAssets'],1)
    def test_family_audio_graph_rejects_missing_case_hash_size_and_event_media(self):
        for mutate,expected in [
            (lambda d:d['assets']['family6/Voice.mp3'].update(file='family6/voice.mp3'),'case-mismatch'),
            (lambda d:d['assets']['family6/Voice.mp3'].update(sha256='0'*64),'native-family-audio-hash'),
            (lambda d:d['assets']['family6/Voice.mp3'].update(bytes=0),'native-family-audio-size'),
            (lambda d:d['events']['voice_select'].update(file='family6/Missing.mp3'),'missing-runtime-file')]:
            data=self.family_audio_fixture();mutate(data);self.assertIn(expected,[row['kind'] for row in self.family_audio_audit(data).errors])
    def test_family_audio_graph_rejects_bad_channels_intervals_and_volume(self):
        for mutate,expected in [
            (lambda d:d['families']['Scout']['intervals'].append(d['families']['Scout']['intervals'][0]),'native-family-audio-interval'),
            (lambda d:d['families']['Scout']['intervals'][0]['effectiveEvents'].update(select=['missing']),'native-family-audio-channel'),
            (lambda d:d['events']['voice_select'].update(volume=float('nan')),'native-family-audio-volume'),
            (lambda d:d['families']['Scout'].update(role='building'),'native-family-audio-role')]:
            data=self.family_audio_fixture();mutate(data);self.assertIn(expected,[row['kind'] for row in self.family_audio_audit(data).errors])
    def test_missing_file_is_not_a_case_mismatch(self):
        a=AssetAudit(self.root);a.ref('Icons/Missing.png','test','fixture');self.assertEqual(a.errors[0]['kind'],'missing-runtime-file')
    def sail_fixture(self):
        base='local-original/sails-native';folder=self.root/base;folder.mkdir(parents=True,exist_ok=True)
        url='Icons/Tree.png';source={'sha256':'a'*64}
        part={'positions':[0,0,0,1,0,0,0,1,0],'normals':[0,0,1]*3,'uvs':[0,0,1,0,0,1],'indices':[0,1,2],'pinned':[1,0,1],'textureUrl':url,'damageMaskUrls':[url]*9}
        composite={'assetKind':0,'id':58,'sourceFamily':'FightingSail','source':source,'epoch':5,'until':7,'effect':'CaravelSail','parts':[part]}
        target=folder/'Ship.json';target.write_text(json.dumps(composite),encoding='utf8')
        return {'format':'ee2-native-sail-initial-composites-v1','assets':{url:{'url':url,'pngSha256':sha(self.root/url)}},'models':[{'url':base+'/Ship.json','sha256':sha(target),'source':source,'epoch':5,'until':7,'effect':'CaravelSail','sails':1}]}
    def sail_audit(self,data):
        (self.root/'local-original/sails-native/manifest.json').write_text(json.dumps(data),encoding='utf8');audit=AssetAudit(self.root);audit.check_sails();return audit
    def test_sail_graph_checks_composite_hash_typed_parent_and_masks(self):
        audit=self.sail_audit(self.sail_fixture());self.assertFalse(audit.errors);self.assertEqual(audit.stats['nativeSailPlanes'],1)
    def test_sail_graph_rejects_wrong_case_and_changed_payloads(self):
        for mutate,expected in [(lambda d:d['models'][0].update(url='local-original/sails-native/ship.json'),'case-mismatch'),(lambda d:d['models'][0].update(sha256='0'*64),'native-sail-composite-hash'),(lambda d:d['assets']['Icons/Tree.png'].update(pngSha256='0'*64),'native-sail-texture-hash'),(lambda d:d['models'][0].update(source={'sha256':'b'*64}),'native-sail-parent-hash')]:
            data=self.sail_fixture();mutate(data);self.assertIn(expected,[row['kind'] for row in self.sail_audit(data).errors])
    def test_sail_graph_rejects_unbound_mask_and_malformed_mesh(self):
        for field,value,expected in [('damageMaskUrls',[],'native-sail-mask-count'),('textureUrl','Icons/Missing.png','native-sail-unbound-texture'),('uvs',[],'native-sail-mesh-layout')]:
            data=self.sail_fixture();target=self.root/data['models'][0]['url'];composite=json.loads(target.read_text());composite['parts'][0][field]=value;target.write_text(json.dumps(composite),encoding='utf8');data['models'][0]['sha256']=sha(target)
            self.assertIn(expected,[row['kind'] for row in self.sail_audit(data).errors])
    def seasonal_fixture(self):
        model='local-original/models/tree.json';base='Icons/Tree.png';source={'archive':'graphics.zip','entry':'tree.nifcache','sha256':'1'*64}
        documents={'local-original/models/manifest.json':{'models':[{'url':model}]},
            model:{'format':'ee2-original-mesh-v1','source':source,'parts':[{'sourceNode':3,'name':'canopy','textureSource':'tree.tga','textureUrl':base}]},
            'local-original/season-model-textures/team/manifest.json':{'textures':{},'texture_bindings':{}},
            'local-original/model-seasons.json':{'format':'ee2-original-model-seasonal-v1','defaultClimate':'Temperate',
                'source':{},'teamManifest':'local-original/season-model-textures/team/manifest.json',
                'models':{model:{'source':source,'parts':{'3':{'key':'oak','name':'canopy','baseTextureSource':'tree.tga','baseTextureUrl':base}}}},
                'sets':{'Temperate':{'oak':{'paths':['tree.tga','tree.tga'],'urls':[base,base]}}},
                'textures':{'tree':{'url':base,'png_sha256':sha(self.root/base)}}}}
        for name,value in documents.items():
            p=self.root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(value),encoding='utf8')
        data=documents['local-original/model-seasons.json'];data['source']['modelManifestSha256']=sha(self.root/'local-original/models/manifest.json')
        return data
    def seasonal_audit(self,data):
        (self.root/'local-original/model-seasons.json').write_text(json.dumps(data),encoding='utf8')
        a=AssetAudit(self.root);a.check_model_seasons();return a
    def test_model_season_graph_checks_source_nodes_and_all_ordered_images(self):
        a=self.seasonal_audit(self.seasonal_fixture());self.assertFalse(a.errors);self.assertEqual(a.stats['modelSeasonMeshBindings'],1)
    def test_model_season_graph_rejects_wrong_case_hidden_inside_array(self):
        data=self.seasonal_fixture();data['sets']['Temperate']['oak']['urls'][1]='icons/tree.png'
        self.assertIn('case-mismatch',[e['kind'] for e in self.seasonal_audit(data).errors])
    def test_model_season_graph_rejects_stale_model_nodes(self):
        data=self.seasonal_fixture();parts=data['models']['local-original/models/tree.json']['parts'];parts['99']=parts.pop('3')
        self.assertIn('season-node-not-in-model',[e['kind'] for e in self.seasonal_audit(data).errors])
    def test_model_season_graph_rejects_missing_declared_array_member(self):
        data=self.seasonal_fixture();data['sets']['Temperate']['oak']['urls'].pop()
        self.assertIn('season-source-list-length-mismatch',[e['kind'] for e in self.seasonal_audit(data).errors])
    def water_fixture(self):
        directory=self.root/'local-original/water-native';directory.mkdir(parents=True,exist_ok=True)
        images={}
        for key in [*(f'skin{i}' for i in range(1,7)),'normal','palette','lookup']:
            target=directory/(key+'.png');target.write_bytes(b'original-pixel-fixture');images[key]={'url':target.name,'pngSha256':sha(target)}
        return {'format':'ee2-native-water-v1','images':images,'profiles':{'high':{'dark':'skin1','detail':'skin2','gloss':'skin4'},'fallback':{'dark':'skin1','detail':'skin5','gloss':'skin6'}},'paletteAlphaBytes':[0]*256,'lookupRGBBytes':[[192]*3,[192]*3,[105]*3]}
    def water_audit(self,data):
        (self.root/'local-original/water-native/manifest.json').write_text(json.dumps(data),encoding='utf8')
        a=AssetAudit(self.root);a.check_water();return a
    def test_native_water_checks_relative_image_graph_and_hashes(self):
        a=self.water_audit(self.water_fixture());self.assertFalse(a.errors);self.assertEqual(a.stats['nativeWaterImages'],9)
    def test_native_water_rejects_case_hash_profile_and_palette_errors(self):
        for mutation,expected in [
            (lambda data:data['images']['skin1'].update(url='Skin1.png'),'case-mismatch'),
            (lambda data:data['images']['skin1'].update(pngSha256='0'*64),'water-png-hash-mismatch'),
            (lambda data:data['profiles']['high'].update(gloss='missing'),'water-invalid-material-binding'),
            (lambda data:data['paletteAlphaBytes'].pop(),'water-invalid-palette-alpha')]:
            data=self.water_fixture();mutation(data);self.assertIn(expected,[e['kind'] for e in self.water_audit(data).errors])
    def farm_fixture(self):
        families={};source={'archive':'graphics.zip','entry':'farm.nifcache','sha256':'1'*64}
        for family in ('FarmWheat','FarmSugarCane','FarmYam'):
            states={}
            for state in ('Seeding','Harvest_1','Harvest_2','Harvest_3'):
                url=f'local-original/models/{family}-{state}.json';target=self.root/url;target.parent.mkdir(parents=True,exist_ok=True)
                model={'format':'ee2-original-mesh-v1','source':source,'nativeFarmComposite':{'family':family,'state':state},
                    'parts':[{'farmPlot':i,'farmPlotOffset':[0,0,0],'textureUrl':'Icons/Tree.png'} for i in (1,2,3,4)]}
                target.write_text(json.dumps(model),encoding='utf8');states[state]={'url':url,'source':source}
            families[family]={'states':states,'defaultState':'Harvest_3'}
        target=self.root/'local-original/farm-composites.json';target.write_text(json.dumps({'format':'ee2-native-farm-composites-v1','families':families}),encoding='utf8')
        return self.root/'local-original/models/FarmYam-Harvest_3.json'
    def test_farm_graph_checks_every_state_material(self):
        self.farm_fixture();manifest=self.root/'local-original/models/manifest.json';manifest.write_text(json.dumps({'models':[{'url':'local-original/models/FarmWheat-Harvest_3.json'}]}));audit=AssetAudit(self.root);audit.check_models();audit.check_farms();self.assertFalse(audit.errors);self.assertEqual(audit.stats['farmStateModels'],12)
    def test_farm_graph_rejects_hidden_state_material_case_and_missing_file(self):
        target=self.farm_fixture();data=json.loads(target.read_text());data['parts'][0]['textureUrl']='icons/tree.png';target.write_text(json.dumps(data));audit=AssetAudit(self.root);audit.check_farms();self.assertIn('case-mismatch',[e['kind'] for e in audit.errors])
        data['parts'][0]['textureUrl']='Icons/Missing.png';target.write_text(json.dumps(data));audit=AssetAudit(self.root);audit.check_farms();self.assertIn('missing-runtime-file',[e['kind'] for e in audit.errors])
    def test_farm_graph_rejects_stale_source_state_and_invalid_offsets(self):
        target=self.farm_fixture();data=json.loads(target.read_text());data['source']['sha256']='2'*64;data['nativeFarmComposite']['state']='Seeding';data['parts'][0]['farmPlotOffset']=[float('nan'),0,0];target.write_text(json.dumps(data));audit=AssetAudit(self.root);audit.check_farms();kinds={e['kind'] for e in audit.errors};self.assertTrue({'farm-model-source-mismatch','farm-model-state-mismatch','farm-invalid-plot-offset'}<=kinds)
    def hud_fixture(self):
        import struct,zlib
        folder=self.root/'local-original/hud-native';folder.mkdir(parents=True,exist_ok=True)
        def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
        png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',4,4,8,6,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+b'\xff'*16)*4))+chunk(b'IEND',b'')
        (folder/'Panel.png').write_bytes(png)
        return {'format':'ee2-native-hud-v1','sourceResolutionScale':4,'coordinateReference':[1600,1200],
            'sprites':{'panel':{'file':'Panel.png','sha256':sha(folder/'Panel.png'),'width':4,'height':4,'logicalWidth':1,'logicalHeight':1,'sourceTexture':'native.texcache','sourceSprite':{'rect':[0,0,4,4]}}},
            'textures':{'native.texcache':{'width':4,'height':4}},
            'widgets':{'root':{'NAME':'root','PARENT':'','SPRITE':'panel'},'panel':{'NAME':'panel','PARENT':'root','SPRITE':'panel','ANCHORTYPE1':'SourceLRTargetLR'}},'art':{'panel':'panel'}}
    def hud_audit(self,data):
        (self.root/'local-original/hud-native/manifest.json').write_text(json.dumps(data),encoding='utf8');audit=AssetAudit(self.root);audit.check_hud();return audit
    def test_hud_graph_checks_native_png_hash_dimensions_and_logical_scale(self):
        a=self.hud_audit(self.hud_fixture());self.assertFalse(a.errors);self.assertEqual(a.stats['hudSprites'],1)
    def test_hud_graph_rejects_case_hash_dimensions_logical_scale_and_crop(self):
        for mutate,expected in [(lambda d:d['sprites']['panel'].update(file='panel.png'),'case-mismatch'),
            (lambda d:d['sprites']['panel'].update(sha256='0'*64),'hud-png-hash-mismatch'),
            (lambda d:d['sprites']['panel'].update(width=8),'hud-png-dimension-mismatch'),
            (lambda d:d['sprites']['panel'].update(logicalWidth=2),'hud-logical-scale-mismatch'),
            (lambda d:d['sprites']['panel']['sourceSprite'].update(rect=[2,0,4,4]),'hud-source-crop-outside-texture')]:
            data=self.hud_fixture();mutate(data);self.assertIn(expected,[e['kind'] for e in self.hud_audit(data).errors])
    def test_hud_graph_rejects_missing_and_cyclic_anchors_and_sprite_bindings(self):
        for mutate,expected in [(lambda d:d['widgets']['panel'].update(ANCHORNAME1='missing'),'hud-missing-widget-anchor'),
            (lambda d:d['widgets']['root'].update(PARENT='panel'),'hud-widget-anchor-cycle'),
            (lambda d:d['art'].update(panel='missing'),'hud-art-missing-sprite'),
            (lambda d:d['widgets']['panel'].update(SPRITE='wrong'),'hud-art-widget-sprite-mismatch')]:
            data=self.hud_fixture();mutate(data);self.assertIn(expected,[e['kind'] for e in self.hud_audit(data).errors])
    def test_rule_profile_release_requires_the_third_binary_without_base_fallback(self):
        for file in ('openepoch.wasm','openepoch-ee2.wasm'):(self.root/file).write_bytes(b'wasm-fixture')
        (self.root/'rules-profile.js').write_text('export const UP15_PROFILE_ENABLED=true;',encoding='utf8')
        audit=AssetAudit(self.root);audit.check_rule_profiles();self.assertEqual(audit.stats['numericalRuleProfileBinaries'],3);self.assertIn('missing-runtime-file',[e['kind'] for e in audit.errors])
        (self.root/'openepoch-ee2-up15.wasm').write_bytes(b'up-wasm-fixture');audit=AssetAudit(self.root);audit.check_rule_profiles();self.assertFalse(audit.errors)
    def diplomacy_fixture(self):
        data=self.hud_fixture();folder=self.root/'local-original/diplomacy-native';folder.mkdir(parents=True,exist_ok=True)
        (folder/'Panel.png').write_bytes((self.root/'local-original/hud-native/Panel.png').read_bytes())
        data.update(format='ee2-native-diplomacy-v1',fonts={'native':{'NAME':'native'}},fontStyles={'white':{'FONT':'native'}},frameStyles={'frame':{'SPRITE':'panel'}},playerStripPixels={})
        data['sprites']['panel']['sourceResolutionScale']=4
        data['widgets']['panel'].update(FRAMESTYLE='frame',FONTSTYLE='white')
        raw=b'\xff'*64
        for key in ('spr_dip_pcolor_strip_glow','spr_dip_pcolor_strip_glow_on','spr_dip_pcolor_strip_2state','spr_dip_pcolor_strip_2state_on'):
            data['sprites'][key]=dict(data['sprites']['panel'],sourceResolutionScale=1,logicalWidth=4,logicalHeight=4)
            data['playerStripPixels'][key]={'width':4,'height':4,'pngFile':'Panel.png','rgba':base64.b64encode(raw).decode(),'rgbaSha256':hashlib.sha256(raw).hexdigest()}
        return data
    def diplomacy_audit(self,data):
        (self.root/'local-original/diplomacy-native/manifest.json').write_text(json.dumps(data),encoding='utf8');audit=AssetAudit(self.root);audit.check_diplomacy();return audit
    def test_diplomacy_graph_checks_mixed_native_scales_and_embedded_strips(self):
        audit=self.diplomacy_audit(self.diplomacy_fixture());self.assertFalse(audit.errors);self.assertEqual(audit.stats['diplomacyPlayerStrips'],4)
    def test_diplomacy_graph_rejects_image_path_hash_dimensions_and_scale(self):
        for mutate,expected in [(lambda d:d['sprites']['panel'].update(file='panel.png'),'case-mismatch'),
            (lambda d:d['sprites']['panel'].update(sha256='0'*64),'diplomacy-png-hash-mismatch'),
            (lambda d:d['sprites']['panel'].update(width=8),'diplomacy-png-dimension-mismatch'),
            (lambda d:d['sprites']['panel'].update(sourceResolutionScale=1),'diplomacy-logical-scale-mismatch'),
            (lambda d:d['sprites']['panel']['sourceSprite'].update(rect=[1,0,4,4]),'diplomacy-source-crop-outside-texture')]:
            data=self.diplomacy_fixture();mutate(data);self.assertIn(expected,[e['kind'] for e in self.diplomacy_audit(data).errors])
    def test_diplomacy_graph_rejects_anchor_binding_font_and_embedded_rgba_errors(self):
        for mutate,expected in [(lambda d:d['widgets']['panel'].update(ANCHORNAME1='missing'),'diplomacy-missing-widget-anchor'),
            (lambda d:d['widgets']['root'].update(PARENT='panel'),'diplomacy-widget-anchor-cycle'),
            (lambda d:d['widgets']['panel'].update(SPRITE='wrong'),'diplomacy-missing-widget-binding'),
            (lambda d:d['fontStyles']['white'].update(FONT='missing'),'diplomacy-style-missing-font'),
            (lambda d:d['playerStripPixels']['spr_dip_pcolor_strip_glow'].update(rgba='bad!'),'diplomacy-invalid-strip-rgba')]:
            data=self.diplomacy_fixture();mutate(data);self.assertIn(expected,[e['kind'] for e in self.diplomacy_audit(data).errors])
    def font_fixture(self):
        hud=self.hud_fixture();folder=self.root/'local-original/fonts-native';folder.mkdir(parents=True,exist_ok=True);(folder/'Atlas.png').write_bytes((self.root/'local-original/hud-native/Panel.png').read_bytes())
        image={'file':'Atlas.png','sha256':sha(folder/'Atlas.png'),'width':4,'height':4};cell={'x':0,'y':0,'width':4,'reserved':0}
        cache={'format':'ee2-original-font-cache-v1','fonts':{'source':{'file':'Atlas.png','pngSha256':image['sha256'],'width':4,'height':4,'glyphs':{'65':cell},'kerning':[[65,65,-1]],'headerObjectOffsets':{'0x2c':4}}}}
        (folder/'manifest.json').write_text(json.dumps(cache),encoding='utf8')
        glyph={'file':'Atlas.png','x':0,'y':0,'width':4,'height':4,'origin':'current-headless-gdi'};glyphs={str(ord(cp)):dict(glyph) for cp in 'ÄÖÜäöüß'};glyphs['65']=dict(glyph,origin='shipped-cache')
        return {'format':'ee2-native-bitmap-font-v1','sourceCacheManifestSha256':sha(folder/'manifest.json'),'images':{'Atlas.png':image},'fonts':{'font':{'nominalSize':4,'lineHeight':4,'flags':64,'spacing':0,'sourceCache':{'key':'source'},'kerning':{'65:65':-1},'glyphs':glyphs}},'bindings':{'native':['font']*6},'fontStyles':{'white':{'FONT':'native'}}}
    def font_audit(self,data):
        (self.root/'local-original/fonts-native/bitmap-manifest.json').write_text(json.dumps(data),encoding='utf8');audit=AssetAudit(self.root);audit.check_fonts();return audit
    def test_font_graph_checks_source_cache_images_glyphs_and_all_tiers(self):
        audit=self.font_audit(self.font_fixture());self.assertFalse(audit.errors);self.assertEqual(audit.stats['nativeFontOriginalGlyphBindings'],1)
    def test_font_graph_rejects_path_hash_and_missing_atlas(self):
        for mutate,expected in [(lambda d:d['images']['Atlas.png'].update(file='atlas.png'),'case-mismatch'),(lambda d:d['images']['Atlas.png'].update(sha256='0'*64),'font-png-hash-mismatch'),(lambda d:d['fonts']['font']['glyphs']['65'].update(file='Missing.png'),'font-glyph-missing-atlas')]:
            data=self.font_fixture();mutate(data);self.assertIn(expected,[row['kind'] for row in self.font_audit(data).errors])
    def test_font_graph_rejects_modified_original_kerning_and_glyph_cells(self):
        for mutate,expected in [(lambda d:d['fonts']['font']['glyphs']['65'].update(x=1),'font-shipped-glyph-changed'),(lambda d:d['fonts']['font'].update(kerning={}), 'font-original-kerning-changed')]:
            data=self.font_fixture();mutate(data);self.assertIn(expected,[row['kind'] for row in self.font_audit(data).errors])
    def test_font_graph_rejects_missing_tiers_styles_german_and_invalid_metrics(self):
        for mutate,expected in [(lambda d:d['bindings']['native'].pop(),'font-invalid-resolution-binding'),(lambda d:d['fontStyles']['white'].update(FONT='missing'),'font-style-missing-font'),(lambda d:d['fonts']['font']['glyphs'].pop(str(ord('ß'))),'font-missing-german-glyph'),(lambda d:d['fonts']['font'].update(lineHeight=5),'font-invalid-native-metrics'),(lambda d:d['fonts']['font'].update(flags='invalid'),'font-invalid-native-metrics')]:
            data=self.font_fixture();mutate(data);self.assertIn(expected,[row['kind'] for row in self.font_audit(data).errors])
    def test_rule_profile_draft_does_not_implicitly_enable_an_absent_up_binary(self):
        for file in ('openepoch.wasm','openepoch-ee2.wasm'):(self.root/file).write_bytes(b'wasm-fixture')
        (self.root/'rules-profile.js').write_text('export const UP15_PROFILE_ENABLED=false;',encoding='utf8');audit=AssetAudit(self.root);audit.check_rule_profiles();self.assertFalse(audit.errors);self.assertEqual(audit.stats['numericalRuleProfileBinaries'],2)

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
