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

mod same_named_traits_emitted_by_one_macro {
    // One `macro_rules!` emits the same trait text in two modules. Neither
    // the text nor the call site of `#[dispatchable]`, which lies in the
    // macro's body for both expansions, tells them apart; the tokens the
    // invocation supplies, the trait's name or a method's, do.
    macro_rules! area_trait {
        ($(#[$meta:meta])* $name:ident) => {
            $(#[$meta])*
            pub trait $name {
                fn area(&self) -> u64;
            }
        };
    }

    macro_rules! measure_trait {
        ($method:ident) => {
            #[whichever::dispatchable]
            pub trait Measure {
                fn $method(&self) -> u64;
            }
        };
    }

    mod one {
        area_trait!(
            #[whichever::dispatchable]
            Area
        );
        measure_trait!(perimeter);

        pub struct Square(pub u64);

        impl Area for Square {
            fn area(&self) -> u64 {
                self.0 * self.0
            }
        }

        impl Measure for Square {
            fn perimeter(&self) -> u64 {
                self.0 * 4
            }
        }

        #[whichever::dispatch(Area)]
        #[dispatch(Measure)]
        pub enum Shape {
            Square(Square),
        }
    }

    mod two {
        area_trait!(
            #[whichever::dispatchable]
            Area
        );
        measure_trait!(perimeter);

        pub struct Rectangle(pub u64, pub u64);

        impl Area for Rectangle {
            fn area(&self) -> u64 {
                self.0 * self.1
            }
        }

        impl Measure for Rectangle {
            fn perimeter(&self) -> u64 {
                (self.0 + self.1) * 2
            }
        }

        #[whichever::dispatch(Area)]
        #[dispatch(Measure)]
        pub enum Shape {
            Rectangle(Rectangle),
        }
    }

    #[test]
    fn exported_macros_do_not_collide() {
        use one::{Area as _, Measure as _};
        use two::{Area as _, Measure as _};
        let square = one::Shape::from(one::Square(3));
        let rectangle = two::Shape::from(two::Rectangle(2, 5));
        assert_eq!((square.area(), square.perimeter()), (9, 12));
        assert_eq!((rectangle.area(), rectangle.perimeter()), (10, 14));
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

mod inherent_methods {
    #[whichever::dispatchable]
    trait Shape {
        fn area(&self) -> u32;
    }

    #[derive(Debug, PartialEq)]
    struct Circle(u32);
    #[derive(Debug, PartialEq)]
    struct Square(u32);

    impl Circle {
        fn radius(&self) -> u32 {
            self.0
        }
        fn describe(&self, prefix: &str) -> String {
            format!("{prefix} circle {}", self.0)
        }
        async fn fetch(&self) -> u32 {
            self.0
        }
        fn into_radius(self) -> u32 {
            self.0
        }
    }

    impl Square {
        fn describe(&self, prefix: &str) -> String {
            format!("{prefix} square {}", self.0)
        }
        async fn fetch(&self) -> u32 {
            self.0 * 10
        }
        fn grow(&mut self, by: u32) {
            self.0 += by;
        }
    }

    impl Shape for Circle {
        fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
    }

    impl Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Shape, try_from = true)]
    enum AnyShape {
        Circle(Circle),
        Square(Square),
    }

    #[whichever::dispatch]
    impl AnyShape {
        /// Only circles have a radius.
        #[dispatch(Circle, with = Some)]
        pub fn radius(&self) -> Option<u32> {
            None
        }

        #[dispatch(*)]
        pub fn describe(&self, prefix: &str) -> String;

        #[dispatch(*, call = describe)]
        pub fn label(&self, prefix: &str) -> String;

        #[dispatch(*)]
        pub async fn fetch(&self) -> u32;

        /// Circles keep their size; the argument is still in scope here.
        #[dispatch(Square)]
        pub fn grow(&mut self, by: u32) {
            assert!(by > 0, "a circle cannot grow by {by}");
        }

        #[dispatch(Circle, with = Some)]
        pub fn into_radius(self) -> Option<u32> {
            None
        }

        pub fn untouched(&self) -> u8 {
            7
        }
    }

    #[test]
    fn marked_methods_forward_and_the_body_is_the_fallback() {
        let circle: AnyShape = Circle(2).into();
        let mut square: AnyShape = Square(3).into();
        assert_eq!((circle.radius(), square.radius()), (Some(2), None));
        assert_eq!(circle.describe("a"), "a circle 2");
        assert_eq!(square.label("the"), "the square 3");
        assert_eq!(pollster::block_on(square.fetch()), 30);
        square.grow(1);
        assert_eq!(square, AnyShape::Square(Square(4)));
        let mut unchanged: AnyShape = Circle(5).into();
        unchanged.grow(1);
        assert_eq!(unchanged, AnyShape::Circle(Circle(5)));
        assert_eq!(unchanged.untouched(), 7);
        assert_eq!(circle.into_radius(), Some(2));
        assert_eq!(AnyShape::from(Square(1)).into_radius(), None);
    }

    #[test]
    fn try_from_returns_the_payload_or_the_enum_back() {
        let circle: Result<Circle, AnyShape> = AnyShape::Circle(Circle(1)).try_into();
        assert_eq!(circle, Ok(Circle(1)));
        let not_a_square: Result<Square, AnyShape> = AnyShape::Circle(Circle(1)).try_into();
        assert_eq!(not_a_square, Err(AnyShape::Circle(Circle(1))));
    }
}

mod enums_inside_functions {
    #[whichever::dispatchable]
    trait Noise {
        fn noise(&self) -> &'static str;
    }

    struct Cat;
    struct Dog;

    impl Cat {
        fn purr(&self) -> &'static str {
            "purr"
        }
    }

    impl Noise for Cat {
        fn noise(&self) -> &'static str {
            "meow"
        }
    }

