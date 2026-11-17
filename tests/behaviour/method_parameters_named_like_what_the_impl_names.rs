//! A trait method's own generic parameter may be named like something the
//! impl gives a meaning around the method's signature: a parameter of the
//! enum or the struct, which in the impl would be two parameters of one
//! name, rustc's E0403 (E0496 for a lifetime); or a type the impl writes
//! into the method, a payload's in an arm or an argument given to the
//! trait, which the parameter would then stand for. A hand renames the
//! method's, and so does the attribute: `fn eq<I>` over `struct Tagged<I>`
//! is written `fn eq<I2>`, with the first number that neither the impl nor
//! the trait writes, and the arms call the payload's method as before.
use std::ops::Range;

/// A struct named like the iterator it holds, which every `I` of
/// `foreign::Iterator`'s methods meets.
#[whichever::dispatch(whichever::foreign::Iterator)]
pub struct Tagged<I> {
    pub inner: I,
}

fn tagged() -> Tagged<Range<u32>> {
    Tagged { inner: 1..4 }
}

#[test]
fn a_structs_parameter_leaves_the_methods_their_own() {
    assert!(tagged().eq([1, 2, 3]));
    assert!(tagged().lt(vec![1, 2, 4]));
    assert_eq!(tagged().sum::<u32>(), 6);
    assert_eq!(tagged().max_by_key(|&number| number % 3), Some(2));
}

/// An enum whose parameters are `fold`'s `B` and its own `A`, through a
/// partial block's header; this failed on 0.8.0 at `fold<B, F>`.
#[whichever::dispatch(from = false)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

#[whichever::dispatch]
impl<A: Iterator, B: Iterator<Item = A::Item>> whichever::foreign::Iterator for Either<A, B> {}

#[test]
fn an_enums_parameters_leave_the_methods_their_own() {
    let left: Either<Range<u32>, std::vec::IntoIter<u32>> = Either::Left(1..4);
    assert_eq!(left.fold(0, |total, number| total * 10 + number), 123);
    let right: Either<Range<u32>, std::vec::IntoIter<u32>> =
        Either::Right(vec![1, 2, 3].into_iter());
    let (even, odd): (Vec<u32>, Vec<u32>) = right.partition(|number| number % 2 == 0);
    assert_eq!((even, odd), (vec![2], vec![1, 3]));
}

/// A lifetime, a type and a const parameter of the method, each named like
/// one of the enum's.
#[whichever::dispatchable]
pub trait Fill {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8;
    fn fill<const N: usize>(&self, with: [u8; N]) -> [u8; N];
    fn make<T: Default>(&self) -> T;
    fn pick<T: Copy, N>(&self, one: T, other: N) -> T;
}

pub struct Plain(pub u8);

impl Fill for Plain {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8 {
        if self.0 > *other {
            &self.0
        } else {
            other
        }
    }
    fn make<T: Default>(&self) -> T {
        T::default()
    }
    fn fill<const N: usize>(&self, mut with: [u8; N]) -> [u8; N] {
        with[0] = self.0;
        with
    }
    fn pick<T: Copy, N>(&self, one: T, _other: N) -> T {
        one
    }
}

pub struct Slice<'s, T, const L: usize>(pub &'s [T; L]);

impl<T, const L: usize> Fill for Slice<'_, T, L> {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8 {
        other
    }
    fn make<U: Default>(&self) -> U {
        U::default()
    }
    fn fill<const N: usize>(&self, mut with: [u8; N]) -> [u8; N] {
        with[0] = L as u8;
        with
    }
    fn pick<T2: Copy, N>(&self, one: T2, _other: N) -> T2 {
        one
    }
}

#[whichever::dispatch(Fill)]
pub enum Holder<'a, T, const N: usize> {
    Plain(Plain),
    Slice(Slice<'a, T, N>),
}

