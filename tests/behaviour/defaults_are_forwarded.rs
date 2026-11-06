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
