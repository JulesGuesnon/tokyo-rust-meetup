#![allow(unused)]

use nom::{
    bytes::complete::{tag, tag_no_case, take_while},
    character::{is_newline, is_space},
    combinator::{map, opt},
    error::ParseError,
    sequence::{delimited, separated_pair},
    IResult, Parser,
};
use std::str;

type Result<'a, E, O = &'a str> = IResult<&'a str, O, E>;

#[derive(Debug)]
struct HelloWorld {
    pub hello: String,
    pub world: String,
    pub is_happy: bool,
}

fn spaces<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E> {
    take_while(|c| is_space(c as u8) || is_newline(c as u8))(i)
}

fn hello_world<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<E, (&'a str, &'a str, bool)> {
    separated_pair(tag_no_case("hello"), spaces, tag_no_case("world"))(i).and_then(
        |(rest, (hello, world))| {
            opt(tag("!"))(rest).map(|(rest, is_happy)| (rest, (hello, world, is_happy.is_some())))
        },
    )
}

fn parse(i: &str) -> Result<nom::error::VerboseError<&str>, HelloWorld> {
    map(
        delimited(spaces, hello_world, spaces),
        |(hello, world, is_happy)| HelloWorld {
            hello: hello.to_owned(),
            world: world.to_owned(),
            is_happy,
        },
    )(i)
}

fn main() {
    let input1 = "Hello world";
    let input2 = "Hello World";
    let input3 = "hello World!";
    let input4 = "    hello      World!   ";
    let input5 = "   \n hello    not  World!   ";
    let input6 = "Helo world";

    println!("{:?}", parse(input1));
    println!("{:?}", parse(input2));
    println!("{:?}", parse(input3));
    println!("{:?}", parse(input4));
    println!("{:?}", parse(input5));
    println!("{:?}", parse(input6));
}
