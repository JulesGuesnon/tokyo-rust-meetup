#![allow(unused)]

use nom::{
    bytes::complete::{tag, tag_no_case, take_while},
    character::complete::char,
    combinator::{map, opt},
    error::{Error, ParseError},
    sequence::{delimited, separated_pair},
    IResult,
};

type Result<'a, O, E> = IResult<&'a str, O, E>;

#[derive(Debug)]
struct HelloWorld {
    pub hello: String,
    pub world: String,
    pub is_happy: bool,
}

fn hello<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<String, E> {
    let (i, hello) = tag_no_case("hello")(i)?;

    Ok((i, hello.to_owned()))
}

fn world<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<String, E> {
    let (i, world) = tag_no_case("world")(i)?;

    Ok((i, world.to_owned()))
}

fn is_happy<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<bool, E> {
    map(opt(char('!')), |opt| opt.is_some())(i)
}

fn whitespaces<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<&'a str, E> {
    take_while(|c| c == ' ' || c == '\n')(i)
}

fn hello_world<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<(String, String, bool), E> {
    let (i, (hello, world)) = separated_pair(hello, whitespaces, world)(i)?;

    let (i, is_happy) = is_happy(i)?;

    Ok((i, (hello, world, is_happy)))
}

fn parse<'a, E: ParseError<&'a str>>(i: &'a str) -> Result<HelloWorld, E> {
    map(
        delimited(whitespaces, hello_world, whitespaces),
        |(hello, world, is_happy)| HelloWorld {
            hello,
            world,
            is_happy,
        },
    )(i)
}

fn main() {
    let input1 = "Hello world";
    let input2 = "Hello World";
    let input3 = "heLlo World!";
    let input4 = "    hello      World!   ";
    let input5 = "   \n hello    not  World!   ";
    let input6 = "Helo world";

    println!("{:?}", parse::<Error<_>>(input1));
    println!("{:?}", parse::<Error<_>>(input2));
    println!("{:?}", parse::<Error<_>>(input3));
    println!("{:?}", parse::<Error<_>>(input4));
    println!("{:?}", parse::<Error<_>>(input5));
    println!("{:?}", parse::<Error<_>>(input6));
}

// type Result<'a, O, E> = IResult<&'a str, O, E>;
//
// fn main() {
//     let response = tag::<_, _, Error<_>>("hello")("hello world");
//
//     println!("{response:?}");
// }
