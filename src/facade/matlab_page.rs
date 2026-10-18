//! docs/matlab.md, the MATLAB wrappers' reference, written from their own help text: each `.m`
//! file of the Python package's `+photonoxide` folder gives its call, its help (what MATLAB's
//! `help photonoxide.<name>` shows) and its `arguments` block, so the page can't drift from what
//! ships. The test fails when the page isn't what the wrappers write;
//! `PHOTONOXIDE_WRITE_MATLAB_PAGE=1 cargo test --lib facade::matlab_page` rewrites it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The repository's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The wrappers' folder.
fn wrappers() -> PathBuf {
    root().join("python/photonoxide/matlab/+photonoxide")
}

/// A wrapper as its file has it.
#[derive(Debug, PartialEq)]
struct Wrapper {
    /// The function's name, as `photonoxide.<name>` calls it.
    name: String,
    /// Its outputs, as the function line has them (`s`, `[a, b]`), empty for none.
    output: String,
    /// Its arguments, as the function line has them.
    inputs: Vec<String>,
    /// The help's first line: what it is.
    summary: String,
    /// The rest of the help, its leading `%` and three spaces taken off; an empty line between
    /// paragraphs.
    help: Vec<String>,
    /// The `arguments` block's lines: (name, size, class, default), `options.` kept.
    arguments: Vec<Argument>,
}

#[derive(Debug, PartialEq)]
struct Argument {
    name: String,
    size: String,
    class: String,
    default: String,
}

/// Reads a wrapper's text.
fn parse(text: &str) -> Result<Wrapper, String> {
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    let first = lines.next().ok_or("an empty file")?;
    let call = first
        .strip_prefix("function ")
        .ok_or_else(|| format!("the first line isn't a function line: {first}"))?;
    let (output, call) = match call.split_once('=') {
        Some((o, c)) => (o.trim().to_owned(), c.trim()),
        None => (String::new(), call.trim()),
    };
    let (name, inputs) = match call.split_once('(') {
        Some((n, rest)) => {
            let inside = rest
                .strip_suffix(')')
                .ok_or_else(|| format!("no closing parenthesis: {first}"))?;
            let inputs = inside
                .split(',')
                .map(|a| a.trim().to_owned())
                .filter(|a| !a.is_empty())
                .collect();
            (n.trim().to_owned(), inputs)
        }
        None => (call.to_owned(), Vec::new()),
    };
    let h1 = lines
        .next()
        .and_then(|l| l.strip_prefix('%'))
        .ok_or_else(|| format!("{name}: no help line after the function line"))?;
    let summary = h1
        .strip_prefix(&name.to_uppercase())
        .ok_or_else(|| {
            format!(
                "{name}: the help's first line doesn't start with {}",
                name.to_uppercase()
            )
        })?
        .trim()
        .to_owned();
    // the help runs to the first line that isn't a comment; an arguments block may follow it
    let mut lines = lines.peekable();
    let mut help = Vec::new();
    while let Some(h) = lines.peek().and_then(|l| l.strip_prefix('%')) {
        help.push(h.strip_prefix("   ").unwrap_or(h.trim_start()).to_owned());
        lines.next();
    }
    let mut arguments = Vec::new();
    if lines.peek().map(|l| l.trim()) == Some("arguments") {
        lines.next();
        for line in lines.map(str::trim) {
            if line == "end" {
                break;
            }
            if !line.is_empty() {
                arguments.push(argument(line)?);
            }
        }
    }
    Ok(Wrapper {
        name,
        output,
        inputs,
        summary,
        help,
        arguments,
    })
}

/// An `arguments` block's line: `name (size) class = default`, each part but the name optional.
fn argument(line: &str) -> Result<Argument, String> {
    let (spec, default) = match line.split_once('=') {
        Some((s, d)) => (s.trim(), d.trim().to_owned()),
        None => (line, String::new()),
    };
    let (name, rest) = spec.split_once(' ').unwrap_or((spec, ""));
    let rest = rest.trim();
    let (size, class) = match rest.strip_prefix('(') {
        Some(r) => {
            let (size, class) = r
                .split_once(')')
                .ok_or_else(|| format!("an argument's size isn't closed: {line}"))?;
            (size.trim().to_owned(), class.trim().to_owned())
        }
        None => (String::new(), rest.to_owned()),
    };
    if name.is_empty() {
        return Err(format!("an argument without a name: {line}"));
    }
    Ok(Argument {
        name: name.to_owned(),
        size,
        class,
        default,
    })
}

