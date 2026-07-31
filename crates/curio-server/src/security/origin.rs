//! Who is asking: Host and Origin validation.
//!
//! Curio listens only on loopback, but a web page in the user's browser also runs on the
//! user's machine — so "local" is not "trusted" (ARCH-06 §Threat model). Two checks run
//! before anything else, and both are about *identity*, not credentials:
//!
//! * **Host** (R-SEC-6) defeats DNS rebinding. An attacker points `evil.example` at
//!   127.0.0.1; the browser dutifully connects to our server, but it still sends
//!   `Host: evil.example`, and that is the tell.
//! * **Origin** (R-SEC-7) defeats a page you are simply visiting from calling loopback
//!   directly.
//!
//! The subtle rule is the third allowed origin: a loopback origin is accepted only when
//! its authority **equals the request's own Host** — never a configured or remembered
//! port (Inventory §10.2). Comparing against a port we know about rather than the one we
//! were actually reached on is how a rebinding check gets quietly defeated.
//!
//! These functions are pure so they can be tested exhaustively; wiring them into the
//! middleware stack is P1's job.

/// The extension's origin, derived from the pinned manifest `key`.
///
/// **One fact in two files** (R-EXT-2, Inventory §10.1). The extension's manifest pins a
/// public key, Chrome derives the extension id from it, and the id becomes this origin.
/// Change one without the other and capture breaks — which is why the derivation is
/// asserted by a test below rather than trusted as a comment.
/// Re-exported rather than declared (R-OV-2). The same value pins `allowed_origins` in
/// the native-messaging manifest, and two copies of a security boundary is one copy too
/// many — the drift would be silent and the symptom would be a hostile extension the CORS
/// check allows or a legitimate one the host refuses.
pub use curio_core::nm::EXTENSION_ORIGIN;

/// Whether a `Host` header names loopback.
///
/// The port is irrelevant — we are asking which *name* the client used to reach us, and
/// only a loopback name is legitimate. A rebound domain fails here even though it
/// resolved to our address.
#[must_use]
pub fn host_is_loopback(host: &str) -> bool {
    let Some(hostname) = hostname_of(host) else {
        return false;
    };
    matches!(hostname.as_str(), "localhost" | "127.0.0.1" | "::1")
}

/// Whether an `Origin` may call a token-bearing route, given the `Host` it used.
///
/// Three origins are allowed and no others (R-SEC-7):
///
/// * **absent or `null`** — a non-CORS caller: curl, an MCP client, a same-origin fetch.
///   Browsers attach an Origin to cross-origin requests, so its absence is not something
///   a hostile page can arrange.
/// * **the pinned extension origin** — exactly one extension id, not any extension.
/// * **a loopback origin whose authority equals this request's Host** — same origin,
///   established by comparison against what the client actually connected to.
#[must_use]
pub fn origin_is_allowed(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    if origin.is_empty() || origin == "null" {
        return true;
    }
    if origin == EXTENSION_ORIGIN {
        return true;
    }

    let Some(authority) = origin.strip_prefix("http://") else {
        // https:// on loopback is not something Curio serves, and every other scheme is
        // someone else's page.
        return false;
    };
    let Some(host) = host else {
        return false;
    };

    // The Host-echo rule. Comparing against a port we remember instead of the one we were
    // reached on is precisely the shortcut that reopens the rebinding hole.
    host_is_loopback(authority) && authority.eq_ignore_ascii_case(host)
}

/// Whether `Sec-Fetch-Site` should cause a rejection.
///
/// Defence in depth only (R-SEC-12). A missing header **must not** reject: non-browser
/// clients do not send it, and treating absence as hostile would break curl, the MCP
/// stdio proxy, and every CLI consumer.
#[must_use]
pub fn sec_fetch_site_is_hostile(value: Option<&str>) -> bool {
    matches!(value, Some("cross-site"))
}

