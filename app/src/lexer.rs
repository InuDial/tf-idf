use crate::trie::TRIE;

pub const MAX_TERM_LENGTH: usize = 50;

const CHINESE_PUNCTUATIONS: &str = "，。《》？！￥（）【】；：‘’“”、";

#[derive(PartialEq, Eq, Debug)]
enum CharType {
    AsciiAlphabetic,
    AsciiDigit,
    AsciiPunctuation,
    AsciiOther,
    ChinesePunctuation,
    WhiteSpace,
    Other,
}

impl CharType {
    fn new(c: char) -> Self {
        if c.is_ascii_alphabetic() {
            Self::AsciiAlphabetic
        } else if c.is_ascii_digit() {
            Self::AsciiDigit
        } else if c.is_ascii_punctuation() {
            Self::AsciiPunctuation
        } else if c.is_whitespace() {
            Self::WhiteSpace
        } else if c.is_ascii() {
            Self::AsciiOther
        } else if CHINESE_PUNCTUATIONS.contains(c) {
            Self::ChinesePunctuation
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
    let mut char_type = CharType::Other;
    let mut sub_end = n;
    let mut utf8_len = 0;
    for i in (0..n).rev() {
        let this_type = CharType::new(content[i].1);
        if char_type != this_type {
            char_type = this_type;
            sub_end = i;
            utf8_len = 0;
        }
        // Other: split by chars
        if char_type == CharType::Other {
            sub_end = i;
            utf8_len = 0;
        }
        utf8_len += content[i].1.len_utf8();

        // 4 bytes per char
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_empty() {
        assert_eq!(tokenize(""), Vec::<usize>::new());
    }

    #[test]
    fn tokenize_ascii_words() {
        // "hello world" → 按空格切分，空格前为一个 token
        let result = tokenize("hello world");
        assert!(!result.is_empty());
    }

    #[test]
    fn tokenize_chinese_in_dict() {
        // 常见中文词应在词典中
        let result = tokenize("人工智能");
        assert!(!result.is_empty(), "常见中文词应被分词");
    }

    #[test]
    fn tokenize_oov_by_char() {
        // OOV 字符（如日文假名或 emoji）按单字符切分
        let result = tokenize("あいう");
        // 每个 Other 类型字符都应产生一个切分点（即 result 非空）
        assert!(!result.is_empty());
    }

    #[test]
    fn tokenize_mixed_chinese_and_ascii() {
        let result = tokenize("hello世界123abc");
        assert!(!result.is_empty());
    }

    #[test]
    fn tokenize_whitespace_only() {
        let result = tokenize("   \n\t  ");
        // 纯空白不产生切分点
        assert!(result.is_empty());
    }

    #[test]
    fn char_type_classification() {
        assert_eq!(CharType::new('a'), CharType::AsciiAlphabetic);
        assert_eq!(CharType::new('Z'), CharType::AsciiAlphabetic);
        assert_eq!(CharType::new('0'), CharType::AsciiDigit);
        assert_eq!(CharType::new('9'), CharType::AsciiDigit);
        assert_eq!(CharType::new('.'), CharType::AsciiPunctuation);
        assert_eq!(CharType::new('@'), CharType::AsciiPunctuation);
        assert_eq!(CharType::new('\x01'), CharType::AsciiOther);
        assert_eq!(CharType::new('。'), CharType::ChinesePunctuation);
        assert_eq!(CharType::new(' '), CharType::WhiteSpace);
        assert_eq!(CharType::new('あ'), CharType::Other);
    }
}
