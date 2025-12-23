//! Pascal VOC XML format implementation.
//!
//! Implements the Pascal Visual Object Classes (VOC) annotation format,
//! which uses one XML file per image.

use std::io::Write;
use std::path::Path;

use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use crate::format::error::FormatError;
use crate::format::project::{
    AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ProjectMetadata, ShapeEntry,
};
use crate::format::traits::{
    AnnotationFormat, ExportOptions, ExportResult, FormatWarning, ImportOptions,
};

/// Pascal VOC XML format.
///
/// Supports:
/// - Bounding boxes only
/// - Per-image annotation files
/// - Object names and bndbox coordinates
///
/// Does not support:
/// - Polygons (skipped with warning)
/// - Points (skipped with warning)
/// - Per-image tags
/// - Category colors
pub struct PascalVocFormat;

impl AnnotationFormat for PascalVocFormat {
    fn id(&self) -> &'static str {
        "voc"
    }

    fn display_name(&self) -> &'static str {
        "Pascal VOC (XML)"
    }

    fn extensions(&self) -> &[&'static str] {
        &["xml"]
    }

    fn supports_polygon(&self) -> bool {
        false
    }

    fn supports_point(&self) -> bool {
        false
    }

    fn supports_per_image(&self) -> bool {
        true
    }

    fn export(
        &self,
        data: &ProjectData,
        path: &Path,
        _options: &ExportOptions,
    ) -> Result<ExportResult, FormatError> {
        log::info!("Exporting Pascal VOC annotations to {:?}", path);

        let output_dir = path;
        std::fs::create_dir_all(output_dir)?;

        let mut warnings = Vec::new();
        let mut files_created = Vec::new();
        let mut annotations_exported = 0;

        // Build category_id -> name map
        let cat_names: std::collections::HashMap<u32, &str> = data
            .categories
            .iter()
            .map(|c| (c.id, c.name.as_str()))
            .collect();

        // Write per-image annotation files
        for image in &data.images {
            let (width, height) = image.dimensions.unwrap_or((0, 0));
            if width == 0 || height == 0 {
                warnings.push(
                    FormatWarning::warning(format!(
                        "Image '{}' has no dimensions, using 0x0",
                        image.filename
                    ))
                    .with_image(&image.path),
                );
            }

            let stem = Path::new(&image.filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let xml_path = output_dir.join(format!("{}.xml", stem));

            let folder = data
                .folder
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let xml_content = self.build_xml(
                folder,
                &image.filename,
                width,
                height,
                &image.annotations,
                &cat_names,
                &mut warnings,
                &image.path,
                &mut annotations_exported,
            )?;

            std::fs::write(&xml_path, xml_content)?;
            files_created.push(xml_path);
        }

        log::info!(
            "Exported {} images with {} annotations ({} warnings)",
            data.images.len(),
            annotations_exported,
            warnings.len()
        );

        Ok(ExportResult {
            images_exported: data.images.len(),
            annotations_exported,
            warnings,
            files_created,
        })
    }

    fn import(&self, path: &Path, _options: &ImportOptions) -> Result<ProjectData, FormatError> {
        log::info!("Importing Pascal VOC annotations from {:?}", path);

        let input_dir = path;
        if !input_dir.is_dir() {
            return Err(FormatError::invalid_format(
                "Pascal VOC import requires a directory path",
            ));
        }

        let mut data = ProjectData::new();
        data.folder = input_dir.to_path_buf();

        // Track unique category names
        let mut category_map: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        let mut next_cat_id = 0u32;

        // Find all .xml files
        let xml_files: Vec<_> = std::fs::read_dir(input_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "xml"))
            .collect();

        // Import each annotation file
        for xml_path in xml_files {
            match self.parse_xml(&xml_path, &mut category_map, &mut next_cat_id) {
                Ok(entry) => data.images.push(entry),
                Err(e) => {
                    log::warn!("Failed to parse {:?}: {}", xml_path, e);
                }
            }
        }

        // Convert category map to entries
        let mut categories: Vec<_> = category_map.into_iter().collect();
        categories.sort_by_key(|(_, id)| *id);
        for (name, id) in categories {
            data.categories.push(CategoryEntry::new(id, name));
        }

        data.metadata = ProjectMetadata::new();
        data.metadata.extra.insert(
            "imported_from".into(),
            serde_json::Value::String("pascal_voc".into()),
        );

        log::info!(
            "Imported {} images with {} annotations",
            data.images.len(),
            data.total_annotations()
        );

        Ok(data)
    }
}

