use proc_macro2::{Ident, TokenStream};
use quote::quote;
use quote::ToTokens;

use super::TranslatorTypes;


pub fn generate_impl(name: Ident, types: TranslatorTypes) -> TokenStream {
    let source_error = &types.error;
    let source_block_metadata = &types.block;
    let source_translator = SourceTranslator { types: &types };

    let cow             = Shorthand::Cow;
    let into            = Shorthand::IntoTrait;
    let into_into       = Shorthand::Into;
    let result          = Shorthand::Result;
    let ok              = Shorthand::Ok;
    let err             = Shorthand::Err;
    let string          = Shorthand::String;
    let fn_trait        = Shorthand::Fn;
    let option          = Shorthand::Option;
    let prim_u32        = Shorthand::U32;
    let prim_str        = Shorthand::Str;
    let error           = Shorthand::Error;
    let block           = Shorthand::Block;
    let block_entity    = Shorthand::BlockEntity;
    let block_or_entity = Shorthand::BlockOrEntity;
    let block_position  = Shorthand::BlockPosition;
    let entity          = Shorthand::Entity;
    let item            = Shorthand::Item;
    let translator      = Shorthand::Translator;
    let block_metadata  = Shorthand::BlockMetadata;

    quote! {
        impl #translator::<#error, #block_metadata, (), ()> for #name
        where
            Self: #source_translator,
            #source_error: #into<#error>,
            #source_block_metadata: #into<#block_metadata>,
        {
            fn translator_name(&self) -> #string {
                #source_translator::translator_name(self)
            }

            fn translate_block(
                &self,
                block: #block,
                block_entity: #option<#block_entity>,
                position: #block_position,
                get_block: &dyn #fn_trait(#block_position) -> (#block, #option<#block_entity>),
            ) -> #result<
                (#block_or_entity, #block_metadata),
                (#error, #block, #option<#block_entity>, #block_metadata),
            > {
                match #source_translator::translate_block(self, block, block_entity, position, get_block) {
                    #ok(ok) => #ok((
                        ok.0,
                        #into_into(ok.1),
                    )),
                    #err(err) => #err((
                        #into_into(err.0),
                        err.1,
                        err.2,
                        #into_into(err.3),
                    ))
                }
            }

            fn translate_entity(&self, entity: #entity) -> Result<
                (#block_or_entity, ()),
                (#error, #entity, ()),
            > {
                match #source_translator::translate_entity(self, entity) {
                    #ok(ok) => #ok((
                        ok.0,
                        (),
                    )),
                    #err(err) => #err((
                        #into_into(err.0),
                        err.1,
                        (),
                    ))
                }
            }

            fn translate_item(&self, item: #item) -> Result<
                (#item, ()),
                (#error, #item, ()),
            > {
                match #source_translator::translate_item(self, item) {
                    #ok(ok) => #ok((ok.0, ())),
                    #err(err) => #err((
                        #into_into(err.0),
                        err.1,
                        (),
                    ))
                }
            }

            fn translate_biome(&self, biome: &#prim_str) -> Result<#cow, #error> {
                match #source_translator::translate_biome(self, biome) {
                    #ok(ok) => #ok(ok),
                    #err(err) => #err(#into_into(err)),
                }
            }

            fn biome_to_numeric(&self, biome: &#prim_str) -> Result<#prim_u32, #error> {
                match #source_translator::biome_to_numeric(self, biome) {
                    #ok(ok) => #ok(ok),
                    #err(err) => #err(#into_into(err)),
                }
            }

            fn biome_from_numeric(&self, num: #prim_u32) -> Result<#cow, #error> {
                match #source_translator::biome_from_numeric(self, num) {
                    #ok(ok) => #ok(ok),
                    #err(err) => #err(#into_into(err)),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Shorthand {
    Cow,
    IntoTrait,
    Into,
    Result,
    Ok,
    Err,
    String,
    Fn,
    Option,
    U32,
    Str,
    Error,
    Block,
    BlockEntity,
    BlockOrEntity,
    BlockPosition,
    Entity,
    Item,
    Translator,
    BlockMetadata,
}

impl ToTokens for Shorthand {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match *self {
            Self::Cow           => quote! {::std::borrow::Cow::<'static, ::std::primitive::str>},
            Self::IntoTrait     => quote! {::std::convert::Into},
            Self::Into          => quote! {::std::convert::Into::into},
            Self::Result        => quote! {::std::result::Result},
            Self::Ok            => quote! {::std::result::Result::Ok},
            Self::Err           => quote! {::std::result::Result::Err},
            Self::String        => quote! {::std::string::String},
            Self::Fn            => quote! {::std::ops::Fn},
            Self::Option        => quote! {::std::option::Option},
            Self::U32           => quote! {::std::primitive::u32},
            Self::Str           => quote! {::std::primitive::str},
            // Intentionally, these are left without the leading :: to add flexibility
            // in how they're imported. Can't be perfectly hygenic without being obnoxious.
            Self::Error         => quote! {anyhow::Error},
            Self::Block         => quote! {prismarine_anchor_translation::datatypes::Block},
            Self::BlockEntity   => quote! {prismarine_anchor_translation::datatypes::BlockEntity},
            Self::BlockOrEntity => quote! {prismarine_anchor_translation::datatypes::BlockOrEntity},
            Self::BlockPosition => quote! {prismarine_anchor_translation::datatypes::BlockPosition},
            Self::Entity        => quote! {prismarine_anchor_translation::datatypes::Entity},
            Self::Item          => quote! {prismarine_anchor_translation::datatypes::Item},
            Self::Translator    => quote! {prismarine_anchor_translation::translator::Translator},
            Self::BlockMetadata => quote! {prismarine_anchor_world::translation::BlockMetadata},
        });
    }
}

struct SourceTranslator<'a> {
    types: &'a TranslatorTypes,
}

impl ToTokens for SourceTranslator<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let TranslatorTypes { error, block, entity, item } = self.types;

        let translator = Shorthand::Translator;

        tokens.extend(quote! {
            #translator::<#error, #block, #entity, #item>
        })
    }
}