#[test]
fn a_lifetime_a_type_and_a_const_are_renamed_alike() {
    let plain: Holder<'_, char, 2> = Holder::Plain(Plain(9));
    assert_eq!(*plain.get(&3), 9);
    assert_eq!(plain.fill([0u8; 3]), [9, 0, 0]);
    assert_eq!(plain.pick('x', 1.5), 'x');
    assert_eq!(plain.make::<String>(), "");
    let letters = ['a', 'b'];
    let slice: Holder<'_, char, 2> = Holder::Slice(Slice(&letters));
    assert_eq!(*slice.get(&3), 3);
    assert_eq!(slice.fill([0u8; 4]), [2, 0, 0, 0]);
    assert_eq!(slice.pick(7u64, "other"), 7);
}

/// A generic trait dispatched at the enum's own parameter, `Encoder<T>` on
/// `enum Codec<T>`, whose method has a parameter named `T` too: the method's
/// is renamed before the trait's `F` becomes the enum's `T`, so the two stay
/// apart, `fn encode<T2>(&self, format: T, extra: T2)`.
#[whichever::dispatchable]
pub trait Encoder<F> {
    fn encode<T>(&self, format: F, extra: T) -> (F, T);
}

pub struct Json;
pub struct Xml<V>(pub V);

impl<F> Encoder<F> for Json {
    fn encode<T>(&self, format: F, extra: T) -> (F, T) {
        (format, extra)
    }
}

impl<F, V> Encoder<F> for Xml<V> {
    fn encode<T>(&self, format: F, extra: T) -> (F, T) {
        (format, extra)
    }
}

#[whichever::dispatch(Encoder<T>)]
pub enum Codec<T> {
    Json(Json),
    Xml(Xml<T>),
}

#[test]
fn the_rename_runs_before_the_trait_parameter_becomes_the_enums() {
    let json: Codec<u8> = Codec::Json(Json);
    assert_eq!(json.encode(4u8, "extra"), (4, "extra"));
    let xml: Codec<u8> = Codec::Xml(Xml(0));
    assert_eq!(xml.encode(5u8, 'x'), (5, 'x'));
}

/// A type the signature names, `T2`, is not the name a method's `T` is
/// given beside the enum's `T`: it is `T3`.
mod a_type_the_signature_names {
    pub struct T2(pub u8);

    #[whichever::dispatchable]
    pub trait Pick {
        fn pick<T: Copy>(&self, one: T, other: T2) -> T;
    }

    pub struct First;

    impl Pick for First {
        fn pick<T: Copy>(&self, one: T, other: T2) -> T {
            assert_eq!(other.0, 7);
            one
        }
    }

    pub struct Second<V>(pub V);

    impl<V> Pick for Second<V> {
        fn pick<T: Copy>(&self, one: T, other: T2) -> T {
            assert_eq!(other.0, 7);
            one
        }
    }

    #[whichever::dispatch(Pick, from = false)]
    pub enum Either<T> {
        First(First),
        Second(Second<T>),
    }

    #[test]
    fn the_signatures_type_keeps_its_meaning() {
        assert_eq!(Either::<u8>::First(First).pick('x', T2(7)), 'x');
        assert_eq!(Either::Second(Second(0u8)).pick(4i64, T2(7)), 4);
    }
}

/// A payload's type, `T2`, is not the name a method's `T` is given either,
/// so the arm's `T2` is still the payload.
mod a_payloads_type {
    #[whichever::dispatchable]
    pub trait Pick {
        fn pick<T: Copy>(&self, one: T) -> T;
    }

    pub struct T2;

    impl Pick for T2 {
        fn pick<T: Copy>(&self, one: T) -> T {
            one
        }
    }

    pub struct Second<V>(pub V);

    impl<V> Pick for Second<V> {
        fn pick<T: Copy>(&self, one: T) -> T {
            one
        }
    }

    #[whichever::dispatch(Pick, from = false)]
    pub enum Either<T> {
        First(T2),
        Second(Second<T>),
    }

    #[test]
    fn the_payloads_type_keeps_its_meaning() {
        assert_eq!(Either::<u8>::First(T2).pick("one"), "one");
        assert_eq!(Either::Second(Second(0u8)).pick(2.5), 2.5);
    }
}

/// A method's parameter named like a payload's type, `Square`, would stand
/// for the payload in the arm; it is renamed `Square2`.
mod a_parameter_named_like_a_payload {
    #[whichever::dispatchable]
    pub trait Pick {
        fn pick<Square: Copy>(&self, one: Square) -> Square;
    }

