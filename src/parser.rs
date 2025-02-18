use nom::bytes::take_while1;
use nom::combinator::value;
use nom::{Parser, AsChar, character};
use nom::branch::alt;
use nom::bytes::complete::{tag, escaped_transform};
use nom::error::{ParseError, make_error, ErrorKind};
use nom::multi::separated_list1;
use nom::sequence::{separated_pair, tuple};
use nom::{IResult, sequence::delimited};
use nom::character::complete::{char, anychar, none_of, one_of, digit1};

/// Form Fuzzing Language
/// functions:
/// static_str(key,val)
/// email(key,domains: arr)
/// choose_any(kvps: arr<(k,v)>)
/// choose_n(n: usize,kvps: arr<(k,v)>)
/// cellphone(name)
/// date(name,min,max)
/// optional(key,val)
/// string(name,maxlen)
/// name(name,maxlen)
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// generate a random email.
    /// name is the name of the parameter
    Email {name: String, domains: Vec<String>},
    YesNo{name: String},
    CellPhone {name: String},
    ChooseAny {options: Vec<(String, String)>},
    ChooseN{n: usize, kvps: Vec<(String, String)>},

    /// outputs in mm/dd/yyyy format
    Date {name: String, min: usize, max: usize},

    /// each checkbox is actually a different param specified by name,value, and then a random subset of them
    /// is selected
    CheckBoxes {kvps: Vec<(String, String)>},

    String {name: String, max_len: usize},

    OptionalString {name: String ,},
    
    Static {name: String, val: String},
    
    Name {name: String, max_len: usize}
}

/// esc characters:
/// - \( -> (
/// - \) -> )
/// - \n -> newline
/// - \" -> "
fn parse_string(input: &str) -> IResult<&str, String> {
    delimited(
        char('\"'),
        escaped_transform(
            take_while1(|c| c != '\\' && c != '\"'),
            '\\',
            alt((
                value("(", char('(')),
                value(")", char(')')),
                value("\n", char('n')),
                value("\"", char('"')),
                value("\\", char('\\')),
            ))
        ),
        char('\"')
    ).parse(input)
}

fn parse_kvp(input: &str) -> IResult<&str, (String, String)> {
    delimited(
        char('('),
        separated_pair(parse_string, char(','), parse_string),
        char(')')
    ).parse(input)
}

fn parse_arr_custom<'a, F, E>(matcher: F) -> impl Parser<&'a str, Output = Vec<<F as Parser<&'a str>>::Output>, Error = E>
where
    E: ParseError<&'a str>,
    F: Parser<&'a str, Error = E>
{
    delimited(char('['), separated_list1(char(','), matcher), char(']'))
}

fn parse_name(input: &str) -> IResult<&str, Parameter> {
    let inner = (parse_string, char(','), take_while1(AsChar::is_dec_digit));
    let (input, (name, _, max_len)) = delimited(tag("name("), 
        inner
        , tag(")")).parse(input)?;
    return IResult::Ok((input, Parameter::Name{name: name.to_string(), max_len: str::parse(max_len).unwrap()}));
}

fn parse_static(input: &str) -> IResult<&str, Parameter> {
    let kvp = delimited(tag("static("), 
        separated_pair(parse_string, char(','), parse_string)
        , tag(")")).parse(input)?;
    return IResult::Ok((kvp.0.into(), Parameter::Static{name:kvp.1.0.to_owned(), val:kvp.1.1.to_owned()}));
}


fn parse_email(input: &str) -> IResult<&str, Parameter> {
    let result = delimited(tag("email("), 
        separated_pair(parse_string, char(','), parse_arr_custom(parse_string)),
        tag(")")).parse(input)?;
    return IResult::Ok((result.0, Parameter::Email{
        name:result.1.0.to_string(), 
        domains:result.1.1}))
}

fn parse_choose_any(input: &str) -> IResult<&str, Parameter> {
    let result = delimited(
        tag("choose_any("),
        parse_arr_custom(parse_kvp),
        tag(")")
    ).parse(input)?;
    Ok((result.0, Parameter::ChooseAny { options: result.1 }))
}

fn parse_choose_n(input: &str) -> IResult<&str, Parameter> {
    let func = separated_pair(
        take_while1(|c| AsChar::is_dec_digit(c)), 
        tag(","),
        parse_arr_custom(parse_kvp)
    );
    let result = delimited(
        tag("choose_n("),
        func,
        tag(")")
    ).parse(input)?;
    Ok((result.0, Parameter::ChooseN { n: result.1.0.parse::<usize>().unwrap(), kvps: result.1.1 }))
}

