//! One module per capability the README claims.
#![allow(dead_code, async_fn_in_trait)]

mod defaults_are_forwarded {
    #[whichever::dispatchable]
    trait Describe {
        fn name(&self) -> &'static str;
        fn kind(&self) -> &'static str {
            "default"
        }
        fn doubled(&self) -> usize
        where
            Self: Sized,
        {
            self.name().len() * 2
        }
    }

    struct Cat;
    struct Dog;

    impl Describe for Cat {
        fn name(&self) -> &'static str {
            "cat"
        }
        fn kind(&self) -> &'static str {
            "feline"
        }
        fn doubled(&self) -> usize {
            100
        }
    }

    impl Describe for Dog {
        fn name(&self) -> &'static str {
            "dog"
        }
    }

    #[whichever::dispatch(Describe)]
    enum Animal {
        Cat(Cat),
        Dog(Dog),
    }

    #[test]
    fn overrides_win_and_defaults_apply() {
        let cat: Animal = Cat.into();
        let dog: Animal = Dog.into();
        assert_eq!(
            (cat.name(), cat.kind(), cat.doubled()),
            ("cat", "feline", 100)
        );
        assert_eq!(
            (dog.name(), dog.kind(), dog.doubled()),
            ("dog", "default", 6)
        );
    }
}

mod self_arguments {
    #[whichever::dispatchable]
    trait Same {
        #[dispatch(mismatch = false)]
        fn same_as(&self, other: &Self) -> bool;
        fn merge(&mut self, other: &mut Self);
        fn absorb(self, other: Self) -> u32
        where
            Self: Sized;
    }

    struct Small(u8);
    struct Big(u32);

    impl Same for Small {
        fn same_as(&self, other: &Self) -> bool {
            self.0 == other.0
        }
        fn merge(&mut self, other: &mut Self) {
            self.0 += other.0;
            other.0 = 0;
        }
        fn absorb(self, other: Self) -> u32 {
            u32::from(self.0 + other.0)
        }
    }

    impl Same for Big {
        fn same_as(&self, other: &Self) -> bool {
            self.0 == other.0
        }
        fn merge(&mut self, other: &mut Self) {
            self.0 += other.0;
            other.0 = 0;
        }
        fn absorb(self, other: Self) -> u32 {
            self.0 + other.0
        }
    }

    #[whichever::dispatch(Same, mismatch(merge = (), absorb = 0))]
    enum Number {
        Small(Small),
        Big(Big),
    }

    #[test]
    fn same_variants_meet_and_different_ones_take_the_fallback() {
        let a: Number = Small(2).into();
        let b: Number = Small(2).into();
        let c: Number = Big(2).into();
        assert!(a.same_as(&b));
        assert!(!a.same_as(&c));

        let mut x: Number = Small(2).into();
        let mut y: Number = Small(3).into();
        x.merge(&mut y);
        assert!(matches!(
            (x, y),
            (Number::Small(Small(5)), Number::Small(Small(0)))
        ));

        let mut big: Number = Big(10).into();
        let mut small: Number = Small(1).into();
        big.merge(&mut small);
        assert!(matches!(
            (big, small),
            (Number::Big(Big(10)), Number::Small(Small(1)))
        ));

        assert_eq!(Number::from(Big(1)).absorb(Big(2).into()), 3);
        assert_eq!(Number::from(Big(1)).absorb(Small(2).into()), 0);
    }
}

mod self_returns {
    #[whichever::dispatchable]
    trait Grow {
        fn grow(&self) -> Self
        where
            Self: Sized;
        fn halve(&self) -> Option<Self>
        where
            Self: Sized;
        fn checked(&self) -> Result<Self, &'static str>
        where
            Self: Sized;
    }

    #[derive(Debug, PartialEq)]
    struct Small(u8);
    #[derive(Debug, PartialEq)]
    struct Big(u32);

    impl Grow for Small {
        fn grow(&self) -> Self {
            Small(self.0 + 1)
        }
        fn halve(&self) -> Option<Self> {
            self.0.is_multiple_of(2).then_some(Small(self.0 / 2))
        }
        fn checked(&self) -> Result<Self, &'static str> {
            Ok(Small(self.0))
        }
    }

    impl Grow for Big {
        fn grow(&self) -> Self {
            Big(self.0 + 1)
        }
        fn halve(&self) -> Option<Self> {
            Some(Big(self.0 / 2))
        }
        fn checked(&self) -> Result<Self, &'static str> {
            Err("too big")
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Grow)]
    enum Number {
        Small(Small),
        Big(Big),
    }

    #[test]
    fn results_are_wrapped_back_into_the_same_variant() {
        let small: Number = Small(4).into();
        let big: Number = Big(4).into();
        assert_eq!(small.grow(), Number::Small(Small(5)));
        assert_eq!(big.grow(), Number::Big(Big(5)));
        assert_eq!(small.halve(), Some(Number::Small(Small(2))));
        assert_eq!(Number::from(Small(3)).halve(), None);
        assert_eq!(small.checked(), Ok(Number::Small(Small(4))));
        assert_eq!(big.checked(), Err("too big"));
    }
}

