extern crate bytes;

use zip_stream::{ZipPacker, ZipEntry};
use std::fs::File;
use std::io::{Read, BufReader, Write, Seek};
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


// struct WW<T> where R: Read {
//     val: T,
// }
//
// impl<T> WW<T> where R: Read {
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


struct Test<W> {
    w: W
}
impl<W: Write> Test<W> {
    fn test(&self) {
        println!("Write only");
    }
}

impl<W: Write + Seek> Test<W> {
    fn test(&self) {
        println!("Write and seek");
    }
}

fn main() {

    let t = Test {
        w: File::open("examples/kappa.rs").unwrap()
    };
    t.test();

    return;

    let mut zip = ZipPacker::new();

    // zip.add_file(ZipEntry::new("Cargo.toml", File::open("Cargo.toml").unwrap()));
    // zip.add_file(ZipEntry::new("Cargo.lock", File::open("Cargo.lock").unwrap()));
    // zip.add_file(ZipEntry::new("examples/kappa.rs", File::open("examples/kappa.rs").unwrap()));
    zip.add_file("zip-stream.zip", File::open("zip-stream.zip").unwrap());

    let mut zip = zip.reader();

    let mut out = File::create("out.zip").unwrap();


    let start = std::time::Instant::now();

    let mut buff = [0u8; 256*1024];
    while let Ok(n) = zip.read(&mut buff) {
        if n > 0 {
            out.write_all(&buff[..n]);
        } else {
            break;
        }
    }

    // let res = std::io::copy(&mut zip, &mut out);
    let end = std::time::Instant::now();
    // println!("{:?}", res);
    println!("{}", (end - start).as_secs_f64());
}