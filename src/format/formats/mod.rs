//! Annotation format implementations.

mod coco;
mod hvat_json;
mod pascal_voc;
mod yolo;

#[cfg(test)]
mod tests;

pub use coco::CocoFormat;
pub use hvat_json::HvatJsonFormat;
pub use pascal_voc::PascalVocFormat;
pub use yolo::YoloFormat;
