"""Read a source-resolution manifest with a small ZipFile-compatible interface.

Virtual paths use native case-insensitive full-path lookup; basenames use the
separate first-record namespace. Every returned byte string is hash verified.
"""
from pathlib import Path, PurePosixPath
import hashlib,json

class ResolvedArchiveReader:
    def __init__(self, manifest):
        self.manifest_path=Path(manifest).resolve()
        self.manifest=json.loads(self.manifest_path.read_text(encoding='utf8'))
        if self.manifest.get('format')!='ee2-up-static-source-resolution-v1':
            raise ValueError('Unsupported source manifest')
        self.files=self.manifest['effectiveByPath']
        self.basenames=self.manifest['effectiveByBasename']
        self.closed=False
        self._cache={}

    def _record(self,name):
        if self.closed:raise ValueError('Resolved archive is closed')
        if not isinstance(name,str):name=name.filename
        name=name.replace('\\','/')
        if name.startswith('/') or '..' in PurePosixPath(name).parts:raise KeyError(name)
        key=name.casefold()
        if '/' not in key:
            choice=self.basenames.get(key)
            if not choice or choice['status']!='resolved-first-basename-record':raise KeyError('Missing or ambiguous native basename: '+name)
            key=choice['path']
        try:return key,self.files[key]
        except KeyError:raise KeyError('Missing native virtual path: '+name) from None

    def read(self,name):
        key,record=self._record(name)
        if key not in self._cache:
            path=Path(record['extracted']).resolve()
            if not path.is_relative_to(self.manifest_path.parent/'files'):
                raise ValueError('Source extraction path escapes manifest folder')
            data=path.read_bytes()
            if len(data)!=record['source']['bytes'] or hashlib.sha256(data).hexdigest()!=record['source']['sha256']:
                raise ValueError('Source bytes or SHA differ: '+record['path'])
            self._cache[key]=data
        return self._cache[key]

    def namelist(self):
        if self.closed:raise ValueError('Resolved archive is closed')
        return [record['path'] for record in self.files.values()]

    def origin(self,name):
        key,record=self._record(name)
        return {'virtualPath':record['path'],'lookupKey':key,**record['source'],
                'initialRecordTime':record['initialRecordTime'],'initialRecordSize':record['initialRecordSize'],
                'insertionPriority':record.get('insertionPriority'),'profile':self.manifest['profile']}

    def origin_ref(self,name):
        source=self.origin(name)
        return source['physical']+('!'+source['member'] if 'member' in source else '')+'#sha256='+source['sha256']

    def close(self):
        self.closed=True;self._cache.clear()

    def __enter__(self):return self
    def __exit__(self,*_):self.close()
