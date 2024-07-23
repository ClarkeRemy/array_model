// #![feature(unboxed_closures)]
// #![feature(fn_traits)]
#![allow(unused)]

// MySlice<T> {
//     data: Arc<[T]>,
//     start: usize,
//     end: usize,
// }

type Rank = isize;
type ArrayRank = usize;
type Axis = usize;
type ShapeSlice = [Axis];
type Shape = Vec<Axis>;
type Data<T> = Vec<T>;

use std::{sync::Arc, io::Read, fmt::format};

enum ArrayError{RankError(String),UnknownIdentityFunction(String)}



#[derive(Clone)]
struct Mono<T,Z>  {r:Rank, f:Arc<dyn for<'a> MyFn<'a, ArrView<'a, T>, Z>> } // dyn for<'a> MyFn<'a, ArrView<'a, T>, Z>
// struct Mono<T,Z: 'static>  {r:Rank, f:Arc<dyn Fn(ArrView<T>)->Z>} // dyn for<'a> MyFn<'a, ArrView<'a, T>, Z>
trait MyFn<'a, I:'a, O: 'a>: Fn(I) -> O {}
impl<'a, I: 'a, O: 'a, F: Fn(I) -> O> MyFn<'a, I, O> for F {}


// use std::sync::Arc;
// struct Wrapper<T, Z>
//     where for<'a> T: 'a,
//           for<'b> Z: 'b,
// {
//     f: Arc<dyn Fn(T) -> Z>
// }

#[derive(Clone)]
struct Duo<T,G,E: 'static>{r:[Rank;2], f:Arc<dyn Fn(ArrView<T>,ArrView<G>)->E>}
// struct Ambi<A,B,C,D,E>{
//  mr: Rank,
//  dr:[Rank;2], 
//  mf:Arc<dyn Fn(ArrayView<A>)->Array<B>>, 
//  df:Arc<dyn Fn(ArrayView<C>,ArrayView<D>)->Array<E>>
// }


pub struct Array<T>{s:Shape,d:Data<T>}
impl<T> Array<T>{
 fn shape_view(&self) ->&ShapeSlice     {&self.s} 
 fn data_view(&self)  ->&[T]       {&self.d}
 fn view(& self)      ->ArrView<T> {ArrView{s:&self.s, d:&self.d}}
}


// fn array_rank(a:&impl ReadArray)->Array_Rank{a.shape_view().len()}
fn frame<'a,T>(ArrView { s, d }: ArrView<'a,T>, r: Rank)->FrArrView<'a,T>{
 let len = s.len() as Rank;
 let f_pos = match r {
 _ if r>=len =>0,
 _ if r<=-len => len,
 _ =>(len-r)%len} as ArrayRank;
 FrArrView { f: &s[..f_pos], c: &s[f_pos..], d }
}





/// {shape,data}
#[derive(Clone,Copy)]
struct ArrView<'a,T>{s:&'a ShapeSlice, d:&'a [T]}

/// {frame,cell_shape,data}
struct FrArrView<'a,T>{f:&'a ShapeSlice,c:&'a ShapeSlice,d:&'a[T]}

/// {frame,data}
/// The output still needs to be flattened after being returned
/// Expected use :: Array -> ReadArray -> FramedReadArray -> FramedOutput -> Array
pub struct FramedOutput<'a,T>{fr: &'a ShapeSlice,d: Data<T>}
// pub struct FramedOutput<'a,T>{fr: &'a Shape,d: Vec<T>}
impl<'a,T> FramedOutput<'a,T> {
 fn map<G>(self,f: fn(T)->G)->Array<G> {
  Array { s: Shape::from(self.fr), d: self.d.into_iter().map(f).collect::<Vec<G>>() }
 }
//  fn fill(self)->Array<T>{}
}
fn map_to_arr<'a,T,G>(fr_o: FramedOutput<T>,f: fn(T)->G)->Array<G> {
  Array { s: fr_o.fr.to_owned(), d: fr_o.d.into_iter().map(f).collect::<Data<G>>() }
}

fn raw_verb_mono<'a,T,G>(arr_view:ArrView<'a,T>, Mono { r, f:func }:&Mono<T,G>)->(&'a ShapeSlice, Vec<G>){
 let FrArrView { f, c, d } = frame(arr_view, *r);

 let [frs,cl] = [f,c].map(|x|x.iter().product()); // [frameS,cell_len]
 let mut end_cells = Vec::with_capacity(frs);
 for fr in 0..frs {let f_idx =fr*cl; end_cells.push(func( ArrView{s:c,d: &d[f_idx..f_idx*cl]} ) )}
 (f, end_cells )
}

