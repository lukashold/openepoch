"""Local integration checks for original EE2 building-ground assets.

The original archives are read-only test inputs. No image fixture is bundled.
Checks follow the DDF/sprite/seasonal/pixel chain independently of the importer,
and verifies the compiled UnitType field conversion before using dimensions.
"""
import csv
import hashlib
import io
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import struct
import unittest
import zipfile

from PIL import Image
from extract_ee2_textures import ROOT, INSTALL, decode_texcache
from import_ee2 import block, without_comments

OUTPUT=ROOT/'docs/local-original'

def digest(data):
    return hashlib.sha256(data).hexdigest()

class OriginalGroundDecalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.compact=json.loads((OUTPUT/'ground-decals.json').read_text(encoding='utf8'))
        cls.audit=json.loads((OUTPUT/'ground-decals-manifest.json').read_text(encoding='utf8'))
        cls.texture_manifest=json.loads((OUTPUT/'textures-manifest.json').read_text(encoding='utf8'))
        cls.db=zipfile.ZipFile(INSTALL/'zips/db.zip')
        cls.textures=zipfile.ZipFile(INSTALL/'zips/textures.zip')

    @classmethod
    def tearDownClass(cls):
        cls.db.close();cls.textures.close()

    def test_preserves_previous_assets_and_rejects_unproved_quads(self):
        original=[f for f in self.texture_manifest['files'] if '/textures/ground/' not in f['url']]
        self.assertEqual(len(original),159)
        epoch_sources={i['archive_member'] for i in self.texture_manifest['nativeSpriteIcons'].values()}
        self.assertEqual(len([f for f in original if f['archive_member'] not in epoch_sources]),153)
        self.assertEqual(len(self.texture_manifest['terrain']),11)
        self.assertEqual(len(self.texture_manifest['icons']),55)
        self.assertEqual(len(self.texture_manifest['iconVariants']),73)
        self.assertEqual(len(self.texture_manifest['hud']),31)
        self.assertEqual(self.texture_manifest['errors'],[])
        ids={e['id'] for e in self.compact['entries']}
        self.assertEqual(ids,{0,1,2,3,4,6,47,48,49,50,51,52,55,56})
        self.assertEqual(self.compact['unresolvedBindings'],[])
        farm=next(e for e in self.compact['entries']if e['id']==2)
        self.assertEqual(farm['width'],struct.unpack('<f',struct.pack('<f',1.5*1.4142135381698608))[0]);self.assertEqual(farm['width'],farm['depth'])
        self.assertEqual(farm['rural'][0]['sprite'],'under_farm')
        # No generic foundation silently assigned to Dock/Outpost/Rig/Mine.
        self.assertTrue(ids.isdisjoint({5,7,53,54}))
        self.assertEqual(self.compact['runtimeSelection']['nativeEpochFirst'],[1,6,11])

    def test_native_dimensions_offsets_and_real_model_children(self):
        expected={0:([3.5,3],4,3,-.25,0),1:([1.5,1.5],1.5,1.5,0,0),
                  3:([2,2],3,2,-.5,0),48:([2.5,2],2.5,2.5,0,.25),
                  52:([2,1],2,1,0,0),56:([3,3],3,3,0,0)}
        for e in self.compact['entries']:
            if e['id'] in expected:
                native,x,y,ox,oy=expected[e['id']];f=lambda value:struct.unpack('<f',struct.pack('<f',value*1.4142135381698608))[0]
                self.assertEqual((e['nativeSize'],e['width'],e['depth'],e['offsetX'],e['offsetY']),(native,f(y),f(x),f(-oy),f(-ox)))
        compact={e['id']:e for e in self.compact['entries']}
        for a in self.audit['entries']:
            e=compact[a['id']]
            member=a['unitType']['source'].split('!',1)[1].rsplit(':',1)[0]
            raw=self.db.read(member);text=without_comments(raw.decode('cp1252'))
            self.assertEqual(digest(raw),a['unitType']['sha256'])
            body=block(text,'UnitType',a['originalUnit'])
            native={k:float(v) for k,v in re.findall(r'\b(Size[XY]|UndertextureSize[XY]|UndertextureOffset[XY])\s*=\s*([-+\d.]+)',body,re.I)}
            self.assertEqual(e['nativeSize'],[native['SizeX'],native['SizeY']])
            self.assertEqual(e['nativeUnitSizing']['raw']['underSize'],[native['UndertextureSizeX'],native['UndertextureSizeY']])
            for axis in ('X','Y'):
                present='UndertextureOffset'+axis in native
                self.assertEqual(axis.lower() in e['offsetDefaults'],not present)
                self.assertEqual(e['nativeUnitSizing']['raw']['underOffset']['XY'.index(axis)],native.get('UndertextureOffset'+axis,0))
            model=block(text,'UnitModel',a['originalUnit'])
            children=re.search(r'ChildNames\s*=\s*\[([^]]*)\]',model)[1].split()
            self.assertIn(a['decal'],children)

    def test_all_state_slots_follow_original_decal_and_sprite_rows(self):
        raw=self.db.read('db/Visuals/visual_decals.ddf')
        text=without_comments(raw.decode('cp1252'))
        rows=list(csv.reader(self.db.read('db/Terrain/dbsprite_overlay.csv').decode('cp1252').splitlines()))
        header=[c.removeprefix('//').strip() for c in rows[0]]
        sprites={row[0].lower():dict(zip(header,row)) for row in rows[1:] if row and not row[0].lstrip().startswith('//')}
        for a in self.audit['entries']+self.audit['unresolvedBindings']:
            self.assertEqual(a['decalSource']['sha256'],digest(raw))
            decal=block(text,'Decal',a['decal'])
            for state,detail in a['states'].items():
                source=re.search(r'\{\s*StateName\s*=\s*'+state+r'\b(.*?)\}',decal,re.S)[1]
                for category in ('Rural','Urban'):
                    names=re.search(category+r'SpriteNames\s*=\s*\[([^]]*)\]',source)[1].split()
                    self.assertEqual([s['sprite'] for s in detail[category.lower()]],names)
                    for slot,s in enumerate(detail[category.lower()]):
                        row=sprites[s['sprite'].lower()]
                        self.assertEqual(s['sourceSlot'],slot)
                        self.assertEqual(s['spriteFilename'],row['FILENAME'])
                        self.assertEqual([float(row[k]) for k in ('U','V','WIDTH','HEIGHT')],[0,0,0,0])
                        self.assertEqual(re.findall(r'\w+',row['FLAGS']),['MipMap'])
                        self.assertEqual((s['uv'],s['flipU'],s['flipV'],s['rotate90']),([0,0,1,1],False,False,False))

    def test_seasonal_selection_preserves_every_original_alternative(self):
        raw=self.db.read('db/Terrain/DbSeasonalTextureSets_Temperate.csv')
        rows={r[0].lower():r[1].split() for r in csv.reader(raw.decode('cp1252').splitlines()) if r and not r[0].lstrip().startswith('//')}
        for a in self.audit['entries']+self.audit['unresolvedBindings']:
            for state in a['states'].values():
                for category in ('rural','urban'):
                    for s in state[category]:
                        paths=rows.get(s['spriteFilename'].lower(),[s['spriteFilename']])
                        self.assertEqual(s['seasonalPaths'],paths)
                        variants={v['originalPath']:v for v in s['physicalVariants']}
                        self.assertEqual(set(variants),set(paths))
                        for p,v in variants.items():
                            self.assertEqual(v['sourceIndices'],[i for i,path in enumerate(paths) if path==p])
                            self.assertEqual(PureWindowsPath(p).stem.lower(),PurePosixPath(v['url']).stem.lower())
                        # Common non-snow view is a labelled adapter, not an epoch/season assertion.
                        self.assertEqual(s['selectedPhysicalPath'],max(dict.fromkeys(paths),key=paths.count))
                        self.assertEqual(s['url'],variants[s['selectedPhysicalPath']]['url'])

    def test_exported_pixels_equal_original_top_mips_without_transforms(self):
        self.assertEqual(len(self.audit['textures']),125)
        transparent=0
        for url,a in self.audit['textures'].items():
            raw=self.textures.read(a['archive_member'])
            self.assertEqual(digest(raw),a['source_sha256'])
            png=(ROOT/'docs'/url).read_bytes()
            self.assertEqual(digest(png),a['png_sha256'])
            original,meta=decode_texcache(raw)
            with Image.open(io.BytesIO(png)) as exported:
                self.assertEqual(exported.mode,'RGBA')
                self.assertEqual(exported.size,original.size)
                self.assertEqual(exported.tobytes(),original.tobytes())
                self.assertEqual(a['alphaRange'],list(original.getchannel('A').getextrema()))
                self.assertEqual(a['alphaBBox'],list(original.getchannel('A').getbbox() or (0,0,0,0)))
                transparent+=int(a['alphaRange'][0]<255)
        self.assertGreater(transparent,100)
        tc=self.audit['textures']['local-original/textures/ground/under_citycenter_1.png']
        self.assertEqual((tc['width'],tc['height'],tc['format'],tc['alphaRange']),(256,256,'DXT3',[0,255]))

    def test_audit_sources_hash_exact_installed_archives(self):
        for name,info in self.audit['archives'].items():
            self.assertEqual(digest((INSTALL/'zips'/name).read_bytes()),info['sha256'])
        for member,source in self.audit['sources'].items():
            self.assertEqual(source['archive'],'db.zip')
            self.assertEqual(digest(self.db.read(member)),source['sha256'])
        self.assertEqual(self.audit['archives']['textures.zip']['sha256'],self.texture_manifest['archive_sha256'])

if __name__=='__main__':unittest.main()
