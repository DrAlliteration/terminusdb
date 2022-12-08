use juniper::{DefaultScalarValue, FromInputValue};
use lazy_static::*;
use rug::Integer;
use serde_json::*;
use std::borrow::Cow;
use std::collections::HashSet;
use terminusdb_store_prolog::terminus_store::structure::*;
use terminusdb_store_prolog::value::split_lang_string;

use crate::consts::XSD_PREFIX;

const TYPE_PREFIX_LEN: usize = XSD_PREFIX.len();
// funnily, this type prefix works for both the xsd types, and our own custom terminusdb xdd types, as the prefix is the same length!
// for terminusdb xdd this is   http://terminusdb.com/schema/xdd#

enum LangOrType<'a> {
    Lang(&'a str, &'a str),
    Type(&'a str, &'a str),
}

// FIXME: this does not currently deal with langstrings
fn value_string_to_slices(s: &str) -> LangOrType {
    // The format of these value strings is something like
    if s.as_bytes()[s.len() - 1] == '\'' as u8 {
        let pos = s[..s.len() - 1].rfind('\'').unwrap();
        if s.as_bytes()[pos - 1] == '^' as u8 {
            assert!(s.as_bytes()[pos - 2] == '^' as u8);
            LangOrType::Type(&s[0..pos - 2], &s[pos + 1 + TYPE_PREFIX_LEN..s.len() - 1])
        } else {
            assert!(s.as_bytes()[pos - 1] == '@' as u8);
            LangOrType::Lang(&s[..pos - 1], &s[pos..])
        }
    } else {
        let pos = s.rfind('@').unwrap();
        LangOrType::Lang(&s[..pos], &s[pos + 1..])
    }
}

pub fn value_to_string(tde: &TypedDictEntry) -> Cow<str> {
    match tde.datatype() {
        _ => todo!(),
    }
}

pub fn value_to_bool(tde: &TypedDictEntry) -> bool {
    match tde.datatype() {
        Datatype::Boolean => tde.as_val::<bool, bool>(),
        _ => panic!("This should not be a string, but a bool"),
    }
}

pub fn value_to_number(tde: &TypedDictEntry) -> Number {
    // should implement marshalling to Serde number?
    match tde.datatype() {
        _ => panic!("This should not be a string, but a number"),
    }
}

pub fn value_to_bigint(tde: &TypedDictEntry) -> Integer {
    match tde.datatype() {
        Datatype::BigInt => tde.as_val::<Integer, Integer>(),
        _ => panic!("This should not be a string, but a big int"),
    }
}

lazy_static! {
    static ref NUMERIC_TYPES: HashSet<&'static str> = [
        "decimal",
        "double",
        "float",
        "byte",
        "short",
        "int",
        "long",
        "unsignedByte",
        "unsignedShort",
        "unsignedInt",
        "unsignedLong",
        "integer",
        "positiveInteger",
        "nonNegativeInteger",
        "negativeInteger",
        "nonPositiveInteger"
    ]
    .into_iter()
    .collect();
    static ref FLOAT_TYPES: HashSet<&'static str> =
        ["decimal", "double", "float",].into_iter().collect();
    static ref SMALL_INTEGER_TYPES: HashSet<&'static str> = [
        "byte",
        "short",
        "int",
        "long",
        "unsignedByte",
        "unsignedShort",
        "unsignedInt",
    ]
    .into_iter()
    .collect();
    static ref BIG_INTEGER_TYPES: HashSet<&'static str> = [
        "unsignedLong",
        "integer",
        "positiveInteger",
        "nonNegativeInteger",
        "negativeInteger",
        "nonPositiveInteger"
    ]
    .into_iter()
    .collect();
    static ref DATETIME_TYPES: HashSet<&'static str> =
        ["dateTime", "dateTimeStamp",].into_iter().collect();
}

#[derive(PartialEq, Eq, Debug)]
pub enum BaseTypeKind {
    String,
    SmallInteger,
    BigIntger,
    Boolean,
    DateTime,
    Float,
}

pub fn base_type_kind(s: &str) -> BaseTypeKind {
    if type_is_small_integer(s) {
        BaseTypeKind::SmallInteger
    } else if type_is_big_integer(s) {
        BaseTypeKind::BigIntger
    } else if type_is_bool(s) {
        BaseTypeKind::Boolean
    } else if type_is_float(s) {
        BaseTypeKind::Float
    } else if type_is_datetime(s) {
        BaseTypeKind::DateTime
    } else {
        BaseTypeKind::String
    }
}

fn type_is_numeric(s: &str) -> bool {
    NUMERIC_TYPES.contains(s)
}

pub fn type_is_bool(s: &str) -> bool {
    s == "boolean"
}

pub fn type_is_small_integer(s: &str) -> bool {
    SMALL_INTEGER_TYPES.contains(s)
}

