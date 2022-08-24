#[derive(Debug, PartialEq, Eq)]
pub enum AnimationType {
    None,
    Fall,
    Combo,
}

impl From<AnimationType> for u64 {
    fn from(value: AnimationType) -> Self {
        match value {
            AnimationType::None => 0,
            AnimationType::Fall => 1,
            AnimationType::Combo => 2,
        }
    }
}

impl TryFrom<u64> for AnimationType {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AnimationType::None),
            1 => Ok(AnimationType::Fall),
            2 => Ok(AnimationType::Combo),
            _ => Err(value),
        }
    }
}
