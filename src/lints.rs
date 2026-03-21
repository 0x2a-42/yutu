use std::collections::BTreeMap;

use crate::lexer::Token;
use crate::parser::{DiagnosticContext, Severity};

use super::parser::{Diagnostic, Span};
use annotate_snippets::{AnnotationKind, Group, Level, Patch, Snippet};
use indoc::indoc;

enum Category {
    Complexity,
    Correctness,
    Pedantic,
    Style,
    Suspicious,
    Restriction,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Category::Complexity => "complexity",
                Category::Correctness => "correctness",
                Category::Pedantic => "pedantic",
                Category::Style => "style",
                Category::Suspicious => "suspicious",
                Category::Restriction => "restriction",
            }
        )
    }
}

pub struct Info {
    message: &'static str,
    pub code: &'static str,
    pub level: Severity,
    category: Category,
    doc_what: &'static str,
    doc_why: &'static str,
    doc_issues: &'static str,
    doc_example: &'static str,
}

impl Info {
    pub fn to_markdown(&self) -> String {
        let mut val = String::new();
        val.push_str("## ");
        val.push_str(self.code);
        val.push('\n');
        val.push_str(match self.level {
            Severity::Allow => "✅ `",
            Severity::Warn => "⚠️ `",
            Severity::Deny => "❌ `",
            Severity::Hint => "ℹ️ `",
        });
        val.push_str(&self.level.to_string());
        val.push_str("` - `");
        val.push_str(&self.category.to_string());
        val.push('`');
        val.push_str("\n### What it does\n");
        val.push_str(self.doc_what);
        val.push_str("\n### Why restrict this?\n");
        val.push_str(self.doc_why);
        if !self.doc_issues.is_empty() {
            val.push_str("\n### Known problems\n");
            val.push_str(self.doc_issues);
        }
        if !self.doc_example.is_empty() {
            val.push_str("\n### Example\n");
            val.push_str(self.doc_example);
        }
        val
    }
}

trait FromLint<'a> {
    fn from_lint(lint: &Info, severity: Severity) -> Group<'a>;
}
impl<'a> FromLint<'a> for Group<'a> {
    fn from_lint(lint: &Info, severity: Severity) -> Group<'a> {
        let level = match severity {
            Severity::Allow => unreachable!(),
            Severity::Warn => Level::WARNING,
            Severity::Deny => Level::ERROR,
            Severity::Hint => Level::INFO,
        };
        Group::with_title(
            level
                .primary_title(lint.message)
                .id(lint.code)
                .id_url(format!(
                    "https://github.com/0x2a-42/yutu/blob/main/lints.md#{}",
                    lint.code
                )),
        )
    }
}

trait Lint<'a, 'b> {
    const INFO_INDEX: usize;
    fn new(ctx: &'b DiagnosticContext<'a>) -> Self;
}

macro_rules! lints {
    { $($i:ident {
            message: $message:expr,
            code:  $code:expr,
            level:  $level:expr,
            category:  $category:expr,
            doc_what:  $doc_what:expr,
            doc_why:  $doc_why:expr,
            doc_issues:  $doc_issues:expr,
            doc_example: $doc_example:expr,
        }),*
    } => {
        enum InfoIndices { $($i,)* Last }
        pub const LINT_COUNT: usize = InfoIndices::Last as usize;
        pub const INFOS: [Info; LINT_COUNT] = [
            $(Info {
                message: $message,
                code:  $code,
                level:  $level,
                category:  $category,
                doc_what:  $doc_what,
                doc_why:  $doc_why,
                doc_issues:  $doc_issues,
                doc_example: $doc_example,
            }),*
        ];
        $(
            pub struct $i<'a, 'b>(&'b DiagnosticContext<'a>);

            impl<'a, 'b> Lint<'a, 'b> for $i<'a, 'b> {
                const INFO_INDEX: usize = InfoIndices::$i as usize;
                fn new(ctx: &'b DiagnosticContext<'a>) -> Self {
                    Self(ctx)
                }
            }
        )*
    }
}