    impl Noise for Dog {
        fn noise(&self) -> &'static str {
            "woof"
        }
    }

    #[test]
    fn a_dispatch_enum_may_live_in_a_function_body() {
        #[whichever::dispatch(Noise)]
        enum Pet {
            Cat(Cat),
            Dog(Dog),
        }

        #[whichever::dispatch]
        impl Pet {
            #[dispatch(Cat)]
            fn purr(&self) -> &'static str {
                "silence"
            }
        }

        assert_eq!(Pet::from(Cat).noise(), "meow");
        assert_eq!(
            (Pet::from(Cat).purr(), Pet::from(Dog).purr()),
            ("purr", "silence")
        );
    }
}

mod generic_enums_with_associated_types {
    #[whichever::dispatchable]
    trait Storage {
        type Error;
        fn load(&self) -> Result<u32, Self::Error>;
    }

    struct Memory<T>(T);
    struct Disk<T>(T);

    impl<T> Storage for Memory<T> {
        type Error = u8;
        fn load(&self) -> Result<u32, u8> {
            Ok(1)
        }
    }

    impl<T> Storage for Disk<T> {
        type Error = u8;
        fn load(&self) -> Result<u32, u8> {
            Err(2)
        }
    }

    #[whichever::dispatch(Storage)]
    enum AnyStorage<T> {
        Memory(Memory<T>),
        Disk(Disk<T>),
    }

    #[whichever::dispatchable]
    trait Lend {
        type Item;
        fn lend(&self) -> Self::Item;
    }

    struct Head<'a>(&'a str);
    struct Tail<'a>(&'a str);

    impl<'a> Lend for Head<'a> {
        type Item = &'a str;
        fn lend(&self) -> &'a str {
            &self.0[..1]
        }
    }

    impl<'a> Lend for Tail<'a> {
        type Item = &'a str;
        fn lend(&self) -> &'a str {
            &self.0[1..]
        }
    }

    #[whichever::dispatch(Lend)]
    enum Part<'a> {
        Head(Head<'a>),
        Tail(Tail<'a>),
    }

    #[test]
    fn the_agreement_check_names_the_enum_generics() {
        let memory: AnyStorage<String> = Memory(String::new()).into();
        let disk: AnyStorage<String> = Disk(String::new()).into();
        assert_eq!((memory.load(), disk.load()), (Ok(1), Err(2)));
        let text = String::from("abc");
        let head: Part<'_> = Head(&text).into();
        assert_eq!(head.lend(), "a");
    }
}

// The explicit forms are the point of this module.
#[allow(clippy::needless_arbitrary_self_type, clippy::needless_lifetimes)]
mod explicit_receivers {
    #[whichever::dispatchable]
    trait Counter {
        fn value<'a>(self: &'a Self) -> &'a u32;
        fn bump(self: &mut Self);
        fn take(self: Self) -> u32;
    }

    struct Ones(u32);
    struct Tens(u32);

    impl Counter for Ones {
        fn value<'a>(self: &'a Self) -> &'a u32 {
            &self.0
        }
        fn bump(self: &mut Self) {
            self.0 += 1;
        }
        fn take(self: Self) -> u32 {
            self.0
        }
    }

    impl Counter for Tens {
        fn value<'a>(self: &'a Self) -> &'a u32 {
            &self.0
        }
        fn bump(self: &mut Self) {
            self.0 += 10;
        }
        fn take(self: Self) -> u32 {
            self.0
        }
    }

    impl Ones {
        fn doubled(&self) -> u32 {
            self.0 * 2
        }
    }

    impl Tens {
        fn doubled(&self) -> u32 {
            self.0 * 2
        }
    }

    #[whichever::dispatch(Counter)]
    enum AnyCounter {
        Ones(Ones),
        Tens(Tens),
    }

    #[whichever::dispatch]
    impl AnyCounter {
        #[dispatch(*)]
        fn doubled(self: &Self) -> u32;
    }

    #[test]
    fn explicit_forms_dispatch_like_the_shorthand() {
        let mut counter: AnyCounter = Tens(5).into();
        counter.bump();
        assert_eq!(*counter.value(), 15);
        assert_eq!(counter.doubled(), 30);
        assert_eq!(counter.take(), 15);
    }
}

