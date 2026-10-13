pub mod ast;
mod lexer;

use crate::{
    error::{Error, Result},
    sql::parser::lexer::{Keyword, Token},
    sql::types::DataType,
};
use ast::Consts;
use lexer::Lexer;
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>, // lexer 是一个 Peekable 的 Iterator，内部迭代元素是 Item = Result<Token>
}
impl<'a> Parser<'a> {
    pub fn new(sql_text: &'a str) -> Self {
        Self {
            lexer: Lexer::new(sql_text).peekable(),
        }
    }

    // parse 解析 lexer，获取 Token 并返回 ast
    pub fn parse(&mut self) -> Result<ast::Statement> {
        // 解析 lexer，获取 Token 并返回 ast
        let stmt = self.parse_statement()?;
        // 期望sql语句后有分号
        self.next_expect(Token::Semicolon)?;
        // 分号之后无其他符号
        if let Some(token) = self.peek()? {
            return Err(Error::Parse(format!("[Parser] Unexpected token: {token}")));
        }

        Ok(stmt)
    }

    fn parse_statement(&mut self) -> Result<ast::Statement> {
        let op_token = self.peek()?;
        match op_token {
            //  ? 若是 Err，则返回 Err
            Some(Token::Keyword(Keyword::Create)) => self.parse_ddl(),
            Some(Token::Keyword(Keyword::Select)) => self.parse_select(),
            Some(Token::Keyword(Keyword::Insert)) => self.parse_insert(),
            Some(token) => Err(Error::Parse(format!(
                "[Parser] Unexpected token: {token:?}"
            ))),
            None => Err(Error::Parse(format!("[Parser] Unexpected end of input"))),
        }
    }

    fn parse_ddl(&mut self) -> Result<ast::Statement> {
        let token = self.next()?;
        match token {
            Token::Keyword(Keyword::Create) => match self.next()? {
                Token::Keyword(Keyword::Table) => self.parse_ddl_create_table(),
                token => Err(Error::Parse(format!(
                    "[Parser] Unexpected token: {token:?}"
                ))),
            },
            token => Err(Error::Parse(format!(
                "[Parser] Unexpected token: {token:?}"
            ))),
        }
    }

    // parse select 解析 select 语句
    fn parse_select(&mut self) -> Result<ast::Statement> {
        self.next_expect(Token::Keyword(Keyword::Select))?;
        self.next_expect(Token::Asterisk)?;
        self.next_expect(Token::Keyword(Keyword::From))?;

        // 期望 FROM 表名
        let table_name = self.next_ident()?;
        Ok(ast::Statement::Select {
            table_name: table_name,
        })
    }

    fn parse_insert(&mut self) -> Result<ast::Statement> {
        // insert into tbl(a, b, c) values (1, 2, 3),(4, 5, 6);
        self.next_expect(Token::Keyword(Keyword::Insert))?;
        self.next_expect(Token::Keyword(Keyword::Into))?;

        // 期望 表名
        let table_name = self.next_ident()?;

        // 解析 tbl(a, b, c) 表示列名
        let colums = if self.next_if_token(Token::OpenParen).is_some() {
            let mut cols = Vec::new();
            loop {
                cols.push(self.next_ident()?.to_string());
                match self.next()? {
                    Token::CloseParen => break,
                    Token::Comma => {}
                    token => {
                        return Err(Error::Parse(format!(
                            "[Parser] Unexpected token: {token:?}"
                        )));
                    }
                }
            }
            Some(cols)
        } else {
            None
        };

        // 期望 values 关键字
        // insert into tbl(a, b, c) values (1, 2, 3),(4, 5, 6);
        self.next_expect(Token::Keyword(Keyword::Values))?;
        let mut values = Vec::new();
        loop {
            let mut row = Vec::new();
            self.next_expect(Token::OpenParen)?;
            loop {
                row.push(self.parse_expression()?);
                match self.next()? {
                    Token::Comma => {}
                    Token::CloseParen => break,
                    token => {
                        return Err(Error::Parse(format!(
                            "[Parser] Unexpected token: {token:?}"
                        )));
                    }
                }
            }
            values.push(row);
            if self.next_if_token(Token::Comma).is_none() {
                break;
            }
        }

        Ok(ast::Statement::Insert {
            table_name,
            columns: colums,
            values,
        })
    }

