//! Versioned civilization extension around the unchanged PaidQueue7 payload.
//! All normalization is temporary; shared Game::load remains transactional.
use crate::{game::*,defs::*};
pub const MAGIC:u32=0x3855454f; // OEU8
const CHUNK:u32=0x31564943; // CIV1
const END:u32=0x45564943; // CIVE
const META:u32=0x31533652;
const META_END:u32=0x45533652;
const BYTES:usize=36;
fn u(b:&[u8],o:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(o..o+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}

pub fn write(g:&Game,b:&mut Vec<u8>){
    // The just-written internal PaidQueue7 descriptor is replaced by CIV1 and
    // the public8/1 descriptor. Ent176/Player96 and every paid record stay exact.
    b.truncate(b.len()-52);
    for v in[CHUNK,1,4]{put(b,v);}
    for c in g.civilizations{put(b,c as u32);}
    for v in[BYTES as u32,END,META,8,1]{put(b,v);}
    b.extend_from_slice(&crate::civ8_registry::FINGERPRINT);
    for v in[52,META_END]{put(b,v);}
    b[..4].copy_from_slice(&MAGIC.to_le_bytes());
}

pub fn normalize(input:&[u8])->Option<(Vec<u8>,[u8;4])>{
    if u(input,0)?!=MAGIC{
        // Old7 is exclusively the known61-definition release. In particular,
        // dropping the CIV tail from a modern80 game never creates a legacy game.
        if u(input,0)?==crate::paid_queue7::SAVE_MAGIC && u(input,12)?!=61{return None;}
        if u(input,0)?==crate::save6::MAGIC6 && ![57,61].contains(&u(input,12)?){return None;}
        return Some((input.to_vec(),[crate::civ8_runtime::LEGACY_NEUTRAL;4]));
    }
    if u(input,4)?!=176||u(input,8)?!=96||u(input,12)? as usize!=DEFS.len(){return None;}
    let descriptor=input.len().checked_sub(52)?;let d=&input[descriptor..];
    if u(d,0)?!=META||u(d,4)?!=8||u(d,8)?!=1||d.get(12..44)?!=crate::civ8_registry::FINGERPRINT||u(d,44)?!=52||u(d,48)?!=META_END{return None;}
    let start=descriptor.checked_sub(BYTES)?;let c=&input[start..descriptor];
    if u(c,0)?!=CHUNK||u(c,4)?!=1||u(c,8)?!=4||u(c,28)?!=BYTES as u32||u(c,32)?!=END{return None;}
    if u(input,start.checked_sub(4)?)?!=crate::paid_queue7::END{return None;}
    let mut civs=[14_u8;4];
    for (i,civ) in civs.iter_mut().enumerate(){let value=u(c,12+4*i)?;if value>14{return None;}*civ=value as u8;}
    let mut prefix=input[..start].to_vec();
    prefix[..4].copy_from_slice(&crate::paid_queue7::SAVE_MAGIC.to_le_bytes());
    crate::paid_queue7::append_descriptor(&mut prefix);
    Some((prefix,civs))
}

///Read-only extension inspectors retain slices into the original buffer.
///Modern8 validates its outer wrapper before deriving the unchanged paid-prefix
///length. The gameplay loader already normalizes8 and takes the no-copy branch.
pub fn paid_prefix_view(input:&[u8])->Option<&[u8]>{
    if u(input,0)?==MAGIC{
        let(normalized,_)=normalize(input)?;
        let length=crate::paid_queue7::strip_wrapper(&normalized)?.0.len();
        input.get(..length)
    }else{Some(crate::paid_queue7::strip_wrapper(input)?.0)}
}
