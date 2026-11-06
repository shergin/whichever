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
