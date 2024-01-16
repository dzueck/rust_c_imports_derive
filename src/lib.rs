extern crate proc_macro;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use syn::{self, LitStr};
use syn::parse::{Parse, ParseBuffer};
use syn::{Expr, Type, Visibility, Ident, Token, parse_macro_input};

struct FromDefineStruct {
    visibility: Visibility,
    name: Ident,
    ty: Type,
    path: LitStr,
}

struct FromDefineStructs {
    declerations: Vec<FromDefineStruct>
}

impl Parse for FromDefineStructs {
    fn parse(input: &ParseBuffer<'_>) -> Result<Self, syn::Error> { 
        let mut out = Vec::new();

        // This will parse FromDefineStructs untill there are none left.
        // It assumes that the only thing in the macro are external derives
        while !input.is_empty() {
            let visibility: Visibility = input.parse()?;
            input.parse::<Token![const]>()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let ty: Type = input.parse()?;
            input.parse::<Token![in]>()?;
            let path: LitStr = input.parse()?;
            input.parse::<Token![;]>()?;
            out.push(FromDefineStruct {
                visibility,
                name,
                ty,
                path,
            })
        }
        Ok(Self {declerations: out})
    }
}

struct Value {
    value: Expr,
}

impl Parse for Value {
    fn parse(input: &ParseBuffer<'_>) -> Result<Self, syn::Error> { 
        let value: Expr = input.parse()?;
        
        Ok(Value {
            value
        })
    }
}

// This macro will take input of:
// const CONST_NAME: type in "./path/to/a/header/containing/CONST_NAME";
// It will be converted to:
// const CONST_NAME: type = VALUE OF #define IN THE GIVEN HEADER
//
// exe: 
// const NUM_PROCESSORS: usize in "../include/octopos/mailbox.h";
// converts to:
// const NUM_PROCESSORS: usize = 12;
// if the header contained:
// #define NUM_PROCESSORS 12
//
// Note: The path is based on the root of the crate not the source file location
// Note: This macro will make the source file recompile if the header is updated
// Note: The value after #define will be treated as rust source code. 
//       If it is dependent on another macro or is not valid rust syntax it will not work.
//       It essentially just copy pastes the value after the #define NAME
//       It will work if it is based on other macros if you also include thoes other defines in the macro
//
// Note: You can have multiple of these statments in a macro block, but it must only contain these declerations
// exe:
// from_c_header! {
//     const FIFO_OS_TO_TIMER: &str in "../arch/umode/include/arch/timer.h";
//     const FIFO_TIMER_TO_OS: &str in "../arch/umode/include/arch/timer.h";
//     const FIFO_TIMER_TO_MAILBOX: &str in "../arch/umode/include/arch/timer.h";
//     const FIFO_TIMER_LOG: &str in "../arch/umode/include/arch/timer.h";
// }
#[proc_macro]
pub fn from_c_header(input: TokenStream) -> TokenStream {
    // quote! used to make output a proc_macro2 TokenStream
    let mut output = quote!();

    let decls = parse_macro_input!(input as FromDefineStructs);
    let mut paths = HashMap::new();
    for decl in decls.declerations {
        let path = decl.path.value();

        // We use absolute paths because when reading the file in this macro, it is run in the crate directory, 
        // but the include_bytes! macro will be relative to the source file.
        let absolute_path = Path::new(&path).canonicalize().expect(&format!("Failed to create absolute path from {path}"));
        paths.entry(absolute_path).or_insert(Vec::new()).push(decl);
    }
    for (absolute_path, decls) in paths {
        let mut header = File::open(&absolute_path).unwrap_or_else(|_| {
            panic!("Failed to open file at {}", absolute_path.display());
        });

        let path_str: &str = absolute_path.to_str().expect("Path was not valid unicode");

        let force_recompile_on_header_change = quote! {
            // Include_bytes is here to force a recompile if the header file changes
            // It should be optimized out since it does not do anything
            // This could impact compile times
            // In the future there should be a feature that lets you tell the compiler this instead of using this hack
            const _: &[u8] = include_bytes!(#path_str);
        };

        output.extend(force_recompile_on_header_change);

        let mut header_str = String::new();
        header.read_to_string(&mut header_str).expect(&format!("Failed to read file at {}", absolute_path.display()));
        for FromDefineStruct { visibility, name, ty, path: _} in decls {
            let name_str = name.to_string();

            // Matches #define {name_str} value
            let define_regex = Regex::new(&format!("(^|\n)\\s*#define\\s*{name_str}\\s*(?P<value>.*)")).unwrap();
            let define_match = define_regex.captures(&header_str).unwrap_or_else(|| {
                panic!("Failed to find #define for {name_str} in {}", absolute_path.display());
            });

            let define_str = &define_match["value"];
            let value_tokens: TokenStream = define_str.parse().expect("Failed to convert to Value");


            let Value {
                value,
            } = match syn::parse(value_tokens) {
                Ok(v) => v,
                Err(_) => panic!("Failed to convert define value of: ({}) to type ({}) from #define {name} in {}", define_str, quote!(#ty).to_string(), absolute_path.display()),
            };


            let this_output = quote! {
                #visibility const #name: #ty = #value;
            };

            output.extend(this_output);
        }
    }

    output.into()
}