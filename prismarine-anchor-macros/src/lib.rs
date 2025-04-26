mod generate;

use proc_macro::TokenStream;
use syn::{parse_macro_input, Token};
use syn::{punctuated::Punctuated, spanned::Spanned as _, Attribute, DeriveInput, Type};

use self::generate::generate_impl;


// TODO: compound! macro, and more convenient deserialization with error handling

#[proc_macro_derive(CustomTranslator, attributes(translator_types))]
pub fn custom_translator(tokens: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(tokens as DeriveInput);

    match parse_input(&input) {
        Ok(types) => generate_impl(&input.ident, &types),
        Err(err) => err.into_compile_error(),
    }.into()
}

struct TranslatorTypes {
    error:  Type,
    block:  Type,
    entity: Type,
    item:   Type,
}

fn parse_input(input: &DeriveInput) -> syn::Result<TranslatorTypes> {
    if let Some(attr) = input.attrs.iter().find(|attr| {
        attr.path().is_ident("translator_types")
    }) {
        parse_types(attr)
    } else {
        Err(syn::Error::new(input.span(), "expected four comma-separated types"))
    }
}

fn parse_types(attr: &Attribute) -> syn::Result<TranslatorTypes> {
    let types: _ = attr.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated)?;

    if types.len() != 4 {
        return Err(syn::Error::new(types.span(), "expected four comma-separated types"));
    }

    let mut types = types.into_iter();

    Ok(TranslatorTypes {
        error:  types.next().unwrap(),
        block:  types.next().unwrap(),
        entity: types.next().unwrap(),
        item:   types.next().unwrap(),
    })
}
