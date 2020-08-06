use core::ops::Range;

use std::{borrow::Cow, collections::BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub index: usize,
    pub ty: Type,
    pub span: Range<usize>,
}

struct Param {
    index: Option<usize>,
    ty: Type,
    span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    BitField(Range<u8>),
    Bool,
    Format, // "{:?}"
    I8,
    I16,
    I32,
    Isize,
    /// String slice (i.e. passed directly; not as interned string indices).
    Str,
    /// Interned string index.
    IStr,
    U8,
    U16,
    U24,
    U32,
    Usize,
    /// Byte slice `{:[u8]}`.
    Slice,
    Array(usize),
    F32,
}

fn is_digit(c: Option<char>) -> bool {
    match c.unwrap_or('\0') {
        '0'..='9' => true,
        _ => false,
    }
}

static EOF: &str = "expected `}` but string was terminated";

fn parse_usize(s: &str) -> Result<Option<(usize, usize /* consumed */)>, &'static str> {
    if is_digit(s.chars().next()) {
        if let Some(end) = s.chars().position(|c| !is_digit(Some(c))) {
            let x = s[..end]
                .parse::<usize>()
                .map_err(|_| "position index must fit in `usize`")?;
            Ok(Some((x, end)))
        } else {
            Err(EOF.into())
        }
    } else {
        Ok(None)
    }
}

fn parse_range(mut s: &str) -> Option<(Range<u8>, usize /* consumed */)> {
    let start_digits = s
        .as_bytes()
        .iter()
        .take_while(|b| is_digit(Some(**b as char)))
        .count();
    let start = s[..start_digits].parse().ok()?;
    if &s[start_digits..start_digits + 2] != ".." {
        return None;
    }
    s = &s[start_digits + 2..];
    let end_digits = s
        .as_bytes()
        .iter()
        .take_while(|b| is_digit(Some(**b as char)))
        .count();
    let end = s[..end_digits].parse().ok()?;

    if end <= start {
        return None;
    }

    if start >= 32 || end >= 32 {
        return None;
    }

    Some((start..end, start_digits + end_digits + 2))
}

