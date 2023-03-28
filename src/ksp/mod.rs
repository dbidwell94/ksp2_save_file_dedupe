use serde::{de::Visitor, Deserialize, Serialize};
use serde_json::Value;

struct MultiUintFloatVisitor {}

impl<'de> Visitor<'de> for MultiUintFloatVisitor {
    type Value = MultiUintFloat;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Unable to deserialize Number into MultiUintFloat")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MultiUintFloat::from(v))
    }
}

#[derive(Hash, Debug, PartialEq, Eq)]
pub struct MultiUintFloat {
    pub integral: u64,
    pub fractional: u64,
}

impl MultiUintFloat {
    fn f64(&self) -> f64 {
        let built_str = format!("{0}.{1}", self.integral, self.fractional);
        return built_str.parse().unwrap();
    }
}

impl Serialize for MultiUintFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.f64())
    }
}

impl<'de> Deserialize<'de> for MultiUintFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_f64(MultiUintFloatVisitor {})
    }
}

impl From<f64> for MultiUintFloat {
    fn from(value: f64) -> Self {
        let val_string = value.to_string();
        let temp_str: Vec<&str> = val_string.split('.').collect();
        let value_one: u64 = temp_str[0].parse().unwrap();
        let value_two: u64 = temp_str[1].parse().unwrap();
        Self {
            integral: value_one,
            fractional: value_two,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct KspSaveData {
    pub metadata: Value,
    pub properties: Value,
    pub galaxy_definition_key: Value,
    pub session_manager: Value,
    pub session_guid: Value,
    pub agencies: Value,
    pub campaign_players: Value,
    pub vessels: Value,
    #[serde(rename = "missionData")]
    pub mission_data: Value,
    pub colony_data: Value,
    pub kerbal_data: Value,
    pub planted_flags: Value,
    pub travel_log_data: TravelLogData,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct TravelLogData {
    pub object_events: Vec<ObjectEvent>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectEvent {
    pub travel_object_ids: Vec<String>,
    pub event_key: String,
    #[serde(rename = "UT")]
    pub ut: MultiUintFloat,
    pub flight_report_args: Vec<String>,
}
