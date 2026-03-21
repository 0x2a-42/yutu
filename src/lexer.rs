use crate::lints::*;
use crate::parser::{Diagnostic, DiagnosticContext, Span};
use logos::{Lexer, Logos};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexerError {
    #[default]
    Invalid,
    UnterminatedString,
    UnterminatedComment,
}

impl LexerError {
    pub fn into_diagnostic<'a>(
        self,
        diag_ctx: &DiagnosticContext<'a>,
        span: Span,
    ) -> Diagnostic<'a> {
        match self {
            Self::Invalid => diag_ctx.invalid_token(span),
            LexerError::UnterminatedString => diag_ctx.unterminated_string(span),
            LexerError::UnterminatedComment => diag_ctx.unterminated_comment(span),
        }
    }
}

fn lex_long_string(lexer: &mut Lexer<'_, Token>) -> Result<(), LexerError> {
    let prefix_len = lexer.slice().len();
    let closing = format!("]{}]", "=".repeat(prefix_len - 2));
    lexer
        .remainder()
        .find(&closing)
        .map(|i| lexer.bump(i + prefix_len))
        .ok_or_else(|| {
            lexer.bump(lexer.remainder().len());
            LexerError::UnterminatedString
        })?;
    Ok(())
}

fn lex_short_string(lexer: &mut Lexer<'_, Token>) -> Result<(), LexerError> {
    let closing = lexer.slice().chars().next().unwrap();
    let mut it = lexer.remainder().chars();
    while let Some(c) = it.next() {
        match c {
            c if c == closing => {
                lexer.bump(1);
                return Ok(());
            }
            '\\' => {
                lexer.bump(1);
                if let Some(c) = it.next() {
                    lexer.bump(c.len_utf8());
                }
            }
            c => {
                lexer.bump(c.len_utf8());
            }
        }
    }
    Err(LexerError::UnterminatedString)
}

fn lex_comments(lexer: &mut Lexer<'_, Token>) -> Result<(), LexerError> {
    let prefix_len = lexer.slice().len() - 2;
    let closing = if prefix_len >= 2 {
        format!("]{}]", "=".repeat(prefix_len - 2))
    } else {
        "\n".to_string()
    };
    if lexer
        .remainder()
        .find(&closing)
        .map(|i| lexer.bump(i + closing.len() - if closing.len() == 1 { 1 } else { 0 }))
        .is_some()
    {
        Ok(())
    } else {
        lexer.bump(lexer.remainder().len());
        Err(LexerError::UnterminatedComment)
    }
}

fn lex_first_line_comment(lexer: &mut Lexer<'_, Token>) -> Token {
    if lexer.span().start == 0 {
        if let Some(i) = lexer.remainder().find('\n') {
            lexer.bump(i + 1)
        }
        return Token::FirstLineComment;
    }
    Token::Hash
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Logos, Debug, PartialEq, Copy, Clone)]
#[logos(error = LexerError)]
#[logos(extras = usize)]
pub enum Token {
    EOF,
    #[regex(r"--\[?", lex_comments)]
    #[regex(r"--\[=*\[", lex_comments)]
    Comment,
    #[regex(r"[ \t\r\n\f]+")]
    Whitespace,
    #[token("break")]
    Break,
    #[token("return")]
    Return,
    #[token("function")]
    Function,
    #[token("end")]
    End,
    #[token("goto")]
    Goto,
    #[token("do")]
    Do,
    #[token("while")]
    While,
    #[token("repeat")]
    Repeat,
    #[token("if")]
    If,
    #[token("then")]
    Then,
    #[token("until")]
    Until,
    #[token("elseif")]
    Elseif,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("local")]
    Local,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("nil")]
    Nil,
    #[token("false")]
    False,
    #[token("true")]
    True,
    #[token("global", |lex| if lex.extras == 5 { Token::Global } else { Token::Name } )]
    Global,
    #[token(";")]
    Semi,
    #[token("::")]
    ColonColon,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("...")]
    Dot3,
    #[token("=")]
    Equal,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("//")]
    SlashSlash,
    #[token("^")]
    Hat,
    #[token("%")]
    Percent,
    #[token("&")]
    Ampersand,
    #[token("~")]
    Tilde,
    #[token("|")]
    Pipe,
    #[token(">>")]
    GtGt,
    #[token("<<")]
    LtLt,
    #[token("..")]
    Dot2,
    #[token("<")]
    Less,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    Greater,
    #[token(">=")]
    GreaterEqual,
    #[token("==")]
    EqualEqual,
    #[token("~=")]
    TildeEqual,
    #[token("#", lex_first_line_comment)]
    Hash,
    FirstLineComment,
    #[token("[")]
    LBrak,
    #[token("]")]
    RBrak,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[regex("[a-zA-Z_][0-9a-zA-Z_]*")]
    Name,
    #[regex("\"|'", lex_short_string)]
    #[regex(r"\[=*\[", lex_long_string)]
    LiteralString,
    #[regex(r"[0-9]+")]
    DecIntNumeral,
    #[regex(r"0[xX][a-fA-F0-9]+")]
    HexIntNumeral,
    #[regex(r"[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?")]
    #[regex(r"\.[0-9]+([eE][+-]?[0-9]+)?")]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+")]
    DecFloatNumeral,
    #[regex(r"0[xX][a-fA-F0-9]+\.[a-fA-F0-9]*([pP][+-]?[a-fA-F0-9]+)?")]
    #[regex(r"0[xX]\.[a-fA-F0-9]+([pP][+-]?[a-fA-F0-9]+)?")]
    #[regex(r"0[xX][a-fA-F0-9]+[pP][+-]?[a-fA-F0-9]+")]
    HexFloatNumeral,
    Error,
}