impl PascalVocFormat {
    /// Build XML content for an image.
    #[allow(clippy::too_many_arguments)]
    fn build_xml(
        &self,
        folder: &str,
        filename: &str,
        width: u32,
        height: u32,
        annotations: &[AnnotationEntry],
        cat_names: &std::collections::HashMap<u32, &str>,
        warnings: &mut Vec<FormatWarning>,
        image_path: &Path,
        annotations_exported: &mut usize,
    ) -> Result<String, FormatError> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

        // XML declaration
        writer
            .write_event(Event::Decl(quick_xml::events::BytesDecl::new(
                "1.0", None, None,
            )))
            .map_err(|e| FormatError::Xml(e.into()))?;

        // <annotation>
        writer
            .write_event(Event::Start(BytesStart::new("annotation")))
            .map_err(|e| FormatError::Xml(e.into()))?;

        // <folder>
        self.write_text_element(&mut writer, "folder", folder)?;

        // <filename>
        self.write_text_element(&mut writer, "filename", filename)?;

        // <size>
        writer
            .write_event(Event::Start(BytesStart::new("size")))
            .map_err(|e| FormatError::Xml(e.into()))?;
        self.write_text_element(&mut writer, "width", &width.to_string())?;
        self.write_text_element(&mut writer, "height", &height.to_string())?;
        self.write_text_element(&mut writer, "depth", "3")?;
        writer
            .write_event(Event::End(BytesEnd::new("size")))
            .map_err(|e| FormatError::Xml(e.into()))?;

        // <segmented>
        self.write_text_element(&mut writer, "segmented", "0")?;

        // <object> elements
        for ann in annotations {
            match &ann.shape {
                ShapeEntry::BoundingBox {
                    x,
                    y,
                    width: w,
                    height: h,
                } => {
                    let name = cat_names
                        .get(&ann.category_id)
                        .copied()
                        .unwrap_or("unknown");

                    writer
                        .write_event(Event::Start(BytesStart::new("object")))
                        .map_err(|e| FormatError::Xml(e.into()))?;

                    self.write_text_element(&mut writer, "name", name)?;
                    self.write_text_element(&mut writer, "pose", "Unspecified")?;
                    self.write_text_element(&mut writer, "truncated", "0")?;
                    self.write_text_element(&mut writer, "difficult", "0")?;

                    // <bndbox>
                    writer
                        .write_event(Event::Start(BytesStart::new("bndbox")))
                        .map_err(|e| FormatError::Xml(e.into()))?;
                    self.write_text_element(&mut writer, "xmin", &(*x as u32).to_string())?;
                    self.write_text_element(&mut writer, "ymin", &(*y as u32).to_string())?;
                    self.write_text_element(&mut writer, "xmax", &((x + w) as u32).to_string())?;
                    self.write_text_element(&mut writer, "ymax", &((y + h) as u32).to_string())?;
                    writer
                        .write_event(Event::End(BytesEnd::new("bndbox")))
                        .map_err(|e| FormatError::Xml(e.into()))?;

                    writer
                        .write_event(Event::End(BytesEnd::new("object")))
                        .map_err(|e| FormatError::Xml(e.into()))?;

                    *annotations_exported += 1;
                }
                ShapeEntry::Point { .. } => {
                    warnings.push(
                        FormatWarning::warning(
                            "Skipped point annotation (Pascal VOC only supports bounding boxes)",
                        )
                        .with_image(image_path),
                    );
                }
                ShapeEntry::Polygon { .. } => {
                    warnings.push(
                        FormatWarning::warning(
                            "Skipped polygon annotation (Pascal VOC only supports bounding boxes)",
                        )
                        .with_image(image_path),
                    );
                }
            }
        }

        // </annotation>
        writer
            .write_event(Event::End(BytesEnd::new("annotation")))
            .map_err(|e| FormatError::Xml(e.into()))?;

