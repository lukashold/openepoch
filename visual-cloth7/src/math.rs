//! Conditional PC53/RC_CHOP arithmetic, never original engine execution.
//! Fast residual transforms use conservative normal operands2^-400..2^400.
//! Outside it, finite IEEE operands use exact u128 significand arithmetic.
const MIN:f64=f64::from_bits(623u64<<52);
const MAX:f64=f64::from_bits(1423u64<<52);
#[inline]fn safe(v:f64)->bool{v==0.0||(v.abs()>=MIN&&v.abs()<=MAX)}
#[inline]fn next(v:f64)->f64{if v==0.0{v}else{f64::from_bits(v.to_bits()-1)}}
#[inline]fn correct(v:f64,e:f64)->f64{if(v>0.0&&e<0.0)||(v<0.0&&e>0.0){next(v)}else{v}}
#[inline]fn err(a:f64,b:f64,p:f64)->f64{
 let sa=134217729.0*a;let sb=134217729.0*b;
 let ah=sa-(sa-a);let bh=sb-(sb-b);let al=a-ah;let bl=b-bh;
 ((ah*bh-p)+ah*bl+al*bh)+al*bl
}
#[inline]pub fn f(v:f64)->f64{let r=v as f32;if(r as f64).abs()>v.abs(){f32::from_bits(r.to_bits()-1)as f64}else{r as f64}}
#[inline]pub fn add(a:f64,b:f64)->f64{
 let s=a+b;
 if !s.is_finite(){return exact_add(a,b);}
 if a==0.0||b==0.0||s==0.0{return s;}
 let e=if a.abs()>=b.abs(){b-(s-a)}else{a-(s-b)};
 correct(s,e)
}
#[inline]pub fn sub(a:f64,b:f64)->f64{add(a,-b)}
#[inline]pub fn mul(a:f64,b:f64)->f64{
 let p=a*b;
 if !a.is_finite()||!b.is_finite(){panic!("finite arithmetic inputs required");}
 if a==0.0||b==0.0||((a as f32)as f64==a&&(b as f32)as f64==b){return p;}
 if !safe(a)||!safe(b){return exact_mul(a,b);}
 correct(p,err(a,b,p))
}
#[inline]pub fn div(a:f64,b:f64)->f64{
 if !safe(a)||!safe(b)||b==0.0{return exact_div(a,b);}
 let q=a/b;let p=q*b;let residual=(a-p)-err(q,b,p);
 correct(q,if b<0.0{-residual}else{residual})
}
#[inline]pub fn sqrt(a:f64)->f64{
 if !safe(a)||a<=0.0{return exact_sqrt(a);}
 // WASM f64.sqrt is a correctly rounded IEEE square-root operation.
 let r=a.sqrt();let p=r*r;let residual=(a-p)-err(r,r,p);
 if residual<0.0{next(r)}else{r}
}
fn parts(v:f64)->(bool,u128,i32){
 assert!(v.is_finite(),"finite arithmetic inputs required");let bits=v.to_bits();let exponent=((bits>>52)&2047)as i32;let mantissa=(bits&0xfffffffffffff)as u128;
 (bits>>63!=0,if exponent==0{mantissa}else{mantissa|(1u128<<52)},if exponent==0{-1074}else{exponent-1075})
}
fn bitlen(n:u128)->i32{128-n.leading_zeros()as i32}
fn rational(negative:bool,n:u128,d:u128,e:i32)->f64{
 assert!(d!=0);let sign=(negative as u64)<<63;if n==0{return f64::from_bits(sign);}
 let mut k=bitlen(n)-bitlen(d);
 if if k>=0{n<d<<k}else{n<<(-k)<d}{k-=1;}
 let exponent=k+e;
 if exponent>1023{return f64::from_bits(sign|0x7fefffffffffffff);}
 if exponent< -1074{return f64::from_bits(sign);}
 let quantum=if exponent>= -1022{exponent-52}else{-1074};let shift=e-quantum;
 let mantissa=if shift>=0{(n<<shift)/d}else{n/(d<<(-shift))};
 f64::from_bits(sign|if exponent>= -1022{((exponent+1023)as u64)<<52|(mantissa as u64-(1u64<<52))}else{mantissa as u64})
}
pub fn exact_add(a:f64,b:f64)->f64{
 let(sa,ma,ea)=parts(a);let(sb,mb,eb)=parts(b);
 if ma==0||mb==0{return a+b;}
 let(large,ls,lm,le,ss,sm,se)=if ea>=eb{(a,sa,ma,ea,sb,mb,eb)}else{(b,sb,mb,eb,sa,ma,ea)};
 let gap=le-se;
 if gap>75{return if ls==ss{large}else{next(large)};}
 let high=lm<<gap;
 if ls==ss{rational(ls,high+sm,1,se)}else if high>=sm{rational(ls,high-sm,1,se)}else{rational(ss,sm-high,1,se)}
}
pub fn exact_mul(a:f64,b:f64)->f64{let(sa,ma,ea)=parts(a);let(sb,mb,eb)=parts(b);rational(sa!=sb,ma*mb,1,ea+eb)}
pub fn exact_div(a:f64,b:f64)->f64{let(sa,ma,ea)=parts(a);let(sb,mb,eb)=parts(b);assert!(mb!=0,"zero divisor");rational(sa!=sb,ma,mb,ea-eb)}
fn isqrt(n:u128)->u128{
 if n==0{return 0;}let mut result=0u128;let mut remainder=n;let mut bit=1u128<<((bitlen(n)-1)&!1);
 while bit!=0{if remainder>=result+bit{remainder-=result+bit;result=(result>>1)+bit;}else{result>>=1;}bit>>=2;}result
}
pub fn exact_sqrt(a:f64)->f64{
 assert!(a>=0.0,"negative sqrt");if a==0.0{return a;}let(_,m,e)=parts(a);let exponent=(bitlen(m)-1+e).div_euclid(2);let quantum=exponent-52;let shift=e-quantum*2;
 let mantissa=isqrt(if shift>=0{m<<shift}else{m>>(-shift)});
 f64::from_bits(((exponent+1023)as u64)<<52|(mantissa as u64-(1u64<<52)))
}

