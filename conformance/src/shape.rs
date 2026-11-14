//! The shape check: the expansion performs the operations the hand
//! performs, arm by arm and in the same order, and nothing else.
//!
//! Token equality with the hand would pin spelling that carries no
//! meaning: `&**inner` against `inner`, `Self::A` against `Enum::A`,
//! `::core::convert::Into::into(x)` against `x.into()`. The type checker
//! and the law already hold the borrows and the paths to account. What
//! neither sees is an operation that type-checks and behaves and that a
//! careful hand would not write: a clone, a boxing, a closure, a
//! conversion nobody asked for, an inherent method called where the
//! trait's was meant. So a method body is read as its operations, in
//! evaluation order, with spelling erased, and compared with the hand's.
//!
//! # Operations
//!
//! A method body is one `match`, possibly inside `async move { .. }`,
//! `Box::pin(async move { .. })`, `unsafe { .. }`, a plain block, or any
//! call of one argument that holds the rest. Its shape is the operations
//! around the `match`, then per arm the variant the pattern names and the
//! operations of the arm's value. The operations around the `match` are
//! its wrappers, outermost first, followed by the operations of its
//! scrutinee: `fn Pin::get_mut` for `match ::core::pin::Pin::get_mut(self)`.
//! A body that is no `match`, such as a struct's one arm written as the
//! bare expression, is read whole into the bare operations, and has no
//! operations around it and no arms.
//!
//! An operation is a short canonical string. Within an arm, operations
//! come in evaluation order: the operations of a call's arguments before
//! the call, of a method's receiver and then its arguments before the
//! method, of an awaited or projected value before `.await` or the field.
//! A block that wraps code, `unsafe`, `async` or a closure, comes before
//! the operations it wraps.
//!
//! | Source | Operation |
//! | --- | --- |
//! | `<X as Trait>::method(..)`, for any `X`, `_` included, and the same as a path handed to a call | `call Trait::method`, by the last segment of the trait's path |
//! | `receiver.name(..)` | `method name` |
//! | `path::to::function(..)`, and the same as a path of two or more segments handed to a call | `fn function`, by the last segment, or the last two when the one before it is a type: `fn Pin::new`, `fn Box::pin` |
//! | `Enum::Variant(..)`, `Self::Variant { .. }`, and a path of two or more segments ending in a capitalised name, read as a value | `construct Variant` |
//! | `Struct(..)`, `Self(..)`, `Struct { .. }`, `Self { .. }`: one capitalised segment called or built | `construct Self` |
//! | `Into::into(x)`, `<X as Into<T>>::into(x)`, `x.into()`, and `Into::into` as a path handed to a call | `into` |
//! | `.map(..)`, `.map_err(..)` | `map`, `map_err`, after the operations of what they are handed |
//! | `Some(..)`, `Ok(..)`, `Err(..)`, under any path, and the same as a path handed to a call | `some`, `ok`, `err` |
//! | `.await` | `await` |
//! | `expr?` | `try` |
//! | `unsafe { .. }` | `unsafe` |
//! | `async { .. }`, `async move { .. }` | `async` |
//! | `expr.field`, `expr.0` | `field name` |
//! | a closure | `closure`, followed by its body's operations; also a violation |
//! | a macro invocation | `macro name`, by the last segment of its path |
//! | a nested `match` | the scrutinee's operations, `match`, then per arm `arm Variant` and its operations, then `end match` |
//! | a binary or unary operator other than `*` | `operator +`, `operator !` and the like, after its operands |
//! | a guard `if cond` on an arm | the condition's operations, then `guard`, before the arm's value |
//!
//! Any other expression, a cast, an index or an `if` say, is named by a
//! word of its own after its operands, so that it is never silently equal
//! to its absence.
//!
//! References, dereferences, parentheses, invisible groups, blocks, `let`
//! bindings and the patterns they take apart, literals, and plain paths
//! read as values (a binding, `self`, `None`, a unit value or a constant
//! of one segment) leave no operation. The self type of a qualified path
//! and every generic argument, turbofish included, are erased. Leading
//! `::core::..` and `::std::..` segments make no difference, since a path
//! is read by its last one or two segments.
//!
//! A pattern names its variant by the last segment of its path: `A` for
//! `Enum::A(inner)`, `Self::A { field: inner, .. }` and `Enum::A(ref mut
//! inner, ..)`; `A A` for the tuple `(Enum::A(x), Enum::A(y))`; `A | B`
//! for an or-pattern; `_` for a wildcard and for a binding such as
//! `__other`. A path of one segment, `Wrapped(inner, ..)`,
//! `Self { field: inner, .. }` or `Labelled { field: inner, .. }`, names a
//! struct, whose name the body alone cannot tell from `Self`, so the
//! pattern names `Self`. A reference, parentheses, a type ascription, a
//! binding `x @ ..` and a guard are seen through; `ref` and `ref mut` and
//! binding names leave no trace.
//!
//! # Comparison
//!
//! Two shapes agree when their operations around the `match`, their bare
//! operations, and their arms agree. Arms are paired by variant and by
//! occurrence, so a missing or extra arm, a missing or extra fallback arm
//! `_`, and arms in a different order are each reported on the arm they
//! concern.
//!
//! # Violations
//!
//! Independent of the hand, a generated body may not hold a closure, may
//! not name a path that starts with `std` or `alloc`, and may name a path
//! with a leading `::` only when it starts `::core::`. The paths of the
//! user's own argument expressions cannot be told apart from the macro's,
//! so the rule holds for every path in the body, types and macro paths
//! included.

use syn::visit::Visit;
use syn::{
    BinOp, Block, Expr, ExprMatch, File, ImplItem, ImplItemFn, Item, ItemImpl, Member, Pat, Path,
    QSelf, Stmt, Type, UnOp,
};

/// The operations of one arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmShape {
    /// The variant the pattern names, as the module docs spell it.
    pub variant: String,
    /// The operations of the arm's value, in evaluation order.
    pub operations: Vec<String>,
}

/// The shape of one method body.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MethodShape {
    /// The operations around the `match`, outermost first.
    pub around: Vec<String>,
    /// The arms, in the order written; empty when the body is no `match`.
    pub arms: Vec<ArmShape>,
    /// The operations of a body that is no `match`: a struct's one arm is
    /// often the bare call.
    pub bare: Vec<String>,
    /// What the body holds that no generated body may.
    pub violations: Vec<String>,
}

/// Reads the shape of a method body.
pub fn method_shape(body: &Block) -> MethodShape {
    let violations = violations(body);
    let mut around = Vec::new();
    let Some(found) = single_expression(body).and_then(|expr| find_match(expr, &mut around)) else {
        let mut bare = Vec::new();
        block_operations(body, &mut bare);
        return MethodShape {
            around: Vec::new(),
            arms: Vec::new(),
            bare,
            violations,
        };
    };
    operations(&found.expr, &mut around);
    let arms = found
        .arms
        .iter()
        .map(|arm| {
            let mut operations = Vec::new();
            arm_operations(arm, &mut operations);
            ArmShape {
                variant: pattern_variant(&arm.pat),
                operations,
            }
        })
        .collect();
    MethodShape {
        around,
        arms,
        bare: Vec::new(),
        violations,
    }
}

/// Returns how the subject's method differs from the hand's, one line per
/// difference, each naming the arm; empty when the two perform the same
/// operations. The subject's own violations are reported first.
pub fn differences(subject: &MethodShape, hand: &MethodShape) -> Vec<String> {
    let mut lines: Vec<String> = subject
        .violations
        .iter()
        .map(|violation| format!("violation: {violation}"))
        .collect();
    if subject.around != hand.around {
        lines.push(format!(
            "around: the subject {}, the hand {}",
            list(&subject.around),
            list(&hand.around)
        ));
    }
    if subject.bare != hand.bare {
        lines.push(format!(
            "bare: the subject {}, the hand {}",
            list(&subject.bare),
            list(&hand.bare)
        ));
    }
    arm_differences(&subject.arms, &hand.arms, &mut lines);
    lines
}

