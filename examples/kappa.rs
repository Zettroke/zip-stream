extern crate bytes;

use zip_stream::kappa;
use std::fs::File;
use std::io::{Read, BufReader, Write};
use bytes::Buf;
use std::ops::DerefMut;
use std::marker::PhantomData;


// pub trait AsMutRef<'a, T: ?Sized + 'a> {
//     fn as_mut_ref(&'a mut self) -> &'a mut T;
// }

// impl <'a, T: ?Sized + 'a, R: AsMut<T>> AsMutRef<'a, T> for R {
//     fn as_mut_ref(&'a mut self) -> &'a mut T {
//         self.as_mut()
//     }
// }
//
// impl<'a, T: ?Sized + 'a> AsMutRef<'a, T> for &'a mut T {
//     fn as_mut_ref(&'a mut self) -> &'a mut T {
//         self
//     }
// }
// // impl<'a, T: ?Sized + 'a> AsMutRef<'a, T> for Box<T> {
// //     fn as_mut_ref(&'a mut self) -> &'a mut T {
// //         &mut **self
// //     }
// // }
//
// impl<'a> AsMutRef<'a, dyn Read> for Box<dyn Read> {
//     fn as_mut_ref(&'a mut self) -> &'a mut dyn Read {
//         self.as_mut()
//     }
// }

// impl<'a, T: ?Sized + 'a, R: AsMut<T>> AsMutRef<'a, T> for R {
//     fn as_mut_ref(&'a mut self) -> &'a mut T {
//         self.as_mut() as &'a mut T
//     }
// }
//
// struct WW<'a, T> where T: AsMutRef<'a, dyn Read> + 'a {
//     val: T,
//     phantom: PhantomData<&'a T>
// }


// struct WW<T> where T: AsMut<dyn Read> {
//     val: T,
// }
//
// impl<T> WW<T> where T: AsMut<dyn Read> {
//     fn new(v: T) -> Self{
//         Self {
//             val: v,
//         }
//     }
//
//     pub fn test(&mut self) {
//         let r = self.val.as_mut();
//
//         let mut s = String::new();
//         r.read_to_string(&mut s);
//         println!("{}", s);
//     }
// }

// fn tst<'a>() -> WW<'a, &'a mut dyn Read> {
//     let mut file = BufReader::new(File::open("./examples/test.txt").unwrap());
//     let aa = &mut file as &mut dyn Read;
//     let mut asd = WW::new(aa);
//
//     return asd;
// }

fn main() {
}