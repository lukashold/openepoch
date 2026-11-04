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
import math
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
                                       'sourceTextures':model.get('sourceTextures',[]),
                                       'nativeFarmComposite':model.get('nativeFarmComposite'),
                                       'parts':[{key:part.get(key) for key in ('sourceNode','name','textureSource','textureUrl','farmPlot','farmPlotOffset')} for part in model.get('parts',[])]}
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

    def check_rule_profiles(self):
        name='rules-profile.js';source=self.files[name].read_text(encoding='utf8')
        enabled=re.search(r'^export\s+const\s+UP15_PROFILE_ENABLED\s*=\s*(true|false)\s*;',source,re.M)
        if not enabled:self.error('missing-explicit-runtime-profile-switch',name);return
        files=['openepoch.wasm','openepoch-ee2.wasm']
        if enabled[1]=='true':files.append('openepoch-ee2-up15.wasm')
        for file in files:self.ref(file,name,'numerical-profile-wasm')
        self.stats['numericalRuleProfileBinaries']=len(files)

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
        self.check_models();self.check_textures_and_ground();self.check_teams();self.check_model_seasons();self.check_audio();self.check_static_runtime()
        if 'original-water.js' in self.files:self.check_water()
        if 'local-original/farm-composites.json' in self.files:self.check_farms()
        if 'original-hud.js' in self.files:self.check_hud()
        if 'rules-profile.js' in self.files:self.check_rule_profiles()
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
