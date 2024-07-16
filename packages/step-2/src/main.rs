#![allow(unused)]

use core::panic;
use nom::{
    bytes::complete::{escaped, tag},
    character::complete::{
        alphanumeric1 as alphanumeric, anychar, char, multispace0, multispace1, one_of,
    },
    combinator::{cut, map, peek, value},
    error::{context, ContextError, ParseError, VerboseError},
    multi::{many0, separated_list0},
    number::complete::double,
    sequence::{preceded, separated_pair, terminated},
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

type Result<'a, O, E> = IResult<&'a str, O, E>;

fn parse_str<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<&'a str, E> {
    escaped(alphanumeric, '\\', one_of("\"n\\"))(i)
}

fn parse_true<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<bool, E> {
    value(true, tag("true"))(i)
}

fn parse_false<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<bool, E> {
    value(false, tag("false"))(i)
}

fn null<'a, E: ParseError<&'a str>>(input: &'a str) -> Result<(), E> {
    value((), tag("null")).parse(input)
}

fn string<'a, E: ParseError<&'a str> + ContextError<&'a str>>(i: &'a str) -> Result<&'a str, E> {
    context(
        "string",
        cut(preceded(char('"'), terminated(parse_str, char('"')))),
    )(i)
}

fn array<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<Vec<JsonValue>, E> {
    context(
        "array",
        preceded(
            cut(char('[')),
            cut(terminated(
                separated_list0(preceded(multispace0, char(',')), json_value),
                preceded(multispace0, char(']')),
            )),
        ),
    )(i)
}

fn key_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<(&'a str, JsonValue), E> {
    separated_pair(
        preceded(multispace0, string),
        cut(preceded(multispace0, char(':'))),
        json_value,
    )
    .parse(i)
}

fn hash<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<HashMap<String, JsonValue>, E> {
    context(
        "map",
        preceded(
            cut(char('{')),
            cut(terminated(
                map(
                    separated_list0(preceded(multispace0, char(',')), key_value),
                    |tuple_vec| {
                        tuple_vec
                            .into_iter()
                            .map(|(k, v)| (String::from(k), v))
                            .collect()
                    },
                ),
                preceded(multispace0, char('}')),
            )),
        ),
    )
    .parse(i)
}

fn json_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<JsonValue, E> {
    let (i, _) = multispace0(i)?;

    let (i, first_char) = peek(anychar)(i)?;

    match first_char {
        '{' => map(hash, JsonValue::Object)(i),
        '[' => map(array, JsonValue::Array)(i),
        '"' => map(string, |s| JsonValue::Str(String::from(s)))(i),
        '-' | '0'..='9' => map(double, JsonValue::Num)(i),
        'f' => map(parse_false, JsonValue::Boolean)(i),
        't' => map(parse_true, JsonValue::Boolean)(i),
        'n' => map(null, |_| JsonValue::Null)(i),
        c => {
            panic!("Unexpected char: {c}");
        }
    }
}

fn parse(i: &str) -> Result<JsonValue, VerboseError<&str>> {
    terminated(json_value, multispace0).parse(i)
}

fn main() {
    let json = read_to_string("./test-files/canada.json").unwrap();

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

// fn main() {
//     let invalid = r#"{"あ": "world"}"#;
//
//     println!("Unsupported parsing {:#?}", parse(invalid));
//     println!("\u{3042}");
// }