        let result = writer.into_inner();
        String::from_utf8(result).map_err(|_| FormatError::invalid_format("Invalid UTF-8 in XML"))
    }

    /// Write a simple text element.
    fn write_text_element<W: Write>(
        &self,
        writer: &mut Writer<W>,
        name: &str,
        value: &str,
    ) -> Result<(), FormatError> {
        writer
            .write_event(Event::Start(BytesStart::new(name)))
            .map_err(|e| FormatError::Xml(e.into()))?;
        writer
            .write_event(Event::Text(BytesText::new(value)))
            .map_err(|e| FormatError::Xml(e.into()))?;
        writer
            .write_event(Event::End(BytesEnd::new(name)))
            .map_err(|e| FormatError::Xml(e.into()))?;
        Ok(())
    }

    /// Parse a Pascal VOC XML file.
    fn parse_xml(
        &self,
        path: &Path,
        category_map: &mut std::collections::HashMap<String, u32>,
        next_cat_id: &mut u32,
    ) -> Result<ImageEntry, FormatError> {
        use quick_xml::Reader;

        let content = std::fs::read_to_string(path)?;
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut filename = String::new();
        let mut width = 0u32;
        let mut height = 0u32;
        let mut annotations = Vec::new();
        let mut ann_id = 0u32;

        // Current parsing state
        let mut current_element = String::new();
        let mut in_object = false;
        let mut in_bndbox = false;
        let mut in_size = false;

        // Current object data
        let mut obj_name = String::new();
        let mut xmin = 0u32;
        let mut ymin = 0u32;
        let mut xmax = 0u32;
        let mut ymax = 0u32;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_element = name.clone();

                    match name.as_str() {
                        "object" => {
                            in_object = true;
                            obj_name.clear();
                            xmin = 0;
                            ymin = 0;
                            xmax = 0;
                            ymax = 0;
                        }
                        "bndbox" => in_bndbox = true,
                        "size" => in_size = true,
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match name.as_str() {
                        "object" => {
                            if !obj_name.is_empty() && xmax > xmin && ymax > ymin {
                                // Get or create category ID
                                let cat_id =
                                    *category_map.entry(obj_name.clone()).or_insert_with(|| {
                                        let id = *next_cat_id;
                                        *next_cat_id += 1;
                                        id
                                    });

                                annotations.push(AnnotationEntry::new(
                                    ann_id,
                                    cat_id,
                                    ShapeEntry::BoundingBox {
                                        x: xmin as f32,
                                        y: ymin as f32,
                                        width: (xmax - xmin) as f32,
                                        height: (ymax - ymin) as f32,
                                    },
                                ));
                                ann_id += 1;
                            }
                            in_object = false;
                        }
                        "bndbox" => in_bndbox = false,
                        "size" => in_size = false,
                        _ => {}
                    }
                    current_element.clear();
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();

                    if in_size {
                        match current_element.as_str() {
                            "width" => width = text.parse().unwrap_or(0),
                            "height" => height = text.parse().unwrap_or(0),
                            _ => {}
                        }
                    } else if in_object {
                        if in_bndbox {
                            match current_element.as_str() {
                                "xmin" => xmin = text.parse().unwrap_or(0),
                                "ymin" => ymin = text.parse().unwrap_or(0),
                                "xmax" => xmax = text.parse().unwrap_or(0),
                                "ymax" => ymax = text.parse().unwrap_or(0),
                                _ => {}
                            }
                        } else if current_element == "name" {
                            obj_name = text;
                        }
                    } else if current_element == "filename" {
                        filename = text;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(FormatError::Xml(e.into()));
                }
                _ => {}
            }
        }

        // Construct image entry
        let folder = path.parent().unwrap_or(Path::new("."));
        let image_path = if filename.is_empty() {
            // Use XML filename with image extension
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            find_image_for_stem(folder, stem)
        } else {
            folder.join(&filename)
        };

        let mut entry = ImageEntry::new(image_path);
        entry.filename = filename;
        if width > 0 && height > 0 {
            entry = entry.with_dimensions(width, height);
        }
        entry.annotations = annotations;

        Ok(entry)
    }
}

/// Find an image file matching the given stem in the directory.
fn find_image_for_stem(dir: &Path, stem: &str) -> std::path::PathBuf {
    const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];

    for ext in IMAGE_EXTENSIONS {
        let path = dir.join(format!("{}.{}", stem, ext));
        if path.exists() {
            return path;
        }
    }

    dir.join(format!("{}.png", stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_metadata() {
        let format = PascalVocFormat;
        assert_eq!(format.id(), "voc");
        assert!(!format.supports_polygon());
        assert!(!format.supports_point());
        assert!(format.supports_per_image());
    }
}