lints! {
    EmptyStatement {
        message: "empty statement",
        code: "empty-statement",
        level: Severity::Warn,
        category: Category::Style,
        doc_what: indoc! {"
            Checks for consecutive semicolons.
        "},
        doc_why: indoc! {"
            This is most likely a typing mistake.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            print(\"hello\");;
            ```
        "},
    },
    UnusedLocal {
        message: "unused local",
        code: "unused-local",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for locals that are never used.
        "},
        doc_why: indoc! {"
            An unused local indicates, that it was either unknowingly unused or later became unused due to a refactoring. \
            It can be safely be removed without changing the semantics of the code.

            The warning can be locally ignored by adding a `_` prefix if `allow_local_unused_hint` is configured as `true`. \
            Otherwise it can also be ignored by using `_` as the name.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local a = 42
            ```
        "},
    },
    TrailingWhitespace {
        message: "line contains trailing whitespace",
        code: "trailing-whitespace",
        level: Severity::Hint,
        category: Category::Style,
        doc_what: indoc! {"
            Checks for trailing whitespaces in a line.
        "},
        doc_why: indoc! {"
            Trailing whitespaces serve no purpose. \
            They are most likely added due to a typing or editing mistake.
        "},
        doc_issues: "",
        doc_example: "",
    },
    OnlyWhitespace {
        message: "line contains only whitespace",
        code: "only-whitespace",
        level: Severity::Hint,
        category: Category::Style,
        doc_what: indoc! {"
            Checks if a line only contains whitespaces.
        "},
        doc_why: indoc! {"
            Lines with only whitespaces serve no purpose. \
            They are most likely added due to a typing or editing mistake.
        "},
        doc_issues: "",
        doc_example: "",
    },
    LowerCaseGlobal {
        message: "global variable in lower-case initial",
        code: "lower-case-global",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for global variables with lower-case initial letter.
        "},
        doc_why: indoc! {"
            By convention in Lua globals start with an upper-case letter.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            a = 42
            ```
        "},
    },
    EmptyBlock {
        message: "empty block in control flow statement",
        code: "empty-block",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks if a block contains no statements.
        "},
        doc_why: indoc! {"
            It usually makes sense to at least explain why a block is empty. \
            Otherwise it could indicate that this was a mistake.

            The warning can be suppressed by adding a comment inside the block.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            if a then
            else
                print(42)
            end
            ```
        "},
    },
    UnnecessaryNegation {
        message: "negation of relational expression can be simplified",
        code: "unnecessary-negation",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks for combinations of negations and relational expressions which can be simplified.
        "},
        doc_why: indoc! {"
            This makes the code more readable.
        "},
        doc_issues: indoc! {"
            If one operand is a NaN value the simplification is not always correct.
        "},
        doc_example: indoc! {"
            ```lua
            if not (a > b) then
                -- do something
            end
            ```
            Use this code instead.
            ```lua
            if a <= b then
                -- do something
            end
            ```
        "},
    },
    ErrorProneNegation {
        message: "negation is executed before relational operator",
        code: "error-prone-negation",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for combinations of negations and relational expressions which are likely unintended.
        "},
        doc_why: indoc! {"
            Negation has a higher precedence than binary operators. \
            Omitting parentheses is likely a mistake, as boolean expressions usually require no comparisons.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            if not a > b then
                -- do something
            end
            ```
        "},
    },
    UnusedLabel {
        message: "unused label",
        code: "unused-label",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for labels that are never used by a `goto` statement.
        "},
        doc_why: indoc! {"
            This is likely due to a mistake or refactoring. \
            The label can be removed without changing the semantics of the code.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            function loop()
                local i = 0;
                ::foo::
                print(i)
                i = i + 1
                if i == 100 then
                    -- forgot to use label
                end
                goto foo
                ::bar:: -- unused label
                return
            end
            ```
        "},
    },
    UsedDespiteUnusedHint {
        message: "used declaration with unused hint",
        code: "used-despite-unused-hint",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks if a declaration with an unused hint (`_` prefix) was used.
        "},
        doc_why: indoc! {"
            If a variable is actually used, the hint should be removed, so mistakes in later changes can be detected.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _a = 42
            print(_a)
            ```
        "},
    },
    UnusedParameter {
        message: "unused parameter",
        code: "unused-parameter",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for parameters that are never used.
        "},
        doc_why: indoc! {"
            An unused parameter indicates, that it was either unknowingly unused or later became unused due to a refactoring.

            The warning can be locally ignored by adding a `_` prefix.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            function foo(a, b)
                print(a)
            end
            ```
        "},
    },
    UnusedLoopvar {
        message: "unused loop variable",
        code: "unused-loopvar",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for loop variables that are never used.
        "},
        doc_why: indoc! {"
            An unused loop variable indicates, that it was either unknowingly unused or later became unused due to a refactoring.

            The warning can be locally ignored by adding a `_` prefix if `allow_loopvar_unused_hint` is configured as `true`. \
            Otherwise it can also be ignored by using `_` as the name.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            for i = 0, 10 do
                print(42)
            end
            ```
        "},
    },
    RedefinedLocal {
        message: "redefined local",
        code: "redefined-local",
        level: Severity::Allow,
        category: Category::Pedantic,
        doc_what: indoc! {"
            Checks for redefinitions of local variables.
        "},
        doc_why: indoc! {"
            Redefinitions of local variables can make it harder to understand the code.
        "},
        doc_issues: indoc! {"
            There are commonly used patterns that will result in warnings.

            ```lua
            local val, err = foo();
            if err then
                print(err)
            end

            local val, err = bar(); -- redefined local
            if err then
                print(err)
            end
            ```
        "},
        doc_example: indoc! {"
            ```lua
            local a = 42
            print(a)

            local a = 100 -- redefined local
            print(a)
            ```
        "},
    },
    RedundantLocal {
        message: "redundant redefinition of a local",
        code: "redundant-local",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for redundant redefinitions of local variables.
        "},
        doc_why: indoc! {"
            Redundant redefinitions of local variables have no effect and are thus likely to be unintended.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local a = 0;
            local a = a;
            ```
        "},
    },
    ShadowingLocal {
        message: "shadowing local",
        code: "shadowing-local",
        level: Severity::Allow,
        category: Category::Pedantic,
        doc_what: indoc! {"
            Checks for locals that shadow locals in a surrounding scope.
        "},
        doc_why: indoc! {"
            This can lead to confusion, when one tries to change the other variable in the inner scope.
        "},
        doc_issues: indoc! {"
            Like with [redefined-local](#redefined-local) there are some commonly used patterns that will result in warnings.
        "},
        doc_example: indoc! {"
            ```lua
            local a = 0
            if b then
                -- ...
                local a = 0 -- shadowing local
                -- ...
                a = 100
            end
            print(a)
            ```
        "},
    },
    UnusedVararg {
        message: "unused variable length argument",
        code: "unused-vararg",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for unused variable length arguments.
        "},
        doc_why: indoc! {"
            This is likely a mistake, as there is otherwise no reason to add the `...` parameter.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            function foo(a, ...)
                print(a)
            end
            ```
        "},
    },
    UnbalancedAssignment {
        message: "unexpected number of expressions on right side of assignment",
        code: "unbalanced-assignment",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks if the left and right side of an assignment have the same number of expressions.
        "},
        doc_why: indoc! {"
            Extra left-hand side values will be assigned `nil` which might be unintended. \
            Extra right-hand side values will be ignored which indicates a mistake.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            A, B = 42 -- B is assigned nil
            C, D = 1, 2, 3 -- 3 is ignored
            ```
        "},
    },
    UnbalancedInitialization {
        message: "unexpected number of expressions on right side of initialization",
        code: "unbalanced-initialization",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks if the left and right side of an assignment have the same number of names and expressions.
        "},
        doc_why: indoc! {"
            Extra left-hand side values will be assigned `nil` which might be unintended. \
            Extra right-hand side values will be ignored which indicates a mistake.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local a, b = 42 -- b is assigned nil
            local c, d = 1, 2, 3 -- 3 is ignored
            ```
        "},
    },
    TooManyParameters {
        message: "function has too many parameters",
        code: "too-many-parameters",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks if the number of function parameters exceeds the threshold configured in `parameter_threshold`.
        "},
        doc_why: indoc! {"
            Functions with too many parameters can be hard to understand.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            function foo(a, b, c, d, e, f, g, h, i, j)
                print(a, b, c, d, e, f, g, h, i, j)
            end
            ```
        "},
    },
    TooManyLines {
        message: "function contains too many lines",
        code: "too-many-lines",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks if the number of lines in a function exceeds the threshold configured in `function_line_threshold`.
        "},
        doc_why: indoc! {"
            Functions with too many lines can be hard to understand.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            function foo()
                local a
                -- 1000 more lines which may modify a
                print(a)
            end
            ```
        "},
    },
    AlmostSwap {
        message: "code sequence almost implements a swap",
        code: "almost-swap",
        level: Severity::Deny,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks for code that almost implements a swap operation.
        "},
        doc_why: indoc! {"
            This is most likely a mistake as the second assignment serves no purpose.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            The following code does not swap `a` and `b`.
            ```lua
            a = b
            b = a
            ```
            Use this code instead.
            ```lua
            a, b = b, a
            ```
        "},
    },
    ApproxPi {
        message: "numeric literal is approximatly pi",
        code: "approx-pi",
        level: Severity::Warn,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks for floating point literals that approximate pi (π), which is already defined in `math`.
        "},
        doc_why: indoc! {"
            Usually the standard library definition is more precise.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local radius = 42
            local area = 3.141 * radius ^ 2
            ```
            Use this code instead.
            ```lua
            local radius = 42
            local area = math.pi * radius ^ 2
            ```
        "},
    },
    RoundsToInf {
        message: "numeric literal rounds to infinity",
        code: "rounds-to-inf",
        level: Severity::Warn,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks if the value of a numeric literal is so large that it would be rounded to infinity.
        "},
        doc_why: indoc! {"
            Using the standard library definition is more clear.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local inf = 2e1000
            ```
            Use this code instead.
            ```lua
            local inf = math.huge
            ```
        "},
    },
    RoundsIntPart {
        message: "integral part of numeric literal will be rounded",
        code: "rounds-int-part",
        level: Severity::Warn,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks if the value of a numeric literal is too large for its integral part to be represented exactly as a 64 bit IEEE-754 float value.
        "},
        doc_why: indoc! {"
            This is very likely unintended behavior and may result in logic bugs.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local a = 100000000000000000000000 -- the actual value is 99999999999999991611392.0
            local b = 100000000000000000000001 -- rounded to same value
            print(a < b) -- false
            ```
        "},
    },
    InexactHexFloat {
        message: "cannot exactly represent hexadecimal float in 64 bit",
        code: "inexact-hex-float",
        level: Severity::Warn,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks if a hexadecimal float literal can be represented exactly as a 64 bit IEEE-754 float value.
        "},
        doc_why: indoc! {"
            This is very likely unintended behavior, as the main use case of hexadecimal float literals is to exactly specify values.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _ = 0x1.p9999
            ```
        "},
    },
    OctalConfusion {
        message: "zero prefixed integer can be confused as octal number",
        code: "octal-confusion",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks if a decimal integer literal has a leading zero.
        "},
        doc_why: indoc! {"
            In C such literals are octal numbers, so some people may expect the same to be true in Lua. \
            As there is no use for such a prefix, it can safely be removed to avoid confusion.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _ = 042
            ```
        "},
    },
    HexIntOverflow {
        message: "overflow in integer literal",
        code: "hex-int-overflow",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks if a hexadecimal integer literal is too large for a signed 64 bit integer value.
        "},
        doc_why: indoc! {"
            In Lua hexadecimal integer literals are truncated if they are too large.
        "},
        doc_issues: "",
        doc_example: indoc!{"
            ```lua
            local _ = 0x10000000000000000 -- actual value is 0
            ```
        "},
    },
    BoolCompare {
        message: "comparison with bool constant can be simplfied",
        code: "bool-compare",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks if a boolean value is compared to a boolean literal.
        "},
        doc_why: indoc! {"
            It is usually clearer to just use the boolean value or its negation.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local is_ok = true
            if is_ok == true then
                -- do something
            end
            ```
            Use this code instead.
            ```lua
            local is_ok = true
            if is_ok then
                -- do something
            end
            ```
        "},
    },
    InvisibleCharacters {
        message: "code contains invisible Unicode characters",
        code: "invisible-characters",
        level: Severity::Deny,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for invisible Unicode characters in the code.
        "},
        doc_why: indoc! {"
            There is no valid use case for invisible Unicode characters in your code.
        "},
        doc_issues: "",
        doc_example: "",
    },
    UnreachableCode {
        message: "unreachable code",
        code: "unreachable-code",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks for code that can never be reached during execution.
        "},
        doc_why: indoc! {"
            Unreachable code can be removed without changing the semantics of the code.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            goto bar
            print(\"foo\") -- unreachable code
            ::bar::
            print(\"bar\")
            ```
        "},
    },
    NextLineArgs {
        message: "arguments of called function start in next line",
        code: "next-line-args",
        level: Severity::Warn,
        category: Category::Suspicious,
        doc_what: indoc! {"
            Checks if the argument list of a function calls start in the next line.
        "},
        doc_why: indoc! {"
            Lua requires no semicolons between statements, so some opening parentheses can unexpectedly be interpreted as the start of the argument list of a function call.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            a = b + c
            (print or io.write)('done')
            ```
        "},
    },
    UnicodeCodePointTooLarge {
        message: "Unicode code point is too large",
        code: "unicode-code-point-too-large",
        level: Severity::Warn,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks for Unicode escape sequences with values larger than `0x10FFFF`.
        "},
        doc_why: indoc! {"
            Lua allows such invalid Unicode code points. \
            As these are however not mapped to a valid Unicode scalar value they should usually be avoided.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _ = \"\\u{110000}\"
            ```
        "},
    },
    UnicodeCodePointIsSurrogate {
        message: "Unicode code point is a surrogate",
        code: "unicode-code-point-is-surrogate",
        level: Severity::Warn,
        category: Category::Correctness,
        doc_what: indoc! {"
            Checks for Unicode escape sequences with values between `0xD800` and `0xDFFF`.
        "},
        doc_why: indoc! {"
            Lua allows unpaired surrogates. \
            As these are invalid Unicode scalar values they should usually be avoided.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _ = \"\\u{D800}\"
            ```
        "},
    },
    NonAsciiLiteral {
        message: "string literal contains non-ASCII character",
        code: "non-ascii-literal",
        level: Severity::Allow,
        category: Category::Restriction,
        doc_what: indoc! {"
            Checks for non-ASCII characters in string literals.
        "},
        doc_why: indoc! {"
            Some editors may not work well with certain Unicode symbols, so using escape sequences instead could be useful.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _ = \"€\"
            ```
            Use this code instead.
            ```lua
            local _ = \"\\u{20ac}\"
            ```
        "},
    },
    CyclomaticComplexity {
        message: "cyclomatic complexity of function is too high",
        code: "cyclomatic-complexity",
        level: Severity::Warn,
        category: Category::Restriction,
        doc_what: indoc! {"
            Checks if the cyclomatic complexity of a function exceeds the threshold configured in `cyclomatic_complexity_threshold`.
        "},
        doc_why: indoc! {"
            Functions with high cyclomatic complexity can be hard to understand and may be candidates for a refactoring.
        "},
        doc_issues: indoc! {"
            Due to missing switch statements Lua code sometimes requires long `if`-`elseif` chains. \
            Such chains can be easy to understand, if the structure is very regular, but they would still result in a high cyclomatic complexity.
        "},
        doc_example: indoc! {"
            ```lua
            function foo()
                if x1 == 0 then
                    -- do something
                end
                if x2 == 0 then
                    -- do something
                end
                -- ...
                if x100 == 0 then
                    -- do something
                end
            end
            ```
        "},
    },
    RedundantParentheses {
        message: "expression contains redundant parentheses",
        code: "redundant-parentheses",
        level: Severity::Warn,
        category: Category::Complexity,
        doc_what: indoc! {"
            Checks for parentheses inside of parentheses.
        "},
        doc_why: indoc! {"
            Double parentheses indicate that there might be a mistake. \
            They can be removed without changing the semantics of the code.
        "},
        doc_issues: "",
        doc_example: indoc! {"
            ```lua
            local _ = ((20 + 1)) * 2
            ```
        "},
    },
    LineTooLong {
        message: "",
        code: "line-too-long",
        level: Severity::Warn,
        category: Category::Restriction,
        doc_what: indoc! {"
            Checks if the number of columns in a line exceeds the threshold configured in `line_length_threshold `.
        "},
        doc_why: indoc! {"
            Lines that are to long are hard to understand.
        "},
        doc_issues: "",
        doc_example: "",
    },
    InconsistentIndentation {
        message: "indentation contains tabs after spaces",
        code: "inconsistent-indentation",
        level: Severity::Warn,
        category: Category::Restriction,
        doc_what: indoc! {"
            Checks for tabs after spaces.
        "},
        doc_why: indoc! {"
            Using tabs after spaces is not useful.
        "},
        doc_issues: "",
        doc_example: "",
    }
}

impl<'a> TrailingWhitespace<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
            ],
        }
    }
}

impl<'a> OnlyWhitespace<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
            ],
        }
    }
}

impl<'a> LowerCaseGlobal<'a, '_> {
    pub fn build(&self, span: Span, sub_exp: bool, explict: bool) -> Diagnostic<'a> {
        let mut groups = vec![
            self.0.main_group(Self::INFO_INDEX).element(
                Snippet::source(self.0.source).path(self.0.path).annotation(
                    AnnotationKind::Primary
                        .span(span.clone())
                        .label(if !sub_exp {
                            Some("did you miss a `local` or misspell it?")
                        } else {
                            None
                        }),
                ),
            ),
            Group::with_title(Level::HELP.secondary_title("start name with upper-case letter")),
        ];
        if !explict {
            groups.push(Group::with_title(
                Level::HELP.secondary_title("mark variable as global in yutu.toml"),
            ));
        }
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups,
        }
    }
}

impl<'a> EmptyBlock<'a, '_> {
    pub fn build(&self, span: Span, block_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::HELP
                        .secondary_title("add a comment inside the block to suppress this warning"),
                )
                .element(Snippet::source(self.0.source).path(self.0.path).patch(
                    Patch::new(
                        block_span.clone(),
                        " --[[ TODO: explain why this block is empty ]] ",
                    ),
                )),
            ],
        }
    }
}

impl<'a> EmptyStatement<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }
}

impl<'a> UnnecessaryNegation<'a, '_> {
    pub fn build(
        &self,
        op_span: Span,
        bin_op_tok: Token,
        bin_op_span: Span,
        cmp_span: Span,
        paren_span: Span,
    ) -> Diagnostic<'a> {
        let inv_op = match bin_op_tok {
            Token::Less => ">=",
            Token::Greater => "<=",
            Token::LessEqual => ">",
            Token::GreaterEqual => "<",
            Token::TildeEqual => "==",
            Token::EqualEqual => "~=",
            _ => unreachable!(),
        };
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(op_span.clone()))
                        .annotation(
                            AnnotationKind::Context
                                .span(bin_op_span.clone())
                                .label("for this relational operator"),
                        ),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("use the inverse relational operator instead"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(bin_op_span.clone(), inv_op))
                        .patch(Patch::new(op_span.start..cmp_span.start, ""))
                        .patch(Patch::new(cmp_span.end..paren_span.end, "")),
                ),
            ],
        }
    }
}

impl<'a> ErrorProneNegation<'a, '_> {
    pub fn build(
        &self,
        un_span: Span,
        op_span: Span,
        bin_span: Span,
        bin_op_span: Span,
    ) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(op_span.clone()))
                        .annotation(
                            AnnotationKind::Context
                                .span(bin_op_span)
                                .label("for this relational operator"),
                        ),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("use parentheses to clarify your intent"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(op_span.start..op_span.start, "("))
                        .patch(Patch::new(op_span.end..op_span.end, ")")),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("invert the result of the relational operation"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(un_span.start..un_span.start, "("))
                        .patch(Patch::new(bin_span.end..bin_span.end, ")")),
                ),
            ],
        }
    }
}

impl<'a> UnusedLabel<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }
}

impl<'a> UnusedLocal<'a, '_> {
    pub fn build(&self, span: Span, delete_span: Option<Span>) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                if let Some(delete_span) = delete_span {
                    Group::with_title(Level::HELP.secondary_title("delete the definition")).element(
                        Snippet::source(self.0.source)
                            .path(self.0.path)
                            .patch(Patch::new(delete_span, "")),
                    )
                } else {
                    if self.0.config.allow_local_unused_hint {
                        Group::with_title(
                            Level::HELP.secondary_title(
                                "use an underscore prefix to suppress this warning",
                            ),
                        )
                        .element(
                            Snippet::source(self.0.source)
                                .path(self.0.path)
                                .patch(Patch::new(span.start..span.start, "_")),
                        )
                    } else {
                        Group::with_title(
                            Level::HELP
                                .secondary_title("use an underscore to suppress this warning"),
                        )
                        .element(
                            Snippet::source(self.0.source)
                                .path(self.0.path)
                                .patch(Patch::new(span, "_")),
                        )
                    }
                },
            ],
        }
    }
}

impl<'a> UsedDespiteUnusedHint<'a, '_> {
    pub fn build(&self, span: Span, decl_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone()))
                        .annotation(
                            AnnotationKind::Context
                                .span(decl_span.clone())
                                .label("declared here"),
                        ),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("remove the underscore prefix in the name"),
                ),
            ],
        }
    }
}

impl<'a> UnusedParameter<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::HELP
                        .secondary_title("use an underscore prefix to suppress this warning"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span.start..span.start, "_")),
                ),
            ],
        }
    }
}

