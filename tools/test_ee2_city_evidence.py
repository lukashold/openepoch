"""Bounded independent PE/source checks for Ghidra CityManager conclusions."""
import hashlib, json, pathlib, re, struct, unittest, zipfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
ANALYSIS = ROOT / 'outputs/EE2-Research/analysis'
ENGINE = ROOT / 'work/EE2-engine.exe'
INSTALL = pathlib.Path(r'C:\Program Files (x86)\GOG.com\Empire Earth II')
SHA = lambda b: hashlib.sha256(b).hexdigest()

class PE:
    def __init__(self, data):
        self.data = data
        p = struct.unpack_from('<I', data, 60)[0]
        n = struct.unpack_from('<H', data, p+6)[0]
        self.base = struct.unpack_from('<I', data, p+52)[0]
        s = p+24+struct.unpack_from('<H', data, p+20)[0]
        self.sections = [struct.unpack_from('<IIII', data, s+i*40+8) for i in range(n)]
    def read(self, address, length):
        rva = address-self.base
        for size, va, raw_size, raw in self.sections:
            if va <= rva < va+max(size, raw_size):
                return self.data[raw+rva-va:raw+rva-va+length]
        raise ValueError(hex(address))
    def call(self, at):
        b = self.read(at, 5)
        assert b[0] == 0xe8
        return at+5+struct.unpack_from('<i', b, 1)[0]

class CityEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        data = ENGINE.read_bytes()
        assert SHA(data) == 'd41a9e15ce0638c7dc700dcc529c41c7e345bde460ca184858f966db60e6455f'
        cls.pe = PE(data)
    def test_registration_and_setter_store_raw_bits(self):
        self.assertEqual(self.pe.read(0x815466, 10).hex(), '8b442428895e3c894608')
        self.assertEqual(self.pe.read(0x814fce, 20).hex(), '8b4c24288b573cd80d1039b4005183c244894f08')
        self.assertEqual(self.pe.call(0x8154c3), 0x814fa0)
    def test_update_only_enters_on_strict_growth(self):
        # fld ability+34; fcomp record+8; fnstsw; and0x4100; jnz skip.
        self.assertEqual(self.pe.read(0x814142, 15).hex(), 'd94134d85d08dfe0250041000075bf')
        self.assertEqual(self.pe.call(0x81415b), 0x814fa0)
        # History only appends on strict increase, with radius at+4 and time at0.
        self.assertEqual(self.pe.read(0x814ff9, 17).hex(), 'd9442428d85cc1fcdfe025004100007547')
        self.assertEqual(self.pe.read(0x815042, 10).hex(), '8954c104d905e4c3c900')
    def test_removal_releases_record_and_replacement_retains_band(self):
        self.assertEqual(self.pe.call(0x8157c4), 0x8127f0)
        self.assertEqual(self.pe.call(0x8157ca), 0xa183b0)
        self.assertEqual(self.pe.read(0x815820, 12).hex(), '8b5424105f89503c5e8b523c')
        body = self.pe.read(0x8157e0, 114)
        self.assertNotIn(bytes.fromhex('895008'), body) # no record radius store
        self.assertNotIn(bytes.fromhex('894008'), body)
    def test_query_uses_native_float_constant_and_stored_sum(self):
        self.assertEqual(self.pe.read(0xb43910, 4).hex(), 'f304b53f')
        self.assertEqual(self.pe.read(0x814448, 4).hex(), 'd95c2418')
        self.assertEqual(self.pe.read(0x814454, 12).hex(), 'd907d80d1039b400d85c2418')
    def test_base_profiles_and_upgrade_factor_identity(self):
        with zipfile.ZipFile(INSTALL/'zips/db.zip') as z:
            data = z.read('db/Units/city_center.ddf')
            self.assertEqual(SHA(data), '4f1cfef27687288b40872426c5d2fd72a0771cfa7b14aa68d90f6976aa58a197')
            values = re.findall(r'factor\s*=\s*CityRadius;\s*method\s*=\s*Set;\s*value\s*=\s*([\d.]+)', data.decode('cp1252'))
            self.assertEqual([float(v) for v in values], [7,8,9,10,11])
        with zipfile.ZipFile(INSTALL/'zips/hdrs.zip') as z:
            data = z.read('DbHdrs/DbUpgradeFactorDefs.h')
            self.assertEqual(SHA(data), '3d4b19c3614c84a6fe229fb9bc5b5bc86333c0945ef6786b8ae38387162ef312')
            text = re.sub(r'//[^\r\n]*', '', data.decode('cp1252')).split('ENUM eDbUpgradeUnitTypeFactor',1)[1].split('}',1)[0]
            self.assertEqual(re.findall(r'\bkDbUpgradeUnitTypeFactor_\w+', text).index('kDbUpgradeUnitTypeFactor_CityRadius'), 37)

if __name__ == '__main__':
    unittest.main()