// The desugared `-> impl Future` spelling is the point of this module.
#[allow(clippy::manual_async_fn, clippy::needless_lifetimes)]
mod impl_future_returns {
    use std::future::Future;

    #[whichever::dispatchable]
    trait Store {
        fn truncate(&mut self, max: u64) -> impl Future<Output = Result<(), ()>> + Send;
        fn consume(self, tag: &str) -> impl Future<Output = usize> + Send
        where
            Self: Sized;
        fn first<'a, 'b>(&'a self, s: &'b str) -> impl Future<Output = (&'a u8, usize)> + Send;
        fn copied(&self) -> impl Future<Output = Self> + Send
        where
            Self: Sized;
    }

    #[derive(Debug, PartialEq)]
    struct Memory(u8);
    #[derive(Debug, PartialEq)]
    struct Disk(u8);

    impl Store for Memory {
        fn truncate(&mut self, max: u64) -> impl Future<Output = Result<(), ()>> + Send {
            self.0 = self.0.min(max as u8);
            async { Ok(()) }
        }
        fn consume(self, tag: &str) -> impl Future<Output = usize> + Send {
            async move { tag.len() + usize::from(self.0) }
        }
        fn first<'a, 'b>(&'a self, s: &'b str) -> impl Future<Output = (&'a u8, usize)> + Send {
            async move { (&self.0, s.len()) }
        }
        fn copied(&self) -> impl Future<Output = Self> + Send {
            async move { Memory(self.0) }
        }
    }

    impl Store for Disk {
        fn truncate(&mut self, _max: u64) -> impl Future<Output = Result<(), ()>> + Send {
            async { Err(()) }
        }
        fn consume(self, tag: &str) -> impl Future<Output = usize> + Send {
            async move { tag.len() * usize::from(self.0) }
        }
        fn first<'a, 'b>(&'a self, s: &'b str) -> impl Future<Output = (&'a u8, usize)> + Send {
            async move { (&self.0, s.len()) }
        }
        fn copied(&self) -> impl Future<Output = Self> + Send {
            async move { Disk(self.0) }
        }
    }

    impl Memory {
        fn fetch(&self) -> impl Future<Output = u8> + Send + '_ {
            async move { self.0 }
        }
    }

    impl Disk {
        fn fetch(&self) -> impl Future<Output = u8> + Send + '_ {
            async move { self.0 * 10 }
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Store)]
    enum AnyStore {
        Memory(Memory),
        Disk(Disk),
    }

    #[whichever::dispatch]
    impl AnyStore {
        #[dispatch(*)]
        fn fetch(&self) -> impl Future<Output = u8> + Send + '_;
    }

    /// Compiles only when the enum's futures are `Send`.
    fn spawnable<T: Store + Sync>(
        store: &mut T,
    ) -> impl Future<Output = Result<(), ()>> + Send + '_ {
        store.truncate(1)
    }

    #[test]
    fn every_arm_is_awaited_inside_one_future() {
        let mut memory: AnyStore = Memory(9).into();
        assert_eq!(pollster::block_on(spawnable(&mut memory)), Ok(()));
        assert_eq!(memory, AnyStore::Memory(Memory(1)));
        let mut disk: AnyStore = Disk(2).into();
        assert_eq!(pollster::block_on(disk.truncate(1)), Err(()));
        assert_eq!(pollster::block_on(disk.first("ab")), (&2, 2));
        assert_eq!(pollster::block_on(disk.copied()), AnyStore::Disk(Disk(2)));
        assert_eq!(pollster::block_on(disk.fetch()), 20);
        assert_eq!(pollster::block_on(disk.consume("abc")), 6);
    }
}

mod qualified_self_returns {
    use std::io;

    #[whichever::dispatchable]
    trait Rebuild {
        fn reload(&self) -> io::Result<Self>
        where
            Self: Sized;
        fn twin(&self) -> std::result::Result<Self, u8>
        where
            Self: Sized;
        fn maybe(&self) -> core::option::Option<Self>
        where
            Self: Sized;
    }

    #[derive(Debug, PartialEq)]
    struct Small(u8);
    #[derive(Debug, PartialEq)]
    struct Big(u32);

    impl Rebuild for Small {
        fn reload(&self) -> io::Result<Self> {
            Ok(Small(self.0))
        }
        fn twin(&self) -> Result<Self, u8> {
            Ok(Small(self.0))
        }
        fn maybe(&self) -> Option<Self> {
            None
        }
    }

    impl Rebuild for Big {
        fn reload(&self) -> io::Result<Self> {
            Err(io::Error::other("offline"))
        }
        fn twin(&self) -> Result<Self, u8> {
            Err(self.0 as u8)
        }
        fn maybe(&self) -> Option<Self> {
            Some(Big(self.0))
        }
    }