mod associated_types {
    #[whichever::dispatchable]
    trait Store {
        type Item;
        fn first(&self) -> Self::Item;
        fn all(&self) -> Vec<Self::Item>;
    }

    struct Shelf;
    struct Crate;

    impl Store for Shelf {
        type Item = u32;
        fn first(&self) -> u32 {
            1
        }
        fn all(&self) -> Vec<u32> {
            vec![1, 2]
        }
    }

    impl Store for Crate {
        type Item = u32;
        fn first(&self) -> u32 {
            10
        }
        fn all(&self) -> Vec<u32> {
            vec![10]
        }
    }

    #[whichever::dispatch(Store)]
    enum Storage {
        Shelf(Shelf),
        Crate(Crate),
    }

    #[whichever::dispatchable]
    trait Fallible {
        type Error;
        fn run(&self) -> Result<u32, Self::Error>;
        fn fail(&self) -> Self::Error;
    }

    struct Narrow;
    struct Wide;

    impl Fallible for Narrow {
        type Error = u8;
        fn run(&self) -> Result<u32, u8> {
            Err(8)
        }
        fn fail(&self) -> u8 {
            1
        }
    }

    impl Fallible for Wide {
        type Error = u16;
        fn run(&self) -> Result<u32, u16> {
            Ok(16)
        }
        fn fail(&self) -> u16 {
            2
        }
    }

    #[whichever::dispatch(Fallible, Error = u16)]
    enum Width {
        Narrow(Narrow),
        Wide(Wide),
    }

    #[test]
    fn the_first_variant_fixes_the_type_and_an_override_converts() {
        let item: <Storage as Store>::Item = Storage::from(Crate).first();
        assert_eq!(item, 10);
        assert_eq!(Storage::from(Shelf).all(), vec![1, 2]);

        let narrow: Width = Narrow.into();
        let wide: Width = Wide.into();
        let error: Result<u32, u16> = narrow.run();
        assert_eq!(error, Err(8));
        assert_eq!(wide.run(), Ok(16));
        let failure: u16 = narrow.fail();
        assert_eq!((failure, wide.fail()), (1, 2));
    }
}

mod associated_consts_and_statics {
    #[whichever::dispatchable]
    trait Tagged {
        const TAG: u8;
        fn kinds() -> &'static [&'static str] {
            &["default"]
        }
        fn value(&self) -> u8;
    }

    struct Left;
    struct Right;

    impl Tagged for Left {
        const TAG: u8 = 7;
        fn kinds() -> &'static [&'static str] {
            &["left"]
        }
        fn value(&self) -> u8 {
            1
        }
    }

    impl Tagged for Right {
        const TAG: u8 = 7;
        fn value(&self) -> u8 {
            2
        }
    }

    #[whichever::dispatch(Tagged)]
    enum Side {
        Left(Left),
        Right(Right),
    }

    #[test]
    fn consts_come_from_the_first_variant_and_statics_keep_their_default() {
        assert_eq!(<Side as Tagged>::TAG, 7);
        assert_eq!(<Side as Tagged>::kinds(), &["default"]);
        assert_eq!(Side::from(Right).value(), 2);
    }
}

mod async_and_unsafe_methods {
    #[whichever::dispatchable]
    trait Fetch {
        async fn fetch(&mut self) -> u32;
        unsafe fn raw(&self) -> u8;
    }

    struct Counter(u32);
    struct Constant;

    impl Fetch for Counter {
        async fn fetch(&mut self) -> u32 {
            self.0 += 1;
            self.0
        }
        unsafe fn raw(&self) -> u8 {
            1
        }
    }

    impl Fetch for Constant {
        async fn fetch(&mut self) -> u32 {
            42
        }
        unsafe fn raw(&self) -> u8 {
            2
        }
    }

    #[whichever::dispatch(Fetch)]
    enum Source {
        Counter(Counter),
        Constant(Constant),
    }

