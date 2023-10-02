use std::fs;

use crate::SLEDError;
use glam::Vec2;
use serde::{Deserialize, Deserializer, Serialize};

static mut DEFAULT_DENSITY: f32 = 0.0;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub center_point: Vec2,
    #[serde(rename = "density")]
    #[serde(deserialize_with = "Config::set_default_density")]
    pub default_density: f32,
    #[serde(rename = "line_segment")]
    pub line_segments: Vec<LineSegment>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineSegment {
    pub start: Vec2,
    pub end: Vec2,
    #[serde(default = "Config::get_default_density")]
    pub density: f32,
}

impl Config {
    pub fn from_toml_file(path: &str) -> Result<Self, SLEDError> {
        let file_contents = fs::read_to_string(path).map_err(SLEDError::from_error)?;
        let config = toml::from_str(&file_contents).map_err(SLEDError::from_error)?;
        Ok(config)
    }

    fn set_default_density<'de, D>(des: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        // I hate this solution, for the record
        let den = f32::deserialize(des);
        unsafe { DEFAULT_DENSITY = den.unwrap_or(0.0) };
        Ok(unsafe { DEFAULT_DENSITY })
    }

    fn get_default_density() -> f32 {
        return unsafe { DEFAULT_DENSITY };
    }
}

impl LineSegment {
    pub fn length(&self) -> f32 {
        self.start.distance(self.end)
    }
}