/// Returns the items of the module at `path` below the file's root, or
/// `None` when a segment names no inline module.
pub fn module<'a>(file: &'a File, path: &[&str]) -> Option<&'a [Item]> {
    let mut items: &'a [Item] = &file.items;
    for segment in path {
        items = items.iter().find_map(|item| match item {
            Item::Mod(module) if module.ident == segment => {
                module.content.as_ref().map(|(_, items)| items.as_slice())
            }
            _ => None,
        })?;
    }
    Some(items)
}

/// Returns the impl blocks among `items` whose self type's last segment is
/// `self_type`: with `trait_name`, the impls of the trait whose path ends
/// in it; without, the inherent impls.
pub fn impls<'a>(
    items: &'a [Item],
    self_type: &str,
    trait_name: Option<&str>,
) -> Vec<&'a ItemImpl> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(block) => Some(block),
            _ => None,
        })
        .filter(|block| type_last_segment(&block.self_ty).is_some_and(|name| name == self_type))
        .filter(|block| {
            let implemented = block
                .trait_
                .as_ref()
                .and_then(|(path, _)| path.segments.last())
                .map(|segment| segment.ident.to_string());
            implemented.as_deref() == trait_name
        })
        .collect()
}

/// Returns the method named `name` of the first of `blocks` that has one.
pub fn method<'a>(blocks: &[&'a ItemImpl], name: &str) -> Option<&'a ImplItemFn> {
    blocks.iter().find_map(|block| {
        block.items.iter().find_map(|item| match item {
            ImplItem::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
    })
}

/// Returns the last segment of a type's path, seen through parentheses and
/// groups, or `None` for a type that is no path.
fn type_last_segment(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        Type::Paren(inner) => type_last_segment(&inner.elem),
        Type::Group(inner) => type_last_segment(&inner.elem),
        _ => None,
    }
}

/// Returns the expression a block consists of, when it holds exactly one
/// expression and nothing else.
fn single_expression(block: &Block) -> Option<&Expr> {
    match block.stmts.as_slice() {
        [Stmt::Expr(expr, None)] => Some(expr),
        _ => None,
    }
}

/// Descends through the wrappers a generated body may put around its
/// `match`, recording each, and returns the `match`; `None` when the
/// expression reaches none.
fn find_match<'a>(expr: &'a Expr, around: &mut Vec<String>) -> Option<&'a ExprMatch> {
    match expr {
        Expr::Match(found) => Some(found),
        Expr::Group(inner) => find_match(&inner.expr, around),
        Expr::Paren(inner) => find_match(&inner.expr, around),
        Expr::Block(inner) => find_match(single_expression(&inner.block)?, around),
        Expr::Async(inner) => {
            around.push("async".to_owned());
            find_match(single_expression(&inner.block)?, around)
        }
        Expr::Unsafe(inner) => {
            around.push("unsafe".to_owned());
            find_match(single_expression(&inner.block)?, around)
        }
        Expr::Call(call) if call.args.len() == 1 => {
            let Expr::Path(function) = strip(&call.func) else {
                return None;
            };
            around.push(path_operation(
                function.qself.as_ref(),
                &function.path,
                true,
            )?);
            find_match(&call.args[0], around)
        }
        _ => None,
    }
}

/// Sees through the spelling of an expression that carries no meaning:
/// parentheses, invisible groups, references and dereferences.
fn strip(expr: &Expr) -> &Expr {
    match expr {
        Expr::Group(inner) => strip(&inner.expr),
        Expr::Paren(inner) => strip(&inner.expr),
        Expr::Reference(inner) => strip(&inner.expr),
        Expr::Unary(inner) if matches!(inner.op, UnOp::Deref(_)) => strip(&inner.expr),
        _ => expr,
    }
}

/// Appends the operations of an arm: its guard, then its value.
fn arm_operations(arm: &syn::Arm, out: &mut Vec<String>) {
    if let Some(guard) = pattern_guard(&arm.pat) {
        operations(guard, out);
        out.push("guard".to_owned());
    }
    operations(&arm.body, out);
}

/// Returns the guard of an arm's pattern, which `syn` reads as part of it.
fn pattern_guard(pattern: &Pat) -> Option<&Expr> {
    match pattern {
        Pat::Guard(guarded) => Some(&guarded.guard),
        _ => None,
    }
}

/// Appends the operations of a block's statements, in order.
fn block_operations(block: &Block, out: &mut Vec<String>) {
    for statement in &block.stmts {
        match statement {
            Stmt::Local(local) => {
                let Some(init) = &local.init else {
                    continue;
                };
                operations(&init.expr, out);
                if let Some((_, diverge)) = &init.diverge {
                    out.push("else".to_owned());
                    operations(diverge, out);
                }
            }
            Stmt::Item(_) => out.push("item".to_owned()),
            Stmt::Expr(expr, _) => operations(expr, out),
            Stmt::Macro(statement) => out.push(macro_operation(&statement.mac.path)),
        }
    }
}

/// Appends the operations of an expression, in evaluation order.
fn operations(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Group(inner) => operations(&inner.expr, out),
        Expr::Paren(inner) => operations(&inner.expr, out),
        Expr::Reference(inner) => operations(&inner.expr, out),
        Expr::Unary(inner) => {
            operations(&inner.expr, out);
            match inner.op {
                UnOp::Deref(_) => {}
                UnOp::Not(_) => out.push("operator !".to_owned()),
                UnOp::Neg(_) => out.push("operator -".to_owned()),
                _ => out.push("operator".to_owned()),
            }
        }
        Expr::Block(inner) => block_operations(&inner.block, out),
        Expr::Async(inner) => {
            out.push("async".to_owned());
            block_operations(&inner.block, out);
        }
        Expr::Unsafe(inner) => {
            out.push("unsafe".to_owned());
            block_operations(&inner.block, out);
        }
        Expr::Const(inner) => {
            out.push("const".to_owned());
            block_operations(&inner.block, out);
        }
        Expr::TryBlock(inner) => {
            out.push("try block".to_owned());
            block_operations(&inner.block, out);
        }
        Expr::Closure(closure) => {
            out.push("closure".to_owned());
            operations(&closure.body, out);
        }
        Expr::Call(call) => {
            for argument in &call.args {
                operations(argument, out);
            }
            match strip(&call.func) {
                Expr::Path(function) => {
                    let operation = path_operation(function.qself.as_ref(), &function.path, true);
                    out.push(operation.unwrap_or_else(|| "fn".to_owned()));
                }
                function => {
                    operations(function, out);
                    out.push("call an expression".to_owned());
                }
            }
        }
        Expr::MethodCall(call) => {
            operations(&call.receiver, out);
            for argument in &call.args {
                operations(argument, out);
            }
            let name = call.method.to_string();
            let operation = match name.as_str() {
                "into" if call.args.is_empty() => "into".to_owned(),
                "map" | "map_err" => name,
                _ => format!("method {name}"),
            };
            out.push(operation);
        }
        Expr::Path(path) => {
            if let Some(operation) = path_operation(path.qself.as_ref(), &path.path, false) {
                out.push(operation);
            }
        }
        Expr::Struct(literal) => {
            for field in &literal.fields {
                operations(&field.expr, out);
            }
            if let Some(rest) = &literal.rest {
                operations(rest, out);
            }
            out.push(construct_operation(&literal.path));
        }
        Expr::Field(field) => {
            operations(&field.base, out);
            out.push(match &field.member {
                Member::Named(ident) => format!("field {ident}"),
                Member::Unnamed(index) => format!("field {}", index.index),
            });
        }
        Expr::Await(awaited) => {
            operations(&awaited.base, out);
            out.push("await".to_owned());
        }
        Expr::Try(tried) => {
            operations(&tried.expr, out);
            out.push("try".to_owned());
        }
        Expr::Match(nested) => {
            operations(&nested.expr, out);
            out.push("match".to_owned());
            for arm in &nested.arms {
                out.push(format!("arm {}", pattern_variant(&arm.pat)));
                arm_operations(arm, out);
            }
            out.push("end match".to_owned());
        }
        Expr::Macro(invocation) => out.push(macro_operation(&invocation.mac.path)),
        Expr::Tuple(tuple) => {
            for element in &tuple.elems {
                operations(element, out);
            }
        }
        Expr::Array(array) => {
            for element in &array.elems {
                operations(element, out);
            }
            out.push("array".to_owned());
        }
        Expr::Binary(binary) => {
            operations(&binary.left, out);
            operations(&binary.right, out);
            out.push(format!("operator {}", binary_operator(&binary.op)));
        }
        Expr::Cast(cast) => {
            operations(&cast.expr, out);
            out.push("cast".to_owned());
        }
        Expr::Index(index) => {
            operations(&index.expr, out);
            operations(&index.index, out);
            out.push("index".to_owned());
        }
        Expr::Range(range) => {
            for bound in [&range.start, &range.end].into_iter().flatten() {
                operations(bound, out);
            }
            out.push("range".to_owned());
        }
        Expr::If(conditional) => {
            operations(&conditional.cond, out);
            out.push("if".to_owned());
            block_operations(&conditional.then_branch, out);
            if let Some((_, otherwise)) = &conditional.else_branch {
                out.push("else".to_owned());
                operations(otherwise, out);
            }
            out.push("end if".to_owned());
        }
        Expr::Let(binding) => {
            operations(&binding.expr, out);
            out.push("let".to_owned());
        }
        Expr::Return(returned) => {
            if let Some(value) = &returned.expr {
                operations(value, out);
            }
            out.push("return".to_owned());
        }
        Expr::RawAddr(address) => {
            operations(&address.expr, out);
            out.push("raw address".to_owned());
        }
        Expr::Lit(_) | Expr::Infer(_) => {}
        Expr::Assign(_) => out.push("assign".to_owned()),
        Expr::Break(_) => out.push("break".to_owned()),
        Expr::Continue(_) => out.push("continue".to_owned()),
        Expr::ForLoop(_) | Expr::Loop(_) | Expr::While(_) => out.push("loop".to_owned()),
        Expr::Repeat(_) => out.push("repeat".to_owned()),
        Expr::Yield(_) => out.push("yield".to_owned()),
        _ => out.push("other expression".to_owned()),
    }
}

