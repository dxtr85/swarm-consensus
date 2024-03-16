use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Data(pub u32);

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}