impl<'a> UnusedLoopvar<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                if self.0.config.allow_loopvar_unused_hint {
                    Group::with_title(
                        Level::HELP
                            .secondary_title("use an underscore prefix to suppress this warning"),
                    )
                    .element(
                        Snippet::source(self.0.source)
                            .path(self.0.path)
                            .patch(Patch::new(span.start..span.start, "_")),
                    )
                } else {
                    Group::with_title(
                        Level::HELP.secondary_title("use an underscore to suppress this warning"),
                    )
                    .element(
                        Snippet::source(self.0.source)
                            .path(self.0.path)
                            .patch(Patch::new(span, "_")),
                    )
                },
            ],
        }
    }
}

impl<'a> RedefinedLocal<'a, '_> {
    pub fn build(&self, span: Span, old_span: Span, is_self: bool) -> Diagnostic<'a> {
        let mut groups = vec![
            self.0.main_group(Self::INFO_INDEX).element(
                Snippet::source(self.0.source)
                    .path(self.0.path)
                    .annotation(AnnotationKind::Primary.span(span))
                    .annotation(
                        AnnotationKind::Context
                            .span(old_span)
                            .label("previous definition in same scope"),
                    ),
            ),
        ];
        if is_self {
            groups.push(Group::with_title(
                Level::NOTE.secondary_title("methods define an implicit `self` parameter"),
            ));
        }
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups,
        }
    }
}

