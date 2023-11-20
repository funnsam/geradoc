use serde_json::*;
use serde::*;

use std::collections::BTreeMap;

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

fn main() {
    let json = std::fs::read("test.json").unwrap();
    let root = from_slice::<Root>(&json[..]).unwrap();
    for (n, m) in root.modules.iter() {
        m.print(n, &root.types);
    }
}

impl Module {
    pub fn print(&self, name: &String, types: &Vec<Type>) {
        println!("# `{name}`");
        for (n, m) in self.modules.iter() {
            m.print(&format!("{name}.{n}"), types);
        }

        for (_, p) in self.procedures.iter() {
            let mut args = Vec::with_capacity(p.parameters.len());
            for p in p.parameters.iter() {
                args.push(format!("{}: {}", p.name, format_type(p.typ_, types)));
            }
            println!("    `{}proc {}({}) -> {}`", if p.external { "extern " } else { "" }, p.name, args.join(", "), format_type(p.return_types, types));
        }

        for (_, c) in self.constants.iter() {
            println!("    `var {} {}`", c.name, format_type(c.types, types));
        }
    }
}

fn format_type(typ_: usize, types: &Vec<Type>) -> String {
    let mut types_result = Vec::new();

    let typ = &types[typ_];
    if typ.any {
        types_result.push(format!("any"));
    } else {
        for i in typ.types.as_ref().unwrap().iter() {
            match i.typ_.as_str() {
                "object" => {
                    let mut fields = Vec::with_capacity(i.member_types.as_ref().unwrap().len());
                    for (n, t) in i.member_types.as_ref().unwrap().iter() {
                        fields.push(format!("{n}: {}", format_type(*t, types)));
                    }
                    types_result.push(format!("{{ {}{}{}}}", if i.fixed.unwrap() { "fixed " } else { "" }, fields.join(", "), if fields.is_empty() { "" } else { " " }));
                },
                "closure" => {
                    let mut params = Vec::with_capacity(i.parameter_types.as_ref().unwrap().len());
                    for t in i.parameter_types.as_ref().unwrap().iter() {
                        params.push(format!("{}", format_type(*t, types)));
                    }
                    types_result.push(format!("(|{}| -> {})", params.join(", "), format_type(i.return_types.unwrap(), types)));
                }
                "array" => types_result.push(format!("{}[]", format_type(i.element_types.unwrap(), types))),
                _ => types_result.push(format!("{}", i.typ_)),
            }
        }
    }

    types_result.join(" | ")
}
