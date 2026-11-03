# Receivers that would allocate are rejected

Status: accepted, 2026-11-02 (0.4.0). Serves rule 1 of the
[vision](../vision.md), the output is the hand's. Reopen for `self: Rc<Self>` and `self: Arc<Self>` when a surveyed project
writes that forwarding by hand and would accept the refcount; it would then
go behind an explicit option on the enum, never the default. Reopen for
`!Unpin` payloads when a project with such payloads and a dispatchable
poll-like trait asks; an unsafe pin projection would be an explicit opt-in.

## Context

0.1.0 rejected every explicit `self: Type` receiver as a scope cut, not a
consequence of the type system. 0.2.1 accepted the explicit synonyms of the
shorthand forms. The question for 0.4.0 was which of the remaining
receivers have a spelling a hand would write without hiding a cost. The
planning note's receivers section names no surveyed project; the receivers
predate the rule that a shape enters on a motivating case, and this record
is the evidence that they were judged on cost alone.

## Decision

- `self: Box<Self>` dispatches on a `deref` variant holding a `Box`: the enum
  is moved out of its box and the payload's own box is handed over. On a
  plain variant it is an error, since boxing the payload again would
  allocate on every call.
- `self: Pin<&Self>` and `self: Pin<&mut Self>` dispatch when every payload
  is `Unpin`: the enum is unpinned, matched, and the payload pinned again,
  which is free.
- `self: Rc<Self>`, `self: Arc<Self>` and `self: Pin<Box<Self>>` are
  rejected, with a sentence naming the supported receivers.

## Evidence

- `match *self { Kind::A(inner) => consume(Box::new(inner)) }` compiles and
  is the only way to serve `Box<Self>` on a plain payload; it allocates per
  call with no sign at the call site. The crate exists to avoid that.
- Cloning the inner `Rc` costs one refcount increment and the callee sees an
  exact count with the outer pointer dropped first; it is still a conversion
  the signature does not ask for.
- `Pin::new` on an `Unpin` payload is a no-op; the generated code is the
  hand's.
- `tests/ui/rc_self_receiver.rs` and `tests/ui/box_self_plain_variant.rs`
  pin the sentences; `tests/expand/receivers.rs` holds the accepted forms.

## Not chosen

- Boxing a plain payload again: rejected, with the error pointing at the
  `deref` variant shape that makes it free.
- Cloning the `Rc`: deferred behind an option, not rejected, pending a case.
- Unsafe structural pin projection for `!Unpin` payloads, as pin-project
  generates: sound only under obligations the macro would have to enforce
  (no `Drop` that moves a payload, no manual `Unpin`, no other `&mut
  Payload` from a pinned enum). Not until asked, and then opt-in.
