//! A separate aligned read-only Pack8 stream. Existing64-byte actor DTO and
//! twelve-float instances remain unchanged for all consumers and old profiles.
use core::ptr::addr_of_mut;
use crate::pack8;
#[repr(C)]#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Record{pub handle:u32,pub flags:u32,pub phase:u32,pub visual:u32,pub visual_start:f32,pub visual_duration:f32,pub history_unknown:u32,pub capable:u32}
pub const STRIDE:u32=32;
const EMPTY:Record=Record{handle:u32::MAX,flags:0,phase:0,visual:0,visual_start:0.0,visual_duration:0.0,history_unknown:0,capable:0};
const _:[();32]=[();core::mem::size_of::<Record>()];
struct Buffers{unsorted:Vec<Record>,sorted:Vec<Record>}
static mut BUFFERS:Buffers=Buffers{unsorted:Vec::new(),sorted:Vec::new()};
fn buffers()->&'static mut Buffers{unsafe{&mut*addr_of_mut!(BUFFERS)}}
pub fn clear(){let b=buffers();b.unsorted.clear();b.sorted.clear();}
pub fn actor(index:usize,s:pack8::State,capable:bool){let b=buffers();if b.unsorted.len()<=index{b.unsorted.resize(index+1,EMPTY);}b.unsorted[index]=Record{handle:s.handle,flags:s.flags as u32,phase:s.phase as u32,visual:s.visual as u32,visual_start:s.visual_start,visual_duration:s.visual_duration,history_unknown:s.history_unknown as u32,capable:capable as u32};}
pub fn prepare(count:usize){let b=buffers();b.unsorted.resize(count,EMPTY);b.sorted.resize(count,EMPTY);}
pub fn sorted_row(source:usize,destination:usize){let b=buffers();b.sorted[destination]=b.unsorted[source];}
pub fn ptr()->*const Record{let b=buffers();if b.sorted.is_empty(){core::ptr::null()}else{b.sorted.as_ptr()}}
pub fn count()->u32{buffers().sorted.len()as u32}