/// A size as the page says it: `(1,1)` a scalar, `(1,4)` 1-by-4, none any.
fn size_text(size: &str) -> String {
    match size {
        "" => "any".to_owned(),
        "1,1" => "scalar".to_owned(),
        s => s.replace(',', "-by-"),
    }
}

/// The help's lines as Markdown: paragraphs as they are, and the lines indented past the text
/// (MATLAB code, in the help) as a MATLAB block.
fn help_markdown(help: &[String], out: &mut String) {
    let mut code = false;
    for line in help {
        let is_code = line.starts_with("  ") && !line.trim().is_empty();
        if is_code && !code {
            out.push_str("```matlab\n");
            code = true;
        } else if !is_code && code && !line.trim().is_empty() {
            out.push_str("```\n\n");
            code = false;
        }
        if code {
            if line.trim().is_empty() {
                continue;
            }
            out.push_str(&line[2..]);
        } else {
            out.push_str(line.trim_start());
        }
        out.push('\n');
    }
    if code {
        out.push_str("```\n");
    }
}

/// The page, from the wrappers' files.
fn page() -> String {
    let mut files: Vec<PathBuf> = std::fs::read_dir(wrappers())
        .expect("the wrappers' folder")
        .map(|e| e.expect("a folder entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "m"))
        .collect();
    files.sort();
    let wrappers: Vec<Wrapper> = files
        .iter()
        .map(|f| {
            let text = std::fs::read_to_string(f).expect("a wrapper");
            parse(&text).unwrap_or_else(|e| panic!("{}: {e}", f.display()))
        })
        .collect();
    let mut out = String::new();
    out.push_str(
        "# photonoxide in MATLAB\n\n\
         Written from the help text of the wrappers in \
         [`python/photonoxide/matlab/+photonoxide`](../python/photonoxide/matlab/+photonoxide) \
         by `src/facade/matlab_page.rs`; don't edit it by hand. CI fails when it isn't what the \
         wrappers say, and `PHOTONOXIDE_WRITE_MATLAB_PAGE=1 cargo test --lib facade::matlab_page` \
         rewrites it. In MATLAB, `help photonoxide.<name>` shows the same text.\n\n\
         MATLAB calls photonoxide through the Python package, from R2022b. Install the package \
         with pip into a Python of 3.10 or later, then, once per session:\n\n\
         ```matlab\n\
         pyenv(Version=\"/path/to/python\");             % a Python with photonoxide installed, once\n\
         addpath(string(py.photonoxide.matlab.path()));\n\
         ```\n\n\
         The wrappers take MATLAB's arrays, structs and name-value arguments and return MATLAB \
         arrays and structs, keeping the Python package's shapes and indexing; photonoxide's \
         errors are MATLAB errors with identifiers such as `photonoxide:InvalidValue`. How they \
         convert, what is tested and what isn't (MATLAB itself isn't run in CI) is in \
         [docs/python.md](python.md#matlab).\n\n\
         ## The functions\n\n\
         | Function | What it is |\n|---|---|\n",
    );
    for w in &wrappers {
        let _ = writeln!(
            out,
            "| [`photonoxide.{}`](#photonoxide{}) | {} |",
            w.name, w.name, w.summary
        );
    }
    for w in &wrappers {
        let _ = writeln!(out, "\n## photonoxide.{}\n\n{}\n", w.name, w.summary);
        let call = format!("photonoxide.{}({})", w.name, w.inputs.join(", "));
        let call = if w.output.is_empty() {
            call
        } else {
            format!("{} = {call}", w.output)
        };
        let _ = writeln!(out, "```matlab\n{call}\n```\n");
        help_markdown(&w.help, &mut out);
        if !w.arguments.is_empty() {
            out.push_str("\n| Argument | Size | Class | Default |\n|---|---|---|---|\n");
            for a in &w.arguments {
                let name = match a.name.strip_prefix("options.") {
                    Some(o) => format!("`{o}` (name-value)"),
                    None => format!("`{}`", a.name),
                };
                let class = if a.class.is_empty() { "any" } else { &a.class };
                let default = if a.default.is_empty() {
                    "required".to_owned()
                } else {
                    format!("`{}`", a.default)
                };
                let default = if a.default.is_empty() && a.name.starts_with("options.") {
                    "none".to_owned()
                } else {
                    default
                };
                let _ = writeln!(
                    out,
                    "| {name} | {} | {class} | {default} |",
                    size_text(&a.size)
                );
            }
        }
    }
    out
}

#[test]
fn the_matlab_page_is_the_wrappers_help() {
    let page = page();
    let path = root().join("docs/matlab.md");
    if std::env::var_os("PHOTONOXIDE_WRITE_MATLAB_PAGE").is_some() {
        std::fs::write(&path, &page).expect("docs/matlab.md written");
    }
    let current = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        current == page,
        "docs/matlab.md isn't what the wrappers' help says: \
         PHOTONOXIDE_WRITE_MATLAB_PAGE=1 cargo test --lib facade::matlab_page rewrites it"
    );
}

#[test]
fn every_facade_function_has_its_wrapper_on_the_page() {
    // the façade's functions, as the Python package and the wrappers name them, but those
    // MATLAB doesn't have yet: a function added to the façade gets its wrapper, or its place here
    const NOT_IN_MATLAB: [&str; 1] = ["check_job"];
    let page = page();
    for f in super::tests::facade_functions() {
        let on_page = page.contains(&format!("## photonoxide.{f}\n"));
        assert_eq!(
            on_page,
            !NOT_IN_MATLAB.contains(&f),
            "photonoxide.{f}: on docs/matlab.md {on_page}, listed as not in MATLAB {}",
            NOT_IN_MATLAB.contains(&f)
        );
    }
    // and the package's own: its version
    assert!(page.contains("## photonoxide.version\n"));
}

#[test]
fn the_feature_tables_matlab_links_are_on_the_page() {
    // docs/features.md links each MATLAB cell's functions to their entries here
    let page = page();
    let table = std::fs::read_to_string(root().join("docs/features.md")).expect("the table");
    let mut links = 0;
    for (at, _) in table.match_indices("(matlab.md#photonoxide") {
        let rest = &table[at + "(matlab.md#photonoxide".len()..];
        let name = &rest[..rest.find(')').expect("a closed link")];
        assert!(
            page.contains(&format!("## photonoxide.{name}\n")),
            "docs/features.md links to photonoxide.{name}, which docs/matlab.md hasn't"
        );
        links += 1;
    }
    assert!(links >= 20, "{links} links");
}

#[test]
fn a_wrapper_reads_its_call_help_and_arguments() {
    let w = parse(
        "function s = thing(a, b, options)\n\
         %THING  A thing.\n\
         %   s = photonoxide.thing(a, b) is it.\n\
         %\n\
         %     s = photonoxide.thing(1, 2);\n\
         %\n\
         %   Options: c.\n\
         arguments\n\
         \x20   a (1,1) string\n\
         \x20   b double = []\n\
         \x20   options.c (1,4) double = [0 0 0 0]\n\
         \x20   options.d\n\
         end\n\
         r = 1;\n",
    )
    .unwrap();
    assert_eq!(w.name, "thing");
    assert_eq!(w.output, "s");
    assert_eq!(w.inputs, ["a", "b", "options"]);
    assert_eq!(w.summary, "A thing.");
    assert_eq!(
        w.help,
        [
            "s = photonoxide.thing(a, b) is it.",
            "",
            "  s = photonoxide.thing(1, 2);",
            "",
            "Options: c."
        ]
    );
    assert_eq!(
        w.arguments,
        [
            Argument {
                name: "a".into(),
                size: "1,1".into(),
                class: "string".into(),
                default: String::new()
            },
            Argument {
                name: "b".into(),
                size: String::new(),
                class: "double".into(),
                default: "[]".into()
            },
            Argument {
                name: "options.c".into(),
                size: "1,4".into(),
                class: "double".into(),
                default: "[0 0 0 0]".into()
            },
            Argument {
                name: "options.d".into(),
                size: String::new(),
                class: String::new(),
                default: String::new()
            },
        ]
    );
    let mut md = String::new();
    help_markdown(&w.help, &mut md);
    assert_eq!(
        md,
        "s = photonoxide.thing(a, b) is it.\n\n```matlab\ns = photonoxide.thing(1, 2);\n```\n\nOptions: c.\n"
    );
    assert!(parse("x = 1;\n").is_err());
    assert!(parse("function f()\n%G  not its name\n").is_err());
}
