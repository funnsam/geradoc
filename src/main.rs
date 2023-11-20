#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::nursery,
    clippy::suspicious,
    clippy::style,
)]
#![allow(
    clippy::semicolon_inside_block,
    clippy::just_underscores_and_digits,
)]

const MAX_RECURSION: usize = 10;

use serde_json::*;
use serde::*;
use clap::Parser;
use regex::Regex;

use std::collections::BTreeMap;
use std::process::*;
use std::io::Write;
use std::fs::*;

#[derive(Debug, Deserialize)]
struct Root {
    pub types: Vec<Type>,
    pub modules: BTreeMap<String, Module>,
}

#[derive(Debug, Deserialize)]
struct Module {
    pub modules: BTreeMap<String, Module>,
    pub procedures: BTreeMap<String, Procedure>,
    pub constants: BTreeMap<String, Constant>,
}

#[derive(Debug, Deserialize)]
struct Procedure {
    pub name: String,
    pub external: bool,
    pub parameters: Vec<Parameter>,
    pub return_types: usize,
    pub public: bool,
}

#[derive(Debug, Deserialize)]
struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub typ_: usize,
}

#[derive(Debug, Deserialize)]
struct Constant {
    pub name: String,
    pub types: usize,
    pub public: bool,
}

#[derive(Debug, Deserialize)]
struct Type {
    pub any: bool,
    pub types: Option<Vec<InnerType>>,
}

#[derive(Debug, Deserialize)]
struct InnerType {
    #[serde(rename = "type")]
    pub typ_: String,

    // array
    pub element_types: Option<usize>,

    // object
    pub member_types: Option<BTreeMap<String, usize>>,

    // variants
    pub variant_types: Option<BTreeMap<String, usize>>,

    // object & variants
    pub fixed: Option<bool>,

    // closure
    pub parameter_types: Option<Vec<usize>>,
    pub return_types: Option<usize>,
}

#[derive(Debug, Parser)]
struct Args {
    pub gera_file_path: Vec<String>,

    #[clap(long, default_value = r"core")]
    pub modules_filter: String,
}

fn main() {
    let args = Args::parse();

    let out = Command::new("gerac")
        .args(args.gera_file_path)
        .args(["-o", "_tmp_geradoc.json", "-t", "symbols"])
        .output()
        .unwrap();

    if !out.status.success() {
        print!("{}", String::from_utf8(out.stdout).unwrap());
        exit(out.status.code().unwrap());
    }

    let json = std::fs::read("_tmp_geradoc.json").unwrap();
    let mut root = from_slice::<Root>(&json[..]).unwrap();

    let filter_regex = Regex::new(&args.modules_filter).expect("Invalid regex passed in");
    root.modules.retain(|n, _| !filter_regex.is_match(n));

    let _ = remove_dir_all("docs");

    for (n, m) in root.modules.iter_mut() {
        m.write(n, n, &root.types);
    }

    write_index(&root.modules);
    write_css();

    remove_file("_tmp_geradoc.json").unwrap();
}

impl Module {
    pub fn write(&mut self, name: &String, this_name: &String, types: &Vec<Type>) {
        let path = format!("docs/{}.html", name.replace("::", "/"));
        let path = std::path::Path::new(&path);
        create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path).unwrap();

        writeln!(file,
            "<!DOCTYPE HTML><html><head>
            <title>GeraDoc — {name}</title>
            <meta charset='UTF-8'>
            <link rel='stylesheet' href='{}style.css'>
            </head><body>",

            "../".repeat(name.matches("::").count())
        ).unwrap();

        writeln!(file, "<h1>Module <code>{name}</code></h1>").unwrap();

        if !self.modules.is_empty() {
            writeln!(file, "<h2>Modules</h2><ul>").unwrap();
        }

        for (n, m) in self.modules.iter_mut() {
            m.write(&format!("{name}::{n}"), n, types);
            writeln!(file, "    <li><a href='./{this_name}/{n}.html'><code>{n}</code></a>").unwrap();
        }

        if !self.modules.is_empty() {
            writeln!(file, "</ul>").unwrap();
        }

