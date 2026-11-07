# Image feature implementation and verification

Implemented locally on `quiz-image-import`, 2026-11-07. Independent review completed; all three reported defects have been fixed and regression-tested locally.

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

## Independent review fixes

The reviewer reproduced three defects against `cd2e9e9`: GIF frame dimensions could exceed a forged small logical screen, PNG decoder construction expanded compressed ICC metadata before the pixel check, and a recovered remote inline image left its enlargement broken.

- GIF preflight now scans all frame descriptors and offsets, global/local color tables, and extension/data blocks without LZW decoding. Both individual frame dimensions and the maximum canvas extents must fit the 20-megapixel limit.
- PNG preflight reads IHDR directly before any decoder construction. It scans compressed ICC/text metadata throughout the file (including after IDAT), streaming decompressed bytes to a sink with an aggregate 8-MiB cap. This explicit validation is necessary because the PNG dependency ignores ICC parsing failures. Decoder construction additionally has an explicit 8-MiB allocation limit. `flate2` 1.1.11 was already locked transitively; it is now a direct dependency with the existing pure-Rust `zlib-rs` backend. No resolved package versions changed. Its declared Rust minimum is 1.67.
- Failed images now unmount the enlargement dialog as well as the inline image; retry recreates both. The browser regression opens the dialog after a 404-to-success retry and checks its decoded natural width.

New deterministic fixtures reproduce a 5000×5000 GIF frame behind a 1×1 logical screen, oversized PNG IHDR with compressed metadata, and ordinary-sized PNG with a 9-MiB expanded ICC profile. Generate them with `tests/fixtures/images/generate-regressions.py`. Native tests also check later GIF frames/offsets and PNG metadata after IDAT. Browser tests require rejection before any image Blob is created, then verify a valid upload still works.

Final validation after fixes:

- `cargo test --locked`: **54 passed**, 0 failed.
- `cargo fmt --check` and `git diff --check`: passed.
- `QUIZ_TEST_PORT=8097 npx playwright test --project=chromium --project=firefox --reporter=line`: **108 passed**, 0 failed, 35.4 s, using a freshly built WASM app through the configured `dx serve` command.
- An initial attempted dialog-key fix failed the new retry regression in both browsers; it was replaced by conditional unmount/remount before the successful final run.

The original WebKit/system-library and physical-device/dataset verification limitations remain. No second independent review was run after these focused fixes. Nothing has been pushed.

## Full-branch Astra review after rebase

Review of `49eb10e…c069169` found two further ZIP issues: raw filename duplicate detection missed decoded-name collisions, and the custom footer preflight could disagree with the directory selected by the ZIP parser.

The importer now uses the exact-pinned `zip` crate's incremental `ZipIterable` API, checks its directory count before collecting entries, and retains those same parsed entries for reads. It indexes entries by decoded name and rejects collisions; reads use the stored entry rather than a second filename lookup. The API is under `zip::unstable::read`, so the existing exact `=9.4.1` pin is required and upgrades must verify this contract.

Added native regressions for fake footers in ZIP comments, decoded duplicate names, and successful CP437 manifest/image lookup, plus browser coverage of collision rejection and CP437 render/export. The reviewer's allocation harness now measures 60 allocated bytes for rejecting the normal 60,000-entry archive and 82 bytes for rejecting its fake-footer variant, versus the reported 16,746,636 bytes before the fix. The reviewer's original duplicate-name archive is also rejected. These numbers measure `prepare()` allocations, excluding the already-read input archive.

After the rebase and these fixes, native tests pass (60), schema tests pass (2), and formatting/diff checks pass. `QUIZ_TEST_PORT=8099 npx playwright test --project=chromium --project=firefox --reporter=line` passed all 114 tests in 36.2 seconds against the freshly built WASM app. Sol's additional independent review remains in progress; CodeRabbit is excluded by user instruction.

## Sol 6.1 full-branch review

Sol independently reviewed the same pinned `c069169` implementation without CodeRabbit or other external review services. It confirmed the footer issue above and found one additional compatibility regression: the byte-based importer rejected a leading UTF-8 BOM that the previous browser `File.text()` path removed.

The shared JSON parsing path now strips exactly one leading UTF-8 BOM for standalone files and selected ZIP manifests. Size checks and aggregate accounting still use the original byte length. Native tests compare BOM and ordinary JSON semantics, check selected manifest provenance, and retain rejection for duplicate BOMs and malformed JSON. Browser tests upload both `bom.json` and `bom.zip` and verify completion and exports.

All reported findings from the Astra and Sol full-branch reviews have been addressed. Neither reviewer re-reviewed the subsequent fixes; executable regression evidence is provided here. CodeRabbit was not used for Sol's review, and no CodeRabbit process remained running when the parent's process check was performed.

Final validation including the BOM fix: `cargo test --locked` passed all **61** tests; formatting and diff checks passed. `QUIZ_TEST_PORT=8099 npx playwright test --project=chromium --project=firefox --reporter=line` passed all **118** tests in 36.5 seconds using a freshly built WASM app. The earlier WebKit and physical-device/dataset limitations remain.