/// Returns the operation a path names, as the function of a call when
/// `called` and as a value otherwise; `None` when it names none.
fn path_operation(qself: Option<&QSelf>, path: &Path, called: bool) -> Option<String> {
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let last = segments.last()?;
    if let Some(qself) = qself {
        if qself.position == 0 {
            return Some(format!("fn {last}"));
        }
        let trait_name = &segments[qself.position - 1];
        if trait_name == "Into" && last == "into" {
            return Some("into".to_owned());
        }
        return Some(format!("call {trait_name}::{last}"));
    }
    match last.as_str() {
        "Some" => return Some("some".to_owned()),
        "Ok" => return Some("ok".to_owned()),
        "Err" => return Some("err".to_owned()),
        "None" => return None,
        _ => {}
    }
    let before = segments.len().checked_sub(2).map(|index| &segments[index]);
    if before.is_some_and(|before| before == "Into") && last == "into" {
        return Some("into".to_owned());
    }
    let Some(before) = before else {
        if !called {
            return None;
        }
        if capitalised(last) {
            return Some("construct Self".to_owned());
        }
        return Some(format!("fn {last}"));
    };
    if capitalised(last) {
        return Some(format!("construct {last}"));
    }
    if capitalised(before) {
        return Some(format!("fn {before}::{last}"));
    }
    Some(format!("fn {last}"))
}

/// Returns the operation of a struct literal or a struct variant built by
/// its path.
fn construct_operation(path: &Path) -> String {
    match path.segments.len() {
        0 | 1 => "construct Self".to_owned(),
        _ => format!("construct {}", path.segments[path.segments.len() - 1].ident),
    }
}

/// Returns the operation of a macro invocation.
fn macro_operation(path: &Path) -> String {
    let name = path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default();
    format!("macro {name}")
}

/// Returns whether a name starts with a capital letter, as a type's, a
/// variant's and a constant's do.
fn capitalised(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

/// Returns the spelling of a binary operator.
fn binary_operator(operator: &BinOp) -> &'static str {
    match operator {
        BinOp::Add(_) => "+",
        BinOp::Sub(_) => "-",
        BinOp::Mul(_) => "*",
        BinOp::Div(_) => "/",
        BinOp::Rem(_) => "%",
        BinOp::And(_) => "&&",
        BinOp::Or(_) => "||",
        BinOp::BitXor(_) => "^",
        BinOp::BitAnd(_) => "&",
        BinOp::BitOr(_) => "|",
        BinOp::Shl(_) => "<<",
        BinOp::Shr(_) => ">>",
        BinOp::Eq(_) => "==",
        BinOp::Lt(_) => "<",
        BinOp::Le(_) => "<=",
        BinOp::Ne(_) => "!=",
        BinOp::Ge(_) => ">=",
        BinOp::Gt(_) => ">",
        BinOp::AddAssign(_) => "+=",
        BinOp::SubAssign(_) => "-=",
        BinOp::MulAssign(_) => "*=",
        BinOp::DivAssign(_) => "/=",
        BinOp::RemAssign(_) => "%=",
        BinOp::BitXorAssign(_) => "^=",
        BinOp::BitAndAssign(_) => "&=",
        BinOp::BitOrAssign(_) => "|=",
        BinOp::ShlAssign(_) => "<<=",
        BinOp::ShrAssign(_) => ">>=",
        _ => "?",
    }
}

/// Returns the variant a pattern names, as the module docs spell it.
fn pattern_variant(pattern: &Pat) -> String {
    match pattern {
        Pat::TupleStruct(tuple) => path_variant(&tuple.path),
        Pat::Struct(fields) => path_variant(&fields.path),
        Pat::Path(path) => path_variant(&path.path),
        Pat::Tuple(tuple) => tuple
            .elems
            .iter()
            .map(pattern_variant)
            .collect::<Vec<_>>()
            .join(" "),
        Pat::Or(cases) => cases
            .cases
            .iter()
            .map(pattern_variant)
            .collect::<Vec<_>>()
            .join(" | "),
        Pat::Wild(_) => "_".to_owned(),
        Pat::Ident(binding) => {
            if let Some((_, inner)) = &binding.subpat {
                return pattern_variant(inner);
            }
            let name = binding.ident.to_string();
            if capitalised(&name) {
                return name;
            }
            "_".to_owned()
        }
        Pat::Reference(inner) => pattern_variant(&inner.pat),
        Pat::Paren(inner) => pattern_variant(&inner.pat),
        Pat::Type(inner) => pattern_variant(&inner.pat),
        Pat::Guard(inner) => pattern_variant(&inner.pat),
        Pat::Lit(_) => "literal".to_owned(),
        Pat::Range(_) => "range".to_owned(),
        Pat::Rest(_) => "..".to_owned(),
        Pat::Slice(_) => "slice".to_owned(),
        Pat::Macro(_) => "macro".to_owned(),
        _ => "pattern".to_owned(),
    }
}

/// Returns the variant a pattern's path names: its last segment, or
/// `Self` for a path of one segment, which names a struct.
fn path_variant(path: &Path) -> String {
    if path.segments.len() < 2 {
        return "Self".to_owned();
    }
    path.segments[path.segments.len() - 1].ident.to_string()
}