    impl Small {
        fn copy(&self) -> Self {
            Small(self.0)
        }
        fn halved(&self) -> Option<Self> {
            self.0.is_multiple_of(2).then_some(Small(self.0 / 2))
        }
        fn grown(&self) -> Self {
            Small(self.0 + 1)
        }
    }

    impl Big {
        fn copy(&self) -> Self {
            Big(self.0)
        }
        fn halved(&self) -> Option<Self> {
            Some(Big(self.0 / 2))
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Rebuild)]
    enum Number {
        Small(Small),
        Big(Big),
    }

    #[whichever::dispatch]
    impl Number {
        #[dispatch(*)]
        fn copy(&self) -> Self;

        #[dispatch(*)]
        fn halved(&self) -> Option<Self>;

        /// The payload returns the plain value; `with` lifts the variant.
        #[dispatch(Small, with = Some)]
        fn grown(&self) -> Option<Self> {
            None
        }
    }

    #[test]
    fn qualified_and_aliased_paths_are_mapped_like_the_bare_ones() {
        let small: Number = Small(4).into();
        let big: Number = Big(7).into();
        assert_eq!(small.reload().unwrap(), Number::Small(Small(4)));
        assert_eq!(big.reload().unwrap_err().to_string(), "offline");
        assert_eq!(small.twin(), Ok(Number::Small(Small(4))));
        assert_eq!(big.twin(), Err(7));
        assert_eq!(
            (small.maybe(), big.maybe()),
            (None, Some(Number::Big(Big(7))))
        );
    }

    #[test]
    fn inherent_methods_wrap_self_back_into_the_variant() {
        let small: Number = Small(4).into();
        let big: Number = Big(7).into();
        assert_eq!(small.copy(), Number::Small(Small(4)));
        assert_eq!(big.halved(), Some(Number::Big(Big(3))));
        assert_eq!(Number::from(Small(3)).halved(), None);
        assert_eq!(small.grown(), Some(Number::Small(Small(5))));
        assert_eq!(big.grown(), None);
    }
}

mod non_pub_traits_stay_in_their_module {
    // Declared before the traits' module on purpose: the enum reaches the
    // carrier macro through the trait's re-export, not through textual order.
    mod enums {
        use super::traits::{Area, Sealed};

        pub(super) struct Square(pub u32);
        pub(super) struct Circle(pub u32);

        impl Area for Square {
            fn area(&self) -> u32 {
                self.0 * self.0
            }
        }

        impl Area for Circle {
            fn area(&self) -> u32 {
                3 * self.0 * self.0
            }
        }

        impl Sealed for Square {}
        impl Sealed for Circle {
            fn sealed(&self) -> bool {
                false
            }
        }

        #[whichever::dispatch(Area)]
        #[whichever::dispatch(Sealed)]
        pub(super) enum Shape {
            Square(Square),
            Circle(Circle),
        }
    }

    mod traits {
        #[whichever::dispatchable]
        pub(crate) trait Area {
            fn area(&self) -> u32;
        }

        #[whichever::dispatchable]
        pub(super) trait Sealed {
            fn sealed(&self) -> bool {
                true
            }
        }
    }

    #[test]
    fn a_crate_private_trait_dispatches_from_a_sibling_module() {
        use traits::{Area, Sealed};
        let square: enums::Shape = enums::Square(2).into();
        let circle: enums::Shape = enums::Circle(1).into();
        assert_eq!((square.area(), circle.area()), (4, 3));
        assert!(square.sealed());
        assert!(!circle.sealed());
    }
}

mod partial_impl_blocks {
    #[whichever::dispatchable]
    trait Handler {
        type Value;
        fn from_handler(handler: AnyHandler) -> Option<Self>
        where
            Self: Sized;
        fn value(&self) -> Self::Value;
        fn is_attached(&self) -> bool;
        fn describe(&self) -> String {
            format!("attached: {}", self.is_attached())
        }
    }

    #[derive(Debug, PartialEq)]
    struct Text(u32);
    #[derive(Debug, PartialEq)]
    struct Map(u8);

    impl Handler for Text {
        type Value = u32;
        fn from_handler(handler: AnyHandler) -> Option<Self> {
            match handler {
                AnyHandler::Text(text) => Some(text),
                _ => None,
            }
        }
        fn value(&self) -> u32 {
            self.0
        }
        fn is_attached(&self) -> bool {
            true
        }
    }

    impl Handler for Map {
        type Value = u8;
        fn from_handler(handler: AnyHandler) -> Option<Self> {
            match handler {
                AnyHandler::Map(map) => Some(map),
                _ => None,
            }
        }
        fn value(&self) -> u8 {
            self.0
        }
        fn is_attached(&self) -> bool {
            false
        }
        fn describe(&self) -> String {
            String::from("a map")
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch]
    enum AnyHandler {
        Text(Text),
        Map(Map),
    }

    #[whichever::dispatch]
    impl Handler for AnyHandler {
        type Value = u64;

        fn from_handler(handler: AnyHandler) -> Option<Self> {
            Some(handler)
        }
    }

