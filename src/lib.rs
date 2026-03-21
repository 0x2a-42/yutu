pub mod ast;
pub mod cfg;
pub mod config;
pub mod errors;
pub mod lexer;
pub mod lints;
pub mod parser;
pub mod sema;

use crate::config::Config;
use crate::parser::{Diagnostic, DiagnosticContext, Parser};
use crate::sema::Checks;
use std::io::Read;
use std::path::Path;

pub fn read_source(path: &Path) -> std::io::Result<String> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(if std::str::from_utf8(&buf).is_ok() {
        // SAFETY: buf contains valid UTF-8 due to the previous check
        unsafe { String::from_utf8_unchecked(buf) }
    } else {
        // ISO 8859-1
        buf.into_iter().map(|c| c as char).collect()
    })
}

pub fn check_source<'a>(path: &'a str, source: &'a str, config: &'a Config) -> Vec<Diagnostic<'a>> {
    let mut diags = vec![];
    // syntactic analysis
    let cst = Parser::new_with_context(
        source,
        &mut diags,
        DiagnosticContext::new(path, source, config),
    )
    .parse(&mut diags);

    // semantic analysis
    let diag_ctx = DiagnosticContext::new(path, source, config);
    Checks::run(diag_ctx, &cst, &mut diags);

    diags
}
