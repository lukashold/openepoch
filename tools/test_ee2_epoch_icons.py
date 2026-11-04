"""Verify final native epoch icons against the installed CSV/DDF/sprite pixels."""
import csv
import hashlib
import io
import json
from pathlib import PurePosixPath, PureWindowsPath
import re
import unittest
import zipfile

from PIL import Image
from extract_ee2_textures import ROOT, INSTALL, decode_texcache
from import_ee2 import block, without_comments

def sha(data):return hashlib.sha256(data).hexdigest()

class NativeEpochIconTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest=json.loads((ROOT/'docs/local-original/textures-manifest.json').read_text(encoding='utf8'))
        cls.icons=cls.manifest['nativeSpriteIcons']
        cls.db=zipfile.ZipFile(INSTALL/'zips/db.zip')
        cls.textures=zipfile.ZipFile(INSTALL/'zips/textures.zip')

    @classmethod
    def tearDownClass(cls):cls.db.close();cls.textures.close()

    def test_exact_six_native_source_identities(self):
        expected={('HeavyArtillery',10,'HeavyArtillery_10'),('LightArtillery2',15,'LightArtillery2_15'),
                  ('HeavyTank',15,'HeavyMounted1_15'),('Submarine',15,'Submarine_15'),
                  ('Fighter',15,'Fighter_15'),('Bomber',14,'Bomber_14')}
        self.assertEqual(len(self.icons),6)
        self.assertEqual({(i['sourceUnit'],i['sourceEpoch'],i['sourceVisual']) for i in self.icons.values()},expected)
        self.assertEqual(self.manifest['errors'],[])
        for key,icon in self.icons.items():
            self.assertEqual(key,icon['sprite'].casefold())
            self.assertEqual((icon['width'],icon['height']),(77,60))
            self.assertEqual(icon['bindingType'],'exact-native-sprite')
            self.assertNotIn('currentDef',icon)
            self.assertNotIn('reusedFromDef',icon)
            self.assertTrue(icon['url'].endswith('-sprite.png'))

    def test_original_csv_visual_and_sprite_chain(self):
        upgrade_member='db/TechTree/upgrade_unittypes.csv';sprite_member='db/UI/UnpackedSprites/dbsprite_unitIcons.csv'
        upgrade_raw=self.db.read(upgrade_member);sprite_raw=self.db.read(sprite_member)
        def rows(raw):
            lines=raw.decode('cp1252').replace('\r','').splitlines()
            header=next(csv.reader([lines[0].removeprefix('//').strip()]))
            return [dict(zip(header,next(csv.reader([line])))) for line in lines[1:] if line.strip() and not line.lstrip().startswith('//')]
        upgrades=rows(upgrade_raw);sprites=rows(sprite_raw)
        for icon in self.icons.values():
            native=next(r for r in upgrades if r['UNIT']==icon['sourceUnit'] and int(float(r['EPOCH']))==icon['sourceEpoch'])
            self.assertEqual(native['VISUAL'],icon['sourceVisual'])
            self.assertEqual(native['ICON'],icon['sprite'])
            self.assertEqual(sha(upgrade_raw),icon['unitUpgradeSha256'])
            self.assertEqual(sha(sprite_raw),icon['spriteSourceSha256'])
            member=icon['visualSource'].split('!',1)[1].rsplit(':',1)[0]
            raw=self.db.read(member);self.assertEqual(sha(raw),icon['visualSourceSha256'])
            visual=block(without_comments(raw.decode('cp1252')),'UnitModel',native['VISUAL'])
            self.assertEqual(re.search(r'\bDefaultModel\s*=\s*"?([\w.\\]+)',visual,re.I)[1],icon['declaredDefaultModel'])
            sprite=next(s for s in sprites if s['NAME'].casefold()==icon['resolvedSprite'].casefold())
            self.assertEqual(sprite['NAME'].casefold(),native['ICON'].casefold())
            self.assertEqual(PureWindowsPath(sprite['FILENAME']).stem.casefold(),PurePosixPath(icon['archive_member']).stem.casefold())
            self.assertEqual(icon['spriteRect'],[int(float(sprite[k])) for k in ('U','V','WIDTH','HEIGHT')])
            # Line references identify actual declarations, not merely archive files.
            for key,raw in [('unitUpgradeSource',upgrade_raw),('spriteSource',sprite_raw),('visualSource',self.db.read(member))]:
                line_no=int(icon[key].rsplit(':',1)[1]);line=raw.decode('cp1252').replace('\r','').splitlines()[line_no-1]
                token=native['UPGRADE'] if key=='unitUpgradeSource' else icon['resolvedSprite'] if key=='spriteSource' else native['VISUAL']
                self.assertIn(token,line)

    def test_native_sprite_pixels_and_hashes(self):
        for icon in self.icons.values():
            raw=self.textures.read(icon['archive_member']);self.assertEqual(sha(raw),icon['source_sha256'])
            original,_=decode_texcache(raw)
            u,v,w,h=icon['spriteRect'];expected=original.crop((u,v,u+w,v+h))
            png=(ROOT/'docs'/icon['url']).read_bytes();self.assertEqual(sha(png),icon['png_sha256'])
            with Image.open(io.BytesIO(png)) as actual:
                self.assertEqual(actual.size,(77,60));self.assertEqual(actual.mode,'RGBA')
                self.assertEqual(actual.tobytes(),expected.tobytes())

    def test_all_eight_missing_transitions_have_their_exact_source_sprite(self):
        models=json.loads((ROOT/'docs/local-original/models/manifest.json').read_text(encoding='utf8'))
        wanted={(20,9),(25,14),(26,14),(34,14),(36,14),(37,14),(38,13),(39,14)}
        found=set()
        for row in models['models']:
            key=(row['id'],row['epoch'])
            if key not in wanted:continue
            p=row['progressionBinding']
            candidates=[i for i in self.icons.values() if (i['sourceUnit'],i['sourceVisual'],i['sourceEpoch'])==
                        (p['sourceUnit'],p['sourceVisual'],row['epoch']+1)]
            self.assertEqual(len(candidates),1)
            upgrade_line=int(p['upgradeSource']['line'])
            source_raw=self.db.read(p['upgradeSource']['entry'])
            source_line=source_raw.decode('cp1252').replace('\r','').splitlines()[upgrade_line-1]
            self.assertIn(','+candidates[0]['sprite']+',',source_line)
            found.add(key)
        self.assertEqual(found,wanted)

    def test_future_archetype_base_icons_are_explicit_visual_adapters(self):
        expected={44:('Fighter',15,'icon_unit_AtmosFighter'),45:('Submarine',15,'icon_unit_Submarine_15'),
                  46:('HeavyTank',15,'icon_unit_MyrmidonTank')}
        for id,(unit,epoch,sprite) in expected.items():
            binding=self.manifest['iconBindings'][str(id)];icon=self.manifest['icons'][str(id)]
            self.assertEqual(binding['bindingType'],'role-adapter')
            self.assertEqual((binding['originalUnit'],binding['originalEpoch'],binding['sprite']),(unit,epoch,sprite))
            native=self.icons[sprite.casefold()]
            self.assertEqual(icon['url'],native['url']);self.assertEqual(icon['png_sha256'],native['png_sha256'])

    def test_fortress_seven_exact_csv_sprites_and_native_cropped_pixels(self):
        raw=self.db.read('db/TechTree/upgrade_unittypes.csv')
        lines=raw.decode('cp1252').splitlines()
        header=next(csv.reader([lines[0].removeprefix('//').strip()]))
        upgrades=[dict(zip(header,next(csv.reader([line])))) for line in lines[1:]
                  if line.strip() and not line.lstrip().startswith('//')]
        stages=[r for r in upgrades if r['UNIT']=='Fortress']
        self.assertEqual([int(r['EPOCH']) for r in stages],[1,4,6,8,10,12,14])
        raw=self.db.read('db/UI/UnpackedSprites/dbsprite_unitIcons.csv')
        lines=raw.decode('cp1252').splitlines()
        header=next(csv.reader([lines[0].removeprefix('//').strip()]))
        sprites={r['NAME'].casefold():r for line in lines[1:] if line.strip() and not line.lstrip().startswith('//')
                 for r in [dict(zip(header,next(csv.reader([line]))))]}
        for row in stages:
            name=row['ICON'].casefold()
            resolved=name+'_we' if name+'_we' in sprites else name
            icon=self.manifest['iconVariants'][resolved]
            sprite=sprites[resolved]
            self.assertIn(resolved,[s.casefold() for s in icon['resolvedSprites']])
            self.assertEqual(PureWindowsPath(sprite['FILENAME']).stem.casefold(),PurePosixPath(icon['archive_member']).stem.casefold())
            rect=[int(float(sprite[k])) for k in ('U','V','WIDTH','HEIGHT')]
            self.assertEqual(rect,icon['spriteRect'])
            self.assertEqual(icon['spriteCropStatus'],'source-verified')
            cached=self.textures.read(icon['archive_member'])
            self.assertEqual(sha(cached),icon['source_sha256'])
            source,_=decode_texcache(cached);u,v,w,h=rect
            png=(ROOT/'docs'/icon['url']).read_bytes();self.assertEqual(sha(png),icon['png_sha256'])
            with Image.open(io.BytesIO(png)) as exported:
                self.assertEqual(exported.size,(w,h))
                self.assertEqual(exported.tobytes(),source.crop((u,v,u+w,v+h)).tobytes())
        base=self.manifest['iconBindings']['56']
        self.assertEqual((base['originalUnit'],base['originalEpoch'],base['sprite']),('Fortress',1,'icon_unit_fortress_2'))
        self.assertEqual(self.manifest['icons']['56']['url'],self.manifest['iconVariants']['icon_unit_fortress_2_we']['url'])

if __name__=='__main__':unittest.main()
