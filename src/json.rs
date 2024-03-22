use crate::errors::{SResult, ToError};
use crate::ParseCharElt;
use core::fmt::{self, Write};
use core::hash;
use std::fs;

macro_rules! define_enum_and_ref {
    ($name:ident, $nameref:ident, $namerefmut:ident, $($variant:ident($t:ty)),*) => {
        #[derive(Debug, Clone)]
        enum $name {
            $($variant($t),)*
        }

        #[derive(Debug)]
        enum $nameref<'to_ref> {
            $($variant(&'to_ref $t),)*
        }

        #[allow(dead_code)]
        #[derive(Debug)]
        enum $namerefmut<'to_ref> {
            $($variant(&'to_ref mut $t),)*
        }

        #[allow(clippy::pattern_type_mismatch)]
        const fn to_ref(elt: &$name) -> $nameref {
            match elt {
                $($name::$variant(content) => $nameref::$variant(content),)*
            }
        }

        #[allow(clippy::pattern_type_mismatch)]
        fn to_refmut(elt: &mut $name) -> $namerefmut {
            match elt {
                $($name::$variant(content) => $namerefmut::$variant(content),)*
            }
        }

    };
}

define_enum_and_ref!(
    ParsedValue,
    ParsedValueRef,
    ParsedValueRefMut,
    Value(String),
    Array(Vec<ParsedValue>),
    Object(Parsed)
);

impl Default for ParsedValue {
    fn default() -> Self {
        Self::Value(String::new())
    }
}

trait PushLast<T> {
    fn push_last(&mut self, ch: T) -> SResult<()>;
}

impl<'main> PushLast<ParseCharElt<'main>> for Vec<ParsedValue> {
    fn push_last(&mut self, ch: ParseCharElt) -> Result<(), String> {
        match self.last_mut() {
            Some(last) => last.push(ch)?,
            None => self.push(ParsedValue::Value(ch.ch.to_string())),
        };
        Ok(())
    }
}

impl ParsedValue {
    fn push(&mut self, ch: ParseCharElt) -> SResult<()> {
        match to_refmut(self) {
            ParsedValueRefMut::Value(val) => val.push(ch.ch),
            ParsedValueRefMut::Array(arr) => arr.push_last(ch)?,
            ParsedValueRefMut::Object(_) => {
                return Err(crate::raise("Missing comma", &ch));
            }
        };
        Ok(())
    }

    fn is_empty(&self) -> bool {
        match to_ref(self) {
            ParsedValueRef::Value(val) => val.is_empty(),
            ParsedValueRef::Array(arr) => arr.last().map_or_else(|| true, Self::is_empty),
            ParsedValueRef::Object(obj) => obj
                .last()
                .map_or_else(|| true, |last| last.value.is_empty()),
        }
    }
}

#[derive(PartialEq)]
enum ParsingIndex {
    Key,
    Value,
}

#[derive(Default, Clone)]
struct ParsingItem {
    key: String,
    value: ParsedValue,
}

impl fmt::Debug for ParsingItem {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "\nJSON {{ <{}> => {:?} }}", self.key, self.value)
    }
}

#[allow(clippy::missing_trait_methods)]
impl PartialEq for ParsingItem {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

// #[allow(clippy::missing_trait_methods)]
// impl Eq for ParsingItem {}

///TODO: understand
impl hash::Hash for ParsingItem {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }

    fn hash_slice<H: hash::Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state);
        }
    }
}

impl ParsingItem {
    fn push(&mut self, ch: ParseCharElt, idx: &ParsingIndex) -> SResult<()> {
        match *idx {
            ParsingIndex::Key => self.key.push(ch.ch),
            ParsingIndex::Value => self.value.push(ch)?,
        }
        Ok(())
    }
}

type Parsed = Vec<ParsingItem>;

fn parse_array(content: &mut Vec<ParseCharElt>) -> SResult<Vec<ParsedValue>> {
    let mut result = Vec::<ParsedValue>::new();
    while let Some(elt) = content.pop() {
        match elt.ch {
            ']' => {
                if result.last().unwrap_or(&ParsedValue::default()).is_empty() {
                    result.pop();
                };
                return Ok(result);
            }
            '[' => {
                if result.last().unwrap_or(&ParsedValue::default()).is_empty() {
                    result.pop();
                }
                let array = parse_array(content)?;
                result.extend(array);
            }
            '{' => {
                if result.last().unwrap_or(&ParsedValue::default()).is_empty() {
                    result.pop();
                }
                let obj = parse_json(content)?;
                result.push(ParsedValue::Object(obj));
            }
            '}' => return Err(crate::raise("Mismatched closing brace", &elt)),
            ',' => result.push(ParsedValue::Value(String::new())),
            _ => result.push_last(elt)?,
        }
    }
    Err("EOF: missing closing bracket".to_owned())
}

