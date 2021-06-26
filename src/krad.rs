use nom::{
    branch::alt,
    bytes::{
        complete::{tag, take_until},
        streaming::is_not,
    },
    character::complete::char,
    combinator::{map, value},
    multi::separated_list1,
    sequence::{pair, separated_pair},
    IResult,
};

// Note: requires newline before eof

const SEPARATOR: &[u8] = " : ".as_bytes();

// Todo: Shouldn't need to clone this
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanjiParts<'a> {
    kanji: &'a [u8],
    radicals: Vec<&'a [u8]>,
}

pub fn lines(b: &[u8]) -> IResult<&[u8], Vec<KanjiParts>> {
    let (i, o) = separated_list1(char('\n'), line)(b)?;
    let kanji: Vec<_> = o.into_iter().filter_map(|e| e).collect();
    Ok((i, kanji))
}

fn line(b: &[u8]) -> IResult<&[u8], Option<KanjiParts>> {
    alt((value(None, comment), map(kanji_line, |k| Some(k))))(b)
}

fn kanji_line(b: &[u8]) -> IResult<&[u8], KanjiParts> {
    let (i, o) = separated_pair(take_until(SEPARATOR), tag(SEPARATOR), radicals)(b)?;
    let (kanji, radicals) = o;
    let parts = KanjiParts { kanji, radicals };
    Ok((i, parts))
}

fn radicals(b: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    separated_list1(char(' '), radical)(b)
}

fn radical(b: &[u8]) -> IResult<&[u8], &[u8]> {
    is_not(" \n")(b)
}

fn comment(b: &[u8]) -> IResult<&[u8], ()> {
    let (i, _o) = pair(char('#'), is_not("\n"))(b)?;
    Ok((i, ()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn is_comment() -> Result<()> {
        let (_i, o) = comment("# September 2007\n".as_bytes())?;
        assert_eq!(o, ());
        Ok(())
    }

    #[test]
    fn parses_radical() -> Result<()> {
        let res = radical("�� �� ��\n".as_bytes())?;
        assert_eq!(res, (" �� ��\n".as_bytes(), "��".as_bytes()));
        Ok(())
    }

    #[test]
    fn parses_radicals() -> Result<()> {
        let res = radicals("�� �� ��\n".as_bytes())?;
        assert_eq!(
            res,
            (
                "\n".as_bytes(),
                vec!["��".as_bytes(), "��".as_bytes(), "��".as_bytes()]
            )
        );
        Ok(())
    }

    #[test]
    fn parses_kanji() -> Result<()> {
        let res = kanji_line("��� : �� �� �� �� ��\n".as_bytes())?;
        assert_eq!(
            res,
            (
                "\n".as_bytes(),
                KanjiParts {
                    kanji: "���".as_bytes(),
                    radicals: vec![
                        "��".as_bytes(),
                        "��".as_bytes(),
                        "��".as_bytes(),
                        "��".as_bytes(),
                        "��".as_bytes(),
                    ],
                }
            )
        );
        Ok(())
    }

    #[test]
    fn parses_line_as_kanji() -> Result<()> {
        let res = line("��� : �� �� �� �� ��\n".as_bytes())?;
        assert_eq!(
            res,
            (
                "\n".as_bytes(),
                Some(KanjiParts {
                    kanji: "���".as_bytes(),
                    radicals: vec![
                        "��".as_bytes(),
                        "��".as_bytes(),
                        "��".as_bytes(),
                        "��".as_bytes(),
                        "��".as_bytes(),
                    ],
                })
            )
        );
        Ok(())
    }

    #[test]
    fn parses_line_as_comment() -> Result<()> {
        let res = line("# September 2007\n".as_bytes())?;
        assert_eq!(res, ("\n".as_bytes(), None));
        Ok(())
    }

    #[test]
    fn parses_lines() -> Result<()> {
        let res = lines("��� : �� �� �� �� ��\n# September 2007\n".as_bytes())?;
        assert_eq!(
            res,
            (
                "\n".as_bytes(),
                vec![
                    Some(KanjiParts {
                        kanji: "���".as_bytes(),
                        radicals: vec![
                            "��".as_bytes(),
                            "��".as_bytes(),
                            "��".as_bytes(),
                            "��".as_bytes(),
                            "��".as_bytes(),
                        ],
                    }),
                    None,
                ],
            )
        );
        Ok(())
    }
}