/// Appends the differences between two lists of arms, paired by variant
/// and occurrence: the subject's arms in order, then the arms only the
/// hand has.
fn arm_differences(subject: &[ArmShape], hand: &[ArmShape], lines: &mut Vec<String>) {
    let subject_keys = arm_keys(subject);
    let hand_keys = arm_keys(hand);
    let common_in_subject: Vec<&(String, usize)> = subject_keys
        .iter()
        .filter(|key| hand_keys.contains(key))
        .collect();
    let common_in_hand: Vec<&(String, usize)> = hand_keys
        .iter()
        .filter(|key| subject_keys.contains(key))
        .collect();
    let reordered = common_in_subject != common_in_hand;
    for (index, (arm, key)) in subject.iter().zip(&subject_keys).enumerate() {
        let Some(hand_index) = hand_keys.iter().position(|other| other == key) else {
            lines.push(format!(
                "arm {}: only in the subject, {}",
                arm.variant,
                list(&arm.operations)
            ));
            continue;
        };
        let hand_arm = &hand[hand_index];
        let common_index = common_in_subject.iter().position(|other| *other == key);
        let hand_common_index = common_in_hand.iter().position(|other| *other == key);
        if reordered && common_index != hand_common_index {
            lines.push(format!(
                "arm {}: at position {} in the subject and {} in the hand, the subject {}, the hand {}",
                arm.variant,
                index + 1,
                hand_index + 1,
                list(&arm.operations),
                list(&hand_arm.operations)
            ));
        }
        if arm.operations != hand_arm.operations {
            lines.push(format!(
                "arm {}: the subject {}, the hand {}",
                arm.variant,
                list(&arm.operations),
                list(&hand_arm.operations)
            ));
        }
    }
    for (arm, key) in hand.iter().zip(&hand_keys) {
        if subject_keys.contains(key) {
            continue;
        }
        lines.push(format!(
            "arm {}: only in the hand, {}",
            arm.variant,
            list(&arm.operations)
        ));
    }
}

/// Returns each arm's key: its variant and how many arms before it name
/// the same.
fn arm_keys(arms: &[ArmShape]) -> Vec<(String, usize)> {
    arms.iter()
        .enumerate()
        .map(|(index, arm)| {
            let occurrence = arms[..index]
                .iter()
                .filter(|earlier| earlier.variant == arm.variant)
                .count();
            (arm.variant.clone(), occurrence)
        })
        .collect()
}

/// Returns a list of operations as one bracketed line.
fn list(operations: &[String]) -> String {
    format!("[{}]", operations.join(", "))
}

/// Returns what a body holds that no generated body may, each once, in
/// the order first met.
fn violations(body: &Block) -> Vec<String> {
    let mut finder = ViolationFinder::default();
    finder.visit_block(body);
    finder.found
}

/// The visitor that collects the violations of a body.
#[derive(Default)]
struct ViolationFinder {
    found: Vec<String>,
}

impl ViolationFinder {
    /// Records a violation unless it is already recorded.
    fn record(&mut self, violation: String) {
        if self.found.contains(&violation) {
            return;
        }
        self.found.push(violation);
    }
}

impl<'ast> Visit<'ast> for ViolationFinder {
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.record("a closure".to_owned());
        syn::visit::visit_expr_closure(self, closure);
    }

    fn visit_path(&mut self, path: &'ast Path) {
        let spelled = spell_path(path);
        if let Some(first) = path.segments.first() {
            let first = first.ident.to_string();
            if first == "std" || first == "alloc" {
                self.record(format!("the path `{spelled}` starts with `{first}`"));
            } else if path.leading_colon.is_some() && first != "core" {
                self.record(format!(
                    "the path `{spelled}` has a leading `::` and does not start with `::core`"
                ));
            }
        }
        syn::visit::visit_path(self, path);
    }
}

