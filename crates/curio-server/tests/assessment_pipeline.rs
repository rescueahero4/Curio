//! End-to-end: a capture lands, the worker drains it, the item becomes findable.
//!
//! This is the path E7 exists to close. Everything around it already worked before —
//! `POST /api/items` wrote the row, the card appeared, the job enqueued — and the item then
//! sat at `processing` forever because nothing claimed the job. Unit tests would not have
//! caught that: every piece passed on its own.
//!
//! So this test runs the **whole** service, against a stub standing in for the Anthropic
//! API, and asserts on what a user would see: the item's status, its tags, and its family.
//!
//! The stub is a real HTTP server rather than a mocked client, because the thing most
//! likely to break is the request the client builds — a wrong header or a body the API
//! rejects is invisible to a mock that only checks the arguments.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::routing::post;

/// What the stub replies with. Deliberately includes a duplicate tag and an unnamed
/// family, so the write-back's cleaning and threshold logic are both exercised.
const ASSESSMENT: &str = r#"{
  "name_suggestion": "Stripe pricing",
  "short_description": "A three-column pricing table with a highlighted middle tier.",
  "design_types": ["pricing page", "Pricing Page"],
  "tags": ["saas", "minimal", "SAAS"],
  "family_scores": [],
  "new_family_proposal": {"name": "Warm Editorial", "description": "Serif headlines on paper-warm neutrals."},
  "image_recipe": null
}"#;

/// A tiny PNG the image pipeline can actually decode.
fn screenshot() -> Vec<u8> {
    let mut buffer = image::RgbImage::new(1200, 900);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        #[allow(clippy::cast_possible_truncation, reason = "test fixture")]
        {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }
    }
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(buffer)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("encode");
    out
}

