use crate::ast::*;
use crate::Rule;
use pest::iterators::Pair;

pub fn parse_program(pair: Pair<Rule>) -> Program {
    let mut globals = Vec::new();
    let mut functions = Vec::new();
    let mut includes = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::include_stmt => includes.push(parse_include(p)),
            Rule::global_var => globals.push(parse_global_var(p)),
            Rule::function => functions.push(parse_function(p)),
            Rule::EOI => (),
            _ => unreachable!("Unexpected rule in program: {:?}", p.as_rule()),
        }
    }

    Program { includes, globals, functions }
}

fn parse_include(pair: Pair<Rule>) -> String {
    let inner = pair.into_inner().next().unwrap();
    let path_pair = inner.into_inner().next().unwrap();
    match path_pair.as_rule() {
        Rule::identifier => format!("@std/{}", path_pair.as_str()),
        Rule::string_lit => {
            let s = path_pair.as_str();
            s[1..s.len()-1].to_string()
        }
        _ => unreachable!("Unexpected path rule: {:?}", path_pair.as_rule()),
    }
}

fn parse_global_var(pair: Pair<Rule>) -> GlobalVar {
    let mut inner = pair.into_inner();
    let ty = parse_type(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    let value = parse_expression(inner.next().unwrap());
    GlobalVar { ty, name, value }
}

fn parse_function(pair: Pair<Rule>) -> Function {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    
    let mut params = Vec::new();
    let mut next = inner.next().unwrap();
    
    // Check if next is parameters or return type or block
    if next.as_rule() == Rule::parameter {
         params.push(parse_parameter(next.clone()));
         while let Some(p) = inner.next() {
             if p.as_rule() == Rule::parameter {
                 params.push(parse_parameter(p));
             } else {
                 next = p;
                 break;
             }
         }
    }
    
    let mut return_ty = Type::Void;
    if next.as_rule() == Rule::type_name {
        return_ty = parse_type(next);
        next = inner.next().unwrap();
    }
    
    let body = parse_block(next);
    
    Function { name, params, return_ty, body }
}

fn parse_parameter(pair: Pair<Rule>) -> Parameter {
    let mut inner = pair.into_inner();
    let ty = parse_type(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    Parameter { ty, name }
}

fn parse_type(pair: Pair<Rule>) -> Type {
    match pair.as_str() {
        "int" => Type::Int,
        "str" => Type::Str,
        "bool" => Type::Bool,
        "void" => Type::Void,
        _ => unreachable!("Unknown type: {}", pair.as_str()),
    }
}

fn parse_block(pair: Pair<Rule>) -> Block {
    let mut statements = Vec::new();
    for p in pair.into_inner() {
        statements.push(parse_statement(p));
    }
    Block { statements }
}

fn parse_statement(pair: Pair<Rule>) -> Statement {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::local_var => Statement::LocalVar(parse_local_var(inner)),
        Rule::if_stmt => {
            let mut i = inner.into_inner();
            let condition = parse_expression(i.next().unwrap());
            let then_block = parse_block(i.next().unwrap());
            let else_block = i.next().map(parse_block);
            Statement::If { condition, then_block, else_block }
        }
        Rule::while_stmt => {
            let mut i = inner.into_inner();
            let condition = parse_expression(i.next().unwrap());
            let block = parse_block(i.next().unwrap());
            Statement::While { condition, block }
        }
        Rule::return_stmt => {
            let val = inner.into_inner().next().map(parse_expression);
            Statement::Return(val)
        }
        Rule::assignment => {
            let mut i = inner.into_inner();
            let name = i.next().unwrap().as_str().to_string();
            let value = parse_expression(i.next().unwrap());
            Statement::Assignment { name, value }
        }
        Rule::expression => Statement::Expression(parse_expression(inner)),
        _ => unreachable!("Unexpected statement: {:?}", inner.as_rule()),
    }
}

fn parse_local_var(pair: Pair<Rule>) -> LocalVar {
    let mut inner = pair.into_inner();
    let ty = parse_type(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    let value = inner.next().map(parse_expression);
    LocalVar { ty, name, value }
}

fn parse_expression(pair: Pair<Rule>) -> Expression {
    let mut inner = pair.into_inner();
    let mut left = parse_term(inner.next().unwrap());

    while let Some(op_pair) = inner.next() {
        let op = parse_binary_op(op_pair);
        let right = parse_term(inner.next().unwrap());
        left = Expression::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }

    left
}

fn parse_term(pair: Pair<Rule>) -> Expression {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::literal => Expression::Literal(parse_literal(inner)),
        Rule::function_call => {
            let mut i = inner.into_inner();
            let name = i.next().unwrap().as_str().to_string();
            let mut args = Vec::new();
            for arg_pair in i {
                args.push(parse_expression(arg_pair));
            }
            Expression::FunctionCall { name, args }
        }
        Rule::identifier => Expression::Identifier(inner.as_str().to_string()),
        Rule::expression => parse_expression(inner),
        _ => unreachable!("Unexpected term: {:?}", inner.as_rule()),
    }
}

fn parse_binary_op(pair: Pair<Rule>) -> BinaryOp {
    match pair.as_str() {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        "==" => BinaryOp::Eq,
        "!=" => BinaryOp::Ne,
        "<" => BinaryOp::Lt,
        ">" => BinaryOp::Gt,
        "<=" => BinaryOp::Le,
        ">=" => BinaryOp::Ge,
        "&&" => BinaryOp::And,
        "||" => BinaryOp::Or,
        _ => unreachable!("Unknown operator: {}", pair.as_str()),
    }
}

fn parse_literal(pair: Pair<Rule>) -> Literal {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::string_lit => {
            let s = inner.as_str();
            Literal::Str(s[1..s.len()-1].to_string())
        }
        Rule::number_lit => Literal::Int(inner.as_str().parse().unwrap()),
        Rule::bool_lit => Literal::Bool(inner.as_str() == "true"),
        _ => unreachable!("Unexpected literal: {:?}", inner.as_rule()),
    }
}