fn check_whitespace<'a>(
    diag_ctx: &DiagnosticContext<'a>,
    value: &str,
    span: &Span,
    diags: &mut Vec<Diagnostic<'a>>,
    last_after_newline: &mut usize,
) {
    let mut after_newline = 0;
    let mut tab_spans = vec![];
    let mut space_spans = vec![];
    for (i, c) in value.char_indices() {
        match c {
            '\n' | '\r' => {
                if after_newline != i {
                    if after_newline != 0 {
                        if let Some(only_whitespace) = diag_ctx.active::<OnlyWhitespace>() {
                            diags.push(
                                only_whitespace.build(span.start + after_newline..span.start + i),
                            )
                        }
                    } else {
                        if let Some(trailing_whitespace) = diag_ctx.active::<TrailingWhitespace>() {
                            diags.push(
                                trailing_whitespace
                                    .build(span.start + after_newline..span.start + i),
                            )
                        }
                    }
                }
                after_newline = i + 1;

                if let Some(line_too_long) = diag_ctx.active::<LineTooLong>() {
                    let line_span = *last_after_newline..span.start + i;
                    let line_length = diag_ctx.source[line_span.clone()].width();
                    if line_length > diag_ctx.config.line_length_threshold {
                        diags.push(line_too_long.build(line_span, line_length));
                    }
                }
                *last_after_newline = span.start + after_newline;

                if let Some(inconsistent_indentation) = diag_ctx.active::<InconsistentIndentation>()
                    && !tab_spans.is_empty()
                    && !space_spans.is_empty()
                {
                    diags.push(inconsistent_indentation.build(&tab_spans, &space_spans));
                }
                tab_spans.clear();
                space_spans.clear();
            }
            ' ' => {
                if let Some(last) = space_spans.last_mut()
                    && last.end == span.start + i
                {
                    last.end += 1;
                } else {
                    space_spans.push(span.start + i..span.start + i + 1);
                }
            }
            '\t' if !space_spans.is_empty() => {
                if let Some(last) = tab_spans.last_mut()
                    && last.end == span.start + i
                {
                    last.end += 1;
                } else {
                    tab_spans.push(span.start + i..span.start + i + 1);
                }
            }
            _ => {}
        }
    }
    if let Some(inconsistent_indentation) = diag_ctx.active::<InconsistentIndentation>()
        && !tab_spans.is_empty()
        && !space_spans.is_empty()
    {
        diags.push(inconsistent_indentation.build(&tab_spans, &space_spans));
    }
}

fn check_invisible_unicode<'a>(
    diag_ctx: &DiagnosticContext<'a>,
    value: &str,
    span: &Span,
    diags: &mut Vec<Diagnostic<'a>>,
    last_after_newline: &mut usize,
) {
    for (i, c) in value.char_indices() {
        if let Some(invisible_characters) = diag_ctx.active::<InvisibleCharacters>()
            && (['\u{200B}', '\u{ad}', '\u{2060}'].contains(&c)
                || ('\u{E000}'..='\u{F8FF}').contains(&c)
                || ('\u{100000}'..='\u{10FFFD}').contains(&c))
        {
            diags
                .push(invisible_characters.build(span.start + i..span.start + i + c.len_utf8(), c));
        }
        if c == '\n' || c == '\r' {
            if let Some(line_too_long) = diag_ctx.active::<LineTooLong>() {
                let line_span = *last_after_newline..span.start + i;
                let line_length = diag_ctx.source[line_span.clone()].width();
                if line_length > diag_ctx.config.line_length_threshold {
                    diags.push(line_too_long.build(line_span, line_length));
                }
            }
            *last_after_newline = span.start + i + 1;
        }
    }
}

