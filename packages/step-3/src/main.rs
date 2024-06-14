#![allow(unused)]

use core::panic;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take},
    character::complete::{
        alphanumeric1 as alphanumeric, anychar, char, multispace0, multispace1, none_of, one_of,
    },
    combinator::{cut, map, map_opt, peek, value, verify},
    error::{context, ContextError, Error, ParseError, VerboseError},
    multi::{fold_many0, many0, separated_list0},
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    IResult, Parser,
};
use std::{collections::HashMap, fs::read_to_string};
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
    let (i, c) = none_of("\"")(i)?;

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
            c => {
                panic!("Invalid escaped char: {c}");
            }
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
            cut(char('"')),
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
            cut(separated_list0(
                preceded(multispace0, char(',')),
                json_value,
            )),
            preceded(multispace0, char(']')),
        ),
    )(i)
}

fn key_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<E, (String, JsonValue)> {
    separated_pair(
        preceded(multispace0, string),
        cut(preceded(multispace0, char(':'))),
        json_value,
    )
    .parse(i)
}

fn hash<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<E, HashMap<String, JsonValue>> {
    println!("Parsed hash");
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
        c => {
            panic!("Unexpected char: {c} {i}");
        }
    }
}

fn parse(i: &str) -> Result<VerboseError<&str>, JsonValue> {
    terminated(json_value, multispace0).parse(i)
}

// fn main() {
//     let now_valid = r#"{"あ": "world"}"#;
//
//     println!("Supported parsing {:#?}", parse(now_valid));
// }

fn main() {
    let json = read_to_string("./test-files/fail.json").unwrap();

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