/// The hostname part of an authority, with any port removed and IPv6 brackets stripped.
fn hostname_of(authority: &str) -> Option<String> {
    let authority = authority.trim();
    if authority.is_empty() {
        return None;
    }

    if let Some(rest) = authority.strip_prefix('[') {
        // `[::1]:53125` — the colons inside the brackets are part of the address.
        let end = rest.find(']')?;
        return Some(rest[..end].to_ascii_lowercase());
    }

    let hostname = authority.split(':').next().unwrap_or(authority);
    Some(hostname.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_hosts_are_accepted_on_any_port() {
        for host in [
            "127.0.0.1:51234",
            "localhost:51234",
            "127.0.0.1",
            "localhost",
            "[::1]:51234",
        ] {
            assert!(host_is_loopback(host), "{host}");
        }
    }

    #[test]
    fn a_rebound_domain_is_rejected_even_though_it_resolved_to_us() {
        // The whole point of the Host check. The attacker controls DNS, not the header.
        assert!(!host_is_loopback("evil.example:51234"));
        assert!(!host_is_loopback("localhost.evil.example"));
        assert!(!host_is_loopback("127.0.0.1.evil.example"));
    }

    #[test]
    fn a_hostname_that_merely_contains_localhost_is_rejected() {
        // A substring check would pass all of these. It is an easy check to write wrongly.
        assert!(!host_is_loopback("notlocalhost"));
        assert!(!host_is_loopback("localhost.attacker.tld"));
        assert!(!host_is_loopback("my-localhost"));
    }

    #[test]
    fn an_absent_origin_is_allowed() {
        // curl, MCP clients, same-origin fetches. Browsers attach an Origin to
        // cross-origin requests, so absence is not something a hostile page can arrange.
        assert!(origin_is_allowed(None, Some("127.0.0.1:51234")));
        assert!(origin_is_allowed(Some(""), Some("127.0.0.1:51234")));
        assert!(origin_is_allowed(Some("null"), Some("127.0.0.1:51234")));
    }

    #[test]
    fn the_pinned_extension_is_allowed_and_no_other_extension_is() {
        // Inventory §10.1: the previous implementation once echoed *any*
        // chrome-extension:// origin, which is the same as having no check.
        assert!(origin_is_allowed(
            Some(EXTENSION_ORIGIN),
            Some("127.0.0.1:51234")
        ));
        assert!(!origin_is_allowed(
            Some("chrome-extension://bmmpclmokobfnnbiffimagopbehjknih"),
            Some("127.0.0.1:51234")
        ));
    }

    #[test]
    fn a_loopback_origin_must_match_the_host_it_arrived_on() {
        // Inventory §10.2, the Host-echo rule. Same port: same origin. Different port:
        // a different origin that happens to be on the same machine.
        assert!(origin_is_allowed(
            Some("http://127.0.0.1:51234"),
            Some("127.0.0.1:51234")
        ));
        assert!(!origin_is_allowed(
            Some("http://127.0.0.1:4321"),
            Some("127.0.0.1:51234")
        ));
        assert!(!origin_is_allowed(
            Some("http://localhost:51234"),
            Some("127.0.0.1:51234")
        ));
    }

    #[test]
    fn a_remote_origin_is_rejected() {
        assert!(!origin_is_allowed(
            Some("https://evil.example"),
            Some("127.0.0.1:51234")
        ));
        assert!(!origin_is_allowed(
            Some("http://evil.example"),
            Some("127.0.0.1:51234")
        ));
    }

    #[test]
    fn https_loopback_is_not_an_origin_we_serve() {
        assert!(!origin_is_allowed(
            Some("https://127.0.0.1:51234"),
            Some("127.0.0.1:51234")
        ));
    }

    #[test]
    fn a_missing_sec_fetch_site_never_rejects() {
        // R-SEC-12 is explicit: absence must not reject. curl, the MCP stdio proxy, and
        // every CLI consumer would break.
        assert!(!sec_fetch_site_is_hostile(None));
        assert!(!sec_fetch_site_is_hostile(Some("same-origin")));
        assert!(!sec_fetch_site_is_hostile(Some("none")));
        assert!(sec_fetch_site_is_hostile(Some("cross-site")));
    }

    #[test]
    fn the_pinned_origin_is_the_one_chrome_derives_from_the_manifest_key() {
        // R-EXT-2 / Inventory §10.1 — one fact in two files, verified rather than
        // asserted in a comment. Chrome derives an extension id by taking SHA-256 of the
        // DER public key and mapping the first 16 bytes' nibbles onto 'a'..'p'.
        //
        // The manifest's key lives in web/extension/manifest.json. If someone regenerates
        // it without updating this constant, capture breaks with a 403 that looks like a
        // pairing bug; this test names the real cause.
        const MANIFEST_KEY: &str = concat!(
            "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA18tkHEiUMV9TAQ5JRUFDOU8QRPscQNd6",
            "2WjYncdz4yrNPwiY7/CoKOJ9clecW6KsoL5n2KWDsV6ns5QbO01UhB8PnRLctRm/2WlNwoR6jgn6",
            "Np5GX4GVi3abLBV3DNwoe3geRy55C384Hl6FmmG5yl4ymyZlPEh/iK1UQMmm1eaSL/en0JX4pMIm",
            "FEjJvEZxBwCVJSA4Xgc9juDs4SkATMgIB3TeqzQD4DwYh5JrsU0b9JrglrOLkGWr7SlSb/gVnLio",
            "EkmWpZBdQzQm0ZdtXIMG5oMKEyve04Inn5RoBFqX/5eVV05C+vKxXxJ7ctEYJFZkoXET6fOSDx05",
            "0gvG0wIDAQAB"
        );

        let expected = format!("chrome-extension://{}", extension_id(MANIFEST_KEY));
        assert_eq!(EXTENSION_ORIGIN, expected);
    }

    /// Chrome's extension-id derivation, implemented only for the test above.
    ///
    /// Kept here rather than in the crate proper because nothing at runtime needs it: the
    /// origin is a constant, and this exists to prove the constant is the right one.
    fn extension_id(base64_key: &str) -> String {
        let der = decode_base64(base64_key);
        let digest = sha256(&der);
        digest[..16]
            .iter()
            .flat_map(|byte| {
                [
                    char::from(b'a' + (byte >> 4)),
                    char::from(b'a' + (byte & 0x0f)),
                ]
            })
            .collect()
    }

    fn decode_base64(input: &str) -> Vec<u8> {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(input)
            .expect("the manifest key is valid base64")
    }

    /// A minimal SHA-256, so this assertion needs no dependency of its own.
    fn sha256(message: &[u8]) -> [u8; 32] {
        const K: [u32; 64] = [
            0x428a_2f98,
            0x7137_4491,
            0xb5c0_fbcf,
            0xe9b5_dba5,
            0x3956_c25b,
            0x59f1_11f1,
            0x923f_82a4,
            0xab1c_5ed5,
            0xd807_aa98,
            0x1283_5b01,
            0x2431_85be,
            0x550c_7dc3,
            0x72be_5d74,
            0x80de_b1fe,
            0x9bdc_06a7,
            0xc19b_f174,
            0xe49b_69c1,
            0xefbe_4786,
            0x0fc1_9dc6,
            0x240c_a1cc,
            0x2de9_2c6f,
            0x4a74_84aa,
            0x5cb0_a9dc,
            0x76f9_88da,
            0x983e_5152,
            0xa831_c66d,
            0xb003_27c8,
            0xbf59_7fc7,
            0xc6e0_0bf3,
            0xd5a7_9147,
            0x06ca_6351,
            0x1429_2967,
            0x27b7_0a85,
            0x2e1b_2138,
            0x4d2c_6dfc,
            0x5338_0d13,
            0x650a_7354,
            0x766a_0abb,
            0x81c2_c92e,
            0x9272_2c85,
            0xa2bf_e8a1,
            0xa81a_664b,
            0xc24b_8b70,
            0xc76c_51a3,
            0xd192_e819,
            0xd699_0624,
            0xf40e_3585,
            0x106a_a070,
            0x19a4_c116,
            0x1e37_6c08,
            0x2748_774c,
            0x34b0_bcb5,
            0x391c_0cb3,
            0x4ed8_aa4a,
            0x5b9c_ca4f,
            0x682e_6ff3,
            0x748f_82ee,
            0x78a5_636f,
            0x84c8_7814,
            0x8cc7_0208,
            0x90be_fffa,
            0xa450_6ceb,
            0xbef9_a3f7,
            0xc671_78f2,
        ];

        let mut h: [u32; 8] = [
            0x6a09_e667,
            0xbb67_ae85,
            0x3c6e_f372,
            0xa54f_f53a,
            0x510e_527f,
            0x9b05_688c,
            0x1f83_d9ab,
            0x5be0_cd19,
        ];

        let mut padded = message.to_vec();
        let bit_len = (message.len() as u64) * 8;
        padded.push(0x80);
        while padded.len() % 64 != 56 {
            padded.push(0);
        }
        padded.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in padded.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (index, word) in chunk.chunks_exact(4).enumerate() {
                w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for index in 16..64 {
                let s0 = w[index - 15].rotate_right(7)
                    ^ w[index - 15].rotate_right(18)
                    ^ (w[index - 15] >> 3);
                let s1 = w[index - 2].rotate_right(17)
                    ^ w[index - 2].rotate_right(19)
                    ^ (w[index - 2] >> 10);
                w[index] = w[index - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[index - 7])
                    .wrapping_add(s1);
            }

            let mut v = h;
            for index in 0..64 {
                let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
                let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
                let temp1 = v[7]
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[index])
                    .wrapping_add(w[index]);
                let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
                let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
                let temp2 = s0.wrapping_add(maj);

                v[7] = v[6];
                v[6] = v[5];
                v[5] = v[4];
                v[4] = v[3].wrapping_add(temp1);
                v[3] = v[2];
                v[2] = v[1];
                v[1] = v[0];
                v[0] = temp1.wrapping_add(temp2);
            }

            for (slot, value) in h.iter_mut().zip(v) {
                *slot = slot.wrapping_add(value);
            }
        }

        let mut digest = [0u8; 32];
        for (index, word) in h.iter().enumerate() {
            digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}
