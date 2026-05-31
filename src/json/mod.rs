mod cdi3;
mod model3;
mod motion3;

pub use cdi3::{Cdi3, CdiEntry, CdiPart};
pub use model3::{Group, HitArea, Model3, MotionReference};
pub use motion3::{Motion3, MotionCurve, MotionMeta, MotionPoint, MotionSegment};