impl<'a> RedundantLocal<'a, '_> {
    pub fn build(&self, span: Span, rhs_span: Span, old_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span))
                        .annotation(AnnotationKind::Primary.span(rhs_span))
                        .annotation(
                            AnnotationKind::Context
                                .span(old_span)
                                .label("previous definition"),
                        ),
                ),
            ],
        }
    }
}

impl<'a> ShadowingLocal<'a, '_> {
    pub fn build(&self, span: Span, old_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span))
                        .annotation(
                            AnnotationKind::Context
                                .span(old_span)
                                .label("previous definition in outer scope"),
                        ),
                ),
            ],
        }
    }
}

impl<'a> UnusedVararg<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }
}

impl<'a> UnbalancedAssignment<'a, '_> {
    pub fn build(
        &self,
        lhs_spans: &[Span],
        rhs_spans: &[Span],
        equal_span: Span,
    ) -> Diagnostic<'a> {
        let lhs_count = lhs_spans.len();
        let rhs_count = rhs_spans.len();
        let lhs_span = lhs_spans[0].start..lhs_spans[lhs_count - 1].end;
        let rhs_span = rhs_spans[0].start..rhs_spans[rhs_count - 1].end;
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(equal_span))
                        .annotation(AnnotationKind::Context.span(lhs_span).label(format!(
                            "found {lhs_count} expression{}",
                            if lhs_count > 1 { "s" } else { "" }
                        )))
                        .annotation(AnnotationKind::Context.span(rhs_span).label(format!(
                            "found {rhs_count} expression{}",
                            if rhs_count > 1 { "s" } else { "" }
                        ))),
                ),
                Group::with_title(Level::NOTE.secondary_title(if lhs_count > rhs_count {
                    format!(
                        "extra left-hand side value{} will be assigned `nil`",
                        if lhs_count - rhs_count > 1 { "s" } else { "" }
                    )
                } else {
                    format!(
                        "extra right-hand side value{} will be redundant",
                        if rhs_count - lhs_count > 1 { "s" } else { "" }
                    )
                }))
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotations(if lhs_count > rhs_count {
                            lhs_spans
                                .iter()
                                .skip(rhs_count)
                                .map(|span| AnnotationKind::Primary.span(span.clone()))
                                .collect::<Vec<_>>()
                        } else {
                            rhs_spans
                                .iter()
                                .skip(lhs_count)
                                .map(|span| AnnotationKind::Primary.span(span.clone()))
                                .collect::<Vec<_>>()
                        }),
                ),
            ],
        }
    }
}