        self.constants.retain(|_, a| a.public);
        if !self.constants.is_empty() {
            writeln!(file, "<h2>Constants</h2><ul>").unwrap();
        }

        for (_, c) in self.constants.iter() {
            writeln!(file, "    <li><code>pub var {}: {}</code></li>", c.name, format_type(c.types, types, 0)).unwrap();
        }

        if !self.constants.is_empty() {
            writeln!(file, "</ul>").unwrap();
        }

        self.procedures.retain(|_, a| a.public);
        if !self.procedures.is_empty() {
            writeln!(file, "<h2>Procedures</h2><ul>").unwrap();
        }

        for (_, p) in self.procedures.iter() {
            let mut args = Vec::with_capacity(p.parameters.len());
            for p in p.parameters.iter() {
                args.push(format!("{}: {}", p.name, format_type(p.typ_, types, 0)));
            }
            writeln!(file, "    <li><code>pub {}proc {}({}) -> {}</code></li>", if p.external { "extern " } else { "" }, p.name, args.join(", "), format_type(p.return_types, types, 0)).unwrap();
        }

        if !self.procedures.is_empty() {
            writeln!(file, "</ul>").unwrap();
        }

        writeln!(file, "</body></html>").unwrap();
    }
}

fn format_type(typ_: usize, types: &Vec<Type>, iter: usize) -> String {
    if iter == MAX_RECURSION {
        return "?".to_string();
    }

    let mut types_result = Vec::new();

    let typ = &types[typ_];
    if typ.any {
        types_result.push("any".to_string());
    } else {
        for i in typ.types.as_ref().unwrap().iter() {
            match i.typ_.as_str() {
                "object" => {
                    let mut fields = Vec::with_capacity(i.member_types.as_ref().unwrap().len());
                    for (n, t) in i.member_types.as_ref().unwrap().iter() {
                        fields.push(format!("{n}: {}", format_type(*t, types, iter+1)));
                    }
                    types_result.push(format!("{{ {}{}}}", fields.join(", "), match (fields.is_empty(), i.fixed.unwrap()) {
                        (true, true) => "",
                        (false, true) => " ",
                        (true, false) => "... ",
                        (false, false) => ", ... ",
                    }));
                },
                "closure" => {
                    let mut params = Vec::with_capacity(i.parameter_types.as_ref().unwrap().len());
                    for t in i.parameter_types.as_ref().unwrap().iter() {
                        params.push(format_type(*t, types, iter+1));
                    }
                    types_result.push(format!("(|{}| -> {})", params.join(", "), format_type(i.return_types.unwrap(), types, iter+1)));
                },
                "variants" => {
                    let mut varis = Vec::with_capacity(i.variant_types.as_ref().unwrap().len());
                    for (n, t) in i.variant_types.as_ref().unwrap().iter() {
                        varis.push(format!("{n}({})", format_type(*t, types, iter+1)));
                    }
                    types_result.push(format!("variant({})", varis.join(", ")));
                },
                "array" => types_result.push(format!("{}[]", format_type(i.element_types.unwrap(), types, iter+1))),
                _ => types_result.push(i.typ_.to_string()),
            }
        }
    }

    types_result.join(" | ")
}

fn write_index(mods: &BTreeMap<String, Module>) {
    let path = "docs/index.html";
    let path = std::path::Path::new(&path);
    create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(path).unwrap();

    writeln!(file,
        "<!DOCTYPE HTML><html><head>
        <title>GeraDoc — Index</title>
        <meta charset='UTF-8'>
        <link rel='stylesheet' href='style.css'>
        </head><body>",
    ).unwrap();

    writeln!(file, "<h1>Document index</h1>").unwrap();

    if !mods.is_empty() {
        writeln!(file, "<h2>Modules</h2><ul>").unwrap();
    }

    for (n, _) in mods.iter() {
        writeln!(file, "    <li><a href='./{n}.html'><code>{n}</code></a>").unwrap();
    }

    if !mods.is_empty() {
        writeln!(file, "</ul>").unwrap();
    }

    writeln!(file, "</body></html>").unwrap();
}

fn write_css() {
    const CSS_FILE: &[u8] = include_bytes!("style.css");
    write("docs/style.css", CSS_FILE).unwrap();
}
