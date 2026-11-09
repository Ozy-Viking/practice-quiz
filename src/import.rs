//! Bounded import staging. Only a successful import replaces app-owned assets.
use crate::model::{QuizFile, validate_quiz};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    rc::Rc,
};

pub const JSON_LIMIT: usize = 2 * 1024 * 1024;
pub const ZIP_LIMIT: usize = 50 * 1024 * 1024;
pub const IMAGE_LIMIT: usize = 10 * 1024 * 1024;
const TOTAL_LIMIT: u64 = 100 * 1024 * 1024;
const ENTRY_LIMIT: usize = 1_000;
const PIXEL_LIMIT: u64 = 20_000_000;
const ZIP_LAYOUT_ERROR: &str =
    "Put quiz JSON and its images at the ZIP root, or together inside one enclosing folder.";

pub fn local_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(format!("Unsafe archive path: {path}"));
    }
    Ok(())
}

/// True for remote HTTPS, false for an exact package-relative path.
pub fn image_source(src: &str) -> Result<bool, String> {
    if let Ok(url) = url::Url::parse(src) {
        if url.scheme() == "https"
            && url.has_host()
            && url.username().is_empty()
            && url.password().is_none()
            && src
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
            && !src.contains(['\\', '\n', '\r', '\t'])
        {
            return Ok(true);
        }
        return Err(format!(
            "Image source must be an HTTPS URL without credentials: {src}"
        ));
    }
    local_path(src)?;
    Ok(false)
}

fn ignored(name: &str) -> bool {
    name.starts_with("__MACOSX/") || name.rsplit('/').next() == Some(".DS_Store")
}

fn quiz_filename(name: &str) -> bool {
    !name.contains('/') && name.to_ascii_lowercase().ends_with(".json")
}

#[derive(Clone)]
pub enum ImportSource {
    Json(Vec<u8>),
    Archive(Rc<Archive>),
}

pub struct Archive {
    bytes: Vec<u8>,
    entries: HashMap<String, zip::read::OwnedZipFileEntry>,
    package_root: String,
    pub candidates: Vec<String>,
}

impl Archive {
    pub fn new(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > ZIP_LIMIT {
            return Err("ZIP exceeds 50 MiB".into());
        }
        // This pinned API discovers the directory without allocating its index.
        // Use the same parsed entries for validation and reads, so an alternate
        // footer cannot switch directories after the entry-count check.
        let mut zip = zip::unstable::read::ZipIterable::try_new(
            Cursor::new(bytes.as_slice()),
            zip::read::Config::default(),
        )
        .map_err(|e| format!("Invalid ZIP: {e}"))?;
        if zip.len() > ENTRY_LIMIT {
            return Err("ZIP exceeds 1,000 entries".into());
        }
        let mut total = 0u64;
        let mut files = Vec::new();
        let mut entries = HashMap::new();
        for entry in zip.files().map_err(|e| format!("Invalid ZIP: {e}"))? {
            let entry = entry.map_err(|e| format!("Invalid ZIP entry: {e}"))?;
            if entries.len() >= ENTRY_LIMIT {
                return Err("ZIP exceeds 1,000 entries".into());
            }
            let name = entry.name().map_err(|e| e.to_string())?.into_owned();
            if entries.contains_key(&name) {
                return Err(format!("ZIP contains duplicate decoded name: {name}"));
            }
            local_path(if entry.is_dir() {
                name.strip_suffix('/').unwrap_or(&name)
            } else {
                &name
            })?;
            if entry.is_symlink() || entry.encrypted() {
                return Err("ZIP symlinks and encryption are unsupported".into());
            }
            if !matches!(
                entry.compression(),
                zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
            ) {
                return Err("ZIP supports only Stored and Deflate compression".into());
            }
            total = total
                .checked_add(entry.size())
                .ok_or("ZIP declared size overflow")?;
            if total > TOTAL_LIMIT {
                return Err("ZIP exceeds 100 MiB uncompressed".into());
            }
            if !entry.is_dir() && !ignored(&name) {
                files.push(name.clone());
            }
            entries.insert(name, entry.into());
        }
        // Root manifests retain priority, even if selecting one later fails.
        // Otherwise every substantive file must belong to a single wrapper.
        let package_root = if files.iter().any(|name| quiz_filename(name)) {
            String::new()
        } else {
            let folder = files
                .first()
                .and_then(|name| name.split_once('/'))
                .map(|(folder, _)| folder)
                .ok_or(ZIP_LAYOUT_ERROR)?;
            let prefix = format!("{folder}/");
            if !files.iter().all(|name| name.starts_with(&prefix)) {
                return Err(ZIP_LAYOUT_ERROR.into());
            }
            prefix
        };
        let mut candidates: Vec<String> = files
            .into_iter()
            .filter(|name| {
                name.strip_prefix(&package_root)
                    .is_some_and(|relative| !ignored(relative) && quiz_filename(relative))
            })
            .collect();
        candidates.sort();
        if candidates.is_empty() {
            return Err(ZIP_LAYOUT_ERROR.into());
        }
        Ok(Self {
            bytes,
            entries,
            package_root,
            candidates,
        })
    }

