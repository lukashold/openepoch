"""Verify BASE code/data equality except Rust panic source-line metadata."""
import hashlib, json, pathlib, struct
ROOT = pathlib.Path(__file__).resolve().parents[3]
sha = lambda b: hashlib.sha256(b).hexdigest()
def leb(b,p,signed=False):
    value=shift=0
    while True:
        v=b[p];p+=1;value|=(v&127)<<shift;shift+=7
        if v<128:break
    if signed and v&64:value-=1<<shift
    return value,p
def sections(b):
    assert b[:8]==b'\0asm\x01\0\0\0'
    out={};p=8
    while p<len(b):
        identity=b[p];p+=1;size,p=leb(b,p);out[identity]=(p,b[p:p+size]);p+=size
    return out
def data_memory(sections):
    _,d=sections[11];count,p=leb(d,0);memory={}
    for _ in range(count):
        flags,p=leb(d,p);assert flags==0 and d[p]==0x41;p+=1
        base,p=leb(d,p,True);assert d[p]==0x0b;p+=1
        n,p=leb(d,p)
        for k in range(n):memory[base+k]=d[p+k]
        p+=n
    return memory
def verify():
    before=(ROOT/'work/city-ground-baseline/openepoch.wasm').read_bytes()
    after=(ROOT/'outputs/OpenEpoch/docs/openepoch.wasm').read_bytes()
    a=sections(before);b=sections(after);assert set(a)==set(b)
    for identity in a:
        if identity!=11:assert a[identity][1]==b[identity][1],f'changed WASM section{identity}'
    am,bm=data_memory(a),data_memory(b);assert am.keys()==bm.keys()
    changed={p for p in am if am[p]!=bm[p]};remaining=changed.copy();records=[]
    for addr in sorted(changed):
        if addr not in remaining:continue
        word=addr//4*4
        raw=bytes(am.get(p,0)for p in range(word-8,word+8))
        file_ptr,file_len,line,col=struct.unpack('<IIII',raw)
        assert 1<=file_len<=128 and 0<line<10000 and 0<col<1000
        file=bytes(am.get(p,0)for p in range(file_ptr,file_ptr+file_len)).decode('utf8')
        assert file.startswith('src\\') or file.startswith('src/'),file
        now=struct.unpack('<I',bytes(bm[p]for p in range(word,word+4)))[0]
        assert 0<now<10000
        records.append({'file':file,'beforeLine':line,'afterLine':now,'column':col})
        remaining-=set(range(word,word+4))
    assert not remaining
    return {'status':'passed','baseline':{'sha256':sha(before),'bytes':len(before)},'current':{'sha256':sha(after),'bytes':len(after)},'codeSectionSha256':sha(a[10][1]),'allOtherSectionsIdentical':True,'dataDifferences':{'changedBytes':len(changed),'sourceLineRecords':len(records),'files':sorted(set(r['file']for r in records))},'limit':'Binary differs only in panic source-line metadata. Executable code and all non-location data are byte-identical.'}
if __name__=='__main__':
    result=verify();path=ROOT/'outputs/EE2-Research/analysis/native-city-ground-base-equivalence.json'
    result['build']='combat-season1'
    result['wasm']={name:{'sha256':sha((ROOT/f'outputs/OpenEpoch/docs/{file}').read_bytes()),'bytes':(ROOT/f'outputs/OpenEpoch/docs/{file}').stat().st_size}for name,file in [('ee2','openepoch-ee2.wasm'),('base','openepoch.wasm')]}
    path.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    print(json.dumps(result))
