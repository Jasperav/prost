use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_str, Expr, ExprLit, Ident, Lit, Meta, MetaNameValue, Path};

use crate::field::{set_bool, set_option, tags_attr, word_attr};

#[derive(Clone)]
pub struct Field {
    pub ty: Path,
    pub tags: Vec<u32>,
    pub strict: bool,
}

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        let mut ty = None;
        let mut tags = None;
        let mut strict = false;
        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("oneof") {
                let t = match *attr {
                    Meta::NameValue(MetaNameValue {
                        value:
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(ref lit),
                                ..
                            }),
                        ..
                    }) => parse_str::<Path>(&lit.value())?,
                    Meta::List(ref list) => list.parse_args::<Ident>()?.into(),
                    _ => bail!("invalid oneof attribute: {:?}", attr),
                };
                set_option(&mut ty, t, "duplicate oneof attribute")?;
            } else if word_attr("strict", attr) {
                set_bool(&mut strict, "duplicate strict attribute")?;
            } else if let Some(t) = tags_attr(attr)? {
                set_option(&mut tags, t, "duplicate tags attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        let ty = match ty {
            Some(ty) => ty,
            None => return Ok(None),
        };

        match unknown_attrs.len() {
            0 => (),
            1 => bail!(
                "unknown attribute for message field: {:?}",
                unknown_attrs[0]
            ),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        let tags = match tags {
            Some(tags) => tags,
            None => bail!("oneof field is missing a tags attribute"),
        };

        Ok(Some(Field { ty, tags, strict }))
    }

    /// Returns a statement which encodes the oneof field.
    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        quote! {
            if let Some(ref oneof) = #ident {
                oneof.encode(buf)
            }
        }
    }

    /// Returns an expression which evaluates to the result of decoding the oneof field.
    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        let ty = &self.ty;
        quote! {
            #ty::merge(#ident, tag, wire_type, buf, ctx)
        }
    }

    /// Returns an expression which evaluates to the encoded length of the oneof field.
    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let ty = &self.ty;
        quote! {
            #ident.as_ref().map_or(0, #ty::encoded_len)
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident = ::core::option::Option::None)
    }
}
