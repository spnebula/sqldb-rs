// Import the Chars type from the std::str module
use std::{
    char,
    iter::{Iterator, Peekable},
    str::Chars,
};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // 关键字
    Keyword(Keyword),
    // 其他类型的字符串Token，比如表名、列名
    Ident(String),
    // 字符串类型的数据
    String(String),
    // 数值类型，比如整数和浮点数
    Number(String),
    // 左括号 (
    OpenParen,
    // 右括号 )
    CloseParen,
    // 逗号 ,
    Comma,
    // 分号 ;
    Semicolon,
    // 星号 *
    Asterisk,
    // 加号 +
    Plus,
    // 减号 -
    Minus,
    // 斜杠 /
    Slash,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Create,
    Table,
    Int,
    Integer,
    Boolean,
    Bool,
    String,
    Text,
    Varchar,
    Float,
    Double,
    Select,
    From,
    Insert,
    Into,
    Values,
    True,
    False,
    Default,
    Not,
    Null,
    Primary,
    Key,
}

impl Keyword {
    pub fn fmt_str(ident: &str) -> Option<Keyword> {
        match ident.to_uppercase().as_str() {
            "CREATE" => Some(Keyword::Create),
            "TABLE" => Some(Keyword::Table),
            "INT" => Some(Keyword::Int),
            "INTEGER" => Some(Keyword::Integer),
            "BOOL" => Some(Keyword::Bool),
            "STRING" => Some(Keyword::String),
            "TEXT" => Some(Keyword::Text),
            "VARCHAR" => Some(Keyword::Varchar),
            "FLOAT" => Some(Keyword::Float),
            "DOUBLE" => Some(Keyword::Double),
            "SELECT" => Some(Keyword::Select),
            "FROM" => Some(Keyword::From),
            "INSERT" => Some(Keyword::Insert),
            "INTO" => Some(Keyword::Into),
            "VALUES" => Some(Keyword::Values),
            "TRUE" => Some(Keyword::True),
            "FALSE" => Some(Keyword::False),
            "DEFAULT" => Some(Keyword::Default),
            "NOT" => Some(Keyword::Not),
            "NULL" => Some(Keyword::Null),
            "PRIMARY" => Some(Keyword::Primary),
            "KEY" => Some(Keyword::Key),
            _ => None,
        }
    }
}

pub struct Lexer<'a> {
    // 'a is the lifetime of the input string.
    // If a struct field is a reference type, the field lifecycle needs to be manually indicated.
    iter: Peekable<Chars<'a>>,
}

// 自定义迭代器
impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.scan() {
            Ok(Some(token)) => Some(Ok(token)),
            // 处理两种情况，错误字符(scan_symbo => None)和正常的 EOF
            Ok(None) => self
                .iter
                .peek()
                .map(|c| Err(Error::Parse(format!("[Lexer] Unexpected char: {}", c)))),
            Err(err) => Some(Err(err)),
        }
    }
}

impl<'a> Lexer<'a> {
    // new returns a new Lexer instance.
    // &'a str 加上'a 是因为 &str 是要传递给结构体字段的 &str，为手动抉择结构体内引用字段的生命周期。
    pub fn new(sql_text: &'a str) -> Self {
        Self {
            iter: sql_text.chars().peekable(),
        }
    }

    // erase_whitespace erase whitespace from sql text.
    fn erase_whitespace(&mut self) {
        self.next_while(|c| c.is_whitespace());
    }

    // if next a char is satisfying f, move to next char and return this char.
    fn next_if<F>(&mut self, f: F) -> Option<char>
    where
        F: Fn(char) -> bool,
    {
        self.iter.peek().filter(|&c| f(*c))?;
        self.iter.next()
    }

    // find a String satisfying f
    fn next_while<F>(&mut self, f: F) -> Option<String>
    where
        F: Fn(char) -> bool,
    {
        let mut value = String::new();
        while let Some(c) = self.next_if(&f) {
            value.push(c);
        }
        Some(value).filter(|v: &String| !v.is_empty())
    }

    // scan get next token from sql text.
    fn scan(&mut self) -> Result<Option<Token>> {
        self.erase_whitespace();
        match self.iter.peek() {
            Some('\'') => self.scan_string(), // 扫描单引号开始的字符串
            Some(c) if c.is_ascii_digit() => Ok(self.scan_number()), // 扫描数字
            Some(c) if c.is_alphabetic() => Ok(self.scan_ident()), // 扫描字母
            Some(_) => Ok(self.scan_symbol()), // 扫描符号
            None => Ok(None),
        }
    }

    // scan_string
    fn scan_string(&mut self) -> Result<Option<Token>> {
        // determine whether it starts with '
        if self.next_if(|c| c == '\'').is_none() {
            return Ok(None);
        }

        let mut value = String::new();
        loop {
            match self.iter.next() {
                Some('\'') => break,      // 单引号结尾的字符类型
                Some(c) => value.push(c), // 常规字符类型
                None => return Err(Error::Parse(format!("[Lexer] Unexpected end of string"))),
            }
        }

        Ok(Some(Token::String(value)))
    }

    // scan_number
    fn scan_number(&mut self) -> Option<Token> {
        let mut num = self.next_while(|c: char| c.is_ascii_digit())?;

        if let Some(c) = self.next_if(|c| c == '.') {
            num.push(c); // 将小数点加入到数字字符串中 

            match self.next_while(|c: char| c.is_ascii_digit()) {
                // 扫描小数点后的数字
                Some(str) => num.push_str(&str),
                _ => {}
            }
        }

        Some(Token::Number(num))
    }

