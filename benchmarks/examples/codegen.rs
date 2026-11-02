//! A code generation probe. One exported function per subject, family,
//! variant count and method, each calling the subject's method once from
//! this crate, so that `scripts/codegen.py` can compile the example to
//! assembly and compare every subject's function against the hand-written
//! one, cross-crate inlining included. Running the example prints the memory
//! footprint of the types instead.

use std::mem::size_of;

use dispatch_subjects::payloads::V0;

macro_rules! probe_fns {
    ($subject:ident, $family:ident, $vmod:ident, $v:literal, $Any:ident) => {
        pub mod $vmod {
            use ::dispatch_subjects::Dispatch;

            type Subject = ::dispatch_subjects::$subject::$family::$Any;

            #[export_name = concat!(
                stringify!($subject), "__", stringify!($family), "__", stringify!($v), "__area"
            )]
            pub extern "Rust" fn area(item: &Subject) -> u64 {
                item.area()
            }

            #[export_name = concat!(
                stringify!($subject), "__", stringify!($family), "__", stringify!($v), "__scale"
            )]
            pub extern "Rust" fn scale(item: &mut Subject, factor: u64) {
                item.scale(factor)
            }
        }
    };
}

macro_rules! probe_sizes {
    ($subject:ident, $family:ident) => {
        pub mod $family {
            probe_fns!($subject, $family, v2, 2, Any2);
            probe_fns!($subject, $family, v8, 8, Any8);
            probe_fns!($subject, $family, v32, 32, Any32);
        }
    };
}

macro_rules! probe_subject {
    ($subject:ident) => {
        pub mod $subject {
            probe_sizes!($subject, trivial);
            probe_sizes!($subject, opaque);
        }
    };
}

probe_subject!(hand);
probe_subject!(whichever);
probe_subject!(enum_dispatch);
probe_subject!(ambassador);
probe_subject!(enum_delegate);
probe_subject!(delegation);
probe_subject!(delegate);
probe_subject!(dynamic);
probe_subject!(mono);

fn main() {
    println!("payload (two u64):        {:>3} bytes", size_of::<V0>());
    println!(
        "enum of 2 variants:       {:>3} bytes",
        size_of::<dispatch_subjects::hand::trivial::Any2>()
    );
    println!(
        "enum of 32 variants:      {:>3} bytes",
        size_of::<dispatch_subjects::whichever::trivial::Any32>()
    );
    println!(
        "Box<dyn Trait>:           {:>3} bytes on the stack + {:>2} bytes on the heap",
        size_of::<dispatch_subjects::dynamic::trivial::Any8>(),
        size_of::<V0>()
    );
}
