//! Source004EAA40/004E2000. No private AI RNG and no implicit global FPU mode.

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Precision53 { NearestEven, TowardZero }

impl Precision53 {
    pub(crate) fn add(self,a:f64,b:f64)->f64 {
        let sum=a+b;
        if self==Self::NearestEven||!sum.is_finite(){return sum;}
        // Error-free TwoSum under this Rust/WASM operation's native nearest53.
        let bv=sum-a;
        let error=(a-(sum-bv))+(b-bv);
        if (sum>0.0&&error<0.0)||(sum<0.0&&error>0.0){f64::from_bits(sum.to_bits()-1)}else{sum}
    }
    pub(crate) fn mul(self,a:f64,b:f64)->f64 {
        if self==Self::NearestEven||a==0.0||b==0.0||!a.is_finite()||!b.is_finite(){return a*b;}
        let parts=|value:f64|{
            let bits=value.to_bits();let exponent=((bits>>52)&2047)as i32;
            let mantissa=(bits&((1_u64<<52)-1))|if exponent!=0{1_u64<<52}else{0};
            (bits>>63,mantissa as u128,if exponent!=0{exponent-1075}else{-1074})
        };
        let(sa,ma,ea)=parts(a);let(sb,mb,eb)=parts(b);
        let full=ma*mb;let shift=(128-full.leading_zeros()).saturating_sub(53);
        let leading=(full>>shift)as u64;let exponent=ea+eb+shift as i32+52;
        // Count inputs are finitef32*f32*signed32. Their nonzero products are
        // normalf64 and never approach f64 exponent overflow/underflow.
        assert!((-1022..=1023).contains(&exponent));
        f64::from_bits(((sa^sb)<<63)|(((exponent+1023)as u64)<<52)|(leading-(1_u64<<52)))
    }
    pub(crate) fn store_f32(self,value:f64)->f32 {
        let rounded=value as f32;
        if self==Self::NearestEven||!value.is_finite()||value==0.0{return rounded;}
        if !rounded.is_finite(){return if value.is_sign_negative(){-f32::MAX}else{f32::MAX};}
        if (rounded as f64).abs()<=value.abs(){rounded}
        else{f32::from_bits(rounded.to_bits()-1)}
    }
}

pub const BIAS:f64=103_079_215_104.0;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Projection { pub count:i32,pub biased_bits:u64 }
pub fn project(count:i32,factor:f32,multiplier:f32,mode:Precision53)->Option<Projection>{
    if !factor.is_finite()||!multiplier.is_finite(){return None;}
    let product=mode.mul(mode.mul(factor as f64,count as f64),multiplier as f64);
    let biased=mode.add(BIAS,product);let bits=biased.to_bits();
    Some(Projection{count:((bits as i64)>>16)as i32,biased_bits:bits})
}

