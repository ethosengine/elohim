//! Syntax diagnostics: all cfg branches, without compiling the owning crate.
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::{parse::Parser, punctuated::Punctuated, visit::Visit, Expr, Lit, Meta, Token};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub rule: String,
    pub path: String,
    pub line: usize,
    pub scope: String,
    pub attribute: String,
    pub detail: String,
}

impl Finding {
    /// Line-independent identity. Multiplicity is retained by the gate.
    pub fn identity(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.rule, self.path, self.scope, self.attribute
        )
    }
}

pub fn scan(path: &str, source: &str) -> Result<Vec<Finding>, String> {
    let file = syn::parse_file(source).map_err(|e| format!("{path}: UNMEASURED: {e}"))?;
    let mut visitor = Scanner {
        path,
        scopes: Vec::new(),
        findings: Vec::new(),
    };
    visitor.visit_file(&file);
    Ok(visitor.findings)
}

struct Scanner<'a> {
    path: &'a str,
    scopes: Vec<String>,
    findings: Vec<Finding>,
}

impl Scanner<'_> {
    fn meta(&mut self, meta: &Meta, span: Span) {
        let Meta::List(list) = meta else { return };
        if list.path.is_ident("cfg_attr") {
            // Ignore the predicate, including macro metavariables such as $cfg.
            // Commas inside groups stay grouped, so only top-level commas split.
            let mut predicate = true;
            let mut part = TokenStream::new();
            for token in list
                .tokens
                .clone()
                .into_iter()
                .chain(std::iter::once(TokenTree::Punct(proc_macro2::Punct::new(
                    ',',
                    proc_macro2::Spacing::Alone,
                ))))
            {
                if matches!(&token, TokenTree::Punct(p) if p.as_char() == ',') {
                    if !predicate {
                        if let Ok(item) = syn::parse2::<Meta>(part.clone()) {
                            self.meta(&item, span);
                        }
                    }
                    predicate = false;
                    part = TokenStream::new();
                } else {
                    part.extend([token]);
                }
            }
            return;
        }
        if !(list.path.is_ident("allow") || list.path.is_ident("expect")) {
            return;
        }
        let valid_reason = Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(list.tokens.clone()).ok().is_some_and(|items| items.iter().any(|item| {
                matches!(item, Meta::NameValue(nv) if nv.path.is_ident("reason")
                    && matches!(&nv.value, Expr::Lit(lit) if matches!(&lit.lit, Lit::Str(s) if !s.value().trim().is_empty())))
            }));
        if !valid_reason {
            self.findings.push(Finding {
                rule: "RS-GOV-04".into(), path: self.path.into(), line: span.start().line,
                scope: self.scopes.join("::"), attribute: meta.to_token_stream().to_string(),
                detail: "suppression needs a non-empty literal reason; explain the invariant or constraint".into(),
            });
        }
    }

    // Macro output is not visited as an AST. Inspect literal attribute token trees;
    // strings/comments containing attribute examples are never treated as code.
    fn macro_tokens(&mut self, tokens: TokenStream) {
        let tokens: Vec<_> = tokens.into_iter().collect();
        for (i, token) in tokens.iter().enumerate() {
            if matches!(token, TokenTree::Punct(p) if p.as_char() == '#') {
                let offset = if matches!(tokens.get(i+1), Some(TokenTree::Punct(p)) if p.as_char() == '!')
                {
                    2
                } else {
                    1
                };
                if let Some(TokenTree::Group(group)) = tokens.get(i + offset) {
                    if group.delimiter() == proc_macro2::Delimiter::Bracket {
                        if let Ok(meta) = syn::parse2::<Meta>(group.stream()) {
                            self.meta(&meta, group.span());
                        }
                    }
                }
            }
            if let TokenTree::Group(group) = token {
                self.macro_tokens(group.stream());
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_field(&mut self, field: &'ast syn::Field) {
        self.scopes.push(format!(
            "field {}",
            field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| field.ty.to_token_stream().to_string())
        ));
        syn::visit::visit_field(self, field);
        self.scopes.pop();
    }
    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        self.scopes.push(variant.ident.to_string());
        syn::visit::visit_variant(self, variant);
        self.scopes.pop();
    }
    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.scopes
            .push(format!("let {}", local.pat.to_token_stream()));
        syn::visit::visit_local(self, local);
        self.scopes.pop();
    }
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let name = match item {
            syn::Item::Fn(x) => x.sig.ident.to_string(),
            syn::Item::Mod(x) => x.ident.to_string(),
            syn::Item::Struct(x) => x.ident.to_string(),
            syn::Item::Enum(x) => x.ident.to_string(),
            syn::Item::Union(x) => x.ident.to_string(),
            syn::Item::Trait(x) => x.ident.to_string(),
            syn::Item::Type(x) => x.ident.to_string(),
            syn::Item::Const(x) => x.ident.to_string(),
            syn::Item::Static(x) => x.ident.to_string(),
            syn::Item::Impl(x) => format!(
                "impl {} {}",
                x.trait_
                    .as_ref()
                    .map(|(_, p, _)| p.to_token_stream().to_string())
                    .unwrap_or_default(),
                x.self_ty.to_token_stream()
            ),
            syn::Item::Macro(x) => x
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| x.mac.path.to_token_stream().to_string()),
            _ => "item".into(),
        };
        self.scopes.push(name);
        syn::visit::visit_item(self, item);
        self.scopes.pop();
    }
    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let name = match item {
            syn::ImplItem::Fn(x) => x.sig.ident.to_string(),
            syn::ImplItem::Const(x) => x.ident.to_string(),
            syn::ImplItem::Type(x) => x.ident.to_string(),
            _ => "impl-item".into(),
        };
        self.scopes.push(name);
        syn::visit::visit_impl_item(self, item);
        self.scopes.pop();
    }
    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        let name = match item {
            syn::TraitItem::Fn(x) => x.sig.ident.to_string(),
            syn::TraitItem::Const(x) => x.ident.to_string(),
            syn::TraitItem::Type(x) => x.ident.to_string(),
            _ => "trait-item".into(),
        };
        self.scopes.push(name);
        syn::visit::visit_trait_item(self, item);
        self.scopes.pop();
    }
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        self.meta(&attr.meta, attr.bracket_token.span.open());
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.macro_tokens(mac.tokens.clone());
    }
}
