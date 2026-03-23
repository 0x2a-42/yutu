use std::path::PathBuf;

use crate::parser::Severity;
use rustc_hash::FxHashMap;
use toml::Table;

pub enum Global {
    ReadOnly,
    ReadWrite,
}

pub struct Config {
    pub lua_minor_version: usize,
    pub levels: FxHashMap<String, Severity>,
    pub parameter_threshold: usize,
    pub function_line_threshold: usize,
    pub nesting_threshold: usize,
    pub cyclomatic_complexity_threshold: usize,
    pub line_length_threshold: usize,
    pub allow_local_unused_hint: bool,
    pub allow_loopvar_unused_hint: bool,
    pub globals: FxHashMap<String, Global>,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            lua_minor_version: 5,
            levels: Default::default(),
            parameter_threshold: 8,
            function_line_threshold: 300,
            nesting_threshold: 6,
            cyclomatic_complexity_threshold: 50,
            line_length_threshold: 120,
            allow_local_unused_hint: true,
            allow_loopvar_unused_hint: true,
            globals: Default::default(),
        };
        config.add_globals();
        config
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Config {
            lua_minor_version,
            levels: _,
            parameter_threshold,
            function_line_threshold,
            nesting_threshold,
            cyclomatic_complexity_threshold,
            line_length_threshold,
            allow_local_unused_hint,
            allow_loopvar_unused_hint,
            globals: _,
        } = self;

        let mut lints = String::new();
        for info in crate::lints::INFOS {
            lints += &info.code.replace('-', "_");
            lints += " = \"";
            lints += &info.level.to_string();
            lints += "\"\n"
        }

        writeln!(
            f,
            "[lua]\
           \nstd = \"5.{lua_minor_version}\"\
           \n\
           \n[globals]\
           \nread_only = []\
           \nread_write = []\
           \n\
           \n[lints]\
           \n{lints}\
           \n[config]\
           \nparameter_threshold = {parameter_threshold}\
           \nfunction_line_threshold = {function_line_threshold}\
           \nnesting_threshold = {nesting_threshold}\
           \ncyclomatic_complexity_threshold = {cyclomatic_complexity_threshold}\
           \nline_length_threshold = {line_length_threshold}\
           \nallow_local_unused_hint = {allow_local_unused_hint}\
           \nallow_loopvar_unused_hint = {allow_loopvar_unused_hint}"
        )
    }
}

fn find_config() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    std::iter::from_fn(move || {
        let current = dir.join("yutu.toml");
        dir.pop().then_some(current)
    })
    .find(|p| p.exists())
}

impl Config {
    fn add_globals(&mut self) {
        let glob_min = [
            ("_G", Global::ReadWrite),
            ("_VERSION", Global::ReadOnly),
            ("arg", Global::ReadOnly),
            ("assert", Global::ReadOnly),
            ("collectgarbage", Global::ReadOnly),
            ("coroutine", Global::ReadOnly),
            ("debug", Global::ReadOnly),
            ("dofile", Global::ReadOnly),
            ("error", Global::ReadOnly),
            ("getmetatable", Global::ReadOnly),
            ("io", Global::ReadOnly),
            ("ipairs", Global::ReadOnly),
            ("load", Global::ReadOnly),
            ("loadfile", Global::ReadOnly),
            ("math", Global::ReadOnly),
            ("next", Global::ReadOnly),
            ("os", Global::ReadOnly),
            ("package", Global::ReadOnly),
            ("pairs", Global::ReadOnly),
            ("pcall", Global::ReadOnly),
            ("print", Global::ReadOnly),
            ("rawequal", Global::ReadOnly),
            ("rawget", Global::ReadOnly),
            ("rawset", Global::ReadOnly),
            ("require", Global::ReadOnly),
            ("select", Global::ReadOnly),
            ("setmetatable", Global::ReadOnly),
            ("string", Global::ReadOnly),
            ("table", Global::ReadOnly),
            ("tonumber", Global::ReadOnly),
            ("tostring", Global::ReadOnly),
            ("type", Global::ReadOnly),
            ("xpcall", Global::ReadOnly),
        ];
        let glob_51 = [
            ("getfenv", Global::ReadOnly),
            ("loadstring", Global::ReadOnly),
            ("module", Global::ReadOnly),
            ("newproxy", Global::ReadOnly),
            ("setfenv", Global::ReadOnly),
            ("unpack", Global::ReadOnly),
            ("gcinfo", Global::ReadOnly),
        ];
        let glob_52 = [
            ("_ENV", Global::ReadWrite),
            ("bit32", Global::ReadOnly),
            ("rawlen", Global::ReadOnly),
        ];
        let glob_53 = [("utf8", Global::ReadOnly)];
        let glob_54 = [("warn", Global::ReadWrite)];

        for (name, glob) in glob_min {
            self.globals.insert(name.to_string(), glob);
        }
        for (name, glob) in glob_51 {
            self.globals.insert(name.to_string(), glob);
        }
        for (name, glob) in glob_52 {
            self.globals.insert(name.to_string(), glob);
        }
        for (name, glob) in glob_53 {
            self.globals.insert(name.to_string(), glob);
        }
        for (name, glob) in glob_54 {
            self.globals.insert(name.to_string(), glob);
        }
    }

