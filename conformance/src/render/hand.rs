//! The hand: the subject as a plain enum with the forwarding a careful
//! hand writes, as `docs/what-is-generated.md` and the goldens show it.
//!
//! The borrows are derived from types, not from a table of cases. The
//! receiver fixes how the `match` binds the variant's field; the route
//! gives a place from the binding; the payload method's receiver says how
//! the place is handed over. The return type says how the answer is
//! converted.

use super::context::{pattern, variant, Context, Site, TRAIT};
use super::writer::Writer;
use crate::model::argument::ArgumentKind;
use crate::model::receiver::{Binding, Handover};
use crate::model::returns::{Associated, Conversion, FutureKind, Node};
use crate::model::target::{Step, TargetDescriptor};
use crate::model::{Receiver, Slot};

/// A place expression, kept as a tree so that a borrow of it can be
/// written the way a hand writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Place {
    /// The binding itself; `reference` says how it borrows the field it
    /// binds, `None` when it binds by value.
    Binding(&'static str, Option<Binding>),
    /// `*place`.
    Deref(Box<Place>),
    /// `place.name`.
    Field(Box<Place>, &'static str),
    /// `place.name()`: a reference the target is behind, or the target
    /// itself as a temporary.
    Call(Box<Place>, &'static str),
}

impl Place {
    /// Returns the place a route reaches from a binding.
    fn of(binding_name: &'static str, binding: Binding, route: &[Step]) -> Place {
        let mut place = match binding {
            Binding::Owned => Place::Binding(binding_name, None),
            Binding::Shared | Binding::Exclusive => {
                Place::Deref(Box::new(Place::Binding(binding_name, Some(binding))))
            }
        };
        for step in route {
            place = match step {
                Step::Deref => Place::Deref(Box::new(place)),
                Step::Field(name) => Place::Field(Box::new(place), name),
                Step::Method(name) => Place::Deref(Box::new(Place::Call(Box::new(place), name))),
                Step::Yield(name) => Place::Call(Box::new(place), name),
            };
        }
        place
    }

    /// Returns the place as an expression.
    fn text(&self) -> String {
        match self {
            Place::Binding(name, _) => String::from(*name),
            Place::Deref(inner) => format!("*{}", inner.text()),
            Place::Field(inner, name) => format!("{}.{name}", inner.receiver_text()),
            Place::Call(inner, name) => format!("{}.{name}()", inner.receiver_text()),
        }
    }

    /// Returns the place as the receiver of a field access or a method
    /// call, where a reference binding is dereferenced implicitly.
    fn receiver_text(&self) -> String {
        match self {
            Place::Deref(inner) if matches!(**inner, Place::Binding(_, Some(_))) => inner.text(),
            Place::Deref(_) => format!("({})", self.text()),
            Place::Binding(_, _) | Place::Field(_, _) | Place::Call(_, _) => self.text(),
        }
    }

    /// Returns the place borrowed, `exclusive` or shared; a dereference of
    /// a reference that is already the borrow asked for is that reference.
    fn borrowed(&self, exclusive: bool) -> String {
        if let Place::Deref(inner) = self {
            let already = match **inner {
                Place::Binding(_, Some(Binding::Exclusive)) => exclusive,
                Place::Binding(_, Some(Binding::Shared)) | Place::Call(_, _) => !exclusive,
                Place::Binding(_, Some(Binding::Owned) | None)
                | Place::Deref(_)
                | Place::Field(_, _) => false,
            };
            if already {
                return inner.text();
            }
        }
        match exclusive {
            true => format!("&mut {}", self.text()),
            false => format!("&{}", self.text()),
        }
    }

    /// Returns the place handed over as `handover` asks.
    fn handed(&self, handover: Handover) -> String {
        match handover {
            Handover::Borrow => self.borrowed(false),
            Handover::BorrowMutably => self.borrowed(true),
            Handover::Move => self.text(),
            Handover::MoveBox => match self {
                Place::Deref(inner) => inner.text(),
                Place::Binding(_, _) | Place::Field(_, _) | Place::Call(_, _) => {
                    unreachable!("the verdict hands a `Box` over only from a `deref` variant")
                }
            },
            Handover::Pin => format!("Pin::new({})", self.borrowed(false)),
            Handover::PinMutably => format!("Pin::new({})", self.borrowed(true)),
        }
    }
}

/// What one forwarded method is.
struct Method {
    name: &'static str,
    signature: String,
    receiver: Receiver,
    argument: Option<ArgumentKind>,
    node: Node,
    asyncness: bool,
    unsafety: bool,
    turbofish: &'static str,
    mismatch: Option<String>,
}

/// Writes `mod hand`.
pub fn write(writer: &mut Writer, context: &Context) {
    writer.line("/// The same subject with the forwarding written by hand.");
    writer.open("mod hand {");
    writer.line("use super::*;");
    writer.blank();
    let generics = context.generics();
    writer.open(format!("pub enum E{generics} {{"));
    for slot in &context.slots {
        let (_, declaration) = variant(slot, false);
        writer.line(declaration);
    }
    writer.close("}");
    writer.blank();
    writer.open(format!("impl{generics} {TRAIT} for E{generics} {{"));
    for (associated, overridden) in context.associated() {
        let name = associated.name();
        let ty = match (associated, overridden) {
            (Associated::Out | Associated::Error, true) => String::from("support::Wide"),
            (Associated::Out | Associated::Error, false) => format!("<P0 as {TRAIT}>::{name}"),
        };
        writer.line(format!("type {name} = {ty};"));
    }
    let base = Method {
        name: "base",
        signature: String::from("fn base(&self) -> u64"),
        receiver: Receiver::Shared,
        argument: None,
        node: Node::Tag,
        asyncness: false,
        unsafety: false,
        turbofish: "",
        mismatch: None,
    };
    write_method(writer, context, &base);
    let mismatch = context
        .argument
        .kind
        .filter(|_| context.slots.len() > 1)
        .map(|_| context.mismatch("E"));
    let method = Method {
        name: "m",
        signature: context.signature(Site::Impl),
        receiver: context.cell.recv,
        argument: context.argument.kind,
        node: context.node,
        asyncness: context.qualifier.asyncness,
        unsafety: context.qualifier.unsafety,
        turbofish: match context.qualifier.generic {
            Some(crate::model::qualifier::Generic::Turbofish) => "::<W>",
            Some(crate::model::qualifier::Generic::Fixed) | None => "",
        },
        mismatch,
    };
    write_method(writer, context, &method);
    writer.close("}");
    writer.close("}");
}

/// How the arms of a method meet its return type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wrapping {
    /// The `match` is the body.
    Plain,
    /// The `match` is in one `async move` block, every arm awaited.
    AsyncBlock,
    /// The `match` is in `Box::pin(async move { .. })`, every arm awaited.
    BoxedBlock,
}

fn write_method(writer: &mut Writer, context: &Context, method: &Method) {
    let receiver = method.receiver.descriptor();
    let unwrap = receiver
        .unwrap
        .expect("the verdict writes a hand only for a forwarded receiver");
    let (wrapping, awaited, conversion) = match method.node.future() {
        Some((FutureKind::Opaque, output)) => (Wrapping::AsyncBlock, true, output.conversion()),
        Some((FutureKind::Boxed, output)) => {
            let conversion = output.conversion();
            match conversion.as_ref().is_some_and(Conversion::is_keep) {
                true => (Wrapping::Plain, false, conversion),
                false => (Wrapping::BoxedBlock, true, conversion),
            }
        }
        None => (Wrapping::Plain, method.asyncness, method.node.conversion()),
    };
    let conversion =
        conversion.expect("the verdict writes a hand only for a conversion that exists");
    writer.blank();
    writer.line("#[inline]");
    writer.open(format!("{} {{", method.signature));
    match wrapping {
        Wrapping::Plain => {}
        Wrapping::AsyncBlock => writer.open("async move {"),
        Wrapping::BoxedBlock => writer.open("Box::pin(async move {"),
    }
    let scrutinee = match method.argument {
        Some(_) => format!("({}, other)", unwrap.scrutinee),
        None => String::from(unwrap.scrutinee),
    };
    writer.open(format!("match {scrutinee} {{"));
    for (slot, _, target) in context.live() {
        let arm = arm(
            &slot,
            &target,
            method,
            unwrap.binding,
            receiver.handover,
            awaited,
            &conversion,
        );
        writer.line(arm);
    }
    if let Some(mismatch) = &method.mismatch {
        writer.line(format!("_ => {mismatch},"));
    }
    writer.close("}");
    match wrapping {
        Wrapping::Plain => {}
        Wrapping::AsyncBlock => writer.close("}"),
        Wrapping::BoxedBlock => writer.close("})"),
    }
    writer.close("}");
}

fn arm(
    slot: &Slot,
    target: &TargetDescriptor,
    method: &Method,
    binding: Binding,
    handover: Handover,
    awaited: bool,
    conversion: &Conversion,
) -> String {
    let own = Place::of("inner", binding, target.route);
    let mut pattern_text = pattern("E", slot, target, "inner");
    let mut arguments = vec![own.handed(handover)];
    if let Some(kind) = method.argument {
        let (other_binding, other_handover) = match kind {
            ArgumentKind::Shared => (Binding::Shared, Handover::Borrow),
            ArgumentKind::Exclusive => (Binding::Exclusive, Handover::BorrowMutably),
            ArgumentKind::Owned => (Binding::Owned, Handover::Move),
        };
        let other = Place::of("same", other_binding, target.route);
        pattern_text = format!("({pattern_text}, {})", pattern("E", slot, target, "same"));
        arguments.push(other.handed(other_handover));
    }
    if method.name == "m" {
        arguments.push(String::from("x"));
    }
    let mut call = format!(
        "<{} as {TRAIT}>::{}{}({})",
        slot.payload(),
        method.name,
        method.turbofish,
        arguments.join(", ")
    );
    if method.unsafety {
        call = format!("unsafe {{ {call} }}");
    }
    if awaited {
        call = format!("{call}.await");
    }
    let variant = format!("E::{}", slot.name());
    format!(
        "{pattern_text} => {},",
        convert(conversion, &call, &variant)
    )
}

/// Returns `value` converted as `conversion` says, `variant` being the
/// constructor a rewrap calls.
fn convert(conversion: &Conversion, value: &str, variant: &str) -> String {
    match conversion {
        Conversion::Keep => String::from(value),
        Conversion::Rewrap => format!("{variant}({value})"),
        Conversion::Into => format!("Into::into({value})"),
        Conversion::Map(inner) => format!("{value}.map({})", path(inner, variant)),
        Conversion::Result(ok, error) => {
            let mut text = String::from(value);
            if !ok.is_keep() {
                text.push_str(&format!(".map({})", path(ok, variant)));
            }
            if !error.is_keep() {
                text.push_str(&format!(".map_err({})", path(error, variant)));
            }
            text
        }
        Conversion::Tuple(elements) => {
            let names: Vec<String> = (0..elements.len())
                .map(|index| format!("element{index}"))
                .collect();
            let rebuilt: Vec<String> = elements
                .iter()
                .zip(&names)
                .map(|(element, name)| convert(element, name, variant))
                .collect();
            format!(
                "{{ let ({}) = {value}; ({}) }}",
                names.join(", "),
                rebuilt.join(", ")
            )
        }
    }
}

/// Returns a conversion as the path a `.map(..)` takes.
fn path(conversion: &Conversion, variant: &str) -> String {
    match conversion {
        Conversion::Rewrap => String::from(variant),
        Conversion::Into => String::from("Into::into"),
        Conversion::Keep | Conversion::Map(_) | Conversion::Result(_, _) | Conversion::Tuple(_) => {
            unreachable!("the verdict maps only a rewrap or an `Into`")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_borrowed_as_a_hand_writes_them() {
        let shared = Place::of("inner", Binding::Shared, &[]);
        assert_eq!(shared.handed(Handover::Borrow), "inner");
        assert_eq!(shared.handed(Handover::Pin), "Pin::new(inner)");
        let exclusive = Place::of("inner", Binding::Exclusive, &[]);
        assert_eq!(exclusive.handed(Handover::BorrowMutably), "inner");
        assert_eq!(exclusive.handed(Handover::Borrow), "&*inner");
        let deref = Place::of("inner", Binding::Shared, &[Step::Deref]);
        assert_eq!(deref.handed(Handover::Borrow), "&**inner");
        let owned = Place::of("inner", Binding::Owned, &[Step::Deref]);
        assert_eq!(owned.handed(Handover::Move), "*inner");
        assert_eq!(owned.handed(Handover::MoveBox), "inner");
        let field = Place::of("inner", Binding::Shared, &[Step::Field("payload")]);
        assert_eq!(field.handed(Handover::Borrow), "&inner.payload");
        let field = Place::of("inner", Binding::Owned, &[Step::Field("1")]);
        assert_eq!(field.handed(Handover::Move), "inner.1");
        let method = Place::of("inner", Binding::Shared, &[Step::Method("lend")]);
        assert_eq!(method.handed(Handover::Borrow), "inner.lend()");
        assert_eq!(method.handed(Handover::Pin), "Pin::new(inner.lend())");
        let value = Place::of("inner", Binding::Shared, &[Step::Yield("copied")]);
        assert_eq!(value.handed(Handover::Borrow), "&inner.copied()");
        assert_eq!(value.handed(Handover::Move), "inner.copied()");
    }

    #[test]
    fn conversions_are_paths_under_a_map() {
        let tuple = Conversion::Tuple(vec![
            Conversion::Rewrap,
            Conversion::Map(Box::new(Conversion::Rewrap)),
        ]);
        assert_eq!(
            convert(&tuple, "call", "E::A"),
            "{ let (element0, element1) = call; (E::A(element0), element1.map(E::A)) }"
        );
        let result = Conversion::Result(Box::new(Conversion::Rewrap), Box::new(Conversion::Into));
        assert_eq!(
            convert(&result, "call", "E::A"),
            "call.map(E::A).map_err(Into::into)"
        );
    }
}
