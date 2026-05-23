use crate::trie::TRIE;

pub const MAX_TERM_LENGTH: usize = 50;

#[derive(PartialEq, Eq)]
enum CharType {
    AsciiAlphabetic,
    AsciiDigit,
    AsciiPunctuation,
    AsciiOther,
    ChinesePunctuation,
    WhiteSpace,
    Other,
    None,
}

const CHINESE_PUNCTUATIONS: &str = "，。《》？！￥（）【】；：‘’“”、";

impl CharType {
    fn new(c: char) -> Self {
        if c.is_ascii_alphabetic() {
            Self::AsciiAlphabetic
        } else if c.is_ascii_digit() {
            Self::AsciiDigit
        } else if c.is_ascii_punctuation() {
            Self::AsciiPunctuation
        } else if c.is_ascii() {
            Self::AsciiOther
        } else if CHINESE_PUNCTUATIONS.contains(c) {
            Self::ChinesePunctuation
        } else if c.is_whitespace() {
            Self::WhiteSpace
        } else {
            Self::Other
        }
    }
}

/// Return split points of content
pub fn tokenize(content: impl AsRef<str>) -> Vec<usize> {
    let content: Vec<_> = content.as_ref().char_indices().collect();
    let n = content.len();
    let mut f = vec![0f64; n + 1];
    let mut next = vec![n; n + 1];
    let mut alphabet = CharType::None;
    let mut sub_end = n;
    let mut utf8_len = 0;
    for i in (0..n).rev() {
        let this_type = CharType::new(content[i].1);
        if alphabet != this_type {
            alphabet = this_type;
            sub_end = i;
            utf8_len = 0;
        }
        utf8_len += content[i].1.len_utf8();

        for _ in 0..4 {
            if utf8_len > MAX_TERM_LENGTH {
                utf8_len -= content[sub_end].1.len_utf8();
                sub_end -= 1;
            } else {
                break;
            }
        }

        f[i] = f[sub_end + 1];
        next[i] = sub_end + 1;
        let mut node = &*TRIE;
        for j in i..n.min(i + MAX_TERM_LENGTH - 1) {
            let next_node = node.seek_char(content[j].1);
            let Some(next_node) = next_node else {
                break;
            };

            if let Some(value) = next_node.value() {
                let nf = f[j + 1] + value;

                if f[i] <= nf {
                    f[i] = nf;
                    next[i] = j + 1;
                }
            }

            node = next_node;
        }
    }
    let mut ret = Vec::new();

    let mut p = next[0];
    while p < n {
        ret.push(content[p].0);
        p = next[p];
    }

    ret
}