    /***
    *   let sql1 = "
            create table tbl1 (
                a int default 100,
                b float not null,
                c varchar null,
                d bool default true
            );
        ";
    */
    fn parse_ddl_create_table(&mut self) -> Result<ast::Statement> {
        // 期望 CREATE TABLE 表名
        let table_name = self.next_ident()?;
        // Create Table 语句，表名后应该是 左括号
        self.next_expect(Token::OpenParen)?;

        // parse columns info
        let mut columns = Vec::new();
        loop {
            let column = self.parse_ddl_column()?;
            columns.push(column);
            if self.next_if_token(Token::Comma).is_none() {
                break;
            }
        }

        self.next_expect(Token::CloseParen)?;
        Ok(ast::Statement::CreateTable {
            name: table_name,
            columns,
        })
    }

    fn parse_ddl_column(&mut self) -> Result<ast::Column> {
        let mut column = ast::Column {
            name: self.next_ident()?,
            datatype: match self.next()? {
                Token::Keyword(Keyword::Int) | Token::Keyword(Keyword::Integer) => {
                    DataType::Integer
                }
                Token::Keyword(Keyword::Bool) | Token::Keyword(Keyword::Boolean) => {
                    DataType::Boolean
                }
                Token::Keyword(Keyword::String)
                | Token::Keyword(Keyword::Text)
                | Token::Keyword(Keyword::Varchar) => DataType::String,
                Token::Keyword(Keyword::Float) | Token::Keyword(Keyword::Double) => DataType::Float,
                token => return Err(Error::Parse(format!("[Parser] Unexpected token: {token}"))),
            },
            nullable: None,
            default: None,
        };

        //
        while let Some(Token::Keyword(keyword)) = self.next_if_keyword() {
            match keyword {
                Keyword::Null => column.nullable = Some(true),
                Keyword::Not => {
                    self.next_expect(Token::Keyword(Keyword::Null))?;
                    column.nullable = Some(false);
                }
                Keyword::Default => column.default = Some(self.parse_expression()?),
                k => return Err(Error::Parse(format!("[Parser] Unexpected token: {k}"))),
            }
        }

        Ok(column)
    }

    fn parse_expression(&mut self) -> Result<ast::Expression> {
        let token = self.next()?;
        match token {
            Token::Keyword(Keyword::Null) => Ok(ast::Expression::Consts(Consts::Null)),
            Token::Keyword(Keyword::True) => Ok(ast::Expression::Consts(Consts::Boolean(true))),
            Token::Keyword(Keyword::False) => Ok(ast::Expression::Consts(Consts::Boolean(false))),
            Token::Number(num) => Ok({
                if num.chars().all(|c| c.is_ascii_digit()) {
                    // let n = num.parse::<i64>();
                    // ? 返回Err - ParseIntoError 时，因为实现 impl From<std::num::ParseIntError> for Error {
                    // 编译器自动调用 into() 方法，底层就是上述的 From<std::num::ParseIntError> for Error 实现
                    // 做到 ParseIntoError 转换为 Error 类型
                    Consts::Integer(num.parse()?).into()
                } else {
                    // let n = num.parse<f64>();
                    Consts::Float(num.parse()?).into()
                }
            }),
            Token::String(str) => Ok(ast::Expression::Consts(Consts::String(str))),
            token => Err(Error::Parse(format!(
                "[Parser] Unexpected token: {token:?}"
            ))),
        }
    }

    fn peek(&mut self) -> Result<Option<Token>> {
        self.lexer.peek().cloned().transpose() // 将 Option<Token> 转换为 Result<Option<Token>>
    }

    fn next(&mut self) -> Result<Token> {
        self.lexer
            .next()
            // .unwrap_or(Err(Error::Parse(format!("[Parser] Unexpected end of input"))))
            .unwrap_or_else(|| Err(Error::Parse(format!("[Parser] Unexpected end of input"))))
    }

