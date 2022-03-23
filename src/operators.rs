use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::collections::HashMap;

use super::State;

use color_eyre::eyre::{Report, Result};

macro_rules! operator {
    ($name:ident, $arity:expr) => {{
        Box::new(move |state: &mut State| {
            #[allow(unused_comparisons)]
            if state.operand_stack.len() < $arity {
                return Err(Report::msg(format!(
                    "/stackunderflow in {}",
                    stringify!($name)
                )));
            }

            $name(state);
            Ok(())
        }) as Box<_>
    }};
}

pub type OperatorFn = dyn Fn(&mut State) -> Result<()>;
pub type OperatorMap = HashMap<String, Box<OperatorFn>>;

pub fn operators() -> OperatorMap {
    let mut m = HashMap::new();

    // math
    m.insert("add".into(), operator!(add, 2));
    m.insert("sub".into(), operator!(sub, 2));
    m.insert("mul".into(), operator!(mul, 2));
    m.insert("div".into(), operator!(div, 2));
    m.insert("neg".into(), operator!(neg, 1));
    m.insert("sqrt".into(), operator!(sqrt, 1));
    m.insert("rand".into(), operator!(rand, 0));

    // stack
    m.insert("exch".into(), operator!(exch, 2));
    m.insert("dup".into(), operator!(dup, 1));
    m.insert("pop".into(), operator!(pop, 1));
    m.insert("pstack".into(), operator!(pstack, 0));
    m.insert("count".into(), operator!(count, 0));
    m.insert("pdict".into(), operator!(pdict, 0));

    // def
    m.insert("def".into(), operator!(def, 2));
    m
}

fn add(state: &mut State) {
    let stack = &mut state.operand_stack;
    let res = stack.pop().unwrap().as_int() + stack.pop().unwrap().as_int();
    stack.push(res.into());
}

fn sub(state: &mut State) {
    let stack = &mut state.operand_stack;
    let n2 = stack.pop().unwrap().as_int();
    let n1 = stack.pop().unwrap().as_int();
    stack.push((n1 - n2).into());
}

fn mul(state: &mut State) {
    let stack = &mut state.operand_stack;
    let res = stack.pop().unwrap().as_int() * stack.pop().unwrap().as_int();
    stack.push(res.into());
}

fn div(state: &mut State) {
    let stack = &mut state.operand_stack;
    let n2 = stack.pop().unwrap().as_float();
    let n1 = stack.pop().unwrap().as_float();
    stack.push((n1 / n2).into());
}

fn neg(state: &mut State) {
    let stack = &mut state.operand_stack;
    let n = stack.pop().unwrap().as_int();
    stack.push((-n).into());
}

fn sqrt(state: &mut State) {
    let stack = &mut state.operand_stack;
    let n = stack.pop().unwrap().as_float();
    stack.push(n.sqrt().into());
}

fn rand(state: &mut State) {
    let stack = &mut state.operand_stack;
    let random_value = RandomState::new().build_hasher().finish() as i32;
    stack.push(random_value.into());
}

fn exch(state: &mut State) {
    let stack = &mut state.operand_stack;
    let a = stack.pop().unwrap();
    let b = stack.pop().unwrap();
    stack.push(a);
    stack.push(b);
}

fn dup(state: &mut State) {
    let stack = &mut state.operand_stack;
    let a = stack.pop().unwrap();
    stack.push(a.clone());
    stack.push(a);
}

fn pop(state: &mut State) {
    let stack = &mut state.operand_stack;
    let _a = stack.pop();
}

fn pstack(state: &mut State) {
    let stack = &state.operand_stack.inner;
    for x in stack.into_iter().rev() {
        println!("{:?}", x);
    }
}

fn count(state: &mut State) {
    let stack = &mut state.operand_stack;
    let len = stack.len();
    stack.push((len as i32).into());
}

fn pdict(state: &mut State) {
    let dict = &state.dictionary;
    for (k, v) in dict {
        println!("{}: {:?}", k, v);
    }
}

fn def(state: &mut State) {
    let item = state.operand_stack.pop().unwrap();
    let name = state.operand_stack.pop().unwrap();

    state.dictionary.insert(name.as_key().to_string(), item);
}