fn parse_json(content: &mut Vec<ParseCharElt>) -> SResult<Parsed> {
    let mut instring = false;
    let mut result = Parsed::default();
    let mut current = ParsingItem::default();
    let mut idx = ParsingIndex::Key;
    while let Some(elt) = content.pop() {
        match elt.ch {
            '"' => {
                instring = !instring;
                if idx == ParsingIndex::Value {
                    current.push(elt, &idx)?;
                }
            }
            _ if instring => current.push(elt, &idx)?,
            ':' => idx = ParsingIndex::Value,
            ',' => {
                idx = ParsingIndex::Key;
                result.push(current.clone());
                current = ParsingItem::default();
            }
            '}' => break,
            '{' => {
                let rec = parse_json(content);
                current.value = ParsedValue::Object(rec?);
            }
            '[' => {
                let array = parse_array(content);
                current.value = ParsedValue::Array(array?);
            }
            ']' => return Err(crate::raise("Mismatched closing bracket", &elt)),
            _ => current.push(elt, &idx)?,
        }
    }
    result.push(current);
    Ok(result)
}

pub trait LocalToString {
    fn to_string(&self, tab: usize, start_indent: bool) -> Result<String, fmt::Error>;
}

fn int2indent(tab: usize) -> String {
    " ".repeat(tab.checked_mul(4).unwrap_or(tab))
}

impl LocalToString for Parsed {
    fn to_string(&self, tab: usize, start_indent: bool) -> Result<String, fmt::Error> {
        let wrap_indent = int2indent(tab);
        let elt_tab = tab.checked_add(1).unwrap_or(tab);
        let elt_indent = int2indent(elt_tab);
        let subelt_tab = tab.checked_add(2).unwrap_or(tab);
        let mut buffer = String::new();
        if start_indent {
            write!(buffer, "\n{wrap_indent}{{\n")?;
        } else {
            buffer.push_str("{\n");
            //     buffer.push('{');
            //     buffer.push('\n');
        }
        let mut last = self.len().checked_sub(1).unwrap_or_default();
        for item in self {
            match to_ref(&item.value) {
                ParsedValueRef::Value(val) => {
                    write!(buffer, "{}\"{}\": {val}", elt_indent, item.key)?;
                }
                ParsedValueRef::Array(arr) => {
                    write!(
                        buffer,
                        "{}\"{}\": {}",
                        elt_indent,
                        item.key,
                        arr.to_string(subelt_tab, false)?,
                    )?;
                }
                ParsedValueRef::Object(obj) => {
                    write!(buffer, "{}\"{}\": ", elt_indent, item.key)?;
                    buffer.push_str(&obj.to_string(elt_tab, false)?);
                    // buffer.push('\n');
                }
            }
            if last != 0 {
                buffer.push_str(",\n");
                last = last.checked_sub(1).unwrap_or_default();
            }
        }
        write!(buffer, "\n{wrap_indent}}}")?;
        Ok(buffer)
    }
}

impl LocalToString for Vec<ParsedValue> {
    fn to_string(&self, tab: usize, _: bool) -> Result<String, fmt::Error> {
        let mut last = self.len().checked_sub(1).unwrap_or_default();
        let beg = &format!("\n{}", &int2indent(tab));
        let mut buffer = String::from("[");
        for item in self {
            match to_ref(item) {
                ParsedValueRef::Value(val) => {
                    buffer.push_str(beg);
                    buffer.push_str(val);
                }
                ParsedValueRef::Array(arr) => {
                    buffer.push_str(&arr.to_string(tab.checked_add(1).unwrap_or(tab), true)?);
                }

                ParsedValueRef::Object(obj) => {
                    buffer.push_str(&obj.to_string(tab, true)?);
                }
            };
            if last != 0 {
                buffer.push(',');
                last = last.checked_sub(1).unwrap_or_default();
            } else {
                buffer.push('\n');
                buffer.push_str(&int2indent(tab.checked_sub(1).unwrap_or_default()));
                buffer.push(']');
            }
        }
        Ok(buffer)
    }
}

pub fn read(content: &mut Vec<ParseCharElt>) -> SResult<String> {
    let parsed = parse_json(content)?;
    // let parsed: Vec<ParsingItem> = vec![];
    parsed.to_string(0, false).cast_error()
}

pub fn append(
    path: &str,
    prevcontent: &mut Vec<ParseCharElt>,
    supplcontent: &mut Vec<ParseCharElt>,
) -> SResult<String> {
    let mut previous = parse_json(prevcontent)?;

    let suppl = parse_json(supplcontent)?;
    previous.extend(suppl);
    let content = previous.to_string(0, false).cast_error()?;
    Ok(content)
    // fs::write(path, content).cast_error()?;
    // Ok(())
    // Ok(String::new())
}
