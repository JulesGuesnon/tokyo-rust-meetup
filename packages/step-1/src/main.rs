use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while},
    character::complete::{alphanumeric1 as alphanumeric, char, one_of},
    combinator::{cut, map, value},
    error::{context, ContextError, ParseError, VerboseError},
    multi::separated_list0,
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

type Result<'a, O, E> = IResult<&'a str, O, E>;

fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<&'a str, E> {
    let chars = " \t\r\n";

    // nom combinators like `take_while` return a function. That function is the
    // parser,to which we can pass the input
    take_while(move |c| chars.contains(c))(i)
}

fn parse_str<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<&'a str, E> {
    escaped(alphanumeric, '\\', one_of("\"n\\"))(i)
}

fn boolean<'a, E: ParseError<&'a str>>(input: &'a str) -> Result<bool, E> {
    let parse_true = value(true, tag("true"));

    let parse_false = value(false, tag("false"));

    alt((parse_true, parse_false)).parse(input)
}

fn null<'a, E: ParseError<&'a str>>(input: &'a str) -> Result<(), E> {
    value((), tag("null")).parse(input)
}

fn string<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context(
        "string",
        preceded(char('\"'), cut(terminated(parse_str, char('\"')))),
    )
    .parse(i)
}

fn array<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<Vec<JsonValue>, E> {
    context(
        "array",
        preceded(
            char('['),
            cut(terminated(
                separated_list0(preceded(sp, char(',')), json_value),
                preceded(sp, char(']')),
            )),
        ),
    )
    .parse(i)
}

fn key_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<(&'a str, JsonValue), E> {
    separated_pair(
        preceded(sp, string),
        cut(preceded(sp, char(':'))),
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
            char('{'),
            cut(terminated(
                map(
                    separated_list0(preceded(sp, char(',')), key_value),
                    |tuple_vec| {
                        tuple_vec
                            .into_iter()
                            .map(|(k, v)| (String::from(k), v))
                            .collect()
                    },
                ),
                preceded(sp, char('}')),
            )),
        ),
    )
    .parse(i)
}

fn json_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> Result<JsonValue, E> {
    preceded(
        sp,
        alt((
            map(hash, JsonValue::Object),
            map(array, JsonValue::Array),
            map(string, |s| JsonValue::Str(String::from(s))),
            map(boolean, JsonValue::Boolean),
            map(null, |_| JsonValue::Null),
            map(double, JsonValue::Num),
        )),
    )
    .parse(i)
}

fn parse(i: &str) -> Result<JsonValue, VerboseError<&str>> {
    delimited(
        sp,
        alt((
            map(hash, JsonValue::Object),
            map(array, JsonValue::Array),
            map(null, |_| JsonValue::Null),
        )),
        sp,
    )
    .parse(i)
}

// fn main() {
//     let data = "  { \"a\"\t: 42,
//   \"b\": [ \"x\", \"y\", 12 ] ,
//   \"c\": { \"hello\" : \"world\"
//   }
//   } ";
//
//     println!(
//         "will try to parse valid JSON data:\n\n**********\n{}\n**********\n",
//         data
//     );
//
//     println!("parsing a valid file:\n{:#?}\n", parse(data));
//
//     let data = "  { \"a\"\t: 42,
//   \"b\": [ \"x\", \"y\", 12 ] ,
//   \"c\": { 1\"hello\" : \"world\"
//   }
//   } ";
//
//     println!(
//         "will try to parse invalid JSON data:\n\n**********\n{}\n**********\n",
//         data
//     );
//
//     println!("parsing a invalid file:\n{:#?}\n", parse(data));
// }

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
