#[derive(Debug)]
pub enum I32OrString {
    Num(i32),
    Str(String),
}

impl I32OrString {
    pub fn to_i32(&self) -> Option<i32> {
        match self {
            I32OrString::Num(num) => Some(*num),
            I32OrString::Str(_) => None,
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self {
            I32OrString::Num(_) => None,
            I32OrString::Str(str) => Some(str.to_string()),
        }
    }
}