fn verb_mono_apply<'a,T,G>(arr_view:ArrView<'a,T>, m:&Mono<T,G>)->FramedOutput<'a,G>{
 let (fr,d) = raw_verb_mono(arr_view, m);
 FramedOutput { fr, d }
}
// fn raw_mono_rank<'a,'b:'a,T,G>(m: &'b Mono<T,G>,r:Rank)->Mono<T,FramedOutput<'a,G> >
//  where G: 'static + Clone, T: 'static + Clone 
// {
//  let m= (*m).clone();
//  let f = Arc::new(move |y : ArrView<'a,T>|->FramedOutput<'a,G>{ verb_mono_apply(y, &m)});
//  Mono{r,f}
// }

fn raw_verb_prefix_agree<'a,T,G,E>(
  arr_view_l:ArrView<'a,T>, arr_view_r:ArrView<'a,G>, Duo { r, f }: &Duo<T,G,E>
 )-> Result<(Shape,Data<E>),ArrayError> {

 let fav_l = frame(arr_view_l, r[0]); // fav => FramedArrayView
 let fav_r =frame(arr_view_r, r[1]);
 let [prefix,end_frame] = 
  if fav_l.f.len() < fav_r.f.len() {[fav_l.f,fav_r.f]} 
  else {[fav_r.f,fav_l.f]};
 let prefix_len = prefix.len();
 for idx in 0..prefix_len { if prefix[idx] != end_frame[idx] {return Err(ArrayError::RankError(
  format!{"Prefix agreement failed after framing arguments\nFrames:\n\tleft : {:?}\n\tright : {:?}", 
    fav_l.f, fav_r.f}))
  }
 }
 
 // one of theses will be &[], it is the prefix
 let [mid_l, mid_r]  = [&fav_l.f[prefix_len..], &fav_r.f[prefix_len..] ];
 let end_cells_count = end_frame.iter().product();
 let mut end_cells   = Data::with_capacity(end_cells_count);

 let [mp_l , mp_r ]  = [mid_l    , mid_r    ].map(|x|x.iter().product());
 let [cl_l , cl_r ]  = [fav_l.c  , fav_r.c  ].map(|x|x.iter().product::<usize>()); // cl => Cell Len
 let [pcl_l, pcl_r]  = [mp_l*cl_l, mp_r*cl_r];

 let proc = |l,r,out : &mut Data<_>| out.push(f(
  ArrView{s:fav_l.c, d: &fav_l.d[l..l+cl_l]}, 
  ArrView{s:fav_r.c, d: &fav_r.d[r..r+cl_r]} ));

 for pre in 0..prefix.iter().product() { let [pi_l, pi_r] = [pre*pcl_l, pre*pcl_r];
  if mp_l == 1 {for mi_r in 0..mp_r { let d_r = pi_r + mi_r*cl_r; proc(pi_l, d_r , &mut end_cells )}}
  else         {for mi_l in 0..mp_l { let d_l = pi_l + mi_l*cl_l; proc(d_l , pi_r, &mut end_cells )}}
 }
 Ok((Shape::from(end_frame), end_cells))
}


// fn verb_prefix_agree<'a,T,G,E>(
//   arr_view_l:ArrView<'a,T>, arr_view_r:ArrView<'a,G>, duo: &Duo<T,G,E>
//  )-> Result<FramedOutput<E>,ArrayError> {

//  match raw_verb_prefix_agree(arr_view_l, arr_view_r, duo) {
//   Ok((fr,d))=> Ok(FramedOutput{ fr,d }),
//   Err(e)    => Err(e)
//  }
// }




// fn raw_duo_rank<T,G,E>(duo: &Duo<T,G,E>, r:[Rank;2])->Duo<T,G,Result<FramedOutput<E>,ArrayError> >
//  where T: 'static + Clone, G: 'static + Clone, E: 'static + Clone,
// {
//  let duo2 = (*duo).clone();
//  let f = Arc::new(move |x: ArrView<T>, y: ArrView<G>|{
//   verb_prefix_agree(x,y, &duo2) 
//  });
//  Duo{r,f}
// }





// fn u_raw_insert_right_to_left<T,G>(duo: &Duo<T,T,G>)->Mono<T,Result<G,ArrayError>>
//  where G: From<T>
// {
//  let f = Arc::new(|y: ArrView<T>|{
//   mut count if y.s.len()==0 { return FramedOutput{fr: vec![], d: vec![G::from(y.d[0])] } } 
//             else { y.s[0]}
//   let item_shape = &y.s[1..];
//   let item_len   = item_shape.iter().product();
//   if count == 0 { return  ArrayError::UnknownIdentityFunction(format!("Undefined right identity for Duo function.")) }
//   let mu acc = 
//  });
//  Mono { r: Rank::MAX, f }
// }

trait ScalarFill {
 fn fill_value()->Self;
}
impl ScalarFill for char{fn  fill_value()->Self {' '}}




#[cfg(test)]
mod tests;






