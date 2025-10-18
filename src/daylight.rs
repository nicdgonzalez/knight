use chrono::NaiveTime;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Daylight {
    pub sunrise: Option<NaiveTime>,
    pub sunset: Option<NaiveTime>,
}
