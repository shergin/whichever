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