/// Internal source-cloth specialization. Its operands are bounded by the
/// validated F32 state, finite forces, fixed positive masses and store points;
/// see DOMAIN.md. General exported arithmetic above retains exact fallbacks.
pub mod bounded{
 use super::{correct,err,next};
 pub use super::f;
 #[inline]pub fn add(a:f64,b:f64)->f64{
  let sum=a+b;if a==0.0||b==0.0||sum==0.0{return sum;}
  correct(sum,if a.abs()>=b.abs(){b-(sum-a)}else{a-(sum-b)})
 }
 #[inline]pub fn sub(a:f64,b:f64)->f64{add(a,-b)}
 #[inline]pub fn mul(a:f64,b:f64)->f64{
  debug_assert!(super::safe(a)&&super::safe(b),"internal multiplication domain");
  let product=a*b;if a==0.0||b==0.0||((a as f32)as f64==a&&(b as f32)as f64==b){product}else{correct(product,err(a,b,product))}
 }
 #[inline]pub fn div(a:f64,b:f64)->f64{debug_assert!(super::safe(a)&&super::safe(b)&&b!=0.0,"internal division domain");let quotient=a/b;let product=quotient*b;let residual=(a-product)-err(quotient,b,product);correct(quotient,if b<0.0{-residual}else{residual})}
 #[inline]pub fn sqrt(a:f64)->f64{debug_assert!(super::safe(a)&&a>=0.0,"internal square-root domain");let root=a.sqrt();let product=root*root;let residual=(a-product)-err(root,root,product);if residual<0.0{next(root)}else{root}}
}