impl<'a> UnbalancedInitialization<'a, '_> {
    pub fn build(
        &self,
        lhs_spans: &[Span],
        rhs_spans: &[Span],
        equal_span: Span,
    ) -> Diagnostic<'a> {
        let lhs_count = lhs_spans.len();
        let rhs_count = rhs_spans.len();
        let lhs_span = lhs_spans[0].start..lhs_spans[lhs_count - 1].end;
        let rhs_span = rhs_spans[0].start..rhs_spans[rhs_count - 1].end;
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(equal_span))
                        .annotation(AnnotationKind::Context.span(lhs_span).label(format!(
                            "found {lhs_count} name{}",
                            if lhs_count > 1 { "s" } else { "" }
                        )))
                        .annotation(AnnotationKind::Context.span(rhs_span).label(format!(
                            "found {rhs_count} expression{}",
                            if rhs_count > 1 { "s" } else { "" }
                        ))),
                ),
                Group::with_title(Level::NOTE.secondary_title(if lhs_count > rhs_count {
                    format!(
                        "extra left-hand side variable{} will be assigned `nil`",
                        if lhs_count - rhs_count > 1 { "s" } else { "" }
                    )
                } else {
                    format!(
                        "extra right-hand side value{} will be redundant",
                        if rhs_count - lhs_count > 1 { "s" } else { "" }
                    )
                }))
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotations(if lhs_count > rhs_count {
                            lhs_spans
                                .iter()
                                .skip(rhs_count)
                                .map(|span| AnnotationKind::Primary.span(span.clone()))
                                .collect::<Vec<_>>()
                        } else {
                            rhs_spans
                                .iter()
                                .skip(lhs_count)
                                .map(|span| AnnotationKind::Primary.span(span.clone()))
                                .collect::<Vec<_>>()
                        }),
                ),
            ],
        }
    }
}

