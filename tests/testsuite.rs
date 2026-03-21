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

            let path = PathBuf::from(format!("tests/testsuite/lua-5.5.0-tests/{name}.lua"));
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

            let path = PathBuf::from(format!("tests/testsuite/lua-5.5.0-tests/{name}.term.svg"));
            let expected_ascii = Data::read_from(&path, Some(DataFormat::TermSvg));
            assert_data_eq!(input, expected_ascii);
        }
    };
}

test!(all);
test!(api);
test!(attrib);
test!(big);
test!(bitwise);
test!(bwcoercion);
test!(calls);
test!(closure);
test!(code);
test!(constructs);
test!(coroutine);
test!(cstack);
test!(db);
test!(errors);
test!(events);
test!(files);
test!(gc);
test!(gengc);
test!(goto);
test!(heavy);
test!(literals);
test!(locals);
test!(main);
test!(math);
test!(memerr);
test!(nextvar);
test!(pm);
test!(sort);
test!(strings);
test!(tpack);
test!(tracegc);
test!(utf8);
test!(vararg);
test!(verybig);