    #[test]
    fn the_block_holds_the_enums_own_items_and_the_rest_is_forwarded() {
        let text = AnyHandler::Text(Text(7));
        let map = AnyHandler::Map(Map(3));
        assert_eq!(
            AnyHandler::from_handler(text),
            Some(AnyHandler::Text(Text(7)))
        );
        assert_eq!(Text::from_handler(map), None);
        assert_eq!(AnyHandler::Text(Text(7)).value(), 7u64);
        assert_eq!(AnyHandler::Map(Map(3)).value(), 3u64);
        assert_eq!(AnyHandler::Text(Text(7)).describe(), "attached: true");
        assert_eq!(AnyHandler::Map(Map(3)).describe(), "a map");
    }

    #[whichever::dispatchable]
    trait Shell {
        fn name(&self) -> &'static str;
        fn prompt(&self) -> String {
            format!("{}$ ", self.name())
        }
    }

    struct Bash;
    struct Fish;
    struct Custom<T>(T);

    impl Shell for Bash {
        fn name(&self) -> &'static str {
            "bash"
        }
    }

    impl Shell for Fish {
        fn name(&self) -> &'static str {
            "fish"
        }
        fn prompt(&self) -> String {
            String::from("> ")
        }
    }

    impl<T: Default + Copy> Shell for Custom<T> {
        fn name(&self) -> &'static str {
            "custom"
        }
    }

    #[whichever::dispatch]
    enum AnyShell<T> {
        Bash(Bash),
        Fish(Fish),
        Custom(Custom<T>),
    }

    // The block's own header, bounds included, carries into the impl, and a
    // method written here overrides every payload's, provided ones included.
    #[whichever::dispatch]
    impl<T: Default + Copy> Shell for AnyShell<T> {
        fn prompt(&self) -> String {
            format!("[{}] ", self.name())
        }
    }

    #[test]
    fn a_method_written_in_the_block_overrides_the_payloads() {
        let fish: AnyShell<u8> = Fish.into();
        assert_eq!(fish.prompt(), "[fish] ");
        assert_eq!(AnyShell::<u8>::Bash(Bash).prompt(), "[bash] ");
        assert_eq!(AnyShell::Custom(Custom(1u8)).prompt(), "[custom] ");
    }
}

mod foreign_traits {
    use std::fmt;
    use std::io::{self, Read, Write};
    use std::num::ParseIntError;

    #[derive(Debug)]
    #[whichever::dispatch(whichever::foreign::Display)]
    enum ClientError {
        Io(io::Error),
        Parse(ParseIntError),
        #[dispatch(from = false)]
        Message(String),
    }

    // `String` is no `Error`, so the enum answers `source` itself; the
    // block names the declaration and the impl is of the real trait.
    #[whichever::dispatch]
    impl whichever::foreign::Error for ClientError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ClientError::Io(error) => Some(error),
                _ => None,
            }
        }
    }

    // Would conflict with a generated impl, so it proves `from = false`.
    impl From<String> for ClientError {
        fn from(message: String) -> Self {
            ClientError::Message(format!("hand-written: {message}"))
        }
    }

    #[derive(Debug)]
    #[whichever::dispatch(whichever::foreign::Display)]
    #[whichever::dispatch(whichever::foreign::Error)]
    enum ReadError {
        Io(io::Error),
        Parse(ParseIntError),
    }

    #[test]
    fn display_and_error_forward_to_the_payload() {
        let parse: ClientError = "x".parse::<u32>().unwrap_err().into();
        assert_eq!(parse.to_string(), "invalid digit found in string");
        let io: ClientError = io::Error::other("offline").into();
        assert_eq!(io.to_string(), "offline");
        assert!(std::error::Error::source(&io).is_some());
        let message: ClientError = String::from("bad url").into();
        assert_eq!(message.to_string(), "hand-written: bad url");
        assert!(std::error::Error::source(&message).is_none());
        let read: ReadError = io::Error::other("offline").into();
        assert_eq!(read.to_string(), "offline");
        assert!(std::error::Error::source(&read).is_none());
    }

    #[derive(Debug)]
    struct Never;

    #[whichever::dispatch(whichever::foreign::Read)]
    #[whichever::dispatch(whichever::foreign::Write)]
    enum Stream {
        Memory(io::Cursor<Vec<u8>>),
        Empty(io::Empty),
        #[cfg(any())]
        Never(Never),
    }

    #[test]
    fn io_traits_forward_the_provided_methods_too() {
        let mut stream: Stream = io::Cursor::new(Vec::new()).into();
        stream.write_all(b"hello ").unwrap();
        write!(stream, "{}", 42).unwrap();
        let Stream::Memory(cursor) = &mut stream else {
            unreachable!()
        };
        cursor.set_position(0);
        let mut text = String::new();
        stream.read_to_string(&mut text).unwrap();
        assert_eq!(text, "hello 42");
        let mut exact = [0u8; 3];
        assert!(Stream::from(io::empty()).read_exact(&mut exact).is_err());
    }

    #[whichever::dispatch(whichever::foreign::Iterator)]
    enum Numbers {
        Range(std::ops::Range<u32>),
        Owned(std::vec::IntoIter<u32>),
    }

    #[test]
    fn iterator_forwards_and_the_adapters_still_work() {
        let range: Numbers = (1..4).into();
        assert_eq!(range.size_hint(), (3, Some(3)));
        assert_eq!(range.fold(String::new(), |s, n| format!("{s}{n}")), "123");
        let owned: Numbers = vec![5, 6, 7].into_iter().into();
        assert_eq!(owned.map(|n| n * 2).collect::<Vec<_>>(), [10, 12, 14]);
        assert_eq!(Numbers::from(1..4).last(), Some(3));
    }

    #[derive(Debug)]
    #[whichever::dispatch(whichever::foreign::PartialEq)]
    enum Id {
        Number(u32),
        Name(String),
    }

    #[test]
    fn partial_eq_compares_same_variants_and_never_different_ones() {
        assert_eq!(Id::Number(1), Id::Number(1));
        assert_ne!(Id::Number(1), Id::Number(2));
        assert_ne!(Id::Number(1), Id::Name(String::from("1")));
    }

    // A restatement coexists with the real trait's import in one module,
    // and a generic method goes through.
    #[whichever::dispatchable(remote = fmt::LowerHex)]
    trait LowerHex {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    #[whichever::dispatchable(remote = std::hash::Hash)]
    trait Hash {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H);
    }

    #[whichever::dispatch(LowerHex)]
    #[whichever::dispatch(Hash)]
    enum Word {
        Short(u8),
        Long(u64),
    }

    #[test]
    fn a_restated_trait_is_implemented_at_its_real_path() {
        use std::hash::{Hash, Hasher};
        assert_eq!(format!("{:x}", Word::Short(255)), "ff");
        let mut hasher = std::hash::DefaultHasher::new();
        Word::Long(7).hash(&mut hasher);
        let mut expected = std::hash::DefaultHasher::new();
        7u64.hash(&mut expected);
        assert_eq!(hasher.finish(), expected.finish());
    }
}