impl<'a> UnreachableCode<'a, '_> {
    pub fn build(&self, spans: BTreeMap<usize, Span>) -> Diagnostic<'a> {
        let annotations: Vec<_> = spans
            .into_values()
            .map(|span| AnnotationKind::Primary.span(span))
            .collect();
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotations(annotations),
                ),
            ],
        }
    }
}

impl<'a> AlmostSwap<'a, '_> {
    pub fn build(&self, span: Span, first: &str, second: &str) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::HELP
                        .secondary_title("use a single statement instead to implement a swap"),
                )
                .element(Snippet::source(self.0.source).path(self.0.path).patch(
                    Patch::new(span, format!("{first}, {second} = {second}, {first}")),
                )),
            ],
        }
    }
}

impl<'a> ApproxPi<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("use math.pi instead if this was intended"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span, "math.pi")),
                ),
            ],
        }
    }
}

impl<'a> RoundsToInf<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("use math.huge instead if this was intended"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span, "math.huge")),
                ),
            ],
        }
    }
}

impl<'a> RoundsIntPart<'a, '_> {
    pub fn build(&self, span: Span, value: f64) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source).path(self.0.path).annotation(
                        AnnotationKind::Primary
                            .span(span)
                            .label(format!("the actual value will be {value:.1}")),
                    ),
                ),
            ],
        }
    }
}

