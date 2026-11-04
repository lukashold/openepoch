"""Bounded NetImmerse 10.0.1.0 / Gamebryo 10.1.0.0 reader.
Format facts: https://github.com/niftools/nifxml/blob/develop/nif.xml
See NIFXML-NOTICE.txt / NIFXML-LICENSE.txt for schema attribution.
See nif_schema.xml for the description-free parsing metadata. Asset files are data,
never executed. This reader validates counts, record markers and full consumption.
"""
import struct,re,xml.etree.ElementTree as E,json,zipfile,pathlib,ast
XML=pathlib.Path(__file__).with_name('nif_schema.xml')
ROOT=E.parse(XML).getroot();TYPES={x.get('name'):x for x in ROOT if x.get('name')}
TOKENS={x.get('token'):x.get('string') for t in ROOT.findall('token') for x in t if x.get('token')}
VER=0x0a010000
from functools import lru_cache
FIELD_NAMES=sorted({'VER','USER','BSVER','ARG','true','false'}|{f.get('name') for t in ROOT for f in t.findall('field')},key=len,reverse=True)
FIELD_PATTERN=re.compile(r'(?<![\w])(?:'+'|'.join(re.escape(k) for k in FIELD_NAMES)+r')(?![\w])')
def version(s):
 if s.startswith('V'):s=s[1:].replace('_','.')
 p=[int(x) for x in s.split('.')];p += [0]*(4-len(p));return sum(v<<(24-8*i) for i,v in enumerate(p))
def expand(s):
 for _ in range(12):
  old=s
  for k,v in TOKENS.items():s=s.replace(k,v)
  if old==s:break
 return s
def expr(s,d={},arg=0):
 s=expand(s).replace('User Version','USER').replace('BS Header\\BS Version','BSVER').replace('Version','VER').replace('#ARG#','ARG')
 for x in re.findall(r'\b\d+\.\d+(?:\.\d+){0,2}',s):s=s.replace(x,str(version(x)))
 scope={**d,'VER':VER,'USER':0,'BSVER':0,'ARG':arg,'true':1,'false':0}
 s=FIELD_PATTERN.sub(lambda m:str(scope.get(m[0],0)),s)
 # Missing version-specific scalar fields default to zero.
 s=re.sub(r'\b(?:BS Data Flags|Has [A-Za-z ]+|Use Internal)\b','0',s)
 s=s.replace('&&',' and ').replace('||',' or ');s=re.sub(r'!(?!=)',' not ',s)
 try:
  tree=ast.parse(s.strip(),mode='eval')
  allowed=(ast.Expression,ast.Constant,ast.BinOp,ast.UnaryOp,ast.BoolOp,ast.Compare,ast.Add,ast.Sub,ast.Mult,ast.Div,ast.FloorDiv,ast.Mod,ast.BitAnd,ast.BitOr,ast.LShift,ast.RShift,ast.Invert,ast.USub,ast.UAdd,ast.Not,ast.And,ast.Or,ast.Eq,ast.NotEq,ast.Lt,ast.LtE,ast.Gt,ast.GtE)
  if not all(isinstance(n,allowed) for n in ast.walk(tree)):raise ValueError(s)
  return eval(compile(tree,'<nif schema>','eval'),{'__builtins__':{}})
 except Exception as e:raise ValueError('expr '+repr(s)) from e
@lru_cache(maxsize=None)
def applicable(f):
 if f.get('since') and version(f.get('since'))>VER:return False
 if f.get('until') and version(f.get('until'))<VER:return False
 if f.get('vercond') and not expr(f.get('vercond')):return False
 return True
class Reader:
 def __init__(self,b):self.b=b;self.p=0;self.used=set();self.objects=[]
 def prim(self,fmt):
  size=struct.calcsize('<'+fmt);v=struct.unpack_from('<'+fmt,self.b,self.p);self.p+=size;return v[0] if len(v)==1 else list(v)
 def string(self):
  n=self.prim('I');assert 0<=n<100000,(self.p,n);v=self.b[self.p:self.p+n].decode('cp1252');self.p+=n;return v
 def read(self,t,arg=0,template='',actual='',d=None):
  self.used.add(t)
  basic={'uint':'I','int':'i','ushort':'H','short':'h','byte':'B','sbyte':'b','float':'f','double':'d','bool':'B','ulittle32':'I','FileVersion':'I','BlockTypeIndex':'H','Ref':'i','Ptr':'i','StringOffset':'I','NiFixedString':'I','char':'b','normbyte':'b'}
  if t in basic:return self.prim(basic[t])
  if t in ('SizedString','HeaderString','LineString'):
   if t=='SizedString':return self.string()
   end=self.b.index(b'\n',self.p);s=self.b[self.p:end].decode('ascii');self.p=end+1;return s
  if t in ('string','FilePath'):return self.string()
  if t=='#T#':return self.read(template,arg,'',actual)
  el=TYPES.get(t)
  if el is None:raise ValueError(('unknown type',t,self.p))
  if el.tag in ('enum','bitflags','bitfield'):return self.read(el.get('storage'),arg,template,actual)
  if el.tag=='basic':raise ValueError(('unhandled basic',t,el.attrib,self.p))
  actual=actual or t;d={} if d is None else d
  if el.get('inherit'):self.read(el.get('inherit'),arg,template,actual,d)
  for f in el.findall('field'):
   if not applicable(f):continue
   if f.get('onlyT') and not self.is_a(actual,f.get('onlyT')):continue
   if f.get('excludeT') and self.is_a(actual,f.get('excludeT')):continue
   if f.get('cond') and not expr(f.get('cond'),d,arg):continue
   ft=f.get('type');name=f.get('name');fa=expr(f.get('arg'),d,arg) if f.get('arg') else 0;temp=f.get('template',template);temp=template if temp=='#T#' else temp
   if f.get('length'):
    length=int(expr(f.get('length'),d,arg));assert 0<=length<100000,(t,name,length,self.p)
    vals=[]
    for i in range(length):
     if f.get('width'):
      width=d[f.get('width')][i] if isinstance(d.get(f.get('width')),list) else int(expr(f.get('width'),d,arg))
      vals.append([self.read(ft,fa,temp) for j in range(width)])
     else:vals.append(self.read(ft,fa,temp))
    d[name]=vals
   else:d[name]=self.read(ft,fa,temp)
  return d
 def is_a(self,t,target):
  while t:
   if t==target:return True
   t=TYPES[t].get('inherit') if t in TYPES else None
  return False
 def parse(self):
  global VER
  newver=struct.unpack_from('<I',self.b,self.b.index(b'\n')+1)[0]
  if newver!=VER:VER=newver;applicable.cache_clear()
  assert VER in (0x0a010000,0x0a000100),hex(VER)
  h=self.read('Header');assert h['Version']==VER,h
  for i,ti in enumerate(h['Block Type Index']):
   t=h['Block Types'][ti];p=self.p;idx=self.prim('I');assert idx==0,('record prefix',i,idx,self.p)
   try:o=self.read(t)
   except Exception as e:raise ValueError(('block',i,t,'start',p,'errorat',self.p)) from e
   o['_type']=t;o['_start']=p;o['_id']=i;self.objects.append(o)
  self.footer=self.read('Footer');assert self.p==len(self.b),(self.p,len(self.b))
  return self
