use annotate_snippets::Renderer;
use annotate_snippets::renderer::AnsiColor;
use snapbox::data::DataFormat;
use snapbox::{Data, assert_data_eq};
use std::path::PathBuf;

macro_rules! test {
    ($name:ident) => {
        test!($name,);
    };
    ($name:ident, $($level:expr),*) => {
        #[test]
        fn $name() {
            let name = stringify!($name);

            let path = PathBuf::from(format!("tests/diagnostics/{name}.lua"));
            let path_string = path.display().to_string();
            let source = yutu::read_source(&path).unwrap();
            #[allow(unused_mut)]
            let mut config = yutu::config::Config::default();
            $(
                config.levels.insert($level.to_string(), yutu::parser::Severity::Warn);
            )*

            let diags = yutu::check_source(&path_string, &source, &config);
            let renderer = Renderer::styled().warning(AnsiColor::BrightYellow.on_default());
            let mut input = String::new();
            for diag in diags {
                input += &renderer.render(&diag.groups);
                input.push_str("\n\n");
            }

            let path = PathBuf::from(format!("tests/diagnostics/{name}.term.svg"));
            let expected_ascii = Data::read_from(&path, Some(DataFormat::TermSvg));
            assert_data_eq!(input, expected_ascii);
        }
    };
}

test!(demo);

test!(unterminated_string);
test!(unterminated_long_string);
test!(unterminated_comment);
test!(unexpected_exp_stat);
test!(unexpected_assign_lhs);
test!(unexpected_attribute);
test!(undefined_label);
test!(redefined_label);
test!(break_outside_loop);
test!(invalid_escape_sequence);
test!(goto_skips_local);
test!(invalid_token);
test!(first_line_comment);
test!(write_const_variable);
test!(undeclared_global);
test!(latin1);
test!(invalid_vararg);

test!(unused_local);
test!(trailing_whitespace);
test!(only_whitespace);
test!(empty_statement);
test!(lower_case_global);
test!(empty_block);
test!(unnecessary_negation);
test!(error_prone_negation);
test!(unused_label);
test!(used_despite_unused_hint);
test!(unused_parameter);
test!(unused_loopvar);
test!(redefined_local, "redefined-local");
test!(redundant_local);
test!(shadowing_local, "shadowing-local");
test!(unused_vararg);
test!(unbalanced_assignment);
test!(unbalanced_initialization);
test!(too_many_parameters);
test!(almost_swap);
test!(approx_pi);
test!(rounds_to_inf);
test!(rounds_int_part);
test!(octal_confusion);
test!(hex_int_overflow);
test!(next_line_args);
test!(too_many_lines);
test!(bool_compare);
test!(unreachable);
test!(invisible_characters);
test!(non_ascii_literal, "non-ascii-literal");
test!(line_too_long);
test!(redundant_parentheses);
test!(inexact_hex_float);
test!(cyclomatic_complexity);
test!(inconsistent_indentation);