    fn read_usize(sec: &toml::Value, name: &str, value: &mut usize) -> Result<(), String> {
        if let Some(val) = sec.get(name) {
            if let Some(val) = val.as_integer()
                && val >= 0
            {
                *value = val as usize;
            } else {
                return Err(format!("expected positive integer value for `{name}`"));
            }
        }
        Ok(())
    }

    fn read_bool(sec: &toml::Value, name: &str, value: &mut bool) -> Result<(), String> {
        if let Some(val) = sec.get(name) {
            if let Some(val) = val.as_bool() {
                *value = val;
            } else {
                return Err(format!("expected boolean value for `{name}`"));
            }
        }
        Ok(())
    }

    pub fn new(lua_minor_version: Option<usize>) -> Result<Config, String> {
        let mut config = Config::default();
        if let Some(lua_minor_version) = lua_minor_version {
            config.lua_minor_version = lua_minor_version;
        }
        let Some(config_path) = find_config() else {
            return Ok(Self::default());
        };
        if let Ok(config_file) = std::fs::read_to_string(config_path) {
            let options = match config_file.parse::<Table>() {
                Ok(options) => options,
                Err(err) => return Err(format!("failed to parse yutu.toml\n\n{err}")),
            };
            if let Some(lua_sec) = options.get("lua")
                && let Some(val) = lua_sec.get("std")
            {
                match val.as_str() {
                    Some("5.5") => config.lua_minor_version = 5,
                    Some("5.4") => config.lua_minor_version = 4,
                    _ => {
                        return Err(format!("unsupported lua version `{val}`"));
                    }
                }
            }
            if let Some(global_sec) = options.get("globals").and_then(|value| value.as_table()) {
                if let Some(read_only) = global_sec
                    .get("read_only")
                    .and_then(|value| value.as_array())
                {
                    for glob in read_only {
                        if let Some(name) = glob.as_str() {
                            config.globals.insert(name.to_string(), Global::ReadOnly);
                        }
                    }
                }
                if let Some(read_write) = global_sec
                    .get("read_write")
                    .and_then(|value| value.as_array())
                {
                    for glob in read_write {
                        if let Some(name) = glob.as_str() {
                            config.globals.insert(name.to_string(), Global::ReadWrite);
                        }
                    }
                }
            }
            if let Some(lints_sec) = options.get("lints").and_then(|value| value.as_table()) {
                for (lint, value) in lints_sec {
                    if let Some(value) = value.as_str()
                        && let Ok(severity) = value.try_into()
                    {
                        config.levels.insert(lint.replace('_', "-"), severity);
                    }
                }
            }
            if let Some(config_sec) = options.get("config") {
                Self::read_usize(
                    config_sec,
                    "parameter_threshold",
                    &mut config.parameter_threshold,
                )?;
                Self::read_usize(
                    config_sec,
                    "function_line_threshold",
                    &mut config.function_line_threshold,
                )?;
                Self::read_usize(
                    config_sec,
                    "nesting_threshold",
                    &mut config.nesting_threshold,
                )?;
                Self::read_usize(
                    config_sec,
                    "cyclomatic_complexity_threshold",
                    &mut config.cyclomatic_complexity_threshold,
                )?;
                Self::read_usize(
                    config_sec,
                    "line_length_threshold",
                    &mut config.line_length_threshold,
                )?;
                Self::read_bool(
                    config_sec,
                    "allow_local_unused_hint",
                    &mut config.allow_local_unused_hint,
                )?;
                Self::read_bool(
                    config_sec,
                    "allow_loopvar_unused_hint",
                    &mut config.allow_loopvar_unused_hint,
                )?;
            }
        }
        Ok(config)
    }
}
