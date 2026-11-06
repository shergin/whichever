// The same `pub` trait, written out in full, emitted twice by one
// macro; `name` keeps the two hidden macros apart.
macro_rules! declare_runtime_trait {
    ($module:ident) => {
        pub mod $module {
            #[whichever::dispatchable(name = $module)]
            pub trait Runtime {
                fn name(&self) -> &'static str;
            }

            pub struct Native;

            impl Runtime for Native {
                fn name(&self) -> &'static str {
                    stringify!($module)
                }
            }

            #[whichever::dispatch(Runtime)]
            pub enum AnyRuntime {
                Native(Native),
            }
        }
    };
}

declare_runtime_trait!(tokio_like);
declare_runtime_trait!(smol_like);

#[test]
fn identical_traits_from_one_macro_get_distinct_macros() {
    use smol_like::Runtime as _;
    use tokio_like::Runtime as _;
    assert_eq!(
        tokio_like::AnyRuntime::Native(tokio_like::Native).name(),
        "tokio_like"
    );
    assert_eq!(
        smol_like::AnyRuntime::Native(smol_like::Native).name(),
        "smol_like"
    );
}
