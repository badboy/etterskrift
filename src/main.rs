use std::collections::HashMap;
use std::{env, fs, mem};

use color_eyre::eyre::{Report, Result};
use pest::Parser;
use pest_derive::Parser;
use rustyline::error::ReadlineError;
use rustyline::Editor;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct PostscriptParser;

mod operators;
mod stack;

use operators::OperatorMap;
use stack::{Item, Stack};

macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err(Report::msg($msg));
    };
    ($msg:expr $(,)?) => {
        return Err(Report::msg($msg));
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(Report::msg(format!($fmt, $($arg)*)));
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct State {
    operand_stack: Stack<Item>,
    dictionary: HashMap<String, Item>,
    dict_stack: Stack<HashMap<String, Item>>,
    block_stack: Stack<String>,
    block_marks: usize,
}

impl Default for State {
    fn default() -> State {
        State::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            operand_stack: Stack::new(),
            dictionary: HashMap::new(),
            dict_stack: Stack::new(),
            block_stack: Stack::new(),
            block_marks: 0,
        }
    }

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
    let mut state = State::new();

    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 1 {
        let code = fs::read_to_string(&args[0])?;

        if let Err(e) = execute(&code, &mut state, operators::operators()) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    let mut rl = Editor::<()>::new();
    loop {
        let prompt = if state.operand_stack.is_empty() {
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
                rl.add_history_entry(line.as_str());
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
                        if !state.block_stack.is_empty() {
                            state.block_stack.push(inner.as_str().to_string());
                            continue;
                        }

                        if let Ok(n) = inner.as_str().parse() {
                            state.operand_stack.push(Item::Number(n));
                            continue;
                        }

                        if let Ok(n) = inner.as_str().parse::<f32>() {
                            state.operand_stack.push(Item::Float(n));
                            continue;
                        }

                        bail!("/invalidnumber in {}", inner.as_str());
                    }
                    Rule::radixnumber => {
                        let code = inner.as_str();
                        let pos = code.find('#').expect("no # found");
                        let radix = code[0..pos].parse().unwrap();
                        if !(2..=36).contains(&radix) {
                            bail!("/undefined in {}", inner.as_str());
                        }
                        let number = match i32::from_str_radix(&code[pos + 1..], radix) {
                            Ok(number) => number,
                            Err(_) => {
                                bail!("/undefined in {}", inner.as_str());
                            }
                        };

                        state.operand_stack.push(Item::Number(number));
                    }
                    Rule::key => {
                        if !state.block_stack.is_empty() {
                            state.block_stack.push(inner.as_str().to_string());
                            continue;
                        }

                        let key = inner.into_inner().next().unwrap().as_str();
                        state.operand_stack.push(key.to_string().into());
                    }
                    Rule::ident => {
                        if !state.block_stack.is_empty() {
                            state.block_stack.push(inner.as_str().to_string());
                            continue;
                        }

                        match inner.as_str() {
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
                                bail!("/undefined in {}", op);
                            }
                        }
                    }
                    Rule::ops => match inner.as_str() {
                        "{" => {
                            state.block_stack.push("{".into());
                            state.block_marks += 1;
                        }
                        "}" => {
                            if state.block_stack.is_empty() {
                                bail!("/syntaxerror in }");
                            }

                            state.block_marks -= 1;
                            if state.block_marks > 0 {
                                state.block_stack.push(inner.as_str().to_string());
                                continue;
                            }

                            let items = mem::take(&mut state.block_stack).inner;
                            let code = items[1..].join(" ");
                            state.operand_stack.push(Item::Block(code));
                        }
                        "[" => {
                            if !state.block_stack.is_empty() {
                                state.block_stack.push(inner.as_str().to_string());
                                continue;
                            }

                            let f = operators.get("[").unwrap();
                            f(state)?;
                        }
                        "]" => {
                            if !state.block_stack.is_empty() {
                                state.block_stack.push(inner.as_str().to_string());
                                continue;
                            }

                            let f = operators.get("]").unwrap();
                            f(state)?;
                        }
                        _ => unreachable!("invalid ops"),
                    },
                    _ => unreachable!("b"),
                }
            }
            Rule::EOI => (),
            _ => unreachable!("a"),
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_runs() {
        let mut state = State::new();

        let ops = operators::operators();
        let code = "1 1 add";
        execute(code, &mut state, ops).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(Item::Number(2));
        assert_eq!(expected, state);
    }

    #[test]
    fn procs_only_run_on_exec() {
        let mut state = State::new();

        let ops = operators::operators();
        let code = "{ 1 1 add }";
        execute(code, &mut state, ops).unwrap();

        let top = state.operand_stack.pop().unwrap();
        assert!(matches!(top, Item::Block(_)));
        assert_eq!(0, state.operand_stack.len());

        let code = "{ 1 1 add } exec";
        execute(code, &mut state, ops).unwrap();
        let mut expected = State::new();
        expected.operand_stack.push(Item::Number(2));
        assert_eq!(expected, state);
    }

    #[test]
    fn procs_do_nest() {
        let mut state = State::new();

        let ops = operators::operators();
        let code = "{ 1 1 { add } exec }";
        execute(code, &mut state, ops).unwrap();

        let top = state.operand_stack.pop().unwrap();
        assert!(matches!(top, Item::Block(_)));
        assert_eq!(0, state.operand_stack.len());
    }

    #[test]
    fn procs_do_nest_and_run() {
        let mut state = State::new();

        let ops = operators::operators();
        let code = "{ 1 1 { add } exec } exec";
        execute(code, &mut state, ops).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(Item::Number(2));
        assert_eq!(expected, state);
    }

    #[test]
    fn parses_float() {
        let mut state = State::new();

        let ops = operators::operators();
        let code = "1.5 +0.5 -0.7";
        execute(code, &mut state, ops).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(Item::Float(1.5));
        expected.operand_stack.push(Item::Float(0.5));
        expected.operand_stack.push(Item::Float(-0.7));
        assert_eq!(expected, state);
    }

    #[test]
    fn parses_with_radix() {
        let mut state = State::new();

        let ops = operators::operators();
        let code = "16#FF 4#3";
        execute(code, &mut state, ops).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(Item::Number(255));
        expected.operand_stack.push(Item::Number(3));
        assert_eq!(expected, state);
    }
}
