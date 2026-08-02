//! End-to-end: three versions in one folder become three versions a user can move between.
//!
//! Modelled on a real generated project — `v1/ v2/ v3/`, each an `index.html` and a
//! `styles.css`, which is exactly what an AI client writes from the prompt template's Output
//! section. Every piece of this has unit tests; what they cannot show is the wiring, and the
//! wiring is where this feature could be silently wrong: a jail that serves the bytes but
//! never reaches the injector, an endpoint registered under a path the script does not call,
//! a stylesheet quietly rewritten because the extension gate sat on the wrong side of a
//! branch.
//!
//! So this runs the real service and asserts what the browser would receive.

use std::path::Path;

/// A project shaped like the ones this feature exists for.
fn generated_project(root: &Path, manifest: Option<&str>) {
    for version in ["v1", "v2", "v3"] {
        let folder = root.join(version);
        std::fs::create_dir_all(&folder).expect("mkdir");
        std::fs::write(
            folder.join("index.html"),
            format!("<html><body><h1>{version}</h1></body></html>"),
        )
        .expect("index");
        std::fs::write(folder.join("styles.css"), "body { color: #111; }").expect("css");
    }
    if let Some(body) = manifest {
        std::fs::write(root.join(curio_core::variants::MANIFEST_FILE_NAME), body)
            .expect("manifest");
    }
}

const MANIFEST: &str = r#"{
  "version": 1,
  "variants": [
    {"folder": "v1", "name": "Print-tech", "family": "Editorial Print",
     "design_type": "Landing page", "tags": ["risograph", "monospace"]},
    {"folder": "v2", "name": "Blue-Ink Agency"},
    {"folder": "v3", "name": "Powder Glass Dashboard"},
    {"folder": "v9", "name": "A version that was never written"}
  ]
}"#;

#[tokio::test]
async fn a_generated_project_gets_a_switcher_over_every_version() {
    let data_root = tempfile::tempdir().expect("data root");
    std::fs::create_dir_all(data_root.path().join("skills")).expect("skills dir");
    std::fs::write(
        data_root
            .path()
            .join(curio_core::paths::SKILL_FILE_RELATIVE),
        "# Rubric\nDescribe what you see.",
    )
    .expect("rubric");

    let project = tempfile::tempdir().expect("project");
    generated_project(project.path(), Some(MANIFEST));

    let service = curio_server::Service::start(curio_server::ServiceConfig {
        data_root: data_root.path().to_path_buf(),
        runtime_file: data_root.path().join("runtime.json"),
        port: None,
        version: "0.0.0-test".to_owned(),
        quit_token: "quit".to_owned(),
        config: curio_core::config::Config::default(),
    })
    .await
    .expect("service starts");

    let port = service.port();
    let token = service.state().token().expose().to_owned();
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");
    let auth = format!("Bearer {token}");

    let registered: serde_json::Value = client
        .post(format!("{base}/api/projects"))
        .header("authorization", &auth)
        .json(&serde_json::json!({ "path": project.path().to_string_lossy() }))
        .send()
        .await
        .expect("register")
        .json()
        .await
        .expect("register json");
    let id = registered["id"].as_str().expect("project id").to_owned();

    // 1. Opening still lands on the newest version. The switcher is what the other two are
    //    reachable through, and it must not have changed where the front door is.
    let opened: serde_json::Value = client
        .post(format!("{base}/api/projects/{id}/open"))
        .header("authorization", &auth)
        .send()
        .await
        .expect("open")
        .json()
        .await
        .expect("open json");
    assert_eq!(opened["entry"], "v3/index.html");

    // 2. The page the user actually receives carries the tag, pointed at itself.
    let page = client
        .get(format!("{base}/p/{id}/v3/index.html"))
        .header("authorization", &auth)
        .send()
        .await
        .expect("page")
        .text()
        .await
        .expect("page body");
    assert!(page.contains("/__curio/variant-switcher.js"), "{page}");
    assert!(
        page.contains(&format!("data-curio-project=\"{id}\"")),
        "{page}"
    );
    assert!(
        page.contains("data-curio-entry=\"v3/index.html\""),
        "{page}"
    );
    assert!(
        page.find("variant-switcher.js") < page.find("</body>"),
        "the tag belongs inside the body: {page}"
    );

    // 3. Only pages. A stylesheet the user wrote comes back exactly as they wrote it — the
    //    single most important promise this feature makes about their files.
    let css = client
        .get(format!("{base}/p/{id}/v1/styles.css"))
        .header("authorization", &auth)
        .send()
        .await
        .expect("css")
        .text()
        .await
        .expect("css body");
    assert_eq!(css, "body { color: #111; }");

    // 4. The endpoint the script calls: every version, oldest first, named by the manifest.
    let variants: serde_json::Value = client
        .get(format!("{base}/api/projects/{id}/variants"))
        .header("authorization", &auth)
        .send()
        .await
        .expect("variants")
        .json()
        .await
        .expect("variants json");

    let list = variants["variants"].as_array().expect("a list");
    assert_eq!(list.len(), 3, "{variants}");
    assert_eq!(list[0]["slug"], "v1");
    assert_eq!(
        list[2]["slug"], "v3",
        "oldest first, the order the brief asked for"
    );
    assert_eq!(list[0]["name"], "Print-tech");
    assert_eq!(list[0]["family"], "Editorial Print");
    assert_eq!(list[0]["tags"][0], "risograph");
    assert_eq!(list[0]["url"], format!("/p/{id}/v1/index.html"));
    assert_eq!(variants["manifest_status"], "ok");
    assert_eq!(
        variants["unknown_folders"][0], "v9",
        "a manifest cannot conjure a version that is not on disk, and says so"
    );

    // 5. The script itself is served, and is JavaScript rather than the dashboard shell.
    let script = client
        .get(format!("{base}/__curio/variant-switcher.js"))
        .header("authorization", &auth)
        .send()
        .await
        .expect("script");
    assert_eq!(script.status(), reqwest::StatusCode::OK);
    assert!(
        script
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/javascript")),
        "{:?}",
        script.headers()
    );
    assert!(
        script
            .text()
            .await
            .expect("script body")
            .contains("attachShadow")
    );

    let _ = service.shutdown().await;
}