pub fn type_is_big_integer(s: &str) -> bool {
    BIG_INTEGER_TYPES.contains(s)
}

pub fn type_is_float(s: &str) -> bool {
    FLOAT_TYPES.contains(s)
}

pub fn type_is_datetime(s: &str) -> bool {
    DATETIME_TYPES.contains(s)
}

pub fn value_to_json(tde: &TypedDictEntry) -> Value {
    match tde.datatype() {
        Datatype::Boolean => Value::Bool(tde.as_val::<bool, bool>()),
        Datatype::Token => Value::Null, // This *really* should be checked.
        Datatype::UInt32
        | Datatype::Int32
        | Datatype::UInt64
        | Datatype::Int64
        | Datatype::Float32
        | Datatype::Float64 => todo!(), // serde number?
        Datatype::String => Value::String(tde.as_val::<String, String>()),
        Datatype::Decimal => Value::String(tde.as_val::<Decimal, String>()),
        Datatype::BigInt => Value::String(tde.as_val::<Integer, String>()),
        // Tokens are currently just used for null but may be used for more things in the future
        //Datatype::Token => Value::String(tde.as_val::<Token, String>()),
        Datatype::LangString => {
            let x = tde.as_val::<LangString, String>();
            let (lang, s) = split_lang_string(&x);
            json!({ "@lang" : lang, "@value" : s })
        }
    }
    /*
    match value_string_to_slices(s) {
        LangOrType::Type(val, typ) => {
            if typ == "boolean" {
                Value::Bool(val == "\"true\"")
            } else if typ == "token" && val == "\"null\"" {
                Value::Null
            } else if type_is_numeric(typ) {
                // it will have been saved unquoted
                //Value::Number(Number::from_string_unchecked(val.to_string())) // undocumented api - we know this is a number, so might as well save parse effort
                Value::Number(Number::from_str(val).unwrap())
            } else {
                // it will be something quoted, which we're gonna return as a string
                Value::String(prolog_string_to_string(val).into_owned())
            }
        }
        LangOrType::Lang(val, lang) => {
            let s = prolog_string_to_string(val);
            let l = lang[1..lang.len() - 1].to_string();
            json!({ "@lang" : l, "@value" : s })
        }
    }*/
}

pub fn value_to_graphql(tde: &TypedDictEntry) -> juniper::Value<DefaultScalarValue> {
    match tde.datatype() {
        Datatype::Boolean => {
            juniper::Value::Scalar(DefaultScalarValue::Boolean(tde.as_val::<bool, bool>()))
        }
        Datatype::String => {
            juniper::Value::Scalar(DefaultScalarValue::String(tde.as_val::<String, String>()))
        }
        Datatype::UInt32 => {
            juniper::Value::Scalar(DefaultScalarValue::Float(tde.as_val::<u64, u64>() as f64))
        }
        Datatype::Int32 => {
            juniper::Value::Scalar(DefaultScalarValue::Int(tde.as_val::<i32, i32>()))
        }
        Datatype::UInt64 => {
            todo!()
        }
        Datatype::Int64 => todo!(),
        Datatype::Float32 => {
            juniper::Value::Scalar(DefaultScalarValue::Float(tde.as_val::<f64, f64>() as f64))
        }
        Datatype::Float64 => {
            juniper::Value::Scalar(DefaultScalarValue::Float(tde.as_val::<f64, f64>() as f64))
        }
        Datatype::Decimal => {
            juniper::Value::Scalar(DefaultScalarValue::String(tde.as_val::<Decimal, String>()))
        }
        Datatype::BigInt => {
            juniper::Value::Scalar(DefaultScalarValue::String(tde.as_val::<Integer, String>()))
        }
        Datatype::Token => {
            juniper::Value::Scalar(DefaultScalarValue::String(tde.as_val::<Token, String>()))
        }
        Datatype::LangString => {
            todo!();
        }
    }
}

pub fn enum_node_to_value(enum_type: &str, enum_uri: &str) -> String {
    let enum_list: Vec<&str> = enum_uri.split(enum_type).collect();
    // 1.. is from slash on
    let enum_value = &(enum_list[1])[1..];
    enum_value.to_string()
}

pub enum ScalarInputValue {
    Boolean(bool),
    Int(i32),
    Float(f64),
    String(String),
}

impl FromInputValue for ScalarInputValue {
    fn from_input_value(v: &juniper::InputValue<DefaultScalarValue>) -> Option<Self> {
        match v {
            juniper::InputValue::Scalar(s) => Some(match s {
                DefaultScalarValue::Int(i) => Self::Int(*i),
                DefaultScalarValue::Float(f) => Self::Float(*f),
                DefaultScalarValue::String(s) => Self::String(s.to_owned()),
                DefaultScalarValue::Boolean(b) => Self::Boolean(*b),
            }),
            _ => None,
        }
    }
}