    #[test]
    fn arms_await_and_wrap_unsafe_calls() {
        let mut counter: Source = Counter(0).into();
        assert_eq!(pollster::block_on(counter.fetch()), 1);
        assert_eq!(pollster::block_on(counter.fetch()), 2);
        assert_eq!(pollster::block_on(Source::from(Constant).fetch()), 42);
        assert_eq!(unsafe { counter.raw() }, 1);
    }
}

mod generic_traits {
    #[whichever::dispatchable]
    trait Convert<X> {
        fn convert(&self, x: X) -> X;
    }

    struct Inc;
    struct Dec;

    impl Convert<u32> for Inc {
        fn convert(&self, x: u32) -> u32 {
            x + 1
        }
    }

    impl Convert<u32> for Dec {
        fn convert(&self, x: u32) -> u32 {
            x - 1
        }
    }

    #[whichever::dispatch(Convert<u32>)]
    enum Step {
        Inc(Inc),
        Dec(Dec),
    }

    trait Format {
        type Value;
    }

    struct Document<F: Format>(F::Value);

    #[whichever::dispatchable]
    trait Encode<F: Format> {
        fn encode(&self, value: &F::Value) -> String;
        fn encode_document(&self, document: &Document<F>) -> String {
            self.encode(&document.0)
        }
    }

    struct Json;

    impl Format for Json {
        type Value = u32;
    }

    struct Compact;
    struct Pretty;

    impl Encode<Json> for Compact {
        fn encode(&self, value: &u32) -> String {
            format!("{value}")
        }
    }

    impl Encode<Json> for Pretty {
        fn encode(&self, value: &u32) -> String {
            format!("{value} ")
        }
    }

    #[whichever::dispatch(Encode<Json>)]
    enum AnyEncoder {
        Compact(Compact),
        Pretty(Pretty),
    }

    #[whichever::dispatchable]
    trait Buffer<const N: usize> {
        fn fill(&self) -> [u8; N];
    }

    struct Ones;
    struct Twos;

    impl Buffer<4> for Ones {
        fn fill(&self) -> [u8; 4] {
            [1; 4]
        }
    }

    impl Buffer<4> for Twos {
        fn fill(&self) -> [u8; 4] {
            [2; 4]
        }
    }

    #[whichever::dispatch(Buffer<4>)]
    enum Filler {
        Ones(Ones),
        Twos(Twos),
    }

    #[test]
    fn parameters_are_substituted_with_the_arguments_the_enum_names() {
        assert_eq!(Step::from(Inc).convert(1), 2);
        assert_eq!(Step::from(Dec).convert(1), 0);
        assert_eq!(AnyEncoder::from(Pretty).encode(&7), "7 ");
        assert_eq!(AnyEncoder::from(Compact).encode_document(&Document(3)), "3");
        assert_eq!(Filler::from(Twos).fill(), [2, 2, 2, 2]);
    }
}

mod generic_methods {
    #[whichever::dispatchable]
    trait Apply {
        fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32;
        fn feed(&self, items: impl Iterator<Item = u32>) -> u32;
    }

    struct One;
    struct Two;

    impl Apply for One {
        fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
            f(1)
        }
        fn feed(&self, items: impl Iterator<Item = u32>) -> u32 {
            items.sum()
        }
    }

    impl Apply for Two {
        fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
            f(2)
        }
        fn feed(&self, items: impl Iterator<Item = u32>) -> u32 {
            items.count() as u32
        }
    }

    #[whichever::dispatch(Apply)]
    enum Which {
        One(One),
        Two(Two),
    }

    #[test]
    fn closures_and_argument_position_impl_trait_pass_through() {
        assert_eq!(Which::from(Two).apply(|x| x * 10), 20);
        assert_eq!(Which::from(One).feed([1, 2, 3].into_iter()), 6);
        assert_eq!(Which::from(Two).feed([1, 2, 3].into_iter()), 3);
    }
}