    fn read(&self, name: &str, limit: usize) -> Result<Vec<u8>, String> {
        if ignored(name) || name.strip_prefix(&self.package_root).is_some_and(ignored) {
            return Err(format!("Metadata is not an image: {name}"));
        }
        let mut reader = Cursor::new(self.bytes.as_slice());
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| format!("Cannot read {name}: file not found"))?
            .as_ref()
            .with_reader(&mut reader, zip::ZipReadOptions::default())
            .map_err(|e| format!("Cannot read {name}: {e}"))?;
        if entry.is_dir() || entry.size() > limit as u64 {
            return Err(format!(
                "{name} exceeds its {} MiB limit or is a directory",
                limit / 1024 / 1024
            ));
        }
        bounded_read(entry, limit).map_err(|e| format!("{name}: {e}"))
    }
}

fn bounded_read(reader: impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Actual decompressed bytes exceed limit".into());
    }
    Ok(bytes)
}

pub fn prepare(name: &str, bytes: Vec<u8>) -> Result<ImportSource, String> {
    match name
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("zip") => Archive::new(bytes).map(|a| ImportSource::Archive(Rc::new(a))),
        Some("json") if bytes.len() <= JSON_LIMIT => Ok(ImportSource::Json(bytes)),
        Some("json") => Err("JSON exceeds 2 MiB".into()),
        _ => Err("Choose a .json or .zip quiz file.".into()),
    }
}

fn check_dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > PIXEL_LIMIT {
        return Err("Image exceeds 20 megapixels or has empty dimensions".into());
    }
    Ok(())
}

/// Inspect every GIF frame without decoding LZW data. Browsers may expand the
/// canvas beyond the logical screen, so that header alone is not a safe bound.
fn check_gif_dimensions(bytes: &[u8]) -> Result<(), String> {
    fn take<'a>(bytes: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
        let data = bytes.get(..n).ok_or("Truncated GIF block")?;
        *bytes = &bytes[n..];
        Ok(data)
    }
    fn table(bytes: &mut &[u8], packed: u8) -> Result<(), String> {
        if packed & 0x80 != 0 {
            take(bytes, 3 * (1usize << ((packed & 7) + 1)))?;
        }
        Ok(())
    }
    fn blocks(bytes: &mut &[u8]) -> Result<(), String> {
        loop {
            let count = take(bytes, 1)?[0] as usize;
            if count == 0 {
                return Ok(());
            }
            take(bytes, count)?;
        }
    }
    fn word(bytes: &[u8]) -> u32 {
        u32::from(u16::from_le_bytes([bytes[0], bytes[1]]))
    }
    let mut remaining = bytes;
    let header = take(&mut remaining, 13)?;
    let mut width = word(&header[6..8]);
    let mut height = word(&header[8..10]);
    check_dimensions(width, height)?;
    table(&mut remaining, header[10])?;
    let mut frames = 0;
    loop {
        match take(&mut remaining, 1)?[0] {
            0x2c => {
                let frame = take(&mut remaining, 9)?;
                let frame_width = word(&frame[4..6]);
                let frame_height = word(&frame[6..8]);
                check_dimensions(frame_width, frame_height)?;
                width = width.max(word(&frame[..2]) + frame_width);
                height = height.max(word(&frame[2..4]) + frame_height);
                check_dimensions(width, height)?;
                table(&mut remaining, frame[8])?;
                take(&mut remaining, 1)?; // LZW minimum code size
                blocks(&mut remaining)?;
                frames += 1;
            }
            0x21 => {
                take(&mut remaining, 1)?; // extension label
                blocks(&mut remaining)?;
            }
            0x3b if frames > 0 => return Ok(()),
            _ => return Err("Invalid GIF block or missing frame".into()),
        }
    }
}

/// Bound compressed ancillary metadata independently: the PNG decoder can
/// ignore malformed/oversized ICC profiles, but a browser may still expand them.
fn check_png_metadata(bytes: &[u8]) -> Result<(), String> {
    fn terminated(bytes: &[u8]) -> Result<&[u8], String> {
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .ok_or("Invalid PNG metadata")?;
        Ok(&bytes[end + 1..])
    }
    const METADATA_LIMIT: u64 = 8 * 1024 * 1024;
    let mut remaining = &bytes[8..];
    let mut expanded = 0;
    while !remaining.is_empty() {
        let header = remaining.get(..8).ok_or("Truncated PNG chunk")?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        let chunk = remaining
            .get(..length.checked_add(12).ok_or("Invalid PNG chunk")?)
            .ok_or("Truncated PNG chunk")?;
        let data = &chunk[8..8 + length];
        let compressed = match &header[4..8] {
            b"iCCP" | b"zTXt" => {
                let value = terminated(data)?;
                if value.first() != Some(&0) {
                    return Err("Invalid PNG compression method".into());
                }
                Some(&value[1..])
            }
            b"iTXt" => {
                let value = terminated(data)?;
                let fields = value.get(..2).ok_or("Invalid PNG text metadata")?;
                let text = terminated(terminated(&value[2..])?)?;
                match fields {
                    [1, 0] => Some(text),
                    [0, 0] => None,
                    _ => return Err("Invalid PNG text compression".into()),
                }
            }
            _ => None,
        };
        if let Some(data) = compressed {
            let decoder = flate2::read::ZlibDecoder::new(data);
            expanded += std::io::copy(
                &mut decoder.take(METADATA_LIMIT - expanded + 1),
                &mut std::io::sink(),
            )
            .map_err(|e| format!("Invalid PNG compressed metadata: {e}"))?;
            if expanded > METADATA_LIMIT {
                return Err("PNG expanded metadata exceeds 8 MiB".into());
            }
        }
        remaining = &remaining[chunk.len()..];
        if &header[4..8] == b"IEND" {
            return Ok(());
        }
    }
    Err("Missing PNG end chunk".into())
}

