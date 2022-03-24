use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::mem;

use color_eyre::eyre::{Report, Result};
use once_cell::sync::OnceCell;

use super::stack::Item;
use super::State;

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

            $name(state)
        }) as Box<_>
    }};
}

pub type OperatorFn = dyn Fn(&mut State) -> Result<()> + Send + Sync;
pub type OperatorMap = HashMap<String, Box<OperatorFn>>;

pub fn operators() -> &'static OperatorMap {
    static OPERATORS: OnceCell<OperatorMap> = OnceCell::new();
    OPERATORS.get_or_init(|| {
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
        m.insert("clear".into(), operator!(clear, 0));
        m.insert("pstack".into(), operator!(pstack, 0));
        m.insert("count".into(), operator!(count, 0));
        m.insert("pdict".into(), operator!(pdict, 0));

        // def
        m.insert("def".into(), operator!(def, 2));

        // control
        m.insert("exec".into(), operator!(exec, 1));
        m.insert("repeat".into(), operator!(repeat, 2));
        m.insert("for".into(), operator!(for_loop, 4));
        m.insert("if".into(), operator!(if_cond, 2));
        m.insert("ifelse".into(), operator!(ifelse_cond, 3));

        // relational
        m.insert("true".into(), operator!(bool_true, 0));
        m.insert("false".into(), operator!(bool_false, 0));
        m.insert("eq".into(), operator!(eq, 2));
        m.insert("ne".into(), operator!(ne, 2));

        // array
        m.insert("]".into(), operator!(array_close, 1));
        m.insert("length".into(), operator!(array_length, 1));
        m.insert("forall".into(), operator!(array_forall, 2));

        // dict
        m.insert("dict".into(), operator!(dict_new, 1));
        m.insert("begin".into(), operator!(dict_begin, 1));
        m.insert("end".into(), operator!(dict_end, 0));

        m
    })
}

fn add(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let res = stack.pop()?.as_int()? + stack.pop()?.as_int()?;
    stack.push(res.into());

    Ok(())
}

fn sub(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let n2 = stack.pop()?.as_int()?;
    let n1 = stack.pop()?.as_int()?;
    stack.push((n1 - n2).into());
    Ok(())
}

fn mul(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let res = stack.pop()?.as_int()? * stack.pop()?.as_int()?;
    stack.push(res.into());
    Ok(())
}

fn div(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let n2 = stack.pop()?.as_float()?;
    let n1 = stack.pop()?.as_float()?;
    stack.push((n1 / n2).into());
    Ok(())
}

fn neg(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let n = stack.pop()?.as_int()?;
    stack.push((-n).into());
    Ok(())
}

fn sqrt(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let n = stack.pop()?.as_float()?;
    stack.push(n.sqrt().into());
    Ok(())
}

fn rand(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let random_value = RandomState::new().build_hasher().finish() as i32;
    stack.push(random_value.into());
    Ok(())
}

fn exch(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let a = stack.pop().unwrap();
    let b = stack.pop().unwrap();
    stack.push(a);
    stack.push(b);
    Ok(())
}

fn dup(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let a = stack.pop()?;
    stack.push(a.clone());
    stack.push(a);
    Ok(())
}

fn pop(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let _a = stack.pop();
    Ok(())
}

fn clear(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    stack.inner.clear();
    Ok(())
}

fn pstack(state: &mut State) -> Result<()> {
    let stack = &state.operand_stack.inner;
    for x in stack.iter().rev() {
        println!("{:?}", x);
    }
    Ok(())
}

fn count(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let len = stack.len();
    stack.push((len as i32).into());
    Ok(())
}

fn pdict(state: &mut State) -> Result<()> {
    let dict = &state.dictionary;
    for (k, v) in dict {
        println!("{}: {:?}", k, v);
    }
    Ok(())
}

fn def(state: &mut State) -> Result<()> {
    let item = state.operand_stack.pop()?;
    let name = state.operand_stack.pop()?;

    state.dictionary.insert(name.as_key()?.to_string(), item);
    Ok(())
}

