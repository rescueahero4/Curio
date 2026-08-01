//! Screenshot downscaling (R-BE-26).
//!
//! Two products from one capture, with different jobs:
//!
//! * a **thumbnail** for the grid — 640 px wide, cropped to the first fold, because a card
//!   showing a 20,000-pixel stitch shrunk to fit is a grey smear that identifies nothing;
//! * a **vision payload** — long edge ≤ 1568 px, tall captures cropped to 4× their width,
//!   because a full-height stitch is mostly footer and costs image tokens per screenful of
//!   it.
//!
//! ## Failure degrades, never fails
//!
//! Every path here falls back to the original bytes (R-BE-26). A capture whose thumbnail
//! could not be produced is still a capture: it shows the full screenshot in the grid and
//! sends the full screenshot to the model. Failing the ingest over an encoder would throw
//! away the one thing that is genuinely irreplaceable — the screenshot — to protect a
//! derived file that is not.
//!
//! ## D32: JPEG rather than WebP
//!
//! ARCH-01 R-BE-26 names WebP q82/q88, inherited from the previous implementation's
//! `sharp`. The Rust `image` stack encodes WebP **losslessly only** — there is no quality
//! knob without linking libwebp, and lossless WebP of a photographic screenshot is larger
//! than the JPEG it replaces, which inverts the point of the rule. JPEG at the same
//! quality numbers is smaller, is accepted by the vision API, and needs no C dependency.
//! The dimension caps, crop rules, and degrade-to-original behaviour — the parts that
//! change what the model sees and what the user sees — are unchanged.

use image::{DynamicImage, ImageFormat, imageops::FilterType};

/// Grid thumbnail width (R-BE-26).
pub const THUMBNAIL_WIDTH: u32 = 640;

/// Longest edge the vision payload may have (R-BE-26).
pub const VISION_MAX_EDGE: u32 = 1568;

/// Thumbnail quality.
pub const THUMBNAIL_QUALITY: u8 = 82;

/// Vision payload quality. Higher, because this is the copy the model reads type from.
pub const VISION_QUALITY: u8 = 88;

/// The item page's copy: wide enough to be sharp at 1:1 on a retina display at the column
/// width that page uses, and **uncropped**.
pub const DETAIL_MAX_WIDTH: u32 = 1600;

/// Detail quality. The highest of the three: this is the image a person studies, and the
/// whole point of the library is that they can.
pub const DETAIL_QUALITY: u8 = 90;

/// A tall capture is cropped to this multiple of its width before scaling.
///
/// Past roughly four screenfuls the extra height is footer and repeated sections; keeping
/// it costs tokens per screenful and pushes the useful part into fewer pixels.
pub const TALL_CROP_RATIO: u32 = 4;

/// Viewport aspect fallback when the capture did not report one (R-BE-26).
pub const FALLBACK_ASPECT: f64 = 16.0 / 10.0;

/// The permitted aspect range. A capture reporting something outside it is reporting
/// nonsense, and cropping to nonsense produces a sliver.
pub const MIN_ASPECT: f64 = 0.5;
pub const MAX_ASPECT: f64 = 4.0;

/// Encoded image bytes and what they are.
#[derive(Debug, Clone)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub media_type: &'static str,
    /// Whether processing succeeded. `false` means these are the original bytes, passed
    /// through under R-BE-26's degrade rule.
    pub processed: bool,
}

impl Encoded {
    /// The original, untouched.
    fn passthrough(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            media_type: "image/png",
            processed: false,
        }
    }
}

