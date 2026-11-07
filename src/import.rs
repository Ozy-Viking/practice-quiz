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

/// True for remote HTTPS, false for an exact archive-relative path.
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

#[derive(Clone)]
pub enum ImportSource {
    Json(Vec<u8>),
    Archive(Rc<Archive>),
}

pub struct Archive {
    zip: zip::ZipArchive<Cursor<Rc<[u8]>>>,
    pub candidates: Vec<String>,
}

// Cap directory allocation before handing untrusted entry counts to the ZIP parser.
fn check_entry_count(bytes: &[u8]) -> Result<(), String> {
    let end = (bytes.len().saturating_sub(65_557)..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            bytes.get(i..i + 4) == Some(b"PK\x05\x06")
                && i + 22 + u16::from_le_bytes([bytes[i + 20], bytes[i + 21]]) as usize
                    == bytes.len()
        })
        .ok_or("Invalid ZIP end record")?;
    let count = u16::from_le_bytes([bytes[end + 10], bytes[end + 11]]) as u64;
    let count = if end >= 20 && bytes.get(end - 20..end - 16) == Some(b"PK\x06\x07") {
        let offset = u64::from_le_bytes(bytes[end - 12..end - 4].try_into().unwrap());
        let offset = usize::try_from(offset).map_err(|_| "Invalid ZIP64 directory")?;
        let record = bytes
            .get(offset..offset.saturating_add(56))
            .ok_or("Invalid ZIP64 directory")?;
        if &record[..4] != b"PK\x06\x06" {
            return Err("Invalid ZIP64 directory".into());
        }
        u64::from_le_bytes(record[32..40].try_into().unwrap())
    } else {
        count
    };
    if count > ENTRY_LIMIT as u64 {
        return Err("ZIP exceeds 1,000 entries".into());
    }
    Ok(())
}

impl Archive {
    pub fn new(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > ZIP_LIMIT {
            return Err("ZIP exceeds 50 MiB".into());
        }
        check_entry_count(&bytes)?;
        let zip = zip::ZipArchive::new(Cursor::new(Rc::<[u8]>::from(bytes)))
            .map_err(|e| format!("Invalid ZIP: {e}"))?;
        if zip.len() > ENTRY_LIMIT {
            return Err("ZIP exceeds 1,000 entries".into());
        }
        if zip.has_duplicate_names() {
            return Err("ZIP contains duplicate names".into());
        }
        let mut total = 0u64;
        let mut candidates = Vec::new();
        for i in 0..zip.len() {
            let entry = zip.by_index_data(i).map_err(|e| e.to_string())?;
            let name = entry.name().map_err(|e| e.to_string())?;
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
            if !entry.is_dir()
                && !ignored(&name)
                && !name.contains('/')
                && name.to_ascii_lowercase().ends_with(".json")
            {
                candidates.push(name.into_owned());
            }
        }
        candidates.sort();
        if candidates.is_empty() {
            return Err("Place at least one quiz JSON file at the ZIP root.".into());
        }
        Ok(Self { zip, candidates })
    }

    fn read(&self, name: &str, limit: usize) -> Result<Vec<u8>, String> {
        if ignored(name) {
            return Err(format!("Metadata is not an image: {name}"));
        }
        let mut zip = self.zip.clone();
        let entry = zip
            .by_name(name)
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
    let (width, height) = image::ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| format!("Invalid image header: {e}"))?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > PIXEL_LIMIT {
        return Err("Image exceeds 20 megapixels or has empty dimensions".into());
    }
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
                return Err("Choose a root JSON file".into());
            }
            archive.read(name, JSON_LIMIT)?
        }
    };
    let mut quiz: QuizFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid JSON: {e}"))?;
    validate_quiz(&quiz)?;
    quiz.source_manifest = manifest;
    let mut assets = AssetStore::default();
    let mut actual_total = bytes.len() as u64;
    for question in &quiz.questions {
        for image in &question.images {
            if !current() {
                return Err("Import superseded".into());
            }
            if image_source(&image.src)? || assets.urls.contains_key(&image.src) {
                continue;
            }
            let ImportSource::Archive(archive) = &source else {
                return Err(format!(
                    "Question {} references {}. Upload a ZIP containing the quiz JSON at its root and that image at the referenced path, or use an HTTPS image URL.",
                    question.id, image.src
                ));
            };
            let bytes = archive.read(&image.src, IMAGE_LIMIT)?;
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
        assert!(Archive::new(bytes).err().unwrap().contains("1,000"));
        let mut bytes = zip(&[("quiz.json", MANIFEST)]);
        let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[central + 24..central + 28].copy_from_slice(&(TOTAL_LIMIT as u32 + 1).to_le_bytes());
        assert!(Archive::new(bytes).err().unwrap().contains("100 MiB"));
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
