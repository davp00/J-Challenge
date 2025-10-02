#[derive(Debug, Clone)]
pub struct AppTime {
    date: u64,
}

impl AppTime {
    #[inline]
    pub fn new(date: u64) -> Self {
        Self { date }
    }

    pub fn is_before(&self, date2: &AppTime) -> bool {
        self.date < date2.date
    }

    pub fn is_before_or_eq(&self, date2: &AppTime) -> bool {
        self.is_before(date2) || self == date2
    }

    pub fn as_millis_u64(&self) -> u64 {
        self.date
    }
}

impl PartialEq for AppTime {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date
    }
}

impl From<u128> for AppTime {
    fn from(value: u128) -> Self {
        AppTime::new(value as u64)
    }
}