pub fn parse(format_string: &str) -> Result<Vec<Parameter>, Cow<'static, str>> {
    let s = format_string;
    let mut chars = s.char_indices();

    let mut params = Vec::<Param>::new();
    while let Some((span_start, c)) = chars.next() {
        match c {
            '{' => {
                let len_start = chars.as_str().len();
                let index = if let Some((idx, skip)) = parse_usize(chars.as_str())? {
                    for _ in 0..skip {
                        drop(chars.next())
                    }
                    Some(idx)
                } else {
                    None
                };

                let c = chars.next().map(|(_, c)| c);
                match c {
                    // escaped `{`
                    Some('{') => {}

                    // format argument
                    Some(':') => {
                        static BOOL: &str = "bool}";
                        static FMT: &str = "?}";
                        static I16: &str = "i16}";
                        static I32: &str = "i32}";
                        static I8: &str = "i8}";
                        static SLICE: &str = "[u8]}";
                        static STR: &str = "str}";
                        static ISTR: &str = "istr}";
                        static U16: &str = "u16}";
                        static U24: &str = "u24}";
                        static U32: &str = "u32}";
                        static F32: &str = "f32}";
                        static U8: &str = "u8}";
                        static USIZE: &str = "usize}";
                        static ISIZE: &str = "isize}";

                        static ARRAY_START: &str = "[u8;";

                        let s = chars.as_str();
                        let ty = if s.starts_with(FMT) {
                            (0..FMT.len()).for_each(|_| drop(chars.next()));
                            Type::Format
                        } else if s.starts_with(STR) {
                            (0..STR.len()).for_each(|_| drop(chars.next()));
                            Type::Str
                        } else if s.starts_with(ISTR) {
                            (0..ISTR.len()).for_each(|_| drop(chars.next()));
                            Type::IStr
                        } else if s.starts_with(U8) {
                            (0..U8.len()).for_each(|_| drop(chars.next()));
                            Type::U8
                        } else if s.starts_with(U16) {
                            (0..U16.len()).for_each(|_| drop(chars.next()));
                            Type::U16
                        } else if s.starts_with(U24) {
                            (0..U24.len()).for_each(|_| drop(chars.next()));
                            Type::U24
                        } else if s.starts_with(U32) {
                            (0..U32.len()).for_each(|_| drop(chars.next()));
                            Type::U32
                        } else if s.starts_with(F32) {
                            (0..F32.len()).for_each(|_| drop(chars.next()));
                            Type::F32
                        } else if s.starts_with(I8) {
                            (0..I8.len()).for_each(|_| drop(chars.next()));
                            Type::I8
                        } else if s.starts_with(I16) {
                            (0..I16.len()).for_each(|_| drop(chars.next()));
                            Type::I16
                        } else if s.starts_with(I32) {
                            (0..I32.len()).for_each(|_| drop(chars.next()));
                            Type::I32
                        } else if s.starts_with(BOOL) {
                            (0..BOOL.len()).for_each(|_| drop(chars.next()));
                            Type::Bool
                        } else if s.starts_with(SLICE) {
                            (0..SLICE.len()).for_each(|_| drop(chars.next()));
                            Type::Slice
                        } else if s.starts_with(USIZE) {
                            (0..USIZE.len()).for_each(|_| drop(chars.next()));
                            Type::Usize
                        } else if s.starts_with(ISIZE) {
                            (0..ISIZE.len()).for_each(|_| drop(chars.next()));
                            Type::Isize
                        } else if s.starts_with(ARRAY_START) {
                            (0..ARRAY_START.len()).for_each(|_| drop(chars.next()));
                            let len_index = chars
                                .as_str()
                                .find(|c: char| c != ' ')
                                .ok_or("invalid array specifier")?;
                            (0..len_index).for_each(|_| drop(chars.next()));
                            let (len, used) = parse_usize(chars.as_str())?
                                .ok_or("expected array length literal")?;
                            (0..used).for_each(|_| drop(chars.next()));
                            if !chars.as_str().starts_with("]}") {
                                return Err("missing `]}` after array length".into());
                            }
                            chars.next();
                            chars.next();
                            Type::Array(len)
                        } else {
                            let (range, used) = parse_range(s).ok_or("invalid format argument")?;
                            (0..used).for_each(|_| drop(chars.next()));
                            if !chars.as_str().starts_with("}") {
                                return Err("missing `}` after bitfield range".into());
                            }
                            chars.next();
                            Type::BitField(range)
                        };

                        if let Some(i) = index {
                            for param in &params {
                                if param.index == index && param.ty != ty {
                                    if let (Type::BitField(_), Type::BitField(_)) = (&param.ty, &ty)
                                    {
                                        // Multiple bitfield uses with different ranges are fine.
                                        continue;
                                    }

                                    return Err(format!(
                                        "argument {} assigned more than one type",
                                        i
                                    )
                                    .into());
                                }
                            }
                        }

                        let len = len_start - chars.as_str().len() + 1;
                        let span = span_start..span_start + len;
                        params.push(Param { ty, index, span })
                    }

                    Some(_) => return Err("`{` must be followed by `:`".into()),

                    None => return Err(EOF.into()),
                }
            }

            '}' => {
                // must be a escaped `}`
                if chars.next().map(|(_, c)| c) != Some('}') {
                    return Err("unmatched `}` in format string".into());
                }
            }

            '@' => return Err("format string cannot contain the `@` character".into()),

            _ => {}
        }
    }

    assign_indices(params)
}

fn assign_indices(params: Vec<Param>) -> Result<Vec<Parameter>, Cow<'static, str>> {
    let mut used = BTreeSet::new();

    let mut i = 0;
    let mut parameters = vec![];
    for param in params {
        let index = if let Some(i) = param.index {
            i
        } else {
            while used.contains(&i) {
                i += 1;
            }
            i
        };

        used.insert(index);
        parameters.push(Parameter {
            index,
            ty: param.ty,
            span: param.span,
        });
    }

    for (i, j) in used.iter().enumerate() {
        if i != *j {
            return Err("the format string contains unused positions".into());
        }
    }

    Ok(parameters)
}

#[cfg(test)]
mod tests {
    use super::{Parameter, Type};

    #[test]
    fn parse_usize() {
        assert_eq!(super::parse_usize("2}"), Ok(Some((2, 1))));
        assert_eq!(super::parse_usize("12}"), Ok(Some((12, 2))));
        assert_eq!(super::parse_usize("001}"), Ok(Some((1, 3))));
    }

