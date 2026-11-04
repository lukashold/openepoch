import unittest
from import_ee2 import without_comments, block, assignments

class DdfTests(unittest.TestCase):
    def test_commented_nuclear_attack_is_not_enabled(self):
        text = 'UnitType Sub { // Attack {damage=9999}\n abilities=[Attack {damage=170}] }'
        self.assertEqual(assignments(block(without_comments(text),'UnitType','Sub')), {'damage':170})

    def test_string_with_slashes_is_preserved(self):
        self.assertEqual(without_comments('name="http://example" // comment'), 'name="http://example"  ')

    def test_balanced_nested_blocks(self):
        self.assertEqual(block('UnitType A { p {x=1} } UnitType AB {x=2}', 'UnitType','A'), ' p {x=1} ')

    def test_unclosed_block_is_error(self):
        with self.assertRaises(ValueError): block('UnitType A { p {x=1}', 'UnitType','A')

if __name__ == '__main__': unittest.main()