fn check_string<'a>(
    diag_ctx: &DiagnosticContext<'a>,
    value: &str,
    span: &Span,
    diags: &mut Vec<Diagnostic<'a>>,
) {
    if value.starts_with('[') {
        // long strings don't support escape sequences
        return;
    }
    let mut it = value.char_indices();
    'outer: while let Some((i, c)) = it.next() {
        match c {
            '\\' => match it.next() {
                Some((
                    _,
                    'a' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '\\' | '"' | '\'' | 'z' | '\n',
                )) => {}
                Some((_, '\r')) => {
                    if let Some((_, c)) = it.next()
                        && c != '\n'
                    {
                        diags.push(
                            diag_ctx
                                .invalid_escape_sequence(span.start + i - 1..span.start + i + 1),
                        );
                        continue;
                    }
                }
                Some((_, 'x')) => {
                    for j in 0..2 {
                        if let Some((_, c)) = it.next()
                            && !c.is_ascii_hexdigit()
                        {
                            diags.push(diag_ctx.invalid_hex_escape_sequence(
                                span.start + i..span.start + i + j + 2,
                            ));
                            continue 'outer;
                        }
                    }
                }
                Some((_, '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9')) => {}
                Some((i, 'u')) => {
                    if let Some((i, c)) = it.next()
                        && c != '{'
                    {
                        diags.push(diag_ctx.invalid_unicode_escape_sequence(
                            span.start + i - 1..span.start + i + 1,
                        ));
                        continue;
                    }
                    let num_start = 4;
                    let mut num_end = num_start;
                    for j in 0..9 {
                        if let Some((_, c)) = it.next() {
                            if c == '}' {
                                break;
                            } else if !c.is_ascii_hexdigit() || j == 8 {
                                diags.push(diag_ctx.invalid_unicode_escape_sequence(
                                    span.start + i - 1..span.start + i + j + 2,
                                ));
                                continue 'outer;
                            }
                            num_end += 1;
                        }
                    }
                    if let Ok(num) = u32::from_str_radix(&value[num_start..num_end], 16) {
                        if num >= 0x8000_0000 {
                            diags.push(
                                diag_ctx.invalid_utf8_value(
                                    span.start + num_start..span.start + num_end,
                                ),
                            );
                        } else if let Some(unicode_code_point_too_large) =
                            diag_ctx.active::<UnicodeCodePointTooLarge>()
                            && num > 0x10FFFF
                        {
                            diags.push(
                                unicode_code_point_too_large
                                    .build(span.start + num_start..span.start + num_end),
                            );
                        } else if let Some(unicode_code_point_is_surrogate) =
                            diag_ctx.active::<UnicodeCodePointIsSurrogate>()
                            && (0xD800..0xE000).contains(&num)
                        {
                            diags.push(
                                unicode_code_point_is_surrogate
                                    .build(span.start + num_start..span.start + num_end),
                            );
                        }
                    }
                }
                Some((i, _)) => {
                    diags.push(
                        diag_ctx.invalid_escape_sequence(span.start + i - 1..span.start + i + 1),
                    );
                }
                None => unreachable!(),
            },
            c => {
                if let Some(non_ascii_literal) = diag_ctx.active::<NonAsciiLiteral>()
                    && !c.is_ascii()
                {
                    diags.push(
                        non_ascii_literal.build(span.start + i..span.start + i + c.len_utf8(), c),
                    );
                }
            }
        }
    }
}

pub fn tokenize<'a>(
    diag_ctx: &DiagnosticContext<'a>,
    source: &'a str,
    diags: &mut Vec<Diagnostic<'a>>,
) -> (Vec<Token>, Vec<Span>) {
    let lexer = Token::lexer_with_extras(source, diag_ctx.config.lua_minor_version);
    let mut tokens = vec![];
    let mut spans = vec![];

    let mut last_after_newline = 0;
    for (token, span) in lexer.spanned() {
        match token {
            Ok(token @ Token::Whitespace) => {
                check_whitespace(
                    diag_ctx,
                    &source[span.clone()],
                    &span,
                    diags,
                    &mut last_after_newline,
                );
                tokens.push(token);
            }
            Ok(token @ Token::LiteralString) => {
                check_string(diag_ctx, &source[span.clone()], &span, diags);
                check_invisible_unicode(
                    diag_ctx,
                    &source[span.clone()],
                    &span,
                    diags,
                    &mut last_after_newline,
                );
                tokens.push(token);
            }
            Ok(token @ Token::Comment) => {
                check_invisible_unicode(
                    diag_ctx,
                    &source[span.clone()],
                    &span,
                    diags,
                    &mut last_after_newline,
                );
                tokens.push(token);
            }
            Ok(token) => {
                tokens.push(token);
            }
            Err(err) => {
                diags.push(err.into_diagnostic(diag_ctx, span.clone()));
                tokens.push(Token::Error);
            }
        }
        spans.push(span);
    }
    (tokens, spans)
}
