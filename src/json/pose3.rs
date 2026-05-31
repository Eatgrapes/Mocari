use serde::Deserialize;

use crate::{Error, Result};

const FORMAT: &str = "pose3.json";

#[derive(Debug, Clone, PartialEq)]
pub struct Pose3 {
    kind: String,
    fade_in_time: Option<f32>,
    groups: Vec<Vec<PosePart>>,
}

impl Pose3 {
    pub fn from_json_str(source: &str) -> Result<Self> {
        let raw: RawPose3 = serde_json::from_str(source).map_err(|error| Error::InvalidJson {
            format: FORMAT,
            message: error.to_string(),
        })?;

        Ok(Self {
            kind: raw.kind,
            fade_in_time: raw.fade_in_time,
            groups: raw.groups,
        })
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn fade_in_time(&self) -> Option<f32> {
        self.fade_in_time
    }

    pub fn groups(&self) -> &[Vec<PosePart>] {
        &self.groups
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawPose3 {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "FadeInTime", default)]
    fade_in_time: Option<f32>,
    #[serde(rename = "Groups", default)]
    groups: Vec<Vec<PosePart>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PosePart {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Link", default)]
    links: Vec<String>,
}

impl PosePart {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn links(&self) -> &[String] {
        &self.links
    }
}