/// Build the grid thumbnail: crop to the first fold, then scale to 640 px wide.
///
/// `viewport_aspect` is the capturing browser's width ÷ height. Out-of-range and absent
/// values both fall back to 16:10 — cropping to a reported aspect of 0.01 would produce a
/// one-pixel strip, which is worse than cropping to a guess.
#[must_use]
pub fn thumbnail(original: &[u8], viewport_aspect: Option<f64>) -> Encoded {
    let Some(image) = decode(original) else {
        return Encoded::passthrough(original.to_vec());
    };

    let aspect = viewport_aspect
        .filter(|value| value.is_finite() && (MIN_ASPECT..=MAX_ASPECT).contains(value))
        .unwrap_or(FALLBACK_ASPECT);

    let width = image.width();
    // The first fold is one viewport tall at this width. `max(1)` because a 1-pixel-wide
    // image would otherwise crop to zero height and panic the resize.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "dimensions are bounded by the source image"
    )]
    let fold_height = ((f64::from(width) / aspect).round() as u32).max(1);
    let cropped = crop_top(&image, fold_height.min(image.height()));

    let scaled = if cropped.width() > THUMBNAIL_WIDTH {
        let height = scale_height(cropped.width(), cropped.height(), THUMBNAIL_WIDTH);
        cropped.resize_exact(THUMBNAIL_WIDTH, height, FilterType::Lanczos3)
    } else {
        cropped
    };

    encode(&scaled, THUMBNAIL_QUALITY).unwrap_or_else(|| Encoded::passthrough(original.to_vec()))
}

/// Build the item page's copy: the **whole** capture, scaled to fit [`DETAIL_MAX_WIDTH`].
///
/// This is the derivative the thumbnail is not. The grid needs a fold crop — a card showing
/// a 20,000-pixel stitch is a grey smear — but the item page has one image and that image is
/// the subject, so nothing here is cropped. Serving the thumbnail there instead was the bug
/// this exists to fix: a full-page capture arrived as a 640 px fold crop, which is both the
/// wrong part of the page and too few pixels to study.
///
/// A capture already narrower than the cap is passed through untouched. Re-encoding a small
/// PNG as JPEG would spend quality to make the file *bigger*, and the file route's fallback
/// serves the original in its place.
#[must_use]
pub fn detail(original: &[u8]) -> Encoded {
    let Some(image) = decode(original) else {
        return Encoded::passthrough(original.to_vec());
    };

    if image.width() <= DETAIL_MAX_WIDTH {
        return Encoded::passthrough(original.to_vec());
    }

    let height = scale_height(image.width(), image.height(), DETAIL_MAX_WIDTH);
    // `CatmullRom`, not `Lanczos3` as the other two use. They work on small images — a
    // cropped fold, a 1568 px payload — where the filter's cost is irrelevant. This one
    // rescales the *whole* stitch, tens of megapixels, where the cheaper filter is worth
    // having and buys away nothing visible at a downscale ratio under 2×.
    //
    // It is not the reason this runs in the background, though: decoding and re-encoding an
    // image this size costs seconds on its own, whatever the filter. See the caller.
    let scaled = image.resize_exact(DETAIL_MAX_WIDTH, height, FilterType::CatmullRom);

    encode(&scaled, DETAIL_QUALITY).unwrap_or_else(|| Encoded::passthrough(original.to_vec()))
}

/// Build the copy the vision model sees: crop very tall captures, then fit within 1568 px.
#[must_use]
pub fn vision_payload(original: &[u8]) -> Encoded {
    let Some(image) = decode(original) else {
        return Encoded::passthrough(original.to_vec());
    };

    let tall_limit = image.width().saturating_mul(TALL_CROP_RATIO).max(1);
    let cropped = if image.height() > tall_limit {
        crop_top(&image, tall_limit)
    } else {
        image
    };

    let longest = cropped.width().max(cropped.height());
    let scaled = if longest > VISION_MAX_EDGE {
        // `resize` preserves aspect and fits inside the box, which is exactly the "long
        // edge ≤ 1568" rule.
        cropped.resize(VISION_MAX_EDGE, VISION_MAX_EDGE, FilterType::Lanczos3)
    } else {
        cropped
    };

    encode(&scaled, VISION_QUALITY).unwrap_or_else(|| Encoded::passthrough(original.to_vec()))
}

fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    match image::load_from_memory(bytes) {
        Ok(image) if image.width() > 0 && image.height() > 0 => Some(image),
        Ok(_) => None,
        Err(err) => {
            tracing::debug!(%err, "the screenshot could not be decoded; using it as-is");
            None
        }
    }
}

fn crop_top(image: &DynamicImage, height: u32) -> DynamicImage {
    image.crop_imm(0, 0, image.width(), height.min(image.height()).max(1))
}

fn scale_height(width: u32, height: u32, target_width: u32) -> u32 {
    let ratio = f64::from(height) / f64::from(width);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded by the target width"
    )]
    let scaled = (f64::from(target_width) * ratio).round() as u32;
    scaled.max(1)
}

fn encode(image: &DynamicImage, quality: u8) -> Option<Encoded> {
    // JPEG has no alpha channel; an RGBA source would otherwise encode with its
    // transparency interpreted as colour. Screenshots are opaque, so this is a no-op in
    // the normal case and a correctness fix in the odd one.
    let opaque = DynamicImage::ImageRgb8(image.to_rgb8());

    let mut bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        std::io::Cursor::new(&mut bytes),
        quality,
    );

    match encoder.encode_image(&opaque) {
        Ok(()) => Some(Encoded {
            bytes,
            media_type: "image/jpeg",
            processed: true,
        }),
        Err(err) => {
            tracing::debug!(%err, "the image could not be encoded; using the original");
            None
        }
    }
}

