/** The repository every external link on this site points into. */
export const REPO = "https://github.com/rescueahero4/Curio";

/**
 * The release asset filenames, and the reason they look the way they do.
 *
 * **These two strings must stay byte-identical across every release.** They are produced by
 * `.github/workflows/release.yml` (the `-DOUT_FILE` passed to makensis) and
 * `packaging/macos/bundle.sh` (the `DMG` variable), and the release job lists them
 * literally rather than by glob so a rename fails the release instead of silently breaking
 * the buttons below.
 *
 * They carry no version on purpose (D36). GitHub's only durable "newest release" URL is
 * `/releases/latest/download/<asset-name>`, and it resolves by exact filename — a version in
 * the name would 404 every button the moment the next tag shipped. The version is still
 * carried by `--version`, `/health`, `runtime.json`, the installer's `VIProductVersion` and
 * the app bundle's `CFBundleShortVersionString`; the filename was always the weakest of them
 * and is the only one a stable URL forbids.
 */
const WINDOWS_ASSET = "curio-windows-x64-setup.exe";
const MACOS_ASSET = "curio-macos-universal.dmg";

/**
 * Where each button in the hero goes.
 *
 * `/releases/latest/download/…` is a redirect GitHub resolves to the newest **published**
 * release — a draft does not count, which is why a freshly tagged release only lights these
 * up once someone presses Publish.
 */
export const downloads = {
	windows: `${REPO}/releases/latest/download/${WINDOWS_ASSET}`,
	macos: `${REPO}/releases/latest/download/${MACOS_ASSET}`,

	/**
	 * The releases index. Worth keeping reachable: it is where someone lands who wants an
	 * older version, the checksums, or the release notes.
	 */
	releases: `${REPO}/releases`,

	/**
	 * The install guide — how to get past SmartScreen and Gatekeeper on an unsigned build.
	 *
	 * It lives in the newest release's notes, so this is `/releases/latest` rather than a
	 * `/releases/tag/v0.1.0` link: GitHub redirects it to the current tag page (today,
	 * exactly `tag/v0.1.0`), and a pinned tag would still be showing v0.1.0's instructions
	 * long after the buttons above had moved on. Same reasoning as the asset URLs (D36).
	 */
	installGuide: `${REPO}/releases/latest`,

	/**
	 * The extension is not on any web store yet (E10), so this points at its source rather
	 * than a listing that does not exist. When the Chrome Web Store item is live, this line
	 * is the only thing that changes.
	 */
	extension: `${REPO}/tree/master/web/extension`,
} as const;
