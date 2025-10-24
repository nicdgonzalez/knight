use chrono::NaiveTime;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Daylight {
    pub sunrise: Option<NaiveTime>,
    pub sunset: Option<NaiveTime>,
}

impl Daylight {
    pub const fn is_defined(&self) -> bool {
        matches!((self.sunrise, self.sunset), (Some(_), Some(_)))
    }
}
