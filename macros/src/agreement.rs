//! The checks that every variant agrees with the first on the associated
//! types and consts the impl takes from it.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::{parse_quote_spanned, GenericArgument, Ident, Path, PathArguments, Type};

use crate::at;
use crate::shape::bare_ident;
use crate::subject::Target;

/// How two values of an associated const's type are compared at compile
/// time: with `==` for the primitives that allow it in a `const`, or byte
/// by byte for `&str`.
pub(crate) enum ConstComparison {
    Primitive,
    Str,
}

pub(crate) fn const_comparison(ty: &Type) -> Option<ConstComparison> {
    const PRIMITIVES: [&str; 15] = [
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
        "bool", "char", "()",
    ];
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Some(ConstComparison::Primitive),
        Type::Reference(reference) if reference.mutability.is_none() => {
            match bare_ident(&reference.elem) {
                Some(ident) if ident == "str" => Some(ConstComparison::Str),
                _ => None,
            }
        }
        _ => bare_ident(ty)
            .filter(|ident| PRIMITIVES.contains(&ident.to_string().as_str()))
            .map(|_| ConstComparison::Primitive),
    }
}

/// What the agreement checks of one impl are written against: the
/// variants, the first among them that supplies the items, and the impl's
/// own header. The checks live outside the impl, so a `Self` in a payload
/// type is spelled as `self_type` there, and the blocks' own tokens carry
/// `site`, the subject's span.
pub(crate) struct Agreement<'a> {
    pub(crate) variants: &'a [&'a Target],
    pub(crate) first: &'a Target,
    pub(crate) trait_path: &'a Path,
    /// The first variant's target type, spelled for a position outside
    /// the impl.
    pub(crate) first_payload: &'a TokenStream2,
    pub(crate) generics: &'a syn::Generics,
    pub(crate) self_type: &'a Type,
    pub(crate) site: Span,
}

impl Agreement<'_> {
    /// Returns every variant but the first, the one the items were taken
    /// from.
    fn others(&self) -> impl Iterator<Item = &&Target> {
        let first = self.first;
        self.variants
            .iter()
            .filter(move |variant| !std::ptr::eq(**variant, first))
    }

    /// Checks at compile time that every variant's associated consts of a
    /// comparable type equal the first variant's, with the failure reported
    /// on the variant. Const evaluation happens after monomorphisation, so
    /// a generic enum gets no check.
    pub(crate) fn consts(&self, consts: &[(&Ident, Type)]) -> TokenStream2 {
        let Agreement {
            variants,
            trait_path,
            first_payload,
            generics,
            self_type,
            site,
            ..
        } = *self;
        if variants.len() < 2 || !generics.params.is_empty() {
            return TokenStream2::new();
        }
        let mut checks = TokenStream2::new();
        let mut needs_str_eq = false;
        for (name, ty) in consts {
            let Some(comparison) = const_comparison(ty) else {
                continue;
            };
            for variant in self.others() {
                // A target reached through a field or a method has no type
                // to name here; a disagreement shows in the method's arm
                // instead.
                let Some(payload) = variant.target_type_outside(self_type) else {
                    continue;
                };
                let cfgs = &variant.cfgs;
                let message = format!(
                    "`{name}` of variant `{}` differs from the first variant's; set it on the enum with `{name} = value`",
                    variant.ident
                );
                let span = at(variant.payload.span());
                let equal = match comparison {
                    ConstComparison::Primitive => quote_spanned! { span =>
                        <#first_payload as #trait_path>::#name == <#payload as #trait_path>::#name
                    },
                    ConstComparison::Str => {
                        needs_str_eq = true;
                        quote_spanned! { span =>
                            __whichever_str_eq(<#first_payload as #trait_path>::#name, <#payload as #trait_path>::#name)
                        }
                    }
                };
                checks.extend(quote_spanned! { span =>
                    #(#cfgs)* assert!(#equal, #message);
                });
            }
        }
        if checks.is_empty() {
            return TokenStream2::new();
        }
        let str_eq = needs_str_eq.then(|| {
            quote_spanned! { site =>
                const fn __whichever_str_eq(a: &str, b: &str) -> bool {
                    let (a, b) = (a.as_bytes(), b.as_bytes());
                    if a.len() != b.len() {
                        return false;
                    }
                    let mut i = 0;
                    while i < a.len() {
                        if a[i] != b[i] {
                            return false;
                        }
                        i += 1;
                    }
                    true
                }
            }
        });
        quote_spanned! { site =>
            const _: () = {
                #str_eq
                #checks
            };
        }
    }

    /// Checks at compile time that every variant's associated types equal
    /// the first variant's, so a disagreement is reported on the variant
    /// instead of inside a generated method body.
    ///
    /// The helpers are associated functions of a local struct wrapping the
    /// subject, inside the anonymous `const`. The struct's field gives its
    /// impl the bounds the subject's fields imply, `T: 'a` for a payload
    /// holding `&'a T`, which free functions copying the subject's generics
    /// would lack; and the subject's parameters belong to the impl, so none
    /// is passed to the helper, where a lifetime the first payload does not
    /// mention would be late-bound and could not be. The struct is local to
    /// the `const`, so two traits dispatched on one subject never meet.
    pub(crate) fn types(&self, names: &[&Ident]) -> TokenStream2 {
        let Agreement {
            variants,
            first,
            trait_path,
            first_payload,
            generics,
            self_type,
            site,
        } = *self;
        if names.is_empty() || variants.len() < 2 {
            return TokenStream2::new();
        }
        let first_span = at(first.payload.span());
        let bindings: Vec<GenericArgument> = names
            .iter()
            .map(|name| {
                parse_quote_spanned!(first_span => #name = <#first_payload as #trait_path>::#name)
            })
            .collect();
        let mut bound = trait_path.clone();
        let last = bound
            .segments
            .last_mut()
            .expect("a path has at least one segment");
        match &mut last.arguments {
            PathArguments::AngleBracketed(arguments) => arguments.args.extend(bindings),
            arguments => {
                *arguments = PathArguments::AngleBracketed(
                    parse_quote_spanned!(first_span => <#(#bindings),*>),
                )
            }
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let checks = self.others().filter_map(|variant| {
            let payload = variant.target_type_outside(self_type)?;
            let cfgs = &variant.cfgs;
            let span = at(variant.payload.span());
            Some(quote_spanned! { span => #(#cfgs)* Self::__same_as_first_variant::<#payload>(); })
        });
        quote_spanned! { site =>
            const _: () = {
                #[allow(dead_code)]
                struct __WhicheverAgreement #impl_generics (#self_type) #where_clause;
                #[allow(dead_code, non_snake_case)]
                impl #impl_generics __WhicheverAgreement #ty_generics #where_clause {
                    fn __same_as_first_variant<__Variant: ?Sized + #bound>() {}
                    fn __whichever_associated_types_agree() {
                        #(#checks)*
                    }
                }
            };
        }
    }
}
