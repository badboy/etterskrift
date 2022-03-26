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
pub type OperatorMap = HashMap<&'static str, Box<OperatorFn>>;

pub fn operators() -> &'static OperatorMap {
    static OPERATORS: OnceCell<OperatorMap> = OnceCell::new();
    OPERATORS.get_or_init(|| {
        let mut m = HashMap::new();

        // math
        m.insert("add", operator!(add, 2));
        m.insert("sub", operator!(sub, 2));
        m.insert("mul", operator!(mul, 2));
        m.insert("div", operator!(div, 2));
        m.insert("neg", operator!(neg, 1));
        m.insert("sqrt", operator!(sqrt, 1));
        m.insert("rand", operator!(rand, 0));

        // stack
        m.insert("exch", operator!(exch, 2));
        m.insert("dup", operator!(dup, 1));
        m.insert("pop", operator!(pop, 1));
        m.insert("clear", operator!(clear, 0));
        m.insert("pstack", operator!(pstack, 0));
        m.insert("count", operator!(count, 0));
        m.insert("pdict", operator!(pdict, 0));

        // def
        m.insert("def", operator!(def, 2));

        // control
        m.insert("exec", operator!(exec, 1));
        m.insert("repeat", operator!(repeat, 2));
        m.insert("for", operator!(for_loop, 4));
        m.insert("if", operator!(if_cond, 2));
        m.insert("ifelse", operator!(ifelse_cond, 3));

        // relational
        m.insert("true", operator!(bool_true, 0));
        m.insert("false", operator!(bool_false, 0));
        m.insert("eq", operator!(eq, 2));
        m.insert("ne", operator!(ne, 2));

        // array
        m.insert("[", operator!(mark, 0));
        m.insert("]", operator!(array_close, 1));
        m.insert("length", operator!(array_length, 1));
        m.insert("forall", operator!(array_forall, 2));

        // dict
        m.insert("dict", operator!(dict_new, 1));
        m.insert("begin", operator!(dict_begin, 1));
        m.insert("end", operator!(dict_end, 0));

        // type
        m.insert("cvi", operator!(cvi, 1));

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
    let a = stack.pop()?;
    let b = stack.pop()?;

    if let (Ok(a), Ok(b)) = (a.as_int(), b.as_int()) {
            stack.push((a * b).into());
            return Ok(());
    }

    if let (Ok(a), Ok(b)) = (a.as_float(), b.as_float()) {
        stack.push((a * b).into());
        return Ok(());
    }

    Err(Report::msg("/typecheck in --mul--"))
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

fn mark(state: &mut State) -> Result<()> {
    state.operand_stack.push(Item::Mark);
    Ok(())
}

fn array_close(state: &mut State) -> Result<()> {
    let stack = &mut state.operand_stack;
    let found = (&stack.inner)
        .iter()
        .rposition(|item| matches!(item, Item::Mark));

    if found.is_none() {
        return Err(Report::msg("/unmatchedmark in --]--"));
    }

    let pos = found.unwrap();
    let mut items: Vec<_> = stack.inner.drain(pos..).collect();
    items.remove(0); // Mark
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

fn cvi(state: &mut State) -> Result<()> {
    let elem = state.operand_stack.pop()?;
    if let Ok(i) = elem.as_int() {
        state.operand_stack.push(i.into());
    } else if let Ok(i) = elem.as_float() {
        state.operand_stack.push((i as i32).into());
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_adds_the_two_top_most_elements() {
        let mut state = State::new();
        state.operand_stack.push(1.into());
        state.operand_stack.push(2.into());

        add(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(3.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn sub_subtracts_the_two_top_most_elements() {
        let mut state = State::new();
        state.operand_stack.push(3.into());
        state.operand_stack.push(1.into());

        sub(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(2.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn mul_multiplies_the_two_top_most_elements() {
        let mut state = State::new();
        state.operand_stack.push(2.into());
        state.operand_stack.push(3.into());

        mul(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(6.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn mul_handles_floats() {
        let mut state = State::new();
        state.operand_stack.push(0.5.into());
        state.operand_stack.push(2.0.into());

        mul(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(1.0.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn mul_handles_mixed_numbers() {
        let mut state = State::new();
        state.operand_stack.push(0.5.into());
        state.operand_stack.push(2.into());

        mul(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(1.0.into());

        assert_eq!(state, expected);
    }

    #[test]
    #[should_panic(expected = "typecheck")]
    fn mul_fails_typecheck() {
        let mut state = State::new();
        state.operand_stack.push("a".to_string().into());
        state.operand_stack.push("b".to_string().into());

        mul(&mut state).unwrap();
    }

    #[test]
    fn div_divides_the_top_two_most_elements() {
        let mut state = State::new();
        state.operand_stack.push(4.into());
        state.operand_stack.push(2.into());

        div(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(2.0.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn neg_negates_the_top_element() {
        let mut state = State::new();
        state.operand_stack.push(1.into());

        neg(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push((-1).into());

        assert_eq!(state, expected);
    }

    #[test]
    fn sqrt_takes_the_square_root_of_the_top_element() {
        let mut state = State::new();
        state.operand_stack.push(4.into());

        sqrt(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(2.0.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn exch_exchanges_the_two_top_most_elements() {
        let mut state = State::new();
        state.operand_stack.push(1.into());
        state.operand_stack.push(2.into());

        exch(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(2.into());
        expected.operand_stack.push(1.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn dup_duplicates_the_top_most_element() {
        let mut state = State::new();
        state.operand_stack.push(1.into());

        dup(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(1.into());
        expected.operand_stack.push(1.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn pop_removes_the_top_element() {
        let mut state = State::new();
        state.operand_stack.push(1.into());
        state.operand_stack.push(2.into());

        pop(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(1.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn clear_removes_all_elements() {
        let mut state = State::new();
        state.operand_stack.push(1.into());
        state.operand_stack.push(2.into());
        state.operand_stack.push(3.into());
        state.operand_stack.push(4.into());
        state.operand_stack.push(5.into());

        clear(&mut state).unwrap();

        let expected = State::new();

        assert_eq!(state, expected);
    }

    #[test]
    fn count_puts_the_number_of_elements_on_stack() {
        let mut state = State::new();
        state.operand_stack.push(1.into());
        state.operand_stack.push(2.0.into());

        count(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(1.into());
        expected.operand_stack.push(2.0.into());
        expected.operand_stack.push(2.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn def_creates_a_binding_in_the_current_dictionary() {
        let mut state = State::new();
        state.operand_stack.push(Item::Key("foo".to_string()));
        state.operand_stack.push(1.into());

        def(&mut state).unwrap();

        let mut expected = State::new();
        expected.dictionary.insert("foo".to_string(), 1.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn bool_true_pushes_true_onto_the_stack() {
        let mut state = State::new();

        bool_true(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(true.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn bool_false_pushes_false_onto_the_stack() {
        let mut state = State::new();

        bool_false(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(false.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn eq_pushes_true_on_the_stack_when_the_top_most_items_are_equal() {
        let mut state = State::new();
        state.operand_stack.inner.push(1.into());
        state.operand_stack.inner.push(1.into());

        eq(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(true.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn eq_pushes_false_on_the_stack_when_the_top_most_items_are_not_equal() {
        let mut state = State::new();
        state.operand_stack.inner.push(1.into());
        state.operand_stack.inner.push(2.into());

        eq(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(false.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn ne_pushes_true_on_the_stack_when_the_top_most_items_are_not_equal() {
        let mut state = State::new();
        state.operand_stack.inner.push(1.into());
        state.operand_stack.inner.push(2.into());

        ne(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(true.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn ne_pushes_false_on_the_stack_when_the_top_most_items_are_equal() {
        let mut state = State::new();
        state.operand_stack.inner.push(1.into());
        state.operand_stack.inner.push(1.into());

        ne(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(false.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn convert_int_to_int() {
        let mut state = State::new();
        state.operand_stack.push(2.into());

        cvi(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(2.into());

        assert_eq!(state, expected);
    }

    #[test]
    fn convert_float_to_int() {
        let mut state = State::new();
        state.operand_stack.push(2.9.into());

        cvi(&mut state).unwrap();

        let mut expected = State::new();
        expected.operand_stack.push(2.into());

        assert_eq!(state, expected);
    }
}
