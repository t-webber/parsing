use crate::{error, errors::SResult, ToError};
use std::{collections::HashMap, env, hash::Hash};

type GlobalArgs = HashMap<String, Vec<String>>;

#[derive(Default)]
pub struct Argvs {
    inner: HashMap<String, Vec<String>>,
}
pub const DEFAULT: &str = "__default__";
pub const FILE: &str = "__file__";

#[derive(Default)]
struct CurrentElt {
    key: String,
    values: Vec<String>,
}

impl CurrentElt {
    fn is_empty(&self) -> bool {
        self.key.is_empty() || self.values.is_empty()
    }
}

pub fn get_dash() -> SResult<(String, GlobalArgs)> {
    let mut args = env::args();
    let path = args.next().expl_error("Very interesting error")?;
    let mut res = GlobalArgs::default();
    let mut current = CurrentElt::default();
    for arg in args {
        if arg.starts_with('-') {
            res.insert(current.key.clone(), current.values.clone());
            current = CurrentElt {
                key: arg.get(1..).unwrap_or_default().to_owned(),
                values: Vec::new(),
            };
        } else {
            current.values.push(arg);
        }
    }
    res.insert(current.key, current.values);
    Ok((path, res))
}

type Local = HashMap<String, (String, Option<usize>)>;

pub fn find() -> Local {
    Local::new()
}

pub trait GetExpected {
    fn get(&self) -> Argvs;
    fn arg(self, args: Vec<&str>, nb: Option<usize>) -> Self;
}

impl GetExpected for &mut Local {
    fn get(&self) -> Argvs {
        let mut args = env::args();
        let path = args.next().unwrap_or_default();
        let mut res = Argvs::default();
        res.inner.insert("__file__".to_owned(), vec![path]);
        let mut current = CurrentElt {
            key: String::from("__default__"),
            values: Vec::new(),
        };
        let mut left: Option<usize> = None;
        for arg in args {
            match (**self).get(&arg) {
                Some(key) => {
                    if !current.is_empty() {
                        res.inner
                            .insert(current.key.clone(), current.values.clone());
                    }
                    current = CurrentElt {
                        key: key.0.clone(),
                        values: Vec::new(),
                    };
                    left = key.1;
                }
                None if left.map_or_else(|| true, |nb| nb != 0_usize) => {
                    current.values.push(arg);
                    left.decr();
                }
                _ => (),
            }
        }
        if !current.is_empty() {
            res.inner
                .insert(current.key.clone(), current.values.clone());
        }
        res
    }

    fn arg(self, args: Vec<&str>, nb: Option<usize>) -> Self {
        let mut iter = args.into_iter();
        if let Some(key) = iter.next() {
            self.insert(key.to_owned(), (key.to_owned(), nb));
            for value in iter {
                self.insert(value.to_owned(), (key.to_owned(), nb));
            }
        }
        self
    }
}

trait OptionLen {
    fn decr(&mut self);
}

impl OptionLen for Option<usize> {
    fn decr(&mut self) {
        if let Some(int) = self.as_mut() {
            *int = int.checked_sub(1_usize).unwrap_or_default();
        }
    }
}

pub trait GetValue {
    fn get_one(&self, key: &str) -> SResult<String>;
    fn get_all(&self, key: &str) -> Option<&Vec<String>>;
}

impl GetValue for Argvs {
    fn get_one(&self, key: &str) -> SResult<String> {
        match self.inner.get(key) {
            Some(values) if values.len() > 1 => Err(error!("Too many values for key {key}")),
            Some(values) if !values.is_empty() => values.first().map_or_else(
                || Err(error!("No value for key {key}")),
                |value| Ok(value.to_owned()),
            ),
            _ => Err(error!("No value for key {key}")),
        }
    }

    fn get_all(&self, key: &str) -> Option<&Vec<String>> {
        self.inner.get(key)
    }
}
