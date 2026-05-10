pub struct Lexer<'a> {
    content: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }

    fn chop_while<P: FnMut(&char) -> bool>(&mut self, mut predicate: P) -> &'a str {
        let mut indices = self.content.char_indices();
        match indices.find(|(_, c)| !predicate(&c)) {
            Some((index, _)) => {
                let (a, b) = self.content.split_at(index);
                self.content = b;
                a
            }
            None => {
                std::mem::replace(&mut self.content, "")
            }
        }
    }
}

pub enum TokenType {
    Number,
    Alphabetic,
    Sentence,
}

pub struct Token<'a>(pub TokenType, pub &'a str);

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.content = self.content.trim_start();

        if self.content.is_empty() {
            return None;
        }

        let first_char = self.content.chars().next().unwrap();
        
        if first_char.is_ascii_digit() {
            return Some(Token(TokenType::Number, self.chop_while(|c| c.is_ascii_digit())));
        }
        
        if first_char.is_alphabetic() {
            return Some(Token(TokenType::Alphabetic, self.chop_while(|c| c.is_alphabetic())));
        }

        Some(Token(TokenType::Sentence, self.chop_while(|c| !c.is_alphanumeric())))
    }
}