mod lifetimes_and_generic_enums {
    #[whichever::dispatchable]
    trait Borrowed<'a> {
        fn get(&self) -> &'a str;
    }

    struct Head<'a>(&'a str);
    struct Tail<'a>(&'a str);

    impl<'a> Borrowed<'a> for Head<'a> {
        fn get(&self) -> &'a str {
            &self.0[..1]
        }
    }

    impl<'a> Borrowed<'a> for Tail<'a> {
        fn get(&self) -> &'a str {
            &self.0[1..]
        }
    }

    #[whichever::dispatch(Borrowed<'a>)]
    enum Part<'a> {
        Head(Head<'a>),
        Tail(Tail<'a>),
    }

    #[whichever::dispatchable]
    trait Len {
        fn len(&self) -> usize;
    }

    impl<T> Len for Vec<T> {
        fn len(&self) -> usize {
            Vec::len(self)
        }
    }

    impl<T> Len for Option<T> {
        fn len(&self) -> usize {
            usize::from(self.is_some())
        }
    }

    #[whichever::dispatch(Len)]
    enum Container<T>
    where
        T: Clone,
    {
        Many(Vec<T>),
        One(Option<T>),
    }

    #[test]
    fn generics_of_the_enum_carry_into_the_impl() {
        let text = String::from("abc");
        let head: Part<'_> = Head(&text).into();
        let tail: Part<'_> = Tail(&text).into();
        assert_eq!((head.get(), tail.get()), ("a", "bc"));
        let many: Container<u8> = vec![1, 2, 3].into();
        let one: Container<u8> = Some(1).into();
        assert_eq!((many.len(), one.len()), (3, 1));
    }
}

mod cfg_on_variants_and_methods {
    #[whichever::dispatchable]
    trait Gated {
        fn always(&self) -> u8;
        #[cfg(any())]
        fn never(&self) -> Missing;
    }

    struct Present;

    impl Gated for Present {
        fn always(&self) -> u8 {
            1
        }
    }

    #[whichever::dispatch(Gated)]
    enum Gate {
        Present(Present),
        #[cfg(any())]
        Absent(Missing),
    }

    #[test]
    fn disabled_items_are_disabled_everywhere() {
        assert_eq!(Gate::from(Present).always(), 1);
    }
}

mod inherent_methods_do_not_shadow {
    #[whichever::dispatchable]
    trait Named {
        fn name(&self) -> &'static str;
    }

    struct Printer;

    impl Printer {
        fn name(&self) -> &'static str {
            "inherent"
        }
    }

    impl Named for Printer {
        fn name(&self) -> &'static str {
            "trait"
        }
    }

    struct Scanner;

    impl Named for Scanner {
        fn name(&self) -> &'static str {
            "scanner"
        }
    }

    #[whichever::dispatch(Named)]
    enum Device {
        Printer(Printer),
        Scanner(Scanner),
    }

    #[test]
    fn calls_are_fully_qualified() {
        assert_eq!(Named::name(&Device::from(Printer)), "trait");
        assert_eq!(Device::from(Printer).name(), "trait");
        assert_eq!(Device::from(Scanner).name(), "scanner");
    }
}

mod several_traits {
    #[whichever::dispatchable]
    trait Base {
        fn base(&self) -> u8;
    }

    #[whichever::dispatchable]
    trait Derived: Base {
        fn derived(&self) -> u8 {
            self.base() * 10
        }
    }

    struct One;
    struct Two;

    impl Base for One {
        fn base(&self) -> u8 {
            1
        }
    }

    impl Base for Two {
        fn base(&self) -> u8 {
            2
        }
    }

    impl Derived for One {}

    impl Derived for Two {
        fn derived(&self) -> u8 {
            99
        }
    }

    #[whichever::dispatch(Base)]
    #[dispatch(Derived)]
    enum Number {
        One(One),
        Two(Two),
    }

    #[test]
    fn each_attribute_dispatches_one_trait() {
        assert_eq!(Number::from(One).derived(), 10);
        assert_eq!(Number::from(Two).derived(), 99);
        assert_eq!(Number::from(Two).base(), 2);
    }
}

mod from_impls {
    #[whichever::dispatchable]
    trait Value {
        fn value(&self) -> u8;
    }

    struct Left(u8);

    impl Value for Left {
        fn value(&self) -> u8 {
            self.0
        }
    }

    impl Value for u8 {
        fn value(&self) -> u8 {
            *self
        }
    }

    #[whichever::dispatch(Value, from = false)]
    enum Handmade {
        Left(Left),
    }

    impl From<Left> for Handmade {
        fn from(left: Left) -> Self {
            Handmade::Left(Left(left.0 + 100))
        }
    }

    #[whichever::dispatch(Value)]
    enum Twice {
        First(u8),
        Second(u8),
    }

    impl From<u8> for Twice {
        fn from(value: u8) -> Self {
            Twice::Second(value)
        }
    }

    #[test]
    fn generation_can_be_switched_off_and_skips_repeated_payloads() {
        assert_eq!(Handmade::from(Left(1)).value(), 101);
        assert!(matches!(Twice::from(3), Twice::Second(3)));
    }
}

