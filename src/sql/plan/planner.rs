use super::Node;
use crate::sql::{
    parser::ast,
    plan::Plan,
    schema::{self, Table},
    types::Value,
};

pub struct Planner;

impl Planner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(&mut self, stmt: ast::Statement) -> Plan {
        Plan(self.build_statement(stmt))
    }

    pub fn build_statement(&mut self, stmt: ast::Statement) -> Node {
        match stmt {
            ast::Statement::CreateTable { name, columns } => Node::CreateTable {
                schema: Table {
                    name,
                    // iterator 的 map 与 Option，Result 不同,
                    // map 相当于设置 迭代器 next() 后需要执行的闭包函数，并返回函数的返回值
                    // collect() 相当于收集 next() 后需要执行的闭包函数的返回值，并返回一个 Vec
                    columns: columns
                        .into_iter()
                        .map(|c| {
                            let nullable = c.nullable.unwrap_or(true);
                            let default = match c.default {
                                Some(expr) => Some(Value::from_expression(expr)),
                                None if nullable => Some(Value::Null),
                                None => None,
                            };

                            schema::Column {
                                name: c.name,
                                datatype: c.datatype,
                                nullable,
                                default,
                            }
                        })
                        .collect(),
                },
            },
            ast::Statement::Insert {
                table_name,
                columns,
                values,
            } => Node::Insert {
                table_name,
                columns: columns.unwrap_or_default(),
                values,
            },
            ast::Statement::Select { table_name } => Node::Scan { table_name },
        }
    }
}