/// Returns a path's segments joined by `::`, without generic arguments.
fn spell_path(path: &Path) -> String {
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let leading = if path.leading_colon.is_some() {
        "::"
    } else {
        ""
    };
    format!("{leading}{}", segments.join("::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the shape of a method body written as a block.
    fn shape(body: &str) -> MethodShape {
        let block: Block = syn::parse_str(body).expect("the body parses");
        method_shape(&block)
    }

    /// Returns the differences between two bodies.
    fn compare(subject: &str, hand: &str) -> Vec<String> {
        differences(&shape(subject), &shape(hand))
    }

    /// Asserts that two bodies perform the same operations.
    fn assert_same(subject: &str, hand: &str) {
        let lines = compare(subject, hand);
        assert!(lines.is_empty(), "unexpected differences: {lines:#?}");
    }

    /// Asserts that two bodies differ, and that some difference names
    /// `place`, an arm as `arm A` or `around` or `bare`.
    fn assert_differs(subject: &str, hand: &str, place: &str) {
        let lines = compare(subject, hand);
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with(&format!("{place}:"))),
            "expected a difference on {place}: {lines:#?}"
        );
    }

    /// Returns the operations of the first arm of a body that names
    /// `variant`.
    fn arm(body: &str, variant: &str) -> Vec<String> {
        shape(body)
            .arms
            .into_iter()
            .find(|arm| arm.variant == variant)
            .expect("the arm exists")
            .operations
    }

    /// Returns owned strings for a list of operations.
    fn strings(operations: &[&str]) -> Vec<String> {
        operations
            .iter()
            .map(|operation| (*operation).to_owned())
            .collect()
    }

    /// Reads the shape of a method.
    fn shape_of(function: &ImplItemFn) -> MethodShape {
        method_shape(&function.block)
    }

    #[test]
    fn plain() {
        let subject = r#"{
            match self {
                Animal::Cat(__inner) => <Cat as Describe>::name(__inner),
                Animal::Dog(__inner) => <Dog as Describe>::name(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Cat(inner) => <Cat as Describe>::name(inner),
                Animal::Dog(dog) => { <Dog as crate::Describe>::name(&dog) }
            }
        }"#;
        assert_same(subject, hand);
        let read = shape(subject);
        assert!(read.around.is_empty());
        assert!(read.bare.is_empty());
        assert!(read.violations.is_empty());
        assert_eq!(read.arms.len(), 2);
        assert_eq!(read.arms[0].variant, "Cat");
        assert_eq!(read.arms[0].operations, strings(&["call Describe::name"]));
    }

    #[test]
    fn deref_through_a_box_and_a_reference() {
        let subject = r#"{
            match self {
                Shape::Boxed(__inner) => {
                    <<Box<Square> as ::core::ops::Deref>::Target as Area>::area(&**__inner)
                }
                Shape::Borrowed(__inner) => {
                    <<&'a Square as ::core::ops::Deref>::Target as Area>::area(&**__inner)
                }
                Shape::Plain(__inner) => <Square as Area>::area(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Shape::Boxed(inner) => <Square as Area>::area(inner),
                Shape::Borrowed(inner) => <_ as Area>::area(*inner),
                Shape::Plain(inner) => <Square as Area>::area(inner),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(arm(subject, "Boxed"), strings(&["call Area::area"]));
    }

    #[test]
    fn routes_through_a_field_and_a_tuple_index() {
        let subject = r#"{
            match self {
                Any::Module(__inner) => <_ as Area>::grow(&mut __inner.base, by),
                Any::Pair(__inner) => <_ as Area>::grow(&mut __inner.1, by),
                Any::Tagged(__inner, ..) => <Square as Area>::grow(__inner, by),
                Any::Row { cell: __inner, .. } => <Square as Area>::grow(__inner, by),
                Any::Plain(__inner) => <Square as Area>::grow(__inner, by),
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Module(account) => <Square as Area>::grow(&mut account.base, by),
                Self::Pair(pair) => <Square as Area>::grow(&mut pair.1, by),
                Self::Tagged(square, _) => <Square as Area>::grow(square, by),
                Self::Row { cell, .. } => <Square as Area>::grow(cell, by),
                Self::Plain(square) => <Square as Area>::grow(square, by),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(
            arm(subject, "Module"),
            strings(&["field base", "call Area::grow"])
        );
        assert_eq!(
            arm(subject, "Pair"),
            strings(&["field 1", "call Area::grow"])
        );
        let subject = r#"{
            match self {
                Self::Module(__inner) => __inner.base.0,
                Self::Tagged(__inner, ..) => __inner.0,
                _ => 0,
            }
        }"#;
        let hand = r#"{
            match self {
                Any::Module(account) => account.base.0,
                Any::Tagged(square, _) => square.0,
                _ => 0,
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(arm(subject, "Module"), strings(&["field base", "field 0"]));
    }

    #[test]
    fn a_marked_field_read_by_value_and_by_reference() {
        let subject = r#"{
            match self {
                Self::Dictionary(__inner) => __inner.handle,
                Self::Tagged(__inner, ..) => *__inner,
                Self::Unknown { handle: __inner, .. } => *__inner,
            }
        }"#;
        let hand = r#"{
            match self {
                Object::Dictionary(dictionary) => dictionary.handle,
                Object::Tagged(handle, _) => *handle,
                Object::Unknown { handle, .. } => *handle,
            }
        }"#;
        assert_same(subject, hand);
        let subject = r#"{
            match self {
                Self::Dictionary(__inner) => Some(&__inner.handle),
                Self::Tagged(__inner, ..) => Some(__inner),
                Self::Unknown { handle: __inner, .. } => Some(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Object::Dictionary(dictionary) => Some(&dictionary.handle),
                Object::Tagged(handle, _) => Some(handle),
                Object::Unknown { handle, .. } => Some(handle),
            }
        }"#;
        assert_same(subject, hand);
        let subject = r#"{
            match self {
                Self::Tagged(__inner, ..) => raw(__inner),
                Self::Unknown { handle: __inner, .. } => raw(__inner),
                _ => 0,
            }
        }"#;
        let hand = r#"{
            match self {
                Object::Tagged(handle, ..) => raw(handle),
                Object::Unknown { handle, .. } => raw(handle),
                _ => 0,
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(arm(subject, "Tagged"), strings(&["fn raw"]));
        assert_eq!(arm(subject, "Unknown"), strings(&["fn raw"]));
    }

    #[test]
    fn a_struct_pattern_of_one_segment_names_self() {
        let subject = r#"{
            match self {
                Self { handle: __inner, .. } => *__inner,
            }
        }"#;
        let hand = r#"{
            match self {
                Slot { handle, .. } => *handle,
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(shape(subject).arms[0].variant, "Self");
    }

    #[test]
    fn pinned_and_boxed_receivers() {
        let subject = r#"{
            match ::core::pin::Pin::get_mut(self) {
                AnySource::Boxed(__inner) => {
                    <<Box<
                        Ready,
                    > as ::core::ops::Deref>::Target as Source>::poll_next(
                        ::core::pin::Pin::new(&mut **__inner),
                        cx,
                    )
                }
            }
        }"#;
        let hand = r#"{
            match Pin::get_mut(self) {
                AnySource::Boxed(inner) => {
                    <Ready as Source>::poll_next(Pin::new(inner), cx)
                }
            }
        }"#;
        assert_same(subject, hand);
        let read = shape(subject);
        assert_eq!(read.around, strings(&["fn Pin::get_mut"]));
        assert_eq!(
            read.arms[0].operations,
            strings(&["fn Pin::new", "call Source::poll_next"])
        );

        let subject = r#"{
            match ::core::pin::Pin::get_ref(self) {
                AnySource::Boxed(__inner) => {
                    <<Box<Ready> as ::core::ops::Deref>::Target as Source>::peek(
                        ::core::pin::Pin::new(&**__inner),
                    )
                }
            }
        }"#;
        let hand = r#"{
            match Pin::get_ref(self) {
                Self::Boxed(inner) => <Ready as Source>::peek(Pin::new(inner)),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(shape(subject).around, strings(&["fn Pin::get_ref"]));

        let subject = r#"{
            match *self {
                AnySource::Boxed(__inner) => {
                    <<Box<Ready> as ::core::ops::Deref>::Target as Source>::finish(__inner)
                }
            }
        }"#;
        let hand = r#"{
            match *self {
                AnySource::Boxed(inner) => <Ready as Source>::finish(inner),
            }
        }"#;
        assert_same(subject, hand);
        assert!(shape(subject).around.is_empty());
    }

    #[test]
    fn a_pinned_receiver_with_every_variant_under_cfg() {
        let subject = r#"{
            match *::core::pin::Pin::get_mut(self) {
                AnyShape::Circle(ref mut __inner) => {
                    <Circle as Shape>::poll_area(::core::pin::Pin::new(__inner))
                }
            }
        }"#;
        let hand = r#"{
            match Pin::get_mut(self) {
                AnyShape::Circle(inner) => <Circle as Shape>::poll_area(Pin::new(inner)),
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn match_on_a_dereference_with_ref_bindings() {
        let subject = r#"{
            match *self {
                AnyShape::Circle(ref mut __inner) => <Circle as Shape>::grow(__inner, by),
            }
        }"#;
        let hand = r#"{
            match self {
                AnyShape::Circle(circle) => <Circle as Shape>::grow(circle, by),
            }
        }"#;
        assert_same(subject, hand);
        let subject = r#"{
            match *self {
                AnyShape::Circle(ref __inner) => <Circle as Shape>::area(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Circle(circle) => <Circle as Shape>::area(circle),
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn a_self_argument_with_the_tuple_scrutinee_and_a_fallback() {
        let subject = r#"{
            match (self, other) {
                (AnyShape::Circle(__inner), AnyShape::Circle(__same0)) => {
                    <Circle as Shape>::congruent(__inner, __same0)
                }
                (AnyShape::Square(__inner), AnyShape::Square(__same0)) => {
                    <Square as Shape>::congruent(__inner, __same0)
                }
                _ => false,
            }
        }"#;
        let hand = r#"{
            match (self, other) {
                (Self::Circle(left), Self::Circle(right)) => {
                    <Circle as Shape>::congruent(left, right)
                }
                (Self::Square(left), Self::Square(right)) => {
                    <Square as Shape>::congruent(left, right)
                }
                _ => false,
            }
        }"#;
        assert_same(subject, hand);
        let read = shape(subject);
        assert_eq!(read.arms[0].variant, "Circle Circle");
        assert_eq!(read.arms[2].variant, "_");
        assert!(read.arms[2].operations.is_empty());
    }

    #[test]
    fn a_self_return() {
        let subject = r#"{
            match self {
                AnyShape::Circle(__inner) => {
                    AnyShape::Circle(<Circle as Shape>::scaled(__inner, factor))
                }
                AnyShape::Square(__inner) => {
                    AnyShape::Square(<Square as Shape>::scaled(__inner, factor))
                }
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Circle(inner) => Self::Circle(<Circle as Shape>::scaled(inner, factor)),
                Self::Square(inner) => Self::Square(<Square as Shape>::scaled(inner, factor)),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(
            arm(subject, "Circle"),
            strings(&["call Shape::scaled", "construct Circle"])
        );
    }

    #[test]
    fn a_result_of_self_and_an_overridden_error() {
        let subject = r#"{
            match self {
                AnyParser::Text(__inner) => {
                    <Text as Parse>::reparse(__inner)
                        .map(AnyParser::Text)
                        .map_err(::core::convert::Into::into)
                }
                AnyParser::Binary(__inner) => {
                    <Binary as Parse>::reparse(__inner)
                        .map(AnyParser::Binary)
                        .map_err(::core::convert::Into::into)
                }
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Text(inner) => <Text as Parse>::reparse(inner).map(Self::Text).map_err(Into::into),
                Self::Binary(inner) => <Binary as Parse>::reparse(inner)
                    .map(Self::Binary)
                    .map_err(<BinaryError as Into<ParseError>>::into),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(
            arm(subject, "Text"),
            strings(&[
                "call Parse::reparse",
                "construct Text",
                "map",
                "into",
                "map_err"
            ])
        );
        let subject = r#"{
            match self {
                AnyParser::Text(__inner) => {
                    <Text as Parse>::error(__inner).map(::core::convert::Into::into)
                }
            }
        }"#;
        let hand = r#"{
            match self {
                AnyParser::Text(inner) => <Text as Parse>::error(inner).map(Into::into),
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn a_tuple_return() {
        let subject = r#"{
            match self {
                Number::Even(__inner) => {
                    let (__element0, __element1) = <Even as Split>::halves(__inner);
                    (::core::convert::Into::into(__element0), __element1.map(Number::Even))
                }
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Even(inner) => {
                    let (rest, half) = <Even as Split>::halves(inner);
                    (rest.into(), half.map(Self::Even))
                }
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(
            arm(subject, "Even"),
            strings(&["call Split::halves", "into", "construct Even", "map"])
        );
        let subject = r#"{
            match self {
                Number::Even(__inner) => {
                    let (__element0, __element1) = <Even as Split>::split(__inner);
                    (Number::Even(__element0), __element1)
                }
            }
        }"#;
        let hand = r#"{
            match self {
                Number::Even(inner) => {
                    let (even, rest) = <Even as Split>::split(inner);
                    (Self::Even(even), rest)
                }
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn an_async_fn() {
        let subject = r#"{
            match self {
                AnyStore::Memory(__inner) => <Memory as Store>::flush(__inner).await,
                AnyStore::Disk(__inner) => <Disk as Store>::flush(__inner).await,
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Memory(inner) => <Memory as Store>::flush(inner).await,
                Self::Disk(inner) => <Disk as Store>::flush(inner).await,
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(
            arm(subject, "Disk"),
            strings(&["call Store::flush", "await"])
        );
    }

    #[test]
    fn an_impl_future_in_one_async_move_block() {
        let subject = r#"{
            async move {
                match self {
                    AnyStore::Memory(__inner) => {
                        AnyStore::Memory(<Memory as Store>::copied(__inner).await)
                    }
                    AnyStore::Disk(__inner) => {
                        AnyStore::Disk(<Disk as Store>::copied(__inner).await)
                    }
                }
            }
        }"#;
        let hand = r#"{
            async move {
                match self {
                    Self::Memory(inner) => Self::Memory(<Memory as Store>::copied(inner).await),
                    Self::Disk(inner) => Self::Disk(<Disk as Store>::copied(inner).await),
                }
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(shape(subject).around, strings(&["async"]));
    }

    #[test]
    fn a_boxed_future_with_and_without_a_converted_output() {
        let subject = r#"{
            Box::pin(async move {
                match self {
                    Any::Memory(__inner) => {
                        <Memory as Store>::load(__inner)
                            .await
                            .map_err(::core::convert::Into::into)
                    }
                    Any::Disk(__inner) => {
                        <Disk as Store>::load(__inner)
                            .await
                            .map_err(::core::convert::Into::into)
                    }
                }
            })
        }"#;
        let hand = r#"{
            Box::pin(async move {
                match self {
                    Self::Memory(inner) => <Memory as Store>::load(inner).await.map_err(Into::into),
                    Self::Disk(inner) => <Disk as Store>::load(inner).await.map_err(Into::into),
                }
            })
        }"#;
        assert_same(subject, hand);
        assert_eq!(shape(subject).around, strings(&["fn Box::pin", "async"]));
        let subject = r#"{
            match self {
                Any::Memory(__inner) => <Memory as Store>::ping(__inner),
                Any::Disk(__inner) => <Disk as Store>::ping(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Memory(inner) => <Memory as Store>::ping(inner),
                Self::Disk(inner) => <Disk as Store>::ping(inner),
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn inherent_methods_fields_and_a_fallback() {
        let subject = r#"{
            match self {
                Self::Circle(__inner) => __inner.describe(),
                Self::Square(__inner) => __inner.describe(),
                Self::Boxed(__inner) => (**__inner).describe(),
            }
        }"#;
        let hand = r#"{
            match self {
                AnyShape::Circle(circle) => circle.describe(),
                AnyShape::Square(square) => square.describe(),
                AnyShape::Boxed(boxed) => boxed.describe(),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(arm(subject, "Boxed"), strings(&["method describe"]));
        let subject = r#"{
            match self {
                Self::Circle(__inner) => Some(__inner.radius()),
                _ => None,
            }
        }"#;
        let hand = r#"{
            match self {
                AnyShape::Circle(circle) => Some(circle.radius()),
                _ => None,
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(arm(subject, "Circle"), strings(&["method radius", "some"]));
        let subject = r#"{
            match self {
                Self::Circle(__inner) => &__inner.0,
                Self::Square(__inner) => &__inner.0,
                Self::Boxed(__inner) => &(**__inner).0,
            }
        }"#;
        let hand = r#"{
            match self {
                AnyShape::Circle(circle) => &circle.0,
                AnyShape::Square(square) => &square.0,
                AnyShape::Boxed(boxed) => &boxed.0,
            }
        }"#;
        assert_same(subject, hand);
        let subject = r#"{
            match self {
                Self::Plain(__inner) => Some(&mut __inner.id),
                _ => None,
            }
        }"#;
        let hand = r#"{
            match self {
                Held::Plain(card) => Some(&mut card.id),
                _ => None,
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn an_inherent_future_awaited() {
        let subject = r#"{
            async move {
                match self {
                    Self::Memory(__inner) => __inner.size().await,
                    Self::Disk(__inner) => __inner.size().await,
                }
            }
        }"#;
        let hand = r#"{
            async move {
                match self {
                    AnyStore::Memory(memory) => memory.size().await,
                    AnyStore::Disk(disk) => disk.size().await,
                }
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn a_struct_newtype_and_a_struct_with_a_named_field() {
        let subject = r#"{
            match self {
                Wrapped(__inner, ..) => Wrapped(<AnyShape as Shape>::grown(__inner)),
            }
        }"#;
        let hand = r#"{
            match self {
                Self(inner) => Self(<AnyShape as Shape>::grown(inner)),
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(
            arm(subject, "Self"),
            strings(&["call Shape::grown", "construct Self"])
        );
        let subject = r#"{
            match self {
                Labelled { shape: __inner, .. } => <AnyShape as Shape>::area(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Self { shape, .. } => <AnyShape as Shape>::area(shape),
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn a_struct_forwarding_an_opaque_type() {
        let subject = r#"{
            match self {
                Wrapped(__inner, ..) => <Few as Source>::items(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Wrapped(inner) => <Few as Source>::items(inner),
            }
        }"#;
        assert_same(subject, hand);
        let subject = r#"{
            async move {
                match self {
                    Wrapped(__inner, ..) => Wrapped(<Few as Source>::reloaded(__inner).await),
                }
            }
        }"#;
        let hand = r#"{
            async move {
                match self {
                    Self(inner) => Self(<Few as Source>::reloaded(inner).await),
                }
            }
        }"#;
        assert_same(subject, hand);
    }

    #[test]
    fn a_bare_body() {
        let subject = r#"{
            Labelled {
                shape: self.shape.grown(),
                label: self.label,
            }
        }"#;
        let hand = r#"{
            Self { shape: self.shape.grown(), label: self.label }
        }"#;
        assert_same(subject, hand);
        let read = shape(subject);
        assert!(read.arms.is_empty());
        assert!(read.around.is_empty());
        assert_eq!(
            read.bare,
            strings(&[
                "field shape",
                "method grown",
                "field label",
                "construct Self"
            ])
        );
        assert_same("{ Self::Cat(__value) }", "{ Animal::Cat(value) }");
    }

    #[test]
    fn generic_methods_lose_their_turbofish() {
        let subject = r#"{
            match self {
                Any::Text(__inner) => <Text as Source>::parse::<T>(__inner),
                Any::Number(__inner) => <Number as Source>::zeros::<N>(__inner),
            }
        }"#;
        let hand = r#"{
            match self {
                Self::Text(inner) => <Text as Source>::parse(inner),
                Self::Number(inner) => <Number as Source>::zeros(inner),
            }
        }"#;
        assert_same(subject, hand);
        assert_same(
            "{ match self { Self::A(x) => x.parse::<T>() } }",
            "{ match self { Self::A(x) => x.parse() } }",
        );
    }

    #[test]
    fn unsafe_blocks_around_and_in_arms() {
        let subject = r#"{
            match self {
                Any::A(__inner) => unsafe { <A as Raw>::read(__inner) },
            }
        }"#;
        let hand = r#"{
            match self {
                Self::A(inner) => unsafe { <A as Raw>::read(inner) },
            }
        }"#;
        assert_same(subject, hand);
        assert_eq!(arm(subject, "A"), strings(&["unsafe", "call Raw::read"]));
        let subject = r#"{
            unsafe {
                match self {
                    Any::A(__inner) => <A as Raw>::read(__inner),
                }
            }
        }"#;
        assert_eq!(shape(subject).around, strings(&["unsafe"]));
    }

    #[test]
    fn spelling_that_carries_no_meaning_is_erased() {
        assert_same(
            "{ match self { Self::A(x) => ::core::convert::Into::into(x) } }",
            "{ match self { E::A(y) => y.into() } }",
        );
        assert_same(
            "{ match self { Self::A(x) => Into::into(x) } }",
            "{ match self { E::A(y) => (y).into() } }",
        );
        assert_same(
            "{ match self { Self::A(x) => <_ as T>::m(Pin::new(x)) } }",
            "{ match self { E::A(y) => <A as T>::m(::core::pin::Pin::new(&mut *y)) } }",
        );
        assert_same(
            "{ match self { Self::A(x) => { { (x.0) } } } }",
            "{ match self { E::A(ref y) => (**y).0 } }",
        );
        assert_same(
            "{ match self { Self::A(x) => x.f().map(Self::B) } }",
            "{ match self { E::A(x) => x.f().map(E::B) } }",
        );
    }

    #[test]
    fn invisible_groups_are_erased() {
        use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
        use std::str::FromStr;

        let inner = TokenStream::from_str("& * __inner").expect("tokens");
        let grouped: TokenStream = TokenTree::Group(Group::new(Delimiter::None, inner)).into();
        let argument: Expr = syn::parse2(grouped).expect("a grouped expression");
        assert!(matches!(argument, Expr::Group(_)));
        let mut body: Block =
            syn::parse_str("{ match self { E::A(__inner) => <A as T>::m(x) } }").expect("body");
        let Some(Stmt::Expr(Expr::Match(found), None)) = body.stmts.first_mut() else {
            panic!("the body is a match");
        };
        let Expr::Call(call) = found.arms[0].body.as_mut() else {
            panic!("the arm is a call");
        };
        call.args[0] = argument;
        let grouped_match = Expr::Group(syn::ExprGroup {
            attrs: Vec::new(),
            group_token: Default::default(),
            expr: Box::new(Expr::Match(found.clone())),
        });
        body.stmts[0] = Stmt::Expr(grouped_match, None);
        let hand = shape("{ match self { E::A(inner) => <A as T>::m(inner) } }");
        let lines = differences(&method_shape(&body), &hand);
        assert!(lines.is_empty(), "unexpected differences: {lines:#?}");
    }

    /// A body with two plain arms, the hand most differences are taken
    /// against.
    const PLAIN: &str = "{ match self { E::A(x) => <A as T>::m(x), E::B(x) => <B as T>::m(x) } }";

    #[test]
    fn an_extra_clone_or_to_owned_differs() {
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x.clone()), E::B(x) => <B as T>::m(x) } }",
            PLAIN,
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x).to_owned(), E::B(x) => <B as T>::m(x) } }",
            PLAIN,
            "arm A",
        );
    }

    #[test]
    fn an_extra_allocation_differs() {
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x), E::B(x) => Box::new(<B as T>::m(x)) } }",
            PLAIN,
            "arm B",
        );
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x), E::B(x) => Rc::new(<B as T>::m(x)) } }",
            PLAIN,
            "arm B",
        );
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x).map(Box::new) } }",
            "{ match self { E::A(x) => <A as T>::m(x).map(E::A) } }",
            "arm A",
        );
    }

    #[test]
    fn an_extra_conversion_differs() {
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x).into(), E::B(x) => <B as T>::m(x) } }",
            PLAIN,
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x), E::B(x) => Into::into(<B as T>::m(x)) } }",
            PLAIN,
            "arm B",
        );
    }

    #[test]
    fn a_closure_differs_and_is_a_violation() {
        let subject = "{ match self { E::A(x) => <A as T>::m(x).map(|y| E::A(y)) } }";
        let hand = "{ match self { E::A(x) => <A as T>::m(x).map(E::A) } }";
        assert_differs(subject, hand, "arm A");
        let lines = compare(subject, hand);
        assert_eq!(lines[0], "violation: a closure");
    }

    #[test]
    fn an_inherent_call_differs_from_the_trait_call() {
        assert_differs(
            "{ match self { E::A(x) => x.m(), E::B(x) => <B as T>::m(x) } }",
            PLAIN,
            "arm A",
        );
        assert_differs(
            PLAIN,
            "{ match self { E::A(x) => x.m(), E::B(x) => <B as T>::m(x) } }",
            "arm A",
        );
    }

    #[test]
    fn a_different_method_trait_field_or_variant_differs() {
        assert_differs(
            "{ match self { E::A(x) => <A as T>::n(x), E::B(x) => <B as T>::m(x) } }",
            PLAIN,
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => <A as U>::m(x), E::B(x) => <B as T>::m(x) } }",
            PLAIN,
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => <_ as T>::m(&x.base) } }",
            "{ match self { E::A(x) => <_ as T>::m(&x.id) } }",
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => E::B(<A as T>::m(x)) } }",
            "{ match self { E::A(x) => E::A(<A as T>::m(x)) } }",
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => x.describe() } }",
            "{ match self { E::A(x) => x.name() } }",
            "arm A",
        );
    }

    #[test]
    fn a_missing_or_extra_await_differs() {
        assert_differs(
            PLAIN,
            "{ match self { E::A(x) => <A as T>::m(x).await, E::B(x) => <B as T>::m(x) } }",
            "arm A",
        );
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x), E::B(x) => <B as T>::m(x).await } }",
            PLAIN,
            "arm B",
        );
    }

    #[test]
    fn a_missing_unsafe_differs() {
        assert_differs(
            PLAIN,
            "{ match self { E::A(x) => unsafe { <A as T>::m(x) }, E::B(x) => <B as T>::m(x) } }",
            "arm A",
        );
        assert_differs(
            PLAIN,
            "{ unsafe { match self { E::A(x) => <A as T>::m(x), E::B(x) => <B as T>::m(x) } } }",
            "around",
        );
    }

    #[test]
    fn a_missing_or_extra_arm_differs() {
        let missing = "{ match self { E::A(x) => <A as T>::m(x) } }";
        assert_eq!(
            compare(missing, PLAIN),
            ["arm B: only in the hand, [call T::m]"]
        );
        assert_eq!(
            compare(PLAIN, missing),
            ["arm B: only in the subject, [call T::m]"]
        );
    }

    #[test]
    fn a_missing_or_extra_fallback_differs() {
        let fallback = "{ match self { E::A(x) => <A as T>::m(x), _ => 0 } }";
        assert_differs(fallback, PLAIN, "arm _");
        assert_differs(PLAIN, fallback, "arm _");
        assert_differs(
            "{ match self { E::A(x) => <A as T>::m(x), __other => 0 } }",
            "{ match self { E::A(x) => <A as T>::m(x) } }",
            "arm _",
        );
    }

    #[test]
    fn arms_in_a_different_order_differ() {
        let swapped = "{ match self { E::B(x) => <B as T>::m(x), E::A(x) => <A as T>::m(x) } }";
        let lines = compare(swapped, PLAIN);
        assert_eq!(lines.len(), 2, "{lines:#?}");
        assert!(lines[0].starts_with("arm B: at position 1 in the subject and 2 in the hand"));
        assert!(lines[1].starts_with("arm A: at position 2 in the subject and 1 in the hand"));
    }

    #[test]
    fn a_difference_around_or_in_the_bare_body_is_named() {
        assert_differs(
            "{ async move { match self { E::A(x) => <A as T>::m(x).await } } }",
            "{ match self { E::A(x) => <A as T>::m(x).await } }",
            "around",
        );
        assert_differs(
            "{ Box::pin(async move { match self { E::A(x) => <A as T>::m(x).await } }) }",
            "{ async move { match self { E::A(x) => <A as T>::m(x).await } } }",
            "around",
        );
        assert_differs(
            "{ <S as T>::m(&self.0).clone() }",
            "{ <S as T>::m(&self.0) }",
            "bare",
        );
    }

    #[test]
    fn a_path_starting_with_std_or_alloc_is_a_violation() {
        let read = shape("{ match self { E::A(x) => std::pin::Pin::new(x) } }");
        assert_eq!(
            read.violations,
            ["the path `std::pin::Pin::new` starts with `std`"]
        );
        let read = shape("{ match self { E::A(x) => ::alloc::boxed::Box::new(x) } }");
        assert_eq!(
            read.violations,
            ["the path `::alloc::boxed::Box::new` starts with `alloc`"]
        );
        let read = shape("{ match self { E::A(x) => <X as ::std::ops::Deref>::deref(x) } }");
        assert_eq!(
            read.violations,
            ["the path `::std::ops::Deref::deref` starts with `std`"]
        );
    }

    #[test]
    fn a_leading_colon_outside_core_is_a_violation() {
        let read = shape("{ match self { E::A(x) => ::other::convert(x) } }");
        assert_eq!(
            read.violations,
            ["the path `::other::convert` has a leading `::` and does not start with `::core`"]
        );
        let read = shape(
            "{ match ::core::pin::Pin::get_mut(self) { E::A(x) => <<Box<A> as ::core::ops::Deref>::Target as T>::m(core::ptr::null()) } }",
        );
        assert!(read.violations.is_empty(), "{:?}", read.violations);
    }

    #[test]
    fn violations_are_reported_first() {
        let subject = "{ match self { E::A(x) => (|| <A as T>::m(x))(), E::B(x) => ::std::convert::identity(x) } }";
        let lines = compare(subject, PLAIN);
        assert_eq!(lines[0], "violation: a closure");
        assert_eq!(
            lines[1],
            "violation: the path `::std::convert::identity` starts with `std`"
        );
        assert!(lines.len() > 2);
        assert!(lines[2..].iter().all(|line| !line.starts_with("violation")));
    }

    #[test]
    fn the_hands_violations_are_not_reported() {
        assert_same(
            "{ match self { E::A(x) => ::core::pin::Pin::new(x) } }",
            "{ match self { E::A(x) => std::pin::Pin::new(x) } }",
        );
    }

    /// A file with a case module, its `hand` submodule, trait and inherent
    /// impls of one type, and generic ones of another.
    const FILE: &str = r#"
        mod c_1234abcd {
            pub trait Area { fn area(&self) -> u32; }
            pub enum Shape { A(A) }
            impl crate::c_1234abcd::Area for Shape {
                fn area(&self) -> u32 { match self { Shape::A(x) => <A as Area>::area(x) } }
            }
            impl Shape {
                fn side(&self) -> u32 { 0 }
            }
            impl<'a, T> Area for Holder<'a, T> {
                fn area(&self) -> u32 { 1 }
            }
            impl<'a, T> self::Holder<'a, T> {
                fn get(&self) -> u32 { 2 }
                fn area(&self) -> u32 { 3 }
            }
            impl Other for Shape {
                fn area(&self) -> u32 { 4 }
            }
            mod hand {
                impl Area for Shape {
                    fn area(&self) -> u32 { 5 }
                }
            }
            mod outlined;
        }
    "#;

    #[test]
    fn the_lookups_find_modules_impls_and_methods() {
        let file: File = syn::parse_str(FILE).expect("the file parses");
        let outer = module(&file, &["c_1234abcd"]).expect("the module");
        assert_eq!(outer.len(), 9);
        let hand = module(&file, &["c_1234abcd", "hand"]).expect("the submodule");
        assert_eq!(hand.len(), 1);
        assert!(module(&file, &["c_1234abcd", "outlined"]).is_none());
        assert!(module(&file, &["missing"]).is_none());
        assert_eq!(module(&file, &[]).map(<[Item]>::len), Some(1));

        let traits = impls(outer, "Shape", Some("Area"));
        assert_eq!(traits.len(), 1);
        let inherent = impls(outer, "Shape", None);
        assert_eq!(inherent.len(), 1);
        assert_eq!(impls(outer, "Shape", Some("Other")).len(), 1);
        assert_eq!(impls(outer, "Holder", Some("Area")).len(), 1);
        assert_eq!(impls(outer, "Holder", None).len(), 1);
        assert!(impls(outer, "Missing", None).is_empty());
        assert_eq!(impls(hand, "Shape", Some("Area")).len(), 1);
        assert!(impls(hand, "Shape", None).is_empty());

        let area = method(&traits, "area").expect("the trait's method");
        assert_eq!(shape_of(area).arms.len(), 1);
        assert!(method(&inherent, "area").is_none());
        assert!(method(&inherent, "side").is_some());
        let holder = impls(outer, "Holder", None);
        let get = method(&holder, "get").expect("the generic inherent method");
        assert_eq!(get.sig.ident, "get");
        let both: Vec<&ItemImpl> = inherent.iter().chain(&holder).copied().collect();
        let area = method(&both, "area").expect("the second block's method");
        assert_eq!(shape_of(area).bare, Vec::<String>::new());
        assert!(method(&[], "area").is_none());
    }

    #[test]
    fn every_golden_body_is_read() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/expand");
        let mut seen = 0;
        for entry in std::fs::read_dir(&directory).expect("the goldens") {
            let path = entry.expect("an entry").path();
            if !path.to_string_lossy().ends_with(".expanded.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a golden");
            let file: File = syn::parse_file(&source).expect("a golden parses");
            for item in &file.items {
                let Item::Impl(block) = item else {
                    continue;
                };
                let generated = block
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("automatically_derived"));
                for impl_item in &block.items {
                    let ImplItem::Fn(function) = impl_item else {
                        continue;
                    };
                    seen += 1;
                    let read = shape_of(function);
                    let all = read
                        .around
                        .iter()
                        .chain(&read.bare)
                        .chain(read.arms.iter().flat_map(|arm| &arm.operations))
                        .chain(read.arms.iter().map(|arm| &arm.variant));
                    for operation in all {
                        assert!(
                            operation != "other expression"
                                && operation != "pattern"
                                && operation != "fn",
                            "{}: `{}` reads `{operation}`",
                            path.display(),
                            function.sig.ident
                        );
                    }
                    if generated {
                        assert!(
                            read.violations.is_empty(),
                            "{}: `{}` holds {:?}",
                            path.display(),
                            function.sig.ident,
                            read.violations
                        );
                    }
                }
            }
        }
        assert!(seen > 100, "only {seen} methods were read");
    }
}