#[tokio::test]
async fn a_project_with_no_manifest_still_lists_its_versions() {
    // The state every already-generated folder is in. Names fall back to the folder, which
    // is worse than "Print-tech" and still enough to move between three designs.
    let data_root = tempfile::tempdir().expect("data root");
    std::fs::create_dir_all(data_root.path().join("skills")).expect("skills dir");
    std::fs::write(
        data_root
            .path()
            .join(curio_core::paths::SKILL_FILE_RELATIVE),
        "# Rubric",
    )
    .expect("rubric");

    let project = tempfile::tempdir().expect("project");
    generated_project(project.path(), None);

    let service = curio_server::Service::start(curio_server::ServiceConfig {
        data_root: data_root.path().to_path_buf(),
        runtime_file: data_root.path().join("runtime.json"),
        port: None,
        version: "0.0.0-test".to_owned(),
        quit_token: "quit".to_owned(),
        config: curio_core::config::Config::default(),
    })
    .await
    .expect("service starts");

    let port = service.port();
    let token = service.state().token().expose().to_owned();
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");
    let auth = format!("Bearer {token}");

    let registered: serde_json::Value = client
        .post(format!("{base}/api/projects"))
        .header("authorization", &auth)
        .json(&serde_json::json!({ "path": project.path().to_string_lossy() }))
        .send()
        .await
        .expect("register")
        .json()
        .await
        .expect("register json");
    let id = registered["id"].as_str().expect("project id").to_owned();

    let variants: serde_json::Value = client
        .get(format!("{base}/api/projects/{id}/variants"))
        .header("authorization", &auth)
        .send()
        .await
        .expect("variants")
        .json()
        .await
        .expect("variants json");

    assert_eq!(variants["manifest_status"], "absent");
    assert_eq!(variants["variants"][0]["name"], "v1");
    assert_eq!(variants["variants"][0]["described"], false);
    assert_eq!(variants["variants"].as_array().expect("list").len(), 3);

    let _ = service.shutdown().await;
}