    #[test]
    fn ty() {
        let fmt = "{:bool}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Bool,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:?}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Format,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:i16}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::I16,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:i32}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::I32,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:i8}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::I8,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:str}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Str,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:u16}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::U16,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:u24}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::U24,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:u32}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::U32,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:f32}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::F32,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:u8}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::U8,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:[u8]}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Slice,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:usize}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Usize,
                span: 0..fmt.len(),
            }])
        );

        let fmt = "{:isize}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Isize,
                span: 0..fmt.len(),
            }])
        );
    }

    #[test]
    fn index() {
        // implicit
        let a = "{:u8}";
        let b = "{:u16}";
        assert_eq!(
            super::parse(&format!("{} {}", a, b)),
            Ok(vec![
                Parameter {
                    index: 0,
                    ty: Type::U8,
                    span: 0..a.len(),
                },
                Parameter {
                    index: 1,
                    ty: Type::U16,
                    span: a.len() + 1..a.len() + b.len() + 1,
                }
            ])
        );

        // single parameter formatted twice
        let a = "{:u8}";
        let b = "{0:u8}";
        assert_eq!(
            super::parse(&format!("{} {}", a, b)),
            Ok(vec![
                Parameter {
                    index: 0,
                    ty: Type::U8,
                    span: 0..a.len(),
                },
                Parameter {
                    index: 0,
                    ty: Type::U8,
                    span: a.len() + 1..a.len() + b.len() + 1,
                }
            ])
        );

        // explicit index
        let a = "{:u8}";
        let b = "{1:u16}";
        assert_eq!(
            super::parse(&format!("{} {}", a, b)),
            Ok(vec![
                Parameter {
                    index: 0,
                    ty: Type::U8,
                    span: 0..a.len(),
                },
                Parameter {
                    index: 1,
                    ty: Type::U16,
                    span: a.len() + 1..a.len() + b.len() + 1,
                }
            ])
        );

        // reversed order
        let a = "{1:u8}";
        let b = "{0:u16}";
        assert_eq!(
            super::parse(&format!("{} {}", a, b)),
            Ok(vec![
                Parameter {
                    index: 1,
                    ty: Type::U8,
                    span: 0..a.len(),
                },
                Parameter {
                    index: 0,
                    ty: Type::U16,
                    span: a.len() + 1..a.len() + b.len() + 1,
                }
            ])
        );

        // two different types for the same index
        assert!(super::parse("{0:u8} {0:u16}").is_err());

        // omitted index 0
        assert!(super::parse("{1:u8}").is_err());

        // index 1 is missing
        assert!(super::parse("{2:u8} {:u16}").is_err());

        // index 0 is missing
        assert!(super::parse("{2:u8} {1:u16}").is_err());
    }

    #[test]
    fn range() {
        let fmt = "{:0..4}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::BitField(0..4),
                span: 0..fmt.len(),
            }])
        );

        let a = "{0:30..31}";
        let b = "{1:0..4}";
        let c = "{1:2..6}";
        assert_eq!(
            super::parse(&format!("{} {} {}", a, b, c)),
            Ok(vec![
                Parameter {
                    index: 0,
                    ty: Type::BitField(30..31),
                    span: 0..a.len(),
                },
                Parameter {
                    index: 1,
                    ty: Type::BitField(0..4),
                    span: a.len() + 1..a.len() + 1 + b.len(),
                },
                Parameter {
                    index: 1,
                    ty: Type::BitField(2..6),
                    span: a.len() + 1 + b.len() + 1..a.len() + 1 + b.len() + 1 + c.len(),
                }
            ])
        );

        // empty range
        assert!(super::parse("{:0..0}").is_err());
        // start > end
        assert!(super::parse("{:1..0}").is_err());
        // out of 32-bit range
        assert!(super::parse("{:0..32}").is_err());
        // just inside 32-bit range
        assert!(super::parse("{:0..31}").is_ok());

        // missing parts
        assert!(super::parse("{:0..4").is_err());
        assert!(super::parse("{:0..}").is_err());
        assert!(super::parse("{:..4}").is_err());
        assert!(super::parse("{:0.4}").is_err());
        assert!(super::parse("{:0...4}").is_err());
    }

    #[test]
    fn arrays() {
        let fmt = "{:[u8; 0]}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Array(0),
                span: 0..fmt.len(),
            }])
        );

        // Space is optional.
        let fmt = "{:[u8;42]}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Array(42),
                span: 0..fmt.len(),
            }])
        );

        // Multiple spaces are ok.
        let fmt = "{:[u8;    257]}";
        assert_eq!(
            super::parse(fmt),
            Ok(vec![Parameter {
                index: 0,
                ty: Type::Array(257),
                span: 0..fmt.len(),
            }])
        );

        // No tabs or other whitespace.
        assert!(super::parse("{:[u8; \t 3]}").is_err());
        assert!(super::parse("{:[u8; \n 3]}").is_err());
        // Too large.
        assert!(super::parse("{:[u8; 9999999999999999999999999]}").is_err());
    }

    #[test]
    fn error_msg() {
        assert_eq!(
            super::parse("{:dunno}"),
            Err("invalid format argument".into())
        );
    }
}