/// What media type the API should be told for bytes we did not re-encode.
///
/// Guessed from the bytes rather than the file extension: the extension is ours, the
/// content is the browser's.
#[must_use]
pub fn sniff_media_type(bytes: &[u8]) -> &'static str {
    match image::guess_format(bytes) {
        Ok(ImageFormat::Jpeg) => "image/jpeg",
        Ok(ImageFormat::WebP) => "image/webp",
        // PNG is what the extension captures, and the right answer when nothing else
        // matched — the API rejects a media type it does not know, so guessing the
        // common case beats sending something invented.
        _ => "image/png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut buffer = RgbImage::new(width, height);
        // Not a flat fill: a solid colour compresses to almost nothing and would make the
        // size assertions meaningless.
        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            *pixel = Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]);
        }
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(buffer)
            .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
            .expect("encode");
        out
    }

    fn dimensions(encoded: &Encoded) -> (u32, u32) {
        let image = image::load_from_memory(&encoded.bytes).expect("decode");
        (image.width(), image.height())
    }

    #[test]
    fn a_thumbnail_is_cropped_to_the_first_fold_at_the_reported_aspect() {
        // A 1200×9000 stitch captured in a 1200×600 viewport: the card should show the
        // top screenful, not a smear of the whole page.
        let thumb = thumbnail(&png(1200, 9000), Some(2.0));
        let (width, height) = dimensions(&thumb);

        assert_eq!(width, THUMBNAIL_WIDTH);
        assert_eq!(height, THUMBNAIL_WIDTH / 2, "one viewport tall, scaled");
        assert!(thumb.processed);
    }

    #[test]
    fn a_detail_copy_keeps_the_whole_capture() {
        // The bug in one assertion: the item page must not receive a fold crop. A
        // 1200×9000 stitch stays 7.5 screenfuls tall, scaled but never cut.
        let full = detail(&png(2400, 18_000));
        let (width, height) = dimensions(&full);

        assert_eq!(width, DETAIL_MAX_WIDTH);
        assert_eq!(height, 12_000, "aspect preserved, nothing cropped");
        assert!(full.processed);
    }

    #[test]
    fn a_capture_narrower_than_the_cap_is_passed_through() {
        // Re-encoding it would spend quality to produce a larger file, and the file route
        // serves the original screenshot in its place.
        let small = detail(&png(1200, 800));

        assert!(!small.processed);
        assert_eq!(small.media_type, "image/png");
    }

    #[test]
    fn an_absent_aspect_falls_back_to_sixteen_by_ten() {
        let thumb = thumbnail(&png(1600, 9000), None);
        let (width, height) = dimensions(&thumb);

        assert_eq!(width, THUMBNAIL_WIDTH);
        assert_eq!(height, 400, "640 ÷ 1.6");
    }

    #[test]
    fn a_nonsense_aspect_is_clamped_rather_than_obeyed() {
        // A reported aspect of 0.01 would crop a 1200 px capture to a 12-pixel strip. The
        // fallback is a guess; the strip is a bug the user sees in their grid.
        let absurd = thumbnail(&png(1600, 9000), Some(0.01));
        let fallback = thumbnail(&png(1600, 9000), None);

        assert_eq!(dimensions(&absurd), dimensions(&fallback));
    }

    #[test]
    fn a_short_capture_is_not_padded_to_a_full_fold() {
        // A 1200×300 capture in a 1200×600 viewport is 300 px tall, not 600.
        let thumb = thumbnail(&png(1200, 300), Some(2.0));
        let (_, height) = dimensions(&thumb);

        assert_eq!(
            height, 160,
            "the source's own 4:1 ratio, scaled to 640 wide"
        );
    }

    #[test]
    fn a_capture_narrower_than_the_thumbnail_is_not_upscaled() {
        // Upscaling adds bytes and no detail.
        let thumb = thumbnail(&png(320, 200), Some(1.6));
        assert_eq!(dimensions(&thumb).0, 320);
    }

    #[test]
    fn the_vision_payload_fits_inside_the_long_edge_cap() {
        let payload = vision_payload(&png(4000, 2000));
        let (width, height) = dimensions(&payload);

        assert!(width <= VISION_MAX_EDGE && height <= VISION_MAX_EDGE);
        assert_eq!(
            width, VISION_MAX_EDGE,
            "the long edge is the one that binds"
        );
    }

    #[test]
    fn a_tall_stitch_is_cropped_to_four_times_its_width_before_scaling() {
        // R-BE-26. Without the crop, a 1000×20000 stitch scales to 78×1568 — a sliver in
        // which nothing is legible, at full image-token price.
        let payload = vision_payload(&png(1000, 20000));
        let (width, height) = dimensions(&payload);

        let ratio = f64::from(height) / f64::from(width);
        assert!(
            (ratio - f64::from(TALL_CROP_RATIO)).abs() < 0.05,
            "expected a 4:1 crop, got {width}×{height}"
        );
        assert!(height <= VISION_MAX_EDGE);
    }

    #[test]
    fn a_small_capture_passes_through_the_vision_path_unscaled() {
        let payload = vision_payload(&png(800, 600));
        assert_eq!(dimensions(&payload), (800, 600));
    }

    #[test]
    fn undecodable_bytes_degrade_to_the_original_rather_than_failing() {
        // R-BE-26's degrade rule. The screenshot is the irreplaceable artefact; a
        // thumbnail is not worth failing an ingest over.
        let garbage = b"this is not an image".to_vec();

        let thumb = thumbnail(&garbage, Some(1.6));
        assert!(!thumb.processed);
        assert_eq!(thumb.bytes, garbage);

        let payload = vision_payload(&garbage);
        assert!(!payload.processed);
        assert_eq!(payload.bytes, garbage);
    }

    #[test]
    fn processing_actually_makes_the_payload_smaller() {
        // The whole point. If the "downscaled" copy were larger than the original, every
        // assessment would cost more than sending the screenshot untouched.
        let original = png(4000, 3000);
        let payload = vision_payload(&original);

        assert!(payload.processed);
        assert!(
            payload.bytes.len() < original.len(),
            "{} bytes from a {} byte original",
            payload.bytes.len(),
            original.len()
        );
    }

    #[test]
    fn the_media_type_matches_what_was_produced() {
        assert_eq!(vision_payload(&png(100, 100)).media_type, "image/jpeg");
        assert_eq!(sniff_media_type(&png(10, 10)), "image/png");
        assert_eq!(sniff_media_type(b"not an image"), "image/png");
    }
}
