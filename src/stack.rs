#[derive(Debug, Clone)]
pub enum Item {
    Number(i32),
    Float(f32),
    //Dict(()),
    Key(String),
    Block(String),
    ArrayOpen,
    Array(Vec<Item>),
}

impl Item {
    pub fn as_int(&self) -> i32 {
        if let &Item::Number(i) = self {
            i
        } else {
            panic!("{:?} not an int", self);
        }
    }

    pub fn as_float(&self) -> f32 {
        match *self {
            Item::Number(i) => i as f32,
            Item::Float(f) => f,
            _ => panic!("{:?} not a float", self),
        }
    }

    pub fn as_key(&self) -> &str {
        if let Item::Key(s) = self {
            s
        } else {
            panic!("{:?} not a key", self);
        }
    }

    pub fn as_block(&self) -> &str {
        if let Item::Block(s) = self {
            s
        } else {
            panic!("{:?} not a block", self);
        }
    }

    pub fn as_array(&self) -> &[Item] {
        if let Item::Array(a) = self {
            a
        } else {
            panic!("{:?} not an array", self);
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

pub struct Stack {
    pub(crate) inner: Vec<Item>,
}

impl Stack {
    pub fn new() -> Self {
        Stack { inner: vec![] }
    }

    pub fn push(&mut self, val: Item) {
        self.inner.push(val);
    }

    pub fn pop(&mut self) -> Option<Item> {
        self.inner.pop()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
