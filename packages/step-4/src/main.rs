#![allow(unused)]

use core::panic;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take},
    character::complete::{
        alphanumeric1 as alphanumeric, anychar, char, multispace0, multispace1, none_of, one_of,
    },
    combinator::{cut, map, map_opt, peek, value, verify},
    error::{context, ContextError, Error, ErrorKind, FromExternalError, ParseError, VerboseError},
    multi::{fold_many0, many0, separated_list0},
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    IResult, Parser,
};
use std::{collections::HashMap, fmt::Display, fs::read_to_string};
use std::{str, time::Instant};

#[derive(Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Str(String),
    Boolean(bool),
    Num(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

type Result<'a, E, O = &'a str> = IResult<&'a str, O, E>;

#[derive(Debug)]
enum JsonError {
    NomError(ErrorKind),
    Custom(String),
}

trait FromStr {
    fn from_str(value: &str) -> Self;
}

impl Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::NomError(message) => write!(f, "{message:?}"),
            JsonError::Custom(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for JsonError {}

impl<T> ParseError<T> for JsonError {
    fn from_error_kind(input: T, kind: nom::error::ErrorKind) -> Self {
        Self::NomError(kind)
    }

    fn append(input: T, kind: nom::error::ErrorKind, other: Self) -> Self {
        Self::NomError(kind)
    }
}

impl FromStr for JsonError {
    fn from_str(value: &str) -> Self {
        Self::Custom(value.to_owned())
    }
}

// impl<'a> FromStr for Error<&'a str> {
//     fn from_str(_value: &str) -> Self {
//         Self::new("", ErrorKind::Fail)
//     }
// }

// impl<T, E> FromExternalError<T, E> for JsonError {
//     fn from_external_error(input: T, _kind: ErrorKind, _e: E) -> Self {
//         Self::NomError(_kind)
//     }
// }

fn parse_str<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E> {
    escaped(alphanumeric, '\\', one_of("\"n\\"))(i)
}

fn parse_true<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E, bool> {
    value(true, tag("true"))(i)
}

fn parse_false<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E, bool> {
    value(false, tag("false"))(i)
}

fn null<'a, E: ParseError<&'a str>>(input: &'a str) -> Result<E, ()> {
    value((), tag("null")).parse(input)
}

fn u16_hex<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E, u16> {
    map(take(4usize), |s: &'a str| {
        u16::from_str_radix(s, 16).unwrap()
    })(i)
}

fn unicode_escape<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E, char> {
    map_opt(
        alt((
            // Not a surrogate
            map(verify(u16_hex, |cp| !(0xD800..0xE000).contains(cp)), |cp| {
                cp as u32
            }),
            // See https://en.wikipedia.org/wiki/UTF-16#Code_points_from_U+010000_to_U+10FFFF for details
            map(
                verify(
                    separated_pair(u16_hex, tag("\\u"), u16_hex),
                    |(high, low)| (0xD800..0xDC00).contains(high) && (0xDC00..0xE000).contains(low),
                ),
                |(high, low)| {
                    let high_ten = (high as u32) - 0xD800;
                    let low_ten = (low as u32) - 0xDC00;
                    (high_ten << 10) + low_ten + 0x10000
                },
            ),
        )),
        // Could probably be replaced with .unwrap() or _unchecked due to the verify checks
        std::char::from_u32,
    )(i)
}

fn parse_char<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E, char> {
    let (i, c) = anychar(i)?;

    if c == '\"' {
        return Err(nom::Err::Error(E::from_char(i, c)));
    }

    if c == '\\' {
        let (i, escaped_char) = anychar(i)?;
        let final_char = match escaped_char {
            '"' | '\\' | '/' => escaped_char,
            'b' => '\x08',
            'f' => '\x0C',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => return unicode_escape(i),
            c => return Err(nom::Err::Failure(E::from_char(i, c))),
        };

        Ok((i, final_char))
    } else {
        Ok((i, c))
    }
}

fn string<'a, E: ParseError<&'a str> + ContextError<&'a str>>(i: &'a str) -> Result<E, String> {
    context(
        "string",
        preceded(
            cut(tag("\"")),
            terminated(
                fold_many0(parse_char, String::new, |mut string, c| {
                    string.push(c);
                    string
                }),
                cut(char('"')),
            ),
        ),
    )(i)
}

fn array<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<E, Vec<JsonValue>> {
    context(
        "array",
        delimited(
            cut(char('[')),
            cut(separated_list0(preceded(multispace0, char(',')), |i| {
                let (i, next_char) = peek(anychar)(i)?;

                if next_char == ']' {
                    return Err(nom::Err::Error(E::from_char(i, next_char)));
                }

                json_value(i)
            })),
            preceded(multispace0, char(']')),
        ),
    )(i)
}

fn key_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<E, (String, JsonValue)> {
    let (i, _) = multispace0(i)?;

    let (i, next_char) = peek(anychar)(i)?;

    if next_char == '}' {
        return Err(nom::Err::Error(E::from_char(i, next_char)));
    }

    separated_pair(string, cut(preceded(multispace0, char(':'))), json_value).parse(i)
}

fn hash<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<E, HashMap<String, JsonValue>> {
    context(
        "map",
        preceded(
            cut(tag("{")),
            cut(terminated(
                map(
                    separated_list0(preceded(multispace0, char(',')), key_value),
                    |tuple_vec| tuple_vec.into_iter().collect(),
                ),
                preceded(multispace0, char('}')),
            )),
        ),
    )
    .parse(i)
}

fn json_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<E, JsonValue> {
    let (i, _) = many0(multispace1)(i)?;

    let (i, first_char) = peek(anychar)(i)?;

    match first_char {
        '{' => map(hash, JsonValue::Object)(i),
        '[' => map(array, JsonValue::Array)(i),
        '"' => map(string, JsonValue::Str)(i),
        '-' | '0'..='9' => map(double, JsonValue::Num)(i),
        'f' => map(parse_false, JsonValue::Boolean)(i),
        't' => map(parse_true, JsonValue::Boolean)(i),
        'n' => map(null, |_| JsonValue::Null)(i),
        c => Err(nom::Err::Failure(E::from_char(i, c))),
    }
}

fn parse(i: &str) -> Result<VerboseError<&str>, JsonValue> {
    terminated(json_value, multispace0).parse(i)
}

// fn main() {
//     let data = r#"{"foo🤔bar": 42}"#;
//
//     println!("Supported parsing {:#?}", parse(data));
// }
fn main() {
    let json = read_to_string("./test-files/twitter.json").unwrap();

    let start = Instant::now();
    let res = parse(&json);

    println!("Elapsed time: {:?}", start.elapsed());

    match res {
        Ok(_) => println!("Success"),
        Err(e) => {
            println!("Oh no: {}", e);
        }
    }
}