#[derive(Clone)]
struct Stub {
    calls: Arc<AtomicUsize>,
    /// Every request body the stub saw, so the test can assert on the *request* rather
    /// than only on what came back.
    seen: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

async fn stub_messages(
    State(stub): State<Stub>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    stub.calls.fetch_add(1, Ordering::Relaxed);
    stub.seen.lock().expect("lock").push(body);

    Json(serde_json::json!({
        "id": "msg_stub",
        "type": "message",
        "role": "assistant",
        "stop_reason": "end_turn",
        "content": [{ "type": "text", "text": ASSESSMENT }],
    }))
}

/// Bring up the stub and return its base URL.
async fn start_stub(stub: Stub) -> String {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("bind stub");
    let port = listener.local_addr().expect("addr").port();

    let app = axum::Router::new()
        .route("/v1/messages", post(stub_messages))
        .with_state(stub);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn a_capture_is_assessed_without_anyone_asking() {
    let stub = Stub {
        calls: Arc::new(AtomicUsize::new(0)),
        seen: Arc::new(std::sync::Mutex::new(Vec::new())),
    };
    let base_url = start_stub(stub.clone()).await;

    let data_root = tempfile::tempdir().expect("data root");
    let runtime_file = data_root.path().join("runtime.json");
    std::fs::create_dir_all(data_root.path().join("skills")).expect("skills dir");
    std::fs::write(
        data_root
            .path()
            .join(curio_core::paths::SKILL_FILE_RELATIVE),
        "# Rubric\nDescribe what you see.",
    )
    .expect("rubric");

    // SAFETY: single-threaded test process; nothing else reads these while they are set.
    unsafe {
        std::env::set_var(curio_server::ai::BASE_URL_ENV, &base_url);
        std::env::set_var(curio_server::secrets::ENV_VAR, "sk-ant-test");
    }

    let service = curio_server::Service::start(curio_server::ServiceConfig {
        data_root: data_root.path().to_path_buf(),
        runtime_file: runtime_file.clone(),
        port: None,
        version: "0.0.0-test".to_owned(),
        quit_token: "quit".to_owned(),
        config: curio_core::config::Config::default(),
    })
    .await
    .expect("service starts");

    let port = service.port();
    let token = service.state().token().expose().to_owned();

    // A real multipart capture, exactly as the extension sends one.
    let boundary = "----curiotest";
    let mut body: Vec<u8> = Vec::new();
    let mut push = |part: &str| body.extend_from_slice(part.as_bytes());
    push(&format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"source_url\"\r\n\r\nhttps://stripe.com/pricing\r\n"
    ));
    push(&format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"viewport_width\"\r\n\r\n1200\r\n"
    ));
    push(&format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"viewport_height\"\r\n\r\n800\r\n"
    ));
    push(&format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"screenshot\"; filename=\"s.png\"\r\nContent-Type: image/png\r\n\r\n"
    ));
    body.extend_from_slice(&screenshot());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let client = reqwest::Client::new();
    let created: serde_json::Value = client
        .post(format!("http://127.0.0.1:{port}/api/items"))
        .header("authorization", format!("Bearer {token}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .expect("ingest")
        .json()
        .await
        .expect("ingest json");

    let item_id = created["item_id"].as_str().expect("item id").to_owned();
    assert_eq!(
        created["item"]["status"], "processing",
        "a capture is a card before it is an assessment (FR-3)"
    );
    assert!(
        created["item"]["thumbnail_path"].is_string(),
        "the grid's copy is written at ingest, where the viewport is known (R-BE-26)"
    );

    // Now the part that did not exist before E7: nobody asks for this.
    let item = wait_for_assessment(&client, port, &token, &item_id).await;

    assert_eq!(
        item["status"], "ready",
        "the proposal branch settles the item rather than holding it for review"
    );
    assert_eq!(item["name"], "Stripe pricing");
    assert_eq!(
        item["tags"].as_array().expect("tags").len(),
        2,
        "the duplicate tag is dropped before it becomes a row: {:?}",
        item["tags"]
    );
    assert_eq!(
        item["design_types"].as_array().expect("types").len(),
        1,
        "case-insensitive duplicates collapse: {:?}",
        item["design_types"]
    );

    let families = item["families"].as_array().expect("families");
    assert_eq!(families.len(), 1);
    assert_eq!(families[0]["name"], "Warm Editorial");
    assert!(
        families[0]["ai_proposed"].as_bool().expect("flag"),
        "a family created from a proposal is marked as one"
    );

    // The sidecar is regenerated in the same transaction as the write (R-DA-4).
    let sidecar = data_root.path().join(format!("items/{item_id}/item.md"));
    let rendered = std::fs::read_to_string(&sidecar).expect("sidecar written");
    assert!(rendered.contains("Warm Editorial"), "{rendered}");

    // And the request that produced all this was shaped the way R-BE-23 requires.
    // Copied out of the lock rather than asserted under it: the guard is not async-aware,
    // and the shutdown below awaits.
    let request = {
        let seen = stub.seen.lock().expect("lock");
        seen.first()
            .cloned()
            .expect("the model was actually called")
    };
    let system = request["system"].as_array().expect("system blocks");
    assert_eq!(
        system
            .iter()
            .filter(|b| b.get("cache_control").is_some())
            .count(),
        2,
        "exactly two cache breakpoints (R-BE-23)"
    );
    assert_eq!(request["max_tokens"], 8000);
    assert_eq!(request["output_config"]["effort"], "medium");
    assert_eq!(request["messages"][0]["content"][0]["type"], "image");
    assert_eq!(
        request["messages"][0]["content"][0]["source"]["media_type"], "image/jpeg",
        "the screenshot is downscaled before it is sent (R-BE-26)"
    );

    assert_eq!(
        stub.calls.load(Ordering::Relaxed),
        1,
        "one call per assessment, not a retry storm"
    );

    service.shutdown().await.expect("clean shutdown");
    assert!(!runtime_file.exists(), "the token dies with the process");
}

/// Poll until the worker has settled the item, or give up loudly.
async fn wait_for_assessment(
    client: &reqwest::Client,
    port: u16,
    token: &str,
    item_id: &str,
) -> serde_json::Value {
    // Generous: the worker polls every two seconds, and CI machines are slow.
    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(200)).await;

        let item: serde_json::Value = client
            .get(format!("http://127.0.0.1:{port}/api/items/{item_id}"))
            .header("authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("read item")
            .json()
            .await
            .expect("item json");

        if item["status"] != "processing" {
            return item;
        }
    }

    panic!("the item never left `processing` — the queue is not draining");
}