impl<'a> InexactHexFloat<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
            ],
        }
    }
}

impl<'a> OctalConfusion<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::NOTE.secondary_title("Lua doesn't have the octal numbers syntax from C"),
                ),
                Group::with_title(Level::HELP.secondary_title("remove the prefix")).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span.start..span.start + 1, "")),
                ),
            ],
        }
    }
}

impl<'a> HexIntOverflow<'a, '_> {
    pub fn build(&self, span: Span, value: i64) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source).path(self.0.path).annotation(
                        AnnotationKind::Primary
                            .span(span)
                            .label(format!("the actual value will be {value}")),
                    ),
                ),
                Group::with_title(Level::NOTE.secondary_title(
                    "hexadecimal integer literals are not implicitly converted to float values",
                )),
            ],
        }
    }
}

impl<'a> TooManyParameters<'a, '_> {
    pub fn build(&self, span: Span, count: usize) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
                Group::with_title(Level::NOTE.secondary_title(format!(
                    "there are {count} parameters and the threshold is set to {}",
                    self.0.config.parameter_threshold
                ))),
            ],
        }
    }
}

impl<'a> NextLineArgs<'a, '_> {
    pub fn build(&self, span: Span, func_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone()))
                        .annotation(
                            AnnotationKind::Context
                                .span(func_span)
                                .label("for call to this function"),
                        ),
                ),
                Group::with_title(Level::HELP.secondary_title(
                    "add a semicolon if this was not intended to be a function call",
                ))
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span.start..span.start, ";")),
                ),
            ],
        }
    }
}

