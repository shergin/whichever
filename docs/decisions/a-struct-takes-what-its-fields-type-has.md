# A struct takes what its field's type has

Status: accepted, 2026-11-17. Serves rules 1 and 3 of the
[vision](../vision.md), the output is the hand's and what earns a shape.
Reopen if a surveyed project forwards a method without a receiver that
takes a `Self` argument, or forwards one of these through an enum whose
variants all hold one type.

## Context

A struct has one target, and its declaration names the target's type. It
already took associated types and consts from it, `<Square as
Shape>::Unit`, and forwarded any return-position `impl Trait`. A trait
method without a receiver and a generic associated type were refused all
the same, with the reason an enum has, "no field to forward to", and a
provided method without a receiver was left to the trait's default, which
hid the field's override of it: the very thing a struct forwards provided
methods to avoid.

rsynko 0.1.2 writes two functions without a receiver by hand for that
reason, sux 0.15.4 declares a generic associated type on a trait its
wrappers forward, and dash-num keeps `delegate!` for two inherent
constructors, `from_u64` and `from_u128`, beside a struct it otherwise
forwards.

## Decision

Where a struct's target has a type the struct names, its one field or the
marked one, reached directly or by `deref`:

- A trait method without a receiver is the type's:
  `<Target as Trait>::name(arguments..)`, the provided ones included, with
  the result converted as a method's is, so `-> Self` is built around it
  where the struct has no other field.
- A generic associated type is the type's, with its own parameters handed
  on and the trait's `where` clause: `type Side<'a> = <Target as
  Trait>::Side<'a> where Self: 'a;`.
- An inherent method without a receiver, marked in a block, is
  `<Target>::name(arguments..)`, which finds an inherent function before a
  trait's, as the method call of the other inherent arms does; `call`
  names it and `with` wraps it as for any other.

What the type cannot be asked is left as it was: a method that takes
`Self` arguments, a field that holds a trait object, and a field reached
through a field or a method, whose type the struct does not say. A trait's
provided method is then left to its default, and a required or an inherent
one is one sentence.

An enum is unchanged: it has no variant to ask before it has a value, and
nothing can check that its variants agree on a generic associated type for
every argument.

## Evidence

- Issue #34, with what a hand writes for each kind of field.
- `tests/behaviour/`, `newtype_delegation` and `inherent_block_on_a_struct`;
  `tests/expand/newtype.rs`.
- `tests/ui/` pins the sentences that remain: a `Self` argument, a routed
  field, a trait object, and `-> Self` beside other fields.

## Not chosen

- The same for required methods and generic associated types only,
  leaving a provided method without a receiver to its default: no change
  for existing code, and the hidden override stays.
- `Self` arguments unwrapped to the field, as beside a receiver: no
  surveyed project has one, and without a receiver there is no `match` to
  bind them in.
- A generic associated type from an enum's first variant: the agreement
  check would have to hold for every argument, which is a higher-ranked
  bound the check cannot write.