    pub struct Square(pub u32);
    pub struct Circle(pub u32);

    impl Pick for Square {
        fn pick<S: Copy>(&self, one: S) -> S {
            one
        }
    }

    impl Pick for Circle {
        fn pick<S: Copy>(&self, one: S) -> S {
            one
        }
    }

    #[whichever::dispatch(Pick)]
    pub enum Shape {
        Square(Square),
        Circle(Circle),
    }

    #[test]
    fn the_arm_still_names_the_payload() {
        assert_eq!(Shape::Square(Square(1)).pick('s'), 's');
        assert_eq!(Shape::Circle(Circle(1)).pick(3u8), 3);
    }
}

/// A method's parameter named like the argument given to the trait,
/// `Json` in `Encoder<Json>`, would stand for it where the trait's `F`
/// becomes `Json`; it is renamed `Json2`.
mod a_parameter_named_like_the_traits_argument {
    #[whichever::dispatchable]
    pub trait Encoder<F> {
        fn encode<Json>(&self, format: F, extra: Json) -> (F, Json);
    }

    #[derive(Debug, PartialEq)]
    pub struct Json(pub u8);

    pub struct Compact;
    pub struct Pretty;

    impl<F> Encoder<F> for Compact {
        fn encode<E>(&self, format: F, extra: E) -> (F, E) {
            (format, extra)
        }
    }

    impl<F> Encoder<F> for Pretty {
        fn encode<E>(&self, format: F, extra: E) -> (F, E) {
            (format, extra)
        }
    }

    #[whichever::dispatch(Encoder<Json>)]
    pub enum Codec {
        Compact(Compact),
        Pretty(Pretty),
    }

    #[test]
    fn the_traits_argument_keeps_its_meaning() {
        assert_eq!(
            Codec::Compact(Compact).encode(Json(1), "extra"),
            (Json(1), "extra")
        );
        assert_eq!(Codec::Pretty(Pretty).encode(Json(2), 3u64), (Json(2), 3));
    }
}

/// A trait method's own `mismatch` expression is written against the
/// method's parameters as the trait names them, and follows their renaming:
/// `T::default()` beside the enum's `T` is `T2::default()`, and `[0; N]`
/// beside the enum's `N` is `[0; N2]`.
mod a_mismatch_follows_the_renaming {
    #[whichever::dispatchable]
    pub trait Meet {
        #[dispatch(mismatch = T::default())]
        fn pick<T: Default + From<u8>>(&self, other: &Self) -> T;
        #[dispatch(mismatch = [0; N])]
        fn fill<const N: usize>(&self, other: &Self) -> [u8; N];
    }

    pub struct First;

    impl Meet for First {
        fn pick<T: Default + From<u8>>(&self, _other: &Self) -> T {
            T::from(1)
        }
        fn fill<const N: usize>(&self, _other: &Self) -> [u8; N] {
            [1; N]
        }
    }

    pub struct Second<V, const L: usize>(pub [V; L]);

    impl<V, const L: usize> Meet for Second<V, L> {
        fn pick<T: Default + From<u8>>(&self, _other: &Self) -> T {
            T::from(2)
        }
        fn fill<const N: usize>(&self, _other: &Self) -> [u8; N] {
            [2; N]
        }
    }

    #[whichever::dispatch(Meet, from = false)]
    pub enum Either<T, const N: usize> {
        First(First),
        Second(Second<T, N>),
    }

    #[test]
    fn two_different_variants_answer_with_the_expression() {
        let first: Either<char, 1> = Either::First(First);
        let second: Either<char, 1> = Either::Second(Second(['x']));
        assert_eq!(first.pick::<u32>(&second), 0);
        assert_eq!(first.fill::<3>(&second), [0, 0, 0]);
        assert_eq!(first.pick::<u32>(&first), 1);
        assert_eq!(second.fill::<2>(&second), [2, 2]);
    }
}

/// A name that follows `::` in a payload's type is no name the impl gives
/// a parameter, so a method's `IntoIter` keeps its name over a payload
/// `std::vec::IntoIter<u32>`, as a hand leaves it.
mod a_name_after_a_path_separator {
    #[whichever::dispatchable]
    pub trait Pick {
        fn pick<IntoIter: Copy>(&self, one: IntoIter) -> IntoIter;
    }

