# A field's trait may be a partial block's

Status: accepted, 2026-11-16. Completes
[A struct's field may name its traits](a-structs-field-may-name-its-traits.md)
and supersedes one clause of it, the sentence for a trait named on a field
and not on the struct. Serves rule 3 of the [vision](../vision.md), what
earns a shape: two things that exist compose. Reopen if a surveyed project
loses a field's mark to a spelling the sentence below does not catch, or if
rustc lets one expansion see another, which would bring the old sentence
back.

## Context

A trait's impl has one home. It is the subject's attribute, or a partial
impl block when the impl needs a header the attribute cannot write, and
then the subject does not name the trait: the block is the impl. A struct
forwards a trait to the field that names it. The two did not compose: the
struct's attribute had to name every trait a field names, and a block of a
trait the attribute names is the second impl.

rsynko 0.1.2's `LawEnv` sends 63 traits to 7 fields, and two of the
catalog's need `where Catalog: MediaSorts<Extractor = <Syntax as
MediaSorts>::Extractor>`, a constraint between two fields' types. Every
spelling was refused. The field naming the trait was told to add it on the
struct; the struct naming it was told to drop it for the block; a method of
the block naming the field was told the struct had no such field. What
worked was making `catalog` the bare field, which a struct has one of, and
naming on `manager` the 27 traits that mark had carried.

## Decision

A field names the traits it is the target of, whichever home implements
them. A trait the struct's attributes do not name is a partial block's.
The struct's attribute still names every trait whose impl it writes, so
the impl's home is still the item that gets it. A field names a trait as
its home spells it, the attribute or the block's header, generic arguments
included, since a macro cannot resolve names.

The struct does not see its blocks, so a field's mark that no home claims
is no longer reported. It forwards nothing, and the trait's first use says
the impl is missing, as it does for a hand that wrote none. What can
still be told is told, at either home: a trait whose home finds no field
of its spelling, beside a field that names it with more or fewer path
segments, or with or without its generic arguments, is one sentence, since
the home would forward to the bare field and the mark would be dead. Two
instantiations, `Encoder<Json>` and `Encoder<Xml>`, and two paths that part
ways, `a::Named` and `b::Named`, are two traits, as they are everywhere
else.

A method of a partial block that names another field than the trait's is
told where the trait goes, and that naming the trait on the field sends it
there.

## Evidence

- rsynko 0.1.2, `rsynko-memory/src/laws.rs`, in the re-survey of
  `ambassador`'s dependents of 2026-11-16; issue #16, with the five
  spellings and what each was told.
- `tests/behaviour/`, `per_trait_fields`; `tests/ui/`, the cases for a
  trait spelled two ways on the struct and on a block, and for a block's
  method naming another field.

## Not chosen

- An option on the block naming its field, `field = catalog` or a route:
  one more option, saying on the block what the field already says, and
  the `target = field` that the completed record did not choose, moved to
  another attribute.
- Leaving it to the bare field: a struct has one bare mark, so one field
  can take a block's trait that way and a second cannot, and the three
  sentences still sent the user in a circle.
- Proving at the struct that every field's trait has an impl. A block is
  another expansion, and nothing is kept between expansions. An assertion
  that the struct implements the trait cannot be written for a header with
  bounds, which is the case a block is for.
- Resolving the field's path so that a misspelt trait is rustc's error. A
  block may live in a module where the trait is in scope and the struct's
  module is not one, and the path would have to resolve in both.
- Comparing traits by their last segment alone: two traits of one name
  from two modules go to two fields, which the completed record keeps.
