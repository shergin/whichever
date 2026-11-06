//! The carrier macro is written once per combination of the trait items'
//! distinct `#[cfg]` predicates, so a fifth one is reported.
#[whichever::dispatchable]
pub trait Wide {
    #[cfg(unix)]
    fn a(&self);
    #[cfg(windows)]
    fn b(&self);
    #[cfg(test)]
    fn c(&self);
    #[cfg(debug_assertions)]
    fn d(&self);
    #[cfg(target_pointer_width = "64")]
    fn e(&self);
}

fn main() {}
