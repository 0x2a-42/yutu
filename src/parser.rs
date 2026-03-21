use crate::Config;
use crate::ast::{AstNode, Exp};
use crate::lexer::{Token, tokenize};
use crate::lints::{INFOS, LINT_COUNT};

pub struct Diagnostic<'a> {
    pub error: bool,
    pub groups: Vec<annotate_snippets::Group<'a>>,
}

#[derive(Copy, Clone)]
pub enum Severity {
    Allow,
    Warn,
    Deny,
    Hint,
}

impl TryFrom<&str> for Severity {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "allow" => Ok(Severity::Allow),
            "warn" => Ok(Severity::Warn),
            "deny" => Ok(Severity::Deny),
            "hint" => Ok(Severity::Hint),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Severity::Allow => "allow",
                Severity::Warn => "warn",
                Severity::Deny => "deny",
                Severity::Hint => "hint",
            }
        )
    }
}

pub struct DiagnosticContext<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub levels: [Severity; LINT_COUNT],
    pub config: &'a Config,
}

impl<'a> DiagnosticContext<'a> {
    pub fn new(path: &'a str, source: &'a str, config: &'a Config) -> Self {
        let mut levels = [Severity::Allow; LINT_COUNT];
        for i in 0..LINT_COUNT {
            levels[i] = if let Some(level) = config.levels.get(INFOS[i].code) {
                *level
            } else {
                INFOS[i].level
            };
        }
        Self {
            path,
            source,
            levels,
            config,
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

impl<'a> ParserCallbacks<'a> for Parser<'a> {
    type Diagnostic = Diagnostic<'a>;
    type Context = DiagnosticContext<'a>;

    fn create_tokens(
        context: &mut Self::Context,
        source: &'a str,
        diags: &mut Vec<Self::Diagnostic>,
    ) -> (Vec<Token>, Vec<Span>) {
        tokenize(context, source, diags)
    }

    fn create_diagnostic(&self, span: Span, message: String) -> Self::Diagnostic {
        self.context.invalid_syntax(span, message)
    }

    fn create_node_expstat(&mut self, node: NodeRef, diags: &mut Vec<Self::Diagnostic>) {
        self.cst.children(node).for_each(|c| {
            if let Some(exp) = Exp::cast(&self.cst, c) {
                match exp {
                    Exp::Callexp(_) => {}
                    _ => {
                        diags.push(self.context.unexpected_exp_stat(self.cst.span(c)));
                    }
                }
            }
        });
    }

    fn create_node_assignstat(&mut self, node: NodeRef, diags: &mut Vec<Self::Diagnostic>) {
        self.cst.children(node).for_each(|c| {
            if let Some(exp) = Exp::cast(&self.cst, c) {
                match exp {
                    Exp::Nameexp(_) | Exp::Indexexp(_) | Exp::Fieldexp(_) => {}
                    _ => {
                        diags.push(self.context.unexpected_assign_lhs(self.cst.span(c)));
                    }
                }
            }
        });
    }

    fn predicate_forstat_1(&self) -> bool {
        self.peek(1) == Token::Equal
    }

    fn predicate_pars_1(&self) -> bool {
        self.peek(1) != Token::Dot3
    }

    fn predicate_tableconstructor_1(&self) -> bool {
        self.peek(1) != Token::RBrace
    }

    fn predicate_field_1(&self) -> bool {
        self.peek(1) == Token::Equal
    }

    fn predicate_varargpar_1(&self) -> bool {
        self.context.config.lua_minor_version >= 5
    }

    fn predicate_localvarstat_1(&self) -> bool {
        self.context.config.lua_minor_version >= 5
    }
}
