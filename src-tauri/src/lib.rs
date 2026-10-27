use pdfjiff_core::{
    compress::{try_compress, CompressOptions, CompressionPreset},
    inspect,
    pages::PdfMerger,
    Inspection, Orientation, PageSize, PdfBuilder,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_MERGE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Capability {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilitiesResponse {
    app_version: &'static str,
    schema_version: u8,
    offline: bool,
    available: Vec<Capability>,
    planned: Vec<Capability>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectionResponse {
    path: String,
    file_name: String,
    input_bytes: u64,
    page_count: Option<u32>,
    pages: Vec<PageResponse>,
    encrypted: bool,
    signatures_detected: bool,
    assembly_sensitive_features: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageResponse {
    number: usize,
    width_pt: f32,
    height_pt: f32,
    rotation: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompressRequest {
    input: String,
    output: String,
    preset: String,
    target_bytes: Option<u64>,
    overwrite: bool,
    allow_signature_invalidation: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MergeRequest {
    inputs: Vec<String>,
    output: String,
    overwrite: bool,
    allow_signature_invalidation: bool,
    allow_structure_loss: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageToPdfRequest {
    inputs: Vec<String>,
    output: String,
    overwrite: bool,
    page_size: String,
    orientation: String,
    margin_mm: f32,
    dpi: f32,
    recompress_quality: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationResponse {
    operation: &'static str,
    output: String,
    input_bytes: u64,
    output_bytes: u64,
    page_count: u32,
    elapsed_ms: u128,
    saved_ratio: Option<f64>,
    attempts: Option<u8>,
    images_recompressed: Option<u32>,
    images_downsampled: Option<u32>,
    warnings: Vec<String>,
}

#[tauri::command]
fn capabilities() -> CapabilitiesResponse {
    CapabilitiesResponse {
        app_version: env!("CARGO_PKG_VERSION"),
        schema_version: 1,
        offline: true,
        available: vec![
            capability(
                "document.inspect",
                "Inspect",
                "Read PDF structure without modifying it",
                true,
            ),
            capability(
                "pdf.compress",
                "Compress",
                "Image-aware or lossless PDF compression",
                true,
            ),
            capability(
                "pdf.merge",
                "Merge",
                "Combine PDFs in an explicit order",
                true,
            ),
            capability(
                "image.to_pdf",
                "Images to PDF",
                "Create an ordered PDF from JPG, JPEG, JFIF, PNG or WebP images",
                true,
            ),
        ],
        planned: vec![
            capability("pdf.split", "Split", "Split by page or range", false),
            capability(
                "pages.organize",
                "Organize pages",
                "Select, reorder, delete and rotate pages",
                false,
            ),
            capability(
                "pdf.to_image",
                "PDF to images",
                "Export PDF pages as images",
                false,
            ),
            capability("ocr", "OCR", "Searchable text for scanned documents", false),
        ],
    }
}

fn capability(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    available: bool,
) -> Capability {
    Capability {
        id,
        label,
        description,
        available,
    }
}

#[tauri::command]
async fn inspect_file(path: String) -> Result<InspectionResponse, String> {
    tauri::async_runtime::spawn_blocking(move || inspect_path(Path::new(&path)))
        .await
        .map_err(|error| format!("Inspection task stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn compress_pdf(request: CompressRequest) -> Result<OperationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || compress_impl(request))
        .await
        .map_err(|error| format!("Compression task stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn merge_pdfs(request: MergeRequest) -> Result<OperationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || merge_impl(request))
        .await
        .map_err(|error| format!("Merge task stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn images_to_pdf(request: ImageToPdfRequest) -> Result<OperationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images_to_pdf_impl(request))
        .await
        .map_err(|error| format!("Image conversion task stopped unexpectedly: {error}"))?
}

fn inspect_path(path: &Path) -> Result<InspectionResponse, String> {
    let bytes = read_limited(path, MAX_INPUT_BYTES)?;
    let inspection = inspect(&bytes).map_err(|error| error.to_string())?;
    Ok(inspection_response(path, bytes.len() as u64, inspection))
}

fn inspection_response(
    path: &Path,
    input_bytes: u64,
    inspection: Inspection,
) -> InspectionResponse {
    InspectionResponse {
        path: path.to_string_lossy().into_owned(),
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("document.pdf")
            .to_owned(),
        input_bytes,
        page_count: inspection.page_count,
        pages: inspection
            .pages
            .into_iter()
            .enumerate()
            .map(|(index, page)| PageResponse {
                number: index + 1,
                width_pt: page.width_pt,
                height_pt: page.height_pt,
                rotation: page.rotation,
            })
            .collect(),
        encrypted: inspection.encrypted,
        signatures_detected: inspection.signatures_detected,
        assembly_sensitive_features: inspection.assembly_sensitive_features,
    }
}

fn compress_impl(request: CompressRequest) -> Result<OperationResponse, String> {
    let started = Instant::now();
    let input = PathBuf::from(&request.input);
    let output = PathBuf::from(&request.output);
    validate_output(&output, std::slice::from_ref(&input), request.overwrite)?;
    let data = read_limited(&input, MAX_INPUT_BYTES)?;
    let before = inspect(&data).map_err(|error| error.to_string())?;
    if before.encrypted {
        return Err("This PDF needs a password. Decrypt a copy before compressing it.".into());
    }
    if before.signatures_detected && !request.allow_signature_invalidation {
        return Err("This PDF contains a detected signature. Confirm signature invalidation before writing a new file.".into());
    }

    let (preset, lossless) = match request.preset.as_str() {
        "quality" => (CompressionPreset::Quality, false),
        "balanced" => (CompressionPreset::Balanced, false),
        "small" => (CompressionPreset::Small, false),
        "lossless" => (CompressionPreset::Quality, true),
        _ => return Err("Choose quality, balanced, small, or lossless compression.".into()),
    };
    let settings = if lossless {
        CompressOptions::lossless()
    } else {
        CompressOptions::from_preset(preset)
    };
    let mut best = try_compress(&data, &settings)?;
    let mut attempts = 1u8;
    if let Some(target) = request.target_bytes {
        if target == 0 {
            return Err("Target size must be greater than zero.".into());
        }
        if lossless && u64::from(best.compressed_size()) > target {
            return Err("Lossless compression cannot reach that target. Choose a lossy preset or increase the target.".into());
        }
        if !lossless && u64::from(best.compressed_size()) > target {
            for (quality, max_edge) in [(65, 1800), (50, 1400), (35, 1000), (25, 800), (15, 600)] {
                let candidate = try_compress(
                    &data,
                    &CompressOptions::new(preset, quality, max_edge, false, false, false, false),
                )?;
                attempts += 1;
                if candidate.compressed_size() < best.compressed_size() {
                    best = candidate;
                }
                if u64::from(best.compressed_size()) <= target {
                    break;
                }
            }
        }
        if u64::from(best.compressed_size()) > target {
            return Err(format!(
                "The requested target was not reached after {attempts} attempts. The smallest result was {} bytes; no output was written.",
                best.compressed_size()
            ));
        }
    }

    let original_size = u64::from(best.original_size());
    let output_size = u64::from(best.compressed_size());
    let page_count = best.page_count();
    let images_recompressed = best.images_recompressed();
    let images_downsampled = best.images_downsampled();
    let bytes = best.take();
    let after =
        inspect(&bytes).map_err(|error| format!("Compressed output did not validate: {error}"))?;
    if after.page_count != before.page_count {
        return Err("Compressed output changed the page count; no output was written.".into());
    }
    atomic_write(&output, &bytes, request.overwrite)?;

    let mut warnings = Vec::new();
    if before.signatures_detected {
        warnings.push("The detected document signature is no longer valid after rewriting.".into());
    }
    Ok(OperationResponse {
        operation: "pdf.compress",
        output: output.to_string_lossy().into_owned(),
        input_bytes: original_size,
        output_bytes: output_size,
        page_count,
        elapsed_ms: started.elapsed().as_millis(),
        saved_ratio: Some(if original_size == 0 {
            0.0
        } else {
            (original_size.saturating_sub(output_size)) as f64 / original_size as f64
        }),
        attempts: Some(attempts),
        images_recompressed: Some(images_recompressed),
        images_downsampled: Some(images_downsampled),
        warnings,
    })
}

fn merge_impl(request: MergeRequest) -> Result<OperationResponse, String> {
    let started = Instant::now();
    if request.inputs.len() < 2 {
        return Err("Choose at least two PDFs to merge.".into());
    }
    let inputs: Vec<PathBuf> = request.inputs.iter().map(PathBuf::from).collect();
    let output = PathBuf::from(&request.output);
    validate_output(&output, &inputs, request.overwrite)?;

    let mut total_bytes = 0u64;
    let mut total_pages = 0u32;
    let mut signature_found = false;
    let mut sensitive_features = Vec::<String>::new();
    let mut merger = PdfMerger::try_new()?;
    for input in &inputs {
        let data = read_limited(input, MAX_INPUT_BYTES)?;
        total_bytes = total_bytes
            .checked_add(data.len() as u64)
            .ok_or_else(|| "The selected inputs are too large.".to_string())?;
        if total_bytes > MAX_MERGE_BYTES {
            return Err("The selected PDFs exceed the 256 MiB merge limit.".into());
        }
        let info = inspect(&data).map_err(|error| error.to_string())?;
        if info.encrypted {
            return Err(format!(
                "{} needs a password. Decrypt a copy before merging it.",
                display_name(input)
            ));
        }
        signature_found |= info.signatures_detected;
        for feature in info.assembly_sensitive_features {
            if !sensitive_features.contains(&feature) {
                sensitive_features.push(feature);
            }
        }
        total_pages = total_pages.saturating_add(info.page_count.unwrap_or(0));
        merger.try_add_document(&data)?;
    }
    if signature_found && !request.allow_signature_invalidation {
        return Err("At least one PDF contains a detected signature. Confirm signature invalidation before merging.".into());
    }
    if !sensitive_features.is_empty() && !request.allow_structure_loss {
        return Err(format!(
            "The merger cannot preserve: {}. Confirm structure loss before merging.",
            sensitive_features.join(", ")
        ));
    }
    let bytes = merger.try_finish()?;
    let output_info =
        inspect(&bytes).map_err(|error| format!("Merged output did not validate: {error}"))?;
    if output_info.page_count != Some(total_pages) {
        return Err(
            "Merged output did not preserve the expected page count; no output was written.".into(),
        );
    }
    atomic_write(&output, &bytes, request.overwrite)?;

    let mut warnings = Vec::new();
    if signature_found {
        warnings.push(
            "Detected signatures from source PDFs are not valid in the merged document.".into(),
        );
    }
    if !sensitive_features.is_empty() {
        warnings.push(format!(
            "The merge omitted: {}.",
            sensitive_features.join(", ")
        ));
    }
    Ok(OperationResponse {
        operation: "pdf.merge",
        output: output.to_string_lossy().into_owned(),
        input_bytes: total_bytes,
        output_bytes: bytes.len() as u64,
        page_count: total_pages,
        elapsed_ms: started.elapsed().as_millis(),
        saved_ratio: None,
        attempts: None,
        images_recompressed: None,
        images_downsampled: None,
        warnings,
    })
}

fn images_to_pdf_impl(request: ImageToPdfRequest) -> Result<OperationResponse, String> {
    let started = Instant::now();
    if request.inputs.is_empty() {
        return Err("Choose at least one JPG, JPEG, JFIF, PNG or WebP image.".into());
    }
    if !request.margin_mm.is_finite() || !(0.0..=100.0).contains(&request.margin_mm) {
        return Err("Margin must be between 0 and 100 mm.".into());
    }
    if !request.dpi.is_finite() || !(18.0..=1200.0).contains(&request.dpi) {
        return Err("DPI must be between 18 and 1200.".into());
    }
    if request.recompress_quality > 100 {
        return Err("Image quality must be between 0 and 100.".into());
    }
    let page_size = match request.page_size.as_str() {
        "fit" => PageSize::FitToImage,
        "a4" => PageSize::A4,
        "letter" => PageSize::Letter,
        _ => return Err("Choose Fit to image, A4, or Letter page size.".into()),
    };
    let orientation = match request.orientation.as_str() {
        "auto" => Orientation::Auto,
        "portrait" => Orientation::Portrait,
        "landscape" => Orientation::Landscape,
        _ => return Err("Choose automatic, portrait, or landscape orientation.".into()),
    };
    let inputs: Vec<PathBuf> = request.inputs.iter().map(PathBuf::from).collect();
    let output = PathBuf::from(&request.output);
    validate_output(&output, &inputs, request.overwrite)?;

    let mut builder = PdfBuilder::try_new(
        page_size,
        orientation,
        request.margin_mm,
        request.dpi,
        request.recompress_quality,
    )?;
    let mut total_bytes = 0u64;
    for input in &inputs {
        let data = read_limited(input, MAX_INPUT_BYTES)?;
        total_bytes = total_bytes
            .checked_add(data.len() as u64)
            .ok_or_else(|| "The selected images are too large.".to_string())?;
        if total_bytes > MAX_MERGE_BYTES {
            return Err("The selected images exceed the 256 MiB total limit.".into());
        }
        builder.try_add_image(&data).map_err(|error| {
            format!(
                "Could not add {}. Use JPG, JPEG, JFIF, PNG or WebP: {error}",
                display_name(input)
            )
        })?;
    }
    let bytes = builder.try_finish()?;
    let output_info =
        inspect(&bytes).map_err(|error| format!("Generated PDF did not validate: {error}"))?;
    let page_count = inputs.len() as u32;
    if output_info.page_count != Some(page_count) {
        return Err("Generated PDF has an unexpected page count; no output was written.".into());
    }
    atomic_write(&output, &bytes, request.overwrite)?;
    Ok(OperationResponse {
        operation: "image.to_pdf",
        output: output.to_string_lossy().into_owned(),
        input_bytes: total_bytes,
        output_bytes: bytes.len() as u64,
        page_count,
        elapsed_ms: started.elapsed().as_millis(),
        saved_ratio: None,
        attempts: None,
        images_recompressed: None,
        images_downsampled: None,
        warnings: Vec::new(),
    })
}

fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not read {}: {error}", display_name(path)))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file.", display_name(path)));
    }
    if metadata.len() > limit {
        return Err(format!(
            "{} exceeds the 128 MiB per-file limit.",
            display_name(path)
        ));
    }
    fs::read(path).map_err(|error| format!("Could not read {}: {error}", display_name(path)))
}

fn validate_output(output: &Path, inputs: &[PathBuf], overwrite: bool) -> Result<(), String> {
    if output.file_name().is_none() {
        return Err("Choose an output file.".into());
    }
    if let Ok(metadata) = fs::symlink_metadata(output) {
        if metadata.file_type().is_symlink() || metadata.is_dir() {
            return Err("The output must be a regular file, not a symlink or directory.".into());
        }
        if !overwrite {
            return Err(
                "The output already exists. Enable overwrite or choose another name.".into(),
            );
        }
    }
    let output_identity = comparable_path(output)?;
    for input in inputs {
        let same_existing_file = output.exists()
            && same_file::is_same_file(input, output)
                .map_err(|error| format!("Could not compare input and output files: {error}"))?;
        if same_existing_file || comparable_path(input)? == output_identity {
            return Err("The output cannot replace or alias an input file.".into());
        }
    }
    Ok(())
}

fn comparable_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("Could not resolve {}: {error}", display_name(path)));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("Could not resolve the output folder: {error}"))?;
    Ok(parent.join(path.file_name().unwrap_or_default()))
}

fn atomic_write(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Could not create a temporary output: {error}"))?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file_mut().sync_all())
        .map_err(|error| format!("Could not write the output: {error}"))?;
    if overwrite {
        temporary
            .persist(path)
            .map_err(|error| format!("Could not publish the output: {}", error.error))?;
    } else {
        temporary
            .persist_noclobber(path)
            .map_err(|error| format!("Could not publish the output: {}", error.error))?;
    }
    Ok(())
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("the selected file")
        .to_owned()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            capabilities,
            inspect_file,
            compress_pdf,
            merge_pdfs,
            images_to_pdf
        ])
        .run(tauri::generate_context!())
        .expect("failed to run PDFJiff");
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, RgbImage};
    fn sample_jpeg(width: u32, height: u32) -> Vec<u8> {
        let mut bitmap = RgbImage::new(width, height);
        for (x, y, pixel) in bitmap.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 255) as u8, (y % 255) as u8, 140]);
        }
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 92)
            .write_image(
                bitmap.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgb8,
            )
            .expect("encode jpeg");
        jpeg
    }

    fn sample_jfif(width: u32, height: u32) -> Vec<u8> {
        let mut jpeg = sample_jpeg(width, height);
        if !jpeg.windows(5).any(|bytes| bytes == b"JFIF\0") {
            let app0 = [
                0xff, 0xe0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
            ];
            jpeg.splice(2..2, app0);
        }
        assert!(jpeg.windows(5).any(|bytes| bytes == b"JFIF\0"));
        jpeg
    }

    fn sample_pdf(width: u32, height: u32) -> Vec<u8> {
        let mut builder =
            PdfBuilder::try_new(PageSize::FitToImage, Orientation::Auto, 0.0, 96.0, 0)
                .expect("builder");
        builder
            .try_add_image(&sample_jpeg(width, height))
            .expect("add image");
        builder.try_finish().expect("finish")
    }

    #[test]
    fn inspect_reports_real_page_geometry() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("sample.pdf");
        fs::write(&input, sample_pdf(640, 480)).expect("fixture");

        let result = inspect_path(&input).expect("inspect");

        assert_eq!(result.page_count, Some(1));
        assert_eq!(result.pages.len(), 1);
        assert!(!result.encrypted);
        assert_eq!(result.file_name, "sample.pdf");
    }

    #[test]
    fn compression_writes_a_valid_atomic_output() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("input.pdf");
        let output = directory.path().join("output.pdf");
        fs::write(&input, sample_pdf(900, 700)).expect("fixture");

        let result = compress_impl(CompressRequest {
            input: input.to_string_lossy().into_owned(),
            output: output.to_string_lossy().into_owned(),
            preset: "balanced".into(),
            target_bytes: None,
            overwrite: false,
            allow_signature_invalidation: false,
        })
        .expect("compress");

        assert_eq!(result.operation, "pdf.compress");
        assert_eq!(result.page_count, 1);
        assert!(output.is_file());
        assert_eq!(
            inspect_path(&output).expect("inspect output").page_count,
            Some(1)
        );
    }

    #[test]
    fn merge_preserves_total_page_count_and_order_inputs() {
        let directory = tempfile::tempdir().expect("tempdir");
        let first = directory.path().join("first.pdf");
        let second = directory.path().join("second.pdf");
        let output = directory.path().join("merged.pdf");
        fs::write(&first, sample_pdf(640, 480)).expect("first fixture");
        fs::write(&second, sample_pdf(480, 640)).expect("second fixture");

        let result = merge_impl(MergeRequest {
            inputs: vec![
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            output: output.to_string_lossy().into_owned(),
            overwrite: false,
            allow_signature_invalidation: false,
            allow_structure_loss: false,
        })
        .expect("merge");

        assert_eq!(result.operation, "pdf.merge");
        assert_eq!(result.page_count, 2);
        assert_eq!(
            inspect_path(&output).expect("inspect output").page_count,
            Some(2)
        );
    }

    #[test]
    fn jfif_images_create_a_valid_ordered_pdf() {
        let directory = tempfile::tempdir().expect("tempdir");
        let first = directory.path().join("wide.jfif");
        let second = directory.path().join("tall.jfif");
        let output = directory.path().join("album.pdf");
        fs::write(&first, sample_jfif(160, 80)).expect("first fixture");
        fs::write(&second, sample_jfif(80, 160)).expect("second fixture");

        let result = images_to_pdf_impl(ImageToPdfRequest {
            inputs: vec![
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            output: output.to_string_lossy().into_owned(),
            overwrite: false,
            page_size: "fit".into(),
            orientation: "auto".into(),
            margin_mm: 0.0,
            dpi: 96.0,
            recompress_quality: 0,
        })
        .expect("convert images");

        assert_eq!(result.operation, "image.to_pdf");
        assert_eq!(result.page_count, 2);
        let inspection = inspect_path(&output).expect("inspect output");
        assert_eq!(inspection.page_count, Some(2));
        assert!(inspection.pages[0].width_pt > inspection.pages[0].height_pt);
        assert!(inspection.pages[1].height_pt > inspection.pages[1].width_pt);
    }

    #[test]
    fn output_cannot_alias_an_input() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("input.pdf");
        fs::write(&input, sample_pdf(320, 240)).expect("fixture");

        let error = compress_impl(CompressRequest {
            input: input.to_string_lossy().into_owned(),
            output: input.to_string_lossy().into_owned(),
            preset: "balanced".into(),
            target_bytes: None,
            overwrite: true,
            allow_signature_invalidation: false,
        })
        .expect_err("alias must fail");

        assert!(error.contains("cannot replace or alias"));
        assert_eq!(
            inspect_path(&input).expect("input preserved").page_count,
            Some(1)
        );
    }

    #[test]
    fn output_cannot_be_a_hard_link_to_an_input() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("input.pdf");
        let output = directory.path().join("linked.pdf");
        fs::write(&input, sample_pdf(320, 240)).expect("fixture");
        fs::hard_link(&input, &output).expect("hard link");

        let error = compress_impl(CompressRequest {
            input: input.to_string_lossy().into_owned(),
            output: output.to_string_lossy().into_owned(),
            preset: "balanced".into(),
            target_bytes: None,
            overwrite: true,
            allow_signature_invalidation: false,
        })
        .expect_err("hard-link alias must fail");

        assert!(error.contains("cannot replace or alias"));
        assert_eq!(
            inspect_path(&input).expect("input preserved").page_count,
            Some(1)
        );
    }
}
