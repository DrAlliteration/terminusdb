use nom::bytes::complete::{tag, take_while, take_while1};
use nom::multi::separated_list1;
use nom::sequence::preceded;
use nom::{
    branch::alt,
    combinator::{map, map_res},
    error::ErrorKind,
    sequence::{delimited, pair, separated_pair, terminated},
    IResult,
};
use std::rc::Rc;

/*

P := . | p(String)
Q,R := P> | <P | Q,R | Q;R | plus(Q) | star(Q) | times(Q,N,M)

 */

#[derive(Debug, PartialEq)]
enum Pred {
    Any,
    Named(String),
}

#[derive(Debug, PartialEq)]
enum Path {
    Seq(Vec<Path>),
    Choice(Vec<Path>),
    Positive(Pred),
    Negative(Pred),
    Plus(Rc<Path>),
    Star(Rc<Path>),
    Times(Rc<Path>, u32, u32),
}

fn is_property_char(c: char) -> bool {
    c.is_alphanumeric() || c == ':' || c == '/' || c == '_' || c == '-'
}

fn named(input: &str) -> IResult<&str, &str> {
    take_while1(is_property_char)(input)
}

fn num(input: &str) -> IResult<&str, u32> {
    map_res(
        take_while(|c: char| c.is_digit(10)),
        |digits: &str| -> Result<u32, _> { Ok::<u32, ErrorKind>(digits.parse::<u32>().unwrap()) },
    )(input)
}

fn pred(input: &str) -> IResult<&str, Pred> {
    alt((
        map(tag("."), |_| Pred::Any),
        map(named, |string| Pred::Named(string.to_string())),
    ))(input)
}

fn positive(input: &str) -> IResult<&str, Pred> {
    alt((terminated(pred, tag(">")), pred))(input)
}

fn negative(input: &str) -> IResult<&str, Pred> {
    preceded(tag("<"), pred)(input)
}

fn patterns(input: &str) -> IResult<&str, Path> {
    alt((
        delimited(tag("("), ands, tag(")")),
        map(negative, |elt| Path::Negative(elt)),
        map(positive, |elt| Path::Positive(elt)),
    ))(input)
}

fn plus(input: &str) -> IResult<&str, Path> {
    terminated(patterns, tag("+"))(input)
}

fn star(input: &str) -> IResult<&str, Path> {
    terminated(patterns, tag("*"))(input)
}

fn size_bracket(input: &str) -> IResult<&str, (u32, u32)> {
    delimited(tag("{"), separated_pair(num, tag(","), num), tag("}"))(input)
}

fn times(input: &str) -> IResult<&str, Path> {
    map(pair(patterns, size_bracket), |(p, (n, m))| {
        Path::Times(Rc::new(p), n, m)
    })(input)
}

fn repeat_patterns(input: &str) -> IResult<&str, Path> {
    alt((
        map(plus, |elt| Path::Plus(Rc::new(elt))),
        map(star, |elt| Path::Star(Rc::new(elt))),
        times,
        patterns,
    ))(input)
}

fn ors(input: &str) -> IResult<&str, Path> {
    alt((
        map(
            pair(
                terminated(repeat_patterns, tag("|")),
                separated_list1(tag("|"), repeat_patterns),
            ),
            |(first, paths)| {
                let mut result = paths;
                result.insert(0, first);
                Path::Choice(result)
            },
        ),
        repeat_patterns,
    ))(input)
}

fn ands(input: &str) -> IResult<&str, Path> {
    alt((
        map(
            pair(terminated(ors, tag(",")), separated_list1(tag(","), ors)),
            |(first, paths)| {
                let mut result = paths;
                result.insert(0, first);
                Path::Seq(result)
            },
        ),
        ors,
    ))(input)
}

fn path(input: &str) -> IResult<&str, Path> {
    ands(input)
}

// Composition of Kleisli arrows (>=>)
fn kleisli_compose<'a, R, S, T>(
    f: dyn Fn(R) -> dyn Iterator<Item = S>,
    g: dyn Fn(S) -> dyn Iterator<Item = T>,
) -> impl Fn(R) -> dyn Iterator<Item = T> {
    |x| f(x).flatmap(g)
}

/*
fn compile_path(
    path: Path,
) -> Box<Fn(IdTriple) -> dyn Iterator<Item = IdTriple> + 'a> {
    match path {
        Path::Seq(vec) => {
            if let Some(first) = vec.pop() {
                let iterfun = compile_path(first);
                for sub_path in vec {
                    iter = compile_path(sub_path)
                }
            }
        }
        Path::Choice(vec) => {
            let branch = iter.clone();
            let or_iter = Box::new(std::iter::empty());
            for sub_path in vec {
                or_iter = or_iter.chain(compile_path(branch, sub_path))
            }
            or_iter
        }
        Path::Positive(_) => todo!(),
        Path::Negative(_) => todo!(),
        Path::Plus(_) => todo!(),
        Path::Star(_) => todo!(),
        Path::Times(_, _, _) => todo!(),
    }
}
 */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pred() {
        let source = "rdf:first";
        let results = pred(source);
        assert_eq!(results, Ok(("", Pred::Named("rdf:first".to_string()))))
    }

    #[test]
    fn parse_cons_chaser() {
        let source = "p,rdf:rest*,rdf:first";
        let results = path(source);
        assert_eq!(
            results,
            Ok((
                "",
                Path::Seq(vec![
                    Path::Positive(Pred::Named("p".to_string())),
                    Path::Star(Rc::new(Path::Positive(Pred::Named("rdf:rest".to_string())))),
                    Path::Positive(Pred::Named("rdf:first".to_string()))
                ])
            ))
        )
    }

    #[test]
    fn parse_backward_repeat_group() {
        let source = "(<effect,cause)+";
        let results = path(source);
        assert_eq!(
            results,
            Ok((
                "",
                Path::Plus(Rc::new(Path::Seq(vec![
                    Path::Negative(Pred::Named("effect".to_string())),
                    Path::Positive(Pred::Named("cause".to_string()))
                ])))
            ))
        )
    }

    #[test]
    fn parse_any() {
        let source = ".";
        let results = path(source);
        assert_eq!(results, Ok(("", Path::Positive(Pred::Any))))
    }

    #[test]
    fn parse_something() {
        let source = "(forward,.,<backward)+";
        let results = path(source);
        assert_eq!(
            results,
            Ok((
                "",
                Path::Plus(Rc::new(Path::Seq(vec![
                    Path::Positive(Pred::Named("forward".to_string())),
                    Path::Positive(Pred::Any),
                    Path::Negative(Pred::Named("backward".to_string()))
                ])))
            ))
        )
    }

    #[test]
    fn repeated_choice() {
        let source = "(child|database)*";
        let results = path(source);
        assert_eq!(
            results,
            Ok((
                "",
                Path::Star(Rc::new(Path::Choice(vec![
                    Path::Positive(Pred::Named("child".to_string())),
                    Path::Positive(Pred::Named("database".to_string()))
                ])))
            ))
        )
    }

    #[test]
    fn and_then_n_m() {
        let source = "first,(second,third){1,4}";
        let results = path(source);
        assert_eq!(
            results,
            Ok((
                "",
                Path::Seq(vec![
                    Path::Positive(Pred::Named("first".to_string())),
                    Path::Times(
                        Rc::new(Path::Seq(vec![
                            Path::Positive(Pred::Named("second".to_string())),
                            Path::Positive(Pred::Named("third".to_string()))
                        ])),
                        1,
                        4
                    )
                ])
            ))
        )
    }
}