mod named_carrier_macros {
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
}

mod associated_const_agreement {
    #[whichever::dispatchable]
    trait Proof {
        const KIND: &'static str;
        const ARITY: usize;
        const UNCHECKED: Option<u8>;
        fn check(&self) -> bool;
    }

    struct Encoding;
    struct Ordering;

    impl Proof for Encoding {
        const KIND: &'static str = "encoding";
        const ARITY: usize = 1;
        const UNCHECKED: Option<u8> = Some(1);
        fn check(&self) -> bool {
            true
        }
    }

    impl Proof for Ordering {
        const KIND: &'static str = "ordering";
        const ARITY: usize = 1;
        const UNCHECKED: Option<u8> = Some(2);
        fn check(&self) -> bool {
            false
        }
    }

    const ANY: &str = "any";

    // `KIND` differs per payload, so the enum sets its own; `ARITY` agrees
    // and is checked; `UNCHECKED` has a type the check cannot compare and
    // comes from the first variant.
    #[whichever::dispatch(Proof, KIND = ANY)]
    enum AnyProof {
        Encoding(Encoding),
        Ordering(Ordering),
    }

    #[whichever::dispatch(Proof, KIND = "literal", ARITY = 1 + 1)]
    enum OtherProof {
        Encoding(Encoding),
        Ordering(Ordering),
    }

    #[test]
    fn consts_are_overridden_or_checked_to_agree() {
        assert_eq!(<AnyProof as Proof>::KIND, "any");
        assert_eq!(<AnyProof as Proof>::ARITY, 1);
        assert_eq!(<AnyProof as Proof>::UNCHECKED, Some(1));
        assert_eq!(<OtherProof as Proof>::KIND, "literal");
        assert_eq!(<OtherProof as Proof>::ARITY, 2);
        assert!(AnyProof::Encoding(Encoding).check());
    }
}

mod newtype_delegation {
    #[whichever::dispatchable]
    trait FlowCtrl {
        fn can_send<M: AsRef<str>>(&self, msg: &M) -> bool;
        fn about_to_send(&mut self, bytes: u32);
        fn sent(&self) -> u32;
        fn label(&self) -> String {
            format!("{} sent", self.sent())
        }
        #[dispatch(mismatch = 0)]
        fn merged(&self, other: &Self) -> u32;
    }

    struct Window(u32);
    struct XonXoff(u32);

    impl FlowCtrl for Window {
        fn can_send<M: AsRef<str>>(&self, msg: &M) -> bool {
            msg.as_ref().len() as u32 <= self.0
        }
        fn about_to_send(&mut self, bytes: u32) {
            self.0 = self.0.saturating_sub(bytes);
        }
        fn sent(&self) -> u32 {
            100 - self.0
        }
        fn label(&self) -> String {
            String::from("window")
        }
        fn merged(&self, other: &Self) -> u32 {
            self.0 + other.0
        }
    }

