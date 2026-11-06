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