    // 扫描 Identifier 类型，例如表名、列名等，也有可能是关键字，true / false
    fn scan_ident(&mut self) -> Option<Token> {
        // 扫描字母: 字母开头
        let mut ident = self.next_if(|c| c.is_alphabetic())?.to_string();

        // 扫描后续的字符串: Ident 后面可能有字母、数字、下划线
        match self.next_while(|c| c.is_alphanumeric() || c == '_') {
            Some(str) => ident.push_str(&str),
            _ => {}
        }

        Some(Keyword::fmt_str(&ident).map_or(Token::Ident(ident.to_lowercase()), Token::Keyword))
    }

    // 扫描符号
    fn scan_symbol(&mut self) -> Option<Token> {
        self.next_if_token(|c| match c {
            '*' => Some(Token::Asterisk),
            '(' => Some(Token::OpenParen),
            ')' => Some(Token::CloseParen),
            ',' => Some(Token::Comma),
            ';' => Some(Token::Semicolon),
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            '/' => Some(Token::Slash),
            _ => None,
        })
    }

    // 只有是 Token 类型，才跳转到下一个，并返回 Token
    fn next_if_token<F>(&mut self, predicate: F) -> Option<Token>
    where
        F: Fn(char) -> Option<Token>,
    {
        let token = self.iter.peek().and_then(|c: &char| predicate(*c))?;
        self.iter.next();
        Some(token)
    }
}

#[cfg(test)]
mod tests {

    use std::vec;

    use super::Lexer;
    use crate::{
        error::Result,
        sql::parser::lexer::{Keyword, Token},
    };

    #[test]
    fn test_lexer_create_table() -> Result<()> {
        let tokens1 = Lexer::new(
            "CREATE table tbl
                (
                    id1 int primary key,
                    id2 integer
                );
                ",
        )
        .peekable()
        .collect::<Result<Vec<_>>>()?;

        assert_eq!(
            tokens1,
            vec![
                Token::Keyword(Keyword::Create),
                Token::Keyword(Keyword::Table),
                Token::Ident("tbl".to_string()),
                Token::OpenParen,
                Token::Ident("id1".to_string()),
                Token::Keyword(Keyword::Int),
                Token::Keyword(Keyword::Primary),
                Token::Keyword(Keyword::Key),
                Token::Comma,
                Token::Ident("id2".to_string()),
                Token::Keyword(Keyword::Integer),
                Token::CloseParen,
                Token::Semicolon
            ]
        );

        println!("{tokens1:?}");

        let tokens2 = Lexer::new(
            "CREATE table tbl
                        (
                            id1 int primary key,
                            id2 integer,
                            c1 bool null,
                            c2 boolean not null,
                            c3 float null,
                            c4 double,
                            c5 string,
                            c6 text,
                            c7 varchar default 'foo',
                            c8 int default 100,
                            c9 integer
                        );
                        ",
        )
        .peekable()
        .collect::<Result<Vec<_>>>()?;

        assert!(tokens2.len() > 0);

        Ok(())
    }

    #[test]
    fn test_lexer_insert_into() -> Result<()> {
        let tokens1 = Lexer::new("insert into tbl values (1, 2, '3', true, false, 4.55);")
            .peekable()
            .collect::<Result<Vec<_>>>()?;

        assert_eq!(
            tokens1,
            vec![
                Token::Keyword(Keyword::Insert),
                Token::Keyword(Keyword::Into),
                Token::Ident("tbl".to_string()),
                Token::Keyword(Keyword::Values),
                Token::OpenParen,
                Token::Number("1".to_string()),
                Token::Comma,
                Token::Number("2".to_string()),
                Token::Comma,
                Token::String("3".to_string()),
                Token::Comma,
                Token::Keyword(Keyword::True),
                Token::Comma,
                Token::Keyword(Keyword::False),
                Token::Comma,
                Token::Number("4.55".to_string()),
                Token::CloseParen,
                Token::Semicolon,
            ]
        );

        let tokens2 = Lexer::new("INSERT INTO       tbl (id, name, age) values (100, 'db', 10);")
            .peekable()
            .collect::<Result<Vec<_>>>()?;

        assert_eq!(
            tokens2,
            vec![
                Token::Keyword(Keyword::Insert),
                Token::Keyword(Keyword::Into),
                Token::Ident("tbl".to_string()),
                Token::OpenParen,
                Token::Ident("id".to_string()),
                Token::Comma,
                Token::Ident("name".to_string()),
                Token::Comma,
                Token::Ident("age".to_string()),
                Token::CloseParen,
                Token::Keyword(Keyword::Values),
                Token::OpenParen,
                Token::Number("100".to_string()),
                Token::Comma,
                Token::String("db".to_string()),
                Token::Comma,
                Token::Number("10".to_string()),
                Token::CloseParen,
                Token::Semicolon,
            ]
        );
        Ok(())
    }

    #[test]
    fn test_lexer_select() -> Result<()> {
        let tokens1 = Lexer::new("select * from tbl;")
            .peekable()
            .collect::<Result<Vec<_>>>()?;

        assert_eq!(
            tokens1,
            vec![
                Token::Keyword(Keyword::Select),
                Token::Asterisk,
                Token::Keyword(Keyword::From),
                Token::Ident("tbl".to_string()),
                Token::Semicolon,
            ]
        );
        Ok(())
    }
}
