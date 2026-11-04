//! Explicit UP6 field codec and checked OEU5 migration. No untrusted bool bytes
//! enter a Rust value before validation. Shared world validation runs afterwards
//! in the ordinary transactional Game::load path.
use crate::{defs::*, game::*};
pub const MAGIC5: u32 = 0x3555454f;
pub const MAGIC6: u32 = 0x3655454f;
pub const ENT5: usize = 148;
pub const ENT6: usize = 176;
pub const PLAYER: usize = 96;
const META: u32 = 0x31533652;
const END: u32 = 0x45533652;
const META_BYTES: usize = 52;
// Frozen 57-role namespace, independent of source file formatting.
pub const REGISTRY: [u8; 32] = crate::roster6_registry::FINGERPRINT;
fn u16v(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn u32v(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}
fn fv(b: &[u8], o: usize) -> Option<f32> {
    let v = f32::from_bits(u32v(b, o)?);
    v.is_finite().then_some(v)
}
fn boolean(b: &[u8], o: usize) -> Option<bool> {
    match *b.get(o)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}
fn put16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn putf(b: &mut [u8], o: usize, v: f32) {
    put32(b, o, v.to_bits());
}
pub fn append_descriptor(b: &mut Vec<u8>) {
    for v in [META, 6, 1] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&REGISTRY);
    b.extend_from_slice(&(META_BYTES as u32).to_le_bytes());
    b.extend_from_slice(&END.to_le_bytes());
}
fn strip_descriptor(b: &[u8]) -> Option<&[u8]> {
    let n = b.len().checked_sub(META_BYTES)?;
    let c = &b[n..];
    if u32v(c, 0)? != META
        || u32v(c, 4)? != 6
        || u32v(c, 8)? != 1
        || !((c.get(12..44)?==REGISTRY && u32v(b,12)? as usize==DEFS.len()) || (u32v(b,12)?==57 && c.get(12..44)?==crate::roster6_registry::PREVIOUS_57))
        || u32v(c, 44)? != META_BYTES as u32
        || u32v(c, 48)? != END
    {
        return None;
    }
    Some(&b[..n])
}
pub fn strip_if_present(b: &[u8]) -> Option<&[u8]> {
    if b.len() >= 4 && u32v(b, b.len() - 4)? == END {
        strip_descriptor(b)
    } else {
        Some(b)
    }
}
pub fn write_players(players: &[Player; 4], b: &mut Vec<u8>) {
    for p in players {
        let start = b.len();
        b.resize(start + PLAYER, 0);
        let r = &mut b[start..];
        for i in 0..3 {
            putf(r, i * 4, p.res[i]);
        }
        put32(r, 12, p.pop);
        put32(r, 16, p.cap);
        put32(r, 20, p.population_limit);
        r[24] = p.epoch;
        r[25] = p.researching as u8;
        r[26] = p.alive as u8;
        r[27] = p.ai as u8;
        put32(r, 28, p.ai_next_attack);
        r[32] = p.ai_nodock as u8;
        r[33] = p.ai_noexp as u8;
        put32(r, 36, p.buildings);
        put32(r, 40, p.tech);
        put32(r, 44, p.tech_busy);
        for i in 0..5 {
            putf(r, 48 + 4 * i, p.atk[i]);
            putf(r, 68 + 4 * i, p.dfn[i]);
        }
        putf(r, 88, p.gather);
        put32(r, 92, p.terr);
    }
}
pub fn write_entities(entities: &[Ent], b: &mut Vec<u8>) {
    for e in entities {
        let start = b.len();
        b.resize(start + ENT6, 0);
        let r = &mut b[start..];
        r[0] = e.alive as u8;
        put16(r, 2, e.def);
        r[4] = e.team;
        put16(r, 6, e.gen);
        macro_rules! floats{($($field:ident:$at:expr),*)=>{$(putf(r,$at,e.$field);)*}}
        floats!(x:8,y:12,px:16,py:20,vx:24,vy:28,rot:32,prot:36,hp:40,cd:44,ox:52,oy:56,built:80,qt:108,rx:112,ry:116,reveal_t:124,carry:132,flash:136);
        r[48] = e.order;
        put32(r, 60, e.otgt);
        put32(r, 64, e.otile);
        put32(r, 68, e.tgt);
        put32(r, 72, e.fkey);
        // Flow slots are transient array indices and are reset on every load.
        // Canonical UP6 saves never embed an incidental cache allocation.
        put16(r, 76, u16::MAX);
        r[78] = e.sel as u8;
        for i in 0..5 {
            put32(r, 84 + i * 4, e.q[i]);
        }
        r[104] = e.qn;
        r[120] = e.revealed;
        r[128] = e.stuck;
        r[140] = e.prev;
        for i in 0..10 {
            put16(r, 142 + i * 2, e.cargo[i]);
            r[162 + i] = e.cargo_hp[i];
        }
        r[172] = e.ncargo;
    }
}
fn read_entity(r: &[u8], legacy: bool) -> Option<Ent> {
    let mut e = E0;
    e.alive = boolean(r, 0)?;
    e.def = if legacy {
        *r.get(1)? as DefId
    } else {
        u16v(r, 2)?
    };
    e.team = *r.get(if legacy { 2 } else { 4 })?;
    e.gen = u16v(r, if legacy { 4 } else { 6 })?;
    macro_rules! floats{($($field:ident:$at:expr),*)=>{$(e.$field=fv(r,$at)?;)*}}
    floats!(x:8,y:12,px:16,py:20,vx:24,vy:28,rot:32,prot:36,hp:40,cd:44,ox:52,oy:56,built:80);
    e.order = *r.get(48)?;
    e.otgt = u32v(r, 60)?;
    e.otile = u32v(r, 64)?;
    e.tgt = u32v(r, 68)?;
    e.fkey = u32v(r, 72)?;
    e.fslot = u16v(r, 76)?;
    e.sel = boolean(r, 78)?;
    for i in 0..5 {
        e.q[i] = if legacy {
            *r.get(84 + i)? as u32
        } else {
            u32v(r, 84 + 4 * i)?
        };
    }
    let (qn, qt, rx, ry, revealed, reveal_t, stuck, carry, flash, prev, cargo, cargo_hp, ncargo) =
        if legacy {
            (89, 92, 96, 100, 104, 108, 112, 116, 120, 124, 125, 135, 145)
        } else {
            (
                104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 142, 162, 172,
            )
        };
    e.qn = *r.get(qn)?;
    e.qt = fv(r, qt)?;
    e.rx = fv(r, rx)?;
    e.ry = fv(r, ry)?;
    e.revealed = *r.get(revealed)?;
    e.reveal_t = fv(r, reveal_t)?;
    e.stuck = *r.get(stuck)?;
    e.carry = fv(r, carry)?;
    e.flash = fv(r, flash)?;
    e.prev = *r.get(prev)?;
    e.ncargo = *r.get(ncargo)?;
    for i in 0..10 {
        e.cargo[i] = if legacy {
            *r.get(cargo + i)? as DefId
        } else {
            u16v(r, cargo + 2 * i)?
        };
        e.cargo_hp[i] = *r.get(cargo_hp + i)?;
    }
    if e.qn > 5 || e.ncargo > 10 {
        return None;
    }
    if legacy && DEFS.get(e.def as usize).is_some_and(|d| d.building) {
        for q in &mut e.q {
            *q = if *q == 255 {
                crate::ids::EPOCH_JOB
            } else if (64..244).contains(q) {
                crate::ids::RESEARCH_BASE + *q - 64
            } else if *q >= 200 {
                crate::ids::TECH_BASE + *q - 200
            } else {
                *q
            };
        }
    }
    if e.alive {
        if let Some(d)=DEFS.get(e.def as usize) {
            if d.building && e.q[..e.qn as usize].iter().any(|&q|crate::ids::decode_job(q).is_none_or(|(kind,_)|kind==2)){return None;}
            if d.has(F_WORKER) && (e.q[0]>9||e.q[1]>1||e.q[2]>9){return None;}
        }
    }
    Some(e)
}
fn validate_player(r: &[u8]) -> Option<()> {
    for i in [25, 26, 27, 32, 33] {
        boolean(r, i)?;
    }
    for at in [0, 4, 8, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88] {
        fv(r, at)?;
    }
    Some(())
}
pub fn normalize_input(input: &[u8]) -> Option<Vec<u8>> {
    if core::mem::size_of::<Ent>() != ENT6 || core::mem::size_of::<Player>() != PLAYER {
        return None;
    }
    let legacy = u32v(input, 0)? == MAGIC5;
    let b = if legacy {
        input
    } else {
        if u32v(input, 0)? != MAGIC6 {
            return None;
        }
        strip_descriptor(input)?
    };
    if !legacy && (b.len() < 4 || u32v(b, b.len() - 4)? != crate::native_attack_clock::END) {
        return None;
    }
    let stride = if legacy { ENT5 } else { ENT6 };
    if u32v(b, 4)? as usize != stride || u32v(b, 8)? as usize != PLAYER || (if legacy {u32v(b,12)?!=57}else{u32v(b,12)? as usize!=DEFS.len()&&u32v(b,12)?!=57}) {
        return None;
    }
    let input_defs=u32v(b,12)? as usize;
    let high = u32v(b, 32)? as usize;
    if high > MAXE {
        return None;
    }
    let entities_start = 36 + PLAYER * 4;
    let rest = entities_start.checked_add(stride.checked_mul(high)?)?;
    b.get(rest..rest + crate::map::NN * 5 + 236)?;
    for i in 0..4 {
        validate_player(&b[36 + i * PLAYER..36 + (i + 1) * PLAYER])?;
    }
    let mut ents = Vec::with_capacity(high);
    for i in 0..high {
        let e=read_entity(&b[entities_start+i*stride..entities_start+(i+1)*stride],legacy)?;
        if e.alive {
            if e.def as usize>=input_defs || e.cargo[..e.ncargo as usize].iter().any(|&id|id as usize>=input_defs){return None;}
            if DEFS[e.def as usize].building && e.q[..e.qn as usize].iter().any(|&q|crate::ids::decode_job(q).is_some_and(|(kind,value)|kind==0&&value as usize>=input_defs)){return None;}
        }
        ents.push(e);
    }
    let mut out = b[..36].to_vec();
    put32(&mut out, 0, MAGIC6);
    put32(&mut out, 4, ENT6 as u32);
    put32(&mut out,12,DEFS.len()as u32);
    out.extend_from_slice(&b[36..entities_start]);
    write_entities(&ents, &mut out);
    out.extend_from_slice(&b[rest..]);
    Some(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_portable_layout() {
        assert_eq!(core::mem::size_of::<Ent>(), ENT6);
        assert_eq!(core::mem::offset_of!(Ent, def), 2);
        assert_eq!(core::mem::offset_of!(Ent, q), 84);
        assert_eq!(core::mem::offset_of!(Ent, qt), 108);
        assert_eq!(core::mem::offset_of!(Ent, cargo), 142);
        assert_eq!(core::mem::offset_of!(Ent, ncargo), 172);
        assert_eq!(core::mem::size_of::<Player>(), PLAYER);
    }
}