pub fn image_info(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.len() > IMAGE_LIMIT {
        return Err("Image exceeds 10 MiB".into());
    }
    let format = image::guess_format(bytes)
        .map_err(|_| "Unsupported image signature (use PNG, JPEG, GIF or WebP)")?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        _ => return Err("Unsupported image format (use PNG, JPEG, GIF or WebP)".into()),
    };
    if format == image::ImageFormat::Gif {
        check_gif_dimensions(bytes)?;
    }
    if format == image::ImageFormat::Png {
        // Read IHDR before constructing a decoder: decoder construction can
        // expand compressed color profiles before returning dimensions.
        let header = bytes.get(..33).ok_or("Truncated PNG header")?;
        if &header[8..16] != b"\x00\x00\x00\x0dIHDR" {
            return Err("Invalid PNG IHDR".into());
        }
        check_dimensions(
            u32::from_be_bytes(header[16..20].try_into().unwrap()),
            u32::from_be_bytes(header[20..24].try_into().unwrap()),
        )?;
        check_png_metadata(bytes)?;
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    // PNG applies this during decoder construction, including iCCP expansion.
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| format!("Invalid image header or metadata limit exceeded: {e}"))?;
    check_dimensions(width, height)?;
    Ok(mime)
}

#[derive(Default)]
pub struct AssetStore {
    pub urls: HashMap<String, String>,
}
impl Drop for AssetStore {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        for url in self.urls.values() {
            let _ = web_sys::Url::revoke_object_url(url);
        }
    }
}

