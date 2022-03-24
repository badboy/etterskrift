use color_eyre::eyre::{Report, Result};
use std::collections::HashMap;

macro_rules! msg {
    ($($rest:tt)+) => {
        Err(Report::msg(format!($($rest)+)))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Number(i32),
    Float(f32),
    Bool(bool),
    Dict(HashMap<String, Item>),
    Key(String),
    Block(String),
    Mark,
    Array(Vec<Item>),
}

impl Eq for Item {}

impl Item {
    pub fn as_int(&self) -> Result<i32> {
        if let &Item::Number(i) = self {
            Ok(i)
        } else {
            msg!("{:?} not an int", self)
        }
    }

    pub fn as_float(&self) -> Result<f32> {
        match *self {
            Item::Number(i) => Ok(i as f32),
            Item::Float(f) => Ok(f),
            _ => msg!("{:?} not a float", self),
        }
    }

    pub fn as_key(&self) -> Result<&str> {
        if let Item::Key(s) = self {
            Ok(s)
        } else {
            msg!("{:?} not a key", self)
        }
    }

    pub fn as_block(&self) -> Result<&str> {
        if let Item::Block(s) = self {
            Ok(s)
        } else {
            msg!("{:?} not a block", self)
        }
    }

    pub fn as_array(&self) -> Result<&[Item]> {
        if let Item::Array(a) = self {
            Ok(a)
        } else {
            panic!("{:?} not an array", self);
        }
    }

    pub fn as_bool(&self) -> Result<bool> {
        if let &Item::Bool(b) = self {
            Ok(b)
        } else {
            panic!("{:?} not a bool", self);
        }
    }

    pub fn into_dict(self) -> Result<HashMap<String, Item>> {
        if let Item::Dict(d) = self {
            Ok(d)
        } else {
            panic!("{:?} not a dict", self);
        }
    }
}

impl From<i32> for Item {
    fn from(val: i32) -> Self {
        Item::Number(val)
    }
}

impl From<f32> for Item {
    fn from(val: f32) -> Self {
        Item::Float(val)
    }
}

impl From<String> for Item {
    fn from(val: String) -> Self {
        Item::Key(val)
    }
}

impl From<bool> for Item {
    fn from(val: bool) -> Self {
        Item::Bool(val)
    }
}

impl From<HashMap<String, Item>> for Item {
    fn from(val: HashMap<String, Item>) -> Self {
        Item::Dict(val)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Stack<T>
where
    T: PartialEq + Eq,
{
    pub(crate) inner: Vec<T>,
}

impl<T> Stack<T>
where
    T: PartialEq + Eq,
{
    pub fn new() -> Self {
        Stack { inner: vec![] }
    }

    pub fn push(&mut self, val: T) {
        self.inner.push(val);
    }

    pub fn pop(&mut self) -> Result<T> {
        self.inner
            .pop()
            .ok_or_else(|| Report::msg("/stackunderflow"))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