impl<'a> TooManyLines<'a, '_> {
    pub fn build(&self, span: Span, count: usize) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
                Group::with_title(Level::NOTE.secondary_title(format!(
                    "there are {count} lines and the threshold is set to {}",
                    self.0.config.function_line_threshold
                ))),
            ],
        }
    }
}

impl<'a> BoolCompare<'a, '_> {
    pub fn build(
        &self,
        span: Span,
        non_const_span: Span,
        inverse: bool,
        is_bin_exp: bool,
    ) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(Level::HELP.secondary_title("remove the comparison")).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(
                            span.start..span.start,
                            if inverse {
                                if is_bin_exp { "not (" } else { "not " }
                            } else {
                                ""
                            },
                        ))
                        .patch(Patch::new(
                            if span.start == non_const_span.start {
                                non_const_span.end..span.end
                            } else {
                                span.start..non_const_span.start
                            },
                            "",
                        ))
                        .patch(Patch::new(
                            span.end..span.end,
                            if inverse {
                                if is_bin_exp { ")" } else { "" }
                            } else {
                                ""
                            },
                        )),
                ),
            ],
        }
    }
}

impl<'a> InvisibleCharacters<'a, '_> {
    pub fn build(&self, span: Span, c: char) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source).path(self.0.path).annotation(
                        AnnotationKind::Primary
                            .span(span)
                            .label(format!("character {c:?} at this position")),
                    ),
                ),
                Group::with_title(
                    Level::NOTE.secondary_title(
                        "there is no good reason for using such characters in code",
                    ),
                ),
            ],
        }
    }
}

impl<'a> UnicodeCodePointTooLarge<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
                Group::with_title(Level::NOTE.secondary_title(
                    "Lua allows invalid Unicode code points larger than 0x10FFFF",
                )),
            ],
        }
    }
}

impl<'a> UnicodeCodePointIsSurrogate<'a, '_> {
    pub fn build(&self, span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span)),
                ),
                Group::with_title(Level::NOTE.secondary_title(
                    "Lua allows invalid Unicode code points between 0xD800 and 0xDFFF",
                )),
            ],
        }
    }
}

impl<'a> NonAsciiLiteral<'a, '_> {
    pub fn build(&self, span: Span, c: char) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(
                    Level::HELP.secondary_title("rewrite the character with an escape sequence"),
                )
                .element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span, format!("\\u{{{:x}}}", c as u32))),
                ),
            ],
        }
    }
}

impl<'a> LineTooLong<'a, '_> {
    pub fn build(&self, span: Span, length: usize) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(Level::NOTE.secondary_title(format!(
                    "line has {length} columns and the threshold is set to {}",
                    self.0.config.line_length_threshold
                ))),
            ],
        }
    }
}

impl<'a> CyclomaticComplexity<'a, '_> {
    pub fn build(&self, span: Span, complexity: usize) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(Level::NOTE.secondary_title(format!(
                    "function has complexity {complexity} and the threshold is set to {}",
                    self.0.config.cyclomatic_complexity_threshold
                ))),
            ],
        }
    }
}

impl<'a> RedundantParentheses<'a, '_> {
    pub fn build(&self, span: Span, inner_span: Span) -> Diagnostic<'a> {
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotation(AnnotationKind::Primary.span(span.clone())),
                ),
                Group::with_title(Level::HELP.secondary_title("remove extra parentheses")).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .patch(Patch::new(span.start..inner_span.start, ""))
                        .patch(Patch::new(inner_span.end..span.end, "")),
                ),
            ],
        }
    }
}

impl<'a> InconsistentIndentation<'a, '_> {
    pub fn build(&self, tab_spans: &[Span], space_spans: &[Span]) -> Diagnostic<'a> {
        let tab_annotations = tab_spans.iter().map(|span| {
            AnnotationKind::Primary
                .span(span.clone())
                .label(if span.len() > 1 { "tabs" } else { "tab" })
        });
        let space_annotations = space_spans.iter().map(|span| {
            AnnotationKind::Context
                .span(span.clone())
                .label(if span.len() > 1 { "spaces" } else { "space" })
        });
        Diagnostic {
            error: self.0.is_error(Self::INFO_INDEX),
            groups: vec![
                self.0.main_group(Self::INFO_INDEX).element(
                    Snippet::source(self.0.source)
                        .path(self.0.path)
                        .annotations(tab_annotations.chain(space_annotations)),
                ),
            ],
        }
    }
}

impl<'a> DiagnosticContext<'a> {
    #[allow(private_bounds)]
    pub fn active<'b, T: Lint<'a, 'b>>(&'b self) -> Option<T> {
        if let Severity::Allow = self.levels[T::INFO_INDEX] {
            None
        } else {
            Some(T::new(self))
        }
    }
    fn is_error(&self, info_index: usize) -> bool {
        matches!(self.levels[info_index], Severity::Deny)
    }
    fn main_group(&self, info_index: usize) -> Group<'a> {
        Group::from_lint(&INFOS[info_index], self.levels[info_index])
    }
}