pub async fn load(
    source: ImportSource,
    manifest: Option<String>,
    current: impl Fn() -> bool,
) -> Result<(QuizFile, Rc<AssetStore>), String> {
    let bytes = match &source {
        ImportSource::Json(bytes) => bytes.clone(),
        ImportSource::Archive(archive) => {
            let name = manifest.as_ref().ok_or("Choose a quiz")?;
            if !archive.candidates.contains(name) {
                return Err("Choose one of the listed quiz JSON files".into());
            }
            archive.read(name, JSON_LIMIT)?
        }
    };
    // Match browser File.text()/TextDecoder's handling of a UTF-8 BOM while
    // retaining the full input length for all byte-limit accounting.
    let json = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let mut quiz: QuizFile =
        serde_json::from_slice(json).map_err(|e| format!("Invalid JSON: {e}"))?;
    validate_quiz(&quiz)?;
    quiz.source_manifest = manifest;
    let mut assets = AssetStore::default();
    let mut actual_total = bytes.len() as u64;
    for question in crate::model::all_questions(&quiz.questions) {
        for image in &question.images {
            if !current() {
                return Err("Import superseded".into());
            }
            if image_source(&image.src)? || assets.urls.contains_key(&image.src) {
                continue;
            }
            let ImportSource::Archive(archive) = &source else {
                return Err(format!(
                    "Question {} references {}. Upload a ZIP containing the quiz JSON and its images at the ZIP root or together inside one enclosing folder, or use an HTTPS image URL.",
                    question.id, image.src
                ));
            };
            let path = format!("{}{}", archive.package_root, image.src);
            let bytes = archive.read(&path, IMAGE_LIMIT)?;
            actual_total += bytes.len() as u64;
            if actual_total > TOTAL_LIMIT {
                return Err("Actual uncompressed bytes exceed 100 MiB".into());
            }
            let mime = image_info(&bytes).map_err(|e| format!("{}: {e}", image.src))?;
            #[cfg(target_arch = "wasm32")]
            {
                let parts = js_sys::Array::new();
                parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));
                let options = web_sys::BlobPropertyBag::new();
                options.set_type(mime);
                let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
                    .map_err(|_| "Cannot create image Blob")?;
                let url = web_sys::Url::create_object_url_with_blob(&blob)
                    .map_err(|_| "Cannot create image URL")?;
                assets.urls.insert(image.src.clone(), url.clone());
                let element =
                    web_sys::HtmlImageElement::new().map_err(|_| "Cannot create image decoder")?;
                element.set_src(&url);
                wasm_bindgen_futures::JsFuture::from(element.decode())
                    .await
                    .map_err(|_| format!("Cannot decode image: {}", image.src))?;
                if !current() {
                    return Err("Import superseded".into());
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = mime;
                image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
                assets.urls.insert(image.src.clone(), image.src.clone());
            }
        }
    }
    Ok((quiz, Rc::new(assets)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        io::Write,
        task::{Context, Poll, Waker},
    };
    use zip::write::SimpleFileOptions;

    #[test]
    fn multipart_archive_loads_both_shared_and_part_images() {
        let bytes = include_bytes!("../examples/authentication/authentication-quiz.zip").to_vec();
        let source = ImportSource::Archive(Rc::new(Archive::new(bytes).unwrap()));
        let (quiz, assets) = load_now(source, Some("quiz.json")).unwrap();
        assert_eq!(assets.urls.len(), 5);
        assert_eq!(quiz.source_manifest.as_deref(), Some("quiz.json"));
        let session = crate::model::build_seeded_session(&quiz.questions, 1, 42, 1.0);
        assert_eq!(session[0].images[0].src, "images/protocol.png");
        assert_eq!(
            session[0].subquestions[0].images[0].src,
            "images/mutual.png"
        );
        let results = crate::model::build_results(
            &session,
            &HashMap::new(),
            &HashMap::new(),
            &quiz.effective_config(),
        );
        let json: serde_json::Value = serde_json::from_str(
            &crate::model::results_json(&quiz, "authentication-quiz.zip", 42, &session, &results)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["source_manifest"], "quiz.json");
        assert_eq!(
            json["questions"][0]["images"][0]["src"],
            "images/protocol.png"
        );
        assert_eq!(
            json["questions"][0]["subquestions"][3]["images"][0]["src"],
            "images/fix.png"
        );
    }

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn load_now(
        source: ImportSource,
        name: Option<&str>,
    ) -> Result<(QuizFile, Rc<AssetStore>), String> {
        let mut future = std::pin::pin!(load(source, name.map(str::to_owned), || true));
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(result) => result,
            _ => panic!("Native import must be synchronous"),
        }
    }
    const MANIFEST: &[u8] = include_bytes!("../tests/fixtures/images/quiz.json");
    const PNG: &[u8] = include_bytes!("../tests/fixtures/images/shapes.png");
    const IMAGELESS_MANIFEST: &[u8] = br#"{"title":"First quiz","questions":[{"id":"q","question":"Explain","type":"short_answer"}]}"#;

    #[test]
    fn exact_paths_and_https_sources() {
        for path in ["shapes.png", "figures/week 1/形.jpg", "a%20b.png"] {
            assert_eq!(image_source(path), Ok(false));
        }
        for path in [
            "/a",
            "../a",
            "a/../b",
            "a//b",
            "a/./b",
            "a\\b",
            "C:/a",
            "//host/a",
            "data:image/png,x",
            "blob:https://a/b",
            "http://a/b",
            "https://user:pass@host/a",
            "https://host/\nx",
        ] {
            assert!(image_source(path).is_err(), "{path}");
        }
        assert_eq!(image_source("https://example.org/endpoint?id=3"), Ok(true));
    }
    #[test]
    fn plain_json_requires_remote_images_and_nonblank_fields_on_both_kinds() {
        let source = ImportSource::Json(MANIFEST.to_vec());
        assert!(
            load_now(source, None)
                .err()
                .unwrap()
                .contains("Upload a ZIP")
        );
        let mut quiz: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
        for q in quiz["questions"].as_array_mut().unwrap() {
            for image in q["images"].as_array_mut().unwrap() {
                image["src"] = "https://example.org/image".into();
            }
        }
        let (parsed, assets) =
            load_now(ImportSource::Json(serde_json::to_vec(&quiz).unwrap()), None).unwrap();
        assert!(assets.urls.is_empty());
        assert!(parsed.source_manifest.is_none());
        for index in 0..2 {
            for value in [
                serde_json::Value::Null,
                serde_json::json!([{"src":"https://example.org/a","alt":" "}]),
                serde_json::json!([{"src":"","alt":"ok"}]),
                serde_json::json!([{"src":7,"alt":"ok"}]),
            ] {
                let mut bad = quiz.clone();
                bad["questions"][index]["images"] = value;
                assert!(
                    load_now(ImportSource::Json(serde_json::to_vec(&bad).unwrap()), None).is_err()
                );
            }
        }
    }
    #[test]
    fn accepts_one_leading_utf8_bom_in_json_and_zip_manifests() {
        let plain = br#"{"title":"BOM quiz","questions":[{"id":"q","question":"Explain","type":"short_answer"}]}"#;
        let mut bom = b"\xef\xbb\xbf".to_vec();
        bom.extend_from_slice(plain);
        let (expected, _) = load_now(ImportSource::Json(plain.to_vec()), None).unwrap();
        let (actual, _) = load_now(ImportSource::Json(bom.clone()), None).unwrap();
        assert_eq!(actual, expected);
        let archive = prepare("bom.zip", zip(&[("named.json", &bom)])).unwrap();
        let (mut bundled, _) = load_now(archive, Some("named.json")).unwrap();
        assert_eq!(
            bundled.source_manifest.take().as_deref(),
            Some("named.json")
        );
        assert_eq!(bundled, expected);
        bom.splice(..0, b"\xef\xbb\xbf".iter().copied());
        assert!(load_now(ImportSource::Json(bom), None).is_err());
        assert!(load_now(ImportSource::Json(b"\xef\xbb\xbf{broken".to_vec()), None).is_err());
    }

    #[test]
    fn arbitrary_root_manifest_shared_images_and_export_provenance() {
        let source = prepare(
            "quiz.ZIP",
            include_bytes!("../tests/fixtures/images/single.zip").to_vec(),
        )
        .unwrap();
        let (quiz, assets) = load_now(source, Some("Biology revision.JSON")).unwrap();
        assert_eq!(assets.urls.len(), 2);
        let session = crate::model::build_seeded_session(&quiz.questions, 2, 42, 1.0);
        let results = crate::model::build_results(
            &session,
            &HashMap::new(),
            &HashMap::new(),
            &quiz.effective_config(),
        );
        let json: serde_json::Value = serde_json::from_str(
            &crate::model::results_json(&quiz, "quiz.ZIP", 42, &session, &results).unwrap(),
        )
        .unwrap();
        assert_eq!(json["format_version"], 4);
        assert_eq!(json["source_manifest"], "Biology revision.JSON");
        assert_eq!(json["source_file"], "quiz.ZIP");
        for q in json["questions"].as_array().unwrap() {
            let original = quiz
                .questions
                .iter()
                .find(|original| original.id == q["id"])
                .unwrap();
            assert_eq!(q["images"], serde_json::to_value(&original.images).unwrap());
        }
    }
    #[test]
    fn chooser_sorted_without_preference_and_invalid_selection_recovery() {
        let source = prepare(
            "chooser.zip",
            include_bytes!("../tests/fixtures/images/chooser.zip").to_vec(),
        )
        .unwrap();
        let ImportSource::Archive(archive) = &source else {
            panic!()
        };
        assert_eq!(
            archive.candidates,
            ["A revision.json", "notes.json", "quiz.json"]
        );
        assert!(load_now(source.clone(), Some("quiz.json")).is_err());
        assert!(load_now(source.clone(), Some("nested.json")).is_err());
        assert!(load_now(source, Some("A revision.json")).is_ok());
    }

    #[test]
    fn wrapped_quiz_resolves_deep_images_preserving_original_references() {
        let path = "images/a/b/c/d/image.png";
        let mut manifest: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
        for question in manifest["questions"].as_array_mut().unwrap() {
            for image in question["images"].as_array_mut().unwrap() {
                image["src"] = path.into();
            }
        }
        manifest["questions"][0]["images"][1]["src"] = "https://example.org/remote-image".into();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let root = prepare("root.zip", zip(&[("quiz.JSON", &bytes), (path, PNG)])).unwrap();
        let wrapped = prepare(
            "wrapped.zip",
            zip(&[
                ("Quiz 名/quiz.JSON", &bytes),
                ("Quiz 名/images/a/b/c/d/image.png", PNG),
                ("Quiz 名/nested/ignored.json", b"{bad"),
            ]),
        )
        .unwrap();
        let (mut expected, _) = load_now(root, Some("quiz.JSON")).unwrap();
        let (mut quiz, assets) = load_now(wrapped, Some("Quiz 名/quiz.JSON")).unwrap();
        assert_eq!(assets.urls.len(), 1);
        assert!(assets.urls.contains_key(path));
        assert_eq!(
            quiz.source_manifest.take().as_deref(),
            Some("Quiz 名/quiz.JSON")
        );
        expected.source_manifest = None;
        assert_eq!(quiz, expected);
    }

    #[test]
    fn wrapped_chooser_selects_each_valid_quiz_and_recovers_from_invalid_json() {
        let second = br#"{"title":"Second quiz","questions":[{"id":"other","question":"Describe","type":"short_answer"}]}"#;
        let source = prepare(
            "wrapped.zip",
            zip(&[
                ("quiz/quiz.json", b"{bad"),
                ("quiz/A revision.JSON", IMAGELESS_MANIFEST),
                ("quiz/second.json", second),
                ("quiz/nested/ignored.json", b"{bad"),
            ]),
        )
        .unwrap();
        let ImportSource::Archive(archive) = &source else {
            panic!()
        };
        assert_eq!(
            archive.candidates,
            ["quiz/A revision.JSON", "quiz/quiz.json", "quiz/second.json"]
        );
        assert!(
            load_now(source.clone(), Some("quiz/quiz.json"))
                .err()
                .unwrap()
                .contains("Invalid JSON")
        );
        assert!(load_now(source.clone(), Some("second.json")).is_err());
        for (name, title) in [
            ("quiz/A revision.JSON", "First quiz"),
            ("quiz/second.json", "Second quiz"),
        ] {
            let (quiz, _) = load_now(source.clone(), Some(name)).unwrap();
            assert_eq!(quiz.title, title);
            assert_eq!(quiz.source_manifest.as_deref(), Some(name));
        }
    }

    #[test]
    fn invalid_root_manifest_never_falls_back_to_a_wrapped_quiz() {
        let source = prepare(
            "root.zip",
            zip(&[
                ("root.json", b"{bad"),
                ("quiz/quiz.json", IMAGELESS_MANIFEST),
            ]),
        )
        .unwrap();
        let ImportSource::Archive(archive) = &source else {
            panic!()
        };
        assert_eq!(archive.candidates, ["root.json"]);
        assert!(
            load_now(source.clone(), Some("root.json"))
                .err()
                .unwrap()
                .contains("Invalid JSON")
        );
        assert!(load_now(source, Some("quiz/quiz.json")).is_err());
    }

    #[test]
    fn wrapper_requires_one_level_and_all_substantive_files_inside() {
        let layouts: &[&[(&str, &[u8])]] = &[
            &[],
            &[("outer/quiz/quiz.json", IMAGELESS_MANIFEST)],
            &[
                ("quiz/quiz.json", IMAGELESS_MANIFEST),
                ("other/quiz.json", IMAGELESS_MANIFEST),
            ],
            &[
                ("quiz/quiz.json", IMAGELESS_MANIFEST),
                ("readme.txt", b"outside"),
            ],
            &[
                ("quiz/quiz.json", IMAGELESS_MANIFEST),
                ("other/image.png", PNG),
            ],
            &[("quiz/__MACOSX/not-a-quiz.json", b"{bad")],
        ];
        for entries in layouts {
            assert_eq!(Archive::new(zip(entries)).err().unwrap(), ZIP_LAYOUT_ERROR);
        }
    }

    #[test]
    fn empty_directory_records_and_metadata_do_not_change_wrapper_detection() {
        let bytes = zip(&[
            ("quiz/quiz.json", IMAGELESS_MANIFEST),
            ("__MACOSX/._quiz", b"metadata"),
            ("quiz/__MACOSX/extra.json", b"{bad"),
            (".DS_Store", b"metadata"),
            ("quiz/.DS_Store", b"metadata"),
            ("other/.DS_Store", b"metadata"),
        ]);
        let mut writer = zip::ZipWriter::new_append(Cursor::new(bytes)).unwrap();
        for directory in ["quiz/", "other/", "quiz/empty/", "__MACOSX/"] {
            writer
                .add_directory(directory, SimpleFileOptions::default())
                .unwrap();
        }
        let source = prepare("wrapped.zip", writer.finish().unwrap().into_inner()).unwrap();
        let ImportSource::Archive(archive) = &source else {
            panic!()
        };
        assert_eq!(archive.candidates, ["quiz/quiz.json"]);
        assert!(load_now(source, Some("quiz/quiz.json")).is_ok());
    }

    #[test]
    fn metadata_image_reads_match_for_root_and_wrapped_packages() {
        for prefix in ["", "quiz/"] {
            for path in ["__MACOSX/image.png", "images/.DS_Store"] {
                let manifest = serde_json::to_vec(&serde_json::json!({
                    "questions": [{"id":"q", "question":"Describe", "type":"short_answer",
                        "images":[{"src":path, "alt":"Figure"}]}]
                }))
                .unwrap();
                let name = format!("{prefix}quiz.json");
                let image = format!("{prefix}{path}");
                let source =
                    prepare("quiz.zip", zip(&[(&name, &manifest), (&image, PNG)])).unwrap();
                assert!(
                    load_now(source, Some(&name))
                        .err()
                        .unwrap()
                        .contains("Metadata is not an image"),
                    "{image}"
                );
            }
        }
        // A deeper ordinary image directory named __MACOSX is not metadata.
        let manifest = br#"{"questions":[{"id":"q","question":"Describe","type":"short_answer","images":[{"src":"images/__MACOSX/image.png","alt":"Figure"}]}]}"#;
        let source = prepare(
            "wrapped.zip",
            zip(&[
                ("quiz/quiz.json", manifest),
                ("quiz/images/__MACOSX/image.png", PNG),
            ]),
        )
        .unwrap();
        assert!(load_now(source, Some("quiz/quiz.json")).is_ok());
    }

    #[test]
    fn wrapped_image_paths_are_exact_without_archive_root_fallback() {
        let manifest = br#"{"questions":[{"id":"q","question":"Describe","type":"short_answer","images":[{"src":"image.png","alt":"Figure"}]}]}"#;
        let source = prepare(
            "wrapped.zip",
            zip(&[("quiz/quiz.json", manifest), ("quiz/Image.png", PNG)]),
        )
        .unwrap();
        assert!(
            load_now(source, Some("quiz/quiz.json"))
                .err()
                .unwrap()
                .contains("quiz/image.png: file not found")
        );
        let bytes = zip(&[("quiz/quiz.json", manifest), ("image.png", PNG)]);
        assert_eq!(Archive::new(bytes).err().unwrap(), ZIP_LAYOUT_ERROR);
        let manifest = br#"{"questions":[{"id":"q","question":"Describe","type":"short_answer","images":[{"src":"../image.png","alt":"Figure"}]}]}"#;
        let source = prepare("wrapped.zip", zip(&[("quiz/quiz.json", manifest)])).unwrap();
        assert!(
            load_now(source, Some("quiz/quiz.json"))
                .err()
                .unwrap()
                .contains("Unsafe archive path")
        );
    }
    #[test]
    fn invalid_archives_paths_and_missing_or_corrupt_images_fail() {
        for file in [
            include_bytes!("../tests/fixtures/images/missing.zip").as_slice(),
            include_bytes!("../tests/fixtures/images/corrupt.zip").as_slice(),
        ] {
            let source = prepare("x.zip", file.to_vec()).unwrap();
            let ImportSource::Archive(archive) = &source else {
                panic!()
            };
            let manifest = archive.candidates[0].clone();
            assert!(load_now(source, Some(&manifest)).is_err());
        }
        assert!(
            prepare(
                "x.zip",
                include_bytes!("../tests/fixtures/images/nested.zip").to_vec()
            )
            .is_err()
        );
        for path in ["../evil", "/evil", "a//b", "a/./b", "a\\b"] {
            assert!(Archive::new(zip(&[("quiz.json", MANIFEST), (path, b"x")])).is_err());
        }
        assert!(prepare("x.txt", vec![]).is_err());
        assert!(Archive::new(b"garbage".to_vec()).is_err());
    }
    #[test]
    fn byte_entry_and_declared_limits_are_bounded() {
        assert!(prepare("x.json", vec![b' '; JSON_LIMIT + 1]).is_err());
        assert!(Archive::new(vec![0; ZIP_LIMIT + 1]).is_err());
        assert!(bounded_read(Cursor::new([0; 5]), 4).is_err());
        assert_eq!(bounded_read(Cursor::new([0; 4]), 4).unwrap().len(), 4);
        assert!(image_info(&vec![0; IMAGE_LIMIT + 1]).is_err());
        let bytes = zip(&[("quiz.json", &vec![b' '; JSON_LIMIT + 1])]);
        let archive = Archive::new(bytes).unwrap();
        assert!(archive.read("quiz.json", JSON_LIMIT).is_err());
        let mut bytes = zip(&[("quiz.json", MANIFEST)]);
        let end = bytes.len() - 22;
        bytes[end + 10..end + 12].copy_from_slice(&1001u16.to_le_bytes());
        // An impossible declared count is rejected by the directory parser.
        assert!(Archive::new(bytes).is_err());
        let mut bytes = zip(&[("quiz.json", MANIFEST)]);
        let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[central + 24..central + 28].copy_from_slice(&(TOTAL_LIMIT as u32 + 1).to_le_bytes());
        assert!(Archive::new(bytes).err().unwrap().contains("100 MiB"));
    }
    #[test]
    fn decoded_names_are_unique_and_used_for_exact_reads() {
        let source = include_bytes!("../tests/fixtures/images/duplicate-decoded.zip");
        assert!(
            Archive::new(source.to_vec())
                .err()
                .unwrap()
                .contains("duplicate decoded")
        );
        let source = include_bytes!("../tests/fixtures/images/cp437.zip");
        let archive = Archive::new(source.to_vec()).unwrap();
        assert_eq!(archive.candidates, ["é.json"]);
        let (quiz, assets) =
            load_now(ImportSource::Archive(Rc::new(archive)), Some("é.json")).unwrap();
        assert_eq!(quiz.title, "CP437 quiz");
        assert!(assets.urls.contains_key("é.png"));
    }

    #[test]
    fn directory_limit_cannot_be_hidden_by_a_fake_footer_in_the_comment() {
        let names: Vec<_> = (0..=ENTRY_LIMIT).map(|i| format!("{i}.json")).collect();
        let entries: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), b"{}".as_slice()))
            .collect();
        let mut bytes = zip(&entries);
        assert!(Archive::new(bytes.clone()).err().unwrap().contains("1,000"));
        let end = bytes.len() - 22;
        bytes[end + 20..end + 22].copy_from_slice(&22u16.to_le_bytes());
        let mut fake = [0u8; 22];
        fake[..4].copy_from_slice(b"PK\x05\x06");
        fake[8..10].copy_from_slice(&1u16.to_le_bytes());
        fake[10..12].copy_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&fake);
        assert!(Archive::new(bytes).is_err());
        // A harmless comment remains accepted.
        let mut bytes = zip(&[("quiz.json", MANIFEST)]);
        let end = bytes.len() - 22;
        bytes[end + 20..end + 22].copy_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(b"note");
        assert!(Archive::new(bytes).is_ok());
    }

    #[test]
    fn rejects_duplicate_encrypted_symlink_and_unsupported_compression() {
        let bytes = zip(&[("one.json", MANIFEST), ("two.json", MANIFEST)]);
        let duplicate = bytes
            .windows(8)
            .enumerate()
            .filter_map(|(i, b)| (b == b"two.json").then_some(i))
            .collect::<Vec<_>>();
        let mut bytes = bytes;
        for i in duplicate {
            bytes[i..i + 8].copy_from_slice(b"one.json");
        }
        assert!(Archive::new(bytes).err().unwrap().contains("duplicate"));
        for (offset, value, expected) in [(8, 1u16, "encryption"), (10, 12u16, "compression")] {
            let mut bytes = zip(&[("quiz.json", MANIFEST)]);
            let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
            bytes[central + offset..central + offset + 2].copy_from_slice(&value.to_le_bytes());
            assert!(Archive::new(bytes).err().unwrap().contains(expected));
        }
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_symlink("quiz.json", "target", SimpleFileOptions::default())
            .unwrap();
        assert!(Archive::new(writer.finish().unwrap().into_inner()).is_err());
    }
    #[test]
    fn supported_signatures_dimensions_and_corruption() {
        for bytes in [
            PNG,
            include_bytes!("../tests/fixtures/images/shapes.jpg"),
            include_bytes!("../tests/fixtures/images/shapes.gif"),
            include_bytes!("../tests/fixtures/images/shapes.webp"),
        ] {
            assert!(image_info(bytes).is_ok());
        }
        assert!(image_info(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_err());
        assert!(image_info(&PNG[..15]).is_err());
        // GIF logical-screen dimensions can be checked without allocating pixel storage.
        let mut gif = include_bytes!("../tests/fixtures/images/shapes.gif").to_vec();
        gif[6..10].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
        assert!(image_info(&gif).err().unwrap().contains("20 megapixels"));
    }
    #[test]
    fn rejects_gif_frame_extent_beyond_forged_logical_screen() {
        let gif = include_bytes!("../tests/fixtures/images/oversized-frame.gif");
        assert_eq!(&gif[6..10], &[1, 0, 1, 0]);
        assert!(image_info(gif).unwrap_err().contains("20 megapixels"));
        // Scan later frames too, including their offsets and local color tables.
        let normal = include_bytes!("../tests/fixtures/images/shapes.gif");
        let mut animation = normal[..normal.len() - 1].to_vec();
        animation.extend_from_slice(&[0x2c, 0xff, 0xff, 0xff, 0xff, 1, 0, 1, 0, 0]);
        animation.extend_from_slice(&[2, 0, 0x3b]);
        assert!(
            image_info(&animation)
                .unwrap_err()
                .contains("20 megapixels")
        );
    }

    #[test]
    fn png_checks_header_before_metadata_and_bounds_compressed_profiles() {
        let huge = include_bytes!("../tests/fixtures/images/oversized-header.png");
        // Must reject on pixels, before attempting any metadata decompression.
        assert!(image_info(huge).unwrap_err().contains("20 megapixels"));
        let metadata = include_bytes!("../tests/fixtures/images/oversized-metadata.png");
        assert!(metadata.len() < 20_000);
        assert!(
            image_info(metadata)
                .unwrap_err()
                .contains("metadata exceeds 8 MiB")
        );
        // The limit also applies to ancillary chunks after image data.
        let length = u32::from_be_bytes(metadata[33..37].try_into().unwrap()) as usize;
        let profile = &metadata[33..33 + length + 12];
        let mut after_data = PNG[..PNG.len() - 12].to_vec();
        after_data.extend_from_slice(profile);
        after_data.extend_from_slice(&PNG[PNG.len() - 12..]);
        assert!(
            image_info(&after_data)
                .unwrap_err()
                .contains("metadata exceeds 8 MiB")
        );
    }

    #[test]
    fn dishonest_declared_sizes_do_not_bypass_actual_manifest_or_image_caps() {
        for (name, cap) in [("quiz.json", JSON_LIMIT), ("image.png", IMAGE_LIMIT)] {
            let mut bytes = if name == "quiz.json" {
                zip(&[(name, &vec![b' '; cap + 1])])
            } else {
                zip(&[("quiz.json", MANIFEST), (name, &vec![b' '; cap + 1])])
            };
            let positions = bytes
                .windows(4)
                .enumerate()
                .filter_map(|(i, b)| (b == b"PK\x01\x02").then_some(i))
                .collect::<Vec<_>>();
            for pos in positions {
                bytes[pos + 24..pos + 28].copy_from_slice(&1u32.to_le_bytes());
            }
            let archive = Archive::new(bytes).unwrap();
            assert!(archive.read(name, cap).is_err());
        }
    }

    #[test]
    fn images_do_not_change_seeded_question_or_option_order() {
        let mut quiz: QuizFile = serde_json::from_slice(MANIFEST).unwrap();
        let with_images = crate::model::build_seeded_session(&quiz.questions, 2, 42, 1.0);
        for q in &mut quiz.questions {
            q.images.clear();
        }
        let without_images = crate::model::build_seeded_session(&quiz.questions, 2, 42, 1.0);
        for (mut original, plain) in with_images.into_iter().zip(without_images) {
            original.images.clear();
            assert_eq!(original, plain);
        }
    }
    #[test]
    fn actual_aggregate_limit_rejects_a_small_archive_with_dishonest_sizes() {
        let images: Vec<_> = (0..11)
            .map(|i| serde_json::json!({"src": format!("{i}.png"), "alt": "Shapes"}))
            .collect();
        let manifest = serde_json::to_vec(&serde_json::json!({"questions": [{"id":"q", "type":"short_answer", "question":"Describe", "images":images}]})).unwrap();
        let mut padded = PNG.to_vec();
        padded.resize(IMAGE_LIMIT, 0);
        let names: Vec<_> = (0..11).map(|i| format!("{i}.png")).collect();
        let mut entries = vec![("quiz.json", manifest.as_slice())];
        entries.extend(names.iter().map(|name| (name.as_str(), padded.as_slice())));
        let mut bytes = zip(&entries);
        let positions: Vec<_> = bytes
            .windows(4)
            .enumerate()
            .filter_map(|(i, b)| (b == b"PK\x01\x02").then_some(i))
            .collect();
        for pos in positions.into_iter().skip(1) {
            bytes[pos + 24..pos + 28].copy_from_slice(&1u32.to_le_bytes());
        }
        let source = prepare("x.zip", bytes).unwrap();
        let error = load_now(source, Some("quiz.json")).err().unwrap();
        // zip rejects the first lying entry even before the aggregate guard.
        assert!(
            error.contains("larger than its declared uncompressed size"),
            "{error}"
        );
    }
}
