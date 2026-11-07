# Image feature implementation and verification

Implemented locally on `quiz-image-import`, 2026-11-07. Independent review is pending.

## Integration

- Starting image-branch commit: `f083adb97c5b87607e2b98c4235d864a7c2c7f22`.
- Preserved the previously uncommitted agreed plan edits in `a7c39f7f5178456aebdb7c051cef9a151a11d155`.
- Fetched remote main (`0cc876e49f014099d599e9fabf05ec4fdd34e1b2`) and the short-answer branch. PR #5 was open at `bb7526872234815d61cb442a4d5e91a8686e385d` when checked.
- Integrated that PR head into this image branch with local merge `5d0703617c36aa0c644106c1e33cad8c509c431c`. This is the implementation diff base.
- No other checkout or PR was modified. Nothing was pushed or merged on GitHub.

## Delivered behavior

Both question kinds share ordered prompt images, alternative text, optional plain-text captions, keyboard enlargement with native dialog focus handling, remote failure messages and retry. Quiz, results and study-review prompts use the same component. Images do not consume RNG draws or change scoring, answers or hints.

The shared picker/drop importer accepts JSON or ZIP bytes. ZIP manifests can have any root filename: one loads automatically; multiple require explicit sorted selection with no preferred `quiz.json`. Invalid selections remain recoverable. Root images, nested image folders, spaces, Unicode and mixed HTTPS/local references work. Unselected JSON and unrelated entries are not decompressed.

Archive and image limits follow the agreed plan. Actual reads are capped in addition to declared sizes. Signatures and dimensions are checked before sequential browser decoding. Remote references are HTTPS without credentials; Blob/data references are not accepted as input. The ZIP parser also rejects entries larger than their declared size, verified with forged metadata.

App-owned assets deduplicate local references. Settings, results, reruns and canceled replacement preserve active assets. Successful replacement, explicit discard and app teardown release assets. Pending task cancellation releases provisional assets immediately, including when a decoder never settles; generation checks additionally guard stale completions. The picker resets after capturing a file so selecting that same file retries reliably in Chromium.

Version 4 copy/download exports retain original image references and exact ZIP manifest provenance, without Blob URLs or embedded bytes. Existing short-answer response, marks, scoring and hint-history fields remain intact. The schema, upload reference, README, URL authoring example, local fixtures and portable sample ZIP are included.

## Dependency compatibility

Inspected the existing manifest/lockfile, compiler and published crate metadata before choosing dependencies. No existing dependency version was upgraded.

- Compiler: `rustc 1.97.1 (8bab26f4f 2026-07-14)`; repository edition 2024, no declared MSRV.
- `zip = 9.4.1`: Rust 1.88 minimum; default features disabled, only `deflate-flate2-zlib-rs` enabled. Stored and pure-Rust Deflate decoding build for the browser target.
- `image = 0.25.10`: Rust 1.88 minimum; default features disabled, PNG/JPEG/GIF/WebP only. Browser imports use it for signature/dimension inspection, then use browser decoding.
- Dioxus crates remain locked at 0.7.9. Installed `dx` is 0.7.10 and emits a version-mismatch diagnostic, but the asset-aware WASM build exits successfully and the browser regressions pass.

## Final checks

| Check | Result |
|---|---|
| `cargo test --locked` | 52 passed, 0 failed; final run 1.31 s |
| `cargo fmt --check` | Passed |
| `git diff --check` | Passed |
| `dx build --web` | Passed; WASM client and assets generated, exit 0 |
| `QUIZ_TEST_PORT=8093 npx playwright test --project=chromium --project=firefox --reporter=line` | 102 passed, 0 failed, 32.8 s; 51 per browser |
| Python `jsonschema.validate` against the repository schema | Passed for existing sample, bundled-image manifest and URL example |
| Isolated WebKit chooser test | Could not launch: missing host libraries `libicu74`, `libxml2`, `libflite1` |

The 15 new browser tests run in each available browser. They cover picker/drop parity, both question kinds, root and nested image paths, repeated references, captions and keyboard focus, multiple manifests and invalid-selection recovery, failure/cancellation preservation, immediate decoder cancellation, stale selections, same-file retry, settings and both reruns, explicit discard, no-referrer HTTPS failure/retry, narrow layout, actual PNG/JPEG/GIF/WebP fixtures, mixed local/remote ZIPs, original references and copy/download equality. Browser decoder rejection is injected against a real local fixture to exercise cleanup independently of browsers' tolerance for damaged image data. Corrupt signatures are tested separately.

Native regressions cover the shared conversion and exports, source/path validation, malformed image fields, arbitrary root filenames, missing/corrupt images, unsupported archive features, duplicate names, limits and forged sizes, supported signatures/dimensions, and unchanged seeded order. All pre-existing native and available-browser regressions also pass.

Earlier runs that reused an unrelated server on port 8081 or overlapped Dioxus builds were discarded as invalid evidence. The final browser run used a dedicated port with no concurrent build. `QUIZ_TEST_PORT` now disables existing-server reuse.

## Remaining verification limits

WebKit still needs its host dependencies before its tests can run. No physical phone performance measurement or representative teaching dataset was available: the documented limits are the agreed initial limits, exercised with local synthetic fixtures. Archive decompression remains bounded synchronous WASM work; image decoding is sequential and asynchronous. Large real quizzes and animated-image memory behavior deserve measurement on target phones before release. Remote image availability remains dependent on the host; test HTTPS requests are fulfilled locally rather than contacting external image services.
