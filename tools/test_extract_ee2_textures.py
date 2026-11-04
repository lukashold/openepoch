"""Regression checks for the independently implemented texture-container reader.
Synthetic blocks avoid shipping original source texture fixtures in test code.
"""
import struct
import unittest
from extract_ee2_textures import decode_texcache

def cache(fmt=4,payload=None,mips=None):
    if payload is None: payload=struct.pack('<HHI',0xf800,0,0) # red DXT1 block
    if mips is None: mips=[(4,4,0)]
    header=b'Gamebryo File Format, Version 10.1.0.0\n'
    header+=struct.pack('<IIIHI',0x0a010000,0,1,1,11)+b'NiPixelData'+struct.pack('<HII',0,0,0)
    pixel_format=struct.pack('<6I',fmt,0,0,0,0,0)+struct.pack('<8BI',fmt,0,0,0,0,0,0,0,0)
    body=struct.pack('<iII',-1,len(mips),0)+b''.join(struct.pack('<III',*m) for m in mips)+struct.pack('<I',len(payload))+payload
    return header+pixel_format+body+struct.pack('<II',1,0)

class TextureCacheTests(unittest.TestCase):
    def test_dxt1_payload_and_dimensions(self):
        image,meta=decode_texcache(cache())
        self.assertEqual(image.size,(4,4));self.assertEqual(image.getpixel((0,0)),(255,0,0,255))
        self.assertEqual(meta['format'],'DXT1')

    def test_mipmap_metadata_does_not_leak_lower_mip(self):
        raw=cache(payload=struct.pack('<HHI',0xf800,0,0)+struct.pack('<HHI',0x001f,0,0),mips=[(4,4,0),(2,2,8)])
        image,meta=decode_texcache(raw)
        self.assertEqual(image.getpixel((3,3)),(255,0,0,255));self.assertEqual(meta['mipmaps_in_source'],2)

    def test_dxt3_preserves_four_bit_alpha(self):
        payload=bytes([0x88]*8)+struct.pack('<HHI',0xf800,0,0)
        image,meta=decode_texcache(cache(fmt=5,payload=payload))
        self.assertEqual(image.getpixel((0,0)),(255,0,0,136))
        self.assertEqual(meta['format'],'DXT3')

    def test_unsupported_format_fails(self):
        with self.assertRaisesRegex(ValueError,'unsupported NiPixelData format'): decode_texcache(cache(fmt=2))

    def test_invalid_and_truncated_data_fail(self):
        with self.assertRaises(ValueError):decode_texcache(cache()[:-12])
        raw=bytearray(cache());raw[:4]=b'NOPE'
        with self.assertRaises(ValueError):decode_texcache(raw)

if __name__=='__main__':unittest.main()