    // 获取下一个 Token 的 Ident 值
    fn next_ident(&mut self) -> Result<String> {
        let token = self.next()?;
        match token {
            Token::Ident(ident) => Ok(ident),
            token => Err(Error::Parse(format!(
                "[Parser] Unexpected token: {token:?}"
            ))),
        }
    }

    // 获取下一个 Token 的 Ident 值，并检查是否与 expected 匹配
    fn next_expect(&mut self, expected: Token) -> Result<()> {
        let token = self.next()?;
        if token == expected {
            Ok(())
        } else {
            Err(Error::Parse(format!(
                "[Parser] Unexpected token: {token}, expected: {expected}"
            )))
        }
    }

    // 若满足 predicate，则 next 推进一个 Token 并返回值
    fn next_if<F>(&mut self, predicate: F) -> Option<Token>
    where
        F: Fn(&Token) -> bool,
    {
        let res_op_token = self.peek();
        // 判断 下个 Option<Token> 是否为 Some，并且 predicate 满足
        res_op_token.unwrap_or(None).filter(|c| predicate(c))?;
        // 将 Result<Token> 转换为 Option<Token>
        self.next().ok()
    }

    // next 期望下一个 Token 是 Keyword
    fn next_if_keyword(&mut self) -> Option<Token> {
        self.next_if(|c| matches!(c, Token::Keyword(_)))
    }

    // next 期望下一个 Token 是 expected
    fn next_if_token(&mut self, expected: Token) -> Option<Token> {
        self.next_if(|c| c == &expected)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parser_create_table() -> Result<()> {
        let sql1 = "
            create table tbl1 (
                a int default 100,
                b float not null,
                c varchar null,
                d bool default true
            );
        ";
        let stmt1 = Parser::new(sql1).parse()?;
        println!("{stmt1:?}");

        let sql2 = "
        create            table tbl1 (
            a int default     100,
            b float not null     ,
            c varchar      null,
            d       bool default        true
        );
        ";
        let stmt2 = Parser::new(sql2).parse()?;
        assert_eq!(stmt1, stmt2);

        let sql3 = "
            create            table tbl1 (
            a int default     100,
            b float not null     ,
            c varchar      null,
            d       bool default        true
        )
        ";

        let stmt3 = Parser::new(sql3).parse();
        assert!(stmt3.is_err());
        Ok(())
    }

    #[test]
    fn test_parser_insert() -> Result<()> {
        let sql1 = "insert into tbl1 values (1, 2, 3, 'a', true);";
        let stmt1 = Parser::new(sql1).parse()?;
        assert_eq!(
            stmt1,
            ast::Statement::Insert {
                table_name: "tbl1".to_string(),
                columns: None,
                values: vec![vec![
                    ast::Consts::Integer(1).into(),
                    ast::Consts::Integer(2).into(),
                    ast::Consts::Integer(3).into(),
                    ast::Consts::String("a".to_string()).into(),
                    ast::Consts::Boolean(true).into(),
                ]],
            }
        );

        let sql2 = "insert into tbl2 (c1, c2, c3) values (3, 'a', true),(4, 'b', false);";
        let stmt2 = Parser::new(sql2).parse()?;
        assert_eq!(
            stmt2,
            ast::Statement::Insert {
                table_name: "tbl2".to_string(),
                columns: Some(vec!["c1".to_string(), "c2".to_string(), "c3".to_string()]),
                values: vec![
                    vec![
                        ast::Consts::Integer(3).into(),
                        ast::Consts::String("a".to_string()).into(),
                        ast::Consts::Boolean(true).into(),
                    ],
                    vec![
                        ast::Consts::Integer(4).into(),
                        ast::Consts::String("b".to_string()).into(),
                        ast::Consts::Boolean(false).into(),
                    ],
                ],
            }
        );

        Ok(())
    }

    #[test]
    fn test_parser_select() -> Result<()> {
        let sql = "select * from tbl1;";
        let stmt = Parser::new(sql).parse()?;
        println!("{:?}", stmt);
        Ok(())
    }
}