    impl FlowCtrl for XonXoff {
        fn can_send<M: AsRef<str>>(&self, _msg: &M) -> bool {
            self.0 > 0
        }
        fn about_to_send(&mut self, _bytes: u32) {
            self.0 += 1;
        }
        fn sent(&self) -> u32 {
            self.0
        }
        fn merged(&self, other: &Self) -> u32 {
            self.0 * other.0
        }
    }

    #[whichever::dispatch(FlowCtrl)]
    enum Inner {
        Window(Window),
        XonXoff(XonXoff),
    }

    /// The public wrapper over the private enum, with a field of its own.
    #[whichever::dispatch(FlowCtrl)]
    struct StreamFlowCtrl {
        #[dispatch]
        inner: Inner,
        hooks: u8,
    }

    /// A payload behind a `Box`.
    #[whichever::dispatch(FlowCtrl)]
    struct BoxedFlowCtrl {
        #[dispatch(deref)]
        inner: Box<Inner>,
    }

    /// A struct that overrides one method keeps the payload's overrides of
    /// the others.
    #[whichever::dispatch]
    struct Newline(Window);

    #[whichever::dispatch]
    impl FlowCtrl for Newline {
        fn sent(&self) -> u32 {
            self.0.sent() + 1
        }
    }

    #[test]
    fn a_struct_forwards_to_its_field_and_keeps_the_payloads_overrides() {
        let mut ctrl = StreamFlowCtrl {
            inner: Window(100).into(),
            hooks: 0,
        };
        assert!(ctrl.can_send(&"hello"));
        ctrl.about_to_send(40);
        assert_eq!((ctrl.sent(), ctrl.hooks), (40, 0));
        assert_eq!(ctrl.label(), "window");
        let other = StreamFlowCtrl {
            inner: Window(10).into(),
            hooks: 1,
        };
        assert_eq!(ctrl.merged(&other), 70);
        let mismatched = StreamFlowCtrl {
            inner: XonXoff(3).into(),
            hooks: 1,
        };
        assert_eq!(ctrl.merged(&mismatched), 0);

        let boxed = BoxedFlowCtrl {
            inner: Box::new(XonXoff(2).into()),
        };
        assert_eq!((boxed.sent(), boxed.label()), (2, String::from("2 sent")));

        let newline = Newline(Window(90));
        assert_eq!(
            (newline.sent(), newline.label()),
            (11, String::from("window"))
        );
    }

    #[whichever::dispatchable]
    trait Grow {
        fn grown(&self) -> Self
        where
            Self: Sized;
        fn halved(&self) -> Option<Self>
        where
            Self: Sized;
    }

    #[derive(Debug, PartialEq)]
    struct Size(u32);

    impl Grow for Size {
        fn grown(&self) -> Self {
            Size(self.0 + 1)
        }
        fn halved(&self) -> Option<Self> {
            self.0.is_multiple_of(2).then_some(Size(self.0 / 2))
        }
    }

    /// A single-field struct needs no marker and is rebuilt around `Self`.
    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Grow)]
    struct Wrapped(Size);

    #[whichever::dispatch]
    impl Wrapped {
        #[dispatch(*)]
        fn grown_again(&self) -> Self;
    }

    impl Size {
        fn grown_again(&self) -> Self {
            Size(self.0 + 2)
        }
    }

    #[test]
    fn a_single_field_struct_is_rebuilt_around_self() {
        assert_eq!(Wrapped(Size(4)).grown(), Wrapped(Size(5)));
        assert_eq!(Wrapped(Size(4)).halved(), Some(Wrapped(Size(2))));
        assert_eq!(Wrapped(Size(3)).halved(), None);
        assert_eq!(Wrapped(Size(4)).grown_again(), Wrapped(Size(6)));
    }
}

mod pinned_and_boxed_receivers {
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    #[whichever::dispatchable]
    trait Source {
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32>;
        fn peek(self: Pin<&Self>) -> u32;
    }

    struct Ready(u32);
    struct Pending;

