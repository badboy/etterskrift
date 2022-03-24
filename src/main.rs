use color_eyre::eyre::{Report, Result};
use pest::Parser;
use pest_derive::Parser;
use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct PostscriptParser;

mod operators;
mod stack;

use operators::OperatorMap;
use stack::{Item, Stack};

pub struct State {
    operand_stack: Stack<Item>,
    dictionary: HashMap<String, Item>,
    dict_stack: Stack<HashMap<String, Item>>,
}

impl State {
    fn contains_key(&self, key: &str) -> bool {
        if self.dictionary.contains_key(key) {
            return true;
        }

        for dict in self.dict_stack.inner.iter().rev() {
            if dict.contains_key(key) {
                return true;
            }
        }

        false
    }

    fn get(&self, key: &str) -> Option<&Item> {
        if let Some(item) = self.dictionary.get(key) {
            return Some(item);
        }

        for dict in self.dict_stack.inner.iter().rev() {
            if let Some(item) = dict.get(key) {
                return Some(item);
            }
        }

        None
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut state = State {
        operand_stack: Stack::new(),
        dictionary: HashMap::new(),
        dict_stack: Stack::new(),
    };

    let mut rl = Editor::<()>::new();
    loop {
        let prompt = if state.operand_stack.len() == 0 {
            "ES>".to_string()
        } else {
            format!("ES<{}>", state.operand_stack.len())
        };
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) if line.is_empty() => {
                continue;
            }
            Ok(line) => {
                if let Err(e) = execute(&line, &mut state, operators::operators()) {
                    eprintln!("Error: {}", e);
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn execute(code: &str, state: &mut State, operators: &OperatorMap) -> Result<()> {
    let program = PostscriptParser::parse(Rule::program, code)?
        .next()
        .unwrap();

    for item in program.into_inner() {
        match item.as_rule() {
            Rule::item => {
                let mut inner = item.into_inner();
                let inner = inner.next().unwrap();
                match inner.as_rule() {
                    Rule::number => {
                        let n = inner.as_str().parse().unwrap();
                        state.operand_stack.push(Item::Number(n));
                    }
                    Rule::key => {
                        let key = inner.into_inner().next().unwrap().as_str();
                        state.operand_stack.push(key.to_string().into());
                    }
                    Rule::ident => match inner.as_str() {
                        key if state.contains_key(key) => {
                            let item = state.get(key).unwrap().clone();
                            if let Item::Block(block) = item {
                                execute(&block, state, operators)?
                            } else {
                                state.operand_stack.push(item);
                            }
                        }
                        op if operators.contains_key(op) => {
                            let f = operators.get(op).unwrap();
                            f(state)?;
                        }
                        op => {
                            return Err(Report::msg(format!("/undefined in {}", op)));
                        }
                    },
                    Rule::ops => match inner.as_str() {
                        "[" => state.operand_stack.push(Item::ArrayOpen),
                        "]" => {
                            let f = operators.get("]").unwrap();
                            f(state)?;
                        }
                        _ => unreachable!("invalid ops"),
                    },
                    _ => unreachable!("b"),
                }
            }
            Rule::block => {
                let inner = item.into_inner();
                let block = inner.as_str().to_string();
                state.operand_stack.push(Item::Block(block));
            }
            Rule::EOI => (),
            _ => unreachable!("a"),
        }
    }

    Ok(())
}