pub fn value_to_usize(tde: &TypedDictEntry) -> usize {
    tde.as_val::<u32, u32>() as usize
}

const SWIPL_CONTROL_CHAR_A: char = 7 as char;
const SWIPL_CONTROL_CHAR_B: char = 8 as char;
const SWIPL_CONTROL_CHAR_E: char = 27 as char;
const SWIPL_CONTROL_CHAR_F: char = 12 as char;
const SWIPL_CONTROL_CHAR_V: char = 11 as char;

fn prolog_string_to_string(s: &str) -> Cow<str> {
    let mut result: Option<String> = None;
    let mut escaping = false;
    let mut characters = s.char_indices().skip(1);
    while let Some((ix, c)) = characters.next() {
        if escaping {
            let result = result.as_mut().unwrap();
            match c {
                '\\' => result.push('\\'),
                '\"' => result.push('\"'),
                'x' => result.push(unescape_legacy_prolog_escape_sequence(&mut characters)),
                'a' => result.push(SWIPL_CONTROL_CHAR_A),
                'b' => result.push(SWIPL_CONTROL_CHAR_B),
                't' => result.push('\t'),
                'n' => result.push('\n'),
                'v' => result.push(SWIPL_CONTROL_CHAR_V),
                'f' => result.push(SWIPL_CONTROL_CHAR_F),
                'r' => result.push('\r'),
                _ => panic!("unknown prolog escape code in string"),
            }

            escaping = false;
        } else {
            if c == '\\' {
                escaping = true;
                if result.is_none() {
                    let mut r = String::with_capacity(s.len());
                    r.push_str(&s[1..ix]);
                    result = Some(r);
                }
            } else if let Some(result) = result.as_mut() {
                result.push(c);
            }
        }
    }

    match result {
        Some(mut result) => {
            assert_eq!(Some('\"'), result.pop());
            Cow::Owned(result)
        }
        None => Cow::Borrowed(&s[1..s.len() - 1]),
    }
}

fn unescape_legacy_prolog_escape_sequence(
    characters: &mut impl Iterator<Item = (usize, char)>,
) -> char {
    let mut digits: String = String::new();
    loop {
        let (_, digit) = characters.next().unwrap();
        if digit == '\\' {
            let hex = u32::from_str_radix(&digits, 16).unwrap();
            return char::from_u32(hex).unwrap();
        } else {
            digits.push(digit);
        }
    }
}

#[allow(unused)]
fn string_to_prolog_string(s: &str) -> String {
    // incomplete - lots of control characters gets printed weirdly by prolog.
    let mut result = String::with_capacity(s.len() + 10);
    result.push('"');

    for c in s.chars() {
        match c {
            '\\' => result.push_str(r#"\\"#),
            '\n' => result.push_str(r#"\n"#),
            '\t' => result.push_str(r#"\t"#),
            '\r' => result.push_str(r#"\r"#),
            SWIPL_CONTROL_CHAR_A => result.push_str(r#"\\a"#),
            SWIPL_CONTROL_CHAR_B => result.push_str(r#"\\b"#),
            SWIPL_CONTROL_CHAR_E => result.push_str(r#"\u001B"#),
            SWIPL_CONTROL_CHAR_F => result.push_str(r#"\\f"#),
            SWIPL_CONTROL_CHAR_V => result.push_str(r#"\\v"#),
            '\"' => result.push_str(r#"\""#),
            _ => result.push(c),
        }
    }

    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_strings_to_prolog_string() {
        assert_eq!(r#""hello there""#, string_to_prolog_string("hello there"));
        assert_eq!(r#""hello\nthere""#, string_to_prolog_string("hello\nthere"));
    }
    #[test]
    fn unescape_prolog_strings() {
        assert_eq!("hello there", prolog_string_to_string(r#""hello there""#));
        assert_eq!("hello\"there", prolog_string_to_string(r#""hello\"there""#));
        assert_eq!("hello\nthere", prolog_string_to_string(r#""hello\nthere""#));
        assert_eq!("hello\\there", prolog_string_to_string(r#""hello\\there""#));
        assert_eq!("\"", prolog_string_to_string(r#""\"""#));
        assert_eq!(
            "foo\u{5555}bar",
            prolog_string_to_string(r#""foo\x5555\bar""#)
        );
        assert_eq!(
            "foo\u{5555}bar\u{6666}baz",
            prolog_string_to_string(r#""foo\x5555\bar\x6666\baz""#)
        );
    }

    #[test]
    fn unescape_prolog_string_with_multibytes() {
        let result = prolog_string_to_string("\"\u{5555}\\n\"");
        assert_eq!("\u{5555}\n", result);
    }
}