    impl Source for Ready {
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
            Poll::Ready(self.0)
        }
        fn peek(self: Pin<&Self>) -> u32 {
            self.0
        }
    }

    impl Source for Pending {
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
            Poll::Pending
        }
        fn peek(self: Pin<&Self>) -> u32 {
            0
        }
    }

    impl Ready {
        fn poll_once(self: Pin<&mut Self>) -> u32 {
            self.0
        }
    }

    #[whichever::dispatch(Source)]
    enum AnySource {
        Ready(Ready),
        Pending(Pending),
        #[dispatch(deref)]
        Boxed(Box<Ready>),
    }

    #[whichever::dispatch]
    impl AnySource {
        #[dispatch(Ready, Boxed)]
        fn poll_once(self: Pin<&mut Self>) -> u32 {
            99
        }
    }

    #[whichever::dispatchable]
    trait Finish {
        fn finish(self: Box<Self>) -> u32;
    }

    impl Finish for Ready {
        fn finish(self: Box<Self>) -> u32 {
            self.0 * 10
        }
    }

    #[whichever::dispatch(Finish)]
    enum Finishable {
        #[dispatch(deref)]
        Boxed(Box<Ready>),
    }

    #[test]
    fn pinned_receivers_unpin_the_enum_and_pin_the_payload() {
        let mut cx = Context::from_waker(Waker::noop());
        let mut ready: AnySource = Ready(5).into();
        assert_eq!(Pin::new(&mut ready).poll_next(&mut cx), Poll::Ready(5));
        assert_eq!(Pin::new(&ready).peek(), 5);
        assert_eq!(Pin::new(&mut ready).poll_once(), 5);
        let mut pending: AnySource = Pending.into();
        assert_eq!(Pin::new(&mut pending).poll_next(&mut cx), Poll::Pending);
        assert_eq!(Pin::new(&mut pending).poll_once(), 99);
        let mut boxed = AnySource::Boxed(Box::new(Ready(7)));
        assert_eq!(Pin::new(&mut boxed).poll_once(), 7);
    }

    #[test]
    fn a_boxed_receiver_hands_over_the_payloads_own_box() {
        let finishable = Box::new(Finishable::Boxed(Box::new(Ready(3))));
        assert_eq!(finishable.finish(), 30);
    }
}

mod variants_without_payloads {
    #[whichever::dispatchable]
    trait Area {
        fn area(&self) -> u32;
    }

    struct Square(u32);

    impl Area for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    /// A test-only variant that is compiled out is no error.
    #[whichever::dispatch(Area)]
    enum Shape {
        Square(Square),
        #[cfg(any())]
        Mock,
    }

    struct IntBounds(u32, u32);
    struct TextBounds(&'static str);

    impl IntBounds {
        fn len(&self) -> usize {
            (self.1 - self.0) as usize
        }
    }

    impl TextBounds {
        fn len(&self) -> usize {
            self.0.len()
        }
    }

    /// A data enum: the odd variants are left to the inherent fallbacks.
    #[whichever::dispatch]
    enum Bounds {
        Int(IntBounds),
        Text(TextBounds),
        NoOrder,
        Pair(u8, u8),
    }

    #[whichever::dispatch]
    impl Bounds {
        #[dispatch(Int, Text)]
        fn len(&self) -> usize {
            0
        }
    }

    #[test]
    fn odd_variants_take_the_fallback_and_cfg_out_variants_are_no_error() {
        let shape: Shape = Square(3).into();
        assert_eq!(shape.area(), 9);
        let bounds: Bounds = IntBounds(2, 5).into();
        assert_eq!(bounds.len(), 3);
        assert_eq!(Bounds::Text(TextBounds("ab")).len(), 2);
        assert_eq!((Bounds::NoOrder.len(), Bounds::Pair(1, 2).len()), (0, 0));
    }
}

mod recursive_payloads_and_conversions {
    #[whichever::dispatchable]
    trait Eval {
        type Out;
        fn eval(&self) -> Self::Out;
    }

    #[derive(Debug, PartialEq)]
    struct Leaf(u32);
    #[derive(Debug, PartialEq)]
    struct Wrap<T>(Box<T>);

    impl Eval for Leaf {
        type Out = u32;
        fn eval(&self) -> u32 {
            self.0
        }
    }

    impl<T: Eval<Out = u32>> Eval for Wrap<T> {
        type Out = u32;
        fn eval(&self) -> u32 {
            self.0.eval() + 1
        }
    }

    /// Payloads naming the enum as `Self` convert in both directions.
    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Eval, try_from = true)]
    enum Node {
        Leaf(Leaf),
        Wrap(Wrap<Self>),
    }

    /// `try_from` on one variant only.
    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Eval)]
    enum Tree {
        Leaf(Leaf),
        #[dispatch(try_from = true)]
        Wrap(Wrap<Self>),
    }

    // Would conflict with a generated impl, so it proves the opt-in is per
    // variant.
    impl TryFrom<Tree> for Leaf {
        type Error = ();
        fn try_from(_tree: Tree) -> Result<Self, ()> {
            Err(())
        }
    }

    #[test]
    fn self_in_a_payload_names_the_enum_everywhere() {
        let node: Node = Wrap(Box::new(Node::Leaf(Leaf(1)))).into();
        assert_eq!(node.eval(), 2);
        let back: Result<Wrap<Node>, Node> = node.try_into();
        assert_eq!(back, Ok(Wrap(Box::new(Node::Leaf(Leaf(1))))));
        let leaf: Result<Leaf, Node> = Node::Leaf(Leaf(3)).try_into();
        assert_eq!(leaf, Ok(Leaf(3)));
        let tree = Tree::Wrap(Wrap(Box::new(Tree::Leaf(Leaf(1)))));
        assert_eq!(tree.eval(), 2);
        let wrap: Result<Wrap<Tree>, Tree> = tree.try_into();
        assert!(wrap.is_ok());
        assert_eq!(Leaf::try_from(Tree::Leaf(Leaf(1))), Err(()));
    }
}
