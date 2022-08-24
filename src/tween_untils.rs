#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TweenType {
    None,
    Fall,
}

impl From<TweenType> for u64 {
    fn from(value: TweenType) -> Self {
        match value {
            TweenType::None => 0,
            TweenType::Fall => 1,
        }
    }
}

impl TryFrom<u64> for TweenType {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TweenType::None),
            1 => Ok(TweenType::Fall),
            _ => Err(value),
        }
    }
}
