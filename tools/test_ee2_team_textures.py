"""Independent byte-layout and asset checks for the original sparse DXT patches."""
import hashlib
import json
import os
import pathlib
import struct
import unittest
import zipfile
from PIL import Image
from extract_ee2_team_textures import apply_patch,pixel_cache,INSTALL,ROOT

def cache(fmt,levels,pixels):
    name=b'NiPixelData'
    raw=b'Gamebryo File Format, Version 10.1.0.0\n'+struct.pack('<IIIH',0x0a010000,0,1,1)
    raw+=struct.pack('<I',len(name))+name+struct.pack('<HII',0,0,0)
    raw+=struct.pack('<6I',fmt,0,0,0,0,8 if fmt==2 else 0)+bytes(8)
    raw+=struct.pack('<IiII',0,-1,len(levels),1 if fmt==2 else 0)
    raw+=b''.join(struct.pack('<III',*level) for level in levels)
    return raw+struct.pack('<I',len(pixels))+pixels+struct.pack('<Ii',1,0)

def patch(records,pixels):
    stream=struct.pack('<I',len(records))+b''.join(struct.pack('<II',*record) for record in records)+pixels
    return cache(2,[(len(stream),1,0)],stream)

class LayoutTests(unittest.TestCase):
    def test_two_mips_and_untouched_blocks(self):
        base=cache(4,[(8,8,0),(4,4,32)],bytes(range(40)))
        replacement=b'X'*16+b'Y'*8
        delta=patch([(1,2),(0xffffffff,0),(0,1),(0xffffffff,0)],replacement)
        modified,metadata=apply_patch(base,delta);payload=pixel_cache(modified)['payload']
        self.assertEqual(payload,bytes(range(8))+b'X'*16+bytes(range(24,32))+b'Y'*8)
        self.assertEqual(metadata['mip_terminators'],2);self.assertEqual(metadata['replaced_blocks'],3)
        self.assertEqual(metadata['replacement_bytes'],24)
        # Source bytes and original cache header/footer remain untouched.
        self.assertEqual(pixel_cache(base)['payload'],bytes(range(40)))
        start=pixel_cache(base)['payload_offset'];self.assertEqual(base[:start],modified[:start])
        self.assertEqual(base[start+40:],modified[start+40:])

    def test_dxt3_and_dxt5_use_complete_16_byte_blocks(self):
        for fmt in (5,6):
            base=cache(fmt,[(4,4,0)],b'A'*16)
            modified,metadata=apply_patch(base,patch([(0,1),(0xffffffff,0)],b'B'*16))
            self.assertEqual(pixel_cache(modified)['payload'],b'B'*16);self.assertEqual(metadata['dxt_block_bytes'],16)

    def test_invalid_runs_terminators_and_trailing_payload_rejected(self):
        base=cache(4,[(8,8,0)],b'A'*32)
        invalid=[
            patch([(4,1),(0xffffffff,0)],b'B'*8),
            patch([(1,2),(2,1),(0xffffffff,0)],b'B'*24),
            patch([(0,1)],b'B'*8),
            patch([(0,1),(0xffffffff,1)],b'B'*16),
            patch([(0,1),(0xffffffff,0)],b'B'*8+b'!'),
        ]
        for raw in invalid:
            with self.assertRaises(ValueError):apply_patch(base,raw)

class ActualAssetsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.archive=zipfile.ZipFile(pathlib.Path(os.environ.get('EE2_INSTALL',INSTALL))/'zips/textures.zip')
        cls.manifest=json.loads((ROOT/'docs/local-original/team-textures/manifest.json').read_text(encoding='utf8'))
    @classmethod
    def tearDownClass(cls):cls.archive.close()

    def test_known_original_stream_counts_and_block_addresses(self):
        for stem,records,mips,blocks,block_bytes,first_offset,first_length in [
            ('lsf1_citizen_m_we',158,9,2060,8,38,2),
            ('bld_citycenter1_we',45,11,633,16,24834,74),
        ]:
            base=self.archive.read('cache/texcache/'+stem+'.texcache')
            raw=self.archive.read('cache/pcpcache/'+stem+'A.pcpatch')
            output,metadata=apply_patch(base,raw);original=pixel_cache(base);patched=pixel_cache(output)
            stream=pixel_cache(raw)['payload'];header=4+records*8
            self.assertEqual(metadata['record_count'],records);self.assertEqual(metadata['mip_terminators'],mips)
            self.assertEqual(metadata['replaced_blocks'],blocks);self.assertEqual(metadata['dxt_block_bytes'],block_bytes)
            start=first_offset*block_bytes;size=first_length*block_bytes
            self.assertEqual(patched['payload'][start:start+size],stream[header:header+size])
            self.assertEqual(patched['payload'][:block_bytes],original['payload'][:block_bytes])
            self.assertEqual(len(stream)-header,blocks*block_bytes)

    def test_source_palette_mapping_and_current_four_player_colours(self):
        self.assertTrue(self.manifest['colour_mapping_verified'])
        self.assertEqual(self.manifest['suffix_by_colour'],{'blue':'A','red':'B','green':'C','orange':'D','magenta':'E','cyan':'F','yellow':'G','lavender':'H','purple':'I','gold':'J','grey':'K'})
        for stem in ('lsf1_citizen_m_we','bld_citycenter1_we'):
            entry=self.manifest['textures'][stem];self.assertTrue(entry['verified'])
            for colour,letter in [('blue','A'),('red','B'),('green','C'),('yellow','G')]:
                variant=entry['variants'][letter];self.assertTrue(variant['verified']);self.assertEqual(variant['colour'],colour)
        # Source letter D is orange, so mapping four teams to A/B/C/D is wrong.
        self.assertEqual(self.manifest['textures']['lsf1_citizen_m_we']['variants']['D']['colour'],'orange')

    def test_every_generated_variant_file_dimensions_and_source_hash(self):
        self.assertFalse(self.manifest['errors'])
        seen_sources={};files=0
        for stem,entry in self.manifest['textures'].items():
            source=entry['base_source'];key=source['entry']
            if key not in seen_sources:seen_sources[key]=hashlib.sha256(self.archive.read(key)).hexdigest()
            self.assertEqual(seen_sources[key],source['sha256'])
            self.assertEqual(set(entry['variants']),set('ABCDEFGHIJK'))
            for letter,variant in entry['variants'].items():
                path=(ROOT/'docs'/variant['url']).resolve();path.relative_to((ROOT/'docs/local-original/team-textures').resolve())
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),variant['png_sha256'])
                with Image.open(path) as image:
                    image.load();self.assertEqual(image.size,(entry['width'],entry['height']));self.assertEqual(image.mode,'RGBA')
                source=variant['patch_source'];self.assertEqual(hashlib.sha256(self.archive.read(source['entry'])).hexdigest(),source['sha256'])
                self.assertEqual(variant['replacement_bytes'],variant['replaced_blocks']*variant['dxt_block_bytes'])
                files+=1
        self.assertGreaterEqual(files,22)

if __name__=='__main__':unittest.main()
