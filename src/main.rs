#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    clippy::cargo
)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::implicit_return)]
#![allow(clippy::question_mark_used)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::single_call_fn)]
#![allow(clippy::std_instead_of_core)]
#![allow(clippy::blanket_clippy_restriction_lints)]
#![feature(stmt_expr_attributes)]
#![allow(clippy::separated_literal_suffix)]
#![allow(dead_code)]
#![allow(unused)]

mod argv;
mod errors;
mod json;

use crate::argv::GetExpected;
use crate::argv::GetValue;
use crate::errors::SResult;
use crate::errors::ToError;

use core::fmt;
use std::collections::HashMap;
use std::env;
use std::fs;

pub struct ParseCharElt<'main> {
    ch: char,
    path: &'main str,
    line: usize,
    col: usize,
}

impl fmt::Debug for ParseCharElt<'_> {
    fn fmt(&self, fmter: &mut fmt::Formatter) -> fmt::Result {
        write!(fmter, "{}", self.ch)
    }
}

#[must_use]
pub fn raise(msg: &str, elt: &ParseCharElt) -> String {
    error!("{}:{}:{}: {}", elt.path, elt.line, elt.col, msg)
}

#[allow(clippy::print_stdout)]
fn read(content: &mut Vec<ParseCharElt>, extension: &str) -> SResult<String> {
    let read = match extension {
        "json" => json::read(content)?,
        _ => return Err(error!("Extension {extension} not supported.")),
    };
    Ok(read)
}

fn append(content: &mut Vec<ParseCharElt>, add: &mut Vec<ParseCharElt>) -> SResult<String> {
    // dbg!(&add);
    let append = json::append("", content, add)?;
    Ok(append)
}

fn string2parsechar<'path>(path: &'path str, content: &str) -> Vec<ParseCharElt<'path>> {
    let mut chars: Vec<ParseCharElt> = content
        .split('\n')
        .enumerate()
        .flat_map(|(nbline, cont)| {
            cont.chars()
                .enumerate()
                .map(move |(nbcol, contcol)| (nbline, nbcol, contcol))
        })
        .map(|(line, col, ch)| ParseCharElt {
            ch,
            path,
            line,
            col,
        })
        .filter(|elt| !elt.ch.is_whitespace())
        .collect::<Vec<_>>();
    chars.reverse();
    chars
}

fn main_wrapper() -> SResult<()> {
    let args = argv::find()
        // .arg(vec![""], None)
        .arg(vec!["-f", "--file"], Some(1))
        .arg(vec!["-t", "--type"], Some(1))
        .arg(vec!["-o", "--output"], Some(1))
        .arg(vec!["-v", "--value"], Some(1))
        .get();
    let filename = args
        .get_one("-f")
        .unwrap_or_else(|err| String::from("./data/test.json"));
    let split = filename.split('.').collect::<Vec<&str>>();
    let extension = (split.len() >= 2)
        .then_some(split.last())
        .flatten()
        .ok_or("No extension found in the filename.")?;
    let mut chars = string2parsechar(&filename, &fs::read_to_string(&filename).cast_error()?);
    let mut add = string2parsechar("", &args.get_one("-v")?);
    let output = args.get_one("-o").unwrap_or_default();
    #[allow(clippy::print_stdout)]
    match (
        match args.get_one("-t")?.as_str() {
            "read" => read(&mut chars, extension),
            "append" => append(&mut chars, &mut add),
            _ => Err(error!("Type not supported.")),
        },
        output.as_str(),
    ) {
        (Err(err), _) => return Err(err),
        (Ok(content), "std" | "stdout" | "out" | "") => println!("{}", info!("{content}")),
        (Ok(content), file) => fs::write(args.get_one("-o")?, content).cast_error()?,
    };
    Ok(())
}

fn main() {
    #[allow(clippy::print_stdout)]
    match main_wrapper() {
        Ok(()) => (),
        Err(err) => println!("{err}"),
    }
}