mod same_named_traits_in_one_crate {
    mod metric {
        #[whichever::dispatchable]
        pub trait Unit {
            fn label(&self) -> &'static str;
        }
        pub struct Meter;
        impl Unit for Meter {
            fn label(&self) -> &'static str {
                "m"
            }
        }
        #[whichever::dispatch(Unit)]
        pub enum Length {
            Meter(Meter),
        }
    }

    mod imperial {
        #[whichever::dispatchable]
        pub trait Unit {
            fn label(&self) -> &'static str;
        }
        pub struct Foot;
        impl Unit for Foot {
            fn label(&self) -> &'static str {
                "ft"
            }
        }
        #[whichever::dispatch(Unit)]
        pub enum Length {
            Foot(Foot),
        }
    }

    #[test]
    fn exported_macros_do_not_collide() {
        use imperial::Unit as _;
        use metric::Unit as _;
        assert_eq!(metric::Length::from(metric::Meter).label(), "m");
        assert_eq!(imperial::Length::from(imperial::Foot).label(), "ft");
    }
}

mod method_attribute_spelled_with_the_crate_path {
    #[whichever::dispatchable]
    trait Eq2 {
        #[whichever::dispatch(mismatch = false)]
        fn eq2(&self, other: &Self) -> bool;
    }

    struct A;
    struct B;

    impl Eq2 for A {
        fn eq2(&self, _: &Self) -> bool {
            true
        }
    }

    impl Eq2 for B {
        fn eq2(&self, _: &Self) -> bool {
            true
        }
    }

    #[whichever::dispatch(Eq2)]
    enum Either {
        A(A),
        B(B),
    }

    #[test]
    fn both_spellings_are_accepted() {
        assert!(Either::from(A).eq2(&A.into()));
        assert!(!Either::from(A).eq2(&B.into()));
    }
}

mod dereferenced_payloads {
    use std::rc::Rc;

    #[whichever::dispatchable]
    trait Area {
        fn area(&self) -> u32;
        #[dispatch(mismatch = false)]
        fn same_area(&self, other: &Self) -> bool;
    }

    #[whichever::dispatchable]
    trait Grow {
        fn grow(&mut self, by: u32);
    }

    #[whichever::dispatchable]
    trait Consume {
        fn into_side(self) -> u32;
    }

    struct Square(u32);
    struct Circle(u32);

    impl Area for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
        fn same_area(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Area for Circle {
        fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
        fn same_area(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Grow for Square {
        fn grow(&mut self, by: u32) {
            self.0 += by;
        }
    }

    impl Consume for Square {
        fn into_side(self) -> u32 {
            self.0
        }
    }

    #[whichever::dispatch(Area)]
    enum Shape<'a> {
        #[dispatch(deref)]
        Boxed(Box<Square>),
        #[dispatch(deref)]
        Shared(Rc<Circle>),
        #[dispatch(deref)]
        Borrowed(&'a Square),
        Plain(Circle),
    }

    #[whichever::dispatch(Grow)]
    enum Growable<'a> {
        #[dispatch(deref)]
        Boxed(Box<Square>),
        #[whichever::dispatch(deref)]
        Borrowed(&'a mut Square),
    }

    #[whichever::dispatch(Consume)]
    enum Consumable {
        #[dispatch(deref)]
        Boxed(Box<Square>),
        Plain(Square),
    }

    #[test]
    fn the_trait_is_called_on_the_dereferenced_payload() {
        let square = Square(2);
        let shapes = [
            Shape::Boxed(Box::new(Square(2))),
            Shape::Shared(Rc::new(Circle(2))),
            Shape::Borrowed(&square),
            Shape::Plain(Circle(2)),
        ];
        let areas: Vec<u32> = shapes.iter().map(Area::area).collect();
        assert_eq!(areas, [4, 12, 4, 12]);
        assert!(shapes[0].same_area(&Shape::Boxed(Box::new(Square(2)))));
        assert!(!shapes[0].same_area(&shapes[2]));

        let mut boxed = Growable::Boxed(Box::new(Square(1)));
        boxed.grow(2);
        let mut owned = Square(5);
        let mut borrowed = Growable::Borrowed(&mut owned);
        borrowed.grow(1);
        assert_eq!(owned.0, 6);

        assert_eq!(Consumable::Boxed(Box::new(Square(3))).into_side(), 3);
        assert_eq!(Consumable::Plain(Square(4)).into_side(), 4);
    }
}