fn parse_cellphone(input: &str) -> IResult<&str, Parameter> {
    let result = delimited(tag("cellphone("), parse_string, tag(")")).parse(input)?;
    return IResult::Ok((result.0, Parameter::CellPhone{name: result.1}));
}

fn parse_date(input: &str) -> IResult<&str, Parameter> {
    let name_dates = (parse_string, char(','), take_while1(AsChar::is_dec_digit), char(','), take_while1(AsChar::is_dec_digit));
    let (input, (name,_,d1,_,d2)) = delimited(
        tag("date("),
        name_dates,
        tag(")")
    ).parse(input)?;
    return IResult::Ok((input, Parameter::Date{
        name: name.to_string(), 
        min: str::parse::<usize>(d1).unwrap(), 
        max: str::parse::<usize>(d2).unwrap()
    }));
}


fn parse_string_entry(input: &str) -> IResult<&str, Parameter> {
    let inner = (parse_string, char(','), take_while1(AsChar::is_dec_digit));
    let (input, (name, _, max_len)) = delimited(tag("string("), 
        inner
        , tag(")")).parse(input)?;
    return IResult::Ok((input, Parameter::String{name: name.to_string(), max_len: str::parse(max_len).unwrap()}));
}

pub fn parse_line(input: &str) -> IResult<&str, Parameter> {
    alt((
        parse_static, 
        parse_email, 
        parse_choose_n, 
        parse_cellphone, 
        parse_choose_any, 
        parse_date, 
        parse_string_entry, 
        parse_name
    )).parse(input)
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_date;
    use super::*;
    
    #[test]
    fn test_parse_string() {
        println!("{:?}",parse_string("\"hello\""));
        assert_eq!(parse_string("\"hello\""), Ok(("", "hello".to_string())));
        assert_eq!(parse_string("\"esca\\\"ped\\(chars\\)\""), Ok(("", "esca\"ped(chars)".to_string())));
        assert_eq!(parse_string("\"newline\\n\""), Ok(("", "newline\n".to_string())));
    }
    
    #[test]
    fn test_parse_static() {
        assert_eq!(
            parse_static("static(\"name\",\"value\")"),
            Ok(("", Parameter::Static { name: "name".to_string(), val: "value".to_string() }))
        );
    }
    
    #[test]
    fn test_parse_email() {
        assert_eq!(
            parse_email("email(\"user\",[\"gmail.com\",\"yahoo.com\"])"),
            Ok(("", Parameter::Email { name: "user".to_string(), domains: vec!["gmail.com".to_string(), "yahoo.com".to_string()] }))
        );
    }
    
    #[test]
    fn test_parse_choose_any() {
        // Each key-value pair is parsed as a tuple ("key","value")
        assert_eq!(
            parse_choose_any("choose_any([(\"key1\",\"val1\"),(\"key2\",\"val2\")])"),
            Ok(("", Parameter::ChooseAny { options: vec![
                ("key1".to_string(), "val1".to_string()),
                ("key2".to_string(), "val2".to_string())
            ] }))
        );
    }
    
    #[test]
    fn test_parse_choose_n() {
        // First argument is the number, then a list of key-value pairs.
        assert_eq!(
            parse_choose_n("choose_n(2,[(\"key1\",\"val1\"),(\"key2\",\"val2\")])"),
            Ok(("", Parameter::ChooseN {
                n: 2,
                kvps: vec![
                    ("key1".to_string(), "val1".to_string()),
                    ("key2".to_string(), "val2".to_string())
                ]
            }))
        );
    }
    
    #[test]
    fn test_parse_cellphone() {
        assert_eq!(
            parse_cellphone("cellphone(\"myphone\")"),
            Ok(("", Parameter::CellPhone { name: "myphone".to_string() }))
        );
    }
    
    #[test]
    fn test_parse_date() {
        assert_eq!(
            parse_date("date(\"name\",11,44)"),
            Ok(("", Parameter::Date { name: "name".to_string(), min: 11, max: 44 }))
        );
    }
    
    #[test]
    fn test_parse_string_entry() {
        assert_eq!(
            parse_string_entry("string(\"name\",11)"),
            Ok(("", Parameter::String { name: "name".to_string(), max_len:11 }))
        );
    }
}
