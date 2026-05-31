use serde::Deserialize;

use crate::{Error, Result};

const FORMAT: &str = "motion3.json";
const SUPPORTED_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct Motion3 {
    version: u32,
    meta: MotionMeta,
    curves: Vec<MotionCurve>,
}

impl Motion3 {
    pub fn from_json_str(source: &str) -> Result<Self> {
        let raw: RawMotion3 = serde_json::from_str(source).map_err(|error| Error::InvalidJson {
            format: FORMAT,
            message: error.to_string(),
        })?;

        if raw.version != SUPPORTED_VERSION {
            return Err(Error::UnsupportedVersion {
                format: FORMAT,
                version: raw.version,
            });
        }

        let curves = raw
            .curves
            .into_iter()
            .map(MotionCurve::try_from)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            version: raw.version,
            meta: raw.meta,
            curves,
        })
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn meta(&self) -> &MotionMeta {
        &self.meta
    }

    pub fn curves(&self) -> &[MotionCurve] {
        &self.curves
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MotionMeta {
    #[serde(rename = "Duration")]
    duration: f32,
    #[serde(rename = "Fps")]
    fps: f32,
    #[serde(rename = "Loop")]
    loop_motion: bool,
    #[serde(rename = "AreBeziersRestricted", default)]
    are_beziers_restricted: bool,
    #[serde(rename = "CurveCount", default)]
    curve_count: u32,
    #[serde(rename = "TotalSegmentCount", default)]
    total_segment_count: u32,
    #[serde(rename = "TotalPointCount", default)]
    total_point_count: u32,
    #[serde(rename = "UserDataCount", default)]
    user_data_count: u32,
    #[serde(rename = "TotalUserDataSize", default)]
    total_user_data_size: u32,
}

impl MotionMeta {
    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn is_looping(&self) -> bool {
        self.loop_motion
    }

    pub fn are_beziers_restricted(&self) -> bool {
        self.are_beziers_restricted
    }

    pub fn curve_count(&self) -> u32 {
        self.curve_count
    }

    pub fn total_segment_count(&self) -> u32 {
        self.total_segment_count
    }

    pub fn total_point_count(&self) -> u32 {
        self.total_point_count
    }

    pub fn user_data_count(&self) -> u32 {
        self.user_data_count
    }

    pub fn total_user_data_size(&self) -> u32 {
        self.total_user_data_size
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionCurve {
    target: String,
    id: String,
    first_point: MotionPoint,
    segments: Vec<MotionSegment>,
    fade_in_time: Option<f32>,
    fade_out_time: Option<f32>,
}

impl MotionCurve {
    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn first_point(&self) -> MotionPoint {
        self.first_point
    }

    pub fn segments(&self) -> &[MotionSegment] {
        &self.segments
    }

    pub fn fade_in_time(&self) -> Option<f32> {
        self.fade_in_time
    }

    pub fn fade_out_time(&self) -> Option<f32> {
        self.fade_out_time
    }

    pub fn sample(&self, time: f32) -> Option<f32> {
        if time <= self.first_point.time {
            return Some(self.first_point.value);
        }

        for segment in &self.segments {
            if time <= segment.end().time {
                return segment.sample(time);
            }
        }

        self.segments
            .last()
            .map(|segment| segment.end().value)
            .or(Some(self.first_point.value))
    }
}

impl TryFrom<RawMotionCurve> for MotionCurve {
    type Error = Error;

    fn try_from(raw: RawMotionCurve) -> Result<Self> {
        let (first_point, segments) = parse_segments(&raw.segments)?;

        Ok(Self {
            target: raw.target,
            id: raw.id,
            first_point,
            segments,
            fade_in_time: raw.fade_in_time,
            fade_out_time: raw.fade_out_time,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MotionPoint {
    pub time: f32,
    pub value: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MotionSegment {
    Linear {
        start: MotionPoint,
        end: MotionPoint,
    },
    Bezier {
        start: MotionPoint,
        control1: MotionPoint,
        control2: MotionPoint,
        end: MotionPoint,
    },
    Stepped {
        start: MotionPoint,
        end: MotionPoint,
    },
    InverseStepped {
        start: MotionPoint,
        end: MotionPoint,
    },
}

impl MotionSegment {
    pub fn end(&self) -> MotionPoint {
        match *self {
            Self::Linear { end, .. }
            | Self::Bezier { end, .. }
            | Self::Stepped { end, .. }
            | Self::InverseStepped { end, .. } => end,
        }
    }

    pub fn sample(&self, time: f32) -> Option<f32> {
        match *self {
            Self::Linear { start, end } => Some(sample_linear(start, end, time)),
            Self::Stepped { start, end } => {
                if time >= end.time {
                    Some(end.value)
                } else {
                    Some(start.value)
                }
            }
            Self::InverseStepped { start, end } => {
                if time <= start.time {
                    Some(start.value)
                } else {
                    Some(end.value)
                }
            }
            Self::Bezier { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawMotion3 {
    #[serde(rename = "Version")]
    version: u32,
    #[serde(rename = "Meta")]
    meta: MotionMeta,
    #[serde(rename = "Curves", default)]
    curves: Vec<RawMotionCurve>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawMotionCurve {
    #[serde(rename = "Target")]
    target: String,
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Segments")]
    segments: Vec<f32>,
    #[serde(rename = "FadeInTime", default)]
    fade_in_time: Option<f32>,
    #[serde(rename = "FadeOutTime", default)]
    fade_out_time: Option<f32>,
}

fn parse_segments(values: &[f32]) -> Result<(MotionPoint, Vec<MotionSegment>)> {
    if values.len() < 2 {
        return Err(invalid_segments(
            "segments must start with a time/value point",
        ));
    }

    let first_point = MotionPoint {
        time: values[0],
        value: values[1],
    };
    let mut cursor = 2;
    let mut start = first_point;
    let mut segments = Vec::new();

    while cursor < values.len() {
        let segment_type = segment_type(values[cursor])?;
        cursor += 1;

        let segment = match segment_type {
            0 => {
                let end = read_point(values, &mut cursor)?;
                MotionSegment::Linear { start, end }
            }
            1 => {
                let control1 = read_point(values, &mut cursor)?;
                let control2 = read_point(values, &mut cursor)?;
                let end = read_point(values, &mut cursor)?;
                MotionSegment::Bezier {
                    start,
                    control1,
                    control2,
                    end,
                }
            }
            2 => {
                let end = read_point(values, &mut cursor)?;
                MotionSegment::Stepped { start, end }
            }
            3 => {
                let end = read_point(values, &mut cursor)?;
                MotionSegment::InverseStepped { start, end }
            }
            _ => return Err(invalid_segments("unsupported segment type")),
        };

        start = segment.end();
        segments.push(segment);
    }

    Ok((first_point, segments))
}

fn read_point(values: &[f32], cursor: &mut usize) -> Result<MotionPoint> {
    if values.len().saturating_sub(*cursor) < 2 {
        return Err(invalid_segments("segment point is incomplete"));
    }

    let point = MotionPoint {
        time: values[*cursor],
        value: values[*cursor + 1],
    };
    *cursor += 2;
    Ok(point)
}

fn segment_type(value: f32) -> Result<u32> {
    if value.fract() != 0.0 || !(0.0..=3.0).contains(&value) {
        return Err(invalid_segments("segment type must be 0, 1, 2, or 3"));
    }

    Ok(value as u32)
}

fn sample_linear(start: MotionPoint, end: MotionPoint, time: f32) -> f32 {
    if start.time == end.time {
        return end.value;
    }

    let amount = ((time - start.time) / (end.time - start.time)).clamp(0.0, 1.0);
    start.value + (end.value - start.value) * amount
}

fn invalid_segments(message: impl Into<String>) -> Error {
    Error::InvalidJson {
        format: FORMAT,
        message: message.into(),
    }
}
