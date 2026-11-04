"""Shared identity/mask emission for isolated UP6 generators.

The DefId alias remains u8 in UP5. Numeric source data and the legacy generator
are unchanged; UP6 alone selects wider simulation identities.
"""
import re


def rust_identities(text):
    text = re.sub(r'\bid:u8\b', 'id:DefId', text)
    text = re.sub(r'\b(attacker_id|defender_id):u8\b', r'\1:DefId', text)
    text = text.replace('pub id:u8', 'pub id:DefId')
    if 'DefId' in text and 'use crate::ids::DefId;' not in text:
        text = 'use crate::ids::DefId;\n' + text
    return text


def mask_literal(targets):
    """Emit all words, including IDs above255, without a u64 truncation."""
    ids = sorted(set(targets))
    if any(not isinstance(i, int) or i < 0 or i > 65535 for i in ids):
        raise ValueError('Target DefId outside u16 namespace')
    words = [0] * (1 + (ids[-1] // 64 if ids else 0))
    for identity in ids:
        words[identity // 64] |= 1 << (identity % 64)
    return 'crate::ids::mask(&[' + ','.join(map(str, words)) + '])'


def selftest():
    assert mask_literal([0, 63, 64, 255, 256]) == 'crate::ids::mask(&[9223372036854775809,1,0,9223372036854775808,1])'
    assert mask_literal([]) == 'crate::ids::mask(&[0])'
    assert rust_identities('pub fn f(id:u8,epoch:u8){}').endswith('pub fn f(id:DefId,epoch:u8){}')
    for bad in [[-1], [65536]]:
        try:
            mask_literal(bad)
        except ValueError:
            continue
        raise AssertionError(bad)


if __name__ == '__main__':
    selftest()
    print('roster6 generator identity/mask boundary checks PASS')