/// Implement for the SAME native_rng9::NativeRng object used by births and
/// personality selection. This module never creates or seeds a random stream.
pub trait Word15 { fn next_word15(&mut self)->u32; }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Cache {
    pub counts:Vec<u32>,
    /// Explicit allocation bound across AIP changes, not current list length
    /// and not a claimed native constant. Native cache may outlive a long list.
    pub max_entries:usize,
}
#[derive(Clone,Copy,Debug)]
pub struct Query {
    pub index:usize,pub min:i32,pub max:i32,
    pub multiplier:f32,pub now:f32,pub refresh_at:f32,pub family_factor:Option<f32>,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Sample { pub projection:Projection,pub cached_count:u32,pub sampled:bool }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Invalid { Bounds,Float,ZeroSpan,Word15 }
pub fn sample<R:Word15>(cache:&mut Cache,q:Query,mode:Precision53,rng:&mut R)->Result<Sample,Invalid>{
    if q.index>=cache.max_entries||cache.counts.len()>cache.max_entries||cache.counts.iter().any(|&n|n>65535){return Err(Invalid::Bounds);}
    let factor=q.family_factor.unwrap_or(1.0);
    if !q.now.is_finite()||!q.refresh_at.is_finite()||!q.multiplier.is_finite()||!factor.is_finite(){return Err(Invalid::Float);}
    let new_slot=q.index>=cache.counts.len();
    if new_slot{cache.counts.resize(q.index+1,1);}
    let sampled=new_slot||q.now>=q.refresh_at;
    if sampled{
        let span=(q.max as u32).wrapping_sub(q.min as u32).wrapping_add(1);
        if span==0{return Err(Invalid::ZeroSpan);}
        let word=rng.next_word15();if word>32767{return Err(Invalid::Word15);}
        cache.counts[q.index]=(word%span).wrapping_add(q.min as u32)&65535;
    }
    let cached_count=cache.counts[q.index];
    Ok(Sample{projection:project(cached_count as i32,factor,q.multiplier,mode).unwrap(),cached_count,sampled})
}

#[cfg(test)]
mod tests{
    use super::*;
    include!("../tools/fixtures/ai9-count-oracle.rs");
    #[test]fn independent_fraction_integer_binary53_bias_vectors(){
        for &(count,factor,multiplier,toward_zero,result,bits)in VECTORS{
            let mode=if toward_zero{Precision53::TowardZero}else{Precision53::NearestEven};
            assert_eq!(project(count,f32::from_bits(factor),f32::from_bits(multiplier),mode),Some(Projection{count:result,biased_bits:bits}),"count{count},factor{factor:08x},multiplier{multiplier:08x},mode{mode:?}");
        }
    }
    struct Shared{state:u32,draws:usize}
    impl Word15 for Shared{fn next_word15(&mut self)->u32{self.state=self.state.wrapping_mul(1664525).wrapping_add(1013904223);self.draws+=1;(self.state>>16)&32767}}
    fn query()->Query{Query{index:3,min:10,max:20,multiplier:1.0,now:0.0,refresh_at:30.0,family_factor:Some(0.5)}}
    #[test]fn cached_rng_draw_schedule_and_shared_birth_interleave(){
        let mut rng=Shared{state:0x12345678,draws:0};let mut cache=Cache{counts:vec![],max_entries:12};
        let a=sample(&mut cache,query(),Precision53::NearestEven,&mut rng).unwrap();
        assert!(a.sampled);assert_eq!(&cache.counts[..3],&[1,1,1]);assert_eq!(rng.draws,1);
        let state=rng.state;let mut q=query();q.min=999;q.max=999;
        let b=sample(&mut cache,q,Precision53::NearestEven,&mut rng).unwrap();
        assert_eq!(b.cached_count,a.cached_count);assert!(!b.sampled);assert_eq!(rng.state,state);
        q=query();q.now=30.0;assert!(sample(&mut cache,q,Precision53::NearestEven,&mut rng).unwrap().sampled);
        assert!(sample(&mut cache,q,Precision53::NearestEven,&mut rng).unwrap().sampled);assert_eq!(rng.draws,3);
        rng.next_word15(); //the ordinary birth resolver uses this same state
        let after_birth=rng.state;q.now=29.0;sample(&mut cache,q,Precision53::NearestEven,&mut rng).unwrap();assert_eq!(rng.state,after_birth);
        q.now=31.0;sample(&mut cache,q,Precision53::NearestEven,&mut rng).unwrap();assert_eq!(rng.draws,5);
    }
    #[test]fn source_cache_survives_smaller_program_and_no_family_factor_is_one(){
        let mut rng=Shared{state:7,draws:0};let mut cache=Cache{counts:vec![7,8,9,10],max_entries:12};
        let q=Query{index:0,family_factor:None,..query()};
        let a=sample(&mut cache,q,Precision53::NearestEven,&mut rng).unwrap();
        assert_eq!(a.projection.count,7);assert_eq!(cache.counts,[7,8,9,10]);assert_eq!(rng.draws,0);
    }
}
