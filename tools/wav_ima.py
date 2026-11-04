"""Decode WAVE_FORMAT_DVI_ADPCM (IMA, 0x11) into browser-compatible PCM16.

Own block decoder for the standard IMA predictor/index algorithm. Mono and
stereo WAVE block layout and nibble ordering were cross-checked against the
primary FFmpeg decoder, not copied as source code:
https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/adpcm.c
No resampling, gain adjustment, denoising, filtering or re-encoding is applied.
"""
import io
import struct
import wave

STEP=(7,8,9,10,11,12,13,14,16,17,19,21,23,25,28,31,34,37,41,45,50,55,60,66,
      73,80,88,97,107,118,130,143,157,173,190,209,230,253,279,307,337,371,408,
      449,494,544,598,658,724,796,876,963,1060,1166,1282,1411,1552,1707,1878,
      2066,2272,2499,2749,3024,3327,3660,4026,4428,4871,5358,5894,6484,7132,
      7845,8630,9493,10442,11487,12635,13899,15289,16818,18500,20350,22385,
      24623,27086,29794,32767)
INDEX=(-1,-1,-1,-1,2,4,6,8)
assert len(STEP)==89

def chunks(raw):
    if len(raw)<12 or raw[:4]!=b'RIFF' or raw[8:12]!=b'WAVE':
        raise ValueError('Not a RIFF/WAVE file')
    end=struct.unpack_from('<I',raw,4)[0]+8
    if end>len(raw):raise ValueError('Truncated RIFF payload')
    pos=12
    result={}
    while pos+8<=end:
        name,size=struct.unpack_from('<4sI',raw,pos);pos+=8
        if pos+size>end:raise ValueError('Truncated WAVE chunk')
        result.setdefault(name,[]).append(raw[pos:pos+size]);pos+=size+(size&1)
    return result

def info(raw):
    c=chunks(raw)
    if b'fmt ' not in c or len(c[b'fmt '][0])<16:raise ValueError('Missing WAVE format')
    fmt=c[b'fmt '][0]
    codec,channels,rate,average,align,bits=struct.unpack_from('<HHIIHH',fmt)
    return {'codec':codec,'channels':channels,'sample_rate':rate,'bits_per_sample':bits,
            'block_align':align,'average_bytes_per_second':average,
            'samples_per_block':struct.unpack_from('<H',fmt,18)[0] if codec==0x11 and len(fmt)>=20 else None}

def decode(raw):
    c=chunks(raw);format=info(raw);channels=format['channels'];align=format['block_align']
    if format['codec']!=0x11 or format['bits_per_sample']!=4:
        raise ValueError('Only 4-bit WAVE_FORMAT_DVI_ADPCM is supported')
    if channels not in (1,2) or align<4*channels or format['sample_rate']<=0:
        raise ValueError('Invalid IMA channel/block format')
    per_block=1+(align-4*channels)*2//channels
    if format['samples_per_block']!=per_block or (align-4*channels)%(4*channels):
        raise ValueError('Invalid IMA samples-per-block or channel grouping')
    payload=b''.join(c.get(b'data',[]))
    if not payload:raise ValueError('Missing ADPCM data')
    output=[]
    for start in range(0,len(payload),align):
        block=payload[start:start+align]
        if len(block)<4*channels or (len(block)-4*channels)%(4*channels):
            raise ValueError('Truncated IMA block/channel group')
        histories=[];indices=[];decoded=[]
        for channel in range(channels):
            predictor,index,_=struct.unpack_from('<hBB',block,channel*4)
            if index>88:raise ValueError('Invalid IMA step index')
            histories.append(predictor);indices.append(index);decoded.append([predictor])
        cursor=4*channels
        while cursor<len(block):
            for channel in range(channels):
                for value in block[cursor:cursor+4]:
                    for nibble in (value&15,value>>4):
                        step=STEP[indices[channel]]
                        delta=(step>>3)+(step if nibble&4 else 0)+(step>>1 if nibble&2 else 0)+(step>>2 if nibble&1 else 0)
                        predictor=histories[channel]+(-delta if nibble&8 else delta)
                        histories[channel]=max(-32768,min(32767,predictor))
                        indices[channel]=max(0,min(88,indices[channel]+INDEX[nibble&7]))
                        decoded[channel].append(histories[channel])
                cursor+=4
        if any(len(samples)!=len(decoded[0]) for samples in decoded):raise ValueError('Unequal IMA channel lengths')
        for frame in zip(*decoded):output.extend(frame)
    if b'fact' in c and len(c[b'fact'][0])>=4:
        frames=struct.unpack_from('<I',c[b'fact'][0])[0]
        if frames>len(output)//channels:raise ValueError('WAVE fact exceeds decoded samples')
        output=output[:frames*channels]
    frames=len(output)//channels
    pcm=struct.pack('<'+'h'*len(output),*output);stream=io.BytesIO()
    with wave.open(stream,'wb') as wav:
        wav.setnchannels(channels);wav.setsampwidth(2);wav.setframerate(format['sample_rate']);wav.writeframes(pcm)
    return stream.getvalue(),{'codec':'PCM signed 16-bit little-endian','channels':channels,
        'sample_rate':format['sample_rate'],'bits_per_sample':16,'frames':frames,
        'duration':frames/format['sample_rate'],'source_format':format,
        'transformation':'Decoded 4-bit IMA ADPCM to interleaved PCM16; original channel count and sample rate retained; no resampling, filtering or normalization.'}
