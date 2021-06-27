use std::path::Path;

use super::jis213::jis_to_utf8;
use encoding::{codec::japanese::EUCJPEncoding, DecoderTrap, Encoding};
use nom::{
    bytes::{
        complete::{tag, take_until},
        streaming::is_not,
    },
    character::complete::char,
    combinator::{map, map_res, opt, value},
    multi::{separated_list0, separated_list1},
    sequence::{pair, separated_pair},
    IResult,
};
use thiserror::Error;

const SEPARATOR: &[u8] = " : ".as_bytes();

/// A decomposition of a kanji into its constituent radicals
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decomposition {
    /// The kanji character
    pub kanji: String,

    /// A list of characters representing the radicals in the kanji
    pub radicals: Vec<String>,
}

/// Enumerates the modules's possible errors
#[derive(Error, Debug)]
pub enum KradError {
    /// Invalid JIS X 0213 codepoint
    #[error("Invalid JIS X 0213 codepoint")]
    Jis,

    /// Invalid EUC-JP codepoint
    #[error("Invalid EUC-JP codepoint")]
    EucJp,

    /// Error while parsing kradfile
    #[error("Error while parsing kradfile")]
    Parse,

    /// Error while reading kradfile
    #[error("Error while reading kradfile")]
    Io(#[from] std::io::Error),
}

type KradResult = Result<Vec<Decomposition>, KradError>;

/// Parses a kradfile or kradfile2 and returns
/// the list of kanji radical decompositions
///
/// # Arguments
///
/// * `path` - A path to the kradfile
pub fn parse_file<P: AsRef<Path>>(path: P) -> KradResult {
    parse_file_implementation(path.as_ref())
}

// Monomorphisation bloat avoidal splitting
fn parse_file_implementation(path: &Path) -> KradResult {
    std::fs::read(path)
        .map_err(|err| err.into())
        .and_then(|b| parse_bytes(&b))
}

/// Parses the contents of a kradfile or kradfile2 and returns
/// the list of kanji radical decompositions
///
/// # Arguments
///
/// * `path` - A path to the kradfile
pub fn parse_bytes(b: &[u8]) -> KradResult {
    lines(b).map(|(_i, o)| o).map_err(|_err| KradError::Parse)
}

fn lines(b: &[u8]) -> IResult<&[u8], Vec<Decomposition>> {
    separated_list1(char('\n'), next_kanji)(b)
}

fn next_kanji(b: &[u8]) -> IResult<&[u8], Decomposition> {
    map(
        separated_pair(comments, opt(char('\n')), kanji_line),
        |(_comments, kanji)| kanji,
    )(b)
}

fn kanji_line(b: &[u8]) -> IResult<&[u8], Decomposition> {
    map(
        separated_pair(kanji, tag(SEPARATOR), radicals),
        |(kanji, radicals)| Decomposition { kanji, radicals },
    )(b)
}

fn kanji(b: &[u8]) -> IResult<&[u8], String> {
    map_res(take_until(" "), decode_jis)(b)
}

fn radicals(b: &[u8]) -> IResult<&[u8], Vec<String>> {
    separated_list1(char(' '), radical)(b)
}

fn radical(b: &[u8]) -> IResult<&[u8], String> {
    map_res(is_not(" \n"), decode_jis)(b)
}

fn comments(b: &[u8]) -> IResult<&[u8], ()> {
    value((), separated_list0(char('\n'), comment))(b)
}

fn comment(b: &[u8]) -> IResult<&[u8], ()> {
    value((), pair(char('#'), take_until("\n")))(b)
}

fn decode_jis(b: &[u8]) -> Result<String, KradError> {
    match b.len() {
        2 => {
            let code = bytes_to_u32(b);
            jis_to_utf8(code)
                .map(|unicode| unicode.to_string())
                .ok_or(KradError::Jis.into())
        }
        3 => EUCJPEncoding
            .decode(b, DecoderTrap::Strict)
            .map_err(|_| KradError::EucJp.into()),
        _ => Err(KradError::Jis.into()),
    }
}

fn bytes_to_u32(b: &[u8]) -> u32 {
    let mut out = 0u32;
    for (i, byte) in b.iter().rev().enumerate() {
        out += (*byte as u32) << 8u32 * (i as u32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // JIS213
    // "亜 : ｜ 一 口\n"
    const KANJI_LINE: &[u8] = &[
        0xB0, 0xA1, 0x20, 0x3A, 0x20, 0xA1, 0xC3, 0x20, 0xB0, 0xEC, 0x20, 0xB8, 0xFD, 0x0A,
    ];

    // First kanji EUC-JP, radicals JIS213
    // "丂 : 一 勹\n"
    const KANJI_LINE2: &[u8] = &[
        0x8F, 0xB0, 0xA1, 0x20, 0x3A, 0x20, 0xB0, 0xEC, 0x20, 0xD2, 0xB1, 0x0A,
    ];

    // JIS213
    // "｜ 一 口\n"
    const RADICALS: &[u8] = &[0xA1, 0xC3, 0x20, 0xB0, 0xEC, 0x20, 0xB8, 0xFD, 0x0A];

    const COMMENT_LINE: &[u8] = "# September 2007\n".as_bytes();
    const NEWLINE: &[u8] = "\n".as_bytes();

    fn parsed_kanji() -> Decomposition {
        Decomposition {
            kanji: "亜".to_string(),
            radicals: vec!["｜".to_string(), "一".to_string(), "口".to_string()],
        }
    }

    fn parsed_kanji_2() -> Decomposition {
        Decomposition {
            kanji: "丂".to_string(),
            radicals: vec!["一".to_string(), "勹".to_string()],
        }
    }

    #[test]
    fn is_comment() {
        let res = comment(COMMENT_LINE);
        assert_eq!(res, Ok((NEWLINE, ())));
    }

    #[test]
    fn is_comment_short() {
        let res = comment("#\n".as_bytes());
        assert_eq!(res, Ok((NEWLINE, ())));
    }

    #[test]
    fn multiple_comment_lines() {
        let line = vec![COMMENT_LINE, COMMENT_LINE].join("".as_bytes());
        let res = comments(&line);
        assert_eq!(res, Ok((NEWLINE, ())));
    }

    #[test]
    fn parses_radical() {
        let res = radical(RADICALS);
        assert_eq!(res, Ok((&RADICALS[2..], "｜".to_string())));
    }

    #[test]
    fn parses_radicals() {
        let res = radicals(RADICALS);
        assert_eq!(res, Ok((NEWLINE, parsed_kanji().radicals)));
    }

    #[test]
    fn parses_kanji() {
        let res = kanji_line(KANJI_LINE);
        assert_eq!(res, Ok((NEWLINE, parsed_kanji())));
    }

    #[test]
    fn parses_kanji_2() {
        let res = kanji_line(KANJI_LINE2);
        assert_eq!(res, Ok((NEWLINE, parsed_kanji_2())));
    }

    #[test]
    fn parses_line_as_kanji() {
        let res = next_kanji(KANJI_LINE);
        assert_eq!(res, Ok((NEWLINE, parsed_kanji())));
    }

    #[test]
    fn ignores_comment() {
        let line = vec![COMMENT_LINE, KANJI_LINE].join("".as_bytes());
        let res = next_kanji(&line);
        assert_eq!(res, Ok((NEWLINE, parsed_kanji())));
    }

    #[test]
    fn parses_lines() {
        let line = vec![KANJI_LINE, COMMENT_LINE, KANJI_LINE].join("".as_bytes());
        let res = lines(&line);
        assert_eq!(res, Ok((NEWLINE, vec![parsed_kanji(), parsed_kanji()])));
    }

    #[test]
    fn works_on_actual_file() {
        let res = parse_file("./edrdg_files/kradfile2");
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap().len(), 6_355);
    }

    #[test]
    fn works_on_actual_file_2() {
        let res = parse_file("./edrdg_files/kradfile2");
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap().len(), 5_801);
    }
}
