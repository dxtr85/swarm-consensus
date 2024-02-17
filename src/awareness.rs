use std::fmt;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Awareness {
    Unaware,
    Aware(u8),
    Confused(u8),
}
impl Awareness {
    pub fn neighborhood(&self) -> Option<u8> {
        if let Awareness::Aware(hood) = self {
            return Some(*hood);
        }
        None
    }

    pub fn is_aware(&self) -> bool {
        matches!(self, Awareness::Aware(_))
    }

    pub fn is_unaware(&self) -> bool {
        matches!(self, Awareness::Unaware)
    }
    pub fn is_confused(&self) -> bool {
        matches!(self, Awareness::Confused(_cd))
    }
}

impl fmt::Display for Awareness{
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         match self{
             Awareness::Unaware => write!(f, "UNA"),
             Awareness::Aware(n)=> write!(f, "A-{}", n),
             Awareness::Confused(n)=> write!(f, "C-{}",n)
         }
     }
}