    impl Pick for std::vec::IntoIter<u32> {
        fn pick<P: Copy>(&self, one: P) -> P {
            one
        }
    }

    impl Pick for std::ops::Range<u32> {
        fn pick<P: Copy>(&self, one: P) -> P {
            one
        }
    }

    #[whichever::dispatch(Pick)]
    pub enum Numbers {
        Owned(std::vec::IntoIter<u32>),
        Range(std::ops::Range<u32>),
    }

    #[test]
    fn the_parameter_keeps_its_name_and_answers() {
        assert_eq!(Numbers::Owned(vec![1].into_iter()).pick('o'), 'o');
        assert_eq!(Numbers::Range(0..1).pick(5u8), 5);
    }
}

/// A trait-side `mismatch` written inside a macro call follows the
/// renaming too: `vec![T::default()]`, and a const that a format macro's
/// format string captures, `{N}` and `{:N$}`, while `{{N}}` and any string
/// that is no format string, `String::from("{N}")`, stay text. The format
/// string is found by its place among the arguments read as expressions,
/// whatever commas stand inside a closure before it.
mod a_mismatch_inside_a_macro {
    #[whichever::dispatchable]
    pub trait Meet {
        #[dispatch(mismatch = vec![T::default()])]
        fn pick<T: Default>(&self, other: &Self) -> Vec<T>;
        #[dispatch(mismatch = format!("{N}:{:N$}:{}:{{N}}", 7, N))]
        fn show<const N: usize>(&self, other: &Self) -> String;
        #[dispatch(mismatch = vec![String::from("{N}"), format!("{N}"), std::format!("{}{N}", "{N}")])]
        fn text<const N: usize>(&self, other: &Self) -> Vec<String>;
        #[dispatch(mismatch = std::fmt::format(format_args!("{:>N$}|", [1u8, 2].iter().map(|a| a + 1).fold(0, |a, b| a + b))))]
        fn padded<const N: usize>(&self, other: &Self) -> String;
        #[dispatch(mismatch = stringify!(N))]
        fn quoted<const N: usize>(&self, other: &Self) -> &'static str;
    }

    pub struct First;

    impl Meet for First {
        fn pick<T: Default>(&self, _other: &Self) -> Vec<T> {
            vec![]
        }
        fn show<const N: usize>(&self, _other: &Self) -> String {
            String::from("first")
        }
        fn text<const N: usize>(&self, _other: &Self) -> Vec<String> {
            vec![]
        }
        fn padded<const N: usize>(&self, _other: &Self) -> String {
            String::new()
        }
        fn quoted<const N: usize>(&self, _other: &Self) -> &'static str {
            ""
        }
    }

    pub struct Second<V, const L: usize>(pub [V; L]);

    impl<V, const L: usize> Meet for Second<V, L> {
        fn pick<T: Default>(&self, _other: &Self) -> Vec<T> {
            vec![]
        }
        fn show<const N: usize>(&self, _other: &Self) -> String {
            String::from("second")
        }
        fn text<const N: usize>(&self, _other: &Self) -> Vec<String> {
            vec![]
        }
        fn padded<const N: usize>(&self, _other: &Self) -> String {
            String::new()
        }
        fn quoted<const N: usize>(&self, _other: &Self) -> &'static str {
            ""
        }
    }

    #[whichever::dispatch(Meet, from = false)]
    pub enum Either<T, const N: usize> {
        First(First),
        Second(Second<T, N>),
    }

    #[test]
    fn two_different_variants_answer_with_the_macros_expression() {
        let first: Either<char, 1> = Either::First(First);
        let second: Either<char, 1> = Either::Second(Second(['x']));
        assert_eq!(first.pick::<u32>(&second), [0]);
        assert_eq!(first.show::<3>(&second), "3:  7:3:{N}");
        assert_eq!(first.show::<3>(&first), "first");
        assert_eq!(first.text::<4>(&second), ["{N}", "4", "{N}4"]);
        assert_eq!(first.padded::<3>(&second), "  5|");
        assert_eq!(
            first.quoted::<2>(&second),
            "N",
            "`stringify!` takes no code"
        );
    }
}
