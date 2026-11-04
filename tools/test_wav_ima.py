"""Known-vector checks for the legacy original ambient WAV conversion."""
import io
import json
import pathlib
import struct
import unittest
import wave
from wav_ima import decode,info

def riff(blocks,channels=1,align=8,fact=None):
    samples=1+(align-4*channels)*2//channels
    fmt=struct.pack('<HHIIHHHH',0x11,channels,8000,8000*align//samples,align,4,2,samples)
    chunks=[(b'fmt ',fmt)]
    if fact is not None:chunks.append((b'fact',struct.pack('<I',fact)))
    chunks.append((b'data',b''.join(blocks)))
    payload=b'WAVE'+b''.join(name+struct.pack('<I',len(data))+data+(b'\0' if len(data)&1 else b'') for name,data in chunks)
    return b'RIFF'+struct.pack('<I',len(payload))+payload

def samples(raw):
    with wave.open(io.BytesIO(raw)) as wav:
        data=wav.readframes(wav.getnframes());return list(struct.unpack('<'+'h'*(len(data)//2),data))

class ImaTests(unittest.TestCase):
    def test_mono_known_nibble_order_and_adaptation(self):
        # Low/high nibbles are 1..8, exercising magnitude, sign and index updates.
        block=struct.pack('<hBB',0,0,0)+bytes([0x21,0x43,0x65,0x87])
        decoded,meta=decode(riff([block]))
        self.assertEqual(samples(decoded),[0,1,4,8,15,27,47,88,82])
        self.assertEqual(meta['frames'],9);self.assertEqual(meta['sample_rate'],8000)

    def test_stereo_channel_groups(self):
        block=struct.pack('<hBBhBB',1000,0,0,-1000,0,0)+b'\x11'*4+b'\x99'*4
        decoded,meta=decode(riff([block],channels=2,align=16))
        self.assertEqual(samples(decoded),[sample for n in range(1000,1009) for sample in (n,-n)])
        self.assertEqual(meta['channels'],2);self.assertEqual(meta['frames'],9)

    def test_block_reset_and_fact_trim(self):
        blocks=[struct.pack('<hBB',initial,0,0)+b'\0'*4 for initial in (0,100)]
        decoded,meta=decode(riff(blocks,fact=12))
        self.assertEqual(samples(decoded),[0]*9+[100]*3);self.assertEqual(meta['frames'],12)

    def test_predictor_clipping_and_invalid_index(self):
        for initial,byte,limit in [(32760,0x77,32767),(-32760,0xff,-32768)]:
            block=struct.pack('<hBB',initial,88,0)+bytes([byte])*4
            self.assertEqual(samples(decode(riff([block]))[0])[1:],[limit]*8)
        with self.assertRaises(ValueError):decode(riff([struct.pack('<hBB',0,89,0)+b'\0'*4]))

    def test_actual_original_and_playback_manifest(self):
        root=pathlib.Path(__file__).resolve().parents[1]/'docs/local-original/audio'
        manifest=json.loads((root/'manifest.json').read_text(encoding='utf8'))
        original=(root/'ambloop_land_01_temp.wav').read_bytes()
        self.assertEqual(info(original)['codec'],0x11)
        pcm,metadata=decode(original);self.assertEqual(pcm,(root/manifest['ambient']['loop']).read_bytes())
        self.assertEqual(metadata['frames'],132310);self.assertEqual(metadata['duration'],16.53875)
        with wave.open(io.BytesIO(pcm)) as wav:
            self.assertEqual(wav.getcomptype(),'NONE');self.assertEqual(wav.getsampwidth(),2)
            self.assertEqual(wav.getnchannels(),1);self.assertEqual(wav.getframerate(),8000)

if __name__=='__main__':unittest.main()
