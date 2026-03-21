//! Errors that must be detected at compile time
use crate::parser::DiagnosticContext;

use super::parser::{Diagnostic, Span};
use annotate_snippets::{AnnotationKind, Group, Level, Patch, Snippet};

impl<'a> DiagnosticContext<'a> {
    pub fn invalid_syntax(&self, span: Span, message: String) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title(message)).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn invalid_token(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("invalid token")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn unterminated_string(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("unterminated string")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn unterminated_comment(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("unterminated comment")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn unexpected_exp_stat(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("unexpected expression kind"))
                    .element(
                        Snippet::source(self.source).path(self.path).annotation(
                            AnnotationKind::Primary
                                .span(span)
                                .label("expected call expression"),
                        ),
                    ),
            ],
        }
    }

    pub fn unexpected_assign_lhs(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("unexpected expression kind"))
                    .element(
                        Snippet::source(self.source).path(self.path).annotation(
                            AnnotationKind::Primary
                                .span(span)
                                .label("expected name, index, or field expression"),
                        ),
                    ),
            ],
        }
    }

    pub fn unexpected_attribute(&self, span: Span, global: bool) -> Diagnostic<'a> {
        let mut groups = vec![
            Group::with_title(Level::ERROR.primary_title("unexpected attribute name")).element(
                Snippet::source(self.source).path(self.path).annotation(
                    AnnotationKind::Primary.span(span.clone()).label(format!(
                        "expected `const`{}",
                        if global { "" } else { " or `close`" }
                    )),
                ),
            ),
            Group::with_title(Level::HELP.secondary_title("replace it with `const`")).element(
                Snippet::source(self.source)
                    .path(self.path)
                    .patch(Patch::new(span.clone(), "const")),
            ),
        ];
        if !global {
            groups.push(
                Group::with_title(Level::HELP.secondary_title("replace it with `close`")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .patch(Patch::new(span.clone(), "close")),
                ),
            );
        }
        Diagnostic {
            error: true,
            groups,
        }
    }

    pub fn undefined_label(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("use of undefined label")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn redefined_label(&self, span: Span, old_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("redefined label")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span))
                        .annotation(
                            AnnotationKind::Context
                                .span(old_span)
                                .label("previous definition"),
                        ),
                ),
            ],
        }
    }

    pub fn break_outside_loop(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("break not inside a loop")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn invalid_escape_sequence(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("invalid escape sequence")).element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }

    pub fn invalid_hex_escape_sequence(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(
                    Level::ERROR.primary_title("invalid hexadecimal escape sequence"),
                )
                .element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
                Group::with_title(
                    Level::NOTE.secondary_title("expected exactly two hexadecimal digits"),
                ),
            ],
        }
    }

    pub fn invalid_unicode_escape_sequence(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("invalid unicode escape sequence"))
                    .element(
                        Snippet::source(self.source)
                            .path(self.path)
                            .annotation(AnnotationKind::Primary.span(span)),
                    ),
            ],
        }
    }

    pub fn invalid_utf8_value(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
            Group::with_title(Level::ERROR.primary_title("UTF-8 value is too large")).element(
                Snippet::source(self.source)
                    .path(self.path)
                    .annotation(AnnotationKind::Primary.span(span)),
            ),
            Group::with_title(
                Level::NOTE.secondary_title("the code point can be any value less than 0x80000000"),
            ),
            Group::with_title(Level::NOTE.secondary_title(
                "the original UTF-8 specification is not restricted to valid Unicode code points",
            )),
        ],
        }
    }

    pub fn goto_skips_local(
        &self,
        span: Span,
        skipped_span: Span,
        local_spans: Vec<Span>,
        next_span: Span,
    ) -> Diagnostic<'a> {
        let mut annotations = vec![
            AnnotationKind::Primary.span(span),
            AnnotationKind::Context
                .span(skipped_span)
                .label("jumped code"),
        ];
        for local_span in local_spans {
            annotations.push(
                AnnotationKind::Context
                    .span(local_span)
                    .label("local declaration"),
            );
        }
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("goto jumps into the scope of local"))
                    .element(
                        Snippet::source(self.source)
                            .path(self.path)
                            .annotations(annotations),
                    ),
                Group::with_title(
                    Level::NOTE
                        .secondary_title("this is only allowed for labels at the end of a block"),
                )
                .element(
                    Snippet::source(self.source).path(self.path).annotation(
                        AnnotationKind::Primary
                            .span(next_span)
                            .label("however label is followed by this statement"),
                    ),
                ),
            ],
        }
    }

    pub fn write_const_variable(
        &self,
        span: Span,
        decl_span: Span,
        loopvar: bool,
    ) -> Diagnostic<'a> {
        let mut groups = vec![
            Group::with_title(
                Level::ERROR.primary_title("attempt to assign to read-only variable"),
            )
            .element(
                Snippet::source(self.source)
                    .path(self.path)
                    .annotation(AnnotationKind::Primary.span(span))
                    .annotation(
                        AnnotationKind::Context
                            .span(decl_span)
                            .label("defined as read-only here"),
                    ),
            ),
        ];
        if loopvar {
            groups.push(Group::with_title(
                Level::NOTE.secondary_title("since Lua 5.5 for-loop variables are read-only"),
            ));
        }
        Diagnostic {
            error: true,
            groups,
        }
    }

    pub fn undeclared_global(&self, span: Span, decl_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(Level::ERROR.primary_title("global variable was not declared"))
                    .element(
                        Snippet::source(self.source)
                            .path(self.path)
                            .annotations(vec![
                                AnnotationKind::Primary.span(span),
                                AnnotationKind::Context
                                    .span(decl_span)
                                    .label("global declaration in surrounding scope"),
                            ]),
                    ),
                Group::with_title(
                    Level::NOTE.secondary_title("explicit globals prevent use of implicit globals"),
                ),
            ],
        }
    }

    pub fn invalid_vararg(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: true,
            groups: vec![
                Group::with_title(
                    Level::ERROR.primary_title("cannot use `...` outside a vararg function"),
                )
                .element(
                    Snippet::source(self.source)
                        .path(self.path)
                        .annotations(vec![AnnotationKind::Primary.span(span)]),
                ),
            ],
        }
    }
}