fn exec(state: &mut State) -> Result<()> {
    let code = state.operand_stack.pop()?.as_block()?.to_string();

    super::execute(&code, state, operators())?;
    Ok(())
}

fn repeat(state: &mut State) -> Result<()> {
    let proc = state.operand_stack.pop()?.as_block()?.to_string();
    let n = state.operand_stack.pop()?.as_int()?;

    for i in 0..n {
        state.operand_stack.push(i.into());
        super::execute(&proc, state, operators())?;
    }
    Ok(())
}

fn for_loop(state: &mut State) -> Result<()> {
    let proc = state.operand_stack.pop()?.as_block()?.to_string();
    let limit = state.operand_stack.pop()?.as_int()?;
    let inc = state.operand_stack.pop()?.as_int()?;
    let init = state.operand_stack.pop()?.as_int()?;

    for i in (init..=limit).step_by(inc as usize) {
        state.operand_stack.push(i.into());
        super::execute(&proc, state, operators())?;
    }
    Ok(())
}

fn if_cond(state: &mut State) -> Result<()> {
    let proc = state.operand_stack.pop()?.as_block()?.to_string();
    let cond = state.operand_stack.pop()?.as_bool()?;

    if cond {
        super::execute(&proc, state, operators())?;
    }
    Ok(())
}

fn ifelse_cond(state: &mut State) -> Result<()> {
    let proc2 = state.operand_stack.pop()?.as_block()?.to_string();
    let proc1 = state.operand_stack.pop()?.as_block()?.to_string();
    let cond = state.operand_stack.pop()?.as_bool()?;

    if cond {
        super::execute(&proc1, state, operators())?;
    } else {
        super::execute(&proc2, state, operators())?;
    }
    Ok(())
}

fn array_close(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let found = (&stack.inner)
        .iter()
        .rev()
        .position(|item| matches!(item, Item::Mark));

    if found.is_none() {
        return Err(Report::msg("/unmatchedmark in --]--"));
    }

    let pos = stack.len() - found.unwrap() - 1;
    let mut items: Vec<_> = stack.inner.drain(pos..).collect();
    items.remove(0); // ArrayOpen
    stack.push(Item::Array(items));
    Ok(())
}

fn array_length(state: &mut State) -> Result<()> {
    let item = state.operand_stack.pop()?;
    let array = item.as_array()?;
    let len = array.len() as i32;
    let stack = &mut state.operand_stack;
    stack.push(len.into());
    Ok(())
}

fn array_forall(state: &mut State) -> Result<()> {
    let proc = state.operand_stack.pop()?.as_block()?.to_string();
    let array = state.operand_stack.pop()?.as_array()?.to_vec();

    for elem in array.into_iter() {
        state.operand_stack.push(elem);
        super::execute(&proc, state, operators()).expect("can't run block");
    }
    Ok(())
}

fn bool_true(state: &mut State) -> Result<()> {
    state.operand_stack.push(true.into());
    Ok(())
}

fn bool_false(state: &mut State) -> Result<()> {
    state.operand_stack.push(false.into());
    Ok(())
}

fn eq(state: &mut State) -> Result<()> {
    let a = state.operand_stack.pop()?;
    let b = state.operand_stack.pop()?;

    state.operand_stack.push((a == b).into());
    Ok(())
}

fn ne(state: &mut State) -> Result<()> {
    let a = state.operand_stack.pop()?;
    let b = state.operand_stack.pop()?;

    state.operand_stack.push((a != b).into());
    Ok(())
}

fn dict_new(state: &mut State) -> Result<()> {
    let n = state.operand_stack.pop()?.as_int()?;
    let dict = HashMap::with_capacity(n as usize);
    state.operand_stack.push(dict.into());
    Ok(())
}

fn dict_begin(state: &mut State) -> Result<()> {
    let dict = state.operand_stack.pop()?.into_dict()?;
    let olddict = mem::replace(&mut state.dictionary, dict);
    state.dict_stack.push(olddict);
    Ok(())
}

fn dict_end(state: &mut State) -> Result<()> {
    let newdict = state.dict_stack.pop()?;
    let _olddict = mem::replace(&mut state.dictionary, newdict);
    Ok(())
}
